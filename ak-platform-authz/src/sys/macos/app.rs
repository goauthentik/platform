use std::path::{Path, PathBuf};

use ak_macos_touchid::AccessRequest;


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

#[cfg(test)]
mod tests {
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
