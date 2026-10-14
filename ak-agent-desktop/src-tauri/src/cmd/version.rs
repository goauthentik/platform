use ak_platform::{
    generated::ping::ping_client::PingClient,
    grpc::grpc_endpoint,
    paths::{SysdSocketID, sysd_socket_path},
    string::PlatformString,
};

use semver::Version;
use sha2::{Digest, Sha512};
use std::time::{Duration, Instant};
use tauri::http;
use tokio::sync::Mutex;

use super::Result;

const VERSION_URL: &str = "https://version.goauthentik.io/versions/platform/latest.json";
const ANALYTICS_URL: &str = "https://goauthentik.io/api/event";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(5);

/// Cached so the UI refreshing doesn't hit version.goauthentik.io every time.
static LATEST: Mutex<Option<(Instant, Option<StableVersion>)>> = Mutex::const_new(None);

#[derive(serde::Deserialize)]
struct VersionInfo {
    stable: StableVersion,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StableVersion {
    pub version: String,
    #[serde(default)]
    pub changelog_url: String,
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
    pub agent: String,
    pub sysd: ComponentVersion,
    /// Set only when upstream has a newer release than this build.
    pub update: Option<StableVersion>,
}

#[tauri::command]
pub async fn get_versions() -> Result<Versions> {
    let sysd = ping_component(sysd_socket_path(SysdSocketID::Default)).await;
    Ok(Versions {
        agent: ak_meta::full_version(),
        sysd,
        update: latest_version()
            .await
            .filter(|l| is_newer(&l.version, &ak_meta::version())),
    })
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| v == "true" || v == "1")
}

fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(ak_meta::user_agent())
        .timeout(TIMEOUT)
        .build()
}

async fn latest_version() -> Option<StableVersion> {
    if env_flag("AUTHENTIK_DISABLE_UPDATE_CHECK") {
        return None;
    }
    let mut cached = LATEST.lock().await;
    if let Some((at, v)) = cached.as_ref()
        && at.elapsed() < CHECK_INTERVAL
    {
        return v.clone();
    }
    let fetched = async {
        let info: VersionInfo = http_client()?
            .get(VERSION_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        reqwest::Result::Ok(info.stable)
    }
    .await
    .inspect_err(|e| tracing::debug!("version check failed: {e:?}"))
    .ok();
    *cached = Some((Instant::now(), fetched.clone()));
    fetched
}

/// Anything unparseable (e.g. dev builds' "Next") is never outdated.
fn is_newer(upstream: &str, local: &str) -> bool {
    match (Version::parse(upstream), Version::parse(local)) {
        (Ok(u), Ok(l)) => u > l,
        _ => false,
    }
}

/// Anonymous startup ping, same shape as authentik's `disable_startup_analytics` one.
pub async fn send_startup_analytics() {
    if cfg!(debug_assertions)
        || cfg!(test)
        || std::env::var_os("CI").is_some()
        || env_flag("AUTHENTIK_DISABLE_STARTUP_ANALYTICS")
    {
        return;
    }
    let Ok(client) = http_client() else { return };
    let install_id = hex::encode(Sha512::digest(tauri_plugin_os::hostname().as_bytes()));
    let _ = client
        .post(ANALYTICS_URL)
        .header(reqwest::header::USER_AGENT, &install_id[..16])
        .header(http::HeaderName::from_static("x-authentik-product"), "platform")
        .json(&serde_json::json!({
            "domain": "authentik",
            "name": "pageview",
            "referrer": ak_meta::full_version(),
            "url": format!("http://localhost/platform?&utm_medium={}:{}", std::env::consts::OS, std::env::consts::ARCH),
        }))
        .send()
        .await;
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.70.1", "0.70.0"));
        assert!(is_newer("0.100.0", "0.70.0"));
        assert!(!is_newer("0.70.0", "0.70.0"));
        assert!(!is_newer("0.63.3", "0.70.0"));
        assert!(!is_newer("0.70.1", "Next"));
        assert!(!is_newer("0.71.0-rc1", "0.71.0"));
    }
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
