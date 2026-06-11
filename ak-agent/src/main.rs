use ak_platform::{
    generated::agent_auth::agent_auth_server::AgentAuthServer, log::init_log_interactive, net::server::{SocketPermMode, listen}, paths::{AgentSocketID, agent_socket_path}
};
use waitgroup::WaitGroup;
use std::error::Error;
use tonic::transport::Server;

use crate::grpc::AgentGRPCServer;

pub mod grpc;
pub struct Agent {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ag = Agent {};
    init_log_interactive();

    let wg = WaitGroup::new();

    let w = wg.worker();
    tokio::spawn(async move {
        let grpc = match AgentGRPCServer::new(ag).await {
            Ok(grpc) => grpc,
            Err(e) => {
                log::error!("Failed to start grpc server: {e:?}");
                return
            }
        };
        grpc.start().await;
        drop(w);
    });
    wg.wait().await;
    Ok(())
}
