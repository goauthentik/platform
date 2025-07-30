use libc::c_char;

use pam::{constants::PamResultCode, module::PamHandle};
use std::ffi::{CStr, CString};

#[link(name = "pam")]
unsafe extern "C" {
    /// Retrieve a single env var (returns a malloc’d C string owned by PAM)
    fn pam_getenv(pamh: *const PamHandle, name: *const c_char) -> *mut c_char;

    /// Add or update an env var in PAM (expects "KEY=VAL" malloc’d internally)
    fn pam_putenv(pamh: *const PamHandle, name_value: *const c_char) -> PamResultCode;
}

pub fn pam_get_env(pamh: &mut PamHandle, key: &str) -> Option<String> {
    let c_key = CString::new(key).ok()?;
    unsafe {
        let ptr = pam_getenv(pamh, c_key.as_ptr());
        if ptr.is_null() {
            None
        } else {
            // pointer owned by libpam; don’t free
            Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
        }
    }
}

pub fn pam_put_env(pamh: &mut PamHandle, key: &str, val: &str) -> Result<(), PamResultCode> {
    let kv = format!("{key}={val}");
    let c_kv = CString::new(kv).map_err(|_| PamResultCode::PAM_INCOMPLETE)?;
    let ret = unsafe { pam_putenv(pamh, c_kv.as_ptr()) };
    if ret == PamResultCode::PAM_SUCCESS {
        Ok(())
    } else {
        Err(ret)
    }
}
