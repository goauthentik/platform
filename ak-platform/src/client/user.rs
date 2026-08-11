use eyre::Result;
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
        agent_ctrl::agent_ctrl_client::AgentCtrlClient, ping::ping_client::PingClient,
    },
    grpc::{
        GrpcError, grpc_endpoint,
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
        let socket = match &path {
            Some(p) => p.clone(),
            None => agent_socket_path(AgentSocketID::Default)?.for_current(),
        };
        match grpc_endpoint(socket).await {
            Ok(t) => Ok(Client {
                c: AnyService(AnyServiceInner::Socket(t)),
            }),
            // There is no socket at the default path, so the agent may still be
            // reachable over a forwarded SSH agent socket. A path the caller
            // asked for explicitly is taken at face value: its errors propagate.
            Err(GrpcError::SocketNotFound())
                if path.is_none() && std::env::var("SSH_AUTH_SOCK").is_ok() =>
            {
                let service = SSHTunnel::new().await?.service(());
                Ok(Client {
                    c: AnyService(AnyServiceInner::Ssh(service)),
                })
            }
            Err(e) => Err(e.into()),
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

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: &str = "/nonexistent/ak-platform-test/agent.sock";

    /// Guards the classification the SSH fallback depends on: dialing a path with
    /// no socket must come back as `SocketNotFound`, which means the connector's
    /// ENOENT really is reachable through `tonic::transport::Error`'s source
    /// chain. If this regresses, `Client::new` never sees `SocketNotFound` and the
    /// fallback goes quietly dead — which is exactly what it did while the error
    /// passed through `eyre::Report`.
    #[tokio::test]
    async fn missing_socket_is_classified_as_socket_not_found() {
        let err = grpc_endpoint(MISSING.to_string())
            .await
            .err()
            .expect("dialing a missing socket must fail");
        assert!(
            matches!(err, GrpcError::SocketNotFound()),
            "expected SocketNotFound, got {err:?}"
        );
    }

    /// A dial that fails for some *other* reason must stay a `Transport` error and
    /// keep the path in its message — the `From` impl can't supply one, so
    /// `grpc_endpoint` has to classify with `from_dial`.
    #[tokio::test]
    async fn other_dial_failures_keep_the_path() {
        // A directory is not a socket: connect(2) fails, but not with ENOENT.
        let err = grpc_endpoint("/tmp".to_string())
            .await
            .err()
            .expect("dialing a directory must fail");
        match err {
            GrpcError::Transport(_, path) => assert_eq!(path, "/tmp"),
            GrpcError::SocketNotFound() => panic!("ENOENT misreported for an existing path"),
            other => panic!("expected Transport, got {other:?}"),
        }
    }

    /// A path the caller asked for explicitly must surface its own error rather
    /// than diverting to SSH — including on a dev machine where SSH_AUTH_SOCK is
    /// set, which is why the guard checks `path.is_none()` first.
    #[tokio::test]
    async fn explicit_path_does_not_fall_back_to_ssh() {
        assert!(
            Client::<AnyService>::new(Some(MISSING.to_string()))
                .await
                .is_err()
        );
    }
}
