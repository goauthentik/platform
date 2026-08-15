//! Loads the real built `ak_cred_provider.dll` and checks the tile matches
//! `wire::TILE_FIELDS` exactly.
//!
//! Hermetic, and deliberately so: `GetFieldDescriptorCount`/`GetFieldDescriptorAt`
//! read a static table and never consult the usage scenario, so this needs no
//! `SetUsageScenario` call — and therefore no mock `ak-sysd` to answer
//! `sys_caps` and no HKLM capability seeding. It runs on every `cargo test`,
//! unlike the opt-in process-level tests in `sign_in_flow.rs`.
#![allow(clippy::expect_used, clippy::panic)]

use e2e::dll::LoadedProvider;
use windows::Win32::System::Com::CoTaskMemFree;

#[test]
fn tile_fields_match_wire_layout() {
    let dll_path = e2e::dll::build_output_dir().join("ak_cred_provider.dll");
    assert!(
        dll_path.exists(),
        "expected {dll_path:?} to exist — build the workspace first"
    );

    let loaded = LoadedProvider::load(&dll_path).expect("load ak_cred_provider.dll");

    unsafe {
        let count = loaded
            .provider()
            .GetFieldDescriptorCount()
            .expect("GetFieldDescriptorCount");
        assert_eq!(count as usize, wire::TILE_FIELDS.len());

        for (i, expected) in wire::TILE_FIELDS.iter().enumerate() {
            let descriptor_ptr = loaded
                .provider()
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
