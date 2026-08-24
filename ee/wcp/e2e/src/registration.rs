//! Registers the real `ak_cred_provider.dll` the way the MSI installer does
//! (`vpkg/windows/Package.wxs`), so a test can activate it through real COM
//! rather than reaching in with `LoadLibraryW`/`DllGetClassObject` as
//! `dll::LoadedProvider` does. That is the path LogonUI and CredUI use, and the
//! only thing here that would catch a wrong or missing registry value.
//!
//! Needs an elevated shell: HKLM is not writable otherwise.

use std::path::Path;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

const PROVIDER_NAME: &str = "authentik Credential Provider";
const CLSID_SUBKEY: &str = r"SOFTWARE\Classes\CLSID\{7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A}";
const PROVIDERS_SUBKEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A}";

/// Writes the registration; `Drop` removes exactly what it wrote.
pub struct RegisteredProvider;

impl RegisteredProvider {
    /// Fails rather than overwriting an already-registered CLSID: on a machine
    /// with a real install, clobbering it points the real credential provider
    /// at this dev build, and `Drop` then deletes its registration outright.
    pub fn register(dll_path: &Path) -> eyre::Result<Self> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if hklm.open_subkey(CLSID_SUBKEY).is_ok() {
            eyre::bail!(
                "{CLSID_SUBKEY} already exists — ak_cred_provider.dll appears to be installed \
                 on this machine; refusing to overwrite its registration. Run this test on a \
                 clean machine or VM instead."
            );
        }

        let (clsid, _) = hklm.create_subkey(CLSID_SUBKEY)?;
        clsid.set_value("", &PROVIDER_NAME.to_string())?;
        let (inproc, _) = clsid.create_subkey("InprocServer32")?;
        inproc.set_value("", &dll_path.to_string_lossy().into_owned())?;
        inproc.set_value("ThreadingModel", &"Apartment".to_string())?;

        let (providers, _) = hklm.create_subkey(PROVIDERS_SUBKEY)?;
        providers.set_value("", &PROVIDER_NAME.to_string())?;

        Ok(Self)
    }
}

impl Drop for RegisteredProvider {
    fn drop(&mut self) {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let _ = hklm.delete_subkey_all(CLSID_SUBKEY);
        let _ = hklm.delete_subkey_all(PROVIDERS_SUBKEY);
    }
}

/// Scopes a classic-COM apartment to the current OS thread. Must be dropped
/// after every COM interface obtained while it was alive, so declare it before
/// them: Rust drops locals in reverse declaration order.
pub struct ComGuard;

impl ComGuard {
    pub fn new() -> windows::core::Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.ok()?;
        Ok(Self)
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}
