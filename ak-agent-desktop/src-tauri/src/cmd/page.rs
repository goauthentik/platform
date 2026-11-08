use std::sync::Mutex;

use tauri::State;

/// Page requested by a deep link, picked up by the frontend once it's loaded.
#[derive(Default)]
pub struct PendingPage(pub Mutex<Option<String>>);

#[tauri::command]
pub fn take_pending_page(pending: State<'_, PendingPage>) -> Option<String> {
    pending.0.lock().ok()?.take()
}
