//! Credential packing: builds the buffer `GetSerialization` hands back to
//! LogonUI, and the random local-account password used to bridge a
//! completed browser sign-in into a real Windows logon.

use windows::{
    Win32::{
        Foundation::E_OUTOFMEMORY,
        Security::Authentication::Identity::{
            KERB_INTERACTIVE_UNLOCK_LOGON, KERB_LOGON_SUBMIT_TYPE, KerbInteractiveLogon,
            KerbWorkstationUnlockLogon, LSA_UNICODE_STRING,
        },
        Security::Credentials::{
            CRED_PACK_FLAGS, CRED_PACK_ID_PROVIDER_CREDENTIALS, CRED_PACK_PROTECTED_CREDENTIALS,
            CredPackAuthenticationBufferW,
        },
        Security::Cryptography::{BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptGenRandom},
        System::Com::CoTaskMemAlloc,
        UI::Shell::{CPUS_UNLOCK_WORKSTATION, CREDENTIAL_PROVIDER_USAGE_SCENARIO},
    },
    core::{PCWSTR, PWSTR},
};

const WIN_PASS_LEN: usize = 50;

/// Generate a cryptographically random password, used only as a throwaway
/// local-account credential the caller immediately submits via `Negotiate` —
/// never shown to or typed by the user.
pub fn generate_random_password() -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut buf = [0u8; WIN_PASS_LEN];
    unsafe {
        let _ = BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG);
    }
    buf.iter()
        .map(|&b| CHARSET[b as usize % CHARSET.len()] as char)
        .collect()
}

pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| ".".to_string())
}

fn kerb_message_type(cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO) -> KERB_LOGON_SUBMIT_TYPE {
    if cpus == CPUS_UNLOCK_WORKSTATION {
        KerbWorkstationUnlockLogon
    } else {
        KerbInteractiveLogon
    }
}

/// Build a `CoTaskMemAlloc`-allocated flat `KERB_INTERACTIVE_UNLOCK_LOGON`
/// buffer for a local account: `[header][domain utf-16][username utf-16][password utf-16]`.
///
/// The `LSA_UNICODE_STRING.Buffer` fields carry byte offsets from the start
/// of the allocation rather than real pointers — LogonUI patches them to
/// real pointers before handing the buffer to LSA. Returns `(ptr, byte_len)`;
/// LogonUI frees the buffer with `CoTaskMemFree`.
pub fn pack_kerb_interactive_unlock_logon(
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

        let domain_offset = header_size;
        std::ptr::copy_nonoverlapping(
            domain_wide.as_ptr() as *const u8,
            buf.add(domain_offset),
            domain_bytes,
        );
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

/// Pack a domain/non-local account's credentials via
/// `CredPackAuthenticationBufferW`, the buffer shape LogonUI expects for
/// accounts that aren't resolved as local Windows users.
pub fn pack_authentication_buffer(
    username: &str,
    password: &str,
) -> windows::core::Result<(*mut u8, u32)> {
    let username_wide: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
    let password_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();
    let flags =
        CRED_PACK_FLAGS(CRED_PACK_PROTECTED_CREDENTIALS.0 | CRED_PACK_ID_PROVIDER_CREDENTIALS.0);

    let mut size = 0u32;
    unsafe {
        let _ = CredPackAuthenticationBufferW(
            flags,
            PCWSTR(username_wide.as_ptr()),
            PCWSTR(password_wide.as_ptr()),
            None,
            &mut size,
        );
    }
    if size == 0 {
        return Err(windows::core::Error::from(E_OUTOFMEMORY));
    }

    let buf = unsafe { CoTaskMemAlloc(size as usize) } as *mut u8;
    if buf.is_null() {
        return Err(windows::core::Error::from(E_OUTOFMEMORY));
    }

    let packed = unsafe {
        CredPackAuthenticationBufferW(
            flags,
            PCWSTR(username_wide.as_ptr()),
            PCWSTR(password_wide.as_ptr()),
            Some(buf),
            &mut size,
        )
    };
    packed?;
    Ok((buf, size))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use windows::Win32::UI::Shell::CPUS_LOGON;

    /// Reads back an `LSA_UNICODE_STRING` whose `Buffer` field is a byte
    /// offset (not a real pointer) into `base`, matching the packed layout.
    unsafe fn read_offset_string(base: *const u8, s: &LSA_UNICODE_STRING) -> String {
        let offset = s.Buffer.0 as usize;
        let len_u16 = s.Length as usize / 2;
        unsafe {
            let ptr = base.add(offset) as *const u16;
            let slice = std::slice::from_raw_parts(ptr, len_u16);
            String::from_utf16_lossy(slice)
        }
    }

    #[test]
    fn kerb_pack_round_trips_domain_username_password() {
        let (buf, len) =
            pack_kerb_interactive_unlock_logon("WORKGROUP", "alice", "s3cret!", CPUS_LOGON)
                .unwrap();

        unsafe {
            let kiul = buf as *const KERB_INTERACTIVE_UNLOCK_LOGON;
            let kil = &(*kiul).Logon;
            assert_eq!(kil.MessageType, KerbInteractiveLogon);
            assert_eq!(read_offset_string(buf, &kil.LogonDomainName), "WORKGROUP");
            assert_eq!(read_offset_string(buf, &kil.UserName), "alice");
            assert_eq!(read_offset_string(buf, &kil.Password), "s3cret!");

            let header_size = std::mem::size_of::<KERB_INTERACTIVE_UNLOCK_LOGON>();
            let expected_len = header_size + (9 + 5 + 7) * 2;
            assert_eq!(len as usize, expected_len);

            windows::Win32::System::Com::CoTaskMemFree(Some(buf as *const _));
        }
    }

    #[test]
    fn kerb_pack_uses_unlock_message_type_for_unlock_scenario() {
        let (buf, _) = pack_kerb_interactive_unlock_logon(
            "WORKGROUP",
            "alice",
            "s3cret!",
            CPUS_UNLOCK_WORKSTATION,
        )
        .unwrap();
        unsafe {
            let kiul = buf as *const KERB_INTERACTIVE_UNLOCK_LOGON;
            assert_eq!((*kiul).Logon.MessageType, KerbWorkstationUnlockLogon);
            windows::Win32::System::Com::CoTaskMemFree(Some(buf as *const _));
        }
    }

    #[test]
    fn random_password_meets_length_and_charset() {
        let pw = generate_random_password();
        assert_eq!(pw.chars().count(), WIN_PASS_LEN);
        assert!(pw.chars().all(|c| c.is_ascii_graphic()));
    }
}
