//! Drives the real built `ak_cred_provider.dll` directly via
//! `LoadLibraryW`/`DllGetClassObject` — no registry entry needed, since
//! `CPUS_CREDUI` (used together with the `debug` capability flag) works on
//! an ordinary interactive desktop.

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows::{
    Win32::{
        Foundation::{E_FAIL, FreeLibrary, HMODULE},
        System::{
            Com::IClassFactory,
            LibraryLoader::{GetProcAddress, LoadLibraryW},
        },
        UI::Shell::ICredentialProvider,
    },
    core::{GUID, HRESULT, Interface, PCWSTR, s},
};

pub const CLSID_CREDENTIAL_PROVIDER: GUID = GUID::from_u128(0x7BCC7941_18BA_4A8E_8E0A_1D0F8E73577A);

type DllGetClassObjectFn =
    unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;

/// Locates the `.dll`/`.exe` build artifacts sitting next to this test
/// binary: `cargo`'s configured `target-dir` puts every workspace member's
/// output in the same profile directory, and this test binary lands one
/// level down, in `deps/`.
pub fn build_output_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().and_then(Path::parent).map(Path::to_path_buf))
        .unwrap_or_default()
}

pub struct LoadedProvider {
    module: HMODULE,
    pub provider: ICredentialProvider,
}

impl LoadedProvider {
    pub fn load(dll_path: &Path) -> windows::core::Result<Self> {
        let wide: Vec<u16> = dll_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let module = unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }?;

        let proc = unsafe { GetProcAddress(module, s!("DllGetClassObject")) }
            .ok_or(windows::core::Error::from(E_FAIL))?;
        let get_class_object: DllGetClassObjectFn = unsafe { std::mem::transmute(proc) };

        let mut factory_obj: *mut c_void = std::ptr::null_mut();
        unsafe {
            get_class_object(
                &CLSID_CREDENTIAL_PROVIDER,
                &IClassFactory::IID,
                &mut factory_obj,
            )
        }
        .ok()?;
        let factory: IClassFactory = unsafe { IClassFactory::from_raw(factory_obj) };

        let provider: ICredentialProvider = unsafe { factory.CreateInstance(None) }?;

        Ok(Self { module, provider })
    }
}

impl Drop for LoadedProvider {
    fn drop(&mut self) {
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
