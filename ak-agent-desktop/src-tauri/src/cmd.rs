use ak_agent::{Agent, token::AuthentikClaims};
use ak_platform::generated::agent_ctrl::Profile;

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
pub async fn get_user_info(
    profile: String,
    state: tauri::State<'_, Agent>,
) -> Result<AuthentikClaims> {
    let ptm = state
        .gtm
        .for_profile(&profile)
        .await
        .ok_or_else(|| format!("profile '{profile}' not found"))?;
    let token = ptm.token().await.map_err(|e| e.to_string())?;
    let claims = token.claims().map_err(|e| e.to_string())?;
    Ok(claims)
}
