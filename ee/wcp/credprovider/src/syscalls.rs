//! Narrow seams around the handful of Windows calls that have real side
//! effects, so the logic that decides *when* to call them can be unit
//! tested against a fake instead of the OS.

use std::ffi::c_void;

use ak_platform_keyring::{KeyringError, windows::WindowsStore};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetForegroundWindow, GetWindowRect,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
};
use windows::{
    Win32::{
        Foundation::{
            ERROR_LOGON_FAILURE, ERROR_NOT_ALL_ASSIGNED, ERROR_PASSWORD_EXPIRED,
            ERROR_PASSWORD_MUST_CHANGE, GENERIC_ALL, GetLastError, HLOCAL, LUID, LocalFree,
        },
        NetworkManagement::NetManagement::{NetUserChangePassword, NetUserSetInfo, USER_INFO_1003},
        Security::{
            ACL, AdjustTokenPrivileges, AllocateLocallyUniqueId,
            Authentication::Identity::{
                LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LSA_STRING, LSA_UNICODE_STRING,
                LsaAddAccountRights, LsaClose, LsaConnectUntrusted, LsaDeregisterLogonProcess,
                LsaFreeReturnBuffer, LsaLogonUser, LsaLookupAuthenticationPackage, LsaOpenPolicy,
                LsaRegisterLogonProcess, MSV1_0_S4U_LOGON, MsV1_0S4ULogon, POLICY_CREATE_ACCOUNT,
                SECURITY_LOGON_TYPE,
            },
            Authorization::{
                EXPLICIT_ACCESS_W, GetSecurityInfo, NO_MULTIPLE_TRUSTEE, SE_OBJECT_TYPE,
                SE_WINDOW_OBJECT, SET_ACCESS, SetEntriesInAclW, SetSecurityInfo, TRUSTEE_IS_SID,
                TRUSTEE_IS_USER, TRUSTEE_W,
            },
            DACL_SECURITY_INFORMATION, LOGON32_LOGON_NETWORK, LOGON32_PROVIDER_DEFAULT,
            LUID_AND_ATTRIBUTES, LogonUserW, LookupAccountNameW, LookupPrivilegeValueW,
            NO_INHERITANCE, PSECURITY_DESCRIPTOR, PSID, QUOTA_LIMITS, SE_PRIVILEGE_ENABLED,
            SE_TCB_NAME, SID_NAME_USE, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_SOURCE,
        },
        Storage::FileSystem::{READ_CONTROL, WRITE_DAC},
        System::StationsAndDesktops::{
            DESKTOP_CONTROL_FLAGS, GetProcessWindowStation, OpenDesktopW,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
    core::{HRESULT, PCWSTR, PSTR, PWSTR, w},
};
use windows_core::BOOL;

const SERVICE_ACCOUNT_STATE_KEY: &str =
    "SOFTWARE\\authentik Security Inc.\\Platform\\WcpServiceAccount";

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

/// The window-manager calls behind pushing `ak_cef.exe`'s window to the front.
/// Windows are a bare `isize` rather than an `HWND` so the policy driving these
/// can be tested against a fake without a desktop to enumerate.
pub trait ForegroundControl {
    fn foreground_pid(&self) -> Option<u32>;
    fn visible_top_level_window(&self, pid: u32) -> Option<isize>;
    fn allow_set_foreground(&self, pid: u32) -> bool;
    fn set_foreground(&self, window: isize) -> bool;
}

pub struct RealSyscalls;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// `LSA_STRING` is counted, but `MaximumLength` is expected to cover a
/// trailing NUL — that is what `LsaInitString` produces for a C literal.
/// Claiming `len + 1` over a buffer that has no terminator overruns it, so
/// `name` must end in `\0`.
fn lsa_string(name: &'static [u8]) -> LSA_STRING {
    LSA_STRING {
        Length: (name.len() - 1) as u16,
        MaximumLength: name.len() as u16,
        Buffer: PSTR(name.as_ptr() as *mut u8),
    }
}

impl ForegroundControl for RealSyscalls {
    fn foreground_pid(&self) -> Option<u32> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return None;
        }
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        (pid != 0).then_some(pid)
    }

    fn visible_top_level_window(&self, pid: u32) -> Option<isize> {
        let mut search = WindowSearch { pid, found: 0 };
        // `EnumWindows` walks the *calling thread's* desktop, so this only
        // works from one of LogonUI's own threads. It reports failure when the
        // callback stops the walk early, which is what finding a match does,
        // so `found` is the answer rather than the result.
        let _ = unsafe {
            EnumWindows(
                Some(find_window_of_process),
                LPARAM(std::ptr::from_mut(&mut search) as isize),
            )
        };
        (search.found != 0).then_some(search.found)
    }

    fn allow_set_foreground(&self, pid: u32) -> bool {
        match unsafe { AllowSetForegroundWindow(pid) } {
            Ok(()) => true,
            Err(e) => {
                log::debug!("AllowSetForegroundWindow({pid}) failed: {e}");
                false
            }
        }
    }

    fn set_foreground(&self, window: isize) -> bool {
        unsafe { SetForegroundWindow(HWND(window as *mut c_void)) }.as_bool()
    }
}

struct WindowSearch {
    pid: u32,
    found: isize,
}

/// Stops at the first window a person could see. Chromium opens several
/// message-only and zero-size helpers first, and handing one of those to
/// `SetForegroundWindow` would look like success while leaving the real window
/// where it was.
unsafe extern "system" fn find_window_of_process(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = unsafe { &mut *(lparam.0 as *mut WindowSearch) };

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid != search.pid || !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return true.into();
    }

    let mut rect = RECT::default();
    if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err()
        || rect.right <= rect.left
        || rect.bottom <= rect.top
    {
        return true.into();
    }

    search.found = hwnd.0 as isize;
    false.into()
}

impl AuthPackageLookup for RealSyscalls {
    fn negotiate_package(&self) -> windows::core::Result<u32> {
        let mut lsa_handle = HANDLE::default();
        unsafe { LsaConnectUntrusted(&mut lsa_handle) }.ok()?;

        let lsa_name = lsa_string(b"Negotiate\0");
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

        // `NET_API_STATUS` is a Win32 error code, so it stays readable.
        if status != 0 {
            return Err(HRESULT::from_win32(status).into());
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
            return Err(HRESULT::from_win32(status).into());
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

/// Keyring-backed [`PasswordStore`], keyed by SID so renaming the account does
/// not orphan the entry.
///
/// Uses [`WindowsStore`] directly rather than `ak_platform_keyring::store()`,
/// which resolves to the in-memory store under `debug_assertions` and would
/// lose the password between LogonUI processes in every development build.
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

/// Name of the dedicated local account `ak_cef.exe` runs as instead of
/// SYSTEM. Created by the installer (`vpkg/windows/Package.wxs`'s
/// `util:User`) — keep this in step with that element's `Name` attribute.
pub const SERVICE_ACCOUNT_NAME: &str = "ak-wcp-browser";

const TOKEN_SOURCE_NAME: [i8; 8] = [
    b'A' as i8, b'k' as i8, b'W' as i8, b'c' as i8, b'p' as i8, b'S' as i8, b'4' as i8, b'U' as i8,
];

/// Resolves the service account's name to a SID: both `LsaAddAccountRights`
/// and the desktop ACL grant below want one, and a name is all the installer
/// leaves behind.
pub fn account_sid(username: &str) -> windows::core::Result<Vec<u8>> {
    let username_wide = wide(username);
    // Large enough for any SID Windows issues (the practical maximum is well
    // under 68 bytes) and any domain name `LookupAccountNameW` might report.
    let mut sid = vec![0u8; 256];
    let mut sid_len = sid.len() as u32;
    let mut domain = [0u16; 256];
    let mut domain_len = domain.len() as u32;
    let mut use_ = SID_NAME_USE::default();
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(username_wide.as_ptr()),
            Some(PSID(sid.as_mut_ptr() as *mut _)),
            &mut sid_len,
            Some(PWSTR(domain.as_mut_ptr())),
            &mut domain_len,
            &mut use_,
        )?;
    }
    sid.truncate(sid_len as usize);
    Ok(sid)
}

fn lsa_unicode_string(wide: &[u16]) -> LSA_UNICODE_STRING {
    let bytes = (wide.len() * 2) as u16;
    LSA_UNICODE_STRING {
        Length: bytes,
        MaximumLength: bytes,
        Buffer: PWSTR(wide.as_ptr() as *mut u16),
    }
}

/// SYSTEM's token holds every privilege this file needs, but — like any
/// privilege not `SE_PRIVILEGE_ENABLED_BY_DEFAULT` — disabled until asked
/// for; the APIs that need one check it as active, not merely present.
pub fn enable_privilege(name: PCWSTR) -> windows::core::Result<()> {
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES, &mut token)?;

        let mut luid = LUID::default();
        let result = (|| {
            LookupPrivilegeValueW(PCWSTR::null(), name, &mut luid)?;
            let privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES {
                    Luid: luid,
                    Attributes: SE_PRIVILEGE_ENABLED,
                }],
            };
            AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None)?;
            // The call above reports success even when the privilege was not
            // actually held to enable — a classic trap, and indistinguishable
            // from a real success without this check.
            if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
                return Err(windows::core::Error::from(HRESULT::from_win32(
                    ERROR_NOT_ALL_ASSIGNED.0,
                )));
            }
            Ok(())
        })();

        let _ = CloseHandle(token);
        result
    }
}

/// Mints a primary token for the service account via an S4U logon — no
/// password needed, only `SE_TCB_NAME`. `Service` is the logon type
/// deliberately: it is the one type `deny_interactive_and_network_logon`
/// does not deny.
pub fn service_account_token(username: &str) -> windows::core::Result<HANDLE> {
    unsafe {
        enable_privilege(SE_TCB_NAME)?;

        let process_name = lsa_string(b"ak_cred_provider\0");
        let mut lsa_handle = HANDLE::default();
        let mut security_mode = 0u32;
        LsaRegisterLogonProcess(&process_name, &mut lsa_handle, &mut security_mode).ok()?;

        let result = s4u_logon(lsa_handle, username);
        let _ = LsaDeregisterLogonProcess(lsa_handle);
        result
    }
}

unsafe fn s4u_logon(lsa_handle: HANDLE, username: &str) -> windows::core::Result<HANDLE> {
    unsafe {
        let package_name = lsa_string(b"MICROSOFT_AUTHENTICATION_PACKAGE_V1_0\0");
        let mut auth_package = 0u32;
        LsaLookupAuthenticationPackage(lsa_handle, &package_name, &mut auth_package).ok()?;

        // S4U wants these as `LSA_UNICODE_STRING`s with no trailing NUL,
        // unlike the `LSA_STRING`s above — keep the backing buffers alive
        // for the whole call, `lsa_unicode_string` only borrows them.
        let username_wide: Vec<u16> = username.encode_utf16().collect();
        let domain_wide: Vec<u16> = ".".encode_utf16().collect();
        let s4u = MSV1_0_S4U_LOGON {
            MessageType: MsV1_0S4ULogon,
            Flags: 0,
            UserPrincipalName: lsa_unicode_string(&username_wide),
            DomainName: lsa_unicode_string(&domain_wide),
        };

        let origin_name = lsa_string(b"ak_cred_provider\0");
        let mut token_source = TOKEN_SOURCE {
            SourceName: TOKEN_SOURCE_NAME,
            ..Default::default()
        };
        AllocateLocallyUniqueId(&mut token_source.SourceIdentifier)?;

        let mut profile_buffer: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut profile_buffer_len = 0u32;
        let mut logon_id = LUID::default();
        let mut token = HANDLE::default();
        let mut quotas = QUOTA_LIMITS::default();
        let mut sub_status = 0i32;

        let status = LsaLogonUser(
            lsa_handle,
            &origin_name,
            SECURITY_LOGON_TYPE::Service,
            auth_package,
            &s4u as *const MSV1_0_S4U_LOGON as *const std::ffi::c_void,
            std::mem::size_of::<MSV1_0_S4U_LOGON>() as u32,
            None,
            &token_source,
            &mut profile_buffer,
            &mut profile_buffer_len,
            &mut logon_id,
            &mut token,
            &mut quotas,
            &mut sub_status,
        );

        if !profile_buffer.is_null() {
            let _ = LsaFreeReturnBuffer(profile_buffer);
        }

        // As with `negotiate_package`, `NTSTATUS::ok()` is only a sign test;
        // `sub_status` carries the more specific reason (e.g. an account
        // that does not exist yet because the installer has not run) and is
        // only meaningful once `status` itself is an error.
        if status.0 != 0 {
            log::error!("S4U logon for {username} failed: {status:?} (substatus {sub_status:#x})");
            return Err(status.to_hresult().into());
        }
        Ok(token)
    }
}

/// Adds `sid` to `handle`'s DACL with `GENERIC_ALL`, preserving every
/// existing entry — `SetEntriesInAclW` merges onto `old_dacl` rather than
/// replacing it, which matters here: replacing outright would drop
/// SYSTEM/Administrators access to the object this process itself needs.
unsafe fn grant_generic_all(
    handle: HANDLE,
    object_type: SE_OBJECT_TYPE,
    sid: &[u8],
) -> windows::core::Result<()> {
    unsafe {
        let mut old_dacl: *mut ACL = std::ptr::null_mut();
        let mut sd = PSECURITY_DESCRIPTOR::default();
        GetSecurityInfo(
            handle,
            object_type,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut old_dacl),
            None,
            Some(&mut sd),
        )
        .ok()?;

        let trustee = TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_USER,
            ptstrName: PWSTR(sid.as_ptr() as *mut u16),
        };
        let entry = EXPLICIT_ACCESS_W {
            grfAccessPermissions: GENERIC_ALL.0,
            grfAccessMode: SET_ACCESS,
            grfInheritance: NO_INHERITANCE,
            Trustee: trustee,
        };

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let entries_result = SetEntriesInAclW(Some(&[entry]), Some(old_dacl), &mut new_dacl);
        let set_result = if entries_result.is_ok() {
            SetSecurityInfo(
                handle,
                object_type,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(new_dacl),
                None,
            )
        } else {
            entries_result
        };

        let _ = LocalFree(Some(HLOCAL(sd.0)));
        if !new_dacl.is_null() {
            let _ = LocalFree(Some(HLOCAL(new_dacl as *mut std::ffi::c_void)));
        }

        set_result.ok()
    }
}

/// Grants the service account's SID access to the secure desktop — a
/// non-SYSTEM token has none by default. Deliberate; see `BROWSER_PRIVILEGE.md`
/// for the tradeoff. Idempotent, and only correct when called from inside
/// LogonUI's own process: `GetProcessWindowStation`/`OpenDesktopW` resolve
/// relative to the *caller's* window station, `WinSta0` here.
pub fn ensure_desktop_access(sid: &[u8]) -> windows::core::Result<()> {
    unsafe {
        let winsta = GetProcessWindowStation()?;
        grant_generic_all(HANDLE(winsta.0), SE_WINDOW_OBJECT, sid)?;

        let desktop = OpenDesktopW(
            w!("Winlogon"),
            DESKTOP_CONTROL_FLAGS(0),
            false,
            (READ_CONTROL | WRITE_DAC).0,
        )?;
        grant_generic_all(HANDLE(desktop.0), SE_WINDOW_OBJECT, sid)
    }
}

/// Denies the service account the logon types that would let it sign someone
/// in — it must not be usable at the very screen it serves. `Service`,
/// what `service_account_token` uses, is deliberately not among these.
/// `LsaAddAccountRights` is itself idempotent, so this is safe on every load.
pub fn deny_interactive_and_network_logon(sid: &[u8]) -> windows::core::Result<()> {
    const RIGHTS: [&str; 3] = [
        "SeDenyInteractiveLogonRight",
        "SeDenyNetworkLogonRight",
        "SeDenyRemoteInteractiveLogonRight",
    ];
    let wide_rights: Vec<Vec<u16>> = RIGHTS.iter().map(|r| r.encode_utf16().collect()).collect();
    let lsa_rights: Vec<LSA_UNICODE_STRING> =
        wide_rights.iter().map(|w| lsa_unicode_string(w)).collect();

    unsafe {
        let mut policy_handle = LSA_HANDLE::default();
        let object_attrs = LSA_OBJECT_ATTRIBUTES::default();
        LsaOpenPolicy(
            None,
            &object_attrs,
            POLICY_CREATE_ACCOUNT as u32,
            &mut policy_handle,
        )
        .ok()?;

        let status = LsaAddAccountRights(policy_handle, PSID(sid.as_ptr() as *mut _), &lsa_rights);
        let _ = LsaClose(policy_handle);
        status.ok()
    }
}

/// Rotates the service account's password to a random value the first time
/// this runs, then never again — nothing needs it back, since
/// `service_account_token` mints tokens via S4U, not `LogonUserW`. Turns the
/// installer's fixed placeholder password into a real secret exactly once;
/// the `HKLM` marker is what stops it happening again on every logon.
pub fn ensure_service_account_password_rotated(username: &str) -> eyre::Result<()> {
    let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let (key, _disp) = hklm.create_subkey(SERVICE_ACCOUNT_STATE_KEY)?;

    if key.get_value::<u32, _>("PasswordRotated").unwrap_or(0) == 1 {
        return Ok(());
    }

    let password = crate::helpers::generate_random_password().map_err(|e| eyre::eyre!("{e}"))?;
    RealSyscalls
        .reset(username, &password)
        .map_err(|e| eyre::eyre!("{e}"))?;
    key.set_value("PasswordRotated", &1u32)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod s4u_tests {
    use super::*;

    /// `MaximumLength` must cover the trailing NUL while `Length` excludes
    /// it — get this backwards and `LsaLookupAuthenticationPackage` either
    /// truncates the last real character or reads one byte past the buffer.
    #[test]
    fn lsa_string_length_excludes_the_trailing_nul() {
        let s = lsa_string(b"Negotiate\0");
        assert_eq!(s.Length, 9);
        assert_eq!(s.MaximumLength, 10);
        let bytes = unsafe { std::slice::from_raw_parts(s.Buffer.0, s.Length as usize) };
        assert_eq!(bytes, b"Negotiate");
    }

    /// Unlike `LSA_STRING`, S4U's `LSA_UNICODE_STRING`s carry no NUL at all —
    /// `Length`/`MaximumLength` are both the exact UTF-16 byte count.
    #[test]
    fn lsa_unicode_string_round_trips_without_a_nul() {
        let wide: Vec<u16> = "ak-wcp-browser".encode_utf16().collect();
        let s = lsa_unicode_string(&wide);
        assert_eq!(s.Length as usize, wide.len() * 2);
        assert_eq!(s.MaximumLength, s.Length);
        let read = unsafe {
            String::from_utf16_lossy(std::slice::from_raw_parts(
                s.Buffer.0,
                (s.Length / 2) as usize,
            ))
        };
        assert_eq!(read, "ak-wcp-browser");
    }
}
