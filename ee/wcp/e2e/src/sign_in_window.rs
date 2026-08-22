//! Finds the real `ak_browser.exe` window while `Connect` is still blocked on
//! it.
//!
//! `Connect` does not return until the sign-in is over and the window is gone,
//! so anything asserted about it has to be seen from another thread. Windows
//! belong to a desktop rather than a thread, and under `CPUS_CREDUI` the child
//! is spawned into this same session, so any thread here can see it.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    System::Threading::{
        OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    },
    UI::WindowsAndMessaging::{
        EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    },
};
use windows_core::{BOOL, PWSTR};

/// What the sign-in window looked like the first time it was seen.
#[derive(Debug, Clone, Copy)]
pub struct Observation {
    pub extended_style: u32,
    pub appeared_after: Duration,
}

/// Watches for `ak_browser.exe`'s window in the background and reports the
/// first sighting, or `None` after `timeout`.
pub fn watch(timeout: Duration) -> mpsc::Receiver<Option<Observation>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let started = Instant::now();
        let found = loop {
            if let Some(hwnd) = find() {
                break Some(Observation {
                    extended_style: unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32,
                    appeared_after: started.elapsed(),
                });
            }
            if started.elapsed() >= timeout {
                break None;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let _ = tx.send(found);
    });
    rx
}

struct Search {
    found: isize,
}

/// The visible sign-in window of an `ak_browser.exe`.
///
/// Matched on the title rather than on enumeration order: the window is only
/// revealed seconds after the spawn, and the WebView2 helper that beats it to
/// the desktop is neither message-only nor zero-size, so the visibility and
/// size filters do not rule that one out.
fn find() -> Option<HWND> {
    let mut search = Search { found: 0 };
    // Reports failure when the callback stops the walk early, which is what
    // finding a match does, so `found` is the answer rather than the result.
    let _ = unsafe {
        EnumWindows(
            Some(visit),
            LPARAM(std::ptr::from_mut(&mut search) as isize),
        )
    };
    (search.found != 0).then_some(HWND(search.found as *mut std::ffi::c_void))
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = unsafe { &mut *(lparam.0 as *mut Search) };

    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return true.into();
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err()
        || rect.right <= rect.left
        || rect.bottom <= rect.top
    {
        return true.into();
    }

    let mut title = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) };
    if len <= 0 || String::from_utf16_lossy(&title[..len as usize]) != ak_ee_wcp_wire::WINDOW_TITLE
    {
        return true.into();
    }

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if !is_browser_host(pid) {
        return true.into();
    }

    search.found = hwnd.0 as isize;
    false.into()
}

fn is_browser_host(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    let Ok(process) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) })
    else {
        return false;
    };

    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    let named = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = windows::Win32::Foundation::CloseHandle(process);
    }
    if named.is_err() {
        return false;
    }

    String::from_utf16_lossy(&buf[..len as usize])
        .to_ascii_lowercase()
        .ends_with("ak_browser.exe")
}
