//! Credential packing helpers — Rust port of Helpers.cpp.
//!
//! Mirrors KerbInteractiveUnlockLogonInit/Pack, RetrieveNegotiateAuthPackage,
//! and the random-password + NetUserSetInfo flow from the C++ provider.

use windows::{
    Win32::{
        Foundation::{E_FAIL, E_OUTOFMEMORY, HANDLE},
        NetworkManagement::NetManagement::{NetUserSetInfo, USER_INFO_1003},
        Security::{
            Authentication::Identity::{
                KERB_INTERACTIVE_UNLOCK_LOGON, KERB_LOGON_SUBMIT_TYPE, KerbInteractiveLogon,
                KerbWorkstationUnlockLogon, LSA_STRING, LSA_UNICODE_STRING, LsaConnectUntrusted,
                LsaDeregisterLogonProcess, LsaLookupAuthenticationPackage,
            },
            Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
        },
        System::Com::CoTaskMemAlloc,
        UI::Shell::CREDENTIAL_PROVIDER_USAGE_SCENARIO,
    },
    core::{PCWSTR, PSTR, PWSTR},
};

use windows::Win32::UI::Shell::CPUS_UNLOCK_WORKSTATION;

const WIN_PASS_LEN: usize = 50;

/// Generate a cryptographically random password string using BCryptGenRandom.
pub fn generate_random_password() -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut buf = [0u8; WIN_PASS_LEN];
    unsafe {
        // None = use system preferred RNG (BCRYPT_USE_SYSTEM_PREFERRED_RNG).
        let _ = BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }
    buf.iter()
        .map(|&b| CHARSET[b as usize % CHARSET.len()] as char)
        .collect()
}

/// Reset the Windows local account password for `username` to `password`.
/// Mirrors the NetUserSetInfo(NULL, username, 1003, ...) call in Connect().
pub fn reset_local_password(username: &str, password: &str) -> windows::core::Result<()> {
    let username_wide: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
    let password_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

    let info = USER_INFO_1003 {
        usri1003_password: PWSTR(password_wide.as_ptr() as *mut u16),
    };

    let result = unsafe {
        NetUserSetInfo(
            PCWSTR::null(), // local machine
            PCWSTR(username_wide.as_ptr()),
            1003,
            &info as *const USER_INFO_1003 as *const u8,
            None,
        )
    };

    if result != 0 {
        // NET_API_STATUS; 0 == NERR_Success
        return Err(windows::core::Error::from(E_FAIL));
    }
    Ok(())
}

/// Return the NetBIOS computer name (used as the domain for local accounts).
pub fn get_computer_name() -> String {
    // COMPUTERNAME is always set on Windows and returns the NetBIOS name.
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| ".".to_string())
}

/// Look up the "Negotiate" authentication package index via LSA.
/// Mirrors RetrieveNegotiateAuthPackage() in Helpers.cpp.
pub fn retrieve_negotiate_auth_package() -> windows::core::Result<u32> {
    let mut lsa_handle = HANDLE::default();
    let status = unsafe { LsaConnectUntrusted(&mut lsa_handle) };
    if status.0 != 0 {
        return Err(windows::core::Error::from(E_FAIL));
    }

    let name = b"Negotiate";
    let lsa_name = LSA_STRING {
        Length: name.len() as u16,
        MaximumLength: (name.len() + 1) as u16,
        Buffer: PSTR(name.as_ptr() as *mut u8),
    };

    let mut auth_package = 0u32;
    let status =
        unsafe { LsaLookupAuthenticationPackage(lsa_handle, &lsa_name, &mut auth_package) };
    unsafe {
        let _ = LsaDeregisterLogonProcess(lsa_handle);
    };

    if status.0 != 0 {
        return Err(windows::core::Error::from(E_FAIL));
    }
    Ok(auth_package)
}

/// Map usage scenario to the KERB MessageType.
fn kerb_message_type(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> KERB_LOGON_SUBMIT_TYPE {
    if cpus == CPUS_UNLOCK_WORKSTATION {
        KerbWorkstationUnlockLogon
    } else {
        KerbInteractiveLogon
    }
}

/// Build a CoTaskMem-allocated flat `KERB_INTERACTIVE_UNLOCK_LOGON` buffer
/// ready for `CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION.rgbSerialization`.
///
/// Layout: [KERB_INTERACTIVE_UNLOCK_LOGON header] [domain UTF-16] [username UTF-16] [password UTF-16]
///
/// The `LSA_UNICODE_STRING.Buffer` fields carry **byte offsets** from the start of
/// the allocation (not real pointers); the OS logon path patches them up before
/// calling LsaLogonUser. This matches KerbInteractiveUnlockLogonPack() in Helpers.cpp.
///
/// Returns `(buf_ptr, byte_len)`. LogonUI frees the buffer with CoTaskMemFree.
pub fn kerb_interactive_unlock_logon_pack(
    domain: &str,
    username: &str,
    password: &str,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
) -> windows::core::Result<(*mut u8, u32)> {
    let domain_wide: Vec<u16> = domain.encode_utf16().collect();
    let username_wide: Vec<u16> = username.encode_utf16().collect();
    let password_wide: Vec<u16> = password.encode_utf16().collect();

    let domain_bytes = domain_wide.len() * 2;
    let username_bytes = username_wide.len() * 2;
    let password_bytes = password_wide.len() * 2;
    let header_size = std::mem::size_of::<KERB_INTERACTIVE_UNLOCK_LOGON>();
    let total = header_size + domain_bytes + username_bytes + password_bytes;

    let buf = unsafe { CoTaskMemAlloc(total) } as *mut u8;
    if buf.is_null() {
        return Err(windows::core::Error::from(E_OUTOFMEMORY));
    }

    unsafe {
        std::ptr::write_bytes(buf, 0, total);

        let kiul = buf as *mut KERB_INTERACTIVE_UNLOCK_LOGON;
        let kil = &mut (*kiul).Logon;
        kil.MessageType = kerb_message_type(cpus);

        // Domain string starts immediately after the header.
        let domain_offset = header_size;
        std::ptr::copy_nonoverlapping(
            domain_wide.as_ptr() as *const u8,
            buf.add(domain_offset),
            domain_bytes,
        );
        // Buffer holds byte offset from allocation start, not a real pointer.
        // The OS logon path adds this offset to the base address at logon time.
        kil.LogonDomainName = LSA_UNICODE_STRING {
            Length: domain_bytes as u16,
            MaximumLength: domain_bytes as u16,
            Buffer: PWSTR(domain_offset as *mut u16),
        };

        let username_offset = domain_offset + domain_bytes;
        std::ptr::copy_nonoverlapping(
            username_wide.as_ptr() as *const u8,
            buf.add(username_offset),
            username_bytes,
        );
        kil.UserName = LSA_UNICODE_STRING {
            Length: username_bytes as u16,
            MaximumLength: username_bytes as u16,
            Buffer: PWSTR(username_offset as *mut u16),
        };

        let password_offset = username_offset + username_bytes;
        std::ptr::copy_nonoverlapping(
            password_wide.as_ptr() as *const u8,
            buf.add(password_offset),
            password_bytes,
        );
        kil.Password = LSA_UNICODE_STRING {
            Length: password_bytes as u16,
            MaximumLength: password_bytes as u16,
            Buffer: PWSTR(password_offset as *mut u16),
        };
    }

    Ok((buf, total as u32))
}
