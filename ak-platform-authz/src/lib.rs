use std::ops::Add;
use std::time::{Duration, Instant};

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use ak_platform::{net::server::creds::ProcCredentials, string::PlatformString};
use eyre::{self, Result, WrapErr, bail};
use tonic::Status;

type MessageFn = dyn (Fn(&ProcCredentials) -> Result<PlatformString>) + Send + Sync;
type UidFn = dyn (Fn(&ProcCredentials) -> Result<String>) + Send + Sync;

pub mod grpc;
pub mod sys;

pub struct AuthorizeAction {
    message: Box<MessageFn>,
    uid: Box<UidFn>,
    timeout_success: Duration,
    timeout_denied: Duration,
    creds: Option<ProcCredentials>,
}

struct AuthState {
    exp: Instant,
    success: bool,
}

static LAST_AUTH_MAP: LazyLock<Mutex<HashMap<String, AuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

impl AuthorizeAction {
    pub fn build() -> AuthorizeActionBuilder {
        AuthorizeActionBuilder::default()
    }

    pub fn timeout(&self, status: bool) -> Duration {
        match status {
            true => self.timeout_success,
            false => self.timeout_denied,
        }
    }

    #[tracing::instrument(skip(self), fields(uid))]
    pub async fn prompt(self, creds: ProcCredentials) -> Result<bool> {
        let uid = (self.uid)(&creds)
            .wrap_err("failed to resolve authorization UID")?
            .clone();
        tracing::Span::current().record("uid", &uid);
        tracing::trace!(uid, "Checking if we need to authorize");
        if let Some(v) = match LAST_AUTH_MAP.try_lock() {
            Ok(it) => it,
            Err(e) => bail!("auth cache lock poisoned: {e}"),
        }
        .get(&uid)
            && v.exp >= Instant::now()
        {
            tracing::trace!(cached = v.success, "Valid last result in cache");
            return Ok(v.success);
        }
        let msg = (self.message)(&creds)
            .wrap_err("failed to build authorization message")?
            .clone();
        tracing::trace!(uid, "Prompting for authz");
        let res = match sys::prompt(msg).await {
            Ok(r) => r,
            Err(e) => {
                tracing::trace!("error during authz: {e:?}");
                return Err(e);
            }
        };

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
            Err(e) => bail!("auth cache lock poisoned: {e}"),
        }
        tracing::trace!(result = res, "Finished authorization");
        Ok(res)
    }

    pub async fn prompt_grpc(self) -> std::result::Result<(), Status> {
        let creds = match self.creds.clone() {
            Some(c) => c,
            None => return Err(Status::permission_denied("No credentials")),
        };
        match self.prompt(creds).await {
            Ok(r) => match r {
                true => Ok(()),
                false => Err(Status::permission_denied("user denied")),
            },
            Err(e) => Err(Status::from_error(e.into())),
        }
    }
}

#[derive(Default)]
pub struct AuthorizeActionBuilder {
    message: Option<Box<MessageFn>>,
    uid: Option<Box<UidFn>>,
    timeout_success: Duration,
    timeout_denied: Duration,
    creds: Option<ProcCredentials>,
}

impl AuthorizeActionBuilder {
    pub fn with_message<F>(mut self, msg: F) -> Self
    where
        F: Fn(&ProcCredentials) -> Result<PlatformString> + Send + Sync + 'static,
    {
        self.message = Some(Box::new(msg));
        self
    }
    pub fn with_uid<F>(mut self, uid: F) -> Self
    where
        F: Fn(&ProcCredentials) -> Result<String> + Send + Sync + 'static,
    {
        self.uid = Some(Box::new(uid));
        self
    }
    pub fn with_success_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_success = timeout;
        self
    }
    pub fn with_denied_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_denied = timeout;
        self
    }
    pub fn with_creds(mut self, creds: Option<ProcCredentials>) -> Self {
        self.creds = creds;
        self
    }
    pub fn build(self) -> Result<AuthorizeAction> {
        let Some(m) = self.message else {
            bail!("Missing message function");
        };
        let Some(u) = self.uid else {
            bail!("Missing uid function");
        };
        Ok(AuthorizeAction {
            message: m,
            uid: u,
            timeout_success: self.timeout_success,
            timeout_denied: self.timeout_denied,
            creds: self.creds,
        })
    }

    pub async fn prompt(self, creds: ProcCredentials) -> Result<bool> {
        self.build()?.prompt(creds).await
    }

    pub async fn finish(self) -> std::result::Result<(), Status> {
        self.build()
            .map_err(|e| Status::from_error(e.into()))?
            .prompt_grpc()
            .await
    }
}
