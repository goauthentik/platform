use std::sync::Arc;

use ak_platform::{
    generated::{
        agent_auth::agent_auth_server::AgentAuthServer,
        agent_cache::agent_cache_server::AgentCacheServer,
        agent_ctrl::agent_ctrl_server::AgentCtrlServer, ping::ping_server::PingServer,
    },
    grpc::method_caller::MethodCaller,
    grpc::ssh::dispatch::dispatch_tunnel_request,
    grpc::ssh::ext::ExtAuthentikAgentTunnelResponse,
    net::server::creds::ProcCredentials,
};
use ssh_agent_lib::{error::AgentError, proto::Extension};
use tonic::Code;

use crate::grpc::AgentGRPCServer;
use crate::ssh::txn::SSHAgentTransaction;

/// Re-exported so both ends of the tunnel name the extension from the same constant.
pub use ak_platform::grpc::ssh::ext::EXT_AUTHENTIK_AGENT_TUNNEL;

pub fn build_method_caller(grpc: Arc<AgentGRPCServer>, creds: ProcCredentials) -> MethodCaller {
    let mut caller = MethodCaller::new(creds);
    caller.add_service(AgentAuthServer::from_arc(Arc::clone(&grpc)));
    caller.add_service(AgentCacheServer::from_arc(Arc::clone(&grpc)));
    caller.add_service(AgentCtrlServer::from_arc(Arc::clone(&grpc)));
    caller.add_service(PingServer::from_arc(Arc::clone(&grpc)));
    caller
}

impl SSHAgentTransaction {
    pub(crate) async fn handle_agent_tunnel(
        &self,
        ext: &Extension,
    ) -> std::result::Result<Option<Extension>, AgentError> {
        let grpc = match AgentGRPCServer::new(Arc::clone(&self.agent)).await {
            Ok(g) => Arc::new(g),
            Err(e) => {
                tracing::warn!("agent-tunnel: failed to create gRPC server: {e:?}");
                let res = ExtAuthentikAgentTunnelResponse::error(
                    String::new(),
                    Code::Internal as i32,
                    "failed to create gRPC server",
                );
                return Ok(Some(Extension {
                    name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
                    details: res.serialize().into(),
                }));
            }
        };
        let mut caller = build_method_caller(grpc, self.creds.clone());

        Ok(Some(
            dispatch_tunnel_request(&mut caller, ext.details.as_ref()).await,
        ))
    }
}
