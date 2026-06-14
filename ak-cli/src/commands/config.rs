use crate::{
    App, format,
    setup::{
        self,
        ak::{DEFAULT_APP_SLUG, DEFAULT_CLIENT_ID},
    },
};
use ak_platform::prelude::*;
use ak_platform::{
    generated::{agent::RequestHeader, agent_ctrl::SetupRequest},
    grpc::assert_response_valid,
};
use clap::Subcommand;
use ratatui::text::Line;
use std::env;
use url::Url;

#[derive(Subcommand, Clone)]
pub enum ConfigCommands {
    /// List profiles
    ListProfiles,
    /// Configure authentik CLI
    Setup {
        #[arg(short, long, required = true)]
        authentik_url: String,
        #[arg(short = 'i', long, default_value = DEFAULT_CLIENT_ID)]
        client_id: String,
        #[arg(short = 'd', long, default_value = DEFAULT_APP_SLUG)]
        app_slug: String,
    },
}

pub async fn list_profiles(app: App) -> Result<()> {
    let res = app
        .user()
        .await?
        .clone()
        .ctrl()
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

pub async fn setup(app: App, authentik_url: &str, client_id: &str, app_slug: &str) -> Result<()> {
    let access_token: String;
    let refresh_token: String;
    if let Ok(at) = env::var("AK_CLI_ACCESS_TOKEN")
        && let Ok(rt) = env::var("AK_CLI_REFRESH_TOKEN")
    {
        access_token = at;
        refresh_token = rt;
    } else {
        let prof = setup::setup(setup::Options {
            profile_name: app.args.profile.clone(),
            authentik_url: Url::parse(authentik_url)?,
            app_slug: app_slug.to_owned(),
            client_id: client_id.to_owned(),
            url_callback: None,
        })
        .await?;
        if let Some(at) = prof.access_token
            && let Some(rt) = prof.refresh_token
        {
            access_token = at;
            refresh_token = rt;
        } else {
            return Err(Box::from(
                "Device-flow setup did not return access/refresh token",
            ));
        }
    }

    let res = app
        .clone()
        .user()
        .await?
        .ctrl()
        .setup(SetupRequest {
            header: Some(RequestHeader {
                profile: app.args.profile.clone(),
            }),
            authentik_url: authentik_url.to_owned(),
            app_slug: app_slug.to_owned(),
            client_id: client_id.to_owned(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;

    Ok(())
}
