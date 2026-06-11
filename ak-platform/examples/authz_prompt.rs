use ak_platform::{authz::prompt, log::init_log_interactive, platform::string::PlatformString};

#[tokio::main]
async fn main() {
    init_log_interactive();
    let res = prompt(PlatformString::new_with_default("authz prompt")).await;
    eprintln!("Authz result: {res:?}");
}
