use tauri::{AppHandle, Emitter, Manager};
use url::Url;

use crate::{cmd::page::PendingPage, ui};

pub const SCHEME: &str = "io.goauthentik.platform:";

/// Route an `io.goauthentik.platform://<route>` URL.
pub fn handle(app: &AppHandle, url: &Url) {
    match url.host_str() {
        Some("open") => open(app, url),
        _ => tracing::warn!("unknown deep link: {url}"),
    }
}

/// `open?page=<page>`: show the main window, optionally on `page`.
fn open(app: &AppHandle, url: &Url) {
    if let Some((_, page)) = url.query_pairs().find(|(k, _)| k == "page")
        && let Ok(mut pending) = app.state::<PendingPage>().0.lock()
    {
        *pending = Some(page.into_owned());
    }
    // A window that's still loading takes the page on startup instead.
    if let Err(e) = app.emit("ak-navigate", ()) {
        tracing::warn!("failed to emit navigate event: {e}");
    }
    ui::show_main(app);
}
