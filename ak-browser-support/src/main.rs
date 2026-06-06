use akp_logger::init_log;
use native_messaging::host::NmError;

use crate::path_handler::PathHandler;

mod handle_get_token;
mod handle_list_profiles;
mod handle_ping;
mod handle_platform_sign_endpoint_header;
mod models;
mod path_handler;

#[tokio::main]
async fn main() -> Result<(), NmError> {
    init_log("ak-browser-support");
    let path_handler = match PathHandler::new().await {
        Ok(ph) => ph,
        Err(e) => {
            log::warn!("Failed to create path handler: {e:?}");
            return Err(NmError::Disconnected);
        }
    };

    path_handler.start().await
}
