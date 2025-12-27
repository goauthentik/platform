use std::error::Error;

use hyper_util::rt::TokioIo;
use tokio::runtime::Builder;
use tonic::transport::Uri;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

use crate::config::Config;

#[cfg(unix)]
async fn grpc_endpoint(ep: Endpoint) -> Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(async move |p: Uri| {
            use tokio::net::UnixStream;

            let path = p.query().unwrap().to_string();
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
async fn grpc_endpoint(ep: Endpoint) -> Result<Channel, tonic::transport::Error> {
    return ep
        .connect_with_connector(service_fn(async |p: Uri| {
            use std::time::Duration;
            use tokio::net::windows::named_pipe::ClientOptions;
            use tokio::time;

            let path = p
                .query()
                .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?
                .to_string();
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

    grpc_request_path(config.socket.to_owned(), future)
}

pub fn grpc_request_path<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
    path: String,
    future: impl Fn(Channel) -> F,
) -> Result<T, Box<dyn Error>> {
    let rt = Builder::new_current_thread().enable_all().build()?;

    rt.block_on(async {
        log::debug!("creating grpc client");
        let ep = match Endpoint::try_from(format!("http://:123/?{}", path)) {
            Ok(e) => e,
            Err(e) => return Err(Box::from(e)),
        };
        let channel = grpc_endpoint(ep).await?;
        match future(channel).await {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    })
}
