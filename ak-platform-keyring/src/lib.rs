use std::{error::Error, fmt::Display, sync::LazyLock};

use eyre::Result;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(any(test, debug_assertions))]
pub mod memory;

pub mod cache;

/// A backend capable of storing, retrieving and deleting credentials.
///
/// One implementation exists per platform (see [`macos`], [`linux`], [`windows`]) plus an
/// in-memory [`memory`] store used for development. Use [`get`] to obtain the correct
/// instance for the current build.
// #[allow(async_fn_in_trait)]
pub trait KeyringStore {
    fn get(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> impl std::future::Future<Output = Result<String, KeyringError>> + Send;
    fn set(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
        data: String,
    ) -> impl std::future::Future<Output = Result<(), KeyringError>> + Send;
    fn delete(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> impl std::future::Future<Output = Result<(), KeyringError>> + Send;
}

// `DefaultStore` resolves to the backend for the current build: the in-memory store for
// development (test/debug) builds on any platform, and the native store otherwise.
#[cfg(any(test, debug_assertions))]
pub type DefaultStore = memory::MemoryStore;
#[cfg(all(not(any(test, debug_assertions)), target_os = "macos"))]
pub type DefaultStore = macos::MacosStore;
#[cfg(all(not(any(test, debug_assertions)), target_os = "linux"))]
pub type DefaultStore = linux::LinuxStore;
#[cfg(all(not(any(test, debug_assertions)), target_os = "windows"))]
pub type DefaultStore = windows::WindowsStore;

static INSTANCE: LazyLock<DefaultStore> = LazyLock::new(DefaultStore::new);

/// Returns the [`KeyringStore`] instance for the current build.
pub fn store() -> &'static DefaultStore {
    &INSTANCE
}

pub fn service(name: &str) -> String {
    #[cfg(debug_assertions)]
    return format!("io.goauthentik.agent-debug.{name}");
    #[cfg(not(debug_assertions))]
    return format!("io.goauthentik.agent.{name}");
}

#[derive(Debug)]
pub enum KeyringError {
    Other(eyre::Report),
    NotFound(),
}

impl Display for KeyringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyringError::NotFound() => write!(f, "entry not found"),
            KeyringError::Other(e) => e.fmt(f),
        }
    }
}
impl Error for KeyringError {}

#[derive(Debug)]
pub enum Accessibility {
    Always,
    User,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn full() {
        store()
            .set(
                &service("foo"),
                "bar",
                Accessibility::User,
                "baz".to_string(),
            )
            .await
            .unwrap();
        assert_eq!(
            store()
                .get(&service("foo"), "bar", Accessibility::User)
                .await
                .unwrap(),
            "baz"
        );
    }
}
