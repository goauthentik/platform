use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use tonic::transport::Channel;
use tower::Service;

use crate::{
    generated::{
        agent_auth::agent_auth_client::AgentAuthClient,
        agent_cache::agent_cache_client::AgentCacheClient,
        agent_ctrl::agent_ctrl_client::AgentCtrlClient,
        ping::ping_client::PingClient,
    },
    grpc::{
        grpc_endpoint,
        ssh::{SSHService, SSHTunnel},
    },
    platform::paths::{AgentSocketID, agent_socket_path},
};

pub struct Client<C> {
    c: C,
}

impl Client<Channel> {
    pub async fn new_with_id(id: AgentSocketID) -> Result<Self, Box<dyn Error>> {
        Self::new_with_path(agent_socket_path(id)?.for_current()).await
    }

    pub async fn new_with_path(p: String) -> Result<Self, Box<dyn Error>> {
        let c = grpc_endpoint(p).await?;
        Ok(Client { c })
    }
}

impl Client<SSHService> {
    pub async fn new_with_ssh() -> Result<Self, Box<dyn Error>> {
        let service = SSHTunnel::new().await?.service(());
        Ok(Client { c: service })
    }
}

type BoxError = Box<dyn Error + Send + Sync>;
type AnyBody = UnsyncBoxBody<Bytes, BoxError>;

enum AnyServiceInner {
    Socket(Channel),
    SSH(SSHService),
}

pub struct AnyService(AnyServiceInner);

impl Service<http::Request<tonic::body::Body>> for AnyService {
    type Response = http::Response<AnyBody>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match &mut self.0 {
            AnyServiceInner::Socket(c) => c.poll_ready(cx).map_err(Into::into),
            AnyServiceInner::SSH(s) => {
                <SSHService as Service<http::Request<tonic::body::Body>>>::poll_ready(s, cx)
            }
        }
    }

    fn call(&mut self, req: http::Request<tonic::body::Body>) -> Self::Future {
        match &mut self.0 {
            AnyServiceInner::Socket(c) => {
                let fut = c.call(req);
                Box::pin(async move {
                    let res = fut.await.map_err(|e| -> BoxError { e.into() })?;
                    Ok(res.map(|b| b.map_err(|e| -> BoxError { Box::new(e) }).boxed_unsync()))
                })
            }
            AnyServiceInner::SSH(s) => {
                let fut = s.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map(|b| b.map_err(|e| -> BoxError { Box::new(e) }).boxed_unsync()))
                })
            }
        }
    }
}

impl Client<AnyService> {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        if std::env::var("SSH_AUTH_SOCK").is_ok() {
            let service = SSHTunnel::new().await?.service(());
            Ok(Client {
                c: AnyService(AnyServiceInner::SSH(service)),
            })
        } else {
            let path = agent_socket_path(AgentSocketID::Default)?.for_current();
            let c = grpc_endpoint(path).await?;
            Ok(Client {
                c: AnyService(AnyServiceInner::Socket(c)),
            })
        }
    }
}

impl<C> Client<C>
where
    C: tonic::client::GrpcService<tonic::body::Body>,
    C::Error: Into<Box<dyn Error + Send + Sync>>,
    C::ResponseBody: http_body::Body<Data = Bytes> + Send + 'static,
    <C::ResponseBody as http_body::Body>::Error: Into<Box<dyn Error + Send + Sync>> + Send,
{
    pub fn auth(self) -> AgentAuthClient<C> {
        AgentAuthClient::new(self.c)
    }

    pub fn cache(self) -> AgentCacheClient<C> {
        AgentCacheClient::new(self.c)
    }

    pub fn ctrl(self) -> AgentCtrlClient<C> {
        AgentCtrlClient::new(self.c)
    }

    pub fn ping(self) -> PingClient<C> {
        PingClient::new(self.c)
    }
}
