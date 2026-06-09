use prost::Message;
use service_binding::Binding;
use ssh_agent_lib::{
    agent::Session,
    client::connect,
    proto::{Extension, Unparsed},
};
use tokio::{runtime::Handle, task::block_in_place};
use tonic::{GrpcMethod, Status, service::Interceptor};

pub const EXT_AUTHENTIK_AGENT_TUNNEL: &str = "agent-tunnel@goauthentik.io";

/// Payload sent to the SSH agent via the tunnel extension.
/// `method` is the gRPC path (e.g. `/package.Service/Method`).
/// `data` is the raw serialized proto request (no gRPC framing).
pub struct ExtAuthentikAgentTunnelData {
    pub method: String,
    pub data: Vec<u8>,
}

impl ExtAuthentikAgentTunnelData {
    fn serialize(&self) -> Vec<u8> {
        let method = self.method.as_bytes();
        let mut buf = Vec::with_capacity(8 + method.len() + self.data.len());
        buf.extend_from_slice(&(method.len() as u32).to_be_bytes());
        buf.extend_from_slice(method);
        buf.extend_from_slice(&(self.data.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.data);
        buf
    }

    fn deserialize(buf: &[u8]) -> Option<Self> {
        if buf.len() < 4 {
            return None;
        }
        let method_len = u32::from_be_bytes(buf[0..4].try_into().ok()?) as usize;

        let method_start = 4;
        let method_end = method_start + method_len;
        if buf.len() < method_end + 4 {
            return None;
        }
        let method = String::from_utf8(buf[method_start..method_end].to_vec()).ok()?;

        let data_len =
            u32::from_be_bytes(buf[method_end..method_end + 4].try_into().ok()?) as usize;

        let data_start = method_end + 4;
        let data_end = data_start + data_len;
        if buf.len() < data_end {
            return None;
        }
        let data = buf[data_start..data_end].to_vec();

        Some(Self { method, data })
    }
}

pub struct SSHTunnel {
    client: Box<dyn Session>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client =
            connect(Binding::FilePath(std::env::var("SSH_AUTH_SOCK")?.into()).try_into()?)?;
        Ok(SSHTunnel { client })
    }
}

impl Interceptor for SSHTunnel {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let body: &() = request.get_ref();

        let meth = match request.extensions().get::<GrpcMethod>() {
            Some(m) => m,
            None => {
                return Err(tonic::Status::from_error(Box::from(
                    "Failed to get GRPC method",
                )));
            }
        };

        let payload = ExtAuthentikAgentTunnelData {
            method: format!("{}/{}", meth.service(), meth.method()),
            data: body.encode_to_vec(),
        };
        println!("metadata: {:?}", request.metadata());
        println!("ext: {:?}", meth);
        println!("body: {:?}", payload.data);
        println!("sz: {:?}", payload.serialize());

        let res_raw = block_in_place(|| {
            Handle::current().block_on(async {
                match self
                    .client
                    .extension(Extension {
                        name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                        details: Unparsed::from(payload.serialize()),
                    })
                    .await
                {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        return {
                            println!("ext err: {e:?}");
                            Err(Status::from_error(Box::from(e)))
                        };
                    }
                }
            })
        })?;

        if let Some(res) = res_raw {
            assert_eq!(res.name, EXT_AUTHENTIK_AGENT_TUNNEL);

            let res = ExtAuthentikAgentTunnelData::deserialize(&res.details.into_bytes());
            if let Some(p_res) = res {
                println!("res: {:?}", p_res.data);
            }
        } else {
            return Err(Status::from_error(Box::from("No response")));
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::generated::{agent::RequestHeader, agent_auth::WhoAmIRequest};

    use super::*;

    #[test]
    fn serialize() {
        let msg = WhoAmIRequest {
            header: Some(RequestHeader {
                profile: "default".to_string(),
            }),
        };
        let encoded = msg.encode_to_vec();
        let ext = ExtAuthentikAgentTunnelData {
            method: "ping.Ping/Ping".to_string(),
            data: encoded,
        };
        assert_eq!(
            ext.serialize(),
            [
                0, 0, 0, 14, 112, 105, 110, 103, 46, 80, 105, 110, 103, 47, 80, 105, 110, 103, 0,
                0, 0, 11, 10, 9, 10, 7, 100, 101, 102, 97, 117, 108, 116
            ]
        );
    }

    #[test]
    fn deserialize() {
        let encoded: Vec<u8> = vec![
0, 0, 0, 27, 97, 103, 101, 110, 116, 95, 97, 117, 116, 104, 46, 65, 103, 101, 110, 116, 65, 117, 116, 104, 47, 87, 104, 111, 65, 109, 73, 0, 0, 0, 0        ];
        let parsed = ExtAuthentikAgentTunnelData::deserialize(&encoded).unwrap();
        assert_eq!(parsed.method, "agent_auth.AgentAuth/WhoAmI");

        let m = WhoAmIRequest::decode(&*parsed.data).unwrap();

        assert_eq!(m.header.unwrap().profile, "default");
    }
}
