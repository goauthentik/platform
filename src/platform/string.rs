use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformString {
    windows: Option<String>,
    darwin: Option<String>,
    linux: Option<String>,
    android: Option<String>,
    fallback: String,
}

impl Default for PlatformString {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformString {
    pub fn new() -> PlatformString {
        PlatformString::new_with_default("")
    }

    pub fn new_with_default(fallback: &str) -> PlatformString {
        PlatformString {
            windows: None,
            darwin: None,
            linux: None,
            android: None,
            fallback: fallback.to_string(),
        }
    }

    pub fn with_windows(mut self, windows: &str) -> PlatformString {
        self.windows = Some(windows.to_string());
        self
    }

    pub fn with_darwin(mut self, darwin: &str) -> PlatformString {
        self.darwin = Some(darwin.to_string());
        self
    }

    pub fn with_linux(mut self, linux: &str) -> PlatformString {
        self.linux = Some(linux.to_string());
        self
    }

    pub fn with_android(mut self, android: &str) -> PlatformString {
        self.android = Some(android.to_string());
        self
    }

    pub fn for_platform(self, name: &str) -> String {
        match name {
            "macos" => self.darwin.or(self.linux).unwrap_or(self.fallback),
            "windows" => self.windows.or(self.linux).unwrap_or(self.fallback),
            "linux" => self.linux.unwrap_or(self.fallback),
            "android" => self.android.or(self.linux).unwrap_or(self.fallback),
            _ => self.fallback,
        }
    }

    pub fn for_current(self) -> String {
        self.for_platform(std::env::consts::OS)
    }
}
