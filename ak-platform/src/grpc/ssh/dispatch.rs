use ssh_agent_lib::proto::Extension;
use tonic::Code;

use crate::grpc::method_caller::MethodCaller;
use crate::grpc::ssh::ext::{
    EXT_AUTHENTIK_AGENT_TUNNEL, ExtAuthentikAgentTunnelData, ExtAuthentikAgentTunnelResponse,
};

/// The agent side of the tunnel: decode a request, run it against `caller`, and encode
/// what came back — a response payload, or the gRPC status that replaced it.
///
/// This lives next to the client-side `SSHService` and the wire types so all three stay
/// in step. The agent supplies only the service registry; everything about the wire is
/// decided here.
pub async fn dispatch_tunnel_request(caller: &mut MethodCaller, raw: &[u8]) -> Extension {
    let Some(req) = ExtAuthentikAgentTunnelData::deserialize(raw) else {
        tracing::warn!("agent-tunnel: failed to decode request");
        return tunnel_response(ExtAuthentikAgentTunnelResponse::error(
            String::new(),
            Code::Internal as i32,
            "failed to decode tunnel request",
        ));
    };

    // A non-OK gRPC status comes back inside the response; `Err` means the call never
    // reached a handler at all.
    let method = req.method;
    let res = match caller.call(&method, &req.data).await {
        Ok(res) => ExtAuthentikAgentTunnelResponse {
            method,
            data: res.data,
            status: res.status,
            message: res.message,
        },
        Err(e) => {
            tracing::warn!(method = method.as_str(), "agent-tunnel: call failed: {e:?}");
            ExtAuthentikAgentTunnelResponse::error(method, Code::Internal as i32, format!("{e}"))
        }
    };

    tunnel_response(res)
}

fn tunnel_response(res: ExtAuthentikAgentTunnelResponse) -> Extension {
    Extension {
        name: EXT_AUTHENTIK_AGENT_TUNNEL.into(),
        details: res.serialize().into(),
    }
}
