//! Generic access to the full authentik API through the clap tree `ak api` is generated from.
//!
//! Exposing every API operation as its own MCP tool would put thousands of schemas into the
//! model's context, so instead the model searches the command tree, reads the help of the
//! command it picked, and calls it with CLI-style arguments.

use std::sync::LazyLock;

use ak_api_cli::ApiArgs;
use clap::{CommandFactory, Parser};
use rmcp::{ErrorData as McpError, model::*, schemars};
use serde::Deserialize;

use crate::mcp::AuthentikMcp;

const MAX_SEARCH_RESULTS: usize = 50;

/// Every leaf command as `(path, summary)`, e.g. `("core applications list", "...")`.
static COMMANDS: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    fn walk(cmd: &clap::Command, prefix: &str, out: &mut Vec<(String, String)>) {
        for sub in cmd.get_subcommands().filter(|s| s.get_name() != "help") {
            let path = format!("{prefix}{}", sub.get_name());
            if sub.has_subcommands() {
                walk(sub, &format!("{path} "), out);
            } else {
                let about = sub.get_about().map(ToString::to_string).unwrap_or_default();
                out.push((path, about));
            }
        }
    }
    let mut out = Vec::new();
    walk(&ApiArgs::command(), "", &mut out);
    out
});

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiSearchArgs {
    /// Space-separated terms that must all appear in the command path or summary, e.g.
    /// "applications list" or "flow".
    pub query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiDescribeArgs {
    /// Command path as returned by api_search, e.g. "core applications list"
    pub command: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiCallArgs {
    /// Command path followed by its arguments, one element each, e.g.
    /// ["core", "applications", "list", "--search", "foo", "--page-size", "20"]
    pub args: Vec<String>,
    /// Identifier of an agent created via `create_agent` to make the call as, instead of the
    /// current user
    #[serde(default)]
    pub agent_identifier: Option<String>,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

fn search(query: &str) -> String {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let hits: Vec<String> = COMMANDS
        .iter()
        .filter(|(path, about)| {
            let hay = format!("{path} {about}").to_lowercase();
            terms.iter().all(|t| hay.contains(t.as_str()))
        })
        .map(|(path, about)| format!("{path} — {about}"))
        .collect();
    if hits.is_empty() {
        return format!("No API commands match \"{query}\".");
    }
    let mut out = hits
        .iter()
        .take(MAX_SEARCH_RESULTS)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    if hits.len() > MAX_SEARCH_RESULTS {
        out.push_str(&format!(
            "\n… {} more, refine the query.",
            hits.len() - MAX_SEARCH_RESULTS
        ));
    }
    out
}

fn describe(command: &str) -> Option<String> {
    let mut root = ApiArgs::command();
    root.build();
    let mut cmd = &mut root;
    for part in command.split_whitespace() {
        cmd = cmd.find_subcommand_mut(part)?;
    }
    Some(cmd.render_long_help().to_string())
}

impl AuthentikMcp {
    pub(crate) async fn _api_search(
        &self,
        args: ApiSearchArgs,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![ContentBlock::text(search(
            &args.query,
        ))]))
    }

    pub(crate) async fn _api_describe(
        &self,
        args: ApiDescribeArgs,
    ) -> Result<CallToolResult, McpError> {
        Ok(match describe(&args.command) {
            Some(help) => CallToolResult::success(vec![ContentBlock::text(help)]),
            None => CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown command \"{}\", use api_search to find one.",
                args.command
            ))]),
        })
    }

    pub(crate) async fn _api_call(&self, args: ApiCallArgs) -> Result<CallToolResult, McpError> {
        // Errors the model can fix (bad arguments, API rejections) are returned as tool errors
        // rather than protocol errors so it sees them and can retry.
        let mut argv =
            match ApiArgs::try_parse_from(std::iter::once("api".to_owned()).chain(args.args)) {
                Ok(argv) => argv,
                Err(e) => {
                    return Ok(CallToolResult::error(vec![ContentBlock::text(
                        e.to_string(),
                    )]));
                }
            };
        // Set after parsing so an agent identifier always wins over an --api-token in args.
        if let Some(identifier) = &args.agent_identifier {
            let Some(token) = self.agent_token(identifier).await else {
                return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                    "Unknown agent \"{identifier}\", create one with create_agent first."
                ))]));
            };
            argv.api_token = Some(token);
        }
        let config = self.configuration(args.profile).await?;
        let out = argv
            .execute(&config)
            .await
            .and_then(|out| Ok(serde_json::to_string(&out)?));
        Ok(match out {
            Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(format!("{e:#}"))]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_finds_and_caps() {
        let out = search("core applications list");
        assert!(out.contains("core applications list — "), "{out}");
        assert!(search("zzz-nothing").starts_with("No API commands"));
        assert!(search("").contains("more, refine the query"));
    }

    #[test]
    fn describe_renders_leaf_help() {
        let help = describe("core applications list").expect("known command");
        assert!(help.contains("Usage: api core applications list"), "{help}");
        assert!(describe("core nope").is_none());
    }

    #[test]
    fn call_args_parse() {
        assert!(ApiArgs::try_parse_from(["api", "core", "applications", "list"]).is_ok());
        assert!(ApiArgs::try_parse_from(["api", "core", "nope"]).is_err());
        let argv =
            ApiArgs::try_parse_from(["api", "--api-token", "t", "core", "applications", "list"])
                .expect("--api-token parses");
        assert_eq!(argv.api_token.as_deref(), Some("t"));
    }
}
