pub mod sys;
use std::ops::Add;
use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use ak_platform::{net::server::creds::ProcCredentials, prelude::*, string::PlatformString};

pub struct AuthorizeAction {
    pub message: fn(parent: ProcCredentials) -> Result<PlatformString>,
    pub uid: fn(parent: ProcCredentials) -> Result<String>,
    pub timeout_success: Duration,
    pub timeout_denied: Duration,
}

impl AuthorizeAction {
    pub fn timeout(self, status: bool) -> Duration {
        match status {
            true => self.timeout_success,
            false => self.timeout_denied,
        }
    }
}

struct AuthState {
    exp: Instant,
    success: bool,
}

static LAST_AUTH_MAP: LazyLock<Mutex<HashMap<String, AuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn prompt(action: AuthorizeAction, creds: ProcCredentials) -> Result<bool> {
    let uid = (action.uid)(creds.clone())?;
    log::trace!("Checking if we need to authorize: {uid}");
    match match LAST_AUTH_MAP.try_lock() {
        Ok(it) => it,
        Err(e) => return Err(Box::from(e.to_string())),
    }
    .get(&uid)
    {
        Some(v) => {
            if v.exp >= Instant::now() {
                log::trace!("Valid last result in cache: {:?}", v.success);
                return Ok(v.success);
            }
        }
        None => {}
    }
    let msg = (action.message)(creds)?;
    log::trace!("Prompting for authz: {uid}");
    let res = sys::prompt(msg).await?;

    match LAST_AUTH_MAP.try_lock() {
        Ok(mut it) => {
            it.insert(
                uid,
                AuthState {
                    exp: Instant::now().add(action.timeout(res)),
                    success: res,
                },
            );
        }
        Err(e) => return Err(Box::from(e.to_string())),
    }

    Ok(res)
}
