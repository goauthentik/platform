use ak_meta::user_agent;
use ak_platform::generated::{
    agent::RequestHeader,
    agent_auth::{CurrentTokenRequest, current_token_request::Type},
};
use authentik_client::apis::configuration::Configuration;
use eyre::{Result, WrapErr};

pub use ak_api_cli::ApiCommand;

pub async fn exec_api_command(mut app: super::App, cmd: &ApiCommand) -> Result<()> {
    let profile = app.profile().await;
    let res = app
        .user()
        .await?
        .clone()
        .auth()
        .get_current_token(CurrentTokenRequest {
            header: Some(RequestHeader { profile }),
            r#type: Type::Verified as i32,
        })
        .await
        .wrap_err("failed to get API access token")?
        .into_inner();

    let config = Configuration {
        base_path: format!("{}/api/v3", res.url),
        bearer_access_token: Some(res.raw),
        user_agent: Some(user_agent()),
        ..Default::default()
    };
    cmd.execute(&config).await.wrap_err("API command failed")
}
