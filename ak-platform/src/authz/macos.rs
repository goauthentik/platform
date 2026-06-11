use std::error::Error;

use localauthentication::prelude::*;

use crate::platform::string::PlatformString;

pub async fn prompt(msg: PlatformString) -> Result<bool, Box<dyn Error>> {
    let context = LAContext::new()?;

    let policy = LAPolicy::DeviceOwnerAuthentication;

    match context.can_evaluate_policy(policy) {
        Ok(true) => (),
        // if we can't evaluate the policy (which should never happen since we accept passcode too)
        // we can't authorize this
        Ok(false) => return Ok(false),
        Err(e) => return Err(Box::from(e)),
    }

    match context.evaluate_policy(policy, &msg.for_current()) {
        Ok(b) => Ok(b),
        Err(e) => Err(Box::from(e)),
    }
}
