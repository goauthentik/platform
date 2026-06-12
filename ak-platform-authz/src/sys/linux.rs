use ak_platform::prelude::*;

use polkit_rs::{self, CheckAuthorizationFlags, SystemBusName};

use ak_platform::string::PlatformString;

pub async fn prompt(_msg: PlatformString) -> Result<bool> {
    let auth = polkit_rs::Authority::get();
    let subj = SystemBusName::new(":1.42");
    let result = auth
        .check_authorization_future(
            &subj,
            "io.goauthentik.platform.authorize",
            None,
            CheckAuthorizationFlags::ALLOW_USER_INTERACTION,
        )
        .await?;
    Ok(result.is_authorized())
}
