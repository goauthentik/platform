use eyre::Result;
use std::error::Error;
use std::future::Future;
use std::io::ErrorKind;
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
        agent_ctrl::agent_ctrl_client::AgentCtrlClient, ping::ping_client::PingClient,
    },
    grpc::{
        grpc_endpoint,
        ssh::{SSHService, SSHTunnel},
    },
    paths::{AgentSocketID, agent_socket_path},
};

pub struct Client<C> {
    c: C,
}

impl Client<Channel> {
    pub async fn new_with_id(id: AgentSocketID) -> Result<Self> {
        Self::new_with_path(agent_socket_path(id)?.for_current()).await
    }

    pub async fn new_with_path(p: String) -> Result<Self> {
        let c = grpc_endpoint(p).await?;
        Ok(Client { c })
    }
}

impl Client<SSHService> {
    pub async fn new_with_ssh() -> Result<Self> {
        let service = SSHTunnel::new().await?.service(());
        Ok(Client { c: service })
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

type AnyBody = UnsyncBoxBody<Bytes, BoxError>;

#[derive(Clone)]
enum AnyServiceInner {
    Socket(Channel),
    Ssh(SSHService),
}

pub struct AnyService(AnyServiceInner);

impl Service<http::Request<tonic::body::Body>> for AnyService {
    type Response = http::Response<AnyBody>;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Self::Error>> {
        match &mut self.0 {
            AnyServiceInner::Socket(c) => c.poll_ready(cx).map_err(Into::into),
            AnyServiceInner::Ssh(s) => {
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
            AnyServiceInner::Ssh(s) => {
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
    pub async fn new(path: Option<String>) -> Result<Self> {
        let mut _path: String;
        if let Some(_p) = path.clone() {
            _path = _p;
        } else {
            _path = agent_socket_path(AgentSocketID::Default)?.for_current();
        }
        match grpc_endpoint(_path).await {
            Ok(t) => Ok(Client {
                c: AnyService(AnyServiceInner::Socket(t)),
            }),
            Err(e) => {
                // If we can't open the socket due to an IO Error of file not found,
                // and we're trying to use the default socket path, attempt SSH connection
                // If the user specified a path, then we return the error
                if let Some(io_err) = e.downcast_ref::<std::io::Error>()
                    && io_err.kind() == ErrorKind::NotFound
                    && let None = path
                    && std::env::var("SSH_AUTH_SOCK").is_ok()
                {
                    let service = SSHTunnel::new().await?.service(());
                    return Ok(Client {
                        c: AnyService(AnyServiceInner::Ssh(service)),
                    });
                }
                Err(e)
            }
        }
    }

    pub fn new_channel(c: Channel) -> Self {
        Client {
            c: AnyService(AnyServiceInner::Socket(c)),
        }
    }
}

impl Clone for AnyService {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Clone for Client<AnyService> {
    fn clone(&self) -> Self {
        Self { c: self.c.clone() }
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
