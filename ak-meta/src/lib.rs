use sentry::{ClientInitGuard, ClientOptions};
use std::{
    borrow::Cow,
    env,
    sync::atomic::{AtomicBool, Ordering},
};

/// Attribute macro that initializes Sentry before starting a multi-threaded tokio
/// runtime and running the annotated `async fn main`, similar to `#[tokio::main]`.
pub use ak_meta_macros::main;

pub fn version() -> String {
    let v = env!("AK_VERSION").to_string();
    if v.is_empty() {
        return "Next".to_string();
    }
    v
}

pub fn build_hash() -> String {
    env!("AK_BUILDHASH").to_string()
}

pub fn tag() -> String {
    env!("AK_TAG").to_string()
}

pub fn full_version() -> String {
    let mut fw = version();
    if !build_hash().is_empty() && tag().is_empty() {
        let mut sh = build_hash();
        sh.truncate(8);
        fw += "-";
        fw += &sh;
    }
    fw
}

pub fn user_agent() -> String {
    format!("goauthentik.io/platform/{}", full_version())
}

pub fn build_url() -> String {
    format!(
        "https://github.com/goauthentik/platform/commit/{}",
        build_hash()
    )
}

/// All components report as one release; `sentry_init` tags which one sent the event.
pub fn sentry_options() -> ClientOptions {
    let release: Cow<'static, str> = Cow::Owned(format!("ak-platform@{}", full_version()));
    ClientOptions::new()
        .dsn("https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512")
        .release(release)
        .attach_stacktrace(true)
        .send_default_pii(false)
        .debug(false)
        .before_send(|e| error_reporting().then_some(e))
        .traces_sampler(|_| if error_reporting() { 0.3 } else { 0.0 })
}

static ERROR_REPORTING: AtomicBool = AtomicBool::new(true);

/// Turns sending events and traces to Sentry on or off. Checked per event, so it can follow
/// configuration that is only loaded after `sentry_init`.
pub fn set_error_reporting(enabled: bool) {
    ERROR_REPORTING.store(enabled, Ordering::Relaxed);
}

fn error_reporting() -> bool {
    ERROR_REPORTING.load(Ordering::Relaxed)
}

/// Tags go on the scope rather than in `before_send` so transactions get them too.
/// Call before spawning threads: new threads copy the main scope when created.
pub fn sentry_init(component: &str, opts: ClientOptions) -> ClientInitGuard {
    let guard = sentry::init(opts);
    sentry::configure_scope(|s| s.set_tag("authentik.component", component));
    guard
}
