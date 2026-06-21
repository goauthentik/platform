use ak_macos_touchid::{AccessRequest, authenticate_with_touchid};
use ak_platform::prelude::BoxError;

use crate::sys::{AuthorizationRequest, macos::app::lookup_app_info};

pub mod app;

pub async fn prompt(req: AuthorizationRequest) -> Result<bool, BoxError> {
    let res = authenticate_with_touchid(AccessRequest {
        title: "authentik Access Request".to_string(),
        reason: req.msg.for_current(),
        ..lookup_app_info(req.proc_info).await?
    });
    Result::<bool, BoxError>::Ok(res)
}
