//! Full activation through the real Windows COM registration path: registers
//! `ak_cred_provider.dll` the way the MSI installer does, then reaches it via
//! `CoCreateInstance` under `CPUS_CREDUI` rather than the direct
//! `LoadLibraryW`/`DllGetClassObject` call `dll::LoadedProvider` uses. The rest
//! of this suite bypasses the registry, so this is the only test that would
//! catch a wrong or missing registry value.
//!
//! Opt-in; see `e2e/harness.rs` and `e2e/README.md` for the preconditions,
//! plus `registration::RegisteredProvider` for what it needs beyond those.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use ak_ee_wcp_e2e::dll::{CLSID_CREDENTIAL_PROVIDER, build_output_dir, get_serialization};
use ak_ee_wcp_e2e::query_continue::query_continue;
use ak_ee_wcp_e2e::registration::{ComGuard, RegisteredProvider};
use ak_ee_wcp_e2e::user_array::{TestUser, user_array};
use ak_ee_wcp_e2e::{harness, mock_sysd, redirect_server::RedirectServer};

use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{
    CPGSR_RETURN_CREDENTIAL_FINISHED, CPSI_SUCCESS, CPUS_CREDUI,
    IConnectableCredentialProviderCredential, ICredentialProvider, ICredentialProviderSetUserArray,
};
use windows::core::Interface;

const HEADER_TOKEN: &str = "credui-registration-header-token";
const VALID_TOKEN: &str = "credui-registration-valid-token";
const USERNAME: &str = "credui-registration-user";

/// `multi_thread`, deliberately, even though real activation makes the DLL an
/// STA object (registered `ThreadingModel=Apartment`). `Connect()` below blocks
/// synchronously for as long as `ak_browser.exe`'s round trip against the mock
/// takes, and on a `current_thread` runtime that starves the only OS thread the
/// runtime has, so the mock server's task is never polled again to answer it —
/// a full deadlock, which hung CI for 30+ minutes.
///
/// Apartment safety comes from ordering instead: the one `.await` here
/// (`mock_sysd::start`) happens *before* `CoInitializeEx`, so once the COM
/// apartment exists there is nothing left to yield on and nothing that could
/// move this task to another worker thread.
#[tokio::test(flavor = "multi_thread")]
async fn registered_provider_completes_sign_in_via_real_com_activation() {
    if !harness::opted_in("registered_provider_completes_sign_in_via_real_com_activation") {
        return;
    }

    let dll_path = build_output_dir().join("ak_cred_provider.dll");
    assert!(
        dll_path.exists(),
        "expected {dll_path:?} to exist — build the workspace first"
    );
    let browser_exe = build_output_dir().join("ak_browser.exe");
    assert!(
        browser_exe.exists(),
        "expected {browser_exe:?} to exist — build the workspace first"
    );

    let server = RedirectServer::start(VALID_TOKEN).expect("start local redirect server");
    let _mock = mock_sysd::start(mock_sysd::MockConfig {
        interactive_auth_url: server.url.clone(),
        header_token: HEADER_TOKEN.to_string(),
        valid_token: VALID_TOKEN.to_string(),
        username: USERNAME.to_string(),
    })
    .await
    .expect("start mock ak-sysd — is a real ak-sysd already bound to the pipe?");

    // Before any COM interface below: Rust drops locals in reverse declaration
    // order, so `CoUninitialize` runs only once every interface obtained under
    // it has been released. `_registration` is earlier still and so runs last,
    // harmlessly — deleting the CLSID registration does not affect an
    // already-activated in-process instance.
    let _registration = RegisteredProvider::register(&dll_path)
        .expect("register ak_cred_provider.dll — needs an elevated shell");
    let _com = ComGuard::new().expect("CoInitializeEx");
    let _caps = harness::DebugCapabilities::enable()
        .expect("seed the Capabilities registry key — needs an elevated shell");

    // The activation this test exists for: found via the registry, loaded and
    // instantiated by ole32 itself rather than by us.
    let provider: ICredentialProvider =
        unsafe { CoCreateInstance(&CLSID_CREDENTIAL_PROVIDER, None, CLSCTX_INPROC_SERVER) }
            .expect("CoCreateInstance(CLSID_CREDENTIAL_PROVIDER) — is InprocServer32 registered?");

    unsafe {
        provider
            .SetUsageScenario(CPUS_CREDUI, 0)
            .expect("SetUsageScenario(CPUS_CREDUI) under the debug capability");

        let set_users: ICredentialProviderSetUserArray = provider
            .cast()
            .expect("provider must implement ICredentialProviderSetUserArray");
        set_users
            .SetUserArray(&user_array(vec![TestUser::non_local(USERNAME)]))
            .expect("SetUserArray");
    }

    let credential = unsafe { provider.GetCredentialAt(0) }
        .expect("GetCredentialAt(0) — SetUserArray should have produced one credential");
    let connectable: IConnectableCredentialProviderCredential = credential
        .cast()
        .expect("credential must implement IConnectableCredentialProviderCredential");

    let (qcws, _probe) = query_continue(None);
    unsafe { connectable.Connect(&qcws) }.expect("Connect should drive the sign-in to completion");

    let (response, icon, serialization, status_text) = unsafe { get_serialization(&credential) };
    assert_eq!(
        response, CPGSR_RETURN_CREDENTIAL_FINISHED,
        "expected a finished credential, status text was {status_text:?}"
    );
    assert_eq!(icon, CPSI_SUCCESS);
    assert!(
        serialization.cbSerialization > 0 && !serialization.rgbSerialization.is_null(),
        "serialization buffer should be populated"
    );
    assert_eq!(
        serialization.clsidCredentialProvider,
        CLSID_CREDENTIAL_PROVIDER
    );

    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(
            serialization.rgbSerialization as *const _,
        ));
    }
}
