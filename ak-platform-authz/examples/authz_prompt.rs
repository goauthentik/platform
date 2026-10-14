use std::time::Duration;

use ak_platform::{log::LogBuilder, net::server::creds::ProcCredentials, string::PlatformString};
use ak_platform_authz::AuthorizeAction;
use eyre::{OptionExt, Result};

#[tokio::main]
async fn main() -> Result<()> {
    LogBuilder::new(PlatformString::new())
        .force_stdout(true)
        .enable();
    let parent = ProcCredentials::current()
        .proc_info()?
        .parent
        .ok_or_eyre("no parent process")?;
    let creds = ProcCredentials::new(Some(parent.pid as i64));
    let res = AuthorizeAction::build()
        .with_message(|_| Ok(PlatformString::new_with_default("authz prompt")))
        .with_uid(|_| Ok("static".to_string()))
        .with_success_timeout(Duration::from_hours(1))
        .with_denied_timeout(Duration::from_mins(5))
        .prompt(creds)
        .await;
    eprintln!("Authz result: {res:?}");
    Ok(())
}
