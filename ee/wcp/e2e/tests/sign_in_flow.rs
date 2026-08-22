//! The full process-level flow: the real `ak_cred_provider.dll` loaded via
//! `LoadLibraryW`, handed a user array, driven through `Connect()` — which
//! spawns the real `ak_browser.exe`, which loads a local page, follows its
//! `goauthentik.io://` redirect and validates the token against a mock
//! `ak-sysd` — down to the buffer `GetSerialization` hands back.
//!
//! Opt-in; see `e2e/harness.rs` and `e2e/README.md` for the preconditions.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use ak_ee_wcp_e2e::query_continue::query_continue;
use ak_ee_wcp_e2e::user_array::{TestUser, user_array};
use ak_ee_wcp_e2e::{
    dll::{LoadedProvider, get_serialization},
    harness, mock_sysd,
    redirect_server::RedirectServer,
};

use windows::Win32::UI::Shell::{
    CPGSR_NO_CREDENTIAL_FINISHED, CPGSR_RETURN_CREDENTIAL_FINISHED, CPSI_SUCCESS, CPSI_WARNING,
    CPUS_CREDUI, IConnectableCredentialProviderCredential, ICredentialProviderSetUserArray,
};
use windows::Win32::UI::WindowsAndMessaging::WS_EX_TOPMOST;
use windows::core::Interface;

const HEADER_TOKEN: &str = "header-token-under-test";
const VALID_TOKEN: &str = "valid-token-under-test";
const USERNAME: &str = "e2e-user";

struct Fixture {
    _mock: mock_sysd::MockSysd,
    _caps: harness::DebugCapabilities,
    server: RedirectServer,
    provider: LoadedProvider,
}

/// Brings up `server` + mock `ak-sysd`, seeds the debug capability, and loads
/// the real DLL with `user` as the only enumerated account.
async fn setup(user: TestUser, server: RedirectServer) -> Fixture {
    let mock = mock_sysd::start(mock_sysd::MockConfig {
        interactive_auth_url: server.url.clone(),
        header_token: HEADER_TOKEN.to_string(),
        valid_token: VALID_TOKEN.to_string(),
        username: USERNAME.to_string(),
    })
    .await
    .expect("start mock ak-sysd — is a real ak-sysd already bound to the pipe?");

    let caps = harness::DebugCapabilities::enable()
        .expect("seed the Capabilities registry key — needs an elevated shell");

    let dll_path = ak_ee_wcp_e2e::dll::build_output_dir().join("ak_cred_provider.dll");
    assert!(
        dll_path.exists(),
        "expected {dll_path:?} to exist — build the workspace first"
    );
    let browser_exe = ak_ee_wcp_e2e::dll::build_output_dir().join("ak_browser.exe");
    assert!(
        browser_exe.exists(),
        "expected {browser_exe:?} to exist — build the workspace first"
    );

    let provider = LoadedProvider::load(&dll_path).expect("load ak_cred_provider.dll");

    unsafe {
        provider
            .provider()
            .SetUsageScenario(CPUS_CREDUI, 0)
            .expect("SetUsageScenario(CPUS_CREDUI) under the debug capability");

        let set_users: ICredentialProviderSetUserArray = provider
            .provider()
            .cast()
            .expect("provider must implement ICredentialProviderSetUserArray");
        set_users
            .SetUserArray(&user_array(vec![user]))
            .expect("SetUserArray");
    }

    Fixture {
        _mock: mock,
        _caps: caps,
        server,
        provider,
    }
}

/// Looks up the Negotiate authentication package the same way the DLL does.
///
/// The DLL is loaded in-process, so this runs under the same token and session
/// and must observe the same answer. Asserting equality rather than "non-zero"
/// matters: `LsaLookupAuthenticationPackage` hands back an index into lsass's
/// authentication package table, assigned at load order, and 0 is a legal id —
/// nothing about the runner's package table is ours to assume.
fn negotiate_package() -> u32 {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::Authentication::Identity::{
        LSA_STRING, LsaConnectUntrusted, LsaDeregisterLogonProcess, LsaLookupAuthenticationPackage,
    };
    use windows::core::PSTR;

    let mut lsa_handle = HANDLE::default();
    unsafe { LsaConnectUntrusted(&mut lsa_handle) }
        .ok()
        .expect("LsaConnectUntrusted");

    let name = b"Negotiate\0";
    let lsa_name = LSA_STRING {
        Length: (name.len() - 1) as u16,
        MaximumLength: name.len() as u16,
        Buffer: PSTR(name.as_ptr() as *mut u8),
    };

    let mut package = 0u32;
    let status = unsafe { LsaLookupAuthenticationPackage(lsa_handle, &lsa_name, &mut package) };
    unsafe {
        let _ = LsaDeregisterLogonProcess(lsa_handle);
    }
    assert_eq!(
        status.0, 0,
        "LsaLookupAuthenticationPackage(\"Negotiate\") should succeed"
    );
    package
}

/// A completed sign-in for a non-local (domain/UPN) account: no local
/// password reset, so this needs no account on the machine and serializes
/// through `CredPackAuthenticationBufferW`.
#[tokio::test(flavor = "multi_thread")]
async fn completed_sign_in_serializes_a_credential() {
    if !harness::opted_in("completed_sign_in_serializes_a_credential") {
        return;
    }

    let server = RedirectServer::start(VALID_TOKEN).expect("start local redirect server");
    let fixture = setup(TestUser::non_local(USERNAME), server).await;

    let credential = unsafe { fixture.provider.provider().GetCredentialAt(0) }
        .expect("GetCredentialAt(0) — SetUserArray should have produced one credential");
    let connectable: IConnectableCredentialProviderCredential = credential
        .cast()
        .expect("credential must implement IConnectableCredentialProviderCredential");

    let (qcws, probe) = query_continue(None);
    unsafe { connectable.Connect(&qcws) }.expect("Connect should drive the sign-in to completion");

    assert!(
        probe
            .status_messages()
            .iter()
            .any(|m| m.contains("authentik")),
        "Connect should have set a status message, got {:?}",
        probe.status_messages()
    );

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
        serialization.ulAuthenticationPackage,
        negotiate_package(),
        "the serialization should carry the Negotiate package LSA resolves for this process"
    );
    assert_eq!(
        serialization.clsidCredentialProvider,
        ak_ee_wcp_e2e::dll::CLSID_CREDENTIAL_PROVIDER
    );

    unsafe {
        windows::Win32::System::Com::CoTaskMemFree(Some(
            serialization.rgbSerialization as *const _,
        ));
    }

    // `browser-host` must put the interactive-auth header on every request it
    // makes, which is what let the (mock) backend hand back a sign-in page.
    let headers = fixture.server.observed_auth_headers();
    assert!(
        !headers.is_empty(),
        "the sign-in window never fetched the page"
    );
    assert!(
        headers.iter().all(|h| h.as_deref() == Some(HEADER_TOKEN)),
        "every request should carry {}: {HEADER_TOKEN}, got {headers:?}",
        ak_ee_wcp_wire::AUTH_HEADER_NAME
    );
}

/// At a real logon the window opens behind LogonUI unless it is put in the
/// topmost band explicitly, with no sign it exists short of an alt-tab.
///
/// Only the z-order is asserted. Taking the foreground is a race, and a race
/// asserted on a CI desktop is a test that gets disabled. `CPUS_CREDUI` is the
/// only scenario reachable outside a real logon prompt, so the secure desktop
/// stays on the manual checklist in `e2e/README.md`.
#[tokio::test(flavor = "multi_thread")]
async fn the_sign_in_window_opens_topmost() {
    if !harness::opted_in("the_sign_in_window_opens_topmost") {
        return;
    }

    // Never redirects, so the window is still up while it is being looked at.
    let server = RedirectServer::start_inert().expect("start local redirect server");
    let fixture = setup(TestUser::non_local(USERNAME), server).await;

    let credential =
        unsafe { fixture.provider.provider().GetCredentialAt(0) }.expect("GetCredentialAt(0)");
    let connectable: IConnectableCredentialProviderCredential = credential
        .cast()
        .expect("credential must implement IConnectableCredentialProviderCredential");

    let sighting = ak_ee_wcp_e2e::sign_in_window::watch(std::time::Duration::from_secs(30));

    // `Connect` blocks until the flow ends, so the window can only be observed
    // from the watcher thread. `QueryContinue` is polled every 200ms while it
    // waits, so this gives the window ~20s to appear before backing out.
    let (qcws, _probe) = query_continue(Some(100));
    unsafe { connectable.Connect(&qcws) }.expect("Connect returns Ok even when cancelled");

    let observed = sighting
        .recv()
        .expect("the window watcher thread should report")
        .expect("ak_browser.exe never revealed its sign-in window");

    assert!(
        observed.extended_style & WS_EX_TOPMOST.0 != 0,
        "the sign-in window should be topmost, got ex-style {:#x} after {:?}",
        observed.extended_style,
        observed.appeared_after
    );
}

/// LogonUI withdrawing consent mid-flow: `QueryContinue` starts failing, the
/// provider signals `ak_browser.exe` over the control pipe, and the flow reports
/// a cancellation rather than a credential.
#[tokio::test(flavor = "multi_thread")]
async fn cancelling_mid_flow_yields_no_credential() {
    if !harness::opted_in("cancelling_mid_flow_yields_no_credential") {
        return;
    }

    // Point the sign-in window at a page that never redirects, so the flow is
    // still waiting when cancellation arrives.
    let server = RedirectServer::start_inert().expect("start local redirect server");
    let fixture = setup(TestUser::non_local(USERNAME), server).await;

    let credential =
        unsafe { fixture.provider.provider().GetCredentialAt(0) }.expect("GetCredentialAt(0)");
    let connectable: IConnectableCredentialProviderCredential = credential
        .cast()
        .expect("credential must implement IConnectableCredentialProviderCredential");

    let (qcws, probe) = query_continue(Some(1));
    unsafe { connectable.Connect(&qcws) }.expect("Connect returns Ok even when cancelled");
    assert!(
        probe.cancelled(),
        "QueryContinue should have reported abort"
    );

    let (response, icon, _serialization, status_text) = unsafe { get_serialization(&credential) };
    assert_eq!(response, CPGSR_NO_CREDENTIAL_FINISHED);
    assert_eq!(icon, CPSI_WARNING);
    assert_eq!(status_text, "Login attempt cancelled");
}
