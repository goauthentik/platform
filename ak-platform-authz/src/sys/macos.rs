use ak_platform::prelude::*;
use localauthentication::{LAContext, LAError, LAPolicy, async_api::AsyncContextExt};

use ak_platform::string::PlatformString;

pub async fn prompt(msg: PlatformString) -> Result<bool> {
    let context = LAContext::new()?;

    let policy = LAPolicy::DeviceOwnerAuthentication;

    match context.can_evaluate_policy(policy) {
        Ok(true) => (),
        // if we can't evaluate the policy (which should never happen since we accept passcode too)
        // we can't authorize this
        Ok(false) => return Ok(false),
        Err(e) => return Err(Box::from(e)),
    }

    match context
        .evaluate_policy_async(policy, &msg.for_current())?
        .await
    {
        Ok(b) => Ok(b),
        Err(e) => match e {
            LAError::AppCancel(_) => Ok(false),
            LAError::BridgeFailed(_) => Ok(false),
            e => Err(Box::from(e)),
        },
    }
}
