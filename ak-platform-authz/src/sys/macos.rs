use ak_macos_touchid::{AccessRequest, authenticate_with_touchid};
use ak_platform::prelude::BoxError;

pub async fn prompt(msg: String) -> std::result::Result<bool, BoxError> {
    let res = authenticate_with_touchid(AccessRequest {
        title: msg,
        // requesting_app: todo!(),
        // profile_name: todo!(),
        // profile_email: todo!(),
        // profile_username: todo!(),
        // profile_groups: todo!(),
        // profile_avatar: todo!(),
        // reason: todo!(),
        ..AccessRequest::default()
    });
    Result::<bool, BoxError>::Ok(res)
    // let context = LAContext::new()?;
    // let policy = LAPolicy::DeviceOwnerAuthentication;
    // match context.can_evaluate_policy(policy) {
    //     Ok(true) => (),
    //     // if we can't evaluate the policy (which should never happen since we accept passcode too)
    //     // we can't authorize this
    //     Ok(false) => return Ok(false),
    //     Err(e) => return Err(Box::from(e) as BoxError),
    // }
    // match context.evaluate_policy_async(policy, &msg)?.await {
    //     Ok(b) => Ok(b),
    //     Err(LAError::AppCancel(_) | LAError::BridgeFailed(_)) => Ok(false),
    //     Err(e) => Err(Box::from(e) as BoxError),
    // }
}
