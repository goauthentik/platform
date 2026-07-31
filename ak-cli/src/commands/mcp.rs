use crate::App;
use ak_meta::user_agent;
use ak_platform::generated::{
    agent::RequestHeader,
    agent_auth::{CurrentTokenRequest, current_token_request::Type},
};
use authentik_client::apis::{
    agents_api::agents_agents_create, configuration::Configuration,
    core_api::core_applications_list,
};
use authentik_client::models::AgentCreateRequest;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
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
    /// UUIDs of applications to grant the agent access to
    #[serde(default)]
    pub applications: Option<Vec<String>>,
    /// Profile to use (defaults to currently active profile)
    #[serde(default)]
    pub profile: Option<String>,
}

#[derive(Clone)]
pub struct AuthentikMcp {
    app: App,
    tool_router: ToolRouter<AuthentikMcp>,
}

#[tool_router]
impl AuthentikMcp {
    pub fn new(app: App) -> Self {
        Self {
            app,
            tool_router: Self::tool_router(),
        }
    }

    /// Build an authenticated API client configuration by fetching the current
    /// access token from the local user agent (mirrors `api::exec_api_command`).
    async fn configuration(&self, profile: Option<String>) -> Result<Configuration, McpError> {
        let mut app = self.app.clone();
        let _profile = match profile {
            Some(p) => p,
            None => app.profile().await
        };
        let res = app
            .user()
            .await
            .map_err(|e| McpError::internal_error(format!("agent connection failed: {e}"), None))?
            .auth()
            .get_current_token(CurrentTokenRequest {
                header: Some(RequestHeader { profile: _profile }),
                r#type: Type::Verified as i32,
            })
            .await
            .map_err(|e| McpError::internal_error(format!("failed to get API token: {e}"), None))?
            .into_inner();

        Ok(Configuration {
            base_path: format!("{}/api/v3", res.url),
            bearer_access_token: Some(res.raw),
            user_agent: Some(user_agent()),
            ..Default::default()
        })
    }

    #[tool(description = "List applications available to the current user")]
    async fn list_applications(
        &self,
        Parameters(args): Parameters<ListApplicationsArgs>,
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

    #[tool(description = "Create an agent (delegate identity) for a parent user")]
    async fn create_agent(
        &self,
        Parameters(args): Parameters<CreateAgentArgs>,
    ) -> Result<CallToolResult, McpError> {
        let config = self.configuration(args.profile).await?;
        let applications = args
            .applications
            .map(|v| {
                v.into_iter()
                    .map(|s| Uuid::parse_str(&s))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()
            .map_err(|e| McpError::invalid_params(format!("invalid application UUID: {e}"), None))?;
        let req = AgentCreateRequest {
            label: args.label,
            applications,
            ..Default::default()
        };
        let result = agents_agents_create(&config, Some(req))
            .await
            .map_err(|e| McpError::internal_error(format!("create agent failed: {e}"), None))?;
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(format!("serialize failed: {e}"), None))?;
        Ok(CallToolResult::success(vec![ContentBlock::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for AuthentikMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "authentik CLI MCP server. Tools: list_applications (applications available \
                 to the current), create_agent (create a delegate agent \
                 identity for the authenticated user)."
                    .to_string(),
            )
    }
}

pub async fn mcp(app: App) -> eyre::Result<()> {
    let service = AuthentikMcp::new(app)
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {e:?}"))?;
    service.waiting().await?;
    Ok(())
}
