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
    string::PlatformString,
};
use ak_platform_authz::AuthorizeAction;
use reqwest::Method;
use std::time::Duration;
use tonic::{Request, Response, Status};

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
            .http_client()
            .request(Method::GET, "")
            .bearer_auth("")
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
                nbf: Some(c.nbf.into()),
                iat: Some(c.iat.into()),
                jti: c.jti,
            }),
            raw: token.access_token,
            url: "".to_string(),
        }))
    }

    async fn cached_token_exchange(
        &self,
        _request: Request<TokenExchangeRequest>,
    ) -> Result<Response<TokenExchangeResponse>, Status> {
        todo!()
    }

    async fn device_token_exchange(
        &self,
        _request: Request<DeviceTokenExchangeRequest>,
    ) -> Result<Response<TokenExchangeResponse>, Status> {
        todo!()
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
