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
};
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentAuth for AgentGRPCServer {
    async fn who_am_i(
        &self,
        _request: Request<WhoAmIRequest>,
    ) -> Result<Response<WhoAmIResponse>, Status> {
        let pc = _request.extensions().get::<ProcCredentials>();
        log::debug!("pc: {pc:?}");
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
        _request: Request<AuthorizeRequest>,
    ) -> Result<Response<AuthorizeResponse>, Status> {
        todo!()
    }
}
