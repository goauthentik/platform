use ak_platform::{authz::prompt, platform::string::PlatformString};

#[tokio::main]
async fn main() {
    let res = prompt(PlatformString::new_with_default("authz prompt")).await;
    eprintln!("Authz result: {res:?}");
}
