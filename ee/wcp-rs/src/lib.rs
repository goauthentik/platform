use std::{
    ffi::c_void,
    mem, ptr,
    sync::atomic::{AtomicUsize, Ordering},
};

use windows::{
    core::{GUID, HRESULT, Interface},
    Win32::{
        Foundation::{
            CLASS_E_CLASSNOTAVAILABLE, E_INVALIDARG,
            E_POINTER, S_FALSE, S_OK,
        },
        System::Com::IClassFactory,
    },
};

use crate::{credprovider::factory::CredentialProviderFactory, utils::init_log};

mod auth;
mod credprovider;
mod utils;


static PROVIDER_FACTORY_REFERENCE_COUNT: AtomicUsize = AtomicUsize::new(0);
const CLSID_CREDENTIAL_PROVIDER: GUID = GUID::from_u128(0x12345678_1234_1234_1234_123456789012);

#[no_mangle]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    // The "class ID" this credential provider is identified by. This value needs to
    // match the value used when registering the credential provider (see the .reg
    // script above)

    // Validate arguments
    if ppv.is_null() {
        return E_POINTER;
    }
    unsafe { *ppv = ptr::null_mut() };
    if rclsid.is_null() || riid.is_null() {
        return E_INVALIDARG;
    }

    init_log();
    log::debug!("CredProvider : DllGetClassObject");

    let rclsid = unsafe { *rclsid };
    let riid = unsafe { *riid };
    // The following isn't strictly correct; a client *could* request an interface other
    // than `IClassFactory::IID`, which this implementation is simply failing.
    // This is safe, even if overly restrictive
    if rclsid != CLSID_CREDENTIAL_PROVIDER || riid != IClassFactory::IID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    // Construct the factory object and return its `IClassFactory` interface
    let factory: IClassFactory = CredentialProviderFactory.into();
    unsafe { *ppv = mem::transmute(factory) };
    S_OK
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    log::debug!(
        "DllCanUnloadNow called, Dll ref count = {}",
        PROVIDER_FACTORY_REFERENCE_COUNT.load(Ordering::SeqCst)
    );
    if PROVIDER_FACTORY_REFERENCE_COUNT.load(Ordering::SeqCst) == 0 {
        return S_OK;
    };
    S_FALSE
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    utils::register_credential_provider(&CLSID_CREDENTIAL_PROVIDER)
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    utils::unregister_credential_provider(&CLSID_CREDENTIAL_PROVIDER)
}
