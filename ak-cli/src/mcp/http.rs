//! HTTP tools scoped to an agent identity.
//!
//! Requests are only sent to hosts adjacent to an application the agent has
//! exchanged a token for (see [`crate::mcp::origin`]), and the exchanged token is
//! attached automatically — the model never handles the credential itself.

use std::{collections::HashMap, time::Duration};

use ak_meta::user_agent;
use chrono::Utc;
use reqwest::{
    Method,
    header::{CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue},
};
use rmcp::{ErrorData as McpError, model::*, schemars};
use serde::Deserialize;
use url::Url;

use crate::mcp::AuthentikMcp;

/// Response bodies beyond this size are truncated before being handed back.
const MAX_BODY_BYTES: usize = 256 * 1024;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Headers the tool controls itself; callers may not override them.
const RESERVED_HEADERS: [&str; 4] = ["authorization", "host", "proxy-authorization", "cookie"];

/// HTTP methods that do not modify the target.
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum SafeMethod {
    #[default]
    Get,
    Head,
}

/// HTTP methods that may modify the target.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum UnsafeMethod {
    Post,
    Put,
    Patch,
    Delete,
}

impl From<SafeMethod> for Method {
    fn from(value: SafeMethod) -> Self {
        match value {
            SafeMethod::Get => Self::GET,
            SafeMethod::Head => Self::HEAD,
        }
    }
}

impl From<UnsafeMethod> for Method {
    fn from(value: UnsafeMethod) -> Self {
        match value {
            UnsafeMethod::Post => Self::POST,
            UnsafeMethod::Put => Self::PUT,
            UnsafeMethod::Patch => Self::PATCH,
            UnsafeMethod::Delete => Self::DELETE,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HttpFetchArgs {
    /// Identifier of the agent user, returned by `create_agent`
    #[serde(default)]
    pub agent_identifier: String,
    /// Absolute URL to request
    pub url: String,
    /// Request method, defaults to GET
    #[serde(default)]
    pub method: Option<SafeMethod>,
    /// Additional request headers
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Skip TLS certificate validation. Only for hosts with a self-signed or
    /// expired certificate; the request becomes open to interception.
    #[serde(default)]
    pub insecure: bool,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct HttpSendArgs {
    /// Identifier of the agent user, returned by `create_agent`
    #[serde(default)]
    pub agent_identifier: String,
    /// Absolute URL to request
    pub url: String,
    /// Request method
    pub method: UnsafeMethod,
    /// Additional request headers
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Request body, sent verbatim
    #[serde(default)]
    pub body: Option<String>,
    /// Content type of the body, defaults to application/json
    #[serde(default)]
    pub content_type: Option<String>,
    /// Skip TLS certificate validation. Only for hosts with a self-signed or
    /// expired certificate; the request becomes open to interception.
    #[serde(default)]
    pub insecure: bool,
}

impl AuthentikMcp {
    pub async fn _http_fetch(&self, args: HttpFetchArgs) -> Result<CallToolResult, McpError> {
        self.http(
            &args.agent_identifier,
            &args.url,
            args.method.unwrap_or_default().into(),
            args.headers,
            None,
            args.insecure,
        )
        .await
    }

    pub async fn _http_send(&self, args: HttpSendArgs) -> Result<CallToolResult, McpError> {
        let content_type = args
            .content_type
            .unwrap_or_else(|| "application/json".to_owned());
        let body = args.body.map(|body| (content_type, body));
        self.http(
            &args.agent_identifier,
            &args.url,
            args.method.into(),
            args.headers,
            body,
            args.insecure,
        )
        .await
    }

    async fn http(
        &self,
        agent_identifier: &str,
        raw_url: &str,
        method: Method,
        headers: Option<HashMap<String, String>>,
        body: Option<(String, String)>,
        insecure: bool,
    ) -> Result<CallToolResult, McpError> {
        let url = Url::parse(raw_url)
            .map_err(|e| McpError::invalid_params(format!("invalid URL: {e}"), None))?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(McpError::invalid_params(
                format!(
                    "unsupported scheme {:?}, only http and https can be requested",
                    url.scheme()
                ),
                None,
            ));
        }
        let headers = build_headers(headers)?;
        let token = self.token_for(agent_identifier, &url).await?;
        if insecure {
            tracing::warn!(
                url = %url,
                agent = agent_identifier,
                "sending the agent's token without validating the TLS certificate"
            );
        }

        // Redirects are not followed: a 3xx from an allowed host would otherwise
        // be able to pull the agent's token to an origin it may not reach.
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .user_agent(user_agent())
            .default_headers(headers)
            .tls_danger_accept_invalid_certs(insecure)
            .build()
            .map_err(|e| {
                McpError::internal_error(format!("failed to build HTTP client: {e}"), None)
            })?;

        let mut request = client.request(method, url).bearer_auth(token);
        if let Some((content_type, body)) = body {
            let content_type = HeaderValue::from_str(&content_type).map_err(|e| {
                McpError::invalid_params(format!("invalid content type: {e}"), None)
            })?;
            request = request.header(CONTENT_TYPE, content_type).body(body);
        }
        let response = request
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("request failed: {e}"), None))?;

        let status = response.status().as_u16();
        let headers = response_headers(response.headers());
        let bytes = response.bytes().await.map_err(|e| {
            McpError::internal_error(format!("failed to read response body: {e}"), None)
        })?;
        let truncated = bytes.len() > MAX_BODY_BYTES;
        let body = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_BODY_BYTES)]).into_owned();

        let json = serde_json::to_string_pretty(&serde_json::json!({
            "status": status,
            "headers": headers,
            "body": body,
            "truncated": truncated,
        }))
        .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Find a live grant of this agent identity that permits reaching `url`, and
    /// return the access token to authenticate with.
    async fn token_for(&self, agent_identifier: &str, url: &Url) -> Result<String, McpError> {
        let grants = self.grants.lock().await;
        let Some(grants) = grants.get(agent_identifier) else {
            return Err(McpError::invalid_params(
                format!(
                    "no token exchange recorded for agent {agent_identifier}, call token_exchange first"
                ),
                None,
            ));
        };
        let now = Utc::now();
        let mut allowed = Vec::new();
        for grant in grants.iter().filter(|grant| grant.expires_at > now) {
            for origin in &grant.origins {
                if origin.allows(url) {
                    return Ok(grant.access_token.clone());
                }
                allowed.push(format!("{origin} (via {})", grant.target_id));
            }
        }
        allowed.sort_unstable();
        allowed.dedup();
        Err(McpError::invalid_params(
            if allowed.is_empty() {
                format!(
                    "agent {agent_identifier} has no unexpired token exchange with a known origin, \
                     call token_exchange again"
                )
            } else {
                format!(
                    "{url} is not adjacent to any application agent {agent_identifier} has access \
                     to, allowed origins: {}",
                    allowed.join(", ")
                )
            },
            None,
        ))
    }
}

fn build_headers(headers: Option<HashMap<String, String>>) -> Result<HeaderMap, McpError> {
    let mut map = HeaderMap::new();
    for (name, value) in headers.unwrap_or_default() {
        if RESERVED_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(McpError::invalid_params(
                format!("header {name} is set by the tool and cannot be overridden"),
                None,
            ));
        }
        let value = HeaderValue::from_str(&value).map_err(|e| {
            McpError::invalid_params(format!("invalid value for header {name}: {e}"), None)
        })?;
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|e| McpError::invalid_params(format!("invalid header name: {e}"), None))?;
        map.insert(name, value);
    }
    Ok(map)
}

/// Render response headers as a JSON object, folding repeated names together.
fn response_headers(headers: &HeaderMap) -> serde_json::Map<String, serde_json::Value> {
    let mut rendered = serde_json::Map::new();
    for (name, value) in headers {
        let value = value.to_str().unwrap_or("<non-utf8>");
        match rendered.get_mut(name.as_str()) {
            Some(serde_json::Value::String(existing)) => {
                existing.push_str(", ");
                existing.push_str(value);
            }
            _ => {
                rendered.insert(name.as_str().to_owned(), value.into());
            }
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use clap::Parser as _;

    use super::*;
    use crate::{App, CliArgs, mcp::Grant, mcp::origin::AllowedOrigin};

    /// Headers carrying the agent's identity cannot be supplied by the caller,
    /// whatever casing they use.
    #[test]
    fn test_reserved_headers_rejected() {
        for name in ["Authorization", "authorization", "HOST", "Cookie"] {
            let headers = HashMap::from([(name.to_owned(), "x".to_owned())]);
            assert!(
                build_headers(Some(headers)).is_err(),
                "{name} should be rejected"
            );
        }
    }

    /// Ordinary headers pass through, and absent headers are not an error.
    #[test]
    fn test_headers_accepted() {
        let headers = HashMap::from([("Accept".to_owned(), "application/json".to_owned())]);
        let built = build_headers(Some(headers)).expect("headers should build");
        assert_eq!(
            built.get("accept").and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        assert!(
            build_headers(None)
                .expect("no headers should build")
                .is_empty()
        );
    }

    /// Header names and values that cannot be put on the wire are refused
    /// rather than silently dropped.
    #[test]
    fn test_invalid_headers_rejected() {
        let name = HashMap::from([("bad name".to_owned(), "x".to_owned())]);
        assert!(build_headers(Some(name)).is_err());
        let value = HashMap::from([("x-test".to_owned(), "bad\nvalue".to_owned())]);
        assert!(build_headers(Some(value)).is_err());
    }

    /// Certificate validation stays on unless the caller explicitly asks for it
    /// to be skipped.
    #[test]
    fn test_insecure_defaults_off() {
        let fetch: HttpFetchArgs = serde_json::from_value(serde_json::json!({
            "url": "https://example.com/"
        }))
        .expect("args should deserialize");
        assert!(!fetch.insecure);

        let send: HttpSendArgs = serde_json::from_value(serde_json::json!({
            "url": "https://example.com/", "method": "POST"
        }))
        .expect("args should deserialize");
        assert!(!send.insecure);

        let fetch: HttpFetchArgs = serde_json::from_value(serde_json::json!({
            "url": "https://example.com/", "insecure": true
        }))
        .expect("args should deserialize");
        assert!(fetch.insecure);
    }

    /// An MCP server holding a single grant for `agent`, expiring in `ttl`.
    async fn with_grant(launch_url: &str, ttl: TimeDelta) -> AuthentikMcp {
        let mcp = AuthentikMcp::new(App::new(CliArgs::parse_from(["ak", "mcp"])));
        mcp.grants.lock().await.insert(
            "agent".to_owned(),
            vec![Grant {
                target_id: "app-uuid".to_owned(),
                access_token: "the-token".to_owned(),
                expires_at: Utc::now() + ttl,
                origins: vec![
                    AllowedOrigin::from_url(launch_url).expect("origin should be derivable"),
                ],
            }],
        );
        mcp
    }

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("test URL should parse")
    }

    /// A request to an adjacent host is authenticated with the grant's token.
    #[tokio::test]
    async fn test_adjacent_host_uses_grant_token() {
        let mcp = with_grant("https://app.example.com/", TimeDelta::minutes(5)).await;
        let token = mcp
            .token_for("agent", &url("https://api.example.com/v1/me"))
            .await
            .expect("adjacent host should be allowed");
        assert_eq!(token, "the-token");
    }

    /// An unrelated host is refused, and the caller is told what it may reach.
    #[tokio::test]
    async fn test_unrelated_host_refused() {
        let mcp = with_grant("https://app.example.com/", TimeDelta::minutes(5)).await;
        let err = mcp
            .token_for("agent", &url("https://evil.com/"))
            .await
            .expect_err("unrelated host should be refused");
        assert!(err.message.contains("https://*.example.com"), "{err:?}");
    }

    /// An expired grant is no longer usable, even for its own origin.
    #[tokio::test]
    async fn test_expired_grant_refused() {
        let mcp = with_grant("https://app.example.com/", TimeDelta::minutes(-1)).await;
        let err = mcp
            .token_for("agent", &url("https://app.example.com/"))
            .await
            .expect_err("expired grant should be refused");
        assert!(err.message.contains("unexpired"), "{err:?}");
    }

    /// Grants are per agent identity; another agent cannot borrow them.
    #[tokio::test]
    async fn test_other_agent_has_no_grant() {
        let mcp = with_grant("https://app.example.com/", TimeDelta::minutes(5)).await;
        let err = mcp
            .token_for("other-agent", &url("https://app.example.com/"))
            .await
            .expect_err("another agent should have no grant");
        assert!(err.message.contains("call token_exchange first"), "{err:?}");
    }
}
