use authentik_client::models::ProcessRequest;
use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::query::{non_empty, query_named};

/// How much is reported about each running process.
///
/// Command lines can carry secrets (tokens or passwords passed as flags), so
/// hosts where that matters can report less, or nothing at all.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessCollection {
    /// The full command line, including arguments.
    #[default]
    Full,
    /// The executable path (or the process name when there is no path),
    /// without arguments.
    Executable,
    /// No processes are reported.
    Disabled,
}

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

fn process_name(row: &ProcessRow, collection: ProcessCollection) -> String {
    if collection == ProcessCollection::Full && !row.cmdline.is_empty() {
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
pub fn gather(collection: ProcessCollection) -> Result<Vec<ProcessRequest>> {
    if collection == ProcessCollection::Disabled {
        return Ok(Vec::new());
    }
    Ok(query_named::<ProcessRow>("processes_with_user")?
        .into_iter()
        .filter_map(|row| {
            let name = process_name(&row, collection);
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

    const CMDLINE: &str = "backup-tool --password placeholder /srv";

    fn row(name: &str, path: &str, cmdline: &str) -> ProcessRow {
        ProcessRow {
            pid: "1".to_string(),
            name: name.to_string(),
            path: path.to_string(),
            cmdline: cmdline.to_string(),
            username: String::new(),
        }
    }

    #[test]
    fn gather_finds_processes() {
        let processes = gather(ProcessCollection::Full).unwrap_or_default();
        assert!(!processes.is_empty());
        assert!(processes.iter().all(|p| !p.name.is_empty() && p.id >= 0));
    }

    #[test]
    fn gather_disabled_reports_nothing() {
        assert!(gather(ProcessCollection::Disabled).is_ok_and(|p| p.is_empty()));
    }

    #[test]
    fn process_name_full_prefers_cmdline() {
        let full = ProcessCollection::Full;
        assert_eq!(
            process_name(&row("backup-tool", "/usr/bin/backup-tool", CMDLINE), full),
            CMDLINE
        );
        assert_eq!(
            process_name(&row("backup-tool", "/usr/bin/backup-tool", ""), full),
            "/usr/bin/backup-tool"
        );
    }

    #[test]
    fn process_name_executable_drops_arguments() {
        let exe = ProcessCollection::Executable;
        assert_eq!(
            process_name(&row("backup-tool", "/usr/bin/backup-tool", CMDLINE), exe),
            "/usr/bin/backup-tool"
        );
        assert_eq!(
            process_name(&row("kworker/0:1", "", ""), exe),
            "kworker/0:1"
        );
        assert_eq!(process_name(&row("", "", CMDLINE), exe), "");
    }

    #[test]
    fn process_collection_serde() {
        for (collection, json) in [
            (ProcessCollection::Full, r#""full""#),
            (ProcessCollection::Executable, r#""executable""#),
            (ProcessCollection::Disabled, r#""disabled""#),
        ] {
            assert!(serde_json::to_string(&collection).is_ok_and(|s| s == json));
            assert!(serde_json::from_str::<ProcessCollection>(json).is_ok_and(|c| c == collection));
        }
        assert!(serde_json::from_str::<ProcessCollection>(r#""cmdline""#).is_err());
    }
}
