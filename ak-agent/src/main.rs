use std::sync::Arc;

use ak_platform::log::init_log_interactive;
use ak_platform::{keyring, prelude::*};
use waitgroup::WaitGroup;

use crate::grpc::AgentGRPCServer;
use crate::ssh::AgentSSHServer;

pub mod config;
pub mod grpc;
pub mod ssh;

pub struct Agent {}

#[tokio::main]
async fn main() -> Result<()> {
    let ag = Agent {};
    keyring::init()?;
    init_log_interactive();

    let wg = WaitGroup::new();

    let w_grpc = wg.worker();
    let w_ssh = wg.worker();

    let shared = Arc::new(ag);
    let shared_grpc = Arc::clone(&shared);

    tokio::spawn(async move {
        let grpc = match AgentGRPCServer::new(shared_grpc).await {
            Ok(grpc) => grpc,
            Err(e) => {
                log::error!("Failed to start grpc server: {e:?}");
                return;
            }
        };
        match grpc.start().await {
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to start grpc server: {e:?}");
            }
        };
        drop(w_grpc);
    });

    tokio::spawn(async move {
        let ssh = AgentSSHServer::new(Arc::clone(&shared)).await;
        match ssh.start().await {
            Ok(()) => (),
            Err(e) => {
                log::error!("failed to start ssh agent: {e:?}");
            }
        };
        drop(w_ssh);
    });
    wg.wait().await;
    Ok(())
}
