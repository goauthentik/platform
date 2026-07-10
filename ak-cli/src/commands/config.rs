use crate::{
    App,
    format::{self, render_timestamp},
    setup::{
        self,
        ak::{DEFAULT_APP_SLUG, DEFAULT_CLIENT_ID},
    },
};
use ak_platform::{
    generated::{agent::RequestHeader, agent_ctrl::SetupRequest},
    grpc::assert_response_valid,
};
use clap::Subcommand;
use eyre::{Result, WrapErr, bail};
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
        .await
        .wrap_err("failed to list profiles")?
        .into_inner();
    assert_response_valid(res.header)?;
    for profile in res.profiles {
        println!(
            "{}:",
            Line::styled(profile.name.to_string(), format::inline_style())
        );
        println!("\tUsername: {}", profile.username);
        println!("\tLast Renewal: {}", render_timestamp(profile.last_renewed));
        println!("\tNext Renewal: {}", render_timestamp(profile.next_renew));
        println!("\tauthentik URL: {}", profile.authentik_url);
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
            profile_name: app.args.profile.clone().unwrap_or(app.profile()),
            authentik_url: Url::parse(authentik_url).wrap_err("invalid authentik URL")?,
            app_slug: app_slug.to_owned(),
            client_id: client_id.to_owned(),
            url_callback: None,
        })
        .await
        .wrap_err("device flow setup failed")?;
        if let Some(at) = prof.access_token
            && let Some(rt) = prof.refresh_token
        {
            access_token = at;
            refresh_token = rt;
        } else {
            bail!("Device-flow setup did not return access/refresh token");
        }
    }

    let res = app
        .clone()
        .user()
        .await?
        .ctrl()
        .setup(SetupRequest {
            header: Some(RequestHeader {
                profile: app.args.profile.clone().unwrap_or(app.profile()),
            }),
            authentik_url: authentik_url.to_owned(),
            app_slug: app_slug.to_owned(),
            client_id: client_id.to_owned(),
            access_token: access_token.clone(),
            refresh_token: refresh_token.clone(),
        })
        .await
        .wrap_err("failed to register profile with agent")?
        .into_inner();
    assert_response_valid(res.header)?;

    Ok(())
}

pub async fn current_profile(app: App) -> Result<()> {
    let res = app
        .user()
        .await?
        .clone()
        .ctrl()
        .current_profile(())
        .await
        .wrap_err("failed to get current profile")?
        .into_inner();
    assert_response_valid(res.header)?;
    println!("{}", res.profile);
    Ok(())
}

pub async fn switch_profile(app: App, profile: &str) -> Result<()> {
    let res = app
        .user()
        .await?
        .clone()
        .ctrl()
        .switch_profile(RequestHeader {
            profile: profile.to_string(),
        })
        .await
        .wrap_err("failed to switch profile")?
        .into_inner();
    assert_response_valid(Some(res))?;
    println!("Successfully switched to profile '{profile}'!");
    Ok(())
}
