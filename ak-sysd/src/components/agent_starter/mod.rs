use crate::components::{Component, SysdContext};
use crate::events::SysdEvent;
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;

#[cfg(windows)]
mod win;

pub struct AgentStarterComponent {
    ctx: SysdContext,
}

impl AgentStarterComponent {
    pub fn new(ctx: SysdContext) -> AgentStarterComponent {
        AgentStarterComponent { ctx }
    }
}

#[tonic::async_trait]
impl Component for AgentStarterComponent {
    fn id() -> &'static str {
        "agent_starter"
    }

    async fn start(&self) -> Result<()> {
        let ctx = self.ctx.clone();
        tokio::spawn(async move {
            if let Err(e) = try_start(&ctx).await {
                tracing::debug!("agent_starter initial start attempt: {e:?}");
            }

            let mut rx = ctx.events.subscribe();
            loop {
                tokio::select! {
                    ev = rx.recv() => {
                        if let Ok(SysdEvent::SessionOpened { .. }) = ev
                            && let Err(e) = try_start(&ctx).await {
                                tracing::warn!("failed to start desktop agent: {e:?}");
                        }
                    }
                    _ = ctx.cancel.cancelled() => return,
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn register(
        self: Arc<Self>,
        _socket: SysdSocketID,
        _routes: &mut tonic::service::RoutesBuilder,
    ) {
        // No gRPC surface of its own.
    }
}

async fn try_start(ctx: &SysdContext) -> Result<()> {
    let debug = ctx.cfg.read().await.debug;
    let user = gui_user::current_gui_user()?;
    let exec_path = agent_exec_path()?;
    exec_as_user::run(&exec_path, &user, debug)
}

/// Per-OS path to the desktop agent binary. macOS execs the `.app` bundle's
/// actual executable (resolved at spawn time, see `exec_as_user`), matching
/// Go's use of `execuser.Run` against the bundle path.
fn agent_exec_path() -> Result<String> {
    #[cfg(target_os = "macos")]
    return Ok("/Applications/authentik Agent.app".to_string());
    #[cfg(target_os = "linux")]
    return Ok("/usr/bin/ak-agent".to_string());
    #[cfg(target_os = "windows")]
    return Ok(r"C:\Program Files\Authentik Security Inc\agent\ak-agent.exe".to_string());
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    eyre::bail!("unsupported platform")
}

mod gui_user {
    use eyre::Result;
    #[cfg(unix)]
    use eyre::bail;

    /// Finds the currently GUI-logged-in user. No existing helper anywhere
    /// in the workspace — this is new platform code on every OS.
    #[cfg(target_os = "macos")]
    pub fn current_gui_user() -> Result<String> {
        let out = std::process::Command::new("stat")
            .args(["-f", "%Su", "/dev/console"])
            .output()?;
        let user = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if user.is_empty() || user == "root" {
            bail!("no GUI-logged-in user found");
        }
        Ok(user)
    }

    /// Best-effort: parses `who` output for a session attached to a
    /// display (`:N`), matching the common desktop-launcher heuristic.
    /// `loginctl list-sessions` would be more precise but its exact output
    /// shape wasn't verified against a running system in this pass.
    #[cfg(target_os = "linux")]
    pub fn current_gui_user() -> Result<String> {
        let out = std::process::Command::new("who").output()?;
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines() {
            if line.contains('(') && line.contains(':') {
                if let Some(user) = line.split_whitespace().next() {
                    return Ok(user.to_string());
                }
            }
        }
        bail!("no GUI-logged-in user found")
    }

    #[cfg(target_os = "windows")]
    pub fn current_gui_user() -> Result<String> {
        let session = super::win::active_session_id()?;
        super::win::session_username(session)
    }
}

mod exec_as_user {
    use eyre::Result;

    /// macOS: hand the launch to the *user's* launchd domain via
    /// `launchctl asuser <uid> open -a`, instead of forking from this daemon
    /// and dropping privileges with `Command::uid()`.
    ///
    /// A plain fork/exec sets the right uid but leaves the child in sysd's
    /// **system** launchd domain, with no Aqua session attached. Such a process
    /// has no login keychain, so securityd falls back to
    /// `/Library/Keychains/System.keychain` (it logs "Enabling System Keychain
    /// Always due to platform"). The agent doesn't run as root, that write is
    /// denied, and every credential save fails with errSecWrPerm — surfacing to
    /// the user as `ak config setup` dying with "Write permissions error".
    ///
    /// `launchctl asuser` puts the agent in the GUI session's domain, where the
    /// login keychain is available. `open -a` also handles the `.app` bundle
    /// natively, so the bundle path needs no resolving.
    #[cfg(target_os = "macos")]
    pub fn run(path: &str, user: &str, debug: bool) -> Result<()> {
        use eyre::bail;
        use std::process::Command;

        let uid = shell_id(user, "-u")?;

        let mut cmd = Command::new("launchctl");
        cmd.args(["asuser", &uid.to_string(), "open", "-a", path]);
        cmd.args(["--env", "AK_AGENT_SUPERVISED=true"]);
        if debug {
            cmd.args(["--env", "AK_AGENT_DEBUG=true"]);
        }

        let status = cmd.status()?;
        if !status.success() {
            bail!("failed to launch agent via `launchctl asuser`: {status}");
        }
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn run(path: &str, user: &str, debug: bool) -> Result<()> {
        use std::process::Command;

        let resolved = resolve_executable(path)?;
        let uid = shell_id(user, "-u")?;
        let gid = shell_id(user, "-g")?;

        use std::os::unix::process::CommandExt;
        let mut cmd = Command::new(resolved);
        cmd.uid(uid).gid(gid);
        cmd.env("AK_AGENT_SUPERVISED", "true");
        if debug {
            cmd.env("AK_AGENT_DEBUG", "true");
        }
        cmd.spawn()?;
        Ok(())
    }

    #[cfg(unix)]
    fn shell_id(user: &str, flag: &str) -> Result<u32> {
        let out = std::process::Command::new("id")
            .args([flag, user])
            .output()?;
        Ok(String::from_utf8_lossy(&out.stdout).trim().parse()?)
    }

    // No macOS `resolve_executable`: `open -a` launches the `.app` bundle
    // directly, so the bundle path never has to be resolved to the inner
    // `CFBundleExecutable` binary.

    #[cfg(all(unix, not(target_os = "macos")))]
    fn resolve_executable(path: &str) -> Result<String> {
        Ok(path.to_string())
    }

    #[cfg(windows)]
    pub fn run(path: &str, _user: &str, debug: bool) -> Result<()> {
        let session = super::win::active_session_id()?;
        super::win::spawn_as_session(path, session, debug)
    }
}
