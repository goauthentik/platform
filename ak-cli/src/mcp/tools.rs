use std::time::Duration;

use crate::mcp::{AuthentikMcp, Grant, origin::AllowedOrigin};
use ak_meta::user_agent;
use ak_platform::generated::agent::RequestHeader;
use ak_platform::generated::agent_auth::TokenExchangeRequest;
use ak_platform::grpc::assert_response_valid;
use authentik_client::models::AgentCreateRequest;
use authentik_client::{
    apis::{
        agents_api::agents_agents_create, core_api::core_applications_list,
        requests_api::requests_grant_requests_agent_create,
    },
    models::{AgentGrantRequestCreateRequest, Application},
};
use chrono::{TimeDelta, Utc};
use rmcp::{ErrorData as McpError, model::*, schemars};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListApplicationsArgs {
    /// Filter applications by name/slug
    #[serde(default)]
    pub search: Option<String>,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateAgentArgs {
    /// Human-readable label for the agent
    #[serde(default)]
    pub label: Option<String>,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RequestAccessArgs {
    /// UUIDs of applications to grant the agent access to
    #[serde(default)]
    pub applications: Option<Vec<String>>,
    /// Identifier of the agent user, returned by `create_agent`
    #[serde(default)]
    pub agent_identifier: String,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TokenExchangeArgs {
    /// PBM UUID/Client ID of target application
    #[serde(default)]
    pub target_id: String,
    /// Identifier of the agent user, returned by `create_agent`
    #[serde(default)]
    pub agent_identifier: String,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
    /// Scopes to request
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlueprintValidateArgs {
    /// Complete proposed Blueprint YAML. This tool performs no changes.
    pub content: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlueprintApplyArgs {
    /// Complete proposed Blueprint YAML to validate and apply.
    pub content: String,
    /// Name of the Agent connector to apply through. Optional when the instance
    /// has exactly one Agent connector.
    #[serde(default)]
    pub connector: Option<String>,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Deserialize)]
struct BlueprintApplyResult {
    success: bool,
}

/// Resolve an Agent connector's UUID by name, or the sole connector when no name
/// is given. Uses the operator's token (the endpoints API is not in the typed
/// client, so this is a raw request).
async fn resolve_connector(
    client: &reqwest::Client,
    base: &str,
    bearer: &str,
    name: Option<&str>,
) -> Result<String, McpError> {
    let resp = client
        .get(format!("{base}/api/v3/endpoints/agents/connectors/"))
        .bearer_auth(bearer)
        .send()
        .await
        .map_err(|e| {
            McpError::internal_error(format!("listing agent connectors failed: {e}"), None)
        })?;
    if !resp.status().is_success() {
        return Err(McpError::internal_error(
            format!(
                "listing agent connectors failed: HTTP {}",
                resp.status().as_u16()
            ),
            None,
        ));
    }
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| McpError::internal_error(format!("invalid connector list: {e}"), None))?;
    let mut connectors: Vec<(String, String)> = body
        .get("results")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|c| {
                    let uuid = c.get("connector_uuid")?.as_str()?.to_string();
                    let cname = c.get("name")?.as_str()?.to_string();
                    Some((uuid, cname))
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some(name) = name {
        connectors.retain(|(_, n)| n == name);
        match connectors.as_slice() {
            [(uuid, _)] => Ok(uuid.clone()),
            [] => Err(McpError::invalid_params(
                format!("no Agent connector named {name:?}"),
                None,
            )),
            _ => Err(McpError::invalid_params(
                format!("more than one Agent connector named {name:?}"),
                None,
            )),
        }
    } else {
        match connectors.as_slice() {
            [(uuid, _)] => Ok(uuid.clone()),
            [] => Err(McpError::invalid_params(
                "no Agent connector is configured on this instance".to_string(),
                None,
            )),
            _ => {
                let names: Vec<&str> = connectors.iter().map(|(_, n)| n.as_str()).collect();
                Err(McpError::invalid_params(
                    format!(
                        "several Agent connectors exist ({}); pass `connector`",
                        names.join(", ")
                    ),
                    None,
                ))
            }
        }
    }
}

impl AuthentikMcp {
    pub async fn _validate_blueprint(
        &self,
        args: BlueprintValidateArgs,
    ) -> Result<CallToolResult, McpError> {
        let result = crate::mcp::blueprint::validate_blueprint(&args.content);
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    /// Validate a proposed Blueprint locally, then apply it on the server as the
    /// bounded, least-privilege Agent identity — never as the operator. The local
    /// validation is fast feedback and a courtesy; the server enforces the same
    /// policy plus per-model RBAC independently.
    pub async fn _blueprint_apply(
        &self,
        args: BlueprintApplyArgs,
    ) -> Result<CallToolResult, McpError> {
        // 1. Validate locally — never send known-bad content to the server.
        let validation = crate::mcp::blueprint::validate_blueprint(&args.content);
        if !validation.ok {
            let json = serde_json::to_string_pretty(&validation)
                .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "Blueprint failed local validation and was not applied:\n{json}"
            ))]));
        }

        // 2. Authenticate through the local agent (this triggers the consent
        //    gate). A missing profile means the device isn't set up yet.
        let token = match self.get_user_token(args.profile).await {
            Ok(token) => token,
            Err(_) => {
                return Ok(CallToolResult::success(vec![ContentBlock::text(
                    "This device is not set up to talk to authentik yet. Run \
                     `ak config setup --authentik-url <your-authentik-url>` to connect it, then \
                     try applying again."
                        .to_string(),
                )]));
            }
        };
        let base = token.url.trim_end_matches('/').to_string();
        let bearer = token.raw;
        let client = reqwest::Client::builder()
            // Do not let a configured instance redirect this bearer token to a
            // different origin.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(user_agent())
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| {
                McpError::internal_error(format!("failed to build HTTP client: {e}"), None)
            })?;

        // 3. Resolve the connector, then apply as the bounded server identity.
        let pk = resolve_connector(&client, &base, &bearer, args.connector.as_deref()).await?;
        let resp = client
            .post(format!(
                "{base}/api/v3/endpoints/agents/connectors/{pk}/apply_blueprint/"
            ))
            .bearer_auth(&bearer)
            .json(&serde_json::json!({ "content": args.content }))
            .send()
            .await
            .map_err(|e| McpError::internal_error(format!("apply request failed: {e}"), None))?;
        let status = resp.status();
        if !status.is_success() {
            return Ok(CallToolResult::success(vec![ContentBlock::text(format!(
                "The server rejected the apply (HTTP {}).",
                status.as_u16()
            ))]));
        }
        let result: BlueprintApplyResult = resp
            .json()
            .await
            .map_err(|e| McpError::internal_error(format!("invalid apply response: {e}"), None))?;
        let message = if result.success {
            "Blueprint applied successfully."
        } else {
            "Blueprint did not pass server validation and was not applied."
        };
        Ok(CallToolResult::success(vec![ContentBlock::text(message)]))
    }

    pub async fn _list_applications(
        &self,
        args: ListApplicationsArgs,
    ) -> Result<CallToolResult, McpError> {
        let config = self.configuration(args.profile).await?;
        let result = core_applications_list(
            &config,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            args.search.as_deref(),
            None,
            None,
        )
        .await
        .map_err(|e| McpError::internal_error(format!("list applications failed: {e}"), None))?;
        // Ignore pagination here as the app list endpoint for policy-accessible apps
        // doesn't use it.
        let json = serde_json::to_string_pretty(&result.results)
            .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    pub async fn _request_access(
        &self,
        args: RequestAccessArgs,
    ) -> Result<CallToolResult, McpError> {
        let mut config = self.configuration(args.profile).await?;
        let Some(agent_token) = self.agent_token(&args.agent_identifier).await else {
            return Err(McpError::invalid_params("Agent identity not found", None));
        };
        config.bearer_access_token = Some(agent_token);
        let Some(pbms) = args
            .applications
            .map(|v| {
                v.into_iter()
                    .map(|s| Uuid::parse_str(&s))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|e| {
                McpError::invalid_params(format!("invalid application UUID: {e}"), None)
            })?
        else {
            return Err(McpError::invalid_params(
                "Empty or invalid applications",
                None,
            ));
        };
        let res =
            requests_grant_requests_agent_create(&config, AgentGrantRequestCreateRequest { pbms })
                .await
                .map_err(|e| {
                    McpError::internal_error(format!("request access failed: {e}"), None)
                })?;
        let cbs = vec![
            ContentBlock::text(
                "Show this link to the user and ask them to open it and approve or deny access.",
            ),
            ContentBlock::resource_link(Resource::new(res.fulfill_url, "Fulfillment URL")),
        ];
        Ok(CallToolResult::success(cbs))
    }

    /// Look up the token of an agent identity created earlier in this session.
    async fn agent_token(&self, identifier: &str) -> Option<String> {
        self.agent_tokens.lock().await.get(identifier).cloned()
    }

    pub async fn _create_agent(&self, args: CreateAgentArgs) -> Result<CallToolResult, McpError> {
        let config = self.configuration(args.profile).await?;
        let req = AgentCreateRequest {
            label: args.label,
            ..Default::default()
        };
        let result = agents_agents_create(&config, Some(req))
            .await
            .map_err(|e| McpError::internal_error(format!("create agent failed: {e}"), None))?;
        self.agent_tokens
            .lock()
            .await
            .insert(result.agent.username.clone(), result.token);
        let mut cb = vec![ContentBlock::text(format!(
            "The agent identity was successfully created. Use {} in future tool calls to use its identity.",
            result.agent.username
        ))];
        if let Some(Some(exp)) = result.agent.expires {
            cb.push(ContentBlock::text(format!(
                "The agent identity will auto-expire at {}. After this time has passed, re-request a new identity.",
                    exp
            )));
        }
        Ok(CallToolResult::success(cb))
    }

    pub async fn _token_exchange(
        &self,
        args: TokenExchangeArgs,
    ) -> Result<CallToolResult, McpError> {
        let mut app = self.app.clone();
        let _profile = match args.profile.clone() {
            Some(p) => p,
            None => app.profile().await,
        };
        let Some(agent_token) = self.agent_token(&args.agent_identifier).await else {
            return Err(McpError::invalid_params("Agent identity not found", None));
        };
        let res = app
            .user()
            .await
            .map_err(|e| McpError::internal_error(format!("agent connection failed: {e}"), None))?
            .auth()
            .cached_token_exchange(TokenExchangeRequest {
                header: Some(RequestHeader { profile: _profile }),
                scopes: args.scopes.unwrap_or_default(),
                audience: args.target_id.clone(),
                actor_token: Some(agent_token.clone()),
                actor_token_type: Some(
                    "goauthentik.io/oauth/token-type/authentik_token".to_owned(),
                ),
            })
            .await
            .map_err(|e| McpError::internal_error(format!("failed to exchange token: {e}"), None))?
            .into_inner();
        assert_response_valid(res.header)
            .map_err(|e| McpError::internal_error(format!("token exchange failed: {e}"), None))?;

        let application = self.application_for(&args.target_id, args.profile).await?;
        tracing::debug!("Application for target: {:?}", application);
        let origins = application
            .as_ref()
            .map(origins_for_application)
            .unwrap_or_default();
        let expires_at = Utc::now() + TimeDelta::seconds(res.expires_in);

        let mut cb = vec![ContentBlock::text(format!(
            "Exchanged a token for {}, valid until {expires_at}.",
            args.target_id
        ))];
        cb.push(ContentBlock::text(if origins.is_empty() {
            // Without an origin there is nothing to check requests against, so
            // the HTTP tools refuse rather than fall back to allowing anything.
            format!(
                "No launch URL could be resolved for {}, so http_fetch and http_send cannot be \
                 used with this token.",
                args.target_id
            )
        } else {
            format!(
                "http_fetch and http_send may now use agent {} against: {}.",
                args.agent_identifier,
                origins
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }));

        self.grants
            .lock()
            .await
            .entry(args.agent_identifier)
            .or_default()
            .push(Grant {
                target_id: args.target_id,
                access_token: res.access_token,
                expires_at,
                origins,
            });
        Ok(CallToolResult::success(cb))
    }

    /// Resolve a token exchange audience to the application it belongs to.
    /// `Application.pk` and `Application.pbm_uuid` hold the same value server
    /// side, so a PBM UUID identifies the application on its own.
    async fn application_for(
        &self,
        target_id: &str,
        profile: Option<String>,
    ) -> Result<Option<Application>, McpError> {
        let config = self.configuration(profile).await?;
        let result = core_applications_list(
            &config,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(100),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| McpError::internal_error(format!("list applications failed: {e}"), None))?;
        Ok(result
            .results
            .into_iter()
            .find(|app| app.pbm_uuid.to_string().to_lowercase() == target_id))
    }
}

/// Origins an agent may reach once it holds a token for `application`, derived
/// from the URLs the application is reachable under.
fn origins_for_application(application: &Application) -> Vec<AllowedOrigin> {
    let mut origins = Vec::new();
    for url in [&application.launch_url, &application.meta_launch_url]
        .into_iter()
        .flatten()
    {
        if let Some(origin) = AllowedOrigin::from_url(url)
            && !origins.contains(&origin)
        {
            origins.push(origin);
        }
    }
    origins
}
