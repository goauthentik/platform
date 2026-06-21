#[swift_bridge::bridge]
mod ffi {
    #[swift_bridge(swift_repr = "struct")]
    struct AccessRequestFfi {
        title: String,
        requesting_app: String,
        app_icon_path: String,
        accent_color: u32,
        profile_name: String,
        profile_email: String,
        profile_username: String,
        profile_groups: String,
        profile_avatar_path: String,
        reason: String,
    }

    extern "Swift" {
        fn authenticate_with_touchid(req: AccessRequestFfi) -> bool;
        fn is_touchid_available() -> bool;
    }
}

pub struct AccessRequest {
    pub title: String,
    pub requesting_app: String,
    pub app_icon: Option<Vec<u8>>,
    pub accent_color: u32,
    pub profile_name: String,
    pub profile_email: String,
    pub profile_username: String,
    pub profile_groups: String,
    pub profile_avatar: Option<Vec<u8>>,
    pub reason: String,
}

impl Default for AccessRequest {
    fn default() -> Self {
        Self {
            title: "authentik Access Requested".to_string(),
            requesting_app: String::new(),
            app_icon: None,
            accent_color: 0xFD4B2D,
            profile_name: String::new(),
            profile_email: String::new(),
            profile_username: String::new(),
            profile_groups: String::new(),
            profile_avatar: None,
            reason: String::new(),
        }
    }
}

pub fn authenticate_with_touchid(req: AccessRequest) -> bool {
    let app_icon_temp = TempImageFile::write(&req.app_icon);
    let avatar_temp = TempImageFile::write(&req.profile_avatar);

    ffi::authenticate_with_touchid(ffi::AccessRequestFfi {
        title: req.title,
        requesting_app: req.requesting_app,
        app_icon_path: app_icon_temp.path(),
        accent_color: req.accent_color,
        profile_name: req.profile_name,
        profile_email: req.profile_email,
        profile_username: req.profile_username,
        profile_groups: req.profile_groups,
        profile_avatar_path: avatar_temp.path(),
        reason: req.reason,
    })
}

pub fn is_touchid_available() -> bool {
    ffi::is_touchid_available()
}

struct TempImageFile(Option<std::path::PathBuf>);

impl TempImageFile {
    fn write(data: &Option<Vec<u8>>) -> Self {
        let bytes = match data.as_deref() {
            Some(b) if !b.is_empty() => b,
            _ => return Self(None),
        };
        static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("ak-touchid-{}-{}.png", std::process::id(), id));
        match std::fs::write(&path, bytes) {
            Ok(()) => Self(Some(path)),
            Err(_) => Self(None),
        }
    }

    fn path(&self) -> String {
        self.0
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }
}

impl Drop for TempImageFile {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_file(path);
        }
    }
}
