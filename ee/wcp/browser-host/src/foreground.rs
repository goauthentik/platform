//! Getting the sign-in window in front of LogonUI.
//!
//! Z-order and input focus are separate problems and conflating them is what
//! left the window behind LogonUI on every logon but the first. Putting the
//! window in the topmost band is `SetWindowPos(HWND_TOPMOST, ...,
//! SWP_NOACTIVATE)`, which no foreground rule applies to, so visibility is
//! settled outright. Focus is a race a freshly spawned process routinely
//! loses, so it gets the bounded retry below — and `credprovider` pushing from
//! inside LogonUI, which is the side that can actually win it.
//!
//! Unlike the CEF host this replaces, the retries run on their own thread
//! rather than on the browser's UI thread: `SetWindowPos`, `BringWindowToTop`
//! and `SetForegroundWindow` may all be called against another thread's window,
//! and keeping them off the UI thread means a stuck renderer cannot swallow the
//! schedule.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowThreadProcessId, HWND_TOPMOST, IsWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetForegroundWindow, SetWindowPos, WS_EX_TOPMOST,
};

/// Milliseconds to wait before each re-activation attempt. Front-loaded
/// because what this exists for is losing the foreground to a LogonUI repaint
/// that settles in a few hundred milliseconds, and finite because a window
/// nobody is fighting over should stop being fought over.
const RETRY_SCHEDULE_MS: [u64; 5] = [150, 400, 900, 1800, 3000];

/// The delay before retry `attempt`, 0-based, or `None` once spent.
fn retry_delay_ms(attempt: u32) -> Option<u64> {
    RETRY_SCHEDULE_MS.get(attempt as usize).copied()
}

/// A window that already holds focus needs nothing more, and one that is gone
/// must not be touched again.
fn next_retry(active: bool, closed: bool, attempt: u32) -> Option<u64> {
    if active || closed {
        return None;
    }
    retry_delay_ms(attempt)
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

fn is_active(hwnd: HWND) -> bool {
    (unsafe { GetForegroundWindow() }) == hwnd
}

fn is_closed(hwnd: HWND) -> bool {
    !unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

fn is_topmost(hwnd: HWND) -> bool {
    let ex_style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    ex_style & WS_EX_TOPMOST.0 != 0
}

/// LogonUI is topmost too, so being in the band is not the same as being at
/// the top of it: re-asserting `HWND_TOPMOST` only matters if the window has
/// dropped out of the band, and `BringWindowToTop` is what re-raises within it.
fn push(hwnd: HWND) {
    if !is_topmost(hwnd)
        && let Err(e) = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        }
    {
        log::warn!("could not put the sign-in window back in the topmost band: {e}");
    }
    let _ = unsafe { BringWindowToTop(hwnd) };
    let _ = unsafe { SetForegroundWindow(hwnd) };
}

/// Runs the retry ladder against `hwnd` on its own thread until the window
/// holds the foreground, is closed, or the schedule runs out.
///
/// `hwnd` is passed as an integer because `HWND` is not `Send`; the window
/// belongs to a desktop rather than to a thread, so the handle is just as
/// valid here as it is on the UI thread.
///
/// `ever_activated` is fed by the app's `Focused` window events and separates
/// "never took focus" from "took it and lost it again": same symptom, different
/// fixes, and none of the calls above report either.
pub fn watch(hwnd: isize, ever_activated: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let hwnd = HWND(hwnd as *mut std::ffi::c_void);
        let mut attempt = 0u32;

        while let Some(delay) = next_retry(is_active(hwnd), is_closed(hwnd), attempt) {
            std::thread::sleep(std::time::Duration::from_millis(delay));

            if is_closed(hwnd) {
                return;
            }
            if is_active(hwnd) {
                log::info!("sign-in window holds the foreground after {attempt} retries");
                return;
            }

            log::info!(
                "sign-in window still not focused, retry {attempt}: {}",
                describe(hwnd)
            );
            push(hwnd);
            attempt += 1;
        }

        // Worth a warning rather than an info: the window is on top and
        // readable, so someone will type into it.
        if !is_closed(hwnd) && !is_active(hwnd) && !ever_activated.load(Ordering::SeqCst) {
            log::warn!(
                "sign-in window is visible but never took focus — keystrokes are going elsewhere: {}",
                describe(hwnd)
            );
        }
    });
}

fn describe(hwnd: HWND) -> String {
    format!(
        "handle={:?} active={} always_on_top={} foreground_pid={:?}",
        hwnd.0,
        is_active(hwnd),
        is_topmost(hwnd),
        foreground_pid(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A schedule that never ran out would keep poking at the foreground for
    /// as long as the sign-in window is open, on the logon screen.
    #[test]
    fn the_retry_schedule_terminates_and_never_busy_loops() {
        let delays: Vec<u64> = (0..).map_while(retry_delay_ms).collect();

        assert_eq!(delays.len(), RETRY_SCHEDULE_MS.len());
        assert!(delays.windows(2).all(|w| w[0] < w[1]), "{delays:?}");
        assert!(delays.first().is_some_and(|d| *d > 0), "{delays:?}");
    }

    /// Re-activating a closed window would use a handle the watcher outlived.
    #[test]
    fn no_retry_once_active_or_closed() {
        assert_eq!(next_retry(true, false, 0), None, "already active");
        assert_eq!(next_retry(false, true, 0), None, "already closed");
        assert_eq!(next_retry(false, false, 0), Some(RETRY_SCHEDULE_MS[0]));
    }

    #[test]
    fn retries_stop_when_the_schedule_runs_out() {
        let last = RETRY_SCHEDULE_MS.len() as u32 - 1;
        assert!(next_retry(false, false, last).is_some());
        assert_eq!(next_retry(false, false, last + 1), None);
    }
}
