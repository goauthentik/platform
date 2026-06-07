use std::error::Error;
use clap::Subcommand;
use authentik_sys::{
    generated::agent_ctrl::agent_ctrl_client::AgentCtrlClient,
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};
use ratatui::text::Line;

use crate::{Cli, format};


#[derive(Subcommand)]
pub enum ConfigCommands {
    /// List profiles
    ListProfiles,
}

pub async fn list_profiles(_cli: &Cli) -> Result<(), Box<dyn Error>> {
    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentCtrlClient::new(c)
        .list_profiles(())
        .await?
        .into_inner();
    assert_response_valid(res.header)?;
    for profile in res.profiles {
        println!("{}", Line::styled(profile.name.to_string(), format::inline_style()))
    }
    Ok(())
}
