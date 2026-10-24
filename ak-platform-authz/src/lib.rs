use std::ops::Add;
use std::time::{Duration, Instant};

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use ak_platform::{net::server::creds::ProcCredentials, string::PlatformString};
use eyre::{self, Result, WrapErr, bail};
use tonic::Status;

type MessageFn = dyn (Fn(&ProcCredentials) -> Result<PlatformString>) + Send + Sync;
type UidFn = dyn (Fn(&ProcCredentials) -> Result<String>) + Send + Sync;

pub mod grpc;
pub mod sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    /// Read the user's account information.
    WhoAmI,
    /// Hand out the agent's own session token.
    CurrentToken,
    /// Hand out a token for another application.
    TokenExchange,
    /// Log in to an SSH host.
    SshLogin,
    /// A generic approval for a service the caller names.
    Authorize,
}

pub struct AuthorizeAction {
    operation: Operation,
    message: Box<MessageFn>,
    uid: Box<UidFn>,
    timeout_success: Duration,
    timeout_denied: Duration,
    creds: Option<ProcCredentials>,
    action: &'static str,
    target: String,
}

struct AuthState {
    exp: Instant,
    success: bool,
}

/// Earlier answers by operation and uid. An answer only stands in for prompts of its own
/// operation, since another operation's polkit action may have a stricter rule.
static LAST_AUTH_MAP: LazyLock<Mutex<HashMap<(Operation, String), AuthState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const HISTORY_SIZE: usize = 100;

static HISTORY: Mutex<VecDeque<AuthRecord>> = Mutex::new(VecDeque::new());

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthOutcome {
    Allowed,
    Denied,
    Failed,
}

impl From<bool> for AuthOutcome {
    fn from(success: bool) -> Self {
        if success { Self::Allowed } else { Self::Denied }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRecord {
    /// Milliseconds since the unix epoch.
    pub time: u64,
    pub action: &'static str,
    /// Application, service or host access was requested for.
    pub target: String,
    pub pid: i64,
    pub cmdline: Option<String>,
    pub parent_cmdline: Option<String>,
    pub outcome: AuthOutcome,
    /// Answered from an earlier prompt's result.
    pub cached: bool,
}

/// Recent authorization decisions of this process, newest first.
pub fn history() -> Vec<AuthRecord> {
    HISTORY
        .lock()
        .map(|h| h.iter().cloned().collect())
        .unwrap_or_default()
}

impl AuthorizeAction {
    pub fn build(operation: Operation) -> AuthorizeActionBuilder {
        AuthorizeActionBuilder {
            operation,
            message: None,
            uid: None,
            timeout_success: Duration::ZERO,
            timeout_denied: Duration::ZERO,
            creds: None,
            action: "",
            target: String::new(),
        }
    }

    pub fn timeout(&self, status: bool) -> Duration {
        match status {
            true => self.timeout_success,
            false => self.timeout_denied,
        }
    }

    #[tracing::instrument(skip(self), fields(operation = ?self.operation, uid))]
    pub async fn prompt(self, creds: ProcCredentials) -> Result<bool> {
        let uid = (self.uid)(&creds)
            .wrap_err("failed to resolve authorization UID")?
            .clone();
        tracing::Span::current().record("uid", &uid);
        tracing::trace!(uid, "Checking if we need to authorize");
        if let Some(success) = self.cached_answer(&uid)? {
            tracing::trace!(cached = success, "Valid last result in cache");
            self.record(&creds, success.into(), true);
            return Ok(success);
        }
        let msg = (self.message)(&creds)
            .wrap_err("failed to build authorization message")?
            .clone();
        tracing::trace!(uid, "Prompting for authz");
        let res = match sys::prompt(self.operation, msg).await {
            Ok(r) => r,
            Err(e) => {
                tracing::trace!("error during authz: {e:?}");
                self.record(&creds, AuthOutcome::Failed, false);
                return Err(e);
            }
        };

        self.remember_answer(uid, res)?;
        tracing::trace!(result = res, "Finished authorization");
        self.record(&creds, res.into(), false);
        Ok(res)
    }

    /// The still-valid answer an earlier prompt of this operation gave `uid`.
    fn cached_answer(&self, uid: &str) -> Result<Option<bool>> {
        let map = match LAST_AUTH_MAP.try_lock() {
            Ok(it) => it,
            Err(e) => bail!("auth cache lock poisoned: {e}"),
        };
        Ok(map
            .get(&(self.operation, uid.to_string()))
            .filter(|v| v.exp >= Instant::now())
            .map(|v| v.success))
    }

    fn remember_answer(&self, uid: String, success: bool) -> Result<()> {
        match LAST_AUTH_MAP.try_lock() {
            Ok(mut it) => {
                it.insert(
                    (self.operation, uid),
                    AuthState {
                        exp: Instant::now().add(self.timeout(success)),
                        success,
                    },
                );
                Ok(())
            }
            Err(e) => bail!("auth cache lock poisoned: {e}"),
        }
    }

    fn record(&self, creds: &ProcCredentials, outcome: AuthOutcome, cached: bool) {
        let info = creds.clone().proc_info().ok();
        let record = AuthRecord {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or_default(),
            action: self.action,
            target: self.target.clone(),
            pid: creds.pid(),
            cmdline: info.as_ref().map(|i| i.cmdline.clone()),
            parent_cmdline: info.and_then(|i| i.parent_cmdline().ok()),
            outcome,
            cached,
        };
        if let Ok(mut h) = HISTORY.lock() {
            h.truncate(HISTORY_SIZE - 1);
            h.push_front(record);
        }
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

pub struct AuthorizeActionBuilder {
    operation: Operation,
    message: Option<Box<MessageFn>>,
    uid: Option<Box<UidFn>>,
    timeout_success: Duration,
    timeout_denied: Duration,
    creds: Option<ProcCredentials>,
    action: &'static str,
    target: String,
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
    /// What is being authorized, shown in [`history`].
    pub fn with_target(mut self, action: &'static str, target: impl Into<String>) -> Self {
        self.action = action;
        self.target = target.into();
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
            operation: self.operation,
            message: m,
            uid: u,
            timeout_success: self.timeout_success,
            timeout_denied: self.timeout_denied,
            creds: self.creds,
            action: self.action,
            target: self.target,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_is_bounded_and_newest_first() -> Result<()> {
        let creds = ProcCredentials::current();
        for i in 0..=HISTORY_SIZE {
            AuthorizeAction::build(Operation::Authorize)
                .with_message(|_| Ok(PlatformString::new()))
                .with_uid(|_| Ok(String::new()))
                .with_target("test", i.to_string())
                .build()?
                .record(&creds, (i % 2 == 0).into(), false);
        }
        let h = history();
        assert_eq!(h.len(), HISTORY_SIZE);
        assert_eq!(h[0].target, HISTORY_SIZE.to_string());
        assert_eq!(h[0].pid, creds.pid());
        assert!(matches!(h[0].outcome, AuthOutcome::Allowed));
        Ok(())
    }

    #[tokio::test]
    async fn approval_only_answers_its_own_operation() -> Result<()> {
        let uid = "approval-only-answers-its-own-operation";
        let action = |operation| {
            AuthorizeAction::build(operation)
                // Only reached without a cached answer; failing keeps the test off the OS prompt.
                .with_message(|_| Err(eyre::eyre!("no cached answer")))
                .with_uid(move |_| Ok(uid.to_string()))
                .with_success_timeout(Duration::from_secs(60))
                .build()
        };
        action(Operation::Authorize)?.remember_answer(uid.to_string(), true)?;
        assert_eq!(
            action(Operation::Authorize)?.cached_answer(uid)?,
            Some(true)
        );
        let res = action(Operation::TokenExchange)?
            .prompt(ProcCredentials::current())
            .await;
        assert!(
            res.is_err(),
            "answered from the Authorize approval: {res:?}"
        );
        Ok(())
    }
}
