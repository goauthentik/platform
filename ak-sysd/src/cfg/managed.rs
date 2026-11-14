use ak_platform_serde_policy::PolicySource;
use eyre::Result;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct SysdManagedConfig {
    #[serde(rename = "RegistrationToken", alias = "registration_token")]
    pub registration_token: String,
    #[serde(rename = "URL", alias = "url")]
    pub url: String,
}

const LINUX_MANAGED_CONFIG_PATH: &str = "/etc/authentik/managed.json";

/// MDM-pushed enrollment config. `Ok(None)` unless both values are set.
pub fn load_managed_config() -> Result<Option<SysdManagedConfig>> {
    // Unlike other policies this holds a secret, so it must not be readable by others either.
    #[cfg(target_os = "linux")]
    if let Ok(meta) = std::fs::metadata(LINUX_MANAGED_CONFIG_PATH) {
        use std::os::unix::fs::MetadataExt as _;
        if meta.mode() & 0o077 != 0 {
            eyre::bail!(
                "Managed config has incorrect permissions: {:o}, should be 0400",
                meta.mode() & 0o777
            );
        }
    }
    let cfg = ak_platform_serde_policy::load::<SysdManagedConfig>(&PolicySource {
        macos_domain: "io.goauthentik.platform",
        windows_key: r"SOFTWARE\authentik Security Inc.\Platform\ManagedConfig",
        file_path: LINUX_MANAGED_CONFIG_PATH,
    })?;
    Ok(cfg.filter(|c| !c.url.is_empty() && !c.registration_token.is_empty()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn reads_platform_and_linux_key_names() {
        let mdm: SysdManagedConfig =
            serde_json::from_str(r#"{"URL": "https://ak", "RegistrationToken": "t"}"#).unwrap();
        let file: SysdManagedConfig =
            serde_json::from_str(r#"{"url": "https://ak", "registration_token": "t"}"#).unwrap();
        for c in [mdm, file] {
            assert_eq!(c.url, "https://ak");
            assert_eq!(c.registration_token, "t");
        }
    }
}
