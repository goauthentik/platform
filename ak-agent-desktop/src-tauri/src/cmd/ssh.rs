use ak_agent::{Agent, config::SshFallbackAgentConfig};
use ak_platform::paths::{AgentSocketID, agent_socket_path};

use super::Result;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshConfig {
    pub socket_path: String,
    /// `SSH_AUTH_SOCK` this app was started with, suggested as fallback agent.
    pub system_socket_path: Option<String>,
    pub fallback_socket_path: String,
    pub extra_passthrough_hosts: Vec<String>,
}

#[tauri::command]
pub async fn get_ssh_config(state: tauri::State<'_, Agent>) -> Result<SshConfig> {
    let fallback = state.cfg.read().await.ssh_fallback_agent.clone();
    Ok(SshConfig {
        socket_path: agent_socket_path(AgentSocketID::SSH)
            .map_err(|e| e.to_string())?
            .for_current(),
        system_socket_path: std::env::var("SSH_AUTH_SOCK").ok(),
        fallback_socket_path: fallback
            .as_ref()
            .map(|f| f.socket_path.clone())
            .unwrap_or_default(),
        extra_passthrough_hosts: fallback
            .map(|f| f.extra_passthrough_hosts)
            .unwrap_or_default(),
    })
}

/// An empty `socket_path` disables the fallback agent.
#[tauri::command]
pub async fn set_ssh_fallback_agent(
    socket_path: String,
    extra_passthrough_hosts: Vec<String>,
    state: tauri::State<'_, Agent>,
) -> Result<()> {
    state.cfg.write().await.ssh_fallback_agent =
        (!socket_path.is_empty()).then_some(SshFallbackAgentConfig {
            socket_path,
            extra_passthrough_hosts,
        });
    state.cfg.save().await.map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SshStatus {
    /// `ssh` uses our agent for any host.
    Active,
    /// `~/.ssh/config` references our agent, but only for some hosts.
    Partial,
    Unconfigured,
    /// Our agent socket doesn't exist.
    NotRunning,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshStatusResponse {
    pub status: SshStatus,
    /// Agent `ssh` uses for hosts without a specific config.
    pub identity_agent: Option<String>,
}

#[tauri::command]
pub async fn get_ssh_status() -> Result<SshStatusResponse> {
    let socket = agent_socket_path(AgentSocketID::SSH)
        .map_err(|e| e.to_string())?
        .for_current();
    // Resolved config for an arbitrary host, i.e. what `Host *` applies; ssh expands `~`.
    let identity_agent = std::process::Command::new("ssh")
        .args(["-G", "authentik-agent-check"])
        .output()
        .ok()
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .find_map(|l| l.strip_prefix("identityagent ").map(str::to_owned))
        });
    let status = if !std::path::Path::new(&socket).exists() {
        SshStatus::NotRunning
    } else if identity_agent.as_deref() == Some(socket.as_str()) {
        SshStatus::Active
    } else if references_socket(&socket) {
        SshStatus::Partial
    } else {
        SshStatus::Unconfigured
    };
    Ok(SshStatusResponse {
        status,
        identity_agent,
    })
}

// plain text search of ~/.ssh/config, doesn't follow `Include`s
fn references_socket(socket: &str) -> bool {
    let Some(home) = std::env::home_dir() else {
        return false;
    };
    let Ok(cfg) = std::fs::read_to_string(home.join(".ssh").join("config")) else {
        return false;
    };
    let home = home.to_string_lossy();
    let tilde = socket.replacen(home.as_ref(), "~", 1);
    cfg.contains(socket) || cfg.contains(&tilde)
}
