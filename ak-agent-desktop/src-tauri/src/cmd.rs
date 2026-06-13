use ak_agent::{Agent, token::{AuthentikClaims}};

pub type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn greet(name: &str) -> Result<String> {
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}

#[tauri::command]
pub async fn get_user_info(profile: String, state: tauri::State<'_, Agent>) -> Result<AuthentikClaims> {
    let ptm = state
        .gtm
        .for_profile(&profile)
        .await
        .ok_or_else(|| format!("profile '{profile}' not found"))?;
    let token = ptm.token().await.map_err(|e| e.to_string())?;
    let claims = token.claims().map_err(|e| e.to_string())?;
    Ok(claims)
}
