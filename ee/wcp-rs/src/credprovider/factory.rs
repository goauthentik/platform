use std::{
    ffi::c_void, mem, ptr, sync::atomic::Ordering
};

use windows::{
    core::{implement, Result, BOOL, GUID, IUnknown, Interface, Ref},
    Win32::{
        Foundation::{
            CLASS_E_NOAGGREGATION, E_INVALIDARG, E_NOINTERFACE,
            E_POINTER
        },
        System::Com::{IClassFactory, IClassFactory_Impl},
        UI::Shell::ICredentialProvider,
    },
};

use crate::{credprovider::credential_provider::CredentialProvider, PROVIDER_FACTORY_REFERENCE_COUNT};


#[implement(IClassFactory)]
pub struct CredentialProviderFactory;

impl IClassFactory_Impl for CredentialProviderFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
        // Validate arguments
        if ppvobject.is_null() {
            return Err(E_POINTER.into());
        }
        unsafe { *ppvobject = ptr::null_mut() };
        if riid.is_null() {
            return Err(E_INVALIDARG.into());
        }
        let riid = unsafe { *riid };
        if punkouter.is_some() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        // We're only handling requests for `IID_ICredentialProvider`
        if riid == ICredentialProvider::IID {
            // Construct credential provider and return it as an `ICredentialProvider`
            // interface
            let provider: ICredentialProvider = CredentialProvider::new().into();
            unsafe { *ppvobject = mem::transmute(provider) };
            return Ok(());
        }
        return Err(E_NOINTERFACE.into());
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            PROVIDER_FACTORY_REFERENCE_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            PROVIDER_FACTORY_REFERENCE_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
