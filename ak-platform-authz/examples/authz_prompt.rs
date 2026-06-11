use ak_platform::{log::init_log_interactive, string::PlatformString};
use ak_platform_authz::prompt;

#[tokio::main]
async fn main() {
    init_log_interactive();
    let res = prompt(PlatformString::new_with_default("authz prompt")).await;
    eprintln!("Authz result: {res:?}");
}
