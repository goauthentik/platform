use crate::generated::pam_session::session_manager_client::SessionManagerClient;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

pub mod pam_session;

pub async fn create_grpc_client()
-> Result<SessionManagerClient<Channel>, Box<dyn std::error::Error>> {
    log::info!("creating grpc client");
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(|_: Uri| {
            UnixStream::connect("/var/run/authentik-session-manager.sock")
        }))
        .await?;

    Ok(SessionManagerClient::new(channel))
}
