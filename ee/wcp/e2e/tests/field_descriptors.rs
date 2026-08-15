//! Drives the real built `ak_cred_provider.dll` under `CPUS_CREDUI` (works
//! on an ordinary interactive desktop — no secure desktop needed) against a
//! mock `ak-sysd`, and checks the tile matches `wire::TILE_FIELDS` exactly.
#![allow(clippy::expect_used, clippy::panic)]

use e2e::{dll::LoadedProvider, harness, mock_sysd};
use windows::Win32::{System::Com::CoTaskMemFree, UI::Shell::CPUS_CREDUI};

#[tokio::test(flavor = "multi_thread")]
async fn tile_fields_match_wire_layout() {
    if !harness::opted_in("tile_fields_match_wire_layout") {
        return;
    }

    let _mock = mock_sysd::start(mock_sysd::MockConfig {
        interactive_auth_url: "http://127.0.0.1:0/unused".to_string(),
        header_token: "unused".to_string(),
        valid_token: "unused".to_string(),
        username: "unused".to_string(),
    })
    .await
    .expect("start mock ak-sysd");

    let dll_path = e2e::dll::build_output_dir().join("ak_cred_provider.dll");
    assert!(
        dll_path.exists(),
        "expected {dll_path:?} to exist — build the workspace first"
    );

    let _caps = harness::DebugCapabilities::enable()
        .expect("seed the Capabilities registry key — needs an elevated shell");

    let loaded = LoadedProvider::load(&dll_path).expect("load ak_cred_provider.dll");

    unsafe {
        loaded
            .provider
            .SetUsageScenario(CPUS_CREDUI, 0)
            .expect("SetUsageScenario(CPUS_CREDUI) should succeed under the debug capability");

        let count = loaded
            .provider
            .GetFieldDescriptorCount()
            .expect("GetFieldDescriptorCount");
        assert_eq!(count as usize, wire::TILE_FIELDS.len());

        for (i, expected) in wire::TILE_FIELDS.iter().enumerate() {
            let descriptor_ptr = loaded
                .provider
                .GetFieldDescriptorAt(i as u32)
                .unwrap_or_else(|_| panic!("GetFieldDescriptorAt({i})"));
            assert!(!descriptor_ptr.is_null());
            let descriptor = &*descriptor_ptr;
            assert_eq!(descriptor.dwFieldID, i as u32);

            if !descriptor.pszLabel.is_null() {
                let label = descriptor.pszLabel.to_string().unwrap_or_default();
                assert_eq!(label, expected.text);
                CoTaskMemFree(Some(descriptor.pszLabel.0 as *const _));
            }
            CoTaskMemFree(Some(descriptor_ptr as *const _));
        }
    }
}
