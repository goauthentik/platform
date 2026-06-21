use std::path::{Path, PathBuf};

use ak_macos_touchid::{AccessRequest, authenticate_with_touchid};
use ak_platform::{net::server::proc_info::ProcInfo, prelude::BoxError};

use crate::sys::AuthorizationRequest;

fn find_app_bundle(exe: &Path) -> Option<PathBuf> {
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

fn populate_from_bundle(ar: &mut AccessRequest, bundle: &Path) {
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

    // pc.parent.parent is always None because ProcInfo only stores one parent
    // level. From pc.parent onward we look up each successive grandparent by
    // calling ProcInfo::from_pid, which returns that process plus one more level.
    let mut lookup_pid = pc.parent.as_ref().map(|p| p.pid);
    while let Some(pid) = lookup_pid {
        let Ok(info) = ProcInfo::from_pid(pid) else {
            break;
        };
        let Some(parent) = &info.parent else { break };

        tracing::trace!(pid = parent.pid, "Checking process (manual walk)");
        if let Some(bundle) = find_app_bundle(&parent.exe) {
            tracing::trace!(bundle = %bundle.display(), "Found app bundle");
            populate_from_bundle(&mut ar, &bundle);
            return Ok(ar);
        }

        // pid 1 is launchd — stop before looping forever
        lookup_pid = if parent.pid > 1 {
            Some(parent.pid)
        } else {
            None
        };
    }

    // No bundle found anywhere in the tree — use the direct process exe name
    if let Some(name) = pc.exe.file_name().and_then(|n| n.to_str()) {
        tracing::trace!("No bundle found, using exe name");
        ar.requesting_app = name.to_string();
    }

    Ok(ar)
}

pub async fn prompt(req: AuthorizationRequest) -> Result<bool, BoxError> {
    let res = authenticate_with_touchid(AccessRequest {
        title: "authentik Access Request".to_string(),
        reason: req.msg.for_current(),
        ..lookup_app_info(req.proc_info).await?
    });
    Result::<bool, BoxError>::Ok(res)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

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
}
