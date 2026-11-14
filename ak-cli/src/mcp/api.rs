//! Generic access to the full authentik API through the clap tree `ak api` is generated from.
//!
//! Exposing every API operation as its own MCP tool would put thousands of schemas into the
//! model's context, so instead the model searches the command tree, reads the help of the
//! command it picked, and calls it with CLI-style arguments.

use std::{collections::BTreeMap, sync::LazyLock};

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
    /// Command path as returned by api_search, e.g. "core applications list"
    pub command: String,
    /// Arguments as shown by api_describe, keyed by name without the leading `--`, e.g.
    /// {"search": "foo", "page-size": 20}. Positional arguments use their name from the usage
    /// line in lower case, e.g. {"slug": "my-app"}. Lists repeat the argument once per element.
    #[serde(default)]
    pub args: BTreeMap<String, serde_json::Value>,
    /// Identifier of the agent to make the call as, returned by `create_agent`
    pub agent_identifier: String,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApiCallDangerousArgs {
    /// Command path as returned by api_search, e.g. "core applications list"
    pub command: String,
    /// Arguments as shown by api_describe, keyed by name without the leading `--`, e.g.
    /// {"search": "foo", "page-size": 20}. Positional arguments use their name from the usage
    /// line in lower case, e.g. {"slug": "my-app"}. Lists repeat the argument once per element.
    #[serde(default)]
    pub args: BTreeMap<String, serde_json::Value>,
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

/// Turn a command path and an argument map into the argv clap expects.
fn to_argv(
    command: &str,
    args: &BTreeMap<String, serde_json::Value>,
) -> Result<Vec<String>, String> {
    let unknown_command = || format!("Unknown command \"{command}\", use api_search to find one.");
    let root = ApiArgs::command();
    let mut cmd = &root;
    let mut argv = vec!["api".to_owned()];
    for part in command.split_whitespace() {
        cmd = cmd.find_subcommand(part).ok_or_else(unknown_command)?;
        argv.push(part.to_owned());
    }
    if cmd.has_subcommands() {
        return Err(unknown_command());
    }

    let mut positionals = BTreeMap::new();
    for (key, value) in args {
        let key = key.trim_start_matches('-');
        let arg = cmd
            .get_arguments()
            .find(|a| a.get_long() == Some(key) || a.get_id().as_str() == key.replace('-', "_"))
            .ok_or_else(|| {
                format!("Unknown argument \"{key}\" for \"{command}\", see api_describe.")
            })?;
        let values: Vec<String> = match value {
            serde_json::Value::Null => continue,
            serde_json::Value::Array(items) => items.iter().map(arg_value).collect(),
            v => vec![arg_value(v)],
        };
        match (arg.get_long(), arg.get_index()) {
            (Some(long), _) => argv.extend(values.iter().map(|v| format!("--{long}={v}"))),
            (None, Some(index)) => {
                positionals.insert(index, values);
            }
            (None, None) => return Err(format!("Argument \"{key}\" cannot be set.")),
        }
    }
    // After `--` so values starting with `-` are not mistaken for flags.
    if !positionals.is_empty() {
        argv.push("--".to_owned());
        argv.extend(positionals.into_values().flatten());
    }
    Ok(argv)
}

/// Strings are passed through as-is, everything else (numbers, bools, JSON bodies) as JSON.
fn arg_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        v => v.to_string(),
    }
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
        let Some(token) = self.agent_token(&args.agent_identifier).await else {
            return Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown agent \"{}\", create one with create_agent first.",
                args.agent_identifier
            ))]));
        };
        self.run_api(&args.command, &args.args, Some(token), args.profile)
            .await
    }

    pub(crate) async fn _api_call_dangerous(
        &self,
        args: ApiCallDangerousArgs,
    ) -> Result<CallToolResult, McpError> {
        self.run_api(&args.command, &args.args, None, args.profile)
            .await
    }

    /// Run an API command, as the agent owning `token` if given, otherwise as the current user.
    async fn run_api(
        &self,
        command: &str,
        args: &BTreeMap<String, serde_json::Value>,
        token: Option<String>,
        profile: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        // Errors the model can fix (bad arguments, API rejections) are returned as tool errors
        // rather than protocol errors so it sees them and can retry.
        let parsed = to_argv(command, args)
            .and_then(|argv| ApiArgs::try_parse_from(argv).map_err(|e| e.to_string()));
        let mut argv = match parsed {
            Ok(argv) => argv,
            Err(e) => return Ok(CallToolResult::error(vec![ContentBlock::text(e)])),
        };
        // Set after parsing so the agent's token always wins over an --api-token in args.
        if token.is_some() {
            argv.api_token = token;
        }
        let config = self.configuration(profile).await?;
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

    #[test]
    fn map_args_to_argv() {
        let args = |v: serde_json::Value| {
            serde_json::from_value::<BTreeMap<String, serde_json::Value>>(v).expect("map")
        };
        let argv = to_argv(
            "core users list",
            &args(serde_json::json!({
                "page-size": 20,
                "groups_by_name": ["a", "b"],
                "--search": "x",
                "is-active": true,
                "path": null,
            })),
        )
        .expect("valid args");
        assert_eq!(
            argv,
            [
                "api",
                "core",
                "users",
                "list",
                "--search=x",
                "--groups-by-name=a",
                "--groups-by-name=b",
                "--is-active=true",
                "--page-size=20",
            ]
        );
        assert!(ApiArgs::try_parse_from(argv).is_ok());

        let argv = to_argv(
            "core applications retrieve",
            &args(serde_json::json!({"slug": "-odd"})),
        )
        .expect("positional");
        assert_eq!(argv[4..], ["--", "-odd"]);
        assert!(ApiArgs::try_parse_from(argv).is_ok());

        assert!(to_argv("core users list", &args(serde_json::json!({"nope": 1}))).is_err());
        assert!(to_argv("core users", &BTreeMap::new()).is_err());
        assert!(to_argv("core nope", &BTreeMap::new()).is_err());
    }

    #[test]
    fn dangerous_tool_requires_flag() {
        use crate::{App, CliArgs};
        let mcp = |allow| AuthentikMcp::new(App::new(CliArgs::parse_from(["ak", "mcp"])), allow);
        assert!(!mcp(false).tool_router.has_route("api_call_dangerous"));
        assert!(mcp(false).tool_router.has_route("api_call"));
        assert!(mcp(true).tool_router.has_route("api_call_dangerous"));
    }
}
