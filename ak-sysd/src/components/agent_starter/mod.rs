use crate::components::session::SessionOpened;
use crate::components::{Component, SysdContext};
use eyre::Result;
use std::time::Duration;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod win;

#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(windows)]
use win as platform;

const RETRY_ATTEMPTS: u32 = 10;
const RETRY_DELAY: Duration = Duration::from_secs(3);

pub struct AgentStarterComponent {
    ctx: SysdContext,
}

impl AgentStarterComponent {
    pub fn new(ctx: SysdContext) -> AgentStarterComponent {
        let hctx = ctx.clone();
        ctx.events.on(move |_: SessionOpened| {
            let ctx = hctx.clone();
            async move { start(ctx).await }
        });
        AgentStarterComponent { ctx }
    }
}

#[tonic::async_trait]
impl Component for AgentStarterComponent {
    fn id() -> &'static str {
        "agent_starter"
    }

    async fn start(&self) -> Result<()> {
        tokio::spawn(start(self.ctx.clone()));
        Ok(())
    }
}

/// Starts the agent, retrying failed launches with a fixed delay. Not finding
/// a GUI user is not a failure; the next `SessionOpened` triggers a new attempt.
async fn start(ctx: SysdContext) {
    let Some(exec_path) = agent_exec_path() else {
        return;
    };
    if !std::path::Path::new(exec_path).exists() {
        return;
    }
    for attempt in 0..RETRY_ATTEMPTS {
        let debug = ctx.cfg.read().await.debug;
        let result = tokio::task::spawn_blocking(move || start_once(exec_path, debug)).await;
        let err = match result {
            Ok(Ok(())) => return,
            Ok(Err(e)) => e,
            Err(e) => e.into(),
        };
        tracing::warn!(attempt, "failed to start agent: {err:?}");
        if attempt + 1 == RETRY_ATTEMPTS {
            return;
        }
        tokio::select! {
            _ = ctx.cancel.cancelled() => return,
            _ = tokio::time::sleep(RETRY_DELAY) => {}
        }
    }
}

fn start_once(exec_path: &str, debug: bool) -> Result<()> {
    let user = match platform::logged_in_via_gui() {
        Ok(Some(user)) => user,
        Ok(None) => {
            tracing::debug!("No GUI user found, skipping ak-agent start");
            return Ok(());
        }
        Err(e) => {
            tracing::debug!("failed to get GUI user: {e:?}");
            return Ok(());
        }
    };
    tracing::debug!(user, "Found GUI user, attempting ak-agent start");
    let mut env = vec![("AK_AGENT_SUPERVISED", "true")];
    if debug {
        env.push(("AK_AGENT_DEBUG", "true"));
    }
    platform::run(exec_path, &user, &env)
}

fn agent_exec_path() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    return Some("/Applications/authentik Agent.app");
    #[cfg(target_os = "linux")]
    return Some("/usr/bin/ak-agent-desktop");
    #[cfg(target_os = "windows")]
    return Some(r"C:\Program Files\Authentik Security Inc\agent\ak-agent-desktop.exe");
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    None
}

#[cfg(unix)]
fn command_stdout(cmd: &mut std::process::Command) -> Result<String> {
    let out = cmd.output()?;
    if !out.status.success() {
        eyre::bail!("{cmd:?} failed: {}", out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
