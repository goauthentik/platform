//! Drives the real `ak_cred_provider.dll` via `LoadLibraryW`/
//! `DllGetClassObject`. No registry entry needed: `CPUS_CREDUI` plus the
//! `debug` capability works on an ordinary interactive desktop.

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

/// The build artifacts next to this test binary: cargo puts every workspace
/// member's output in one profile directory, with test binaries in `deps/`.
pub fn build_output_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().and_then(Path::parent).map(Path::to_path_buf))
        .unwrap_or_default()
}

pub struct LoadedProvider {
    module: HMODULE,
    /// `Option` so `Drop` can release it *before* `FreeLibrary`. Every COM
    /// pointer here is backed by vtable code inside the DLL, so releasing one
    /// after the module is unmapped jumps into freed pages —
    /// `STATUS_ACCESS_VIOLATION`.
    provider: Option<ICredentialProvider>,
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

        Ok(Self {
            module,
            provider: Some(provider),
        })
    }

    /// The provider instance. Borrowed rather than exposed as a field so it
    /// cannot outlive the `LoadedProvider` that keeps the DLL mapped.
    pub fn provider(&self) -> &ICredentialProvider {
        #[allow(clippy::expect_used)]
        self.provider
            .as_ref()
            .expect("provider is only taken in Drop")
    }
}

impl Drop for LoadedProvider {
    fn drop(&mut self) {
        // Order matters: `Drop::drop` runs before fields are dropped, so
        // releasing the interface has to happen explicitly here. Leaving it to
        // the field drop would run it after `FreeLibrary` below.
        drop(self.provider.take());
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}
