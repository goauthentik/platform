use ak_agent::{Agent, config::SshFallbackAgentConfig};
use ak_platform::{
    generated::{
        agent_ctrl::{Profile, ProfileStatus},
        ping::ping_client::PingClient,
    },
    grpc::grpc_endpoint,
    paths::{AgentSocketID, SysdSocketID, agent_socket_path, sysd_socket_path},
    string::PlatformString,
};
use authentik_client::{apis::core_api::core_users_me_retrieve, models::SessionUser};

pub type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn list_profiles(state: tauri::State<'_, Agent>) -> Result<Vec<Profile>> {
    let snapshot: Vec<_> = {
        let cfg = state.cfg.read().await;
        cfg.profiles
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };
    let mut profiles = vec![];
    for (key, c_prof) in snapshot {
        let failed_profile = || Profile {
            name: key.clone(),
            username: String::new(),
            authentik_url: c_prof.authentik_url.clone(),
            last_renewed: None,
            next_renew: None,
            status: ProfileStatus::Failed as i32,
        };

        let Some(ptm) = state.gtm.for_profile(&key).await else {
            tracing::warn!(profile = key, "no token manager for profile");
            profiles.push(failed_profile());
            continue;
        };

        let token = match ptm.token().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(profile = key, "failed to renew token: {e:?}");
                profiles.push(failed_profile());
                continue;
            }
        };
        let claims = match token.claims() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(profile = key, "failed to parse claims: {e:?}");
                profiles.push(failed_profile());
                continue;
            }
        };

        profiles.push(Profile {
            name: key.clone(),
            username: claims.preferred_username,
            authentik_url: c_prof.authentik_url.clone(),
            last_renewed: Some(claims.iat.into()),
            next_renew: Some(claims.exp.into()),
            status: ProfileStatus::Active as i32,
        });
    }
    Ok(profiles)
}

#[tauri::command]
pub async fn get_user_info(profile: String, state: tauri::State<'_, Agent>) -> Result<SessionUser> {
    let prof = state
        .cfg
        .read()
        .await
        .profiles
        .get(&profile)
        .cloned()
        .ok_or_else(|| "profile not found".to_string())?;
    let me = core_users_me_retrieve(&prof.api_config().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?;
    Ok(me)
}

#[tauri::command]
pub async fn active_profile(state: tauri::State<'_, Agent>) -> Result<String> {
    Ok(state.cfg.read().await.active_profile.clone())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentVersion {
    pub version: Option<String>,
    pub server_version: Option<String>,
    pub error: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Versions {
    pub desktop: String,
    pub agent: ComponentVersion,
    pub sysd: ComponentVersion,
}

#[tauri::command]
pub async fn get_versions() -> Result<Versions> {
    let agent = match agent_socket_path(AgentSocketID::Default) {
        Ok(p) => ping_component(p).await,
        Err(e) => ComponentVersion {
            version: None,
            server_version: None,
            error: Some(e.to_string()),
        },
    };
    let sysd = ping_component(sysd_socket_path(SysdSocketID::Default)).await;
    Ok(Versions {
        desktop: ak_meta::full_version(),
        agent,
        sysd,
    })
}

async fn ping_component(p: PlatformString) -> ComponentVersion {
    let channel = match grpc_endpoint(p.for_current()).await {
        Ok(c) => c,
        Err(e) => {
            return ComponentVersion {
                version: None,
                server_version: None,
                error: Some(format!("{e:?}")),
            };
        }
    };
    match PingClient::new(channel).ping(()).await {
        Ok(res) => {
            let res = res.into_inner();
            ComponentVersion {
                version: Some(res.version),
                server_version: Some(res.server_version),
                error: None,
            }
        }
        Err(e) => ComponentVersion {
            version: None,
            server_version: None,
            error: Some(format!("{e:?}")),
        },
    }
}

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
