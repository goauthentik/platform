use authentik_client::models::ProcessRequest;
use eyre::Result;
use sysinfo::{ProcessesToUpdate, System, Users};

/// Matches Go's `Cmdline()` → `Exe()` → `Name()` fallback chain.
fn process_name(process: &sysinfo::Process) -> String {
    let cmd = process.cmd();
    if !cmd.is_empty() {
        return cmd
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
    }
    if let Some(exe) = process.exe() {
        return exe.to_string_lossy().to_string();
    }
    process.name().to_string_lossy().to_string()
}

/// Enumerates running processes for the current host. Fully cross-platform
/// via `sysinfo` — no per-OS code needed, unlike Go's `gopsutil`-based
/// implementation which still had a separate Windows name-resolution path.
pub fn gather() -> Result<Vec<ProcessRequest>> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);
    let users = Users::new_with_refreshed_list();

    Ok(system
        .processes()
        .values()
        .filter_map(|process| {
            let name = process_name(process);
            if name.is_empty() {
                return None;
            }
            let user = process
                .user_id()
                .and_then(|uid| users.get_user_by_id(uid))
                .map(|u| u.name().to_string());
            Some(ProcessRequest {
                id: process.pid().as_u32() as i32,
                name,
                user,
            })
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
