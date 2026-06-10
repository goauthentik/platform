use ak_platform::{
    generated::ping::ping_client::PingClient,
    grpc::grpc_endpoint,
    platform::{
        paths::{AgentSocketID, SysdSocketID, agent_socket_path, sysd_socket_path},
        string::PlatformString,
    },
};
use ratatui::text::Line;
use std::error::Error;

use crate::{App, format};

pub async fn version(_app: App) -> Result<(), Box<dyn Error>> {
    let user_version = agent_version(agent_socket_path(AgentSocketID::Default)?).await;
    let system_version = agent_version(sysd_socket_path(SysdSocketID::Default)).await;
    let versions = vec![
        format!("authentik Agent CLI: {}", env!("CARGO_PKG_VERSION")),
        format!("Agent: {}", user_version),
        format!("System: {}", system_version),
    ];
    for line in &versions {
        println!("{}", Line::styled(line, format::inline_style()))
    }
    Ok(())
}

async fn agent_version(p: PlatformString) -> String {
    let c = match grpc_endpoint(p.for_current()).await {
        Ok(c) => c,
        Err(e) => return format!("{e:?}"),
    };
    let res = match PingClient::new(c).ping(()).await {
        Ok(res) => res,
        Err(e) => return format!("{e:?}"),
    };
    res.into_inner().version
}
