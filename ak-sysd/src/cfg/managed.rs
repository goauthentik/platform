use eyre::Result;

pub struct SysdManagedConfig {
    pub registration_token: String,
    pub url: String,
}

/// Reads MDM-pushed enrollment config, mirroring Go's `managed_config`
/// package. Returns `Ok(None)` if there is no managed config source on this
/// platform or no value is currently set (both are normal, not errors).
#[cfg(target_os = "macos")]
const MANAGED_APP_ID: &str = "io.goauthentik.platform";

/// Reads a single MDM-forced string preference.
///
/// `CFPreferencesCopyAppValue` is the API Go used, and it is the only way to
/// see the managed layer: profile-delivered values live in
/// `/Library/Managed Preferences/<app-id>.plist`, and `defaults read <app-id>`
/// does **not** resolve them — it reports "domain does not exist" on a machine
/// that plainly has the plist. Shelling out therefore made managed enrollment a
/// silent no-op on every managed Mac.
///
/// `CFPreferencesAppValueIsForced` gates on the value actually being managed.
/// `CopyAppValue` alone returns the *effective* value, which may come from
/// ordinary user defaults — and this value decides which authentik a root
/// daemon enrolls against, so anything not delivered by MDM is ignored.
#[cfg(target_os = "macos")]
fn managed_string(key: &str) -> Option<String> {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use core_foundation_sys::base::{CFGetTypeID, CFRelease};
    use core_foundation_sys::preferences::{
        CFPreferencesAppValueIsForced, CFPreferencesCopyAppValue,
    };
    use core_foundation_sys::string::CFStringRef;

    let cf_key = CFString::new(key);
    let cf_app = CFString::new(MANAGED_APP_ID);

    unsafe {
        if CFPreferencesAppValueIsForced(cf_key.as_concrete_TypeRef(), cf_app.as_concrete_TypeRef())
            == 0
        {
            return None;
        }

        let value =
            CFPreferencesCopyAppValue(cf_key.as_concrete_TypeRef(), cf_app.as_concrete_TypeRef());
        if value.is_null() {
            return None;
        }

        // Copy* returns +1; we own it from here.
        if CFGetTypeID(value) != CFString::type_id() {
            CFRelease(value);
            tracing::warn!(key, "managed preference is not a string, ignoring");
            return None;
        }
        let s = CFString::wrap_under_create_rule(value as CFStringRef).to_string();
        if s.is_empty() { None } else { Some(s) }
    }
}

#[cfg(target_os = "macos")]
pub fn load_managed_config() -> Result<Option<SysdManagedConfig>> {
    let Some(url) = managed_string("URL") else {
        return Ok(None);
    };
    let Some(registration_token) = managed_string("RegistrationToken") else {
        return Ok(None);
    };
    Ok(Some(SysdManagedConfig {
        registration_token,
        url,
    }))
}

#[cfg(target_os = "windows")]
pub fn load_managed_config() -> Result<Option<SysdManagedConfig>> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = match hklm.open_subkey(r"SOFTWARE\authentik Security Inc.\Platform\ManagedConfig") {
        Ok(k) => k,
        Err(_) => return Ok(None),
    };
    let url: String = match key.get_value("URL") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let registration_token: String = match key.get_value("RegistrationToken") {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(Some(SysdManagedConfig {
        registration_token,
        url,
    }))
}

#[cfg(target_os = "linux")]
pub fn load_managed_config() -> Result<Option<SysdManagedConfig>> {
    // Go has no managed-config source on Linux either.
    Ok(None)
}
