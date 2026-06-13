pub type Result<T> = std::result::Result<T, String>;

#[tauri::command]
pub async fn greet(name: &str) -> Result<String> {
    Ok(format!("Hello, {}! You've been greeted from Rust!", name))
}
