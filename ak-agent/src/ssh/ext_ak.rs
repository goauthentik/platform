use std::sync::Arc;

use ak_platform::{
    generated::{
        agent_auth::agent_auth_server::AgentAuthServer,
        agent_cache::agent_cache_server::AgentCacheServer,
        agent_ctrl::agent_ctrl_server::AgentCtrlServer, ping::ping_server::PingServer,
    },
    grpc::method_caller::MethodCaller,
    net::server::creds::ProcCredentials,
};
use ssh_agent_lib::{error::AgentError, proto::Extension, ssh_encoding::Encode as SshEncode};

use crate::grpc::AgentGRPCServer;
use crate::ssh::txn::SSHAgentTransaction;

pub const EXT_AUTHENTIK_AGENT_TUNNEL: &str = "agent-tunnel@goauthentik.io";
const SSH_AGENT_EXT_RESPONSE_TYPE: u8 = 29;

struct TunnelRequest {
    method: String,
    data: Vec<u8>,
}

impl ssh_agent_lib::ssh_encoding::Decode for TunnelRequest {
    type Error = ssh_agent_lib::ssh_encoding::Error;

    fn decode(
        reader: &mut impl ssh_agent_lib::ssh_encoding::Reader,
    ) -> ssh_agent_lib::ssh_encoding::Result<Self> {
        let method = <String as ssh_agent_lib::ssh_encoding::Decode>::decode(reader)?;
        let data = <Vec<u8> as ssh_agent_lib::ssh_encoding::Decode>::decode(reader)?;
        Ok(Self { method, data })
    }
}

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
        let raw = ext.details.as_ref();
        let req =
            match <TunnelRequest as ssh_agent_lib::ssh_encoding::Decode>::decode(&mut &raw[..]) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!("agent-tunnel: failed to decode request: {e:?}");
                    return Ok(Some(Extension {
                        name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
                        details: vec![].into(),
                    }));
                }
            };

        let grpc = match AgentGRPCServer::new(Arc::clone(&self.agent)).await {
            Ok(g) => Arc::new(g),
            Err(e) => {
                tracing::warn!("agent-tunnel: failed to create gRPC server: {e:?}");
                return Ok(Some(Extension {
                    name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
                    details: vec![].into(),
                }));
            }
        };
        let mut caller = build_method_caller(grpc, self.creds.clone());

        let response_data = match caller.call(&req.method, &req.data).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::warn!(method = req.method, "agent-tunnel: call failed: {e:?}");
                return Ok(Some(Extension {
                    name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
                    details: vec![].into(),
                }));
            }
        };

        // Response wire: [u8 type=29][string method][bytes response_data]
        let mut buf: Vec<u8> = Vec::new();
        SshEncode::encode(&SSH_AGENT_EXT_RESPONSE_TYPE, &mut buf).map_err(AgentError::other)?;
        SshEncode::encode(&req.method, &mut buf).map_err(AgentError::other)?;
        SshEncode::encode(&response_data, &mut buf).map_err(AgentError::other)?;

        Ok(Some(Extension {
            name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
            details: buf.into(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ssh_agent_lib::ssh_encoding::Encode as SshEncode;

    fn encode_tunnel_request(method: &str, data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        SshEncode::encode(&method.to_string(), &mut buf).unwrap();
        SshEncode::encode(&data.to_vec(), &mut buf).unwrap();
        buf
    }

    #[test]
    fn decode_valid_tunnel_request() {
        let raw = encode_tunnel_request("/ping.Ping/Ping", b"proto");
        let req =
            <TunnelRequest as ssh_agent_lib::ssh_encoding::Decode>::decode(&mut raw.as_slice())
                .unwrap();
        assert_eq!(req.method, "/ping.Ping/Ping");
        assert_eq!(req.data, b"proto");
    }

    #[test]
    fn decode_empty_data_field() {
        let raw = encode_tunnel_request("/ping.Ping/Ping", b"");
        let req =
            <TunnelRequest as ssh_agent_lib::ssh_encoding::Decode>::decode(&mut raw.as_slice())
                .unwrap();
        assert!(req.data.is_empty());
    }

    #[test]
    fn decode_truncated_bytes_fails() {
        let result = <TunnelRequest as ssh_agent_lib::ssh_encoding::Decode>::decode(
            &mut [0u8, 0, 0, 1].as_slice(),
        );
        assert!(result.is_err());
    }
}
