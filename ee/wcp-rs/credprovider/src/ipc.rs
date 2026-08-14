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
        Foundation::{CloseHandle, E_FAIL, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS},
        Security::SECURITY_ATTRIBUTES,
        System::Pipes::CreatePipe,
        System::Threading::{
            CreateProcessAsUserW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
            InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW,
            UpdateProcThreadAttribute, WaitForSingleObject,
        },
    },
    core::{PCWSTR, PWSTR},
};

use crate::syscalls::acquire_interactive_token;
use wire::AuthResult;

/// Runs the full sign-in flow: spawns `ak_cef.exe`, waits for its result,
/// and returns it. `should_continue` is polled while waiting so LogonUI's
/// own cancellation (e.g. the user backing out of the tile) can tear the
/// browser process down instead of leaving it orphaned.
pub trait AuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult;
}

pub struct CefAuthFlow {
    pub cef_exe: std::path::PathBuf,
}

impl AuthFlow for CefAuthFlow {
    fn run(&self, should_continue: &mut dyn FnMut() -> bool) -> AuthResult {
        run_cef_host(&self.cef_exe, should_continue)
    }
}

struct DuplexPipes {
    result_read: HANDLE,
    result_write_inheritable: HANDLE,
    cancel_write: HANDLE,
    cancel_read_inheritable: HANDLE,
}

fn create_duplex_pipes() -> windows::core::Result<DuplexPipes> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };

    let mut result_read = HANDLE::default();
    let mut result_write = HANDLE::default();
    unsafe { CreatePipe(&mut result_read, &mut result_write, Some(&sa), 0) }?;
    unsafe { windows::Win32::Foundation::SetHandleInformation(result_read, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }?;

    let mut cancel_read = HANDLE::default();
    let mut cancel_write = HANDLE::default();
    unsafe { CreatePipe(&mut cancel_read, &mut cancel_write, Some(&sa), 0) }?;
    unsafe { windows::Win32::Foundation::SetHandleInformation(cancel_write, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }?;

    Ok(DuplexPipes {
        result_read,
        result_write_inheritable: result_write,
        cancel_write,
        cancel_read_inheritable: cancel_read,
    })
}

fn run_cef_host(cef_exe: &Path, should_continue: &mut dyn FnMut() -> bool) -> AuthResult {
    let pipes = match create_duplex_pipes() {
        Ok(p) => p,
        Err(e) => {
            log::error!("failed to create IPC pipes: {e}");
            return AuthResult::Failed {
                reason: "failed to create IPC pipes".to_string(),
            };
        }
    };

    let spawn = spawn_cef_host(cef_exe, &pipes);
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

    let result = wait_for_result(pipes.result_read, pipes.cancel_write, process.hProcess, should_continue);

    unsafe {
        let _ = WaitForSingleObject(process.hProcess, 5_000);
        let _ = CloseHandle(process.hProcess);
        let _ = CloseHandle(process.hThread);
    }

    result
}

/// Polls the result pipe in short slices so `should_continue` gets a chance
/// to run; on cancellation, signals `ak_cef.exe` to close via the control
/// pipe rather than killing it outright.
fn wait_for_result(
    result_read: HANDLE,
    cancel_write: HANDLE,
    process: HANDLE,
    should_continue: &mut dyn FnMut() -> bool,
) -> AuthResult {
    let mut result_file = unsafe { File::from_raw_handle(result_read.0) };
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let outcome = wire::read_auth_result(&mut result_file)
            .ok()
            .flatten()
            .unwrap_or(AuthResult::Cancelled);
        let _ = tx.send(outcome);
    });

    loop {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(outcome) => {
                unsafe {
                    let _ = CloseHandle(cancel_write);
                }
                return outcome;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if !should_continue() {
                    signal_cancel(cancel_write);
                }
                if unsafe { WaitForSingleObject(process, 0) } == windows::Win32::Foundation::WAIT_OBJECT_0 {
                    // Host exited without sending a result.
                    return AuthResult::Cancelled;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return AuthResult::Cancelled,
        }
    }
}

fn signal_cancel(cancel_write: HANDLE) {
    let mut f = unsafe { File::from_raw_handle(cancel_write.0) };
    let _ = wire::write_frame(&mut f, &wire::CancelSignal {});
    std::mem::forget(f);
}

fn spawn_cef_host(
    cef_exe: &Path,
    pipes: &DuplexPipes,
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

    let inherit_handles = [pipes.result_write_inheritable, pipes.cancel_read_inheritable];
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
    let mut pi = PROCESS_INFORMATION::default();

    let token = acquire_interactive_token()?;

    let spawned = unsafe {
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
    };

    unsafe {
        DeleteProcThreadAttributeList(attr_list);
        let _ = CloseHandle(token);
    }

    spawned?;
    Ok(pi)
}
