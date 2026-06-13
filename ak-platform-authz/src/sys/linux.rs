use ak_platform::prelude::*;

use polkit_rs::{self, CheckAuthorizationFlags, SystemBusName};

use ak_platform::string::PlatformString;

pub async fn prompt(_msg: PlatformString) -> Result<bool> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| -> BoxError { Box::from(e.to_string()) })
            .and_then(|rt| {
                rt.block_on(async move {
                    let auth = polkit_rs::Authority::get();
                    let subj = SystemBusName::new(":1.42");
                    auth.check_authorization_future(
                        &subj,
                        "io.goauthentik.platform.authorize",
                        None,
                        CheckAuthorizationFlags::ALLOW_USER_INTERACTION,
                    )
                    .await
                    .map(|r| r.is_authorized())
                    .map_err(|e| -> BoxError { Box::from(e.to_string()) })
                })
            });
        let _ = tx.send(result);
    });

    rx.await
        .map_err(|_| -> BoxError { Box::from("polkit thread dropped before sending result") })?
}
