use crate::{App, format};
use eyre::{Result, WrapErr};
use ak_platform::{
    generated::{agent::RequestHeader, agent_auth::WhoAmIRequest},
    grpc::assert_response_valid,
};

pub async fn whoami(app: App) -> Result<()> {
    let res = app
        .clone()
        .user()
        .await?
        .auth()
        .who_am_i(WhoAmIRequest {
            header: Some(RequestHeader {
                profile: app.profile(),
            }),
        })
        .await
        .wrap_err("whoami RPC failed")?
        .into_inner();
    assert_response_valid(res.header).map_err(|e| eyre::eyre!("{e}"))?;
    format::render_json(res.body, "User Information", app.args.json)
}
