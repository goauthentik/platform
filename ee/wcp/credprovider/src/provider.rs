use std::sync::Mutex;

use windows::{
    Win32::{
        Foundation::{E_INVALIDARG, E_NOTIMPL, FALSE},
        Storage::EnhancedStorage::PKEY_Identity_QualifiedUserName,
        System::Com::CoTaskMemAlloc,
        UI::Shell::{
            CPUS_CHANGE_PASSWORD, CPUS_CREDUI, CPUS_LOGON, CPUS_UNLOCK_WORKSTATION,
            CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION, CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR,
            CREDENTIAL_PROVIDER_USAGE_SCENARIO, ICredentialProvider, ICredentialProvider_Impl,
            ICredentialProviderCredential, ICredentialProviderEvents,
            ICredentialProviderSetUserArray, ICredentialProviderSetUserArray_Impl,
            ICredentialProviderUser, ICredentialProviderUserArray, Identity_LocalUserProvider,
        },
    },
    core::{BOOL, Ref, Result, implement},
};

use crate::credential::Credential;
use crate::ipc::CefAuthFlow;
use crate::strings::take_pwstr;
use crate::syscalls::RealSyscalls;
use crate::tile;

#[implement(ICredentialProvider, ICredentialProviderSetUserArray)]
pub struct CredentialProvider {
    cpus: Mutex<CREDENTIAL_PROVIDER_USAGE_SCENARIO>,
    users: Mutex<Option<ICredentialProviderUserArray>>,
    credentials: Mutex<Option<Vec<ICredentialProviderCredential>>>,
}

impl CredentialProvider {
    pub fn new() -> Self {
        Self {
            cpus: Mutex::new(CPUS_LOGON),
            users: Mutex::new(None),
            credentials: Mutex::new(None),
        }
    }

    fn ensure_credentials_built(&self) {
        let mut credentials = self.credentials.lock().unwrap_or_else(|e| e.into_inner());
        if credentials.is_some() {
            return;
        }

        let users = self.users.lock().unwrap_or_else(|e| e.into_inner());
        let cpus = *self.cpus.lock().unwrap_or_else(|e| e.into_inner());
        let mut built = Vec::new();

        if let Some(array) = users.as_ref() {
            let count = unsafe { array.GetCount() }.unwrap_or(0);
            for i in 0..count {
                let Ok(user) = (unsafe { array.GetAt(i) }) else {
                    continue;
                };
                built.push(credential_from_user(&user, cpus).into());
            }
        }

        *credentials = Some(built);
    }
}

fn credential_from_user(
    user: &ICredentialProviderUser,
    cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
) -> Credential {
    let is_local_user = unsafe { user.GetProviderID() }
        .map(|provider| provider == Identity_LocalUserProvider)
        .unwrap_or(false);
    let qualified_username = unsafe { user.GetStringValue(&PKEY_Identity_QualifiedUserName) }
        .map(take_pwstr)
        .unwrap_or_default();
    let sid = unsafe { user.GetSid() }.map(take_pwstr).unwrap_or_default();

    let cef_exe = crate::dll_dir().join("ak_cef.exe");
    Credential::new(
        sid,
        qualified_username,
        is_local_user,
        cpus,
        Box::new(CefAuthFlow { cef_exe, cpus }),
        Box::new(RealSyscalls),
        Box::new(RealSyscalls),
    )
}

impl ICredentialProvider_Impl for CredentialProvider_Impl {
    fn SetUsageScenario(
        &self,
        cpus: CREDENTIAL_PROVIDER_USAGE_SCENARIO,
        _dwflags: u32,
    ) -> Result<()> {
        let caps = match sysd_client::sys_caps() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("sys_caps failed: {e}");
                return Err(E_NOTIMPL.into());
            }
        };
        if !caps.interactive_auth_available {
            log::info!("interactive auth not available, not showing credential UI");
            return Err(E_NOTIMPL.into());
        }

        let result = match cpus {
            CPUS_LOGON | CPUS_UNLOCK_WORKSTATION => Ok(()),
            CPUS_CREDUI if caps.debug => Ok(()),
            CPUS_CREDUI | CPUS_CHANGE_PASSWORD => Err(E_NOTIMPL.into()),
            _ => Err(E_INVALIDARG.into()),
        };
        if result.is_ok() {
            *self.cpus.lock().unwrap_or_else(|e| e.into_inner()) = cpus;
            *self.credentials.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
        result
    }

    fn SetSerialization(
        &self,
        _pcpcs: *const CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn Advise(
        &self,
        _pcpe: Ref<'_, ICredentialProviderEvents>,
        _upadvisecontext: usize,
    ) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn UnAdvise(&self) -> Result<()> {
        Err(E_NOTIMPL.into())
    }

    fn GetFieldDescriptorCount(&self) -> Result<u32> {
        Ok(tile::field_count())
    }

    fn GetFieldDescriptorAt(
        &self,
        dwindex: u32,
    ) -> Result<*mut CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR> {
        let descriptor = tile::field_descriptor_at(dwindex)?;
        unsafe {
            let ptr = CoTaskMemAlloc(size_of::<CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR>())
                as *mut CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR;
            if ptr.is_null() {
                return Err(windows::Win32::Foundation::E_OUTOFMEMORY.into());
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
        self.ensure_credentials_built();
        let count = self
            .credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0);
        unsafe {
            *pdwcount = count as u32;
            *pdwdefault = u32::MAX;
            *pbautologonwithdefault = FALSE;
        }
        Ok(())
    }

    fn GetCredentialAt(&self, dwindex: u32) -> Result<ICredentialProviderCredential> {
        self.ensure_credentials_built();
        self.credentials
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|c| c.get(dwindex as usize).cloned())
            .ok_or(E_INVALIDARG.into())
    }
}

impl ICredentialProviderSetUserArray_Impl for CredentialProvider_Impl {
    fn SetUserArray(&self, users: Ref<'_, ICredentialProviderUserArray>) -> Result<()> {
        *self.users.lock().unwrap_or_else(|e| e.into_inner()) = users.as_ref().cloned();
        *self.credentials.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Ok(())
    }
}
