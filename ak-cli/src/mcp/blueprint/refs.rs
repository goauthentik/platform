//! Validate references used by `ref`-binned attributes.
//!
//! `!Find` must match an approved model, field, and value; `!KeyOf` must point
//! to an entry in the same Blueprint.

use std::collections::HashSet;

use crate::mcp::blueprint::policy::{
    CURATED_FLOWS, CURATED_SCOPE_MAPPINGS, DEFAULT_SIGNING_KEY_NAME, EXCLUDED_SCOPES,
};
use crate::mcp::blueprint::tags::{RefTag, TaggedRef};
use crate::mcp::blueprint::yaml::Node;

const REF_PERMITTED_TAGS: &[&str] = &["!Find", "!KeyOf"];
const NOT_A_REF: &str = "must be a permitted reference (a curated !Find or an in-blueprint !KeyOf), not a plain literal";

/// A violation message if a tagged reference is not curated, else `None`.
/// `defined_ids` is the set of entry `id`s in this blueprint, used to validate
/// that a `!KeyOf` target is self-contained.
pub fn check_ref(reference: &TaggedRef, defined_ids: &HashSet<String>) -> Option<String> {
    let target = &reference.target;
    match reference.tag {
        RefTag::KeyOf => {
            if defined_ids.contains(target) {
                None
            } else {
                Some(format!(
                    "!KeyOf \"{target}\" does not reference an entry defined in this blueprint"
                ))
            }
        }
        RefTag::Find => {
            let lookup = (
                reference.lookup_model.as_deref(),
                reference.lookup_field.as_deref(),
                target.as_str(),
            );
            if lookup.0 == Some("authentik_providers_oauth2.scopemapping")
                && lookup.1 == Some("managed")
            {
                if EXCLUDED_SCOPES.contains(&target.as_str()) {
                    return Some(format!(
                        "external reference \"{target}\" is not permitted (excluded scope)"
                    ));
                }
                if CURATED_SCOPE_MAPPINGS.contains(&target.as_str()) {
                    return None;
                }
            }
            if lookup.0 == Some("authentik_flows.flow")
                && lookup.1 == Some("slug")
                && CURATED_FLOWS.contains(&target.as_str())
            {
                return None;
            }
            if lookup
                == (
                    Some("authentik_crypto.certificatekeypair"),
                    Some("name"),
                    DEFAULT_SIGNING_KEY_NAME,
                )
            {
                return None;
            }
            Some(format!(
                "external reference ({:?}, {:?}, \"{target}\") is not permitted (only curated built-ins may be referenced)",
                lookup.0, lookup.1
            ))
        }
    }
}

/// A `ref`-binned attribute REQUIRES a permitted reference: a single tagged
/// `!Find`/`!KeyOf`, or an untagged sequence of such (an empty list is allowed,
/// clearing the relation). A plain literal or a non-permitted tag is rejected.
/// The curated-only restriction on the target is enforced separately by the
/// tag walk + [`check_ref`].
pub fn check_ref_attr(node: Option<&Node>) -> Option<String> {
    let node = match node {
        Some(n) => n,
        None => return Some(NOT_A_REF.into()),
    };

    if let Some(tag) = node.tag() {
        // A tagged node is a single reference (note: `!Find` is structurally a
        // sequence carrying the tag, so its tag must be inspected before any
        // is-sequence branch, or it would be misread as a plain list).
        if !REF_PERMITTED_TAGS.contains(&tag) {
            return Some(NOT_A_REF.into());
        }
        return None;
    }

    if let Node::Seq { items, .. } = node {
        for item in items {
            match item.tag() {
                Some(t) if REF_PERMITTED_TAGS.contains(&t) => {}
                _ => {
                    return Some(
                        "every reference in the list must be a permitted reference (a curated !Find or an in-blueprint !KeyOf), not a plain literal".into(),
                    )
                }
            }
        }
        return None;
    }

    Some(NOT_A_REF.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::blueprint::yaml::parse_document;

    fn ids(list: &[&str]) -> HashSet<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Build a one-entry blueprint and return the value node for `attrs[key]`.
    fn attr_node(key: &str, value_expr: &str) -> Option<Node> {
        let y = format!(
            "version: 1\nentries:\n  - model: authentik_core.application\n    attrs:\n      {key}: {value_expr}"
        );
        let root = parse_document(&y).unwrap().unwrap();
        root.get("entries")
            .and_then(|e| match e {
                Node::Seq { items, .. } => items.first(),
                _ => None,
            })
            .and_then(|entry| entry.get("attrs"))
            .and_then(|attrs| attrs.get(key))
            .cloned()
    }

    #[test]
    fn keyof_must_reference_id_in_this_blueprint() {
        assert_eq!(
            check_ref(
                &TaggedRef {
                    tag: RefTag::KeyOf,
                    lookup_model: None,
                    lookup_field: None,
                    target: "p".into()
                },
                &ids(&["p"])
            ),
            None
        );
        assert!(
            check_ref(
                &TaggedRef {
                    tag: RefTag::KeyOf,
                    lookup_model: None,
                    lookup_field: None,
                    target: "p".into()
                },
                &ids(&[])
            )
            .unwrap()
            .contains("does not reference")
        );
    }

    #[test]
    fn curated_find_targets_are_permitted() {
        let none = ids(&[]);
        for t in [
            "goauthentik.io/providers/oauth2/scope-openid",
            "default-provider-invalidation-flow",
            "authentik Self-signed Certificate",
        ] {
            assert_eq!(
                check_ref(
                    &TaggedRef {
                        tag: RefTag::Find,
                        lookup_model: Some(
                            match t {
                                "authentik Self-signed Certificate" =>
                                    "authentik_crypto.certificatekeypair",
                                "default-provider-invalidation-flow" => "authentik_flows.flow",
                                _ => "authentik_providers_oauth2.scopemapping",
                            }
                            .into()
                        ),
                        lookup_field: Some(
                            match t {
                                "authentik Self-signed Certificate" => "name",
                                "default-provider-invalidation-flow" => "slug",
                                _ => "managed",
                            }
                            .into()
                        ),
                        target: t.into()
                    },
                    &none
                ),
                None
            );
        }
    }

    #[test]
    fn excluded_and_non_curated_targets_rejected() {
        let none = ids(&[]);
        assert!(
            check_ref(
                &TaggedRef {
                    tag: RefTag::Find,
                    lookup_model: Some("authentik_providers_oauth2.scopemapping".into()),
                    lookup_field: Some("managed".into()),
                    target: "goauthentik.io/providers/oauth2/scope-authentik_api".into()
                },
                &none
            )
            .unwrap()
            .contains("excluded scope")
        );
        assert!(
            check_ref(
                &TaggedRef {
                    tag: RefTag::Find,
                    lookup_model: Some("authentik_flows.flow".into()),
                    lookup_field: Some("slug".into()),
                    target: "some-other-flow".into()
                },
                &none
            )
            .unwrap()
            .contains("not permitted")
        );
    }

    #[test]
    fn single_permitted_reference_passes() {
        assert_eq!(
            check_ref_attr(attr_node("provider", "!KeyOf p").as_ref()),
            None
        );
        assert_eq!(
            check_ref_attr(
                attr_node(
                    "authorization_flow",
                    "!Find [authentik_flows.flow, [slug, x]]"
                )
                .as_ref()
            ),
            None
        );
    }

    #[test]
    fn plain_literal_or_missing_is_rejected() {
        assert!(
            check_ref_attr(attr_node("provider", "some-string").as_ref())
                .unwrap()
                .contains("permitted reference")
        );
        assert!(
            check_ref_attr(None)
                .unwrap()
                .contains("permitted reference")
        );
    }

    #[test]
    fn non_permitted_tag_is_rejected() {
        assert!(
            check_ref_attr(attr_node("provider", "!Context x").as_ref())
                .unwrap()
                .contains("permitted reference")
        );
    }

    #[test]
    fn list_of_refs_passes_list_with_literal_fails_empty_passes() {
        assert_eq!(
            check_ref_attr(
                attr_node("property_mappings", "[!Find [m, [a, b]], !KeyOf p]").as_ref()
            ),
            None
        );
        assert!(
            check_ref_attr(attr_node("property_mappings", "[!Find [m, [a, b]], plain]").as_ref())
                .unwrap()
                .contains("every reference in the list")
        );
        assert_eq!(
            check_ref_attr(attr_node("property_mappings", "[]").as_ref()),
            None
        );
    }
}
