use std::time::Duration;

use ak_platform::{
    generated::{
        agent::ResponseHeader,
        agent_auth::{
            AuthorizeRequest, AuthorizeResponse, CurrentTokenRequest, CurrentTokenResponse,
            DeviceTokenExchangeRequest, TokenExchangeRequest, TokenExchangeResponse, WhoAmIRequest,
            WhoAmIResponse, agent_auth_server::AgentAuth,
        },
    },
    net::server::creds::ProcCredentials,
    string::PlatformString,
};
use ak_platform_authz::AuthorizeAction;
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentAuth for AgentGRPCServer {
    async fn who_am_i(
        &self,
        request: Request<WhoAmIRequest>,
    ) -> Result<Response<WhoAmIResponse>, Status> {
        let pc = request.extensions().get::<ProcCredentials>();
        log::trace!("pc: {pc:?}");
        log::debug!("whoami");
        Ok(Response::new(WhoAmIResponse {
            header: Some(ResponseHeader { successful: true }),
            body: "".to_string(),
        }))
    }

    async fn get_current_token(
        &self,
        _request: Request<CurrentTokenRequest>,
    ) -> Result<Response<CurrentTokenResponse>, Status> {
        todo!()
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
                Ok(PlatformString::new()
                    .with_darwin(&format!("authorize access to '{}'", service)))
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
