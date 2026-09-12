use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::Mutex;

use crate::{
    App,
    mcp::{
        http::{HttpFetchArgs, HttpSendArgs},
        origin::AllowedOrigin,
        tools::{
            BlueprintApplyArgs, BlueprintValidateArgs, CreateAgentArgs, ListApplicationsArgs,
            RequestAccessArgs, TokenExchangeArgs,
        },
    },
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

pub mod blueprint;
pub mod http;
pub mod origin;
pub mod tools;

/// An on-behalf-of token an agent identity obtained for a single application,
/// together with the origins it may be used against.
pub(crate) struct Grant {
    pub target_id: String,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub origins: Vec<AllowedOrigin>,
}

#[derive(Clone)]
pub struct AuthentikMcp {
    app: App,
    tool_router: ToolRouter<AuthentikMcp>,
    pub(crate) agent_tokens: Arc<Mutex<HashMap<String, String>>>,
    /// Agent identifier to the grants it obtained during this session.
    pub(crate) grants: Arc<Mutex<HashMap<String, Vec<Grant>>>>,
}

#[tool_router]
impl AuthentikMcp {
    pub fn new(app: App) -> Self {
        Self {
            app,
            tool_router: Self::tool_router(),
            agent_tokens: Arc::new(Mutex::new(HashMap::new())),
            grants: Arc::new(Mutex::new(HashMap::new())),
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

    #[tool(
        description = "List applications available to the current user",
        annotations(title = "List available applications")
    )]
    async fn list_applications(
        &self,
        Parameters(args): Parameters<ListApplicationsArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._list_applications(args).await
    }

    #[tool(
        description = "Request access to an application",
        annotations(title = "Request application access")
    )]
    async fn request_access(
        &self,
        Parameters(args): Parameters<RequestAccessArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._request_access(args).await
    }

    #[tool(
        description = "Create an agent (delegate identity) for a parent user",
        annotations(title = "Create agent identity")
    )]
    async fn create_agent(
        &self,
        Parameters(args): Parameters<CreateAgentArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._create_agent(args).await
    }

    #[tool(
        description = "Exchange an agent identity token for an OIDC On-behalf-of token",
        annotations(title = "Exchange token")
    )]
    async fn token_exchange(
        &self,
        Parameters(args): Parameters<TokenExchangeArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._token_exchange(args).await
    }

    #[tool(
        description = "Validate proposed authentik Blueprint YAML before applying it. Rejects destructive entries, secret fields, non-curated references, unsafe tags, and malformed YAML. Does not make changes.",
        annotations(title = "Validate Blueprint", read_only_hint = true)
    )]
    async fn blueprint_validate(
        &self,
        Parameters(args): Parameters<BlueprintValidateArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._validate_blueprint(args).await
    }

    #[tool(
        description = "Validate and apply a proposed authentik Blueprint. The content is checked \
                       against the local policy, then applied on the server as a bounded, \
                       least-privilege identity (never as you), so it can only add or change \
                       Applications and OAuth2/SAML providers. If this device is not set up yet, \
                       the tool explains how to connect it with `ak config setup`.",
        annotations(
            title = "Apply Blueprint",
            read_only_hint = false,
            destructive_hint = true
        )
    )]
    async fn blueprint_apply(
        &self,
        Parameters(args): Parameters<BlueprintApplyArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._blueprint_apply(args).await
    }

    #[tool(
        description = "Send a read-only HTTP request (GET/HEAD) as an agent identity. Only hosts \
                       sharing a registrable domain with an application the agent exchanged a token \
                       for can be reached; that token is attached automatically. Redirects are not \
                       followed, 3xx responses are returned as-is. Set insecure only after a \
                       request has actually failed TLS validation, never up front.",
        annotations(
            title = "HTTP request (read-only)",
            read_only_hint = true,
            open_world_hint = true
        )
    )]
    async fn http_fetch(
        &self,
        Parameters(args): Parameters<HttpFetchArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._http_fetch(args).await
    }

    #[tool(
        description = "Send a modifying HTTP request (POST/PUT/PATCH/DELETE) as an agent identity. \
                       Subject to the same origin restrictions as http_fetch, and the same caveat \
                       about insecure.",
        annotations(
            title = "HTTP request (modifying)",
            read_only_hint = false,
            destructive_hint = true,
            open_world_hint = true
        )
    )]
    async fn http_send(
        &self,
        Parameters(args): Parameters<HttpSendArgs>,
    ) -> Result<CallToolResult, McpError> {
        self._http_send(args).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AuthentikMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("authentik Agent", ak_meta::full_version())
                    .with_website_url("https://goauthentik.io"),
            )
            .with_instructions(
                "authentik CLI MCP server. Tools: list_applications (applications available to \
                 the current user), create_agent (create a delegate agent identity for the \
                 authenticated user), request_access (ask for access to applications on behalf of \
                 an agent identity), token_exchange (exchange an agent identity for an \
                 on-behalf-of token for one application), http_fetch and http_send (call an \
                 application as the agent identity), and blueprint_validate / blueprint_apply \
                 (check, then apply, proposed Blueprint content — blueprint_apply validates first \
                 and applies as a bounded server identity, so prefer it over crafting raw writes). \
                 The usual order for application access is create_agent, request_access, \
                 token_exchange, then http_fetch/http_send; those last two can only reach hosts \
                 adjacent to an application the agent exchanged a token for."
                    .to_string(),
            )
    }
}
