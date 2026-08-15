//! Spawns `ak_cef.exe` in the interactive session and exchanges
//! `wire`-framed messages with it over a duplex pair of anonymous pipes: a
//! result pipe it writes to, and a control pipe this process writes a
//! cancel signal to.

use std::ffi::c_void;
use std::fs::File;
use std::mem::size_of;
use std::os::windows::io::FromRawHandle;
use std::path::Path;

use windows::{
    Win32::{
        Foundation::{
            CloseHandle, E_FAIL, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, SetHandleInformation,
            WAIT_OBJECT_0,
        },
        Security::SECURITY_ATTRIBUTES,
        System::Pipes::CreatePipe,
        System::Threading::{
            CreateProcessAsUserW, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, InitializeProcThreadAttributeList,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION,
            STARTUPINFOEXW, UpdateProcThreadAttribute, WaitForSingleObject,
        },
        UI::Shell::{CPUS_CREDUI, CREDENTIAL_PROVIDER_USAGE_SCENARIO},
    },
    core::{PCWSTR, PWSTR},
};

use crate::syscalls::acquire_interactive_token;
use wire::AuthResult;

/// Spawns `ak_cef.exe` and waits for its result. `should_continue` is polled
/// while waiting, so LogonUI cancelling (the user backing out of the tile)
/// tears the browser process down instead of orphaning it.
pub trait AuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult;
}

pub struct CefAuthFlow {
    pub cef_exe: std::path::PathBuf,
    pub cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
}

impl AuthFlow for CefAuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult {
        run_cef_host(&self.cef_exe, self.cpus, should_continue)
    }
}

/// Only `CPUS_CREDUI` may fall back to launching in the caller's own session.
/// It is debug-gated and runs on an ordinary desktop, where the caller is
/// already the interactive user and holds no `SE_TCB_NAME`. The logon
/// scenarios must never take it: they run as SYSTEM under LogonUI, so it
/// would put Chromium on the secure desktop with SYSTEM's token.
fn may_launch_in_current_session(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> bool {
    cpus == CPUS_CREDUI
}

/// The desktop LogonUI draws on. A window created on any other desktop of the
/// same window station is fully functional but invisible to the person signing
/// in, so the logon scenarios have to name it: with `lpDesktop` left NULL,
/// `CreateProcess*` gives the child whichever desktop the caller happens to be
/// on, which is only incidentally the right one.
const SECURE_DESKTOP: &str = r"WinSta0\Winlogon";

/// `CPUS_CREDUI` is the debug-gated scenario that runs on the ordinary
/// interactive desktop, so it keeps `lpDesktop` NULL and inherits it.
fn desktop_for(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> Option<Vec<u16>> {
    if may_launch_in_current_session(cpus) {
        return None;
    }
    Some(
        SECURE_DESKTOP
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
    )
}

struct DuplexPipes {
    result_read: HANDLE,
    result_write_inheritable: HANDLE,
    cancel_write: HANDLE,
    cancel_read_inheritable: HANDLE,
}

/// One inheritable pipe pair each way. Our own end of each is marked
/// non-inheritable so the child can't hold it open and mask an EOF.
fn create_duplex_pipes() -> windows::core::Result<DuplexPipes> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };

    // Returns (ours, child's) for a pipe flowing in the given direction.
    let pipe = |ours_reads: bool| -> windows::core::Result<(HANDLE, HANDLE)> {
        let mut read = HANDLE::default();
        let mut write = HANDLE::default();
        unsafe { CreatePipe(&mut read, &mut write, Some(&sa), 0) }?;
        let (ours, theirs) = if ours_reads {
            (read, write)
        } else {
            (write, read)
        };
        unsafe { SetHandleInformation(ours, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }?;
        Ok((ours, theirs))
    };

    let (result_read, result_write_inheritable) = pipe(true)?;
    let (cancel_write, cancel_read_inheritable) = pipe(false)?;

    Ok(DuplexPipes {
        result_read,
        result_write_inheritable,
        cancel_write,
        cancel_read_inheritable,
    })
}

fn run_cef_host(
    cef_exe: &Path,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    let pipes = match create_duplex_pipes() {
        Ok(p) => p,
        Err(e) => {
            log::error!("failed to create IPC pipes: {e}");
            return AuthResult::Failed {
                reason: "failed to create IPC pipes".to_string(),
            };
        }
    };

    let spawn = spawn_cef_host(cef_exe, &pipes, cpus);
    unsafe {
        let _ = CloseHandle(pipes.result_write_inheritable);
        let _ = CloseHandle(pipes.cancel_read_inheritable);
    }

    let process = match spawn {
        Ok(p) => p,
        Err(e) => {
            log::error!("failed to launch {}: {e}", cef_exe.display());
            unsafe {
                let _ = CloseHandle(pipes.result_read);
                let _ = CloseHandle(pipes.cancel_write);
            }
            return AuthResult::Failed {
                reason: "failed to launch sign-in window".to_string(),
            };
        }
    };

    let result = wait_for_result(
        pipes.result_read,
        pipes.cancel_write,
        process.hProcess,
        should_continue,
    );

    unsafe {
        let _ = CloseHandle(pipes.cancel_write);
        let _ = WaitForSingleObject(process.hProcess, 5_000);
        let _ = CloseHandle(process.hProcess);
        let _ = CloseHandle(process.hThread);
    }

    result
}

/// What the result-pipe reader thread saw. `Eof` and `Error` both end up as a
/// cancellation for the user, but they mean very different things — the host
/// died vs. the pipe misbehaved — so they stay separate long enough to be
/// logged.
enum PipeOutcome {
    Result(AuthResult),
    Eof,
    Error(String),
}

/// Polls in short slices so `should_continue` gets a turn. On cancellation it
/// asks `ak_cef.exe` to close over the control pipe rather than killing it.
///
/// Every route out of here other than a real `AuthResult` looks identical to
/// the user ("Login attempt cancelled"), so each one logs why: a silent
/// cancellation is indistinguishable from the sign-in window never appearing.
fn wait_for_result(
    result_read: HANDLE,
    cancel_write: HANDLE,
    process: HANDLE,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    let mut result_file = unsafe { File::from_raw_handle(result_read.0) };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = match wire::read_auth_result(&mut result_file) {
            Ok(Some(result)) => PipeOutcome::Result(result),
            Ok(None) => PipeOutcome::Eof,
            Err(e) => PipeOutcome::Error(e.to_string()),
        };
        let _ = tx.send(outcome);
    });

    let mut cancel_signalled = false;
    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(PipeOutcome::Result(outcome)) => {
                log::info!("sign-in window reported {outcome:?}");
                return outcome;
            }
            Ok(PipeOutcome::Eof) => {
                log::warn!(
                    "sign-in window closed the result pipe without sending a result ({}); \
                     treating as cancelled",
                    describe_exit(process)
                );
                return AuthResult::Cancelled;
            }
            Ok(PipeOutcome::Error(e)) => {
                log::error!("failed to read the sign-in result: {e}; treating as cancelled");
                return AuthResult::Cancelled;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if !cancel_signalled && !should_continue() {
                    log::info!("LogonUI withdrew the sign-in; asking the window to close");
                    cancel_signalled = true;
                    signal_cancel(cancel_write);
                }
                // Host exited without sending a result.
                if unsafe { WaitForSingleObject(process, 0) } == WAIT_OBJECT_0 {
                    log::warn!(
                        "sign-in window exited without sending a result ({})",
                        describe_exit(process)
                    );
                    return AuthResult::Cancelled;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                log::error!("the result-pipe reader thread died; treating as cancelled");
                return AuthResult::Cancelled;
            }
        }
    }
}

/// Exit code of `ak_cef.exe` rendered for a log line. The value is the whole
/// diagnosis when the host dies before it can say anything: `0xC0000005` is a
/// crash, `0xC0000142` is a `DLL_INIT_FAILED` from a desktop the process has no
/// access to, and `0` is a clean exit that simply skipped the result.
fn describe_exit(process: HANDLE) -> String {
    let mut code = 0u32;
    if unsafe { GetExitCodeProcess(process, &mut code) }.is_err() {
        return "exit code unavailable".to_string();
    }
    // 259 is STILL_ACTIVE.
    if code == 259 {
        return "still running".to_string();
    }
    format!("exit code {code:#010x}")
}

fn signal_cancel(cancel_write: HANDLE) {
    let mut f = unsafe { File::from_raw_handle(cancel_write.0) };
    let _ = wire::write_frame(&mut f, &wire::CancelSignal {});
    std::mem::forget(f);
}

fn spawn_cef_host(
    cef_exe: &Path,
    pipes: &DuplexPipes,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
) -> windows::core::Result<PROCESS_INFORMATION> {
    let cmdline = format!(
        "\"{}\" --result-pipe {} --cancel-pipe {}",
        cef_exe.display(),
        pipes.result_write_inheritable.0 as usize,
        pipes.cancel_read_inheritable.0 as usize
    );
    let mut cmdline_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();

    let mut attr_size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(None, 1, Some(0), &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list = LPPROC_THREAD_ATTRIBUTE_LIST(attr_buf.as_mut_ptr() as *mut c_void);
    unsafe { InitializeProcThreadAttributeList(Some(attr_list), 1, Some(0), &mut attr_size) }?;

    let inherit_handles = [
        pipes.result_write_inheritable,
        pipes.cancel_read_inheritable,
    ];
    let update = unsafe {
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherit_handles.as_ptr() as *const c_void),
            size_of::<[HANDLE; 2]>(),
            None,
            None,
        )
    };
    if update.is_err() {
        unsafe { DeleteProcThreadAttributeList(attr_list) };
        return Err(windows::core::Error::from(E_FAIL));
    }

    let mut si = STARTUPINFOEXW {
        lpAttributeList: attr_list,
        ..Default::default()
    };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    // Outlives every `CreateProcess*` call below; `lpDesktop` borrows it.
    let mut desktop = desktop_for(cpus);
    if let Some(desktop) = desktop.as_mut() {
        si.StartupInfo.lpDesktop = PWSTR(desktop.as_mut_ptr());
    }
    let mut pi = PROCESS_INFORMATION::default();

    let token = match acquire_interactive_token() {
        Ok(token) => Some(token),
        Err(e) if may_launch_in_current_session(cpus) => {
            log::debug!("no interactive-session token ({e}); launching in the current session");
            None
        }
        Err(e) => {
            log::error!("could not acquire an interactive-session token: {e}");
            unsafe { DeleteProcThreadAttributeList(attr_list) };
            return Err(e);
        }
    };
    log::info!(
        "launching {} on desktop {} with {} token",
        cef_exe.display(),
        desktop
            .as_ref()
            .map(|_| SECURE_DESKTOP)
            .unwrap_or("<inherited>"),
        if token.is_some() {
            "an interactive-session"
        } else {
            "the caller's own"
        }
    );

    let mut spawned = match token {
        Some(token) => unsafe {
            CreateProcessAsUserW(
                Some(token),
                PCWSTR::null(),
                Some(PWSTR(cmdline_wide.as_mut_ptr())),
                None,
                None,
                true,
                EXTENDED_STARTUPINFO_PRESENT,
                None,
                PCWSTR::null(),
                &si.StartupInfo,
                &mut pi,
            )
        },
        None => spawn_in_current_session(&cmdline, &si.StartupInfo, &mut pi),
    };

    // Holding a token is not the same as being allowed to assign it: without
    // SE_ASSIGNPRIMARYTOKEN/SE_INCREASE_QUOTA, `CreateProcessAsUserW` fails
    // even though a plain `CreateProcessW` in this session would work. Under
    // `CPUS_CREDUI` that is still the right outcome, so retry rather than
    // failing the whole flow. Never reached for the real logon scenarios.
    if let Err(e) = spawned.as_ref()
        && token.is_some()
        && may_launch_in_current_session(cpus)
    {
        log::debug!("CreateProcessAsUserW failed ({e}); retrying in the current session");
        spawned = spawn_in_current_session(&cmdline, &si.StartupInfo, &mut pi);
    }

    unsafe {
        DeleteProcThreadAttributeList(attr_list);
        if let Some(token) = token {
            let _ = CloseHandle(token);
        }
    }

    spawned?;
    Ok(pi)
}

/// `CreateProcessW` may write into the command-line buffer it is handed, so
/// each attempt gets a fresh copy.
fn spawn_in_current_session(
    cmdline: &str,
    startup_info: &windows::Win32::System::Threading::STARTUPINFOW,
    pi: &mut PROCESS_INFORMATION,
) -> windows::core::Result<()> {
    let mut cmdline_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(PWSTR(cmdline_wide.as_mut_ptr())),
            None,
            None,
            true,
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            PCWSTR::null(),
            startup_info,
            pi,
        )
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use windows::Win32::UI::Shell::{CPUS_CHANGE_PASSWORD, CPUS_LOGON, CPUS_UNLOCK_WORKSTATION};

    /// Exercises the real attribute-list / handle-inheritance / CreateProcess
    /// machinery against a throwaway target, without needing an interactive
    /// token, elevation, or anything listening on the `ak-sysd` pipe. A
    /// failure here means `Connect` can never launch the sign-in window,
    /// which otherwise only surfaces as one generic "Sign-in failed" string.
    #[test]
    fn credui_spawn_succeeds_without_an_interactive_token() {
        let pipes = create_duplex_pipes().expect("create duplex pipes");

        // Any real executable will do: this asserts the process is created,
        // not what it does. It exits immediately on the unknown arguments.
        let exe = std::path::PathBuf::from(
            std::env::var("COMSPEC").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".to_string()),
        );

        let spawned = spawn_cef_host(&exe, &pipes, CPUS_CREDUI);

        unsafe {
            let _ = CloseHandle(pipes.result_read);
            let _ = CloseHandle(pipes.result_write_inheritable);
            let _ = CloseHandle(pipes.cancel_write);
            let _ = CloseHandle(pipes.cancel_read_inheritable);
        }

        match spawned {
            Ok(pi) => unsafe {
                let _ = windows::Win32::System::Threading::TerminateProcess(pi.hProcess, 0);
                let _ = CloseHandle(pi.hProcess);
                let _ = CloseHandle(pi.hThread);
            },
            Err(e) => panic!("spawn_cef_host under CPUS_CREDUI failed: {e}"),
        }
    }

    #[test]
    fn only_credui_may_fall_back_to_the_current_session() {
        assert!(may_launch_in_current_session(CPUS_CREDUI));
        for cpus in [CPUS_LOGON, CPUS_UNLOCK_WORKSTATION, CPUS_CHANGE_PASSWORD] {
            assert!(
                !may_launch_in_current_session(cpus),
                "{cpus:?} must require a real interactive-session token"
            );
        }
    }
}
