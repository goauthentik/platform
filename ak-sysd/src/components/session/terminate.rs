use crate::state::SessionRecord;
use eyre::Result;

/// Terminates the process associated with an expired session. Linux sends
/// SIGTERM then SIGKILL (ported from `terminate_linux.go`); Windows calls
/// `TerminateProcess` (Go has no implementation to port). Untested on Windows.
pub async fn terminate_session(session: &SessionRecord) -> Result<()> {
    if let Some(socket) = &session.local_socket {
        std::fs::remove_file(socket).ok();
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(pid) = session.pid {
            let pid = pid as libc::pid_t;
            unsafe { libc::kill(pid, libc::SIGTERM) };
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let still_alive = unsafe { libc::kill(pid, 0) } == 0;
            if still_alive {
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(pid) = session.pid {
            terminate_pid(pid);
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn terminate_pid(pid: u32) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    unsafe {
        match OpenProcess(PROCESS_TERMINATE, false, pid) {
            Ok(handle) => {
                if let Err(e) = TerminateProcess(handle, 1) {
                    tracing::warn!(pid, "failed to terminate process: {e}");
                }
                let _ = CloseHandle(handle);
            }
            Err(e) => tracing::warn!(pid, "failed to open process for termination: {e}"),
        }
    }
}
