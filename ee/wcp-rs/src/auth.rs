//! Cross-process authentication flow.
//!
//! Mirrors the goauthentik C++ credential provider's `Connect()` behaviour: the
//! provider triggers a browser UI and then blocks until that UI signals that
//! authentication is complete (carrying the authenticated username), at which
//! point `GetSerialization` can build the logon credential.
//!
//! The C++ version embeds CEF *in-process* and signals completion through a
//! shared `sHookData` struct. Tauri owns its own event loop and runs as a
//! separate process, so instead we launch `auth-app.exe` and wait for it to
//! signal completion over an *inherited anonymous pipe*. The pipe is this
//! provider's `sHookData`.
//!
//! An anonymous pipe (rather than a named pipe) is used so the channel is
//! reachable only by the child we explicitly hand the write handle to: the
//! handle is inherited through `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, so no other
//! process can connect, squat the name, or forge a result.

use std::ffi::c_void;
use std::mem::size_of;
use std::path::PathBuf;

use windows::{
    Win32::{
        Foundation::{
            CloseHandle, HANDLE, HANDLE_FLAG_INHERIT, HANDLE_FLAGS, HMODULE, SetHandleInformation,
        },
        Security::SECURITY_ATTRIBUTES,
        Storage::FileSystem::ReadFile,
        System::{
            LibraryLoader::{
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW,
                GetModuleHandleExW,
            },
            Pipes::CreatePipe,
            Threading::{
                CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
                INFINITE, InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROCESS_INFORMATION, STARTUPINFOEXW,
                UpdateProcThreadAttribute, WaitForSingleObject,
            },
        },
    },
    core::{PCWSTR, PWSTR},
};

/// Result of the auth flow, analogous to the C++ `sHookData` complete/cancel
/// state plus the resolved username.
pub enum AuthOutcome {
    Completed { username: String },
    Cancelled,
}

/// Launch the Tauri auth window and block until it reports a result.
///
/// This is the equivalent of the C++ `Connect()` busy-wait loop
/// (`while (!m_oHookData.IsComplete()) { ... }`).
pub fn run_auth_flow() -> AuthOutcome {
    let exe = match auth_app_path() {
        Some(p) => p,
        None => {
            log::error!("could not resolve auth-app.exe next to the DLL");
            return AuthOutcome::Cancelled;
        }
    };

    // Create the anonymous pipe. The write end is marked inheritable via the
    // SECURITY_ATTRIBUTES so the child can receive it; the read end stays
    // private to this process.
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: std::ptr::null_mut(),
        bInheritHandle: true.into(),
    };
    let mut read_handle = HANDLE::default();
    let mut write_handle = HANDLE::default();
    if unsafe { CreatePipe(&mut read_handle, &mut write_handle, Some(&sa), 0) }.is_err() {
        log::error!("CreatePipe failed");
        return AuthOutcome::Cancelled;
    }

    // Ensure the read end is NOT inheritable, so only the write end crosses into
    // the child.
    if unsafe { SetHandleInformation(read_handle, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0)) }.is_err()
    {
        log::error!("SetHandleInformation failed to clear inherit on read end");
        unsafe {
            let _ = CloseHandle(read_handle);
            let _ = CloseHandle(write_handle);
        }
        return AuthOutcome::Cancelled;
    }

    // Build the command line. The child receives the inherited write handle as a
    // raw integer it can reconstruct. The auth URL and redirect prefix are
    // fetched by auth-app itself via ak_ffi::sys_auth_start_async(), mirroring
    // how the C++ CEF browser fetches them from the backend on startup.
    let cmdline = format!(
        "\"{}\" --pipe-handle {}",
        exe.display(),
        write_handle.0 as usize
    );
    let mut cmdline_wide: Vec<u16> = cmdline.encode_utf16().chain(std::iter::once(0)).collect();

    log::info!("Launching auth-app: {cmdline}");

    // Restrict inheritance to exactly the write handle via
    // PROC_THREAD_ATTRIBUTE_HANDLE_LIST, so no other inheritable handle leaks
    // into the child.
    let mut attr_size = 0usize;
    unsafe {
        // First call computes the required size; it "fails" with the size out-param set.
        let _ = InitializeProcThreadAttributeList(None, 1, Some(0), &mut attr_size);
    }
    let mut attr_buf = vec![0u8; attr_size];
    let attr_list = LPPROC_THREAD_ATTRIBUTE_LIST(attr_buf.as_mut_ptr() as *mut c_void);
    if unsafe { InitializeProcThreadAttributeList(Some(attr_list), 1, Some(0), &mut attr_size) }
        .is_err()
    {
        log::error!("InitializeProcThreadAttributeList failed");
        unsafe {
            let _ = CloseHandle(read_handle);
            let _ = CloseHandle(write_handle);
        }
        return AuthOutcome::Cancelled;
    }

    let inherit_handles = [write_handle];
    let update_ok = unsafe {
        UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            Some(inherit_handles.as_ptr() as *const c_void),
            size_of::<HANDLE>(),
            None,
            None,
        )
    };
    if update_ok.is_err() {
        log::error!("UpdateProcThreadAttribute failed");
        unsafe {
            DeleteProcThreadAttributeList(attr_list);
            let _ = CloseHandle(read_handle);
            let _ = CloseHandle(write_handle);
        }
        return AuthOutcome::Cancelled;
    }

    let mut si = STARTUPINFOEXW::default();
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.lpAttributeList = attr_list;
    let mut pi = PROCESS_INFORMATION::default();

    let spawned = unsafe {
        CreateProcessW(
            None,
            Some(PWSTR(cmdline_wide.as_mut_ptr())),
            None,
            None,
            true, // inherit handles (restricted to the handle list)
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            None,
            &si.StartupInfo,
            &mut pi,
        )
    };

    unsafe {
        DeleteProcThreadAttributeList(attr_list);
    }

    if spawned.is_err() {
        log::error!("CreateProcessW failed for auth-app");
        unsafe {
            let _ = CloseHandle(read_handle);
            let _ = CloseHandle(write_handle);
        }
        return AuthOutcome::Cancelled;
    }

    // The child owns the write end now. Close our copy so the read end sees EOF
    // once the child exits or finishes writing.
    unsafe {
        let _ = CloseHandle(write_handle);
    }

    // Block until the auth-app writes its result (or closes the pipe).
    let line = read_pipe_line(read_handle);

    unsafe {
        let _ = CloseHandle(read_handle);
        // Wait for the child to fully exit before tearing down.
        WaitForSingleObject(pi.hProcess, INFINITE);
        let _ = CloseHandle(pi.hProcess);
        let _ = CloseHandle(pi.hThread);
    }

    log::info!("auth-app signalled: {line:?}");
    match line.as_deref().map(str::trim) {
        Some(msg) => {
            if let Some(user) = msg.strip_prefix("OK") {
                AuthOutcome::Completed {
                    username: user.trim().to_string(),
                }
            } else {
                AuthOutcome::Cancelled
            }
        }
        None => AuthOutcome::Cancelled,
    }
}

/// Read bytes from the pipe's read end until the first newline (the result line).
fn read_pipe_line(read_handle: HANDLE) -> Option<String> {
    let mut data: Vec<u8> = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let mut read = 0u32;
        let ok = unsafe { ReadFile(read_handle, Some(&mut buf), Some(&mut read), None) };
        if ok.is_err() || read == 0 {
            break;
        }
        data.extend_from_slice(&buf[..read as usize]);
        if data.contains(&b'\n') {
            break;
        }
    }
    if data.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&data);
    text.lines().next().map(|s| s.to_string())
}

/// Resolve `auth-app.exe` sitting next to this DLL by locating our own module
/// on disk via its load address.
fn auth_app_path() -> Option<PathBuf> {
    unsafe {
        let mut module = HMODULE::default();
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(auth_app_path as *const u16),
            &mut module,
        )
        .ok()?;

        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(Some(module), &mut buf);
        if len == 0 {
            return None;
        }
        let dll_path = PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
        Some(dll_path.parent()?.join("auth-app.exe"))
    }
}
