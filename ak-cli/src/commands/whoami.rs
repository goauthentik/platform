use std::error::Error;

use ak_platform::{
    generated::{
        agent::RequestHeader,
        agent_auth::WhoAmIRequest,
    },
    grpc::assert_response_valid,
};

use crate::{App, format};

pub async fn whoami(app: App) -> Result<(), Box<dyn Error>> {
    let res = app
        .clone()
        .user()
        .await?
        .auth()
        .who_am_i(WhoAmIRequest {
            header: Some(RequestHeader {
                profile: app.args.profile.clone(),
            }),
        })
        .await?
        .into_inner();
    assert_response_valid(res.header)?;
    format::render_json(res.body, "User Information", app.args.json)
}
