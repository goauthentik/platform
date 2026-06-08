use ak_platform::log::init_log;
use ak_platform::platform::string::PlatformString;

fn main() {
    init_log(
        PlatformString::new_with_default("log-example")
            .with_darwin("io.goauthentik.test")
            .with_linux("authentik-test")
            .with_windows("authentik Test"),
    );
    log::debug!("foo");
}
