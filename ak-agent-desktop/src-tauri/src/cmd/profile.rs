use ak_agent::{Agent, config::ConfigV1Profile};
use ak_platform::{
    dpop::LocalDpopProver,
    generated::agent_ctrl::{Profile, ProfileStatus},
    setup,
};
use authentik_client::{apis::core_api::core_users_me_retrieve, models::SessionUser};
use tauri::Emitter;
use url::Url;

use super::Result;

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
            dpop_bound: c_prof.dpop_enabled(),
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
            dpop_bound: c_prof.dpop_enabled(),
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

#[tauri::command]
pub async fn setup_profile(
    app: tauri::AppHandle,
    state: tauri::State<'_, Agent>,
    name: String,
    authentik_url: String,
    client_id: String,
    app_slug: String,
) -> Result<()> {
    let (signer, _) = state
        .prepare_dpop_key(
            &name,
            authentik_url.clone(),
            app_slug.clone(),
            client_id.clone(),
        )
        .await
        .map_err(|e| format!("failed to prepare DPoP key: {e:#}"))?;
    let prof = setup::setup(
        setup::Options {
            authentik_url: Url::parse(&authentik_url)
                .map_err(|e| format!("invalid authentik URL: {e}"))?,
            app_slug: app_slug.clone(),
            client_id: client_id.clone(),
            user_agent: ak_meta::user_agent(),
        },
        setup::DpopKey {
            jkt: signer.thumbprint().map_err(|e| e.to_string())?,
            prover: &LocalDpopProver(&signer),
        },
        // Frontend opens the URL and shows it as a fallback link.
        |url| Ok(app.emit("ak-setup-url", url.to_string())?),
    )
    .await
    .map_err(|e| format!("device flow setup failed: {e:#}"))?;
    let (Some(at), Some(rt)) = (prof.access_token, prof.refresh_token) else {
        return Err("device flow setup did not return access/refresh token".to_string());
    };
    state
        .setup_profile(
            &name,
            ConfigV1Profile::from_tokens(authentik_url, app_slug, client_id, at, rt),
            prof.dpop_bound,
        )
        .await
        .map_err(|e| format!("failed to save profile: {e:#}"))
}

#[tauri::command]
pub async fn delete_profile(state: tauri::State<'_, Agent>, name: String) -> Result<()> {
    state
        .delete_profile(&name)
        .await
        .map_err(|e| format!("failed to delete profile: {e:#}"))
}
