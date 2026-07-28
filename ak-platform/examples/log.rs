use ak_platform::log::LogBuilder;
use ak_platform::string::PlatformString;

fn main() {
    LogBuilder::new(
        PlatformString::new_with_default("log-example")
            .with_darwin("io.goauthentik.test")
            .with_linux("authentik-test")
            .with_windows("authentik Test"),
    )
    .enable();
    log::debug!("foo");
    tracing::debug!("foo");
    let e = eyre::eyre!("my test error");
    tracing::warn!("tracing with an inline error: {e:?}");
    tracing::warn!(error = ?e, "tracing with field error");
}
