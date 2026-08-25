use crate::string::PlatformString;
use dirs_next::{config_dir, data_dir};
use eyre::{Result, bail};
use std::env;

pub const DEFAULT_PROFILE: &str = "default";

pub enum SysdSocketID {
    Default,
    CTRL,
}

pub fn sysd_config_file() -> PlatformString {
    PlatformString::new()
        .with_darwin("/opt/authentik/config/config.json")
        .with_linux("/etc/authentik/config.json")
        .with_windows(r"C:\Program Files\Authentik Security Inc\sysd\config.json")
}

pub fn sysd_state_file() -> PlatformString {
    PlatformString::new()
        .with_darwin("/opt/authentik/sysd-state-v2.db")
        .with_linux("/var/lib/authentik/sysd-state-v2.db")
        .with_windows(r"C:\ProgramData\Authentik Security Inc\sysd-state-v2.db")
}

pub fn sysd_socket_path(id: SysdSocketID) -> PlatformString {
    match id {
        SysdSocketID::CTRL => PlatformString::new()
            .with_windows(r"\\.\pipe\authentik\sysd-ctrl")
            .with_darwin("/var/run/authentik-sysd-ctrl.sock")
            .with_linux("/var/run/authentik/sys-ctrl.sock"),
        SysdSocketID::Default => PlatformString::new()
            .with_windows(r"\\.\pipe\authentik\sysd")
            .with_darwin("/var/run/authentik-sysd.sock")
            .with_linux("/var/run/authentik/sys.sock"),
    }
}

/// Mach service name the macOS CTRL relay daemon advertises, and the label of
/// the `SMAppService` LaunchDaemon plist that declares it. Shared by the
/// client (`net::elevate::macos`) and the daemon (`ak-sysd-ctrl-relay`); the
/// plist in `vpkg/macos/scripts/sysd-ctrl-relay-daemon.plist` is the one
/// remaining copy and must match.
pub const SYSD_CTRL_RELAY_MACH_SERVICE: &str = "io.goauthentik.platform.sysd-ctrl-relay";

/// Where packaging installs the elevated CTRL relay helper. Kept here rather
/// than in `net::elevate` so the three install locations stay next to the
/// other packaged paths — `vpkg/linux/agent-desktop/nfpm.yaml`,
/// `vpkg/windows/Package.wxs` and `vpkg/macos/Makefile` must agree with this.
/// On Linux the value is also the `org.freedesktop.policykit.exec.path` in
/// `io.goauthentik.platform.policy`, which pkexec matches on exactly.
pub fn sysd_ctrl_relay_path() -> PlatformString {
    PlatformString::new()
        .with_darwin("/Applications/authentik Agent.app/Contents/MacOS/ak-sysd-ctrl-relay")
        .with_linux("/usr/bin/ak-sysd-ctrl-relay")
        .with_windows(r"C:\Program Files\Authentik Security Inc\sysd\ak-sysd-ctrl-relay.exe")
}

pub enum AgentSocketID {
    Default,
    SSH,
}

fn xdg_data_path(last_seg: &str) -> Result<String> {
    let mut data = match data_dir() {
        Some(d) => d,
        None => bail!("Failed to get XDG data path"),
    };
    data.push("authentik");
    data.push(last_seg);
    match data.as_path().to_str() {
        Some(p) => Ok(p.to_string()),
        None => bail!("Failed to convert path to string"),
    }
}

pub fn xdg_config_path(last_seg: &str) -> Result<String> {
    let mut data = match config_dir() {
        Some(d) => d,
        None => bail!("Failed to get XDG data path"),
    };
    data.push("authentik");
    data.push(last_seg);
    match data.as_path().to_str() {
        Some(p) => Ok(p.to_string()),
        None => bail!("Failed to convert path to string"),
    }
}

pub fn agent_socket_path(id: AgentSocketID) -> Result<PlatformString> {
    match id {
        AgentSocketID::Default => {
            if let Ok(x) = env::var("AUTHENTIK_CLI_SOCKET") {
                return Ok(PlatformString::new_with_default(&x));
            }
            Ok(PlatformString::new()
                .with_windows(r"\\.\pipe\authentik\socket")
                .with_linux(&xdg_data_path("agent.sock")?))
        }
        AgentSocketID::SSH => Ok(PlatformString::new()
            .with_windows(r"\\.\pipe\authentik\socket-ssh")
            .with_linux(&xdg_data_path("agent-ssh.sock")?)),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn test_agent_default_macos() {
        let binding = env::home_dir().unwrap();
        let home = binding.to_str().unwrap();
        assert_eq!(
            agent_socket_path(AgentSocketID::Default)
                .unwrap()
                .for_platform("macos"),
            format!("{}/Library/Application Support/authentik/agent.sock", home)
        )
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_agent_default_linux() {
        let binding = env::home_dir().unwrap();
        let home = binding.to_str().unwrap();
        assert_eq!(
            agent_socket_path(AgentSocketID::Default)
                .unwrap()
                .for_platform("linux"),
            format!("{}/.local/share/authentik/agent.sock", home)
        )
    }
}
