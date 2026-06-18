use std::{mem::size_of, sync::Mutex};

use windows::{
    Win32::{
        Foundation::{E_INVALIDARG, E_NOTIMPL, E_OUTOFMEMORY, FALSE},
        System::Com::CoTaskMemAlloc,
        UI::Shell::{
            CPFT_LARGE_TEXT, CPFT_SUBMIT_BUTTON, CPUS_CHANGE_PASSWORD, CPUS_CREDUI, CPUS_LOGON,
            CPUS_UNLOCK_WORKSTATION, CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
            CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR, CREDENTIAL_PROVIDER_USAGE_SCENARIO,
            ICredentialProvider, ICredentialProvider_Impl, ICredentialProviderCredential,
            ICredentialProviderEvents, ICredentialProviderSetUserArray,
            ICredentialProviderSetUserArray_Impl, ICredentialProviderUserArray,
        },
    },
    core::{BOOL, GUID, Ref, Result, implement},
};

use crate::credprovider::credential::Credential;

#[implement(ICredentialProvider, ICredentialProviderSetUserArray)]
pub struct CredentialProvider {
    cpus: Mutex<CREDENTIAL_PROVIDER_USAGE_SCENARIO>,
}

impl CredentialProvider {
    pub fn new() -> Self {
        Self {
            cpus: Mutex::new(CPUS_LOGON),
        }
    }
}

impl ICredentialProvider_Impl for CredentialProvider_Impl {
    fn SetUsageScenario(
        &self,
        cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
        _dwflags: u32,
    ) -> Result<()> {
        let (available, debug) = match ak_ffi::ffi::sys_caps() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("ak_sys_caps failed: {e}");
                return Err(E_NOTIMPL.into());
            }
        };
        if !available {
            log::info!("Interactive authentication not available, not showing cred UI");
            return Err(E_NOTIMPL.into());
        }
        let result = match cpus {
            CPUS_LOGON | CPUS_UNLOCK_WORKSTATION => Ok(()),
            CPUS_CREDUI if debug => Ok(()),
            CPUS_CREDUI | CPUS_CHANGE_PASSWORD => Err(E_NOTIMPL.into()),
            _ => Err(E_INVALIDARG.into()),
        };
        if result.is_ok() {
            *self.cpus.lock().unwrap() = cpus;
        }
        result
    }

    fn SetSerialization(
        &self,
        _pcpcs: *const CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
    ) -> Result<()> {
        Ok(())
    }

    fn Advise(
        &self,
        _pcpe: Ref<'_, ICredentialProviderEvents>,
        _upadvisecontext: usize,
    ) -> Result<()> {
        Ok(())
    }

    fn UnAdvise(&self) -> Result<()> {
        Ok(())
    }

    fn GetFieldDescriptorCount(&self) -> Result<u32> {
        Ok(2)
    }

    fn GetFieldDescriptorAt(
        &self,
        dwindex: u32,
    ) -> Result<*mut CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR> {
        let descriptor = match dwindex {
            0 => CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR {
                dwFieldID: 0,
                cpft: CPFT_LARGE_TEXT,
                pszLabel: crate::utils::cotask_pwstr("authentik Login"),
                guidFieldType: GUID::zeroed(),
            },
            1 => CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR {
                dwFieldID: 1,
                cpft: CPFT_SUBMIT_BUTTON,
                pszLabel: crate::utils::cotask_pwstr("Sign In with authentik"),
                guidFieldType: GUID::zeroed(),
            },
            _ => return Err(E_INVALIDARG.into()),
        };

        unsafe {
            let ptr = CoTaskMemAlloc(size_of::<CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR>())
                as *mut CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR;
            if ptr.is_null() {
                return Err(E_OUTOFMEMORY.into());
            }
            std::ptr::write(ptr, descriptor);
            Ok(ptr)
        }
    }

    fn GetCredentialCount(
        &self,
        pdwcount: *mut u32,
        pdwdefault: *mut u32,
        pbautologonwithdefault: *mut BOOL,
    ) -> Result<()> {
        unsafe {
            *pdwcount = 1;
            *pdwdefault = 0;
            *pbautologonwithdefault = FALSE;
        }
        Ok(())
    }

    fn GetCredentialAt(&self, dwindex: u32) -> Result<ICredentialProviderCredential> {
        if dwindex == 0 {
            let cpus = *self.cpus.lock().unwrap();
            let credential = Credential::new(cpus);
            Ok(credential.into())
        } else {
            Err(E_INVALIDARG.into())
        }
    }
}

impl ICredentialProviderSetUserArray_Impl for CredentialProvider_Impl {
    fn SetUserArray(&self, _users: Ref<'_, ICredentialProviderUserArray>) -> Result<()> {
        Ok(())
    }
}
