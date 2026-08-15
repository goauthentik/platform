//! A minimal stand-in for `ak-sysd`'s gRPC endpoints, listening on the same
//! named pipe the real daemon uses (`ak-platform`'s `Config` falls back to
//! that fixed path whenever the production config file isn't present, which
//! is always true on a test machine). Lets `e2e` tests drive the real
//! `credprovider`/`cef-host` binaries against a fully deterministic backend
//! with no Docker or real authentik server involved.

use ak_platform::generated::ping::{
    CapabilitiesResponse, PingResponse,
    capabilities_response::Capability,
    ping_server::{Ping, PingServer},
};
use ak_platform::generated::sys_auth::{
    InteractiveAuthAsyncResponse, InteractiveAuthRequest, InteractiveChallenge, SshCertAuthRequest,
    SshCertAuthResponse, TokenAuthRequest, TokenAuthResponse,
    system_auth_interactive_server::{SystemAuthInteractive, SystemAuthInteractiveServer},
    system_auth_token_server::{SystemAuthToken, SystemAuthTokenServer},
};
use ak_platform::net::server::{SocketPermMode, listen};
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use tonic::{Request, Response, Status};

/// What the mock hands back for a completed sign-in: the URL `cef-host`
/// opens, and the token embedded in its `goauthentik.io://` redirect that
/// `token_auth` is expected to validate.
#[derive(Clone)]
pub struct MockConfig {
    pub interactive_auth_url: String,
    pub header_token: String,
    pub valid_token: String,
    pub username: String,
}

struct MockPing;

#[tonic::async_trait]
impl Ping for MockPing {
    async fn ping(&self, _request: Request<()>) -> Result<Response<PingResponse>, Status> {
        Ok(Response::new(PingResponse {
            component: "mock-sysd".to_string(),
            version: "0.0.0".to_string(),
            server_version: "0.0.0".to_string(),
        }))
    }

    async fn capabilities(
        &self,
        _request: Request<()>,
    ) -> Result<Response<CapabilitiesResponse>, Status> {
        Ok(Response::new(CapabilitiesResponse {
            capabilities: vec![Capability::AuthInteractive as i32, Capability::Debug as i32],
        }))
    }
}

struct MockSystemAuthInteractive {
    config: MockConfig,
}

#[tonic::async_trait]
impl SystemAuthInteractive for MockSystemAuthInteractive {
    async fn interactive_auth(
        &self,
        _request: Request<InteractiveAuthRequest>,
    ) -> Result<Response<InteractiveChallenge>, Status> {
        Err(Status::unimplemented("not used by credprovider/cef-host"))
    }

    async fn interactive_auth_async(
        &self,
        _request: Request<()>,
    ) -> Result<Response<InteractiveAuthAsyncResponse>, Status> {
        Ok(Response::new(InteractiveAuthAsyncResponse {
            url: self.config.interactive_auth_url.clone(),
            header_token: self.config.header_token.clone(),
        }))
    }
}

struct MockSystemAuthToken {
    config: MockConfig,
}

#[tonic::async_trait]
impl SystemAuthToken for MockSystemAuthToken {
    async fn token_auth(
        &self,
        request: Request<TokenAuthRequest>,
    ) -> Result<Response<TokenAuthResponse>, Status> {
        let successful = request.into_inner().token == self.config.valid_token;
        Ok(Response::new(TokenAuthResponse {
            successful,
            token: successful.then(|| ak_platform::generated::agent::Token {
                preferred_username: self.config.username.clone(),
                ..Default::default()
            }),
            session_id: "mock-session".to_string(),
        }))
    }

    async fn ssh_cert_auth(
        &self,
        _request: Request<SshCertAuthRequest>,
    ) -> Result<Response<SshCertAuthResponse>, Status> {
        Err(Status::unimplemented("not used by credprovider/cef-host"))
    }
}

/// Starts the mock server on the real `ak-sysd` named pipe path and returns
/// a handle whose `Drop` stops it — callers keep it alive for the duration
/// of a test.
pub struct MockSysd {
    task: tokio::task::JoinHandle<()>,
}

impl Drop for MockSysd {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn start(config: MockConfig) -> eyre::Result<MockSysd> {
    let listener = listen(
        sysd_socket_path(SysdSocketID::Default),
        SocketPermMode::Everyone,
    )
    .await?;

    let ping = PingServer::new(MockPing);
    let interactive = SystemAuthInteractiveServer::new(MockSystemAuthInteractive {
        config: config.clone(),
    });
    let token = SystemAuthTokenServer::new(MockSystemAuthToken { config });

    let task = tokio::spawn(async move {
        let result = tonic::transport::Server::builder()
            .add_service(ping)
            .add_service(interactive)
            .add_service(token)
            .serve_with_incoming(listener)
            .await;
        if let Err(e) = result {
            log::error!("mock ak-sysd server exited: {e}");
        }
    });

    Ok(MockSysd { task })
}
