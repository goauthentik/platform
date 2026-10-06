//! Maps the shared `ak_ee_wcp_wire::TILE_FIELDS` table onto the COM field
//! descriptor/state types LogonUI expects, and loads the tile bitmap embedded
//! as a Win32 resource.

use windows::{
    Win32::{
        Graphics::Gdi::HBITMAP,
        UI::Shell::{
            CPCFO_ENABLE_TOUCH_KEYBOARD_AUTO_INVOKE, CPCFO_NONE, CPFG_CREDENTIAL_PROVIDER_LABEL,
            CPFG_CREDENTIAL_PROVIDER_LOGO, CPFG_STANDALONE_SUBMIT_BUTTON, CPFIS_FOCUSED,
            CPFIS_NONE, CPFS_DISPLAY_IN_BOTH, CPFS_DISPLAY_IN_SELECTED_TILE, CPFS_HIDDEN,
            CPFT_LARGE_TEXT, CPFT_SMALL_TEXT, CPFT_SUBMIT_BUTTON, CPFT_TILE_IMAGE,
            CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS, CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR,
            CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE, CREDENTIAL_PROVIDER_FIELD_STATE,
            CREDENTIAL_PROVIDER_FIELD_TYPE,
        },
        UI::WindowsAndMessaging::{IMAGE_BITMAP, LR_CREATEDIBSECTION, LR_DEFAULTSIZE, LoadImageW},
    },
    core::GUID,
};

use crate::strings::cotask_pwstr;
use ak_ee_wcp_wire::{FieldKind, TILE_FIELDS};

/// The tile-image resource ID embedded via `res/resource.rc`.
const TILE_IMAGE_RESOURCE_ID: u16 = 101;

pub fn field_count() -> u32 {
    TILE_FIELDS.len() as u32
}

fn cpft(kind: FieldKind) -> CREDENTIAL_PROVIDER_FIELD_TYPE {
    match kind {
        FieldKind::TileImage => CPFT_TILE_IMAGE,
        FieldKind::HiddenLabel => CPFT_SMALL_TEXT,
        FieldKind::LargeText => CPFT_LARGE_TEXT,
        FieldKind::SubmitButton => CPFT_SUBMIT_BUTTON,
    }
}

fn field_group_guid(kind: FieldKind) -> GUID {
    match kind {
        FieldKind::TileImage => CPFG_CREDENTIAL_PROVIDER_LOGO,
        FieldKind::HiddenLabel => CPFG_CREDENTIAL_PROVIDER_LABEL,
        FieldKind::SubmitButton => CPFG_STANDALONE_SUBMIT_BUTTON,
        FieldKind::LargeText => GUID::zeroed(),
    }
}

pub fn field_descriptor_at(
    index: u32,
) -> windows::core::Result<CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR> {
    let field = TILE_FIELDS
        .get(index as usize)
        .ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;

    Ok(CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR {
        dwFieldID: index,
        cpft: cpft(field.kind),
        pszLabel: cotask_pwstr(field.text),
        guidFieldType: field_group_guid(field.kind),
    })
}

/// Field visibility/interactive-state pair for `GetFieldState`. The label stays
/// hidden — it exists only to satisfy `CPFG_CREDENTIAL_PROVIDER_LABEL` — and the
/// submit button appears once the tile is selected.
pub fn field_state_at(
    index: u32,
) -> windows::core::Result<(
    CREDENTIAL_PROVIDER_FIELD_STATE,
    CREDENTIAL_PROVIDER_FIELD_INTERACTIVE_STATE,
)> {
    let field = TILE_FIELDS
        .get(index as usize)
        .ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;

    Ok(match field.kind {
        FieldKind::TileImage => (CPFS_DISPLAY_IN_BOTH, CPFIS_FOCUSED),
        FieldKind::HiddenLabel => (CPFS_HIDDEN, CPFIS_NONE),
        FieldKind::LargeText => (CPFS_DISPLAY_IN_BOTH, CPFIS_NONE),
        FieldKind::SubmitButton => (CPFS_DISPLAY_IN_SELECTED_TILE, CPFIS_NONE),
    })
}

/// Field options for `GetFieldOptions`: the tile image gets the touch keyboard
/// auto-invoked on it, so tapping it on a touch device opens the sign-in flow
/// without an extra tap to raise the keyboard first.
pub fn field_options_at(
    index: u32,
) -> windows::core::Result<CREDENTIAL_PROVIDER_CREDENTIAL_FIELD_OPTIONS> {
    let field = TILE_FIELDS
        .get(index as usize)
        .ok_or(windows::core::Error::from(
            windows::Win32::Foundation::E_INVALIDARG,
        ))?;
    Ok(if field.kind == FieldKind::TileImage {
        CPCFO_ENABLE_TOUCH_KEYBOARD_AUTO_INVOKE
    } else {
        CPCFO_NONE
    })
}

/// A fresh `HBITMAP` per call rather than a shared one: LogonUI takes ownership
/// and destroys it.
pub fn load_tile_bitmap() -> windows::core::Result<HBITMAP> {
    unsafe {
        let hinstance = windows::Win32::Foundation::HINSTANCE(crate::own_module().0);
        let handle = LoadImageW(
            Some(hinstance),
            windows::core::PCWSTR(TILE_IMAGE_RESOURCE_ID as *const u16),
            IMAGE_BITMAP,
            0,
            0,
            LR_CREATEDIBSECTION | LR_DEFAULTSIZE,
        )?;
        Ok(HBITMAP(handle.0))
    }
}
