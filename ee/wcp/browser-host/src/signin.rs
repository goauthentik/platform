//! What this process reports to the credential provider, and the two
//! inherited standard handles it travels over: stdout carries exactly one
//! [`HostReport`], stdin carries the provider's request to close.
//!
//! Note what is *not* here. Validating the token in a `goauthentik.io://`
//! redirect needs `ak-sysd`, which this process has no access to, so the
//! redirect URL is reported verbatim and `credprovider` resolves it into an
//! `AuthResult` on the other side.

use std::fs::File;
use std::os::windows::io::FromRawHandle;
use std::sync::{Arc, Mutex};

use ak_ee_wcp_wire::{HostCommand, HostReport};
use tauri::{AppHandle, Manager};
use windows::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

/// The result pipe, which carries exactly one message.
pub struct Completion {
    result_pipe: Mutex<Option<File>>,
}

impl Completion {
    pub fn new(result_pipe: File) -> Self {
        Self {
            result_pipe: Mutex::new(Some(result_pipe)),
        }
    }

    /// Sends `report` if nothing has been sent yet, and does nothing
    /// afterwards.
    ///
    /// Send-once is what stops the window-closed handler reporting a
    /// cancellation on top of a sign-in that already reached the redirect —
    /// completing the flow closes the window, so both paths run on every
    /// successful sign-in.
    pub fn send(&self, report: HostReport) {
        let mut guard = self
            .result_pipe
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(mut pipe) = guard.take() else {
            return;
        };
        log::info!("reporting {report:?} to the credential provider");
        if let Err(e) = ak_ee_wcp_wire::write_host_report(&mut pipe, &report) {
            log::error!("failed to write the report to the pipe: {e}");
        }
    }
}

/// The inherited standard handles, as owned `File`s.
///
/// Both were opened and access-checked in `credprovider` (running as SYSTEM
/// on the real logon scenarios) before this process existed, and handed over
/// through `STARTUPINFOW` — this process's own token is restricted enough
/// that it could not open them itself.
pub struct StdPipes {
    pub result: File,
    /// Best-effort: without it there is no way to hear a cancellation, but
    /// the sign-in itself does not depend on one.
    pub cancel: Option<File>,
}

pub fn inherited_pipes() -> Option<StdPipes> {
    let result = match unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } {
        // SAFETY: inherited from the parent for exactly this purpose, so
        // taking ownership is correct.
        Ok(handle) => unsafe { File::from_raw_handle(handle.0) },
        Err(e) => {
            log::error!("could not get the inherited result pipe: {e}");
            return None;
        }
    };
    let cancel = match unsafe { GetStdHandle(STD_INPUT_HANDLE) } {
        Ok(handle) => Some(unsafe { File::from_raw_handle(handle.0) }),
        Err(e) => {
            log::error!("could not get the inherited cancel pipe: {e}");
            None
        }
    };
    Some(StdPipes { result, cancel })
}

/// Closes the sign-in window, which ends the flow: Tauri exits once the last
/// window is gone, and the credential provider is watching for that.
///
/// Falls back to exiting outright, since a window that cannot be found is
/// still a process the provider is blocked on.
pub fn close(app: &AppHandle) {
    let windows = app.webview_windows();
    if windows.is_empty() {
        log::warn!("no sign-in window left to close; exiting");
        app.exit(0);
        return;
    }
    for (label, window) in windows {
        if let Err(e) = window.close() {
            log::error!("could not close the '{label}' window: {e}");
            app.exit(0);
        }
    }
}

/// Reads commands off the control pipe in a background thread for as long as
/// the flow lasts.
///
/// `on_start` runs for a `StartSignIn`; anything else ends the flow. A closed
/// pipe and a read error both cancel, and only one of the three is the provider
/// actually asking — a bad handle reads as an immediate error and would
/// otherwise tear the window down the instant it opens, so they stay
/// distinguishable in the log.
pub fn watch_control_pipe(
    mut pipe: File,
    app: AppHandle,
    completion: Arc<Completion>,
    on_start: impl Fn(String, String) + Send + 'static,
) {
    std::thread::spawn(move || {
        loop {
            match ak_ee_wcp_wire::read_host_command(&mut pipe) {
                Ok(Some(HostCommand::StartSignIn { url, header_token })) => {
                    log::info!("credential provider asked the window to start signing in");
                    on_start(url, header_token);
                    // Keep reading: until the redirect fires, a cancellation
                    // over this pipe is the only way the provider can call the
                    // flow off.
                }
                Ok(Some(HostCommand::Cancel)) => {
                    log::info!("credential provider asked the window to close");
                    break;
                }
                Ok(None) => {
                    log::warn!("control pipe closed; cancelling");
                    break;
                }
                Err(e) => {
                    log::error!("control pipe read failed ({e}); cancelling");
                    break;
                }
            }
        }
        completion.send(HostReport::Cancelled);
        close(&app);
    });
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    /// The window-closed handler runs after a completed sign-in too, so a
    /// second send would overwrite the redirect URL with a cancellation — the
    /// symptom being a sign-in that works and still lands the user back at the
    /// logon screen.
    #[test]
    fn only_the_first_report_is_sent() {
        let dir = std::env::temp_dir().join(format!("ak_browser_report_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("result-pipe");
        let file = std::fs::File::create(&path).expect("create a stand-in for the result pipe");

        let completion = Completion::new(file);
        completion.send(HostReport::Redirected {
            url: "goauthentik.io://callback?ak-auth-ia-token=abc".to_string(),
        });
        completion.send(HostReport::Cancelled);

        let mut written = std::fs::File::open(&path).expect("reopen the written pipe stand-in");
        let first = ak_ee_wcp_wire::read_host_report(&mut written)
            .expect("decode the frame")
            .expect("a frame should have been written");
        assert_eq!(
            first,
            HostReport::Redirected {
                url: "goauthentik.io://callback?ak-auth-ia-token=abc".to_string()
            }
        );
        assert_eq!(
            ak_ee_wcp_wire::read_host_report(&mut written).expect("decode past the first frame"),
            None,
            "the second send should not have reached the pipe"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
