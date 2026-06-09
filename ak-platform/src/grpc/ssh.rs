use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{Request, Response};
use http_body_util::{BodyExt, Full};
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tonic::service::Interceptor;
use tower::{Layer, Service};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

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
}

pub struct SSHTunnel {
    stream: Arc<Mutex<UnixStream>>,
}

impl SSHTunnel {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sock_path = std::env::var("SSH_AUTH_SOCK")
            .map_err(|_| "SSH_AUTH_SOCK is not set")?;
        let stream = UnixStream::connect(sock_path).await?;
        Ok(SSHTunnel {
            stream: Arc::new(Mutex::new(stream)),
        })
    }
}

impl Clone for SSHTunnel {
    fn clone(&self) -> Self {
        SSHTunnel {
            stream: Arc::clone(&self.stream),
        }
    }
}

/// Send an SSH agent extension request and return the response bytes.
///
/// Frame format (request):
///   [4 bytes BE] frame body length
///   [1 byte]     SSH_AGENTC_EXTENSION (0x1b)
///   [4 bytes BE] extension-type length
///   [n bytes]    extension-type
///   [4 bytes BE] extension-contents length
///   [n bytes]    extension-contents
///
/// Frame format (response on success):
///   [4 bytes BE] frame body length
///   [1 byte]     SSH_AGENT_SUCCESS (0x06)
///   [4 bytes BE] response data length
///   [n bytes]    response data
async fn agent_extension(
    stream: &mut UnixStream,
    ext_type: &[u8],
    ext_data: &[u8],
) -> Result<Vec<u8>, BoxError> {
    const SSH_AGENTC_EXTENSION: u8 = 0x1b;
    const SSH_AGENT_SUCCESS: u8 = 0x06;
    const MAX_FRAME: usize = 256 * 1024;

    let body_len = 1 + 4 + ext_type.len() + 4 + ext_data.len();
    let mut frame = Vec::with_capacity(4 + body_len);
    frame.extend_from_slice(&(body_len as u32).to_be_bytes());
    frame.push(SSH_AGENTC_EXTENSION);
    frame.extend_from_slice(&(ext_type.len() as u32).to_be_bytes());
    frame.extend_from_slice(ext_type);
    frame.extend_from_slice(&(ext_data.len() as u32).to_be_bytes());
    frame.extend_from_slice(ext_data);

    stream.write_all(&frame).await?;
    stream.flush().await?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let frame_len = u32::from_be_bytes(len_buf) as usize;
    if frame_len > MAX_FRAME {
        return Err("SSH agent response exceeds maximum frame size".into());
    }
    let mut resp = vec![0u8; frame_len];
    stream.read_exact(&mut resp).await?;

    if resp.first() != Some(&SSH_AGENT_SUCCESS) {
        return Err("SSH agent extension call failed".into());
    }
    if resp.len() < 5 {
        return Ok(Vec::new());
    }
    let data_len = u32::from_be_bytes([resp[1], resp[2], resp[3], resp[4]]) as usize;
    resp.get(5..5 + data_len)
        .map(ToOwned::to_owned)
        .ok_or_else(|| "SSH agent response truncated".into())
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

/// Tower service that replaces the tonic transport, routing each gRPC call
/// through the SSH agent tunnel extension instead of a direct socket.
pub struct SSHService {
    tunnel: SSHTunnel,
}

impl Clone for SSHService {
    fn clone(&self) -> Self {
        SSHService {
            tunnel: self.tunnel.clone(),
        }
    }
}

impl<B> Service<Request<B>> for SSHService
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    type Response = Response<tonic::body::Body>;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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

            let proto = strip_grpc_frame(&body_bytes)?;
            let payload = ExtAuthentikAgentTunnelData {
                method,
                data: proto.to_vec(),
            };

            let mut stream = tunnel.stream.lock().await;
            let resp_proto = agent_extension(
                &mut stream,
                EXT_AUTHENTIK_AGENT_TUNNEL.as_bytes(),
                &payload.serialize(),
            )
            .await?;
            drop(stream);

            let framed = add_grpc_frame(&resp_proto);
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


impl Interceptor for SSHTunnel {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let body: &() = request.get_ref();   // &impl Message
        let _ = body.encode_to_vec();    // == []  (zero-length)

        log::debug!("{:?}", request.metadata());

        todo!()
    }
}
