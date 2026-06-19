use ak_platform::log::init_log;
use ak_platform::string::PlatformString;

use crate::path_handler::PathHandler;

mod handle_get_token;
mod handle_list_profiles;
mod handle_ping;
mod handle_platform_sign_endpoint_header;
mod models;
mod path_handler;

#[tokio::main]
async fn main() {
    init_log(
        PlatformString::new()
            .with_windows("authentik Browser Support")
            .with_linux("ak-browser-support")
            .with_darwin("io.goauthentik.platform.browser-support"),
    );
    let path_handler = match PathHandler::new().await {
        Ok(ph) => ph,
        Err(e) => {
            tracing::warn!("Failed to create path handler: {e:?}");
            return;
        }
    };

    match path_handler.start().await {
        Ok(_) => return,
        Err(e) => {
            tracing::warn!("Failed to start native messaging handler: {e:?}");
            return;
        }
    };
}
