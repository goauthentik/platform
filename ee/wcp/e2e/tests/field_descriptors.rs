//! Loads the real built `ak_cred_provider.dll` and checks the tile: the field
//! table matches `ak_ee_wcp_wire::TILE_FIELDS`, and the tile image loads.
//!
//! Hermetic, deliberately: none of these paths consult the usage scenario, so
//! there is no `SetUsageScenario` call and hence no mock `ak-sysd` and no HKLM
//! seeding. Unlike `sign_in_flow.rs`, these run on every `cargo test`.
#![allow(clippy::expect_used, clippy::panic)]

use ak_ee_wcp_e2e::dll::LoadedProvider;
use ak_ee_wcp_e2e::user_array::{TestUser, user_array};
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::ICredentialProviderSetUserArray;
use windows::core::Interface;

#[test]
fn tile_fields_match_wire_layout() {
    let dll_path = ak_ee_wcp_e2e::dll::build_output_dir().join("ak_cred_provider.dll");
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
        assert_eq!(count as usize, ak_ee_wcp_wire::TILE_FIELDS.len());

        for (i, expected) in ak_ee_wcp_wire::TILE_FIELDS.iter().enumerate() {
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

/// The blank-tile regression: `GetBitmapValue`, what LogonUI calls to paint the
/// tile, fails whenever the embedded BMP is not something `LoadImage` accepts.
/// A 32-bit `BITMAPV5HEADER`/`BI_BITFIELDS` export fails for every flag
/// combination and renders blank with no other symptom; the image must carry no
/// alpha channel either.
#[test]
fn tile_image_loads_from_the_embedded_resource() {
    let dll_path = ak_ee_wcp_e2e::dll::build_output_dir().join("ak_cred_provider.dll");
    assert!(dll_path.exists(), "expected {dll_path:?} to exist");

    let loaded = LoadedProvider::load(&dll_path).expect("load ak_cred_provider.dll");

    let tile_image_field = ak_ee_wcp_wire::TILE_FIELDS
        .iter()
        .position(|f| f.kind == ak_ee_wcp_wire::FieldKind::TileImage)
        .expect("a tile-image field") as u32;

    unsafe {
        // `GetCredentialAt` needs an enumerated user but not a usage scenario,
        // so this stays hermetic.
        let set_users: ICredentialProviderSetUserArray =
            loaded.provider().cast().expect("SetUserArray interface");
        set_users
            .SetUserArray(&user_array(vec![TestUser::non_local("tile-test-user")]))
            .expect("SetUserArray");

        let credential = loaded
            .provider()
            .GetCredentialAt(0)
            .expect("GetCredentialAt(0)");

        let bitmap = credential
            .GetBitmapValue(tile_image_field)
            .expect("GetBitmapValue must succeed — a blank tile is the symptom when it doesn't");
        assert!(!bitmap.is_invalid(), "tile bitmap handle should be valid");
        let _ = DeleteObject(bitmap.into());
    }
}
