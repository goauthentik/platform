//! Narrow seams around the handful of Windows calls that have real side
//! effects, so the logic that decides *when* to call them can be unit
//! tested against a fake instead of the OS.

use ak_platform_keyring::{KeyringError, windows::WindowsStore};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::{
    Win32::{
        Foundation::{
            E_FAIL, ERROR_LOGON_FAILURE, ERROR_PASSWORD_EXPIRED, ERROR_PASSWORD_MUST_CHANGE,
        },
        NetworkManagement::NetManagement::{NetUserChangePassword, NetUserSetInfo, USER_INFO_1003},
        Security::{
            Authentication::Identity::{
                LSA_STRING, LsaConnectUntrusted, LsaDeregisterLogonProcess,
                LsaLookupAuthenticationPackage,
            },
            DuplicateTokenEx, LOGON32_LOGON_NETWORK, LOGON32_PROVIDER_DEFAULT, LogonUserW,
            SecurityImpersonation, TOKEN_ACCESS_MASK, TOKEN_ALL_ACCESS, TOKEN_ASSIGN_PRIMARY,
            TOKEN_DUPLICATE, TOKEN_QUERY, TokenPrimary,
        },
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        System::RemoteDesktop::{
            ProcessIdToSessionId, WTSGetActiveConsoleSessionId, WTSQueryUserToken,
        },
        System::Threading::{
            GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_QUERY_INFORMATION,
        },
    },
    core::{HRESULT, PCWSTR, PSTR, w},
};

pub trait AuthPackageLookup {
    fn negotiate_package(&self) -> windows::core::Result<u32>;
}

pub trait LocalAccountPassword {
    /// Administrative reset. Orphans the account's DPAPI master key — stored
    /// passwords, EFS files and personal certificates become unreadable — so
    /// this is only for first use and for recovering an account whose password
    /// we no longer know.
    fn reset(&self, username: &str, password: &str) -> windows::core::Result<()>;

    /// Self-service change. Supplying the old password lets LSA re-encrypt the
    /// DPAPI master key instead of orphaning it, which is why rotation goes
    /// through here and not through `reset`.
    fn change(&self, username: &str, old: &str, new: &str) -> windows::core::Result<()>;

    /// Whether `password` is still this account's password. Only codes that
    /// say something definite about the credential produce an `Ok`; anything
    /// else is `Err`, because "could not tell" must not be read as "wrong
    /// password".
    fn validate(&self, username: &str, password: &str) -> windows::core::Result<PasswordCheck>;
}

#[derive(Debug, PartialEq, Eq)]
pub enum PasswordCheck {
    Valid,
    /// Correct, but the account will not accept it for logon until it changes.
    Expired,
    Rejected,
}

/// Where the local account's password is kept between sign-ins.
pub trait PasswordStore {
    fn load(&self, sid: &str) -> eyre::Result<Option<String>>;
    fn save(&self, sid: &str, password: &str) -> eyre::Result<()>;
}

pub struct RealSyscalls;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

impl AuthPackageLookup for RealSyscalls {
    fn negotiate_package(&self) -> windows::core::Result<u32> {
        let mut lsa_handle = HANDLE::default();
        unsafe { LsaConnectUntrusted(&mut lsa_handle) }.ok()?;

        // `LSA_STRING` is counted, but `MaximumLength` is expected to cover a
        // trailing NUL — that is what `LsaInitString` produces for a C literal.
        // Claiming `len + 1` over a buffer that has no terminator overruns it.
        let name = b"Negotiate\0";
        let lsa_name = LSA_STRING {
            Length: (name.len() - 1) as u16,
            MaximumLength: name.len() as u16,
            Buffer: PSTR(name.as_ptr() as *mut u8),
        };

        let mut auth_package = 0u32;
        let status =
            unsafe { LsaLookupAuthenticationPackage(lsa_handle, &lsa_name, &mut auth_package) };
        unsafe {
            let _ = LsaDeregisterLogonProcess(lsa_handle);
        }

        // `NTSTATUS::ok()` is only a sign test, so any informational status
        // would pass while leaving `auth_package` at the 0 it was initialised
        // to, and we would hand LogonUI a package id that LSA never wrote.
        // Only STATUS_SUCCESS means the out-param is meaningful.
        if status.0 != 0 {
            return Err(status.to_hresult().into());
        }
        log::debug!("LSA resolved the Negotiate authentication package to {auth_package}");
        Ok(auth_package)
    }
}

impl LocalAccountPassword for RealSyscalls {
    fn reset(&self, username: &str, password: &str) -> windows::core::Result<()> {
        let username_wide = wide(username);
        let password_wide = wide(password);

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
            return Err(net_api_error(status));
        }
        Ok(())
    }

    fn change(&self, username: &str, old: &str, new: &str) -> windows::core::Result<()> {
        let username_wide = wide(username);
        let old_wide = wide(old);
        let new_wide = wide(new);

        let status = unsafe {
            NetUserChangePassword(
                PCWSTR::null(),
                PCWSTR(username_wide.as_ptr()),
                PCWSTR(old_wide.as_ptr()),
                PCWSTR(new_wide.as_ptr()),
            )
        };

        if status != 0 {
            return Err(net_api_error(status));
        }
        Ok(())
    }

    fn validate(&self, username: &str, password: &str) -> windows::core::Result<PasswordCheck> {
        let username_wide = wide(username);
        let password_wide = wide(password);

        // A network logon validates the credential without building a session,
        // and `.` scopes the lookup to this machine's account database.
        let mut token = HANDLE::default();
        let result = unsafe {
            LogonUserW(
                PCWSTR(username_wide.as_ptr()),
                w!("."),
                PCWSTR(password_wide.as_ptr()),
                LOGON32_LOGON_NETWORK,
                LOGON32_PROVIDER_DEFAULT,
                &mut token,
            )
        };

        match result {
            Ok(()) => {
                unsafe {
                    let _ = CloseHandle(token);
                }
                Ok(PasswordCheck::Valid)
            }
            // Anything not listed here — a policy denying network logons, a
            // locked-out or disabled account — means the check was
            // inconclusive, not that the password is wrong, and the caller
            // must not reset on the strength of it.
            Err(e) if e.code() == ERROR_LOGON_FAILURE.to_hresult() => Ok(PasswordCheck::Rejected),
            Err(e)
                if e.code() == ERROR_PASSWORD_EXPIRED.to_hresult()
                    || e.code() == ERROR_PASSWORD_MUST_CHANGE.to_hresult() =>
            {
                Ok(PasswordCheck::Expired)
            }
            Err(e) => Err(e),
        }
    }
}

/// `NET_API_STATUS` codes are Win32 error codes, so they survive the trip
/// through `HRESULT` and stay readable in the log.
fn net_api_error(status: u32) -> windows::core::Error {
    windows::core::Error::from_hresult(HRESULT::from_win32(status))
}

/// Keyring-backed [`PasswordStore`], keyed by SID so renaming the account does
/// not orphan the entry.
///
/// This reaches for [`WindowsStore`] rather than `ak_platform_keyring::store()`
/// because the latter resolves to the in-memory store under `debug_assertions`,
/// which would silently lose the password between LogonUI processes in every
/// development build. Local-machine persistence keeps a secret that is
/// meaningless off this box from roaming to a domain.
pub struct KeyringPasswordStore {
    store: WindowsStore,
    service: String,
}

impl Default for KeyringPasswordStore {
    fn default() -> Self {
        KeyringPasswordStore::new()
    }
}

impl KeyringPasswordStore {
    pub fn new() -> Self {
        KeyringPasswordStore {
            store: WindowsStore::new_local_machine(),
            service: ak_platform_keyring::service("wcp-account-password"),
        }
    }
}

impl PasswordStore for KeyringPasswordStore {
    fn load(&self, sid: &str) -> eyre::Result<Option<String>> {
        match self.store.get_blocking(&self.service, sid) {
            Ok(password) => Ok(Some(password)),
            Err(KeyringError::NotFound()) => Ok(None),
            Err(e) => Err(eyre::eyre!("{e}")),
        }
    }

    fn save(&self, sid: &str, password: &str) -> eyre::Result<()> {
        self.store
            .set_blocking(&self.service, sid, password)
            .map_err(|e| eyre::eyre!("{e}"))
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
            log::error!("no active console session");
            return Err(windows::core::Error::from(E_FAIL));
        }

        let mut token = HANDLE::default();
        if WTSQueryUserToken(session, &mut token).is_ok() {
            return Ok(token);
        }

        // This provider is loaded into the process drawing the logon UI, so its
        // own session is the one the person is signing in to. That should be
        // the console session; log it when it is not, because then the console
        // session is the wrong thing to be looking for winlogon in.
        let mut own_session = 0u32;
        if ProcessIdToSessionId(GetCurrentProcessId(), &mut own_session).is_ok()
            && own_session != session
        {
            log::warn!(
                "console session is {session} but this provider is in session {own_session}"
            );
        }

        winlogon_token_for_session(session)
    }
}

/// There is one `winlogon.exe` per session, so the snapshot has to be searched
/// to the end: stopping at the first one found gives up as soon as the
/// enumeration happens to reach another session's copy first, which is what a
/// logoff/logon cycle produces once the console session id has moved on.
fn winlogon_token_for_session(session_id: u32) -> windows::core::Result<HANDLE> {
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut result = Err(windows::core::Error::from(E_FAIL));
        let mut sessions_seen = Vec::new();

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
                    if ProcessIdToSessionId(entry.th32ProcessID, &mut proc_session).is_ok() {
                        sessions_seen.push(proc_session);
                        if proc_session == session_id {
                            match duplicate_process_primary_token(entry.th32ProcessID) {
                                Ok(dup) => {
                                    result = Ok(dup);
                                    break;
                                }
                                // Keep looking: another instance in the same
                                // session may still hand one over.
                                Err(e) => log::warn!(
                                    "could not duplicate the token of winlogon.exe \
                                     (pid {}, session {proc_session}): {e}",
                                    entry.th32ProcessID
                                ),
                            }
                        }
                    }
                }

                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snap);
        if result.is_err() {
            log::warn!(
                "no usable winlogon.exe token for console session {session_id}; \
                 saw winlogon in sessions {sessions_seen:?}"
            );
        }
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
