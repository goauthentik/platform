use std::time::Duration;

use ak_platform::{
    log::init_log_interactive, net::server::creds::ProcCredentials, prelude::BoxError,
    string::PlatformString,
};
use ak_platform_authz::AuthorizeAction;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    init_log_interactive();
    let creds = ProcCredentials::new(Some(ProcCredentials::current().proc_info()?.parent.unwrap().pid as i64));
    let res = AuthorizeAction {
        message: Box::new(|_| Ok(PlatformString::new_with_default("authz prompt"))),
        uid: Box::new(|_| Ok("static".to_string())),
        timeout_success: Duration::from_hours(1),
        timeout_denied: Duration::from_mins(5),
    }
    .prompt(creds)
    .await;
    eprintln!("Authz result: {res:?}");
    Ok(())
}
