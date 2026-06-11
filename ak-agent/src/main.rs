use ak_platform::log::init_log_interactive;
use ak_platform::prelude::*;
use waitgroup::WaitGroup;

use crate::grpc::AgentGRPCServer;

pub mod grpc;
pub struct Agent {}

#[tokio::main]
async fn main() -> Result<()> {
    let ag = Agent {};
    init_log_interactive();

    let wg = WaitGroup::new();

    let w = wg.worker();
    tokio::spawn(async move {
        let grpc = match AgentGRPCServer::new(ag).await {
            Ok(grpc) => grpc,
            Err(e) => {
                log::error!("Failed to start grpc server: {e:?}");
                return;
            }
        };
        grpc.start().await;
        drop(w);
    });
    wg.wait().await;
    Ok(())
}
