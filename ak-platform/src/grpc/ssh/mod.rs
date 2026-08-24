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
use tonic::{Code, Status};
use tower::{Layer, Service};

use interprocess::local_socket::tokio::Stream as LocalSocketStream;

use crate::grpc::ssh::ext::EXT_AUTHENTIK_AGENT_TUNNEL;
use crate::grpc::ssh::ext::ExtAuthentikAgentTunnelData;
use crate::grpc::ssh::ext::ExtAuthentikAgentTunnelResponse;
use crate::net::client::connect;
use crate::string::PlatformString;

pub mod dispatch;
pub mod ext;

pub struct SSHTunnel {
    client: Arc<Mutex<Client<LocalSocketStream>>>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self> {
        let sock_path =
            std::env::var("SSH_AUTH_SOCK").map_err(|_| eyre::eyre!("SSH_AUTH_SOCK is not set"))?;
        Self::connect_to(&sock_path).await
    }

    /// Connect to an SSH agent socket by path, rather than through `SSH_AUTH_SOCK`.
    pub async fn connect_to(sock_path: &str) -> Result<Self> {
        let st = match connect(PlatformString::new_with_default(sock_path)).await {
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

            // The full path, slash included: the agent feeds it to `MethodCaller`, which
            // puts it in a request URI, and a relative path is not a valid URI.
            let payload = ExtAuthentikAgentTunnelData {
                method,
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

            // An empty payload means the agent could not say anything at all — either it
            // predates the status fields and hit one of its old bail-out paths, or the
            // wire is out of sync. Everything else, failures included, parses.
            let raw_bytes = raw_res.details.into_bytes();
            if raw_bytes.is_empty() {
                return Err(Box::from("empty tunnel response"));
            }

            let res = match ExtAuthentikAgentTunnelResponse::deserialize(&raw_bytes) {
                Some(d) => d,
                None => return Err(Box::from("failed to parse response")),
            };

            let status = Status::new(Code::from_i32(res.status), res.message);
            // A non-OK status carries no message frame; sending an empty body keeps this
            // a trailers-only response, which is what tonic expects for an error.
            let body = if status.code() == Code::Ok {
                Full::new(Bytes::from(grpc_frame(&res.data)))
            } else {
                Full::new(Bytes::new())
            };

            let mut response = Response::builder()
                .status(200)
                .header("content-type", "application/grpc+proto")
                .body(tonic::body::Body::new(body))
                .map_err(|e| -> BoxError { Box::new(e) })?;
            status
                .add_header(response.headers_mut())
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
    use tonic::{Code, Status};
    use tower::Service;

    use super::SSHTunnel;
    use crate::generated::ping::{
        CapabilitiesResponse, PingResponse,
        ping_server::{Ping, PingServer},
    };
    use crate::grpc::method_caller::{MethodCaller, grpc_frame, grpc_unframe};
    use crate::grpc::ssh::dispatch::dispatch_tunnel_request;
    use crate::grpc::ssh::ext::{
        EXT_AUTHENTIK_AGENT_TUNNEL, ExtAuthentikAgentTunnelData, ExtAuthentikAgentTunnelResponse,
    };
    use crate::net::server::creds::ProcCredentials;

    // --- Integration test: full gRPC-over-SSH-tunnel flow ---

    /// Echoes the request back and records the method it was asked for, so a test can
    /// assert what actually went over the wire. `ak-agent` feeds that method straight
    /// into an HTTP request URI, so anything but a full `/pkg.Service/Method` path fails
    /// there — which is invisible from this side unless the mock checks.
    #[derive(Clone, Default)]
    struct MockTunnelAgent {
        seen: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[ssh_agent_lib::async_trait]
    impl Session for MockTunnelAgent {
        async fn extension(&mut self, ext: Extension) -> Result<Option<Extension>, AgentError> {
            let req = ExtAuthentikAgentTunnelData::deserialize(&ext.details.into_bytes())
                .ok_or(AgentError::Failure)?;

            self.seen.lock().unwrap().push(req.method.clone());

            Ok(Some(Extension {
                name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                details: Unparsed::from(
                    ExtAuthentikAgentTunnelResponse::ok(req.method, req.data).serialize(),
                ),
            }))
        }
    }

    /// Answers every request with a non-OK gRPC status and no payload, the way
    /// `ext_ak.rs` reports a handler that returned a `Status`.
    #[derive(Clone, Default)]
    struct FailingTunnelAgent;

    #[ssh_agent_lib::async_trait]
    impl Session for FailingTunnelAgent {
        async fn extension(&mut self, ext: Extension) -> Result<Option<Extension>, AgentError> {
            let req = ExtAuthentikAgentTunnelData::deserialize(&ext.details.into_bytes())
                .ok_or(AgentError::Failure)?;

            Ok(Some(Extension {
                name: EXT_AUTHENTIK_AGENT_TUNNEL.to_string(),
                details: Unparsed::from(
                    ExtAuthentikAgentTunnelResponse::error(
                        req.method,
                        Code::NotFound as i32,
                        "no such user",
                    )
                    .serialize(),
                ),
            }))
        }
    }

    /// What every failure path in `ext_ak.rs` used to answer with, and what an agent
    /// older than the status fields still answers with when it bails out. There is
    /// nothing to parse, so this has to be an error — and, once upon a time, a panic.
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
        let agent = MockTunnelAgent::default();
        let seen = Arc::clone(&agent.seen);
        let server_handle = tokio::spawn(async move { ssh_listen(listener, agent).await });

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

        // Regression: the method used to go out with its leading slash stripped, which
        // the agent could not turn back into a request URI.
        assert_eq!(seen.lock().unwrap().as_slice(), ["/some.Service/Method"]);

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }

    /// A gRPC status from the far end has to arrive as that status. Before the tunnel
    /// carried one, every failure came back as an empty payload and the caller could
    /// only report "empty tunnel response".
    #[cfg(unix)]
    #[tokio::test]
    async fn ssh_service_surfaces_non_ok_status()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use interprocess::local_socket::{
            GenericFilePath,
            tokio::{Stream as LocalSocketStream, prelude::*},
        };
        use ssh_agent_lib::client::Client;
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-status.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, FailingTunnelAgent).await });

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

        let resp = svc.call(req).await?;

        let status = Status::from_header_map(resp.headers()).expect("status must be present");
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no such user");

        // Trailers-only: an error response carries no message frame.
        assert!(resp.into_body().collect().await?.to_bytes().is_empty());

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

    // --- End-to-end: real client encoder against the real agent-side dispatcher ---

    /// Runs the tunnel's real agent-side handler, so the request this client encodes has
    /// to survive everything the agent does with it — including being turned back into
    /// an HTTP request URI, which is where a path without its leading slash dies.
    #[derive(Clone, Default)]
    struct RealDispatchAgent;

    #[ssh_agent_lib::async_trait]
    impl Session for RealDispatchAgent {
        async fn extension(&mut self, ext: Extension) -> Result<Option<Extension>, AgentError> {
            let mut caller = MethodCaller::new(ProcCredentials::new(None));
            caller.add_service(PingServer::new(TestPing));

            Ok(Some(
                dispatch_tunnel_request(&mut caller, ext.details.as_ref()).await,
            ))
        }
    }

    struct TestPing;

    #[tonic::async_trait]
    impl Ping for TestPing {
        async fn ping(
            &self,
            _req: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<PingResponse>, Status> {
            Ok(tonic::Response::new(PingResponse {
                component: "tunnel".into(),
                version: "1.0".into(),
                server_version: String::new(),
            }))
        }

        async fn capabilities(
            &self,
            _req: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<CapabilitiesResponse>, Status> {
            Err(Status::permission_denied("nope"))
        }
    }

    /// The test that was missing: every earlier tunnel test mocked one side, so a
    /// client/agent disagreement about the method path could not show up. Here the real
    /// encoder talks to the real dispatcher over a real socket.
    #[cfg(unix)]
    #[tokio::test]
    async fn tunnel_round_trips_through_the_real_dispatcher()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use prost::Message;
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-dispatch.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, RealDispatchAgent).await });

        let mut svc = SSHTunnel::connect_to(sock_path).await?.service(());

        let req = Request::builder()
            .method("POST")
            .uri("/ping.Ping/Ping")
            .header("content-type", "application/grpc+proto")
            .body(Full::new(Bytes::from(grpc_frame(&[]))))?;

        let resp = svc.call(req).await?;
        assert_eq!(
            Status::from_header_map(resp.headers()).map(|s| s.code()),
            Some(Code::Ok)
        );

        let body_bytes = resp.into_body().collect().await?.to_bytes();
        let ping = PingResponse::decode(&*grpc_unframe(&body_bytes)?)?;
        assert_eq!(ping.component, "tunnel");

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }

    /// What `ak` actually does: a generated tonic client on top of the tunnel service.
    /// Driving `SSHService::call` by hand skips tonic's own response handling, which is
    /// picky about what a response with a `grpc-status` header may contain.
    #[cfg(unix)]
    #[tokio::test]
    async fn tonic_client_round_trips_over_the_tunnel()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::generated::ping::ping_client::PingClient;
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-tonic-client.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, RealDispatchAgent).await });

        let svc = SSHTunnel::connect_to(sock_path).await?.service(());
        let mut client = PingClient::new(svc);

        let resp = client.ping(()).await?;
        assert_eq!(resp.into_inner().component, "tunnel");

        let err = client
            .capabilities(())
            .await
            .expect_err("the handler returns a status");
        assert_eq!(err.code(), Code::PermissionDenied);
        assert_eq!(err.message(), "nope");

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }

    /// The same path for a handler that fails: the status has to cross the tunnel intact
    /// rather than collapsing into "empty tunnel response".
    #[cfg(unix)]
    #[tokio::test]
    async fn tunnel_carries_handler_status_through_the_real_dispatcher()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use tokio::net::UnixListener;

        let sock_path = "/tmp/ak-test-grpc-ssh-dispatch-status.sock";
        let _ = std::fs::remove_file(sock_path);

        let listener = UnixListener::bind(sock_path)?;
        let server_handle =
            tokio::spawn(async move { ssh_listen(listener, RealDispatchAgent).await });

        let mut svc = SSHTunnel::connect_to(sock_path).await?.service(());

        let req = Request::builder()
            .method("POST")
            .uri("/ping.Ping/Capabilities")
            .header("content-type", "application/grpc+proto")
            .body(Full::new(Bytes::from(grpc_frame(&[]))))?;

        let resp = svc.call(req).await?;
        let status = Status::from_header_map(resp.headers()).expect("status must be present");
        assert_eq!(status.code(), Code::PermissionDenied);
        assert_eq!(status.message(), "nope");

        server_handle.abort();
        let _ = server_handle.await;
        let _ = std::fs::remove_file(sock_path);

        Ok(())
    }
}
