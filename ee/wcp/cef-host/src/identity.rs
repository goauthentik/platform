//! One diagnostic: which account and SID this process's own token actually
//! carries. Logged unconditionally at startup, next to the build hash — this
//! process only ever runs as SYSTEM or the dedicated service account
//! (`BROWSER_PRIVILEGE.md`), and confirming which one needs no separate,
//! correlated capture on the far end.

use std::ffi::c_void;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    GetTokenInformation, LookupAccountSidW, SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::{PCWSTR, PWSTR};

/// `"DOMAIN\name (S-1-5-...)"`, or a placeholder describing which step
/// failed — every step here can fail independently, and which one did says
/// something different about what is actually running.
pub fn current_token_identity() -> String {
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
            let account = if LookupAccountSidW(
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
            };

            let sid_string = {
                let mut wide_sid = PWSTR(std::ptr::null_mut());
                if ConvertSidToStringSidW(sid, &mut wide_sid).is_ok() {
                    let len = (0..).take_while(|&i| *wide_sid.0.add(i) != 0).count();
                    let s = String::from_utf16_lossy(std::slice::from_raw_parts(wide_sid.0, len));
                    let _ = windows::Win32::Foundation::LocalFree(Some(
                        windows::Win32::Foundation::HLOCAL(wide_sid.0 as *mut c_void),
                    ));
                    s
                } else {
                    "<could not render the SID as a string>".to_string()
                }
            };

            format!("{account} ({sid_string})")
        } else {
            "<could not read the token's user>".to_string()
        };

        let _ = CloseHandle(token);
        identity
    }
}
