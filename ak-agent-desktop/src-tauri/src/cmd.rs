use ak_agent::Agent;
use ak_platform::{
    generated::{agent_ctrl::Profile, ping::ping_client::PingClient},
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
        let ptm = state
            .gtm
            .for_profile(&key)
            .await
            .ok_or("profile not found")?;
        let token = ptm.token().await.map_err(|e| e.to_string())?;
        let claims = token.claims().map_err(|e| e.to_string())?;
        let o_prof = Profile {
            name: key.clone(),
            username: claims.preferred_username,
            authentik_url: c_prof.authentik_url.clone(),
            last_renewed: Some(claims.iat.into()),
            next_renew: Some(claims.exp.into()),
        };
        profiles.push(o_prof);
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
