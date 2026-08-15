//! Narrow seams around the handful of Windows calls that have real side
//! effects, so the logic that decides *when* to call them can be unit
//! tested against a fake instead of the OS.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::{
    Win32::{
        Foundation::E_FAIL,
        NetworkManagement::NetManagement::{NetUserSetInfo, USER_INFO_1003},
        Security::{
            Authentication::Identity::{
                LSA_STRING, LsaConnectUntrusted, LsaDeregisterLogonProcess,
                LsaLookupAuthenticationPackage,
            },
            DuplicateTokenEx, SecurityImpersonation, TOKEN_ACCESS_MASK, TOKEN_ALL_ACCESS,
            TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, TokenPrimary,
        },
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        System::RemoteDesktop::{
            ProcessIdToSessionId, WTSGetActiveConsoleSessionId, WTSQueryUserToken,
        },
        System::Threading::{OpenProcess, OpenProcessToken, PROCESS_QUERY_INFORMATION},
    },
    core::{PCWSTR, PSTR},
};

pub trait AuthPackageLookup {
    fn negotiate_package(&self) -> windows::core::Result<u32>;
}

pub trait LocalAccountPasswordReset {
    fn reset(&self, username: &str, password: &str) -> windows::core::Result<()>;
}

pub struct RealSyscalls;

impl AuthPackageLookup for RealSyscalls {
    fn negotiate_package(&self) -> windows::core::Result<u32> {
        let mut lsa_handle = HANDLE::default();
        unsafe { LsaConnectUntrusted(&mut lsa_handle) }.ok()?;

        let name = b"Negotiate";
        let lsa_name = LSA_STRING {
            Length: name.len() as u16,
            MaximumLength: (name.len() + 1) as u16,
            Buffer: PSTR(name.as_ptr() as *mut u8),
        };

        let mut auth_package = 0u32;
        let result =
            unsafe { LsaLookupAuthenticationPackage(lsa_handle, &lsa_name, &mut auth_package) };
        unsafe {
            let _ = LsaDeregisterLogonProcess(lsa_handle);
        }
        result.ok()?;
        Ok(auth_package)
    }
}

impl LocalAccountPasswordReset for RealSyscalls {
    fn reset(&self, username: &str, password: &str) -> windows::core::Result<()> {
        let username_wide: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
        let password_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

        let info = USER_INFO_1003 {
            usri1003_password: windows::core::PWSTR(password_wide.as_ptr() as *mut u16),
        };

        let status = unsafe {
            NetUserSetInfo(
                PCWSTR::null(),
                PCWSTR(username_wide.as_ptr()),
                1003,
                &info as *const USER_INFO_1003 as *const u8,
                None,
            )
        };

        if status != 0 {
            return Err(windows::core::Error::from(E_FAIL));
        }
        Ok(())
    }
}

/// Acquire a primary token for the active console (interactive) session, so
/// `ak_cef.exe` can be launched there rather than in LogonUI's Session 0.
///
/// `WTSQueryUserToken` works once a user token exists (unlock scenario); on
/// a fresh logon no such token exists yet, so this falls back to duplicating
/// `winlogon.exe`'s own token in that session.
pub fn acquire_interactive_token() -> windows::core::Result<HANDLE> {
    unsafe {
        let session = WTSGetActiveConsoleSessionId();
        if session == 0xFFFF_FFFF {
            return Err(windows::core::Error::from(E_FAIL));
        }

        let mut token = HANDLE::default();
        if WTSQueryUserToken(session, &mut token).is_ok() {
            return Ok(token);
        }

        winlogon_token_for_session(session)
    }
}

fn winlogon_token_for_session(session_id: u32) -> windows::core::Result<HANDLE> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut result = Err(windows::core::Error::from(E_FAIL));

        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let nul = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..nul]);

                if name.eq_ignore_ascii_case("winlogon.exe") {
                    let mut proc_session = 0u32;
                    if ProcessIdToSessionId(entry.th32ProcessID, &mut proc_session).is_ok()
                        && proc_session == session_id
                        && let Ok(dup) = duplicate_process_primary_token(entry.th32ProcessID)
                    {
                        result = Ok(dup);
                    }
                    break;
                }

                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snap);
        result
    }
}

fn duplicate_process_primary_token(pid: u32) -> windows::core::Result<HANDLE> {
    unsafe {
        let hproc = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid)?;
        let access = TOKEN_ACCESS_MASK(TOKEN_DUPLICATE.0 | TOKEN_QUERY.0 | TOKEN_ASSIGN_PRIMARY.0);
        let mut raw = HANDLE::default();
        let opened = OpenProcessToken(hproc, access, &mut raw);
        if opened.is_err() {
            let _ = CloseHandle(hproc);
            return Err(windows::core::Error::from(E_FAIL));
        }

        let mut dup = HANDLE::default();
        let result = DuplicateTokenEx(
            raw,
            TOKEN_ALL_ACCESS,
            None,
            SecurityImpersonation,
            TokenPrimary,
            &mut dup,
        );
        let _ = CloseHandle(raw);
        let _ = CloseHandle(hproc);
        result?;
        Ok(dup)
    }
}
