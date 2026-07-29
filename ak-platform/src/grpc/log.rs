use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Instant;

use tonic::Code;
use tower::{Layer, Service};
use tracing::{Instrument, Level, Span, span};

use crate::net::server::creds::ProcCredentials;

/// Tower layer that logs a "started call" / "finished call" line for every gRPC
/// request, mirroring the go-grpc-middleware logging fields.
#[derive(Clone, Copy, Default)]
pub struct TraceLayer;

impl TraceLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for TraceLayer {
    type Service = TraceService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceService { inner }
    }
}

#[derive(Clone)]
pub struct TraceService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for TraceService<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        let (service, method) = split_grpc_path(req.uri().path());
        let peer_pid = req
            .extensions()
            .get::<ProcCredentials>()
            .map(|c| c.pid())
            .unwrap_or(-1);
        let start = Instant::now();

        // Tower clone trick: swap the ready service out, put a fresh clone back.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            tracing::info!(
                grpc.service = %service,
                grpc.method = %method,
                peer.pid = peer_pid,
                id = ?Span::current().id(),
                "started call"
            );

            let result = inner.call(req).await;
            let time_ms = start.elapsed().as_secs_f64() * 1000.0;
            let code = match &result {
                Ok(resp) => grpc_code_from_headers(resp.headers()),
                Err(_) => Code::Unknown,
            };

            tracing::info!(
                grpc.service = %service,
                grpc.method = %method,
                grpc.code = ?code,
                grpc.time_ms = time_ms,
                peer.pid = peer_pid,
                id = ?Span::current().id(),
                "finished call"
            );

            result
        })
    }
}

/// gRPC request paths are `/<package.Service>/<Method>`.
fn split_grpc_path(path: &str) -> (String, String) {
    let trimmed = path.trim_start_matches('/');
    match trimmed.split_once('/') {
        Some((service, method)) => (service.to_string(), method.to_string()),
        None => (trimmed.to_string(), String::new()),
    }
}

/// A trailers-only response (the common tonic error path) carries `grpc-status`
/// in the headers; a successful unary call carries it in the trailers, so an
/// absent header is treated as `Ok`.
fn grpc_code_from_headers(headers: &http::HeaderMap) -> Code {
    headers
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i32>().ok())
        .map(Code::from)
        .unwrap_or(Code::Ok)
}
