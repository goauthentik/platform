use std::{env, error::Error, path::PathBuf};

use crate::platform::string::PlatformString;

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

/**
 * The XDG crate does not correctly use Library/Application Support on macos, and always uses
 * .local/... on all unix-like platforms
 */
fn macos_lib_app_support(last_seg: &str) -> Result<String, Box<dyn Error>> {
    let mut root = PathBuf::new();
    let home = match env::home_dir() {
        Some(h) => match h.to_str() {
            Some(ps) => ps.to_string(),
            None => return Err(Box::from("Failed to convert home_dir to path")),
        },
        None => {
            let username = whoami::username()?;
            format!("/Users/{}", username)
        }
    };
    root.push(home);
    root.push("Library");
    root.push("Application Support");
    root.push("authentik");
    root.push(last_seg);
    match root.as_path().to_str() {
        Some(p) => Ok(p.to_string()),
        None => Err(Box::from("Failed to convert path to string")),
    }
}

pub fn agent_socket_path(id: AgentSocketID) -> Result<PlatformString, Box<dyn Error>> {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("authentik");
    let mut unix_sock = match xdg_dirs.data_home {
        Some(p) => p,
        None => return Err(Box::from("Failed to get XDG data path")),
    };
    match id {
        AgentSocketID::Default => {
            if let Ok(x) = env::var("AUTHENTIK_CLI_SOCKET") {
                return Ok(PlatformString::new_with_default(&x));
            }
            unix_sock.push("agent.sock");
            Ok(PlatformString::new()
                .with_windows(r"\\.\pipe\authentik\socket")
                .with_darwin(&macos_lib_app_support("agent.sock")?)
                .with_linux(unix_sock.as_path().to_str().unwrap_or("")))
        }
        AgentSocketID::SSH => {
            unix_sock.push("agent-ssh.sock");
            Ok(PlatformString::new()
                .with_windows(r"\\.\pipe\authentik\socket-ssh")
                .with_darwin(&macos_lib_app_support("agent-ssh.sock")?)
                .with_linux(unix_sock.as_path().to_str().unwrap_or("")))
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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

    #[test]
    fn test_agent_default_linux() {
        let binding = env::home_dir().unwrap();
        let home = binding.to_str().unwrap();
        assert_eq!(
            agent_socket_path(AgentSocketID::Default)
                .unwrap()
                .for_platform("linux"),
            format!("{}/.local/share/agent.sock", home)
        )
    }
}
