use std::time::Duration;

use ak_platform::{log::LogBuilder, net::server::creds::ProcCredentials, string::PlatformString};
use ak_platform_authz::{AuthorizeAction, Operation};

#[tokio::main]
async fn main() {
    LogBuilder::new(PlatformString::new())
        .force_stdout(true)
        .enable();
    let operation = match std::env::args().nth(1).as_deref() {
        None | Some("authorize") => Operation::Authorize,
        Some("whoami") => Operation::WhoAmI,
        Some("current-token") => Operation::CurrentToken,
        Some("token-exchange") => Operation::TokenExchange,
        Some("ssh-login") => Operation::SshLogin,
        Some(other) => {
            eprintln!("unknown operation {other}");
            std::process::exit(2);
        }
    };
    let creds = ProcCredentials::current();
    let res = AuthorizeAction::build(operation)
        .with_message(|_| Ok(PlatformString::new_with_default("authz prompt")))
        .with_uid(|_| Ok("static".to_string()))
        .with_success_timeout(Duration::from_hours(1))
        .with_denied_timeout(Duration::from_mins(5))
        .prompt(creds)
        .await;
    eprintln!("Authz result: {res:?}");
}
