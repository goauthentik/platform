//! Getting the sign-in window in front of LogonUI.
//!
//! Z-order and input focus are separate problems and conflating them is what
//! left the window behind LogonUI on every logon but the first.
//! `set_always_on_top` is `SetWindowPos(HWND_TOPMOST, ..., SWP_NOACTIVATE)`
//! underneath, which no foreground rule applies to, so visibility is settled
//! outright. Focus is a race a freshly spawned process routinely loses, so it
//! gets the bounded retry below — and `credprovider` pushing from inside
//! LogonUI, which is the side that can actually win it.

use std::ffi::c_void;

use cef::*;
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};

/// Milliseconds to wait before each re-activation attempt. Front-loaded
/// because what this exists for is losing the foreground to a LogonUI repaint
/// that settles in a few hundred milliseconds, and finite because these are
/// posted onto the thread the browser renders on.
const RETRY_SCHEDULE_MS: [i64; 5] = [150, 400, 900, 1800, 3000];

/// The delay before retry `attempt`, 0-based, or `None` once spent.
fn retry_delay_ms(attempt: u32) -> Option<i64> {
    RETRY_SCHEDULE_MS.get(attempt as usize).copied()
}

/// `active` and `closed` are `is_active()`/`is_closed()` as CEF reports them:
/// a window that already holds focus needs nothing more, and one that is gone
/// must not be touched again.
fn next_retry(active: i32, closed: i32, attempt: u32) -> Option<i64> {
    if active != 0 || closed != 0 {
        return None;
    }
    retry_delay_ms(attempt)
}

/// CEF's `cef_window_handle_t` is `cef_dll_sys`'s own `HWND` newtype, not the
/// `windows` crate's.
fn hwnd_of(window: &Window) -> HWND {
    HWND(window.window_handle().0.cast::<c_void>())
}

/// The process owning the foreground window on this thread's desktop — the
/// one thing that says whether the window never took focus or took it and had
/// it taken straight back.
pub fn foreground_pid() -> Option<u32> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    (pid != 0).then_some(pid)
}

// Holds the window by CEF's own refcount, so it stays valid even if the
// sign-in finishes before the schedule runs out — hence the `is_closed` check.
wrap_task! {
    struct ReassertForeground {
        window: Window,
        attempt: u32,
        ever_activated: std::rc::Rc<std::cell::Cell<bool>>,
    }

    impl Task {
        fn execute(&self) {
            if self.window.is_closed() != 0 {
                return;
            }
            if self.window.is_active() != 0 {
                log::info!(
                    "sign-in window holds the foreground after {} retries",
                    self.attempt
                );
                return;
            }

            log::info!(
                "sign-in window still not focused, retry {}: {}",
                self.attempt,
                describe(&self.window)
            );
            // LogonUI is topmost too, so being in the band is not the same as
            // being at the top of it: `set_always_on_top` is a no-op once
            // already set, and `bring_to_top` is what re-raises within it.
            if self.window.is_always_on_top() == 0 {
                self.window.set_always_on_top(1);
            }
            self.window.bring_to_top();
            self.window.activate();

            schedule_reactivation(&self.window, self.attempt + 1, &self.ever_activated);
        }
    }
}

/// Posts retry `attempt`, or gives up and says so.
pub fn schedule_reactivation(
    window: &Window,
    attempt: u32,
    ever_activated: &std::rc::Rc<std::cell::Cell<bool>>,
) {
    let Some(delay) = next_retry(window.is_active(), window.is_closed(), attempt) else {
        // Worth a warning rather than an info: the window is on top and
        // readable, so someone will type into it.
        if window.is_closed() == 0 && window.is_active() == 0 && !ever_activated.get() {
            log::warn!(
                "sign-in window is visible but never took focus — keystrokes are going elsewhere: {}",
                describe(window)
            );
        }
        return;
    };

    let mut task = ReassertForeground::new(window.clone(), attempt, ever_activated.clone());
    post_delayed_task(ThreadId::UI, Some(&mut task), delay);
}

fn describe(window: &Window) -> String {
    format!(
        "handle={:?} active={} always_on_top={} foreground_pid={:?}",
        hwnd_of(window).0,
        window.is_active(),
        window.is_always_on_top(),
        foreground_pid(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These post themselves back onto the thread the browser renders on, so a
    /// schedule that never ran out would spin it for as long as the sign-in
    /// window is open, on the logon screen.
    #[test]
    fn the_retry_schedule_terminates_and_never_busy_loops() {
        let delays: Vec<i64> = (0..).map_while(retry_delay_ms).collect();

        assert_eq!(delays.len(), RETRY_SCHEDULE_MS.len());
        assert!(delays.windows(2).all(|w| w[0] < w[1]), "{delays:?}");
        assert!(delays.first().is_some_and(|d| *d > 0), "{delays:?}");
    }

    /// Re-activating a closed window would use a handle the task outlived.
    #[test]
    fn no_retry_once_active_or_closed() {
        assert_eq!(next_retry(1, 0, 0), None, "already active");
        assert_eq!(next_retry(0, 1, 0), None, "already closed");
        assert_eq!(next_retry(0, 0, 0), Some(RETRY_SCHEDULE_MS[0]));
    }

    #[test]
    fn retries_stop_when_the_schedule_runs_out() {
        let last = RETRY_SCHEDULE_MS.len() as u32 - 1;
        assert!(next_retry(0, 0, last).is_some());
        assert_eq!(next_retry(0, 0, last + 1), None);
    }
}
