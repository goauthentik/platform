use ak_agent::Agent;
use ak_platform::generated::agent_ctrl::Profile;
use authentik_client::{apis::core_api::core_users_me_retrieve, models::SessionUser};

pub type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn list_profiles(state: tauri::State<'_, Agent>) -> Result<Vec<Profile>> {
    let mut profiles = vec![];
    for (key, c_prof) in state.cfg.read().await.profiles.iter() {
        let ptm = state
            .gtm
            .for_profile(key)
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
