//! Serves the skills bundled with `ak` over the MCP Skills extension (SEP-2640). rmcp handles the
//! protocol; this builds the entries for `skills/list` and `skills/get` and serves the files
//! through `resources/read`.

use std::sync::LazyLock;

use rmcp::{ErrorData as McpError, model::*};

const TTL_MS: u64 = 300_000;

/// Skill path to the contents of its `SKILL.md`.
const SOURCES: &[(&str, &str)] = &[
    // single-file skills only (just SKILL.md); list every file in `resources` once one ships
    // supporting files.
    (
        "authentik-api",
        include_str!("skills/authentik-api/SKILL.md"),
    ),
    (
        "authentik-app-access",
        include_str!("skills/authentik-app-access/SKILL.md"),
    ),
];

/// Every bundled skill with the contents of its `SKILL.md`. A skill whose frontmatter doesn't
/// parse is left out; `bundled_skills_are_valid` catches that.
static SKILLS: LazyLock<Vec<(Skill, &'static str)>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .filter_map(|(path, content)| {
            let uri = format!("skill://{path}/{SKILL_MD}");
            let files = vec![SkillResource::from_bytes(&uri, content.as_bytes())];
            let skill = Skill::new(uri, frontmatter(content)?, SkillResources::Files(files));
            Some((skill, *content))
        })
        .collect()
});

fn frontmatter(content: &str) -> Option<SkillFrontmatter> {
    let (yaml, _) = content.strip_prefix("---\n")?.split_once("\n---\n")?;
    yaml_serde::from_str(yaml).ok()
}

fn find(uri: &str) -> Option<&'static (Skill, &'static str)> {
    SKILLS.iter().find(|(skill, _)| skill.uri == uri)
}

pub fn list() -> ListSkillsResult {
    ListSkillsResult::with_all_items(SKILLS.iter().map(|(skill, _)| skill.clone()).collect())
        .with_ttl_ms(TTL_MS)
        .with_cache_scope(CacheScope::Public)
}

pub fn get(uri: &str) -> Result<GetSkillResult, McpError> {
    let (skill, _) = find(uri)
        .ok_or_else(|| McpError::invalid_params(format!("No skill is served at {uri}"), None))?;
    Ok(GetSkillResult::new(skill.clone())
        .with_ttl_ms(TTL_MS)
        .with_cache_scope(CacheScope::Public))
}

pub fn resources() -> Vec<Resource> {
    SKILLS
        .iter()
        .map(|(skill, content)| {
            Resource::new(&skill.uri, &skill.frontmatter.name)
                .with_description(&skill.frontmatter.description)
                .with_mime_type("text/markdown")
                .with_size(content.len() as u64)
        })
        .collect()
}

pub fn read(uri: &str) -> Result<ReadResourceResult, McpError> {
    let (_, content) = find(uri)
        .ok_or_else(|| McpError::invalid_params(format!("No resource is served at {uri}"), None))?;
    Ok(ReadResourceResult::new(vec![
        ResourceContents::text(*content, uri).with_mime_type("text/markdown"),
    ])
    .with_ttl_ms(TTL_MS)
    .with_cache_scope(CacheScope::Public))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_skills_are_valid() {
        assert_eq!(
            SKILLS.len(),
            SOURCES.len(),
            "a SKILL.md has bad frontmatter"
        );
        assert_eq!(validate_skills(&list().skills), Ok(()));
        for (skill, _) in SKILLS.iter() {
            assert!(read(&skill.uri).is_ok());
        }
    }

    #[test]
    fn get_unknown_skill() {
        assert!(get("skill://authentik-api/SKILL.md").is_ok());
        let err = get("skill://nope/SKILL.md").expect_err("unknown skill");
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
    }
}
