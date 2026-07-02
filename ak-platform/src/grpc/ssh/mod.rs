use crate::grpc::method_caller::grpc_frame;
use crate::grpc::method_caller::grpc_unframe;
use crate::prelude::*;
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
        let sock_path = std::env::var("SSH_AUTH_SOCK")
            .map_err(|_| eyre::eyre!("SSH_AUTH_SOCK is not set"))?;
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

            // Required as the inner message is also SSH encoded as a whole
            let mut raw_bytes = raw_res.details.into_bytes();
            raw_bytes.drain(0..4);

            let res = match ExtAuthentikAgentTunnelData::deserialize(&raw_bytes) {
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
}
