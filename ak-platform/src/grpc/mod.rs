use crate::prelude::*;
use base64::{Engine, prelude::BASE64_STANDARD};
use eyre::bail;
use tokio::runtime::{Builder, Runtime};
use tonic::transport::Uri;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

use crate::config::Config;
use crate::generated::agent::ResponseHeader;
use crate::net;
use crate::string::PlatformString;

pub mod method_caller;
pub mod ssh;

pub async fn grpc_endpoint(path: String) -> Result<Channel> {
    let u = Uri::builder()
        .scheme("http")
        .authority(":123")
        .path_and_query(path.replace(" ", "%20"))
        .build()?;
    let endpoint = Endpoint::from(u);
    let channel = grpc_dial(endpoint).await?;
    Ok(channel)
}

async fn grpc_dial(ep: Endpoint) -> std::result::Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(async move |p: Uri| {
            let path = p.path().replace("%20", " ");
            tracing::debug!(path = path, "Connecting to GRPC socket");
            net::client::connect(PlatformString::new_with_default(&path)).await
        }))
        .await;
}

pub fn grpc_request<T, F: Future<Output = Result<T>>>(future: impl Fn(Channel) -> F) -> Result<T> {
    let config = Config::get();

    grpc_request_path(config.socket_default.for_current().to_owned(), future)
}

pub fn grpc_request_path<T, F: Future<Output = Result<T>>>(
    path: String,
    future: impl Fn(Channel) -> F,
) -> Result<T> {
    let rt = Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        let channel = grpc_endpoint(path).await?;
        match future(channel).await {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    })
}

pub trait SysdBridge {
    fn grpc_request<T, F: Future<Output = Result<T>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> Result<T>;
    fn grpc_request_path<T, F: Future<Output = Result<T>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> Result<T>;
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
    fn grpc_request<T, F: Future<Output = Result<T>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> Result<T> {
        let config = Config::get();

        self.grpc_request_path(config.socket_default.for_current().to_owned(), future)
    }

    fn grpc_request_path<T, F: Future<Output = Result<T>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> Result<T> {
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
