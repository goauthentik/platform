use authentik_sys::{
    generated::{
        agent::RequestHeader,
        agent_ctrl::{SetupRequest, agent_ctrl_client::AgentCtrlClient},
    },
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};
use clap::Subcommand;
use ratatui::text::Line;
use std::{env, error::Error};
use url::Url;

use crate::{
    Cli, format,
    setup::{
        self,
        ak::{DEFAULT_APP_SLUG, DEFAULT_CLIENT_ID},
    },
};

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// List profiles
    ListProfiles,
    /// Configure authentik CLI
    Setup {
        #[arg(short, long, required = true)]
        authentik_url: String,
        #[arg(short, long, required = true, default_value = DEFAULT_CLIENT_ID)]
        client_id: String,
        #[arg(short, long, required = true, default_value = DEFAULT_APP_SLUG)]
        app_slug: String,
    },
}

pub async fn list_profiles(_cli: &Cli) -> Result<(), Box<dyn Error>> {
    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentCtrlClient::new(c)
        .list_profiles(())
        .await?
        .into_inner();
    assert_response_valid(res.header)?;
    for profile in res.profiles {
        println!(
            "{}",
            Line::styled(profile.name.to_string(), format::inline_style())
        )
    }
    Ok(())
}

pub async fn setup(
    cli: &Cli,
    authentik_url: &String,
    client_id: &String,
    app_slug: &String,
) -> Result<(), Box<dyn Error>> {
    let access_token: String;
    let refresh_token: String;
    if let Ok(at) = env::var("AK_CLI_ACCESS_TOKEN")
        && let Ok(rt) = env::var("AK_CLI_REFRESH_TOKEN")
    {
        access_token = at;
        refresh_token = rt;
    } else {
        let prof = setup::setup(setup::Options {
            profile_name: cli.profile.clone(),
            authentik_url: Url::parse(authentik_url)?,
            app_slug: app_slug.clone(),
            client_id: client_id.clone(),
            url_callback: None,
        })
        .await?;
        access_token = prof.access_token.unwrap();
        refresh_token = prof.refresh_token.unwrap();
    }

    let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
    let res = AgentCtrlClient::new(c)
        .setup(SetupRequest {
            header: Some(RequestHeader {
                profile: cli.profile.clone(),
            }),
            authentik_url: authentik_url.clone(),
            app_slug: app_slug.clone(),
            client_id: client_id.clone(),
            access_token,
            refresh_token,
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;

    Ok(())
}
