use base64::{Engine, prelude::BASE64_STANDARD};
use eyre::Report;
use eyre::Result;
use eyre::bail;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::future::Future;
use std::io::ErrorKind;
use tokio::runtime::{Builder, Runtime};
use tonic::transport::Uri;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

use crate::config::Config;
use crate::generated::agent::ResponseHeader;
use crate::net;
use crate::string::PlatformString;

pub mod log;
pub mod method_caller;
pub mod ssh;

pub use tonic::Code;
pub use tonic::Status;

pub type GrpcResult<T> = std::result::Result<T, GrpcError>;

#[derive(Debug)]
pub enum GrpcError {
    Status(tonic::Status),
    Transport(tonic::transport::Error, String),
    SocketNotFound(),
    Other(Report),
}

impl Display for GrpcError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            GrpcError::Status(s) => s.fmt(f),
            GrpcError::Transport(error, path) => {
                write!(f, "error connecting to {}: {:?}", path, error)
            }
            GrpcError::SocketNotFound() => write!(f, "socket not found"),
            GrpcError::Other(report) => report.fmt(f),
        }
    }
}
impl std::error::Error for GrpcError {}

impl From<tonic::Status> for GrpcError {
    fn from(value: tonic::Status) -> Self {
        GrpcError::Status(value)
    }
}

impl GrpcError {
    /// Whether a failed dial means "there is no socket at that path".
    ///
    /// `tonic::transport::Error` holds the connector's `io::Error` down its
    /// `source()` chain rather than exposing it directly, so the ENOENT is only
    /// visible by walking it.
    fn socket_missing(e: &tonic::transport::Error) -> bool {
        let mut cur: Option<&(dyn Error + 'static)> = Some(e);
        while let Some(err) = cur {
            if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
                return io_err.kind() == ErrorKind::NotFound;
            }
            cur = err.source();
        }
        false
    }

    /// Classifies a failed dial, keeping `path` for the error message.
    ///
    /// Prefer this over `From<tonic::transport::Error>` wherever the path is
    /// known — the `From` impl has to leave it blank.
    fn from_dial(e: tonic::transport::Error, path: String) -> Self {
        if GrpcError::socket_missing(&e) {
            GrpcError::SocketNotFound()
        } else {
            GrpcError::Transport(e, path)
        }
    }
}

pub async fn grpc_endpoint(path: String) -> GrpcResult<Channel> {
    // Dummy URI to satisfy Endpoint::from() type requirements
    // The connector ignores this and uses the socket_path parameter directly
    let u = Uri::builder()
        .scheme("http")
        .authority(":123")
        .path_and_query("/")
        .build()
        .map_err(|e| GrpcError::Other(e.into()))?;
    let endpoint = Endpoint::from(u);
    let channel = grpc_dial(endpoint, path.clone())
        .await
        .map_err(|e| GrpcError::from_dial(e, path))?;
    Ok(channel)
}

async fn grpc_dial(
    ep: Endpoint,
    socket_path: String,
) -> std::result::Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(move |_p: Uri| {
            let path = socket_path.clone();
            async move {
                tracing::debug!(path = path, "Connecting to GRPC socket");
                net::client::connect(PlatformString::new_with_default(&path)).await
            }
        }))
        .await;
}

pub fn grpc_request<T, F: Future<Output = GrpcResult<T>>>(
    future: impl Fn(Channel) -> F,
) -> GrpcResult<T> {
    let config = Config::get();

    grpc_request_path(config.socket_default.for_current().to_owned(), future)
}

pub fn grpc_request_path<T, F: Future<Output = GrpcResult<T>>>(
    path: String,
    future: impl Fn(Channel) -> F,
) -> GrpcResult<T> {
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| GrpcError::Other(e.into()))?;

    rt.block_on(async {
        let channel = grpc_endpoint(path).await?;
        future(channel).await
    })
}

pub trait SysdBridge {
    fn grpc_request<T, F: Future<Output = GrpcResult<T>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> GrpcResult<T>;
    fn grpc_request_path<T, F: Future<Output = GrpcResult<T>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> GrpcResult<T>;
}

pub struct Bridge {
    rt: Runtime,
}

impl Bridge {
    pub fn new() -> Result<Self> {
        let rt = Builder::new_current_thread().enable_all().build()?;
        Ok(Self { rt })
    }
}

impl SysdBridge for Bridge {
    fn grpc_request<T, F: Future<Output = GrpcResult<T>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> GrpcResult<T> {
        let config = Config::get();

        self.grpc_request_path(config.socket_default.for_current().to_owned(), future)
    }

    fn grpc_request_path<T, F: Future<Output = GrpcResult<T>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> GrpcResult<T> {
        self.rt.block_on(async {
            tracing::debug!("creating grpc client");
            let channel = grpc_endpoint(path).await?;
            match future(channel).await {
                Ok(t) => Ok(t),
                Err(e) => Err(e),
            }
        })
    }
}

pub fn decode_pb<T: ::prost::Message + Default>(token: String) -> Result<T> {
    let raw = BASE64_STANDARD.decode(token)?;
    let msg = T::decode(&*raw)?;
    Ok(msg)
}

pub fn encode_pb<T: ::prost::Message>(msg: T) -> Result<String> {
    let raw = msg.encode_to_vec();
    Ok(BASE64_STANDARD.encode(raw))
}

pub fn assert_response_valid(header: Option<ResponseHeader>) -> Result<()> {
    if let Some(header) = header
        && !header.successful
    {
        bail!("unsuccessful request");
    }
    Ok(())
}
