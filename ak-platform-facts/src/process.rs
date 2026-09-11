use authentik_client::models::ProcessRequest;
use eyre::Result;
use serde::Deserialize;

use crate::query::{non_empty, query_named};

#[derive(Deserialize)]
struct ProcessRow {
    #[serde(default)]
    pid: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    cmdline: String,
    #[serde(default)]
    username: String,
}

fn process_name(row: &ProcessRow) -> String {
    if !row.cmdline.is_empty() {
        return row.cmdline.clone();
    }
    if !row.path.is_empty() {
        return row.path.clone();
    }
    row.name.clone()
}

/// Fully cross-platform via `osquery`'s `processes`/`users` tables —
/// unlike the other subsystems in this crate, there's no per-OS module
/// here.
pub fn gather() -> Result<Vec<ProcessRequest>> {
    Ok(query_named::<ProcessRow>("processes_with_user")?
        .into_iter()
        .filter_map(|row| {
            let name = process_name(&row);
            if name.is_empty() {
                return None;
            }
            let id = row.pid.parse::<i32>().ok()?;
            let user = non_empty(row.username);
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
