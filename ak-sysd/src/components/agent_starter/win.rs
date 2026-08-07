//! Win32 GUI-session helpers, ported from Fleet's `execuser_windows.go`.
//! Compile-checked for windows-msvc but not runtime-tested.

use std::ffi::c_void;
use std::ptr::null_mut;

use eyre::{Result, bail};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    DuplicateTokenEx, SecurityImpersonation, TOKEN_ALL_ACCESS, TokenPrimary,
};
use windows::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};
use windows::Win32::System::RemoteDesktop::{
    WTS_SESSION_INFOW, WTSActive, WTSEnumerateSessionsW, WTSFreeMemory,
    WTSGetActiveConsoleSessionId, WTSQuerySessionInformationW, WTSQueryUserToken, WTSUserName,
};
use windows::Win32::System::Threading::{
    CREATE_NEW_CONSOLE, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, PROCESS_CREATION_FLAGS,
    PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
};
use windows::core::{PCWSTR, PWSTR};

const DESKTOP: &str = "winsta0\\default";
const SW_SHOW: u16 = 5;

/// Active GUI session, falling back to the physical console.
pub fn active_session_id() -> Result<u32> {
    let mut info: *mut WTS_SESSION_INFOW = null_mut();
    let mut count: u32 = 0;
    unsafe {
        WTSEnumerateSessionsW(None, 0, 1, &mut info, &mut count)
            .map_err(|e| eyre::eyre!("WTSEnumerateSessions failed: {e}"))?;
        let sessions = std::slice::from_raw_parts(info, count as usize);
        let found = sessions
            .iter()
            .find(|s| s.State == WTSActive)
            .map(|s| s.SessionId);
        WTSFreeMemory(info as *mut c_void);
        Ok(found.unwrap_or_else(|| WTSGetActiveConsoleSessionId()))
    }
}

pub fn session_username(session_id: u32) -> Result<String> {
    let mut buf = PWSTR::null();
    let mut len: u32 = 0;
    unsafe {
        WTSQuerySessionInformationW(None, session_id, WTSUserName, &mut buf, &mut len)
            .map_err(|e| eyre::eyre!("WTSQuerySessionInformation failed: {e}"))?;
        let name = buf.to_string().unwrap_or_default();
        WTSFreeMemory(buf.as_ptr() as *mut c_void);
        if name.is_empty() {
            bail!("no GUI-logged-in user found");
        }
        Ok(name)
    }
}

/// Spawns `path` as the user of `session_id`, in their environment and desktop.
pub fn spawn_as_session(path: &str, session_id: u32, debug: bool) -> Result<()> {
    // WTSQueryUserToken -> DuplicateTokenEx -> CreateProcessAsUserW.
    unsafe {
        let mut user_token = HANDLE::default();
        WTSQueryUserToken(session_id, &mut user_token)
            .map_err(|e| eyre::eyre!("WTSQueryUserToken failed: {e}"))?;

        let mut token = HANDLE::default();
        let dup = DuplicateTokenEx(
            user_token,
            TOKEN_ALL_ACCESS,
            None,
            SecurityImpersonation,
            TokenPrimary,
            &mut token,
        );
        let _ = CloseHandle(user_token);
        dup.map_err(|e| eyre::eyre!("DuplicateTokenEx failed: {e}"))?;

        let result = spawn_with_token(token, path, debug);
        let _ = CloseHandle(token);
        result
    }
}

unsafe fn spawn_with_token(token: HANDLE, path: &str, debug: bool) -> Result<()> {
    let mut env_ptr: *mut c_void = null_mut();
    unsafe { CreateEnvironmentBlock(&mut env_ptr, Some(token), false) }
        .map_err(|e| eyre::eyre!("CreateEnvironmentBlock failed: {e}"))?;
    let env = unsafe { build_environment(env_ptr as *const u16, debug) };
    let _ = unsafe { DestroyEnvironmentBlock(env_ptr) };

    let app: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut desktop: Vec<u16> = DESKTOP.encode_utf16().chain(std::iter::once(0)).collect();

    let mut si = STARTUPINFOW {
        cb: size_of::<STARTUPINFOW>() as u32,
        lpDesktop: PWSTR(desktop.as_mut_ptr()),
        dwFlags: STARTF_USESHOWWINDOW,
        wShowWindow: SW_SHOW,
        ..Default::default()
    };
    let mut pi = PROCESS_INFORMATION::default();

    unsafe {
        CreateProcessAsUserW(
            Some(token),
            PCWSTR(app.as_ptr()),
            None,
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(CREATE_UNICODE_ENVIRONMENT.0 | CREATE_NEW_CONSOLE.0),
            Some(env.as_ptr() as *const c_void),
            PCWSTR::null(),
            &mut si,
            &mut pi,
        )
    }
    .map_err(|e| eyre::eyre!("CreateProcessAsUser failed: {e}"))?;

    unsafe {
        let _ = CloseHandle(pi.hProcess);
        let _ = CloseHandle(pi.hThread);
    }
    Ok(())
}

/// Copies the user's env block (`KEY=VALUE\0`-joined, double-null terminated)
/// and appends the agent's own variables.
unsafe fn build_environment(ptr: *const u16, debug: bool) -> Vec<u16> {
    let mut block = Vec::new();
    let mut i = 0isize;
    unsafe {
        loop {
            let c = *ptr.offset(i);
            let n = *ptr.offset(i + 1);
            block.push(c);
            if c == 0 && n == 0 {
                break;
            }
            i += 1;
        }
    }
    let mut push_var = |kv: &str| {
        block.extend(kv.encode_utf16());
        block.push(0);
    };
    push_var("AK_AGENT_SUPERVISED=true");
    if debug {
        push_var("AK_AGENT_DEBUG=true");
    }
    block.push(0);
    block
}
