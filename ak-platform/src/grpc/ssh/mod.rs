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

use interprocess::local_socket::tokio::Stream as LocalSocketStream;

use crate::grpc::ssh::ext::EXT_AUTHENTIK_AGENT_TUNNEL;
use crate::grpc::ssh::ext::ExtAuthentikAgentTunnelData;
use crate::net::client::connect;
use crate::platform::string::PlatformString;

pub mod ext;

pub struct SSHTunnel {
    client: Arc<Mutex<Client<LocalSocketStream>>>,
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bytes::Bytes;
    use http::Request;
    use http_body_util::{BodyExt, Full};
    use ssh_agent_lib::{
        agent::{Session, listen as ssh_listen},
        error::AgentError,
        proto::{Extension, Unparsed},
    };
    use tokio::sync::Mutex;
    use tower::Service;

    use super::{SSHTunnel, add_grpc_frame, strip_grpc_frame};
    use crate::grpc::ssh::ext::{EXT_AUTHENTIK_AGENT_TUNNEL, ExtAuthentikAgentTunnelData};

    // --- strip_grpc_frame ---

    #[test]
    fn strip_grpc_frame_valid() {
        let data = [0x00u8, 0x00, 0x00, 0x00, 0x05, 1, 2, 3, 4, 5];
        let result = strip_grpc_frame(&data).unwrap();
        assert_eq!(result, &[1u8, 2, 3, 4, 5]);
    }

    #[test]
    fn strip_grpc_frame_empty_payload() {
        let data = [0x00u8, 0x00, 0x00, 0x00, 0x00];
        let result = strip_grpc_frame(&data).unwrap();
        assert_eq!(result, &[] as &[u8]);
    }

    #[test]
    fn strip_grpc_frame_too_short() {
        let err = strip_grpc_frame(&[0x00u8, 0x01, 0x02]).unwrap_err();
        assert!(err.to_string().contains("too short"), "error was: {err}");
    }

    #[test]
    fn strip_grpc_frame_length_exceeds_buffer() {
        // Header claims 100 bytes, only 3 bytes of payload follow.
        let mut data = vec![0x00u8, 0x00, 0x00, 0x00, 100];
        data.extend_from_slice(&[1u8, 2, 3]);
        let err = strip_grpc_frame(&data).unwrap_err();
        assert!(
            err.to_string().contains("exceeds buffer"),
            "error was: {err}"
        );
    }

    // --- add_grpc_frame ---

    #[test]
    fn add_grpc_frame_header_bytes() {
        assert_eq!(
            add_grpc_frame(b"hello"),
            [0x00, 0x00, 0x00, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn add_grpc_frame_empty() {
        assert_eq!(add_grpc_frame(b""), [0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn roundtrip_add_then_strip() {
        let original = b"some proto bytes";
        let framed = add_grpc_frame(original);
        let stripped = strip_grpc_frame(&framed).unwrap();
        assert_eq!(stripped, original);
    }

    // --- Integration test: full gRPC-over-SSH-tunnel flow ---

    #[derive(Clone, Default)]
    struct MockTunnelAgent;

    #[ssh_agent_lib::async_trait]
    impl Session for MockTunnelAgent {
        async fn extension(&mut self, ext: Extension) -> Result<Option<Extension>, AgentError> {
            let req = ExtAuthentikAgentTunnelData::deserialize(&ext.details.into_bytes())
                .ok_or(AgentError::Failure)?;

            let serialized = ExtAuthentikAgentTunnelData {
                method: req.method,
                data: req.data,
            }
            .serialize();

            // mod.rs drains the first 4 bytes of the response before deserializing,
            // matching the SSH encoding convention where extension bodies carry a
            // u32 length prefix.
            let mut prefixed = Vec::with_capacity(4 + serialized.len());
            prefixed.extend_from_slice(&(serialized.len() as u32).to_be_bytes());
            prefixed.extend_from_slice(&serialized);

            Ok(Some(Extension {
                name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                details: Unparsed::from(prefixed),
            }))
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ssh_service_routes_request_through_tunnel()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use interprocess::local_socket::{
            tokio::{prelude::*, Stream as LocalSocketStream},
            GenericFilePath,
        };
        use ssh_agent_lib::client::Client;
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-integration.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, MockTunnelAgent).await });

        let name = sock_path.to_fs_name::<GenericFilePath>()?;
        let stream = LocalSocketStream::connect(name).await?;
        let client = Client::new(stream);

        let tunnel = SSHTunnel {
            client: Arc::new(Mutex::new(client)),
        };
        let mut svc = tunnel.service(());

        let proto_payload: &[u8] = &[0x01, 0x02, 0x03];
        let req = Request::builder()
            .method("POST")
            .uri("/some.Service/Method")
            .header("content-type", "application/grpc+proto")
            .body(Full::new(Bytes::from(add_grpc_frame(proto_payload))))?;

        let resp = svc.call(req).await?;

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("grpc-status")
                .and_then(|v| v.to_str().ok()),
            Some("0")
        );

        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let stripped = strip_grpc_frame(&body_bytes)?;
        assert_eq!(stripped, proto_payload);

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }
}
