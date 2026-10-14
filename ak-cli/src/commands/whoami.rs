use crate::{App, format};
use ak_platform::{
    generated::{agent::RequestHeader, agent_auth::WhoAmIRequest},
    grpc::assert_response_valid,
};
use eyre::{Result, WrapErr};

pub async fn whoami(mut app: App) -> Result<()> {
    let res = app
        .clone()
        .user()
        .await?
        .auth()
        .who_am_i(WhoAmIRequest {
            header: Some(RequestHeader {
                profile: app.profile().await,
            }),
        })
        .await
        .wrap_err("whoami RPC failed")?
        .into_inner();
    assert_response_valid(res.header)?;
    format::render_json(res.body, "User Information", app.args.json)
}
