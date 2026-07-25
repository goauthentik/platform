use std::sync::{Arc, Mutex};

use ak_flow_executor::CookieStoreMutex;
use eyre::Result;
use reqwest::{Client, Url, redirect};

/// Max redirect hops before giving up. Custom policies bypass reqwest's
/// built-in cap, so we enforce our own.
const MAX_HOPS: usize = 10;

/// Builds a client sharing the flow executor's cookie jar that captures the
/// terminal `goauthentik.io://platform/finished` redirect instead of following
/// it — that scheme has no network handler, and the auth token rides in its
/// query string. The captured URL lands in the returned slot.
pub fn build_finish_client(jar: Arc<CookieStoreMutex>) -> Result<(Client, Arc<Mutex<Option<Url>>>)> {
    let captured: Arc<Mutex<Option<Url>>> = Arc::new(Mutex::new(None));
    let slot = Arc::clone(&captured);

    let policy = redirect::Policy::custom(move |attempt| {
        if attempt.url().scheme() == "goauthentik.io" {
            if let Ok(mut slot) = slot.lock() {
                *slot = Some(attempt.url().clone());
            }
            return attempt.stop();
        }
        if attempt.previous().len() >= MAX_HOPS {
            return attempt.stop();
        }
        attempt.follow()
    });

    let client = Client::builder().cookie_provider(jar).redirect(policy).build()?;
    Ok((client, captured))
}
