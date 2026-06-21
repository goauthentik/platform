use std::path::{Path, PathBuf};

use ak_macos_touchid::AccessRequest;
use ak_platform::{
    net::server::proc_info::{ProcInfo, proc_exe_path, proc_parent_pid},
    prelude::BoxError,
};

pub fn find_app_bundle(exe: &Path) -> Option<PathBuf> {
    let mut path = exe.to_path_buf();
    while path.pop() {
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".app"))
            && path.exists()
        {
            return Some(path);
        }
    }
    None
}

pub fn populate_from_bundle(ar: &mut AccessRequest, bundle: &Path) {
    let plist_path = bundle.join("Contents/Info.plist");
    if let Ok(plist::Value::Dictionary(dict)) = plist::Value::from_file(&plist_path) {
        let name = dict
            .get("CFBundleDisplayName")
            .or_else(|| dict.get("CFBundleName"))
            .and_then(|v| v.as_string());
        if let Some(name) = name {
            ar.requesting_app = name.to_string();
        }

        if let Some(icon_file) = dict.get("CFBundleIconFile").and_then(|v| v.as_string()) {
            let icon_name = if icon_file.ends_with(".icns") {
                icon_file.to_string()
            } else {
                format!("{icon_file}.icns")
            };
            let icon_path = bundle.join("Contents/Resources").join(icon_name);
            if let Ok(bytes) = std::fs::read(icon_path) {
                ar.app_icon = Some(bytes);
            }
        }
    }

    // Fallback: use the .app bundle stem if plist had no name
    if ar.requesting_app.is_empty()
        && let Some(stem) = bundle.file_stem().and_then(|n| n.to_str())
    {
        ar.requesting_app = stem.to_string();
    }
}

pub async fn lookup_app_info(pc: Option<ProcInfo>) -> Result<AccessRequest, BoxError> {
    let mut ar = AccessRequest::default();

    let Some(pc) = pc else {
        tracing::debug!("No proc info given");
        return Ok(ar);
    };

    // Walk the pre-fetched chain (pc → pc.parent); both are already in memory.
    let mut current = Some(&pc);
    while let Some(proc) = current {
        tracing::trace!(pid = proc.pid, cmd = proc.cmdline, "Checking process");
        if let Some(bundle) = find_app_bundle(&proc.exe) {
            tracing::trace!(bundle = %bundle.display(), "Found app bundle");
            populate_from_bundle(&mut ar, &bundle);
            return Ok(ar);
        }
        current = proc.parent.as_deref();
    }

    // Continue walking up the process tree beyond the one parent level stored
    // in ProcInfo. proc_parent_pid/proc_exe_path are kept decoupled here so
    // that a process with an unreadable exe (sandboxed, system-owned) doesn't
    // stop the walk — we can still get its parent PID and keep climbing.
    let mut walk_pid = pc.parent.as_ref().map(|p| p.pid);
    while let Some(pid) = walk_pid {
        let Some(ppid) = proc_parent_pid(pid) else {
            break;
        };

        if let Some(exe) = proc_exe_path(ppid) {
            tracing::trace!(pid = ppid, exe = %exe.display(), "Checking process (tree walk)");
            if let Some(bundle) = find_app_bundle(&exe) {
                tracing::trace!(bundle = %bundle.display(), "Found app bundle");
                populate_from_bundle(&mut ar, &bundle);
                return Ok(ar);
            }
        }

        walk_pid = Some(ppid);
    }

    // No bundle found anywhere in the tree — use the direct process exe name
    if let Some(name) = pc.exe.file_name().and_then(|n| n.to_str()) {
        tracing::trace!("No bundle found, using exe name");
        ar.requesting_app = name.to_string();
    }

    Ok(ar)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    #[tokio::test]
    async fn test_find_app_bundle() {
        let bundle =
            find_app_bundle(Path::new("/Applications/Safari.app/Contents/MacOS/Safari")).unwrap();
        assert_eq!(bundle.to_str().unwrap(), "/Applications/Safari.app");
        let not_fund =
            find_app_bundle(Path::new("/Applications/not found.app/Contents/MacOS/test"));
        assert!(not_fund.is_none());
    }

    #[tokio::test]
    async fn test_populate_bundle() {
        let mut ar = AccessRequest::default();
        populate_from_bundle(&mut ar, Path::new("/Applications/Safari.app"));
        assert_eq!(ar.requesting_app, "Safari");
        assert!(ar.app_icon.is_some());
    }

    #[tokio::test]
    async fn test_lookup_app_info() {
        let info = lookup_app_info(Some(ProcInfo::from_pid(process::id()).unwrap())).await.unwrap();
        assert!(info.requesting_app != "");
    }
}
