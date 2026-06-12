use localauthentication::{prelude::*, async_api::AsyncContextExt};

use ak_platform::prelude::BoxError;

pub async fn prompt(msg: String) -> std::result::Result<bool, BoxError> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let result: std::result::Result<bool, BoxError> =
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| Box::from(e) as BoxError)
                .and_then(|rt| {
                    rt.block_on(async move {
                        let context = LAContext::new()?;
                        let policy = LAPolicy::DeviceOwnerAuthentication;
                        match context.can_evaluate_policy(policy) {
                            Ok(true) => (),
                            // if we can't evaluate the policy (which should never happen since we accept passcode too)
                            // we can't authorize this
                            Ok(false) => return Ok(false),
                            Err(e) => return Err(Box::from(e) as BoxError),
                        }
                        match context.evaluate_policy_async(policy, &msg)?.await {
                            Ok(b) => Ok(b),
                            Err(LAError::AppCancel(_) | LAError::BridgeFailed(_)) => Ok(false),
                            Err(e) => Err(Box::from(e) as BoxError),
                        }
                    })
                });
        let _ = tx.send(result);
    });

    rx.await
        .map_err(|e| BoxError::from(e.to_string()))?
}
