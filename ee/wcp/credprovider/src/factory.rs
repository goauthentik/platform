use std::{ffi::c_void, mem, ptr, sync::atomic::Ordering};

use windows::{
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_INVALIDARG, E_NOINTERFACE, E_POINTER},
        System::Com::{IClassFactory, IClassFactory_Impl},
        UI::Shell::ICredentialProvider,
    },
    core::{BOOL, GUID, IUnknown, Interface, Ref, Result, implement},
};

use crate::{FACTORY_LOCK_COUNT, provider::CredentialProvider};

#[implement(IClassFactory)]
pub struct CredentialProviderFactory;

impl IClassFactory_Impl for CredentialProviderFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> Result<()> {
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

        if riid == ICredentialProvider::IID {
            let provider: ICredentialProvider = CredentialProvider::new().into();
            unsafe { *ppvobject = mem::transmute::<ICredentialProvider, *mut c_void>(provider) };
            return Ok(());
        }
        Err(E_NOINTERFACE.into())
    }

    fn LockServer(&self, flock: BOOL) -> Result<()> {
        if flock.as_bool() {
            FACTORY_LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
        } else {
            FACTORY_LOCK_COUNT.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}
