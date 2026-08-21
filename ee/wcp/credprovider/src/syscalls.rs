//! Narrow seams around the handful of Windows calls that have real side
//! effects, so the logic that decides *when* to call them can be unit
//! tested against a fake instead of the OS.

use std::ffi::c_void;

use ak_platform_keyring::{KeyringError, windows::WindowsStore};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, EnumWindows, GetForegroundWindow, GetWindowRect, GetWindowTextW,
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
            ACL, AdjustTokenPrivileges,
            Authentication::Identity::{
                LSA_HANDLE, LSA_OBJECT_ATTRIBUTES, LSA_STRING, LSA_UNICODE_STRING,
                LsaAddAccountRights, LsaClose, LsaConnectUntrusted, LsaDeregisterLogonProcess,
                LsaLookupAuthenticationPackage, LsaOpenPolicy, POLICY_CREATE_ACCOUNT,
            },
            Authorization::{
                ConvertSidToStringSidW, EXPLICIT_ACCESS_W, GetSecurityInfo, NO_MULTIPLE_TRUSTEE,
                SE_OBJECT_TYPE, SE_WINDOW_OBJECT, SET_ACCESS, SetEntriesInAclW, SetSecurityInfo,
                TRUSTEE_IS_SID, TRUSTEE_IS_USER, TRUSTEE_W,
            },
            CreateRestrictedToken, DACL_SECURITY_INFORMATION, DISABLE_MAX_PRIVILEGE,
            GetTokenInformation, LOGON32_LOGON_NETWORK, LOGON32_LOGON_SERVICE,
            LOGON32_PROVIDER_DEFAULT, LUID_AND_ATTRIBUTES, LogonUserW, LookupAccountNameW,
            LookupAccountSidW, LookupPrivilegeValueW, NO_INHERITANCE, PSECURITY_DESCRIPTOR, PSID,
            SE_PRIVILEGE_ENABLED, SID_NAME_USE, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
            TOKEN_QUERY, TOKEN_USER, TokenUser,
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

/// The window-manager calls behind pushing `ak_browser.exe`'s window to the front.
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

/// Stops at the sign-in window, matched on its title.
///
/// Not the first visible window in the process: WebView2 puts up a
/// `WS_EX_NOACTIVATE` tool window long before the sign-in window is revealed,
/// and `SetForegroundWindow` can never activate that one, so nudging it spends
/// the `AllowSetForegroundWindow` grant for nothing.
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

    let mut title = [0u16; 128];
    let len = unsafe { GetWindowTextW(hwnd, &mut title) };
    if len <= 0 || String::from_utf16_lossy(&title[..len as usize]) != ak_ee_wcp_wire::WINDOW_TITLE
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

/// Name of the dedicated local account `ak_browser.exe` runs as instead of
/// SYSTEM. Created by the installer (`vpkg/windows/Package.wxs`'s
/// `util:User`) — keep this in step with that element's `Name` attribute.
pub const SERVICE_ACCOUNT_NAME: &str = "ak-wcp-browser";

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

/// Best-effort "who are we actually running as" for the log line right
/// before a privilege-enable failure. A privilege can be missing either
/// because policy genuinely denies it to this account, or because the
/// token isn't the SYSTEM token this whole design assumes it is — those
/// need different fixes, and the log line otherwise can't tell them apart.
fn current_token_identity() -> String {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return "<could not open process token>".to_string();
        }

        let mut buf = [0u8; 256];
        let mut ret_len = 0u32;
        let got_user = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr() as *mut c_void),
            buf.len() as u32,
            &mut ret_len,
        );

        let identity = if got_user.is_ok() {
            let sid = (*(buf.as_ptr() as *const TOKEN_USER)).User.Sid;
            let mut name = [0u16; 256];
            let mut name_len = name.len() as u32;
            let mut domain = [0u16; 256];
            let mut domain_len = domain.len() as u32;
            let mut use_ = SID_NAME_USE::default();
            if LookupAccountSidW(
                PCWSTR::null(),
                sid,
                Some(PWSTR(name.as_mut_ptr())),
                &mut name_len,
                Some(PWSTR(domain.as_mut_ptr())),
                &mut domain_len,
                &mut use_,
            )
            .is_ok()
            {
                format!(
                    "{}\\{}",
                    String::from_utf16_lossy(&domain[..domain_len as usize]),
                    String::from_utf16_lossy(&name[..name_len as usize])
                )
            } else {
                "<could not resolve the token's account name>".to_string()
            }
        } else {
            "<could not read the token's user>".to_string()
        };

        let _ = CloseHandle(token);
        identity
    }
}

/// SYSTEM's token holds every privilege this file needs, but — like any
/// privilege not `SE_PRIVILEGE_ENABLED_BY_DEFAULT` — disabled until asked
/// for; the APIs that need one check it as active, not merely present.
/// `display` is only for the log line on failure — `AdjustTokenPrivileges`
/// doesn't say which of the (here, always one) privileges it couldn't grant.
pub fn enable_privilege(name: PCWSTR, display: &str) -> windows::core::Result<()> {
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
                log::error!(
                    "{display} is not held by this token at all (running as {})",
                    current_token_identity()
                );
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

/// Mints a primary token for the service account by logging it on with its
/// stored password, then strips every privilege from the result — the same
/// pattern GCPW uses for its own LogonUI-hosted sign-in UI (`CreateLogonToken`
/// in `chrome/credential_provider/gaiacp/gcp_utils.cc`). `LOGON32_LOGON_SERVICE`,
/// not `_BATCH`: a batch-logon token cannot create named synchronization
/// objects, fatal to Chromium's own `ProcessSingleton` — see
/// `BROWSER_PRIVILEGE.md`'s "Roads not taken" for why.
pub fn service_account_token(username: &str, password: &str) -> windows::core::Result<HANDLE> {
    let username_wide = wide(username);
    let password_wide = wide(password);

    let mut primary = HANDLE::default();
    unsafe {
        LogonUserW(
            PCWSTR(username_wide.as_ptr()),
            w!("."),
            PCWSTR(password_wide.as_ptr()),
            LOGON32_LOGON_SERVICE,
            LOGON32_PROVIDER_DEFAULT,
            &mut primary,
        )?;
    }

    let mut restricted = HANDLE::default();
    let result = unsafe {
        CreateRestrictedToken(
            primary,
            DISABLE_MAX_PRIVILEGE,
            None,
            None,
            None,
            &mut restricted,
        )
    };
    unsafe {
        let _ = CloseHandle(primary);
    }
    result?;
    Ok(restricted)
}

/// String form of a SID, for the keyring's `sid` key. `account_sid` returns
/// raw bytes for the Win32 calls that want a `PSID`; the keyring store
/// (like the interactive user's, `credential.rs`) is keyed by the string
/// form instead.
pub(crate) fn sid_to_string(sid: &[u8]) -> windows::core::Result<String> {
    unsafe {
        let mut wide_sid = PWSTR(std::ptr::null_mut());
        ConvertSidToStringSidW(PSID(sid.as_ptr() as *mut _), &mut wide_sid)?;
        let len = (0..).take_while(|&i| *wide_sid.0.add(i) != 0).count();
        let result = String::from_utf16_lossy(std::slice::from_raw_parts(wide_sid.0, len));
        let _ = LocalFree(Some(HLOCAL(wide_sid.0 as *mut c_void)));
        Ok(result)
    }
}

/// Adds `sid` to `handle`'s DACL with `access_mask`, preserving every
/// existing entry — `SetEntriesInAclW` merges onto `old_dacl` rather than
/// replacing it, which matters here: replacing outright would drop
/// SYSTEM/Administrators access to the object this process itself needs.
unsafe fn grant_access(
    handle: HANDLE,
    object_type: SE_OBJECT_TYPE,
    sid: &[u8],
    access_mask: u32,
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
            grfAccessPermissions: access_mask,
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
        grant_access(HANDLE(winsta.0), SE_WINDOW_OBJECT, sid, GENERIC_ALL.0)?;

        let desktop = OpenDesktopW(
            w!("Winlogon"),
            DESKTOP_CONTROL_FLAGS(0),
            false,
            (READ_CONTROL | WRITE_DAC).0,
        )?;
        grant_access(HANDLE(desktop.0), SE_WINDOW_OBJECT, sid, GENERIC_ALL.0)
    }
}

/// Grants the service account's SID rights to create objects in this
/// session's `BaseNamedObjects` — the Object Manager directory Windows
/// resolves `Local\`-prefixed names into. No Win32 wrapper exists for
/// opening an arbitrary Object Manager directory the way `OpenDesktopW`
/// does for desktops, hence the native `NtOpenDirectoryObject` call.
/// Mirrors GCPW's own `AllowLogonSIDOnLocalBasedNamedObjects`
/// (`chrome/credential_provider/gaiacp/os_process_manager.cc`), including
/// its narrower-than-`GENERIC_ALL` mask — see `BROWSER_PRIVILEGE.md`.
pub fn ensure_base_named_objects_access(sid: &[u8]) -> windows::core::Result<()> {
    use windows::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows::Wdk::Storage::FileSystem::NtOpenDirectoryObject;
    use windows::Wdk::System::SystemServices::{
        DIRECTORY_CREATE_OBJECT, DIRECTORY_CREATE_SUBDIRECTORY, DIRECTORY_QUERY, DIRECTORY_TRAVERSE,
    };
    use windows::Win32::Foundation::UNICODE_STRING;
    use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
    use windows::Win32::System::Threading::GetCurrentProcessId;

    let mut session_id = 0u32;
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id)? };

    let path = if session_id == 0 {
        r"\BaseNamedObjects".to_string()
    } else {
        format!(r"\Sessions\{session_id}\BaseNamedObjects")
    };
    let path_wide = wide(&path);
    let mut name = UNICODE_STRING {
        Length: ((path_wide.len() - 1) * 2) as u16,
        MaximumLength: (path_wide.len() * 2) as u16,
        Buffer: PWSTR(path_wide.as_ptr() as *mut u16),
    };
    let object_attributes = OBJECT_ATTRIBUTES {
        Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
        ObjectName: &mut name,
        ..Default::default()
    };

    unsafe {
        let mut directory = HANDLE::default();
        NtOpenDirectoryObject(
            &mut directory,
            DIRECTORY_TRAVERSE | READ_CONTROL.0 | WRITE_DAC.0,
            &object_attributes,
        )
        .ok()?;

        let result = grant_access(
            directory,
            SE_WINDOW_OBJECT,
            sid,
            DIRECTORY_QUERY
                | DIRECTORY_TRAVERSE
                | DIRECTORY_CREATE_OBJECT
                | DIRECTORY_CREATE_SUBDIRECTORY,
        );
        let _ = CloseHandle(directory);
        result
    }
}

/// Denies the service account the logon types that would let it sign someone
/// in — it must not be usable at the very screen it serves. `Service`, what
/// `service_account_token` uses (`LOGON32_LOGON_SERVICE`), is deliberately
/// not among these. `LsaAddAccountRights` is itself idempotent, so this is
/// safe on every load.
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

/// The service account's password: established once and reused after that,
/// same state machine and same reasoning as the interactive user's own
/// account (`credential.rs::account_password`, `LOCAL_PASSWORD.md`) — an
/// administrative reset orphans the DPAPI master key, so once a password is
/// known, only `change` touches the account again. That reasoning is about
/// DPAPI survival, which does not apply to an account nothing ever signs
/// into interactively; kept anyway; there is no reason to churn the account
/// on every logon when reuse costs nothing.
pub fn service_account_password() -> eyre::Result<String> {
    let sid = account_sid(SERVICE_ACCOUNT_NAME).map_err(|e| eyre::eyre!("{e}"))?;
    let sid = sid_to_string(&sid).map_err(|e| eyre::eyre!("{e}"))?;
    let store = KeyringPasswordStore::new();

    if let Some(stored) = store.load(&sid)? {
        match RealSyscalls.validate(SERVICE_ACCOUNT_NAME, &stored) {
            Ok(PasswordCheck::Valid) => return Ok(stored),
            Ok(PasswordCheck::Expired) => {
                let new =
                    crate::helpers::generate_random_password().map_err(|e| eyre::eyre!("{e}"))?;
                if RealSyscalls
                    .change(SERVICE_ACCOUNT_NAME, &stored, &new)
                    .is_ok()
                {
                    let _ = store.save(&sid, &new);
                    return Ok(new);
                }
            }
            // Changed out of band; fall through to a reset.
            Ok(PasswordCheck::Rejected) => {}
            // Inconclusive, not wrong — see credential.rs::stored_password.
            Err(e) => {
                log::warn!(
                    "could not verify the service account's stored password ({e}); using it anyway"
                );
                return Ok(stored);
            }
        }
    }

    let password = crate::helpers::generate_random_password().map_err(|e| eyre::eyre!("{e}"))?;
    RealSyscalls
        .reset(SERVICE_ACCOUNT_NAME, &password)
        .map_err(|e| eyre::eyre!("{e}"))?;
    if let Err(e) = store.save(&sid, &password) {
        log::error!("could not store the service account's password: {e}");
    }
    Ok(password)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod byte_layout_tests {
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

    /// Unlike `LSA_STRING`, `deny_interactive_and_network_logon`'s rights
    /// list carries no NUL at all — `Length`/`MaximumLength` are both the
    /// exact UTF-16 byte count.
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
