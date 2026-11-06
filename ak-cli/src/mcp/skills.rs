//! Serves the skills bundled with `ak` over the MCP Skills extension
//! (`io.modelcontextprotocol/skills`, SEP-2640): `skills/list` and `skills/get` describe them, and
//! their files are read with the standard `resources/read`.

use rmcp::{ErrorData as McpError, model::*};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub const EXTENSION: &str = "io.modelcontextprotocol/skills";
const TTL_MS: u64 = 300_000;

// ponytail: single-file skills only (just SKILL.md); list every file in `resources` once one ships
// supporting files.
/// Skill path to the contents of its `SKILL.md`.
const SKILLS: &[(&str, &str)] = &[
    (
        "authentik-api",
        include_str!("skills/authentik-api/SKILL.md"),
    ),
    (
        "authentik-app-access",
        include_str!("skills/authentik-app-access/SKILL.md"),
    ),
];

fn uri(path: &str) -> String {
    format!("skill://{path}/SKILL.md")
}

fn find(uri_: &str) -> Option<&'static str> {
    SKILLS
        .iter()
        .find(|(path, _)| uri(path) == uri_)
        .map(|(_, content)| *content)
}

/// The `SKILL.md` frontmatter as a JSON object, empty if it is missing or not a YAML mapping.
fn frontmatter(content: &str) -> Map<String, Value> {
    content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .and_then(|(yaml, _)| yaml_serde::from_str(yaml).ok())
        .unwrap_or_default()
}

fn entry(path: &str, content: &str) -> Value {
    let uri = uri(path);
    json!({
        "uri": uri,
        "frontmatter": frontmatter(content),
        "resources": [{
            "uri": uri,
            "digest": format!("sha256:{}", hex::encode(Sha256::digest(content.as_bytes()))),
            "size": content.len(),
        }],
    })
}

pub fn capability() -> ExtensionCapabilities {
    ExtensionCapabilities::from([(EXTENSION.to_owned(), JsonObject::new())])
}

pub fn resources() -> Vec<Resource> {
    SKILLS
        .iter()
        .map(|(path, content)| {
            let fm = frontmatter(content);
            let field = |k: &str| {
                fm.get(k)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned()
            };
            Resource::new(uri(path), field("name"))
                .with_description(field("description"))
                .with_mime_type("text/markdown")
                .with_size(content.len() as u64)
        })
        .collect()
}

pub fn read(uri: &str) -> Result<ReadResourceResult, McpError> {
    let content = find(uri)
        .ok_or_else(|| McpError::invalid_params(format!("No resource is served at {uri}"), None))?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(content, uri).with_mime_type("text/markdown"),
    ])
    .with_ttl_ms(TTL_MS)
    .with_cache_scope(CacheScope::Public))
}

/// Handles the extension's methods, `None` for any other method.
pub fn handle(request: &CustomRequest) -> Option<Result<CustomResult, McpError>> {
    let mut result = match request.method.as_str() {
        "skills/list" => json!({
            "skills": SKILLS.iter().map(|(p, c)| entry(p, c)).collect::<Vec<_>>(),
        }),
        "skills/get" => {
            let uri = request
                .params
                .as_ref()
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some((path, content)) = SKILLS.iter().find(|(p, _)| self::uri(p) == uri) else {
                return Some(Err(McpError::invalid_params(
                    format!("No skill is served at {uri}"),
                    None,
                )));
            };
            json!({ "skill": entry(path, content) })
        }
        _ => return None,
    };
    if let Value::Object(map) = &mut result {
        map.insert("resultType".into(), "complete".into());
        map.insert("ttlMs".into(), TTL_MS.into());
        map.insert("cacheScope".into(), "public".into());
    }
    Some(Ok(CustomResult::new(result)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_skills_are_valid() {
        for (path, content) in SKILLS {
            let fm = frontmatter(content);
            // The spec requires the last path segment to equal the frontmatter name.
            assert_eq!(
                fm.get("name").and_then(Value::as_str),
                path.rsplit('/').next()
            );
            assert!(
                fm.get("description")
                    .and_then(Value::as_str)
                    .is_some_and(|d| !d.is_empty())
            );
            assert!(read(&uri(path)).is_ok());
        }
    }

    #[test]
    fn get_and_list() {
        let get = |uri: &str| {
            handle(&CustomRequest::new(
                "skills/get",
                Some(json!({ "uri": uri })),
            ))
            .expect("handled")
        };
        let skill = get("skill://authentik-api/SKILL.md")
            .expect("known skill")
            .0;
        assert_eq!(skill["resultType"], "complete");
        assert!(
            skill["skill"]["resources"][0]["digest"]
                .as_str()
                .is_some_and(|d| d.starts_with("sha256:") && d.len() == 71)
        );
        let err = get("skill://nope/SKILL.md").expect_err("unknown skill");
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);

        let list = handle(&CustomRequest::new("skills/list", None))
            .expect("handled")
            .expect("ok");
        assert_eq!(
            list.0["skills"].as_array().map(Vec::len),
            Some(SKILLS.len())
        );
        assert!(handle(&CustomRequest::new("other/method", None)).is_none());
    }
}
