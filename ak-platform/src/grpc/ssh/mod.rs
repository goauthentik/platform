use crate::grpc::method_caller::grpc_frame;
use crate::grpc::method_caller::grpc_unframe;
use eyre::Result;
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
use crate::string::PlatformString;

pub mod ext;

pub struct SSHTunnel {
    client: Arc<Mutex<Client<LocalSocketStream>>>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self> {
        let sock_path =
            std::env::var("SSH_AUTH_SOCK").map_err(|_| eyre::eyre!("SSH_AUTH_SOCK is not set"))?;
        let st = match connect(PlatformString::new_with_default(&sock_path)).await {
            Ok(s) => s,
            Err(e) => return Err(e.into()),
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
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
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
                data: grpc_unframe(&body_bytes)?,
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

            // Response wire (ext_ak.rs `handle_agent_tunnel`): a single type-tag
            // byte, then the ExtAuthentikAgentTunnelData payload. The server also
            // legitimately answers with an *empty* response on its failure paths
            // (decode/gRPC-server/call failures), so check length before slicing
            // off the tag instead of unconditionally draining a fixed prefix.
            let raw_bytes = raw_res.details.into_bytes();
            let Some(inner) = raw_bytes.get(1..) else {
                return Err(Box::from("empty tunnel response"));
            };

            let res = match ExtAuthentikAgentTunnelData::deserialize(inner) {
                Some(d) => d,
                None => return Err(Box::from("failed to parse response")),
            };

            let framed = grpc_frame(&res.data);
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

    use super::SSHTunnel;
    use crate::grpc::method_caller::{grpc_frame, grpc_unframe};
    use crate::grpc::ssh::ext::{EXT_AUTHENTIK_AGENT_TUNNEL, ExtAuthentikAgentTunnelData};

    // --- Integration test: full gRPC-over-SSH-tunnel flow ---

    /// The real response-type tag `ak-agent/src/ssh/ext_ak.rs` (`handle_agent_tunnel`)
    /// prepends to every non-empty response, ahead of the ExtAuthentikAgentTunnelData
    /// payload.
    const SSH_AGENT_EXT_RESPONSE_TYPE: u8 = 29;

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

            // Mirrors the real server's wire format: a single type-tag byte
            // ahead of the payload, not a length prefix over the whole thing.
            let mut prefixed = Vec::with_capacity(1 + serialized.len());
            prefixed.push(SSH_AGENT_EXT_RESPONSE_TYPE);
            prefixed.extend_from_slice(&serialized);

            Ok(Some(Extension {
                name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                details: Unparsed::from(prefixed),
            }))
        }
    }

    /// Mirrors `ext_ak.rs`'s `handle_agent_tunnel` on any of its failure paths
    /// (request-decode failure, gRPC-server-creation failure, method-call
    /// failure): it always answers with an empty `details`, not a shorter
    /// valid payload.
    #[derive(Clone, Default)]
    struct EmptyResponseAgent;

    #[ssh_agent_lib::async_trait]
    impl Session for EmptyResponseAgent {
        async fn extension(&mut self, _ext: Extension) -> Result<Option<Extension>, AgentError> {
            Ok(Some(Extension {
                name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                details: Unparsed::from(Vec::new()),
            }))
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ssh_service_routes_request_through_tunnel()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use interprocess::local_socket::{
            GenericFilePath,
            tokio::{Stream as LocalSocketStream, prelude::*},
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
            .body(Full::new(Bytes::from(grpc_frame(proto_payload))))?;

        let resp = svc.call(req).await?;

        assert_eq!(resp.status(), 200);
        assert_eq!(
            resp.headers()
                .get("grpc-status")
                .and_then(|v| v.to_str().ok()),
            Some("0")
        );

        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let stripped = grpc_unframe(&body_bytes)?;
        assert_eq!(stripped, proto_payload);

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }

    /// Regression test: an empty tunnel response (what the real server sends
    /// on every one of its failure paths) used to panic with "range end index
    /// 4 out of range for slice of length 0" from an unconditional
    /// `Vec::drain(0..4)`. It must surface as an `Err` instead.
    #[cfg(unix)]
    #[tokio::test]
    async fn ssh_service_errors_instead_of_panicking_on_empty_response()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use interprocess::local_socket::{
            GenericFilePath,
            tokio::{Stream as LocalSocketStream, prelude::*},
        };
        use ssh_agent_lib::client::Client;
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-empty-response.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, EmptyResponseAgent).await });

        let name = sock_path.to_fs_name::<GenericFilePath>()?;
        let stream = LocalSocketStream::connect(name).await?;
        let client = Client::new(stream);

        let tunnel = SSHTunnel {
            client: Arc::new(Mutex::new(client)),
        };
        let mut svc = tunnel.service(());

        let req = Request::builder()
            .method("POST")
            .uri("/some.Service/Method")
            .header("content-type", "application/grpc+proto")
            .body(Full::new(Bytes::from(grpc_frame(&[0x01]))))?;

        assert!(svc.call(req).await.is_err());

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }
}
