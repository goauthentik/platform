use ak_platform_authz::AuthRecord;

#[tauri::command]
pub fn recent_authorizations() -> Vec<AuthRecord> {
    ak_platform_authz::history()
}
