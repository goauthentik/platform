use crate::components::{Component, SysdContext};
use crate::events::SysdEvent;
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;

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

    fn register(self: Arc<Self>, _socket: SysdSocketID, _routes: &mut tonic::service::RoutesBuilder) {
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
    use eyre::{Result, bail};

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
        bail!(
            "GUI-logged-in user detection not yet implemented on Windows — \
             needs WTSEnumerateSessions via a new windows-sys dependency"
        );
    }
}

mod exec_as_user {
    use eyre::Result;

    #[cfg(unix)]
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

    /// On macOS, `path` may be a `.app` bundle directory, which can't be
    /// exec'd directly — resolve it to the binary named by the bundle's
    /// `Info.plist` `CFBundleExecutable` key.
    ///
    /// `Contents/MacOS/` is not guaranteed to hold a single executable: this
    /// workspace's own `authentik Agent.app` bundle packages several sibling
    /// binaries (`ak`, `ak-agent-desktop`, `ak-browser-support`, `ak-sysd`)
    /// under the same directory, so picking "whichever file `read_dir` finds
    /// first" is unreliable — verified live on a real build, where it picked
    /// `ak-sysd` itself instead of `ak-agent-desktop`.
    #[cfg(target_os = "macos")]
    fn resolve_executable(path: &str) -> Result<String> {
        use eyre::bail;
        if !path.ends_with(".app") {
            return Ok(path.to_string());
        }
        let info_plist = std::path::Path::new(path).join("Contents/Info");
        let out = std::process::Command::new("defaults")
            .arg("read")
            .arg(&info_plist)
            .arg("CFBundleExecutable")
            .output()?;
        if !out.status.success() {
            bail!("failed to read CFBundleExecutable from {}", info_plist.display());
        }
        let exe_name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if exe_name.is_empty() {
            bail!("CFBundleExecutable is empty in {}", info_plist.display());
        }
        Ok(std::path::Path::new(path)
            .join("Contents/MacOS")
            .join(exe_name)
            .to_string_lossy()
            .to_string())
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    fn resolve_executable(path: &str) -> Result<String> {
        Ok(path.to_string())
    }

    #[cfg(windows)]
    pub fn run(_path: &str, _user: &str, _debug: bool) -> Result<()> {
        eyre::bail!(
            "exec-as-user not yet implemented on Windows — needs WTSQueryUserToken + \
             CreateProcessAsUserW via a new windows-sys dependency"
        );
    }
}
