pub mod sys;
use std::ops::Add;
use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use ak_platform::{net::server::creds::ProcCredentials, prelude::*, string::PlatformString};
use tonic::Status;

type MessageFn = Box<dyn (Fn(&ProcCredentials) -> Result<PlatformString>) + Send>;
type UidFn = Box<dyn (Fn(&ProcCredentials) -> Result<String>) + Send>;

pub struct AuthorizeAction {
    pub message: MessageFn,
    pub uid: UidFn,
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

impl AuthorizeAction {
    pub async fn prompt(self, creds: ProcCredentials) -> Result<bool> {
        let uid = (self.uid)(&creds)?.clone();
        log::trace!("Checking if we need to authorize: {uid}");
        if let Some(v) = match LAST_AUTH_MAP.try_lock() {
            Ok(it) => it,
            Err(e) => return Err(Box::from(e.to_string())),
        }
        .get(&uid)
            && v.exp >= Instant::now()
        {
            log::trace!("Valid last result in cache: {:?}", v.success);
            return Ok(v.success);
        }
        let msg = (self.message)(&creds)?.clone();
        log::trace!("Prompting for authz: {uid}");
        let res = sys::prompt(msg).await?;

        match LAST_AUTH_MAP.try_lock() {
            Ok(mut it) => {
                it.insert(
                    uid,
                    AuthState {
                        exp: Instant::now().add(self.timeout(res)),
                        success: res,
                    },
                );
            }
            Err(e) => return Err(Box::from(e.to_string())),
        }

        Ok(res)
    }

    pub async fn prompt_grpc(
        self,
        creds: Option<ProcCredentials>,
    ) -> std::result::Result<bool, Status> {
        let creds = match creds {
            Some(c) => c,
            None => return Err(Status::permission_denied("No credentials")),
        };
        match self.prompt(creds.clone()).await {
            Ok(r) => Ok(r),
            Err(e) => Err(Status::from_error(e)),
        }
    }
}
