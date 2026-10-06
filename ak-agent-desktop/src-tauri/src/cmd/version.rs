use ak_platform::{
    generated::ping::ping_client::PingClient,
    grpc::grpc_endpoint,
    paths::{SysdSocketID, sysd_socket_path},
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
    pub agent: String,
    pub sysd: ComponentVersion,
}

#[tauri::command]
pub async fn get_versions() -> Result<Versions> {
    let sysd = ping_component(sysd_socket_path(SysdSocketID::Default)).await;
    Ok(Versions {
        agent: ak_meta::full_version(),
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
