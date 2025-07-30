use std::error::Error;

use authentik_sys::{config::Config, generated::nss::nss_client::NssClient};
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

pub async fn create_grpc_client(config: Config) -> Result<NssClient<Channel>, Box<dyn Error>> {
    log::info!("creating grpc client");
    let path = config.socket.to_owned();
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            UnixStream::connect(path.to_owned())
        }))
        .await?;

    Ok(NssClient::new(channel))
}
