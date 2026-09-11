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
            Com::{CoTaskMemFree, IClassFactory},
            LibraryLoader::{GetProcAddress, LoadLibraryW},
        },
        UI::Shell::{
            CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
            CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE, CREDENTIAL_PROVIDER_STATUS_ICON,
            ICredentialProvider, ICredentialProviderCredential,
        },
    },
    core::{GUID, HRESULT, Interface, PCWSTR, PWSTR, s},
};

pub const CLSID_CREDENTIAL_PROVIDER: GUID = GUID::from_u128(0x7BCC7941_18BA_4A8E_8E0A_1D0F8E73577A);

type DllGetClassObjectFn =
    unsafe extern "system" fn(*const GUID, *const GUID, *mut *mut c_void) -> HRESULT;

/// Cargo puts every workspace member's output in one profile directory, with
/// test binaries in `deps/`.
pub fn build_output_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().and_then(Path::parent).map(Path::to_path_buf))
        .unwrap_or_default()
}

pub struct LoadedProvider {
    module: HMODULE,
    /// `Option` so `Drop` can release it *before* `FreeLibrary`: every COM
    /// pointer here is backed by vtable code inside the DLL, and releasing one
    /// after the module is unmapped jumps into freed pages.
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
        // `Drop::drop` runs before fields are dropped, so leaving this to the
        // field drop would release the interface after `FreeLibrary` below.
        drop(self.provider.take());
        unsafe {
            let _ = FreeLibrary(self.module);
        }
    }
}

/// Shared by every test that drives a `Connect()`ed credential to completion,
/// however the provider was activated.
///
/// # Safety
/// `credential` must be a live `ICredentialProviderCredential` whose
/// `Connect()` has already run (or that has no outcome recorded — a valid
/// call in production, always answered with `CPGSR_NO_CREDENTIAL_FINISHED`).
pub unsafe fn get_serialization(
    credential: &ICredentialProviderCredential,
) -> (
    CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE,
    CREDENTIAL_PROVIDER_STATUS_ICON,
    CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
    String,
) {
    let mut response = CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE::default();
    let mut serialization = CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION::default();
    let mut status_text = PWSTR::null();
    let mut status_icon = CREDENTIAL_PROVIDER_STATUS_ICON::default();

    unsafe {
        #[allow(clippy::expect_used)]
        credential
            .GetSerialization(
                &mut response,
                &mut serialization,
                &mut status_text,
                &mut status_icon,
            )
            .expect("GetSerialization");
    }

    let text = if status_text.is_null() {
        String::new()
    } else {
        let s = unsafe { status_text.to_string() }.unwrap_or_default();
        unsafe {
            CoTaskMemFree(Some(status_text.0 as *const _));
        }
        s
    };

    (response, status_icon, serialization, text)
}
