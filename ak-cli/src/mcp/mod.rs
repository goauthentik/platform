use crate::{
    App,
    mcp::tools::{CreateAgentArgs, ListApplicationsArgs, RequestAccessArgs},
};
use ak_meta::user_agent;
use ak_platform::generated::{
    agent::RequestHeader,
    agent_auth::{CurrentTokenRequest, CurrentTokenResponse, current_token_request::Type},
};
use authentik_client::apis::configuration::Configuration;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
};

pub mod obo;
pub mod tools;

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

    async fn get_user_token(
        &self,
        profile: Option<String>,
    ) -> Result<CurrentTokenResponse, McpError> {
        let mut app = self.app.clone();
        let _profile = match profile {
            Some(p) => p,
            None => app.profile().await,
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
        Ok(res)
    }

    /// Build an authenticated API client configuration by fetching the current
    /// access token from the local user agent (mirrors `api::exec_api_command`).
    async fn configuration(&self, profile: Option<String>) -> Result<Configuration, McpError> {
        let token = self.get_user_token(profile).await?;

        Ok(Configuration {
            base_path: format!("{}/api/v3", token.url),
            bearer_access_token: Some(token.raw),
            user_agent: Some(user_agent()),
            ..Default::default()
        })
    }

    #[tool(description = "List applications available to the current user")]
    async fn list_applications(
        &self,
        Parameters(args): Parameters<ListApplicationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._list_applications(args).await
    }

    #[tool(description = "Request access to an application")]
    async fn request_access(
        &self,
        Parameters(args): Parameters<RequestAccessArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._request_access(args).await
    }

    #[tool(description = "Create an agent (delegate identity) for a parent user")]
    async fn create_agent(
        &self,
        Parameters(args): Parameters<CreateAgentArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._create_agent(args).await
    }
}

#[tool_handler]
impl ServerHandler for AuthentikMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("authentik Agent", ak_meta::full_version())
                    .with_website_url("https://goauthentik.io"),
            )
            .with_instructions(
                "authentik CLI MCP server. Tools: list_applications (applications available \
                 to the current), create_agent (create a delegate agent \
                 identity for the authenticated user)."
                    .to_string(),
            )
    }
}
