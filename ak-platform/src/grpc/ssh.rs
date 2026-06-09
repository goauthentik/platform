use prost::Message;
use russh::{CryptoVec, keys::agent::client::AgentClient};
use tokio::{net::UnixStream, runtime::Handle, task::block_in_place};
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
    client: AgentClient<UnixStream>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = AgentClient::connect_env().await?;
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

        let res_raw = block_in_place(|| {
            Handle::current().block_on(async {
                let cv = CryptoVec::from(payload.serialize());
                let res = match self.client.query_extension(EXT_AUTHENTIK_AGENT_TUNNEL.as_bytes(), cv.clone()).await {
                    Ok(_) => Ok(cv),
                    Err(e) => return {
                        println!("ext err: {e:?}");
                        Err(Status::from_error(Box::from(e)))
                    },
                };
                return res;
            })
        })?;

        // let res = ExtAuthentikAgentTunnelData::deserialize(res_raw);

        println!("metadata: {:?}", request.metadata());
        println!("ext: {:?}", meth);
        println!("body: {:?}", payload.data);
        println!("res: {:?}", res_raw);

        todo!()
    }
}
