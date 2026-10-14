use std::{
    ffi::c_void,
    mem, ptr,
    sync::OnceLock,
    sync::atomic::{AtomicUsize, Ordering},
};

use windows::{
    Win32::{
        Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_INVALIDARG, E_POINTER, HMODULE, S_FALSE, S_OK},
        System::{
            Com::IClassFactory,
            LibraryLoader::{
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, GetModuleFileNameW,
                GetModuleHandleExW,
            },
        },
    },
    core::{GUID, HRESULT, Interface, PCWSTR},
};

mod credential;
mod factory;
mod helpers;
mod ipc;
mod provider;
mod strings;
mod syscalls;
mod sysd;
mod tile;

use factory::CredentialProviderFactory;

static FACTORY_LOCK_COUNT: AtomicUsize = AtomicUsize::new(0);
static DLL_MODULE: OnceLock<usize> = OnceLock::new();
static DLL_DIR: OnceLock<std::path::PathBuf> = OnceLock::new();

pub use ak_ee_wcp_wire::CLSID_CREDENTIAL_PROVIDER;

/// This DLL's own module handle: the tile bitmap is embedded here, not in the
/// host process's `.exe`.
pub(crate) fn own_module() -> HMODULE {
    HMODULE(*DLL_MODULE.get_or_init(|| resolve_own_module().0 as usize) as *mut c_void)
}

/// Directory this DLL was loaded from — `ak_browser.exe` lives next to it.
pub(crate) fn dll_dir() -> &'static std::path::Path {
    DLL_DIR.get_or_init(|| {
        module_file_name(own_module())
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default()
    })
}

fn resolve_own_module() -> HMODULE {
    unsafe {
        let mut module = HMODULE::default();
        let found = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            PCWSTR(resolve_own_module as *const u16),
            &mut module,
        );
        if found.is_err() {
            return HMODULE::default();
        }
        module
    }
}

fn module_file_name(module: HMODULE) -> std::path::PathBuf {
    unsafe {
        let mut buf = [0u16; 1024];
        let len = GetModuleFileNameW(Some(module), &mut buf);
        if len == 0 {
            return std::path::PathBuf::new();
        }
        std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// LogonUI locates this provider via the registry entries the MSI installer
/// writes (`InprocServer32` etc.) — the DLL itself does not self-register.
#[unsafe(no_mangle)]
extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() {
        return E_POINTER;
    }
    unsafe { *ppv = ptr::null_mut() };
    if rclsid.is_null() || riid.is_null() {
        return E_INVALIDARG;
    }

    static LOG_INIT: std::sync::Once = std::sync::Once::new();
    LOG_INIT.call_once(|| {
        ak_platform::log::LogBuilder::new(ak_platform::string::PlatformString::new_with_default(
            "authentik Credential Provider",
        ))
        .with_default_filters()
        .allow_platform(true)
        // LogonUI has no console, so a shipped provider logs only to the
        // platform log. Debug builds — what `e2e` runs — also log to stdout,
        // the only way a failure inside the DLL shows up in captured output.
        .allow_stdout(cfg!(debug_assertions))
        .enable();
    });

    let rclsid = unsafe { *rclsid };
    let riid = unsafe { *riid };
    if rclsid != CLSID_CREDENTIAL_PROVIDER || riid != IClassFactory::IID {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory: IClassFactory = CredentialProviderFactory.into();
    unsafe { *ppv = mem::transmute::<IClassFactory, *mut c_void>(factory) };
    S_OK
}

#[unsafe(no_mangle)]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if FACTORY_LOCK_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}
