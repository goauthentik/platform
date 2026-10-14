use std::path::{Path, PathBuf};

use ak_macos_touchid::AccessRequest;
use ak_platform::net::server::proc_info::{ProcInfo, proc_exe_path, proc_parent_pid};
use eyre::Result;

/// Bundle ID prefix of our own apps. The `ak` CLI lives inside our bundle, so these
/// are skipped to find the app the user is actually working in.
const OWN_BUNDLE_PREFIX: &str = "io.goauthentik.platform.";

/// Outermost `.app` containing `exe`, so helper apps nested inside another app
/// (e.g. `Code Helper.app` in `Visual Studio Code.app`) resolve to the main app.
pub fn find_app_bundle(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .filter(|p| p.extension().is_some_and(|e| e == "app") && p.exists())
        .last()
        .map(Path::to_path_buf)
}

fn bundle_info(bundle: &Path) -> Option<plist::Dictionary> {
    plist::Value::from_file(bundle.join("Contents/Info.plist"))
        .ok()?
        .into_dictionary()
}

fn is_own_bundle(info: Option<&plist::Dictionary>) -> bool {
    info.and_then(|d| d.get("CFBundleIdentifier"))
        .and_then(|v| v.as_string())
        .is_some_and(|id| id.starts_with(OWN_BUNDLE_PREFIX))
}

pub fn populate_from_bundle(ar: &mut AccessRequest, bundle: &Path) {
    let name = bundle_info(bundle).and_then(|d| {
        d.get("CFBundleDisplayName")
            .or_else(|| d.get("CFBundleName"))
            .and_then(|v| v.as_string())
            .map(str::to_string)
    });
    ar.requesting_app = name
        .or_else(|| Some(bundle.file_stem()?.to_str()?.to_string()))
        .unwrap_or_default();
    // The icon itself is resolved by NSWorkspace on the Swift side, which also
    // handles icons that only exist in an asset catalog.
    ar.app_bundle = Some(bundle.to_path_buf());
}

pub async fn lookup_app_info(pc: Option<ProcInfo>) -> Result<AccessRequest> {
    let mut ar = AccessRequest::default();

    let Some(pc) = pc else {
        tracing::debug!("No proc info given");
        return Ok(ar);
    };

    // Walk up the process tree. proc_parent_pid/proc_exe_path are kept separate so
    // a process with an unreadable exe (sandboxed, system-owned) doesn't stop the walk.
    let mut pid = Some(pc.pid);
    while let Some(p) = pid {
        if let Some(bundle) = proc_exe_path(p).and_then(|exe| find_app_bundle(&exe)) {
            if is_own_bundle(bundle_info(&bundle).as_ref()) {
                tracing::trace!(pid = p, bundle = %bundle.display(), "Skipping own app bundle");
            } else {
                tracing::trace!(pid = p, bundle = %bundle.display(), "Found app bundle");
                populate_from_bundle(&mut ar, &bundle);
                return Ok(ar);
            }
        }
        pid = proc_parent_pid(p);
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

    #[test]
    fn test_find_app_bundle() {
        let bundle =
            find_app_bundle(Path::new("/Applications/Safari.app/Contents/MacOS/Safari")).unwrap();
        assert_eq!(bundle.to_str().unwrap(), "/Applications/Safari.app");
        let not_fund =
            find_app_bundle(Path::new("/Applications/not found.app/Contents/MacOS/test"));
        assert!(not_fund.is_none());
    }

    #[test]
    fn test_populate_bundle() {
        let mut ar = AccessRequest::default();
        populate_from_bundle(&mut ar, Path::new("/Applications/Safari.app"));
        assert_eq!(ar.requesting_app, "Safari");
        assert!(ar.app_bundle.is_some());
    }

    #[test]
    fn test_is_own_bundle() {
        assert!(!is_own_bundle(
            bundle_info(Path::new("/Applications/Safari.app")).as_ref()
        ));
        let mut d = plist::Dictionary::new();
        d.insert(
            "CFBundleIdentifier".into(),
            "io.goauthentik.platform.agent".into(),
        );
        assert!(is_own_bundle(Some(&d)));
    }

    #[tokio::test]
    async fn test_lookup_app_info() {
        let info = lookup_app_info(Some(ProcInfo::from_pid(process::id()).unwrap()))
            .await
            .unwrap();
        assert!(!info.requesting_app.is_empty());
    }
}
