use authentik_client::models::ProcessRequest;
use eyre::Result;

use crate::query::query_named;

fn process_name(row: &osquery::Row) -> String {
    if let Some(cmdline) = row.get("cmdline").filter(|s| !s.is_empty()) {
        return cmdline.to_string();
    }
    if let Some(path) = row.get("path").filter(|s| !s.is_empty()) {
        return path.to_string();
    }
    row.get("name").cloned().unwrap_or_default()
}

/// Fully cross-platform via `osquery`'s `processes`/`users` tables —
/// unlike the other subsystems in this crate, there's no per-OS module
/// here.
pub fn gather() -> Result<Vec<ProcessRequest>> {
    Ok(query_named("processes_with_user")?
        .into_iter()
        .filter_map(|row| {
            let name = process_name(&row);
            if name.is_empty() {
                return None;
            }
            let id = row.get("pid")?.parse::<i32>().ok()?;
            let user = row.get("username").filter(|s| !s.is_empty()).cloned();
            Some(ProcessRequest { id, name, user })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_finds_processes() {
        let processes = gather().unwrap_or_default();
        assert!(!processes.is_empty());
        assert!(processes.iter().all(|p| !p.name.is_empty() && p.id >= 0));
    }
}
