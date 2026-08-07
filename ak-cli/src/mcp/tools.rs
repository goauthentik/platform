use crate::mcp::AuthentikMcp;
use crate::mcp::obo::token_exchange;
use authentik_client::models::AgentCreateRequest;
use authentik_client::{
    apis::{
        agents_api::agents_agents_create, core_api::core_applications_list,
        requests_api::requests_grant_requests_agent_create,
    },
    models::AgentGrantRequestCreateRequest,
};
use rmcp::{ErrorData as McpError, handler::server::wrapper::Parameters, model::*, schemars};
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
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TokenExchangeArgs {
    /// Token of the agent user
    #[serde(default)]
    pub agent_token: String,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
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
        let config = self.configuration(args.profile).await?;
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
                    McpError::internal_error(format!("list applications failed: {e}"), None)
                })?;
        let cbs = vec![
            // ContentBlock::text(""),
            ContentBlock::resource_link(Resource::new(res.fulfill_url, "Fulfillment URL")),
        ];
        Ok(CallToolResult::success(cbs))
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
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }

    pub async fn _token_exchange(
        &self,
        args: TokenExchangeArgs,
    ) -> Result<CallToolResult, McpError> {
        // todo: implement OBO token exchange
        let user_token = self.get_user_token(args.profile).await?;
        let agent_token = args.agent_token;
        token_exchange(
            &reqwest::Client::new(),
            "",
            "",
            "",
            &user_token.raw,
            &agent_token,
            "",
        )
        .await
        .map_err(|e| McpError::internal_error(format!("token exchange failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![]))
    }
}
