use std::{future::Future, pin::Pin, sync::Arc, task::Poll};

use bytes::Bytes;
use http::request::Request;
use http::response::Response;
use http_body_util::BodyExt;
use http_body_util::Full;
use ssh_agent_lib::{
    agent::Session,
    client::Client,
    proto::{Extension, Unparsed},
};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tower::{Layer, Service};
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
    client: Arc<Mutex<Client<UnixStream>>>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sock_path = std::env::var("SSH_AUTH_SOCK")
            .map_err(|_| "SSH_AUTH_SOCK is not set")?;
        let stream = UnixStream::connect(sock_path).await?;
        let client = Client::new(stream);
        Ok(SSHTunnel {
            client: Arc::new(Mutex::new(client)),
        })
    }
}

impl Clone for SSHTunnel {
    fn clone(&self) -> Self {
        SSHTunnel {
            client: Arc::clone(&self.client),
        }
    }
}

pub struct SSHLayer {
    tunnel: SSHTunnel,
}

impl SSHLayer {
    pub fn new(tunnel: SSHTunnel) -> Self {
        Self { tunnel }
    }

    pub fn service<S>(&self, _inner: S) -> SSHService {
        SSHService {
            tunnel: self.tunnel.clone(),
        }
    }
}

impl<S> Layer<S> for SSHLayer {
    type Service = SSHService;

    fn layer(&self, _inner: S) -> Self::Service {
        SSHService {
            tunnel: self.tunnel.clone(),
        }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct SSHService {
    tunnel: SSHTunnel,
}

impl<B> Service<Request<B>> for SSHService
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    type Response = Response<tonic::body::Body>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let tunnel = self.tunnel.clone();

        Box::pin(async move {
            let method = req.uri().path().to_string();
            let body_bytes = req
                .into_body()
                .collect()
                .await
                .map_err(Into::into)?
                .to_bytes();

            let payload = ExtAuthentikAgentTunnelData {
                method: method.trim_start_matches("/").to_string(),
                data: strip_grpc_frame(&body_bytes)?.into(),
            };
            println!("meth: {:?}", payload.method);
            println!("data: {:?}", payload.data);

            let raw_res = match tunnel
                .client
                .lock()
                .await
                .extension(Extension {
                    name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                    details: Unparsed::from(payload.serialize()),
                })
                .await
            {
                Ok(res) => match res {
                    Some(rres) => rres,
                    None => return Err(Box::from("No response")),
                },
                Err(e) => {
                    eprintln!("failed to send ext: {e:?}");
                    return Err(Box::from(e));
                }
            };

            println!("{:?}", raw_res);
            let res = match ExtAuthentikAgentTunnelData::deserialize(&raw_res.details.into_bytes())
            {
                Some(d) => d,
                None => return Err(Box::from("failed to parse response")),
            };

            let framed = add_grpc_frame(&res.data);
            let body = tonic::body::Body::new(Full::new(Bytes::from(framed)));
            let response = Response::builder()
                .status(200)
                .header("content-type", "application/grpc+proto")
                .header("grpc-status", "0")
                .body(body)
                .map_err(|e| -> BoxError { Box::new(e) })?;

            Ok(response)
        })
    }
}

/// Strip the 5-byte gRPC framing from a request body to get the raw proto bytes.
fn strip_grpc_frame(data: &[u8]) -> Result<&[u8], BoxError> {
    if data.len() < 5 {
        return Err("gRPC frame too short".into());
    }
    // data[0] = compression flag; data[1..5] = message length
    let msg_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    data.get(5..5 + msg_len)
        .ok_or_else(|| "gRPC frame length exceeds buffer".into())
}

/// Wrap raw proto bytes in a gRPC frame (5-byte uncompressed header).
fn add_grpc_frame(proto: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + proto.len());
    buf.push(0u8); // no compression
    buf.extend_from_slice(&(proto.len() as u32).to_be_bytes());
    buf.extend_from_slice(proto);
    buf
}

#[cfg(test)]
mod tests {
    use crate::generated::{agent::RequestHeader, agent_auth::WhoAmIRequest};
    use prost::Message;
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
            0, 0, 0, 27, 97, 103, 101, 110, 116, 95, 97, 117, 116, 104, 46, 65, 103, 101, 110, 116,
            65, 117, 116, 104, 47, 87, 104, 111, 65, 109, 73, 0, 0, 0, 0,
        ];
        let parsed = ExtAuthentikAgentTunnelData::deserialize(&encoded).unwrap();
        assert_eq!(parsed.method, "agent_auth.AgentAuth/WhoAmI");

        let m = WhoAmIRequest::decode(&*parsed.data).unwrap();

        assert_eq!(m.header.unwrap().profile, "default");
    }
}
