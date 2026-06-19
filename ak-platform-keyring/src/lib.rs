use std::{error::Error, fmt::Display};

#[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
use std::collections::HashMap;

#[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
use keyring::use_named_store;
#[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
use keyring_core::{Entry, Error::NoEntry};

use ak_platform::prelude::BoxError;
#[cfg(target_os = "macos")]
pub mod macos;

pub mod cache;

#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_GROUP: &str = "group.232G855Y8N.io.goauthentik.platform.shared";

#[allow(unreachable_code)]
pub fn init() -> Result<(), BoxError> {
    #[cfg(any(test, debug_assertions))]
    return Ok(use_named_store("sample")?);
    // On macOS release builds the keychain is accessed directly via security-framework
    // (no keyring store needed — see the get/set/delete implementations below).
    #[cfg(target_os = "macos")]
    return Ok(());
    #[cfg(target_os = "windows")]
    return Ok(use_named_store("windows")?);
    #[cfg(target_os = "linux")]
    return Ok(use_named_store("keyutils")?);
    Err(Box::from("no keychain implementation for current OS"))
}

pub fn service(name: &str) -> String {
    #[cfg(debug_assertions)]
    return format!("io.goauthentik.agent-debug.{name}");
    #[cfg(not(debug_assertions))]
    return format!("io.goauthentik.agent.{name}");
}

#[derive(Debug)]
pub enum KeyringError {
    Other(BoxError),
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

// ---------------------------------------------------------------------------
// Fallback: all other platforms (and macOS test/debug builds) use keyring store
// ---------------------------------------------------------------------------
#[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
fn entry_modifies(
    _service: &str,
    _user: &str,
    _access: Accessibility,
) -> HashMap<&'static str, &'static str> {
    HashMap::new()
}

#[tracing::instrument(fields(service,user))]
pub async fn get(service: &str, user: &str, access: Accessibility) -> Result<String, KeyringError> {
    #[cfg(all(target_os = "macos", not(any(test, debug_assertions))))]
    return macos::get(service, user, &access);

    #[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
    {
        let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
            .map_err(|e| KeyringError::Other(e.into()))?;
        match e.get_password() {
            Ok(p) => Ok(p),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(KeyringError::Other(e.into())),
        }
    }
}

#[tracing::instrument(fields(service,user))]
pub async fn set(
    service: &str,
    user: &str,
    access: Accessibility,
    data: String,
) -> Result<(), KeyringError> {
    #[cfg(all(target_os = "macos", not(any(test, debug_assertions))))]
    return macos::set(service, user, &access, &data);

    #[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
    {
        let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
            .map_err(|e| KeyringError::Other(e.into()))?;
        match e.set_password(&data) {
            Ok(()) => Ok(()),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(KeyringError::Other(e.into())),
        }
    }
}

#[tracing::instrument(fields(service,user))]
pub async fn delete(service: &str, user: &str, access: Accessibility) -> Result<(), KeyringError> {
    #[cfg(all(target_os = "macos", not(any(test, debug_assertions))))]
    return macos::delete(service, user, &access);

    #[cfg(not(all(target_os = "macos", not(any(test, debug_assertions)))))]
    {
        let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
            .map_err(|e| KeyringError::Other(e.into()))?;
        match e.delete_credential() {
            Ok(()) => Ok(()),
            Err(NoEntry) => Ok(()),
            Err(e) => Err(KeyringError::Other(e.into())),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[tokio::test]
    async fn full() {
        init().unwrap();
        set(
            &service("foo"),
            "bar",
            Accessibility::User,
            "baz".to_string(),
        )
        .await
        .unwrap();
        assert_eq!(
            get(&service("foo"), "bar", Accessibility::User)
                .await
                .unwrap(),
            "baz"
        );
    }
}
