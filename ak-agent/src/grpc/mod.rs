use ak_platform::generated::agent::RequestHeader;
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use std::sync::Arc;
use std::time::Duration;
use tonic::Status;

use crate::Agent;
use crate::config::ConfigV1Profile;
use ak_platform::generated::agent_cache::agent_cache_server::AgentCacheServer;
use ak_platform::generated::agent_ctrl::agent_ctrl_server::AgentCtrlServer;
use ak_platform::generated::ping::ping_server::PingServer;
use ak_platform::prelude::*;
use ak_platform::{
    generated::agent_auth::agent_auth_server::AgentAuthServer,
    net::server::{SocketPermMode, listen},
    paths::{AgentSocketID, agent_socket_path},
};
use tonic::codegen::http;
use tonic::transport::Server;
use tracing::Span;

pub mod agent_auth;
pub mod agent_cache;
pub mod agent_ctrl;
pub mod ping;

pub struct AgentGRPCServer {
    agent: Arc<Agent>,
}

impl AgentGRPCServer {
    pub async fn new(agent: Arc<Agent>) -> Result<AgentGRPCServer> {
        Ok(AgentGRPCServer { agent })
    }

    pub async fn start(self) -> Result<()> {
        let listener = match listen(
            agent_socket_path(AgentSocketID::Default)?,
            SocketPermMode::Owner,
        )
        .await
        {
            Ok(l) => l,
            Err(e) => {
                log::warn!("failed to listen: {e:?}");
                return Err(e);
            }
        };
        let shared = Arc::new(self);
        Ok(Server::builder()
            .layer(NewSentryLayer::new_from_top())
            .layer(SentryHttpLayer::new().enable_transaction())
            .layer(
                tower_http::trace::TraceLayer::new_for_grpc()
                    .on_request(|request: &http::Request<tonic::body::Body>, _span: &Span| {
                        log::info!("started call: {}", request.uri().path());
                    })
                    .on_response(
                        |_response: &http::Response<tonic::body::Body>,
                         latency: Duration,
                         _span: &Span| {
                            log::info!("finished call, took {:?}", latency);
                        },
                    ),
            )
            .add_service(AgentAuthServer::from_arc(Arc::clone(&shared)))
            .add_service(AgentCacheServer::from_arc(Arc::clone(&shared)))
            .add_service(AgentCtrlServer::from_arc(Arc::clone(&shared)))
            .add_service(PingServer::from_arc(Arc::clone(&shared)))
            .serve_with_incoming(listener)
            .await?)
    }

    pub async fn profile_for_request(
        &self,
        header: Option<RequestHeader>,
    ) -> std::result::Result<ConfigV1Profile, Status> {
        let read = self.agent.cfg.read().await;
        let h = match header {
            Some(h) => h,
            None => return Err(Status::invalid_argument("no request header")),
        };
        let profile = match read.profiles.get(&h.profile) {
            Some(p) => p.clone(),
            None => return Err(Status::not_found("profile not found")),
        };
        Ok(profile)
    }
}
