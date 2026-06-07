use std::error::Error;

use base64::{Engine, prelude::BASE64_STANDARD};
use hyper_util::rt::TokioIo;
use tokio::runtime::{Builder, Runtime};
use tonic::transport::Uri;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

use crate::config::Config;
use crate::generated::agent::ResponseHeader;

pub async fn grpc_endpoint(path: String) -> Result<Channel, Box<dyn Error>> {
    let u = Uri::builder()
        .scheme("http")
        .authority(":123")
        .path_and_query(path.replace(" ", "%20"))
        .build()?;
    let endpoint = Endpoint::from(u);
    let channel = grpc_dial(endpoint).await?;
    Ok(channel)
}

#[cfg(unix)]
async fn grpc_dial(ep: Endpoint) -> Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(async move |p: Uri| {
            use tokio::net::UnixStream;

            let path = p.path().replace("%20", " ");
            log::debug!("Connecting to GRPC socket '{path}'");
            let client = match UnixStream::connect(path).await {
                Ok(c) => c,
                Err(e) => {
                    return Err(e);
                }
            };
            Ok(TokioIo::new(client))
        }))
        .await;
}

#[cfg(windows)]
async fn grpc_dial(ep: Endpoint) -> Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(async |p: Uri| {
            use std::time::Duration;
            use tokio::net::windows::named_pipe::ClientOptions;
            use tokio::time;

            let path = p.path().replace("%20", " ");
            log::debug!("Connecting to GRPC socket '{path}'");
            let client = loop {
                match ClientOptions::new().open(&path) {
                    Ok(client) => break client,
                    Err(e) if e.raw_os_error() == Some(231) => (),
                    Err(e) => return Err(e),
                }

                time::sleep(Duration::from_millis(50)).await;
            };

            Ok(TokioIo::new(client))
        }))
        .await;
}

pub fn grpc_request<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
    future: impl Fn(Channel) -> F,
) -> Result<T, Box<dyn Error>> {
    let config = Config::get();

    grpc_request_path(config.socket_default.for_current().to_owned(), future)
}

pub fn grpc_request_path<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
    path: String,
    future: impl Fn(Channel) -> F,
) -> Result<T, Box<dyn Error>> {
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
    fn grpc_request<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> Result<T, Box<dyn Error>>;
    fn grpc_request_path<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> Result<T, Box<dyn Error>>;
}

pub struct Bridge {
    rt: Runtime,
}

impl Bridge {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let rt = Builder::new_current_thread().enable_all().build()?;
        Ok(Self { rt })
    }
}

impl SysdBridge for Bridge {
    fn grpc_request<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
        &self,
        future: impl Fn(Channel) -> F,
    ) -> Result<T, Box<dyn Error>> {
        let config = Config::get();

        self.grpc_request_path(config.socket_default.for_current().to_owned(), future)
    }

    fn grpc_request_path<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
        &self,
        path: String,
        future: impl Fn(Channel) -> F,
    ) -> Result<T, Box<dyn Error>> {
        self.rt.block_on(async {
            log::debug!("creating grpc client");
            let channel = grpc_endpoint(path).await?;
            match future(channel).await {
                Ok(t) => Ok(t),
                Err(e) => Err(e),
            }
        })
    }
}

pub fn decode_pb<T: ::prost::Message + Default>(token: String) -> Result<T, Box<dyn Error>> {
    let raw = BASE64_STANDARD.decode(token)?;
    let msg = T::decode(&*raw)?;
    Ok(msg)
}

pub fn encode_pb<T: ::prost::Message>(msg: T) -> Result<String, Box<dyn Error>> {
    let raw = msg.encode_to_vec();
    Ok(BASE64_STANDARD.encode(raw))
}

pub fn assert_response_valid(header: Option<ResponseHeader>) -> Result<(), Box<dyn Error>> {
    if let Some(header) = header {
        if !header.successful {
            return Err(Box::from("unsuccessful request"));
        }
    }
    Ok(())
}
