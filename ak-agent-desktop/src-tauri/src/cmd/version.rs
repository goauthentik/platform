use ak_platform::{
    generated::ping::ping_client::PingClient,
    grpc::grpc_endpoint,
    paths::{AgentSocketID, SysdSocketID, agent_socket_path, sysd_socket_path},
    string::PlatformString,
};

use super::Result;

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
