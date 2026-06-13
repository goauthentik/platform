use ak_platform::{
    generated::{
        agent::{ResponseHeader, Token},
        agent_auth::{
            AuthorizeRequest, AuthorizeResponse, CurrentTokenRequest, CurrentTokenResponse,
            DeviceTokenExchangeRequest, TokenExchangeRequest, TokenExchangeResponse, WhoAmIRequest,
            WhoAmIResponse, agent_auth_server::AgentAuth, current_token_request::Type,
        },
    },
    net::server::creds::ProcCredentials,
    storage::cache::{Cache, CacheData},
    string::PlatformString,
};
use ak_platform_authz::AuthorizeAction;
use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tonic::{Request, Response, Status};

#[derive(Clone, Serialize, Deserialize)]
struct CachedExchangeToken {
    #[serde(rename = "at")]
    access_token: String,
    expires_in: i64,
    #[serde(rename = "iat")]
    created: DateTime<Utc>,
}

impl CacheData for CachedExchangeToken {
    fn expiry(&self) -> DateTime<Utc> {
        self.created + chrono::TimeDelta::seconds(self.expires_in)
    }
}

#[derive(Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    expires_in: Option<i64>,
}

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentAuth for AgentGRPCServer {
    async fn who_am_i(
        &self,
        request: Request<WhoAmIRequest>,
    ) -> Result<Response<WhoAmIResponse>, Status> {
        let pc = request.extensions().get::<ProcCredentials>().cloned();
        let profile = self
            .profile_for_request(request.into_inner().header)
            .await?;

        AuthorizeAction {
            message: Box::new(|c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access to your account info in '{cmd}'"))
                    .with_windows(format!("'{cmd}' is attempting to access your account info"))
                    .with_linux(format!("'{cmd}' is attempting to access your account info")))
            }),
            uid: Box::new(|c| c.clone().proc_info()?.unique_process_id()),
            timeout_success: Duration::from_secs(0),
            timeout_denied: Duration::from_secs(0),
        }
        .prompt_grpc(pc)
        .await?;

        let req = match profile
            .clone()
            .http_client()
            .request(
                Method::GET,
                format!("{}/application/o/userinfo/", profile.clone().authentik_url),
            )
            .bearer_auth(profile.access_token())
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return Err(Status::from_error(e.into())),
        };
        if !req.status().is_success() {
            return Err(Status::internal("Invalid status code for whoami request"));
        }

        Ok(Response::new(WhoAmIResponse {
            header: Some(ResponseHeader { successful: true }),
            body: req
                .text()
                .await
                .map_err(|e| Status::from_error(Box::from(e)))?,
        }))
    }

    async fn get_current_token(
        &self,
        request: Request<CurrentTokenRequest>,
    ) -> Result<Response<CurrentTokenResponse>, Status> {
        let proc_creds = request.extensions().get::<ProcCredentials>().cloned();
        let inner_req = request.into_inner();
        let token_manager = self
            .agent
            .gtm
            .for_profile(
                &inner_req
                    .clone()
                    .header
                    .ok_or(Status::invalid_argument("missing header"))?
                    .profile,
            )
            .await
            .ok_or(Status::invalid_argument("profile not found"))?;

        AuthorizeAction {
            message: Box::new(|c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access to your account in '{cmd}'"))
                    .with_windows(format!("'{cmd}' is attempting to access your account"))
                    .with_linux(format!("'{cmd}' is attempting to access your account")))
            }),
            uid: Box::new(|_| Ok("".to_string())),
            timeout_success: Duration::from_secs(0),
            timeout_denied: Duration::from_secs(0),
        }
        .prompt_grpc(proc_creds)
        .await?;

        let token = match inner_req.r#type() {
            Type::Unspecified => Err(Status::invalid_argument("unsupported token type")),
            Type::Unverified => Ok(token_manager
                .unverified()
                .await
                .map_err(Status::from_error)?),
            Type::Verified => Ok(token_manager.token().await.map_err(Status::from_error)?),
        }?;
        let c = token.claims().map_err(Status::from_error)?;

        Ok(Response::new(CurrentTokenResponse {
            header: Some(ResponseHeader { successful: true }),
            token: Some(Token {
                preferred_username: c.preferred_username,
                iss: c.iss,
                sub: c.sub,
                aud: c.aud,
                exp: Some(c.exp.into()),
                nbf: None,
                iat: Some(c.iat.into()),
                jti: c.jti,
            }),
            raw: token.access_token,
            url: "".to_string(),
        }))
    }

    async fn cached_token_exchange(
        &self,
        request: Request<TokenExchangeRequest>,
    ) -> Result<Response<TokenExchangeResponse>, Status> {
        let pc = request.extensions().get::<ProcCredentials>().cloned();
        let inner = request.into_inner();
        let profile_name = inner
            .header
            .as_ref()
            .ok_or(Status::invalid_argument("missing header"))?
            .profile
            .clone();
        let profile = self.profile_for_request(inner.header).await?;
        let client_id = inner.client_id;

        let cid1 = client_id.clone();
        let cid2 = client_id.clone();
        AuthorizeAction {
            message: Box::new(move |c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!(
                        "authorize access to your account '{cid1}' in '{cmd}'"
                    ))
                    .with_windows(format!(
                        "'{cid1}' is attempting to access your account in '{cmd}'"
                    ))
                    .with_linux(format!(
                        "'{cid1}' is attempting to access your account in '{cmd}'"
                    )))
            }),
            uid: Box::new(move |c| {
                let pid = c.clone().proc_info()?.unique_process_id()?;
                Ok(format!("{cid2}:{pid}"))
            }),
            timeout_success: Duration::from_secs(30 * 60),
            timeout_denied: Duration::from_secs(1),
        }
        .prompt_grpc(pc)
        .await?;

        let cache = Cache::<CachedExchangeToken>::new(
            profile_name.clone(),
            vec!["token-cache".to_string(), client_id.clone()],
        );
        if let Ok(cached) = cache.get().await {
            log::debug!("cached_token_exchange: returning cached token for '{client_id}'");
            return Ok(Response::new(TokenExchangeResponse {
                header: Some(ResponseHeader { successful: true }),
                access_token: cached.access_token,
                expires_in: cached.expires_in,
            }));
        }

        let token_url = format!("{}/application/o/token/", profile.authentik_url);
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "client_credentials")
            .append_pair("client_id", &client_id)
            .append_pair(
                "client_assertion_type",
                "urn:ietf:params:oauth:client-assertion-type:jwt-bearer",
            )
            .append_pair("client_assertion", &profile.access_token())
            .append_pair("scope", "openid email profile")
            .finish();

        let res = reqwest::Client::new()
            .post(&token_url)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(
                reqwest::header::USER_AGENT,
                format!("authentik-agent v{}", env!("CARGO_PKG_VERSION")),
            )
            .body(body)
            .send()
            .await
            .map_err(|e| Status::from_error(e.into()))?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(Status::internal(format!("token exchange failed: {body}")));
        }

        let new_token: OAuthTokenResponse =
            res.json().await.map_err(|e| Status::from_error(e.into()))?;
        let expires_in = new_token.expires_in.unwrap_or(0);

        let cached = CachedExchangeToken {
            access_token: new_token.access_token.clone(),
            expires_in,
            created: Utc::now(),
        };
        let cache = Cache::<CachedExchangeToken>::new(
            profile_name,
            vec!["token-cache".to_string(), client_id.clone()],
        );
        if let Err(e) = cache.set(cached).await {
            log::warn!("cached_token_exchange: failed to write cache: {e:?}");
        }

        log::debug!("cached_token_exchange: exchanged new token for '{client_id}'");
        Ok(Response::new(TokenExchangeResponse {
            header: Some(ResponseHeader { successful: true }),
            access_token: new_token.access_token,
            expires_in,
        }))
    }

    async fn device_token_exchange(
        &self,
        request: Request<DeviceTokenExchangeRequest>,
    ) -> Result<Response<TokenExchangeResponse>, Status> {
        let pc = request.extensions().get::<ProcCredentials>().cloned();
        let inner = request.into_inner();
        let profile = self.profile_for_request(inner.header).await?;
        let device_name = inner.device_name;

        let dn1 = device_name.clone();
        let dn2 = device_name.clone();
        AuthorizeAction {
            message: Box::new(move |c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access device '{dn1}' in '{cmd}'"))
                    .with_windows(format!("'{dn1}' is attempting to access '{cmd}'"))
                    .with_linux(format!("'{dn1}' is attempting to access '{cmd}'")))
            }),
            uid: Box::new(move |c| {
                let pid = c.clone().proc_info()?.unique_process_id()?;
                Ok(format!("{dn2}:{pid}"))
            }),
            timeout_success: Duration::from_secs(30 * 60),
            timeout_denied: Duration::from_secs(5 * 60),
        }
        .prompt_grpc(pc)
        .await?;

        let api_config = authentik_client::apis::configuration::Configuration {
            base_path: format!("{}/api/v3", profile.authentik_url),
            bearer_access_token: Some(profile.access_token()),
            user_agent: Some(format!("authentik-agent v{}", env!("CARGO_PKG_VERSION"))),
            ..Default::default()
        };

        let dt =
            authentik_client::apis::endpoints_api::endpoints_agents_connectors_auth_fed_create(
                &api_config,
                &device_name,
            )
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        log::debug!("device_token_exchange: exchanged token for device '{device_name}'");
        Ok(Response::new(TokenExchangeResponse {
            header: Some(ResponseHeader { successful: true }),
            access_token: dt.token,
            expires_in: dt.expires_in.unwrap_or(0) as i64,
        }))
    }

    async fn authorize(
        &self,
        request: Request<AuthorizeRequest>,
    ) -> Result<Response<AuthorizeResponse>, Status> {
        let pc = request.extensions().get::<ProcCredentials>().cloned();
        let inner = request.into_inner();
        let service = inner.service.clone();
        let uid = inner.uid.clone();

        let result = AuthorizeAction {
            message: Box::new(move |_c| {
                Ok(PlatformString::new().with_darwin(format!("authorize access to '{}'", service)))
            }),
            uid: Box::new(move |_c| Ok(uid.clone())),
            timeout_success: Duration::from_hours(2),
            timeout_denied: Duration::from_mins(5),
        }
        .prompt_grpc(pc)
        .await?;

        Ok(Response::new(AuthorizeResponse {
            header: Some(ResponseHeader { successful: result }),
        }))
    }
}
