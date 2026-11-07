//! Settings users set in their config file and admins enforce with a policy (see
//! [`super::policy`]). Both use the same schema (`settings.schema.json`): anything set in
//! the policy wins over the user's value, except durations, where the shorter one applies.
use std::time::Duration;

use ak_platform_authz::Operation;
use eyre::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::policy::policy;

/// Settings of the authentik agent. Unset values use their defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Settings {
    /// Log at trace instead of info level. Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<bool>,
    /// Optional fallback/passthrough SSH agent used for hosts not managed by authentik
    /// (e.g. github.com, gitlab.com). When unset, nothing is forwarded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh_fallback_agent: Option<SshFallbackAgentConfig>,
    #[serde(skip_serializing_if = "is_default")]
    pub profile_management: ProfileManagementSettings,
    #[serde(skip_serializing_if = "is_default")]
    pub authorization: AuthorizationSettings,
    #[serde(skip_serializing_if = "is_default")]
    pub ssh: SshSettings,
    #[serde(skip_serializing_if = "is_default")]
    pub telemetry: TelemetrySettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SshFallbackAgentConfig {
    /// Path to the fallback SSH agent socket (e.g. the system's original
    /// `SSH_AUTH_SOCK`, or a hardware-key agent) used for hosts not managed
    /// by authentik.
    pub socket_path: String,
    /// Extra hosts that should always be routed to the fallback agent,
    /// bypassing authentik entirely, given as `known_hosts`-format lines
    /// (e.g. the output of `ssh-keyscan your-host.example.com`).
    #[serde(default)]
    pub extra_passthrough_hosts: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct ProfileManagementSettings {
    /// authentik URLs profiles may be set up for. Default: any URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_urls: Option<Vec<String>>,
    /// Whether profiles can be deleted. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_delete: Option<bool>,
    /// Whether tokens may be stored in plain text in the config file when no keyring is
    /// available. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_plaintext_tokens: Option<bool>,
    /// Only allow profiles whose tokens are bound to a key on this device (DPoP). Needs a
    /// provider that supports key binding. Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_dpop: Option<bool>,
}

impl ProfileManagementSettings {
    pub fn check_url(&self, url: &str) -> Result<()> {
        let Some(allowed) = &self.allowed_urls else {
            return Ok(());
        };
        let url = url.trim_end_matches('/');
        if allowed
            .iter()
            .any(|a| a.trim_end_matches('/').eq_ignore_ascii_case(url))
        {
            return Ok(());
        }
        bail!("authentik URL '{url}' is not allowed by policy")
    }

    pub fn allow_delete(&self) -> bool {
        self.allow_delete.unwrap_or(true)
    }

    pub fn allow_plaintext_tokens(&self) -> bool {
        self.allow_plaintext_tokens.unwrap_or(true)
    }

    pub fn require_dpop(&self) -> bool {
        self.require_dpop.unwrap_or(false)
    }

    /// Fails for a profile without a DPoP key when the policy requires one.
    pub fn check_dpop(&self, dpop_bound: bool) -> Result<()> {
        if self.require_dpop() && !dpop_bound {
            bail!("profile is not key-bound (DPoP), which is required by policy");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct AuthorizationSettings {
    /// How long answers to authorization prompts are remembered for the same caller.
    #[serde(skip_serializing_if = "is_default")]
    pub remember: RememberSettings,
    /// Audiences (client IDs) tokens may be exchanged for. Default: any audience.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_exchange_audiences: Option<Vec<String>>,
}

impl AuthorizationSettings {
    /// How long an allowed and a denied answer to a prompt for `op` are remembered.
    pub fn remember(&self, op: Operation) -> (Duration, Duration) {
        let r = &self.remember;
        let (set, (allow, deny)) = match op {
            Operation::WhoAmI => (&r.whoami, (0, 0)),
            Operation::CurrentToken => (&r.current_token, (5 * 60, 15 * 60)),
            Operation::TokenExchange => (&r.token_exchange, (30 * 60, 1)),
            Operation::SshLogin => (&r.ssh_login, (30 * 60, 5 * 60)),
            Operation::Authorize => (&r.authorize, (2 * 60 * 60, 5 * 60)),
        };
        (
            Duration::from_secs(set.allow.unwrap_or(allow)),
            Duration::from_secs(set.deny.unwrap_or(deny)),
        )
    }

    pub fn exchange_allowed(&self, audience: &str) -> bool {
        self.allowed_exchange_audiences
            .as_ref()
            .is_none_or(|a| a.iter().any(|x| x == audience))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct RememberSettings {
    /// Reading account info. Default: 0s allowed, 0s denied.
    #[serde(skip_serializing_if = "is_default")]
    pub whoami: Remember,
    /// Handing out the session token. Default: 300s allowed, 900s denied.
    #[serde(skip_serializing_if = "is_default")]
    pub current_token: Remember,
    /// Exchanging a token for another application. Default: 1800s allowed, 1s denied.
    #[serde(skip_serializing_if = "is_default")]
    pub token_exchange: Remember,
    /// Logging in to an SSH host. Default: 1800s allowed, 300s denied.
    #[serde(skip_serializing_if = "is_default")]
    pub ssh_login: Remember,
    /// Generic approvals for a service the caller names. Default: 7200s allowed, 300s denied.
    #[serde(skip_serializing_if = "is_default")]
    pub authorize: Remember,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Remember {
    /// Seconds an allowed request isn't prompted for again.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<u64>,
    /// Seconds a denied request is denied again without prompting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct SshSettings {
    /// Serve the SSH agent. Takes effect after a restart. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Answer authentik agent tunnel requests from forwarded SSH agents, which lets remote
    /// hosts call this agent (authorization prompts still apply). Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_tunnel: Option<bool>,
    /// Send github.com and gitlab.com straight to the fallback agent. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builtin_passthrough_hosts: Option<bool>,
}

impl SshSettings {
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn agent_tunnel(&self) -> bool {
        self.agent_tunnel.unwrap_or(true)
    }

    pub fn builtin_passthrough_hosts(&self) -> bool {
        self.builtin_passthrough_hosts.unwrap_or(true)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct TelemetrySettings {
    /// Check for new releases. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_check: Option<bool>,
    /// Send errors and performance traces to authentik's Sentry. Default: true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reporting: Option<bool>,
}

impl TelemetrySettings {
    pub fn update_check(&self) -> bool {
        self.update_check.unwrap_or(true)
    }

    pub fn error_reporting(&self) -> bool {
        self.error_reporting.unwrap_or(true)
    }
}

impl Settings {
    pub fn debug(&self) -> bool {
        self.debug.unwrap_or(false)
    }

    /// These (user) settings with the admin [`policy`] enforced on top.
    pub fn effective(&self) -> Settings {
        self.with_policy(policy())
    }

    fn with_policy(&self, policy: &Settings) -> Settings {
        let (Ok(mut user), Ok(policy_v)) =
            (serde_json::to_value(self), serde_json::to_value(policy))
        else {
            return policy.clone();
        };
        enforce(&mut user, policy_v);
        serde_json::from_value(user).unwrap_or_else(|_| policy.clone())
    }
}

/// Lays `policy` over `user`: objects merge key by key, numbers (durations) keep the lower
/// value and anything else is replaced. Unset values aren't serialized, so they never override.
fn enforce(user: &mut Value, policy: Value) {
    match (user, policy) {
        (Value::Object(user), Value::Object(policy)) => {
            for (k, p) in policy {
                match user.get_mut(&k) {
                    Some(u) => enforce(u, p),
                    None => {
                        user.insert(k, p);
                    }
                }
            }
        }
        (Value::Number(user), Value::Number(policy)) => {
            if policy.as_u64() < user.as_u64() {
                *user = policy;
            }
        }
        (user, policy) => *user = policy,
    }
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(v: Value) -> Settings {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn policy_overrides_user_except_longer_durations() {
        let user = settings(serde_json::json!({
            "debug": true,
            "ssh": { "enabled": true, "agent_tunnel": true },
            "authorization": { "remember": { "current_token": { "allow": 60, "deny": 60 } } },
            "ssh_fallback_agent": { "socket_path": "/user", "extra_passthrough_hosts": ["a"] },
        }));
        let policy = settings(serde_json::json!({
            "ssh": { "agent_tunnel": false },
            "authorization": { "remember": { "current_token": { "allow": 30, "deny": 600 } } },
            "ssh_fallback_agent": { "socket_path": "/admin" },
        }));
        let eff = user.with_policy(&policy);

        assert!(eff.debug());
        assert!(eff.ssh.enabled());
        assert!(!eff.ssh.agent_tunnel());
        assert_eq!(
            eff.authorization.remember(Operation::CurrentToken),
            (Duration::from_secs(30), Duration::from_secs(60))
        );
        assert_eq!(
            eff.ssh_fallback_agent,
            Some(SshFallbackAgentConfig {
                socket_path: "/admin".into(),
                extra_passthrough_hosts: vec![],
            })
        );
        assert_eq!(
            eff.authorization.remember(Operation::SshLogin),
            (Duration::from_secs(30 * 60), Duration::from_secs(5 * 60))
        );
    }

    #[test]
    fn allowed_urls() {
        let p = ProfileManagementSettings {
            allowed_urls: Some(vec!["https://auth.example.com/".into()]),
            ..Default::default()
        };
        assert!(p.check_url("https://AUTH.example.com").is_ok());
        assert!(p.check_url("https://evil.example.com").is_err());
        assert!(ProfileManagementSettings::default().check_url("x").is_ok());
    }

    #[test]
    fn require_dpop() {
        let p = ProfileManagementSettings {
            require_dpop: Some(true),
            ..Default::default()
        };
        assert!(p.check_dpop(true).is_ok());
        assert!(p.check_dpop(false).is_err());
        assert!(ProfileManagementSettings::default().check_dpop(false).is_ok());
    }

    #[test]
    fn unset_settings_are_not_written() {
        assert_eq!(serde_json::to_string(&Settings::default()).unwrap(), "{}");
    }

    /// Regenerate with `UPDATE_SCHEMA=1 cargo test -p ak-agent schema_is_up_to_date`.
    #[test]
    fn schema_is_up_to_date() {
        let schema = serde_json::to_string_pretty(&schemars::schema_for!(Settings)).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/settings.schema.json");
        if std::env::var_os("UPDATE_SCHEMA").is_some() {
            std::fs::write(path, &schema).unwrap();
        }
        assert_eq!(std::fs::read_to_string(path).unwrap_or_default(), schema);
    }
}
