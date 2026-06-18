use std::fs::File;

use log::LevelFilter;
use simplelog::{Config, WriteLogger};

use windows::{
    core::{w, GUID, HRESULT, HSTRING, PWSTR},
    Win32::{
        Foundation::{E_FAIL, S_OK},
        System::{
            Com::CoTaskMemAlloc,
            Registry::{
                RegCloseKey, RegCreateKeyExW, RegDeleteKeyW, RegSetValueExW, HKEY,
                HKEY_LOCAL_MACHINE, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
            },
        },
    },
};

pub fn init_log() {
    let logger = WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("my_rust_binary.log").unwrap(),
    );
    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .expect("Failed to setup logger");
}

/// Allocate a NUL-terminated wide string with `CoTaskMemAlloc` so it can be
/// handed back to LogonUI through an out-param (LogonUI frees it with
/// `CoTaskMemFree`).
pub fn cotask_pwstr(s: &str) -> PWSTR {
    let wide: Vec<u16> = s.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let ptr = CoTaskMemAlloc(wide.len() * 2) as *mut u16;
        if ptr.is_null() {
            return PWSTR::null();
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
        PWSTR(ptr)
    }
}

pub fn register_credential_provider(clsid: &GUID) -> HRESULT {
    unsafe {
        let clsid_str = format!("{{{:#?}}}", clsid);
        let key_path = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{}",
            clsid_str
        );

        let mut key = HKEY::default();
        let result = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            &HSTRING::from(&key_path),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        );
        let data = w!("CEF Credential Provider");
        let data_wide = data.as_wide();
        let data_bytes = std::slice::from_raw_parts(
            data_wide.as_ptr() as *const u8,
            data_wide.len() * 2, // Each u16 is 2 bytes
        );
        if result.is_ok() {
            let _ = RegSetValueExW(key, w!(""), None, REG_SZ, Some(data_bytes));
            let _ = RegCloseKey(key);
            S_OK
        } else {
            E_FAIL
        }
    }
}

pub fn unregister_credential_provider(clsid: &GUID) -> HRESULT {
    unsafe {
        let clsid_str = format!("{{{:#?}}}", clsid);
        let key_path = format!(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{}",
            clsid_str
        );

        let result = RegDeleteKeyW(HKEY_LOCAL_MACHINE, &HSTRING::from(&key_path));
        if result.is_ok() {
            S_OK
        } else {
            E_FAIL
        }
    }
}
