use std::collections::HashMap;
use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use tonic::server::NamedService;
use tower::ServiceExt;
use tower::util::BoxCloneService;

use crate::net::server::creds::ProcCredentials;
use crate::prelude::*;

type BoxedSvc =
    BoxCloneService<http::Request<Full<Bytes>>, http::Response<tonic::body::Body>, Infallible>;

/// Routes raw proto bytes to registered tonic services over their HTTP interface,
/// mirroring the Go `MethodCaller` / `grpc.ServiceRegistrar` pattern.
///
/// Services are registered with `add_service` using the same generated server
/// wrappers (`AgentAuthServer::from_arc(...)`) as the real gRPC server, so adding
/// a new method to any service automatically becomes available through the caller
/// without any changes here.
pub struct MethodCaller {
    services: HashMap<String, BoxedSvc>,
    creds: ProcCredentials,
}

impl MethodCaller {
    pub fn new(creds: ProcCredentials) -> Self {
        Self {
            services: HashMap::new(),
            creds,
        }
    }

    /// Register a tonic service. Accepts the same generated server wrappers as
    /// `tonic::transport::Server::add_service`.
    pub fn add_service<S>(&mut self, service: S)
    where
        S: NamedService
            + tower::Service<
                http::Request<Full<Bytes>>,
                Response = http::Response<tonic::body::Body>,
                Error = Infallible,
            > + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
    {
        self.services
            .insert(S::NAME.to_string(), BoxCloneService::new(service));
    }

    pub fn service_names(&self) -> impl Iterator<Item = &str> {
        self.services.keys().map(String::as_str)
    }

    /// Call a gRPC method by its full path (e.g. `/ping.Ping/Ping`).
    /// `data` is the raw serialised proto request (no gRPC framing).
    /// Returns the raw serialised proto response on success.
    pub async fn call(&mut self, method: &str, data: &[u8]) -> Result<Vec<u8>> {
        let service_name = parse_service_name(method)?;

        let svc = self
            .services
            .get(service_name)
            .ok_or_else(|| -> BoxError { Box::from(format!("unknown service: {service_name}")) })?
            .clone();

        let mut req = http::Request::builder()
            .method("POST")
            .uri(method)
            .header("content-type", "application/grpc+proto")
            .header("te", "trailers")
            .body(Full::new(Bytes::from(grpc_frame(data))))
            .map_err(|e| -> BoxError { Box::from(e.to_string()) })?;
        req.extensions_mut().insert(self.creds.clone());

        let resp = svc
            .oneshot(req)
            .await
            .unwrap_or_else(|_: Infallible| unreachable!());

        let (parts, body) = resp.into_parts();
        let collected = body
            .collect()
            .await
            .map_err(|e| -> BoxError { Box::from(e.to_string()) })?;

        // gRPC status lives in HTTP/2 trailers; fall back to headers for
        // direct (non-transport) calls where they may be merged.
        let status = collected
            .trailers()
            .and_then(|t| t.get("grpc-status"))
            .or_else(|| parts.headers.get("grpc-status"))
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if status != 0 {
            let msg = collected
                .trailers()
                .and_then(|t| t.get("grpc-message"))
                .or_else(|| parts.headers.get("grpc-message"))
                .and_then(|v| v.to_str().ok())
                .unwrap_or("gRPC call failed");
            return Err(Box::from(format!("gRPC status {status}: {msg}")));
        }

        grpc_unframe(&collected.to_bytes())
    }
}

fn parse_service_name(method: &str) -> Result<&str> {
    let path = method.trim_start_matches('/');
    let end = path
        .rfind('/')
        .ok_or_else(|| -> BoxError { Box::from(format!("invalid method path: {method}")) })?;
    Ok(&path[..end])
}

pub fn grpc_frame(data: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + data.len());
    buf.push(0u8);
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(data);
    buf
}

pub fn grpc_unframe(data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 5 {
        return Err(Box::from("gRPC response frame too short"));
    }
    let msg_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    data.get(5..5 + msg_len)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| Box::from("gRPC response truncated") as BoxError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ping::{
        CapabilitiesResponse, PingResponse,
        ping_server::{Ping, PingServer},
    };
    use prost::Message;

    struct TestPing;

    #[tonic::async_trait]
    impl Ping for TestPing {
        async fn ping(
            &self,
            _req: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<PingResponse>, tonic::Status> {
            Ok(tonic::Response::new(PingResponse {
                component: "test".into(),
                version: "1.0".into(),
            }))
        }

        async fn capabilities(
            &self,
            _req: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<CapabilitiesResponse>, tonic::Status> {
            Ok(tonic::Response::new(CapabilitiesResponse {
                capabilities: vec![],
            }))
        }
    }

    fn make_caller() -> MethodCaller {
        let mut caller = MethodCaller::new(ProcCredentials::new(None));
        caller.add_service(PingServer::new(TestPing));
        caller
    }

    #[tokio::test]
    async fn call_known_method_returns_decoded_response() {
        let mut caller = make_caller();
        let bytes = caller.call("/ping.Ping/Ping", &[]).await.unwrap();
        let resp = PingResponse::decode(&*bytes).unwrap();
        assert_eq!(resp.component, "test");
        assert_eq!(resp.version, "1.0");
    }

    #[tokio::test]
    async fn call_second_method_on_same_service() {
        let mut caller = make_caller();
        let bytes = caller.call("/ping.Ping/Capabilities", &[]).await.unwrap();
        let resp = CapabilitiesResponse::decode(&*bytes).unwrap();
        assert!(resp.capabilities.is_empty());
    }

    #[tokio::test]
    async fn unknown_service_returns_error() {
        let mut caller = make_caller();
        let err = caller
            .call("/unknown.Service/Method", &[])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("unknown service"), "got: {err}");
    }

    #[tokio::test]
    async fn invalid_method_path_returns_error() {
        let mut caller = make_caller();
        let err = caller.call("badpath", &[]).await.unwrap_err();
        assert!(
            err.to_string().contains("invalid method path"),
            "got: {err}"
        );
    }

    #[test]
    fn service_names_contains_registered_service() {
        let caller = make_caller();
        let names: Vec<&str> = caller.service_names().collect();
        assert!(names.contains(&"ping.Ping"), "got: {names:?}");
    }

    // --- grpc_unframe ---

    #[test]
    fn grpc_unframe_valid() {
        let data = [0x00u8, 0x00, 0x00, 0x00, 0x05, 1, 2, 3, 4, 5];
        let result = grpc_unframe(&data).unwrap();
        assert_eq!(result, &[1u8, 2, 3, 4, 5]);
    }

    #[test]
    fn grpc_unframe_empty_payload() {
        let data = [0x00u8, 0x00, 0x00, 0x00, 0x00];
        let result = grpc_unframe(&data).unwrap();
        assert_eq!(result, &[] as &[u8]);
    }

    #[test]
    fn grpc_unframe_too_short() {
        let err = grpc_unframe(&[0x00u8, 0x01, 0x02]).unwrap_err();
        assert!(err.to_string().contains("too short"), "error was: {err}");
    }

    #[test]
    fn grpc_unframe_length_exceeds_buffer() {
        // Header claims 100 bytes, only 3 bytes of payload follow.
        let mut data = vec![0x00u8, 0x00, 0x00, 0x00, 100];
        data.extend_from_slice(&[1u8, 2, 3]);
        let err = grpc_unframe(&data).unwrap_err();
        assert!(err.to_string().contains("truncated"), "error was: {err}");
    }

    // --- grpc_frame ---

    #[test]
    fn grpc_frame_header_bytes() {
        assert_eq!(
            grpc_frame(b"hello"),
            [0x00, 0x00, 0x00, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn grpc_frame_empty() {
        assert_eq!(grpc_frame(b""), [0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn roundtrip_add_then_strip() {
        let original = b"some proto bytes";
        let framed = grpc_frame(original);
        let stripped = grpc_unframe(&framed).unwrap();
        assert_eq!(stripped, original);
    }
}
