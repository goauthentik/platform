//! Default-deny YAML-tag walker. Walks a parsed blueprint AST, rejecting every
//! tag except the curated `!Find` / `!KeyOf` references and extracting those
//! references' targets so the validator can curate-check them.
//!
//! We enumerate only permitted tags, never dangerous ones — this structurally
//! closes whole classes of bypass (`!FindObject`, `!Context`, `!Format`,
//! `!Env`, …). It never panics: malformed shapes become violations.

use crate::mcp::blueprint::yaml::{Node, is_string_scalar};

const PERMITTED_TAGS: &[&str] = &["!Find", "!KeyOf"];

/// A tagged reference whose target must be curate-checked.
#[derive(Debug, Clone, PartialEq)]
pub struct TaggedRef {
    pub tag: RefTag,
    /// `!Find` lookup model; absent for `!KeyOf`.
    pub lookup_model: Option<String>,
    /// `!Find` lookup field; absent for `!KeyOf`.
    pub lookup_field: Option<String>,
    /// The resolved target string (scope slug, flow slug, key name, or KeyOf id).
    pub target: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RefTag {
    Find,
    KeyOf,
}

/// Result of walking the AST: collected refs plus any structural violations.
#[derive(Debug, Default)]
pub struct WalkResult {
    pub refs: Vec<TaggedRef>,
    pub violations: Vec<String>,
}

/// Walk a document AST, enforcing default-deny on tags and extracting the
/// curate-checkable targets from permitted (`!Find` / `!KeyOf`) nodes.
pub fn collect_tagged_refs(node: Option<&Node>) -> WalkResult {
    let mut out = WalkResult::default();
    if let Some(n) = node {
        walk(n, &mut out);
    }
    out
}

fn walk(n: &Node, out: &mut WalkResult) {
    if let Some(tag) = n.tag() {
        if !PERMITTED_TAGS.contains(&tag) {
            out.violations.push(format!(
                "tag \"{tag}\" is not permitted (only !Find and !KeyOf are allowed)"
            ));
            // Do not recurse into a rejected node — its shape is untrusted.
            return;
        }
        if tag == "!Find" {
            extract_find(n, out);
            return;
        }
        if tag == "!KeyOf" {
            match n {
                Node::Scalar { value, .. } => out.refs.push(TaggedRef {
                    tag: RefTag::KeyOf,
                    lookup_model: None,
                    lookup_field: None,
                    target: value.clone(),
                }),
                _ => out.violations.push(
                    "!KeyOf must be a scalar id referencing an entry in this blueprint".into(),
                ),
            }
            return;
        }
    }
    recurse(n, out);
}

fn recurse(n: &Node, out: &mut WalkResult) {
    match n {
        Node::Map { pairs, .. } => {
            for (k, v) in pairs {
                walk(k, out);
                walk(v, out);
            }
        }
        Node::Seq { items, .. } => {
            for item in items {
                walk(item, out);
            }
        }
        _ => {}
    }
}

/// Validate and extract a `!Find` node. The only understood shape mirrors
/// authentik's `Find.__init__`:
///   `!Find [ <model>, [field, scalar], [field, scalar], ... ]`
/// A nested tag anywhere in the model/field/value positions is attacker-
/// controlled lookup/IO and is rejected; the understood `!Find` contains only
/// plain, untagged scalars. Every condition value is extracted (all are
/// AND-combined server-side), so each is curate-checked.
fn extract_find(n: &Node, out: &mut WalkResult) {
    let items = match n {
        Node::Seq { items, .. } => items,
        _ => {
            out.violations
                .push("!Find must be a sequence [model, [field, value], ...]".into());
            return;
        }
    };

    if items.len() < 2 {
        out.violations
            .push("!Find must have a model and at least one [field, value] condition".into());
        return;
    }

    let model_node = &items[0];
    if let Some(tag) = model_node.tag() {
        out.violations.push(format!(
            "!Find model name must be a plain untagged scalar, got tag \"{tag}\""
        ));
        return;
    }
    if !is_string_scalar(model_node) {
        out.violations
            .push("!Find model name must be a scalar string".into());
        return;
    }
    let Node::Scalar { value: model, .. } = model_node else {
        unreachable!("is_string_scalar only accepts scalar nodes");
    };

    for cond in &items[1..] {
        let cond_items = match cond {
            Node::Seq { items, .. } => items,
            _ => {
                out.violations
                    .push("!Find condition must be a [field, value] sequence".into());
                return;
            }
        };
        if cond_items.len() != 2 {
            out.violations
                .push("!Find condition must be exactly [field, value]".into());
            return;
        }
        let field_node = &cond_items[0];
        let val_node = &cond_items[1];

        if let Some(tag) = field_node.tag() {
            out.violations.push(format!(
                "!Find condition field must be a plain untagged scalar, got tag \"{tag}\""
            ));
            return;
        }
        if let Some(tag) = val_node.tag() {
            out.violations.push(format!(
                "!Find condition value must be a plain untagged scalar, got tag \"{tag}\""
            ));
            return;
        }
        if !is_string_scalar(field_node) {
            out.violations
                .push("!Find condition field must be a scalar string".into());
            return;
        }
        if !is_string_scalar(val_node) {
            out.violations
                .push("!Find condition value must be a scalar string".into());
            return;
        }
        if let (Node::Scalar { value: field, .. }, Node::Scalar { value, .. }) =
            (field_node, val_node)
        {
            out.refs.push(TaggedRef {
                tag: RefTag::Find,
                lookup_model: Some(model.clone()),
                lookup_field: Some(field.clone()),
                target: value.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::blueprint::yaml::parse_document;

    fn walk_yaml(y: &str) -> WalkResult {
        collect_tagged_refs(parse_document(y).unwrap().as_ref())
    }

    #[test]
    fn extracts_condition_value_from_curated_find() {
        let r = walk_yaml(
            "x: !Find [authentik_flows.flow, [slug, default-provider-invalidation-flow]]",
        );
        assert!(r.violations.is_empty());
        assert_eq!(r.refs.len(), 1);
        assert_eq!(r.refs[0].tag, RefTag::Find);
        assert_eq!(r.refs[0].target, "default-provider-invalidation-flow");
    }

    #[test]
    fn extracts_every_condition_of_multi_condition_find() {
        let r = walk_yaml("x: !Find [m, [a, 'v1'], [b, 'v2']]");
        assert!(r.violations.is_empty());
        let targets: Vec<&str> = r.refs.iter().map(|r| r.target.as_str()).collect();
        assert_eq!(targets, vec!["v1", "v2"]);
    }

    #[test]
    fn extracts_keyof_scalar_id() {
        let r = walk_yaml("x: !KeyOf my-provider");
        assert!(r.violations.is_empty());
        assert_eq!(
            r.refs,
            vec![TaggedRef {
                tag: RefTag::KeyOf,
                lookup_model: None,
                lookup_field: None,
                target: "my-provider".into()
            }]
        );
    }

    #[test]
    fn default_denies_any_tag_outside_find_keyof() {
        for tag in ["!Context", "!Format", "!Env", "!File", "!FindObject"] {
            let r = walk_yaml(&format!("x: {tag} whatever"));
            assert_eq!(r.refs.len(), 0, "{tag} must extract no ref");
            assert!(
                r.violations.iter().any(|v| v.contains("not permitted")),
                "{tag} should be rejected"
            );
        }
    }

    #[test]
    fn malformed_find_shapes_violate_and_never_panic() {
        assert!(!walk_yaml("x: !Find evil").violations.is_empty());
        assert!(!walk_yaml("x: !Find []").violations.is_empty());
        assert!(!walk_yaml("x: !Find [only-model]").violations.is_empty());
        assert!(
            !walk_yaml("x: !Find [m, scope-openid]")
                .violations
                .is_empty()
        );
    }

    #[test]
    fn rejects_tag_nested_in_find_model_field_or_value() {
        assert!(
            walk_yaml("x: !Find [!Context m, [slug, v]]")
                .violations
                .iter()
                .any(|v| v.to_lowercase().contains("model"))
        );
        assert!(
            walk_yaml("x: !Find [m, [!File f, v]]")
                .violations
                .iter()
                .any(|v| v.to_lowercase().contains("field"))
        );
        assert!(
            walk_yaml("x: !Find [m, [slug, !Context v]]")
                .violations
                .iter()
                .any(|v| v.to_lowercase().contains("value"))
        );
    }

    #[test]
    fn numeric_condition_value_is_rejected() {
        let r = walk_yaml("x: !Find [m, [pk, 999]]");
        assert!(r.violations.iter().any(|v| v.contains("scalar string")));
    }

    #[test]
    fn empty_and_untagged_input_never_panics() {
        assert!(collect_tagged_refs(None).refs.is_empty());
        assert!(walk_yaml("x: plain").refs.is_empty());
        assert!(walk_yaml("x: plain").violations.is_empty());
        let r = walk_yaml("x: [a, b, c]");
        assert!(r.refs.is_empty() && r.violations.is_empty());
    }
}
