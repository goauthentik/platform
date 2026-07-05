use eyre::{Error, Result, WrapErr};
use localauthentication::{async_api::AsyncContextExt, prelude::*};

pub async fn prompt(msg: String) -> Result<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let result: Result<bool> = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .wrap_err("failed to build tokio runtime")
            .and_then(|rt| {
                rt.block_on(async move {
                    let context = LAContext::new()
                        .wrap_err("failed to create LocalAuthentication context")?;
                    let policy = LAPolicy::DeviceOwnerAuthentication;
                    match context.can_evaluate_policy(policy) {
                        Ok(true) => (),
                        // if we can't evaluate the policy (which should never happen since we accept passcode too)
                        // we can't authorize this
                        Ok(false) => return Ok(false),
                        Err(e) => {
                            return Err(Error::from(e)
                                .wrap_err("LocalAuthentication: policy not evaluable"));
                        }
                    }
                    match context
                        .evaluate_policy_async(policy, &msg)
                        .wrap_err("failed to initiate policy evaluation")?
                        .await
                    {
                        Ok(b) => Ok(b),
                        Err(LAError::AppCancel(_) | LAError::BridgeFailed(_)) => Ok(false),
                        Err(e) => Err(Error::from(e)
                            .wrap_err("LocalAuthentication: policy evaluation failed")),
                    }
                })
            });
        let _ = tx.send(result);
    });

    rx.await.wrap_err("authentication thread did not respond")?
}
