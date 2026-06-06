use std::env;

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

pub fn agent_socket_path(id: AgentSocketID) -> PlatformString {
    let xdg_dirs = xdg::BaseDirectories::with_prefix("authentik");
    let mut unix_sock = xdg_dirs.data_home.unwrap();
    match id {
        AgentSocketID::Default => {
            if let Ok(x) = env::var("AUTHENTIK_CLI_SOCKET") {
                return PlatformString::new_with_default(&x);
            }
            unix_sock.push("agent.sock");
            PlatformString::new()
                .with_windows(r"\\.\pipe\authentik\socket")
                .with_linux(unix_sock.as_path().to_str().unwrap())
        }
        AgentSocketID::SSH => {
            unix_sock.push("agent-ssh.sock");
            PlatformString::new()
                .with_windows(r"\\.\pipe\authentik\socket-ssh")
                .with_linux(unix_sock.as_path().to_str().unwrap())
        }
    }
}
