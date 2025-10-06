pub mod agent;
pub mod agent_auth;
pub mod nss;
pub mod pam;
pub mod pam_session;
use std::error::Error;

use tokio::net::UnixStream;
use tokio::runtime::Builder;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use crate::{config::Config, generated::nss::nss_client::NssClient};

pub async fn create_grpc_client(config: Config) -> Result<NssClient<Channel>, Box<dyn Error>> {
    log::debug!("creating grpc client");
    let path = config.socket.to_owned();
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            UnixStream::connect(path.to_owned())
        }))
        .await?;

    Ok(NssClient::new(channel))
}

pub fn grpc_request<T, F: Future<Output = Result<T, Box<dyn Error>>>>(
    future: impl Fn(Channel) -> F,
) -> Result<T, Box<dyn Error>> {
    let rt = match Builder::new_current_thread().enable_all().build() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {e}");
            return Err(Box::from(e));
        }
    };
    let config = Config::get();

    rt.block_on(async {
        log::debug!("creating grpc client");
        let path = config.socket.to_owned();
        let ep = match Endpoint::try_from("http://[::]:50051") {
            Ok(e) => e,
            Err(e) => return Err(Box::from(e)),
        };
        let channel = ep
            .connect_with_connector(service_fn(move |_: Uri| {
                UnixStream::connect(path.to_owned())
            }))
            .await?;
        match future(channel).await {
            Ok(t) => Ok(t),
            Err(e) => Err(e),
        }
    })
}
