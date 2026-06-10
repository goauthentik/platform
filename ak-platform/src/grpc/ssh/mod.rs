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
use tokio::sync::Mutex;
use tower::{Layer, Service};

use crate::grpc::ssh::ext::EXT_AUTHENTIK_AGENT_TUNNEL;
use crate::grpc::ssh::ext::ExtAuthentikAgentTunnelData;
use crate::net::client::StreamType;
use crate::net::client::connect;
use crate::platform::string::PlatformString;

pub mod ext;

pub struct SSHTunnel {
    client: Arc<Mutex<Client<StreamType>>>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sock_path = std::env::var("SSH_AUTH_SOCK").map_err(|_| "SSH_AUTH_SOCK is not set")?;
        let st = match connect(PlatformString::new_with_default(&sock_path)).await {
            Ok(s) => s,
            Err(e) => return Err(e),
        };
        let client = Client::new(st.into_inner());
        Ok(SSHTunnel {
            client: Arc::new(Mutex::new(client)),
        })
    }

    pub fn service<S>(&self, _inner: S) -> SSHService {
        SSHService {
            layer: self.clone(),
        }
    }
}

impl Clone for SSHTunnel {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

impl<S> Layer<S> for SSHTunnel {
    type Service = SSHService;

    fn layer(&self, _inner: S) -> Self::Service {
        SSHService {
            layer: self.clone(),
        }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct SSHService {
    layer: SSHTunnel,
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
        let tunnel = self.layer.clone();

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
                    return Err(Box::from(e));
                }
            };

            // Required as the inner message is also SSH encoded as a whole
            let mut raw_bytes = raw_res.details.into_bytes();
            raw_bytes.drain(0..4);

            let res = match ExtAuthentikAgentTunnelData::deserialize(&raw_bytes) {
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
