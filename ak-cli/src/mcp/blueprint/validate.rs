//! Validate a proposed Blueprint without applying it.
//!
//! Malformed or unsafe input is returned as a violation rather than causing a
//! panic.

use std::collections::HashSet;

use crate::mcp::blueprint::duration::parse_token_duration;
use crate::mcp::blueprint::policy::{Bin, allowed_models, is_destructive_entry, models};
use crate::mcp::blueprint::refs::{check_ref, check_ref_attr};
use crate::mcp::blueprint::tags::collect_tagged_refs;
use crate::mcp::blueprint::yaml::{Node, Plain, parse_document};

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FlagItem {
    pub entry_index: usize,
    pub model: String,
    pub attr: String,
    pub value: Plain,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BlueprintValidation {
    pub ok: bool,
    pub violations: Vec<String>,
    pub flags: Vec<FlagItem>,
}

const SECRET_FIELDS: &[&str] = &["client_secret", "token", "password", "key_data"];

/// True if the raw text carries a `!Env` tag (word-boundary match, like the TS
/// `/!Env\b/` pre-parse scan).
fn has_env_tag(content: &str) -> bool {
    for (idx, _) in content.match_indices("!Env") {
        let after = idx + "!Env".len();
        let boundary = content[after..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if boundary {
            return true;
        }
    }
    false
}

/// True if a second YAML document is present (`/\n---(\s|$)/`). authentik loads
/// a single document; a second would be silently ignored.
fn has_second_document(content: &str) -> bool {
    for (idx, _) in content.match_indices("\n---") {
        let after = idx + "\n---".len();
        match content[after..].chars().next() {
            None => return true,
            Some(c) if c.is_whitespace() => return true,
            _ => {}
        }
    }
    false
}

fn attrs_kind(node: Option<&Node>) -> &'static str {
    match node {
        None => "undefined",
        Some(Node::Seq { .. }) => "array",
        Some(Node::Scalar { .. }) => "scalar",
        Some(Node::Alias) => "alias",
        Some(Node::Map { .. }) => "object",
    }
}

pub fn validate_blueprint(content: &str) -> BlueprintValidation {
    let mut violations: Vec<String> = Vec::new();
    let mut flags: Vec<FlagItem> = Vec::new();

    // --- Forbidden tag: !Env (raw scan before parse) ---
    if has_env_tag(content) {
        violations.push("forbidden tag !Env (can read environment/secrets)".into());
    }

    // --- Multi-document rejection (raw scan) ---
    if has_second_document(content) {
        violations.push("multi-document YAML is not permitted; supply a single document".into());
        return BlueprintValidation {
            ok: false,
            violations,
            flags,
        };
    }

    // --- Parse with tag preservation ---
    let root = match parse_document(content) {
        Ok(Some(node)) => node,
        Ok(None) => {
            violations.push("blueprint has no `entries` list".into());
            return BlueprintValidation {
                ok: false,
                violations,
                flags,
            };
        }
        Err(e) => {
            violations.push(format!("unparseable YAML: {e}"));
            return BlueprintValidation {
                ok: false,
                violations,
                flags,
            };
        }
    };

    // --- entries must be a sequence ---
    let entries = match root.get("entries") {
        Some(Node::Seq { items, .. }) => items,
        _ => {
            violations.push("blueprint has no `entries` list".into());
            return BlueprintValidation {
                ok: false,
                violations,
                flags,
            };
        }
    };

    // --- Collect entry `id`s for self-contained !KeyOf checks ---
    let mut defined_ids: HashSet<String> = HashSet::new();
    for entry in entries {
        if let Some(Node::Scalar { value, .. }) = entry.get("id")
            && !value.is_empty()
        {
            defined_ids.insert(value.clone());
        }
    }

    let all_models = models();
    let allowed = allowed_models();

    for (i, entry) in entries.iter().enumerate() {
        let model = match entry.get("model").and_then(Node::as_str) {
            Some(m) if !m.is_empty() => m.to_lowercase(),
            _ => {
                violations.push(format!("entry {i}: missing model"));
                continue;
            }
        };

        // --- Allow-list check ---
        if !allowed.iter().any(|m| m == &model) {
            violations.push(format!(
                "entry {i}: model \"{model}\" is not in the allow-list (only curated models are permitted)"
            ));
            continue;
        }

        let state = match entry.get("state") {
            None => None,
            Some(Node::Scalar {
                value, tag: None, ..
            }) => Some(value.as_str()),
            Some(_) => {
                violations.push(format!(
                    "entry {i}: state must be a plain untagged scalar when supplied"
                ));
                continue;
            }
        };
        if is_destructive_entry(&model, state) {
            violations.push(format!(
                "entry {i}: destructive state/model is not permitted"
            ));
            continue;
        }

        // --- attrs must be a plain object ---
        let attrs_node = entry.get("attrs");
        let attrs = match attrs_node {
            Some(m @ Node::Map { .. }) => m,
            other => {
                violations.push(format!(
                    "entry {i}: attrs must be a plain object, got {}",
                    attrs_kind(other)
                ));
                continue;
            }
        };

        let rules = all_models.get(model.as_str());

        if let Node::Map { pairs, .. } = attrs {
            for (key_node, val_node) in pairs {
                let key = match key_node.as_str() {
                    Some(k) => k,
                    None => continue,
                };

                if SECRET_FIELDS.contains(&key) {
                    violations.push(format!(
                        "entry {i}: secret field \"{key}\" must be omitted (auto-generated)"
                    ));
                    continue;
                }

                let rule = rules.and_then(|r| r.get(key));
                let rule = match rule {
                    Some(r) => r,
                    None => {
                        violations.push(format!(
                            "entry {i}: attribute \"{key}\" is not permitted for model \"{model}\""
                        ));
                        continue;
                    }
                };

                match rule {
                    Bin::Pass => {}
                    Bin::Flag => flags.push(FlagItem {
                        entry_index: i,
                        model: model.clone(),
                        attr: key.to_string(),
                        value: val_node.to_plain(),
                    }),
                    Bin::Force(expected) => {
                        if let Some(tag) = val_node.tag() {
                            violations.push(format!(
                                "entry {i}: attribute \"{key}\" must be a plain untagged literal (a forced/capped attribute may not be a reference), got tag \"{tag}\""
                            ));
                            continue;
                        }
                        if &val_node.to_plain() != expected {
                            violations.push(format!(
                                "entry {i}: attribute \"{key}\" must be {expected:?} (policy-enforced), got {:?}",
                                val_node.to_plain()
                            ));
                        }
                    }
                    Bin::Cap(max_seconds) => {
                        if let Some(tag) = val_node.tag() {
                            violations.push(format!(
                                "entry {i}: attribute \"{key}\" must be a plain untagged literal (a forced/capped attribute may not be a reference), got tag \"{tag}\""
                            ));
                            continue;
                        }
                        let num = parse_token_duration(&val_node.to_plain());
                        let ok = matches!(num, Some(n) if n >= 0 && (n as u64) <= *max_seconds);
                        if !ok {
                            violations.push(format!(
                                "entry {i}: attribute \"{key}\" must be a non-negative value of at most {max_seconds}s"
                            ));
                        }
                    }
                    Bin::Ref => {
                        if let Some(msg) = check_ref_attr(Some(val_node)) {
                            violations.push(format!("entry {i}: attribute \"{key}\" {msg}"));
                        }
                    }
                }
            }
        }
    }

    // --- Whole-document tag walk: default-deny + curate-check refs ---
    let walked = collect_tagged_refs(Some(&root));
    violations.extend(walked.violations);
    for reference in &walked.refs {
        if let Some(msg) = check_ref(reference, &defined_ids) {
            violations.push(msg);
        }
    }

    BlueprintValidation {
        ok: violations.is_empty(),
        violations,
        flags,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(r: &BlueprintValidation) -> String {
        r.violations.join(" ")
    }

    #[test]
    fn serializes_to_natural_json() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: grafana\n      redirect_uris: [\"https://grafana.company/cb\"]",
        );
        let j = serde_json::to_string(&r).expect("serialize");
        assert!(j.contains("\"ok\":true"), "{j}");
        assert!(j.contains("\"attr\":\"redirect_uris\""), "{j}");
        // The flagged value serializes as a real JSON array, not a tagged enum.
        assert!(j.contains("[\"https://grafana.company/cb\"]"), "{j}");
    }

    // --- v1 ---
    #[test]
    fn accepts_application_only_blueprint() {
        let r = validate_blueprint(
            "version: 1\nmetadata: {name: ok}\nentries:\n  - model: authentik_core.application\n    identifiers: {slug: my-app}\n    attrs: {name: My App}",
        );
        assert!(r.ok, "{}", joined(&r));
        assert!(r.violations.is_empty());
    }

    #[test]
    fn rejects_superuser_group_denied_model() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.group\n    attrs: {is_superuser: true}",
        );
        assert!(!r.ok);
        assert!(!r.violations.is_empty());
    }

    #[test]
    fn rejects_token_role_and_crypto_models() {
        for m in [
            "authentik_core.token",
            "authentik_rbac.role",
            "authentik_crypto.certificatekeypair",
        ] {
            let r = validate_blueprint(&format!(
                "version: 1\nentries:\n  - model: {m}\n    attrs: {{}}"
            ));
            assert!(!r.ok, "{m} should be denied");
        }
    }

    #[test]
    fn rejects_env_tag() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    attrs: {name: !Env SECRET}",
        );
        assert!(joined(&r).contains("!Env"));
    }

    #[test]
    fn rejects_explicit_secret_fields() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: p, client_secret: hunter2}",
        );
        assert!(joined(&r).contains("client_secret"));
    }

    #[test]
    fn rejects_documents_with_no_entries() {
        assert!(!validate_blueprint("version: 1\nmetadata: {name: x}").ok);
    }

    // --- v2 policy enforcement ---
    #[test]
    fn accepts_provider_and_surfaces_redirect_uris_flag() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: grafana\n      redirect_uris: [\"https://grafana.company/oauth/callback\"]",
        );
        assert!(r.ok, "{}", joined(&r));
        assert!(r.flags.iter().any(|f| f.attr == "redirect_uris"));
    }

    #[test]
    fn rejects_non_allow_listed_model() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_policies_expression.expressionpolicy\n    attrs: {name: x, expression: \"return True\"}",
        );
        assert!(!r.ok);
        assert!(joined(&r).contains("expressionpolicy"));
    }

    #[test]
    fn rejects_attribute_not_in_allow_list() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, signing_key: anything}",
        );
        assert!(!r.ok);
        assert!(joined(&r).contains("signing_key"));
    }

    #[test]
    fn rejects_forced_attr_wrong_value() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, include_claims_in_id_token: true}",
        );
        assert!(!r.ok);
    }

    #[test]
    fn rejects_creating_property_mapping() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.scopemapping\n    attrs: {name: evil, scope_name: x, expression: \"return token\"}",
        );
        assert!(!r.ok);
    }

    #[test]
    fn rejects_find_to_non_curated_scope() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-authentik_api]]",
        );
        assert!(!r.ok);
        let j = joined(&r);
        assert!(j.contains("authentik_api") || j.contains("not permitted"));
    }

    #[test]
    fn permits_find_to_curated_scope() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-openid]]",
        );
        assert!(r.ok, "{}", joined(&r));
    }

    #[test]
    fn rejects_policy_binding() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    attrs: {name: x, slug: x, policies: [\"some-policy\"]}",
        );
        assert!(!r.ok);
    }

    #[test]
    fn rejects_multi_document() {
        let r = validate_blueprint(
            "version: 1\nentries: []\n---\nversion: 1\nentries:\n  - model: authentik_core.group\n    attrs: {is_superuser: true}",
        );
        assert!(!r.ok);
        assert!(
            joined(&r).to_lowercase().contains("multi-document") || joined(&r).contains("single")
        );
    }

    #[test]
    fn rejects_non_object_attrs() {
        assert!(
            !validate_blueprint(
                "version: 1\nentries:\n  - model: authentik_core.application\n    attrs: \"oops\""
            )
            .ok
        );
    }

    #[test]
    fn rejects_destructive_or_tagged_state() {
        let absent = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    state: absent\n    attrs: {name: x}",
        );
        assert!(!absent.ok, "{}", joined(&absent));
        let tagged = validate_blueprint(
            "version: 1\nentries:\n  - id: present\n    model: authentik_core.application\n    attrs: {name: x}\n  - model: authentik_core.application\n    state: !KeyOf present\n    attrs: {name: y}",
        );
        assert!(!tagged.ok, "{}", joined(&tagged));
    }

    #[test]
    fn rejects_find_with_unapproved_model_or_field() {
        for find in [
            "!Find [authentik_core.user, [managed, goauthentik.io/providers/oauth2/scope-openid]]",
            "!Find [authentik_providers_oauth2.scopemapping, [name, goauthentik.io/providers/oauth2/scope-openid]]",
        ] {
            let r = validate_blueprint(&format!(
                "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings: [{find}]"
            ));
            assert!(!r.ok, "{find}: {}", joined(&r));
        }
    }

    #[test]
    fn rejects_duplicate_yaml_mapping_keys() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    model: authentik_core.group\n    attrs: {name: x}",
        );
        assert!(!r.ok);
        assert!(joined(&r).contains("duplicate"));
    }

    #[test]
    fn normalizes_model_case() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: AUTHENTIK_CORE.Application\n    attrs: {name: x, slug: x}",
        );
        assert!(r.ok, "{}", joined(&r));
    }

    // --- adversarial bypass (default-deny on tags) ---
    #[test]
    fn c1a_rejects_empty_find_sequence() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find []",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c1b_rejects_find_with_scalar_condition() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, scope-openid]",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c1c_rejects_find_with_nested_seq_value() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, [nested, seq]]]",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c2_rejects_multi_condition_find() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-openid], [pk, 999]]",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c3a_rejects_findobject() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !FindObject [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-openid]]",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c3b_rejects_context() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    attrs:\n      name: x\n      slug: x\n      meta_description: !Context some_secret",
        );
        assert!(!r.ok);
    }

    #[test]
    fn c3c_rejects_other_resolving_tags() {
        for t in [
            "!Format [foo]",
            "!Condition [AND, true]",
            "!If [true, a, b]",
            "!File secret.txt",
            "!Enumerate [[], SEQ, x]",
            "!Value x",
            "!Index 0",
            "!AtIndex 0",
            "!ParseJSON '{}'",
        ] {
            let r = validate_blueprint(&format!(
                "version: 1\nentries:\n  - model: authentik_core.application\n    attrs:\n      name: x\n      slug: x\n      meta_description: {t}"
            ));
            assert!(!r.ok, "tag {t} should be rejected");
        }
    }

    #[test]
    fn c4_scalar_find_violates_never_panics() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_saml.samlprovider\n    attrs:\n      name: x\n      acs_url: !Find evil",
        );
        assert!(!r.ok);
    }

    #[test]
    fn keyof_to_undefined_id_rejected() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_core.application\n    attrs:\n      name: x\n      slug: x\n      meta_launch_url: !KeyOf some-external-id",
        );
        assert!(!r.ok);
    }

    #[test]
    fn keyof_to_in_blueprint_id_permitted() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - id: my-provider\n    model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: prov\n  - model: authentik_core.application\n    attrs:\n      name: x\n      slug: x\n      meta_launch_url: !KeyOf my-provider",
        );
        assert!(r.ok, "{}", joined(&r));
    }

    #[test]
    fn i1_rejects_negative_cap() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      access_token_validity: -5",
        );
        assert!(!r.ok);
    }

    #[test]
    fn i2_rejects_unknown_duration_units() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      access_token_validity: \"fortnights=10;hours=1\"",
        );
        assert!(!r.ok);
    }

    // --- FIX A: tags inside a !Find's children ---
    #[test]
    fn fix_a_rejects_tag_in_find_model_position() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [!Context m, [managed, goauthentik.io/providers/oauth2/scope-openid]]",
        );
        assert!(!r.ok);
        let j = joined(&r).to_lowercase();
        assert!(j.contains("model") || j.contains("untagged") || j.contains("context"));
    }

    #[test]
    fn fix_a_rejects_tag_in_find_field_position() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [!File /etc/passwd, goauthentik.io/providers/oauth2/scope-openid]]",
        );
        assert!(!r.ok);
        let j = joined(&r).to_lowercase();
        assert!(j.contains("field") || j.contains("untagged") || j.contains("file"));
    }

    #[test]
    fn fix_a_rejects_tag_in_find_value_position() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, !Context x]]",
        );
        assert!(!r.ok);
        let j = joined(&r).to_lowercase();
        assert!(j.contains("value") || j.contains("untagged") || j.contains("context"));
    }

    // --- FIX B: ref bin ---
    #[test]
    fn fix_b_application_provider_keyof_permitted() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - id: my-provider\n    model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: prov\n  - model: authentik_core.application\n    attrs:\n      name: x\n      slug: x\n      provider: !KeyOf my-provider",
        );
        assert!(r.ok, "{}", joined(&r));
        assert!(r.violations.is_empty());
    }

    #[test]
    fn fix_b_oauth2_authorization_flow_curated_find_permitted() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      authorization_flow: !Find [authentik_flows.flow, [slug, default-provider-authorization-explicit-consent]]",
        );
        assert!(r.ok, "{}", joined(&r));
    }

    #[test]
    fn fix_b_oauth2_signing_key_default_key_permitted() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      signing_key: !Find [authentik_crypto.certificatekeypair, [name, authentik Self-signed Certificate]]",
        );
        assert!(r.ok, "{}", joined(&r));
    }

    #[test]
    fn fix_b_oauth2_authorization_flow_non_curated_rejected() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      authorization_flow: !Find [authentik_flows.flow, [slug, some-other-flow]]",
        );
        assert!(!r.ok);
        let j = joined(&r).to_lowercase();
        assert!(j.contains("not permitted") || j.contains("curated"));
    }

    #[test]
    fn fix_b_oauth2_signing_key_plain_string_rejected() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: x\n      signing_key: some-key-name",
        );
        assert!(!r.ok);
        assert!(joined(&r).contains("signing_key"));
    }

    #[test]
    fn fix_b_end_to_end_onboarding_validates() {
        let r = validate_blueprint(
            "version: 1\nmetadata: {name: onboard grafana}\nentries:\n  - id: grafana-provider\n    model: authentik_providers_oauth2.oauth2provider\n    attrs:\n      name: grafana\n      client_type: confidential\n      redirect_uris: [\"https://grafana.company/oauth/callback\"]\n      authorization_flow: !Find [authentik_flows.flow, [slug, default-provider-authorization-explicit-consent]]\n      invalidation_flow: !Find [authentik_flows.flow, [slug, default-provider-invalidation-flow]]\n      signing_key: !Find [authentik_crypto.certificatekeypair, [name, authentik Self-signed Certificate]]\n      sub_mode: hashed_user_id\n      issuer_mode: per_provider\n      include_claims_in_id_token: false\n      property_mappings:\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-openid]]\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-email]]\n        - !Find [authentik_providers_oauth2.scopemapping, [managed, goauthentik.io/providers/oauth2/scope-profile]]\n  - model: authentik_core.application\n    attrs:\n      name: Grafana\n      slug: grafana\n      meta_launch_url: https://grafana.company\n      provider: !KeyOf grafana-provider",
        );
        assert!(r.ok, "{}", joined(&r));
        assert!(r.violations.is_empty());
        assert!(r.flags.iter().any(|f| f.attr == "redirect_uris"));
    }

    // --- FIX C: forced/capped must be plain untagged literals ---
    #[test]
    fn fix_c_rejects_keyof_decoy_for_sub_mode() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - id: hashed_user_id\n    model: authentik_core.application\n    attrs: {name: a, slug: a}\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, sub_mode: !KeyOf hashed_user_id}",
        );
        assert!(!r.ok);
        let j = joined(&r).to_lowercase();
        assert!(j.contains("sub_mode") || j.contains("untagged") || j.contains("literal"));
    }

    #[test]
    fn fix_c_rejects_keyof_decoy_for_issuer_mode() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - id: per_provider\n    model: authentik_core.application\n    attrs: {name: a, slug: a}\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, issuer_mode: !KeyOf per_provider}",
        );
        assert!(!r.ok);
    }

    #[test]
    fn fix_c_rejects_tag_on_capped_attr() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - id: \"60\"\n    model: authentik_core.application\n    attrs: {name: a, slug: a}\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, access_token_validity: !KeyOf \"60\"}",
        );
        assert!(!r.ok);
    }

    #[test]
    fn fix_c_plain_sub_mode_passes() {
        let r = validate_blueprint(
            "version: 1\nentries:\n  - model: authentik_providers_oauth2.oauth2provider\n    attrs: {name: x, sub_mode: hashed_user_id}",
        );
        assert!(r.ok, "{}", joined(&r));
    }
}
