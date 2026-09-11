//! The supported-model registry and policy data. Every supported model is
//! declared once in [`models`]; the allow-list and per-attribute rules are
//! derived from it, so they can never drift apart.

use std::collections::BTreeMap;

use crate::mcp::blueprint::yaml::Plain;

/// How a single attribute is gated.
#[derive(Debug, Clone, PartialEq)]
pub enum Bin {
    /// Always permitted as-is.
    Pass,
    /// Permitted, but surfaced to the operator for review.
    Flag,
    /// Pinned to a required literal value.
    Force(Plain),
    /// A duration bounded to at most `max_seconds`.
    Cap(u64),
    /// A relationship field: requires a permitted reference (curated `!Find`
    /// or in-blueprint `!KeyOf`).
    Ref,
}

/// 24h cap; adjust to the admin global max when that exists.
const TOKEN_MAX: u64 = 60 * 60 * 24;

/// Per-model attribute rules, keyed by model name. Ordered map so the derived
/// model list is deterministic.
pub fn models() -> BTreeMap<&'static str, BTreeMap<&'static str, Bin>> {
    let mut m: BTreeMap<&'static str, BTreeMap<&'static str, Bin>> = BTreeMap::new();

    let mut application: BTreeMap<&'static str, Bin> = BTreeMap::new();
    application.insert("name", Bin::Pass);
    application.insert("slug", Bin::Pass);
    application.insert("group", Bin::Pass);
    application.insert("meta_launch_url", Bin::Pass);
    application.insert("meta_description", Bin::Pass);
    application.insert("meta_publisher", Bin::Pass);
    application.insert("meta_icon", Bin::Pass);
    application.insert("provider", Bin::Ref);
    m.insert("authentik_core.application", application);

    let mut oauth2: BTreeMap<&'static str, Bin> = BTreeMap::new();
    oauth2.insert("name", Bin::Pass);
    oauth2.insert("client_type", Bin::Flag);
    oauth2.insert("redirect_uris", Bin::Flag);
    oauth2.insert("property_mappings", Bin::Ref);
    oauth2.insert("authorization_flow", Bin::Ref);
    oauth2.insert("invalidation_flow", Bin::Ref);
    oauth2.insert("signing_key", Bin::Ref);
    oauth2.insert("sub_mode", Bin::Force(Plain::Str("hashed_user_id".into())));
    oauth2.insert("issuer_mode", Bin::Force(Plain::Str("per_provider".into())));
    oauth2.insert("include_claims_in_id_token", Bin::Force(Plain::Bool(false)));
    oauth2.insert("access_code_validity", Bin::Cap(TOKEN_MAX));
    oauth2.insert("access_token_validity", Bin::Cap(TOKEN_MAX));
    m.insert("authentik_providers_oauth2.oauth2provider", oauth2);

    let mut saml: BTreeMap<&'static str, Bin> = BTreeMap::new();
    saml.insert("name", Bin::Pass);
    saml.insert("acs_url", Bin::Flag);
    saml.insert("audience", Bin::Flag);
    saml.insert("sp_binding", Bin::Flag);
    saml.insert("authorization_flow", Bin::Ref);
    saml.insert("invalidation_flow", Bin::Ref);
    saml.insert("signing_kp", Bin::Ref);
    saml.insert("property_mappings", Bin::Ref);
    m.insert("authentik_providers_saml.samlprovider", saml);

    m
}

/// The set of permitted model names — derived from [`models`].
pub fn allowed_models() -> Vec<String> {
    models().keys().map(|s| s.to_string()).collect()
}

/// Only references resolving to these built-ins are permitted.
pub const CURATED_FLOWS: &[&str] = &[
    "default-provider-authorization-explicit-consent",
    "default-provider-invalidation-flow",
];
pub const DEFAULT_SIGNING_KEY_NAME: &str = "authentik Self-signed Certificate";
pub const CURATED_SCOPE_MAPPINGS: &[&str] = &[
    "goauthentik.io/providers/oauth2/scope-openid",
    "goauthentik.io/providers/oauth2/scope-email",
    "goauthentik.io/providers/oauth2/scope-profile",
    "goauthentik.io/providers/oauth2/scope-offline_access",
];
pub const EXCLUDED_SCOPES: &[&str] = &[
    "goauthentik.io/providers/oauth2/scope-authentik_api",
    "goauthentik.io/providers/oauth2/scope-entitlements",
];

/// A blueprint entry that deletes any model or touches crypto is irreversible.
pub fn is_destructive_entry(model: &str, state: Option<&str>) -> bool {
    state == Some("absent") || model.starts_with("authentik_crypto.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_three_onboarding_models_are_allowed() {
        let mut got = allowed_models();
        got.sort();
        assert_eq!(
            got,
            vec![
                "authentik_core.application".to_string(),
                "authentik_providers_oauth2.oauth2provider".to_string(),
                "authentik_providers_saml.samlprovider".to_string(),
            ]
        );
    }

    #[test]
    fn oauth2_forces_safe_token_trust_attrs_and_flags_redirect_uris() {
        let models = models();
        let a = &models["authentik_providers_oauth2.oauth2provider"];
        assert_eq!(
            a["include_claims_in_id_token"],
            Bin::Force(Plain::Bool(false))
        );
        assert_eq!(a["redirect_uris"], Bin::Flag);
        assert_eq!(
            a["issuer_mode"],
            Bin::Force(Plain::Str("per_provider".into()))
        );
        assert_eq!(
            a["sub_mode"],
            Bin::Force(Plain::Str("hashed_user_id".into()))
        );
    }

    #[test]
    fn relationship_fields_are_binned_as_ref() {
        let models = models();
        assert_eq!(models["authentik_core.application"]["provider"], Bin::Ref);
        let o = &models["authentik_providers_oauth2.oauth2provider"];
        for f in [
            "authorization_flow",
            "invalidation_flow",
            "signing_key",
            "property_mappings",
        ] {
            assert_eq!(o[f], Bin::Ref, "{f}");
        }
        let s = &models["authentik_providers_saml.samlprovider"];
        for f in [
            "authorization_flow",
            "invalidation_flow",
            "signing_kp",
            "property_mappings",
        ] {
            assert_eq!(s[f], Bin::Ref, "{f}");
        }
    }

    #[test]
    fn curated_scopes_exclude_authentik_api_and_use_explicit_consent() {
        assert!(CURATED_SCOPE_MAPPINGS.contains(&"goauthentik.io/providers/oauth2/scope-openid"));
        assert!(
            CURATED_SCOPE_MAPPINGS
                .contains(&"goauthentik.io/providers/oauth2/scope-offline_access")
        );
        assert!(
            !CURATED_SCOPE_MAPPINGS
                .contains(&"goauthentik.io/providers/oauth2/scope-authentik_api")
        );
        assert!(EXCLUDED_SCOPES.contains(&"goauthentik.io/providers/oauth2/scope-authentik_api"));
        assert!(CURATED_FLOWS.contains(&"default-provider-authorization-explicit-consent"));
        assert!(!CURATED_FLOWS.iter().any(|f| f.contains("implicit-consent")));
    }

    #[test]
    fn destructive_entry_covers_deletes_and_crypto() {
        assert!(is_destructive_entry(
            "authentik_sources_oauth.oauthsource",
            Some("absent")
        ));
        assert!(is_destructive_entry(
            "authentik_crypto.certificatekeypair",
            None
        ));
        assert!(!is_destructive_entry(
            "authentik_providers_oauth2.oauth2provider",
            None
        ));
    }
}
