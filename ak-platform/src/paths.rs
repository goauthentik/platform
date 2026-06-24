use dirs_next::{config_dir, data_dir};
use std::env;

use crate::prelude::*;
use crate::string::PlatformString;

pub const DEFAULT_PROFILE: &str = "default";

pub enum SysdSocketID {
    Default,
    CTRL,
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

pub enum AgentSocketID {
    Default,
    SSH,
}

fn xdg_data_path(last_seg: &str) -> Result<String> {
    let mut data = match data_dir() {
        Some(d) => d,
        None => return Err(Box::from("Failed to get XDG data path")),
    };
    data.push("authentik");
    data.push(last_seg);
    match data.as_path().to_str() {
        Some(p) => Ok(p.to_string()),
        None => Err(Box::from("Failed to convert path to string")),
    }
}

pub fn xdg_config_path(last_seg: &str) -> Result<String> {
    let mut data = match config_dir() {
        Some(d) => d,
        None => return Err(Box::from("Failed to get XDG data path")),
    };
    data.push("authentik");
    data.push(last_seg);
    match data.as_path().to_str() {
        Some(p) => Ok(p.to_string()),
        None => Err(Box::from("Failed to convert path to string")),
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
