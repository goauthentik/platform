use windows::{Win32::System::Com::CoTaskMemAlloc, core::PWSTR};

/// Allocates a NUL-terminated wide string with `CoTaskMemAlloc` so it can be
/// handed back to LogonUI through an out-param, which LogonUI then frees with
/// `CoTaskMemFree`.
pub fn cotask_pwstr(s: &str) -> PWSTR {
    let wide = crate::syscalls::wide(s);
    unsafe {
        let ptr = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
        if ptr.is_null() {
            return PWSTR::null();
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        PWSTR(ptr)
    }
}

/// Reads a `CoTaskMemAlloc`-owned `PWSTR` the shell handed us (e.g. from
/// `ICredentialProviderUser::GetSid`), and frees it.
pub fn take_pwstr(s: windows::core::PWSTR) -> String {
    if s.is_null() {
        return String::new();
    }
    let value = unsafe { s.to_string().unwrap_or_default() };
    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(s.0 as *const _));
    }
    value
}
