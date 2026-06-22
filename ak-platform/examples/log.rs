use ak_platform::log::init_log;
use ak_platform::prelude::BoxError;
use ak_platform::string::PlatformString;

fn main() {
    init_log(
        PlatformString::new_with_default("log-example")
            .with_darwin("io.goauthentik.test")
            .with_linux("authentik-test")
            .with_windows("authentik Test"),
    );
    log::debug!("foo");
    tracing::debug!("foo");
    let e: BoxError = Box::from("my test error");
    tracing::warn!("tracing with an inline error: {e:?}");
    tracing::warn!(e, "tracing with field error");
}
