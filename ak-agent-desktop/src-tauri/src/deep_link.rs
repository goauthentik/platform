use tauri::AppHandle;
use url::Url;

use crate::ui;

pub const SCHEME: &str = "io.goauthentik.platform:";

/// Route an `io.goauthentik.platform://<route>` URL.
pub fn handle(app: &AppHandle, url: &Url) {
    match url.host_str() {
        Some("open") => ui::show_main(app),
        _ => tracing::warn!("unknown deep link: {url}"),
    }
}
