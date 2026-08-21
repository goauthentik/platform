//! Credential packing: builds the buffer `GetSerialization` hands back to
//! LogonUI, and the random local-account password used to bridge a
//! completed browser sign-in into a real Windows logon.

use windows::{
    Win32::{
        Foundation::E_OUTOFMEMORY,
        Security::Authentication::Identity::{
            KERB_INTERACTIVE_UNLOCK_LOGON, KerbInteractiveLogon, KerbWorkstationUnlockLogon,
            LSA_UNICODE_STRING,
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

/// Throwaway local-account credential, submitted immediately via `Negotiate`
/// and never shown to or typed by the user.
///
/// Errors rather than falling back: `buf` starts zeroed, so a discarded RNG
/// failure would yield the fixed string `"AAAA…"` and `GetSerialization`
/// would then set *that* as the account's real Windows password. Only
/// `STATUS_SUCCESS` means the buffer was written.
pub fn generate_random_password() -> windows::core::Result<String> {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut buf = [0u8; WIN_PASS_LEN];
    let status = unsafe { BCryptGenRandom(None, &mut buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.0 != 0 {
        return Err(status.into());
    }
    Ok(buf
        .iter()
        .map(|&b| CHARSET[b as usize % CHARSET.len()] as char)
        .collect())
}

/// Flat `KERB_INTERACTIVE_UNLOCK_LOGON` for a local account, laid out as
/// `[header][domain][username][password]` in UTF-16.
///
/// Each `LSA_UNICODE_STRING.Buffer` holds a byte offset from the start of the
/// allocation, not a real pointer; LogonUI patches them before passing the
/// buffer to LSA. Returns `(ptr, byte_len)`, freed by LogonUI with
/// `CoTaskMemFree`.
pub fn pack_kerb_interactive_unlock_logon(
    domain: &str,
    username: &str,
    password: &str,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
) -> windows::core::Result<(*mut u8, u32)> {
    let parts: [Vec<u16>; 3] = [domain, username, password].map(|s| s.encode_utf16().collect());
    let header_size = size_of::<KERB_INTERACTIVE_UNLOCK_LOGON>();
    let total = header_size + parts.iter().map(|p| p.len() * 2).sum::<usize>();

    let buf = unsafe { CoTaskMemAlloc(total) } as *mut u8;
    if buf.is_null() {
        return Err(windows::core::Error::from(E_OUTOFMEMORY));
    }

    unsafe {
        std::ptr::write_bytes(buf, 0, total);

        let mut offset = header_size;
        let [domain, username, password] = parts.map(|wide| {
            let bytes = wide.len() * 2;
            std::ptr::copy_nonoverlapping(wide.as_ptr() as *const u8, buf.add(offset), bytes);
            let packed = LSA_UNICODE_STRING {
                Length: bytes as u16,
                MaximumLength: bytes as u16,
                Buffer: PWSTR(offset as *mut u16),
            };
            offset += bytes;
            packed
        });

        let kil = &mut (*(buf as *mut KERB_INTERACTIVE_UNLOCK_LOGON)).Logon;
        kil.MessageType = if cpus == CPUS_UNLOCK_WORKSTATION {
            KerbWorkstationUnlockLogon
        } else {
            KerbInteractiveLogon
        };
        kil.LogonDomainName = domain;
        kil.UserName = username;
        kil.Password = password;
    }

    Ok((buf, total as u32))
}

/// The buffer shape LogonUI expects for accounts that don't resolve as local
/// Windows users.
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
        let test_password = "test-password";
        let (buf, len) =
            pack_kerb_interactive_unlock_logon("WORKGROUP", "alice", test_password, CPUS_LOGON)
                .unwrap();

        unsafe {
            let kiul = buf as *const KERB_INTERACTIVE_UNLOCK_LOGON;
            let kil = &(*kiul).Logon;
            assert_eq!(kil.MessageType, KerbInteractiveLogon);
            assert_eq!(read_offset_string(buf, &kil.LogonDomainName), "WORKGROUP");
            assert_eq!(read_offset_string(buf, &kil.UserName), "alice");
            assert_eq!(read_offset_string(buf, &kil.Password), test_password);

            let header_size = std::mem::size_of::<KERB_INTERACTIVE_UNLOCK_LOGON>();
            let expected_len = header_size + (9 + 5 + 13) * 2;
            assert_eq!(len as usize, expected_len);

            windows::Win32::System::Com::CoTaskMemFree(Some(buf as *const _));
        }
    }

    #[test]
    fn kerb_pack_uses_unlock_message_type_for_unlock_scenario() {
        let test_password = "test-password";
        let (buf, _) = pack_kerb_interactive_unlock_logon(
            "WORKGROUP",
            "alice",
            test_password,
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
        let pw = generate_random_password().unwrap();
        assert_eq!(pw.chars().count(), WIN_PASS_LEN);
        assert!(pw.chars().all(|c| c.is_ascii_graphic()));
        // Would still hold for the all-zero buffer a discarded RNG failure
        // leaves behind, so pin that it isn't a single repeated character.
        assert!(
            pw.chars().collect::<std::collections::HashSet<_>>().len() > 1,
            "password should not be a constant run: {pw}"
        );
    }
}
