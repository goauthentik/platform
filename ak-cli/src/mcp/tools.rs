use crate::mcp::AuthentikMcp;
use ak_platform::generated::agent::RequestHeader;
use ak_platform::generated::agent_auth::TokenExchangeRequest;
use authentik_client::models::AgentCreateRequest;
use authentik_client::{
    apis::{
        agents_api::agents_agents_create, core_api::core_applications_list,
        requests_api::requests_grant_requests_agent_create,
    },
    models::AgentGrantRequestCreateRequest,
};
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

impl AuthentikMcp {
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
        let cbs = vec![ContentBlock::resource_link(Resource::new(
            res.fulfill_url,
            "Fulfillment URL",
        ))];
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
        let _profile = match args.profile {
            Some(p) => p,
            None => app.profile().await,
        };
        let Some(agent_token) = self.agent_token(&args.agent_identifier).await else {
            return Err(McpError::invalid_params("Agent identity not found", None));
        };
        let _res = app
            .user()
            .await
            .map_err(|e| McpError::internal_error(format!("agent connection failed: {e}"), None))?
            .auth()
            .cached_token_exchange(TokenExchangeRequest {
                header: Some(RequestHeader { profile: _profile }),
                scopes: args.scopes.unwrap_or_default(),
                audience: args.target_id,
                actor_token: Some(agent_token.clone()),
                actor_token_type: Some(
                    "goauthentik.io/oauth/token-type/authentik_token".to_owned(),
                ),
            })
            .await
            .map_err(|e| McpError::internal_error(format!("failed to exchange token: {e}"), None))?
            .into_inner();
        Ok(CallToolResult::success(vec![]))
    }
}
