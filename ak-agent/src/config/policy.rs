use std::sync::LazyLock;

use ak_platform_serde_policy::PolicySource;
use eyre::{Result, eyre};

use super::Settings;

static POLICY: LazyLock<std::result::Result<Settings, String>> = LazyLock::new(|| {
    ak_platform_serde_policy::load(&PolicySource {
        macos_domain: "io.goauthentik.platform.agent",
        windows_key: r"SOFTWARE\Policies\authentik Security Inc.\Platform\Agent",
        file_path: "/etc/authentik/agent-policy.json",
    })
    .map(Option::unwrap_or_default)
    .map_err(|e| format!("failed to load agent policy: {e:#}"))
});

static NO_POLICY: LazyLock<Settings> = LazyLock::new(Settings::default);

impl Settings {
    pub fn policy() -> &'static Settings {
        POLICY.as_ref().unwrap_or(&NO_POLICY)
    }

    /// Fails if a policy is set but can't be used, so it isn't silently left unenforced.
    pub fn check_policy() -> Result<()> {
        POLICY.as_ref().map(|_| ()).map_err(|e| eyre!("{e}"))
    }
}
