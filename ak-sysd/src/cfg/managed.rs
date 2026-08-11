use eyre::Result;

pub struct SysdManagedConfig {
    pub registration_token: String,
    pub url: String,
}

/// Reads MDM-pushed enrollment config, mirroring Go's `managed_config`
/// package. Returns `Ok(None)` if there is no managed config source on this
/// platform or no value is currently set (both are normal, not errors).
#[cfg(target_os = "macos")]
pub fn load_managed_config() -> Result<Option<SysdManagedConfig>> {
    // `defaults read` resolves the same preferences domain
    // (`io.goauthentik.platform`) that Go's CFPreferencesCopyAppValue call
    // reads from. Shelling out avoids a raw Core Foundation FFI dependency;
    // if strict merging of the MDM-managed preferences layer (as opposed to
    // this host's local preferences) turns out to matter, replace this with
    // a direct CFPreferencesCopyAppValue call instead.
    let read = |key: &str| -> Option<String> {
        let out = std::process::Command::new("defaults")
            .args(["read", "io.goauthentik.platform", key])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let val = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if val.is_empty() { None } else { Some(val) }
    };

    let Some(url) = read("URL") else {
        return Ok(None);
    };
    let Some(registration_token) = read("RegistrationToken") else {
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
