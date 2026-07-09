use sentry::{ClientOptions, types::Dsn};
use std::{borrow::Cow, env, str::FromStr};

/// Attribute macro that initializes Sentry before starting a multi-threaded tokio
/// runtime and running the annotated `async fn main`, similar to `#[tokio::main]`.
pub use ak_meta_macros::main;

pub fn version() -> String {
    env!("AK_VERSION").to_string()
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

pub fn sentry_options<T: ToString>(name: T) -> ClientOptions {
    let release: Cow<'static, str> = Cow::Owned(format!("{}@{}", name.to_string(), full_version()));
    ClientOptions {
        dsn: Some(Dsn::from_str("https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512").expect("Static value")),
        release: Some(release),
        send_default_pii: false,
        traces_sample_rate: 0.3,
        debug: true,
        ..Default::default()
    }
}
