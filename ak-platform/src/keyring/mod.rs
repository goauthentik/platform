use std::{collections::HashMap, error::Error, fmt::Display};

#[cfg(test)]
use keyring::use_named_store;
#[cfg(not(target_os = "macos"))]
use keyring::use_named_store;
#[cfg(target_os = "macos")]
use keyring::use_named_store_with_modifiers;
use keyring_core::{Entry, Error::NoEntry};

use crate::prelude::BoxError;

#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_GROUP: &str = "group.232G855Y8N.io.goauthentik.platform.shared";

#[allow(unreachable_code)]
pub fn init() -> Result<(), BoxError> {
    #[cfg(test)]
    return Ok(use_named_store("sample")?);
    #[cfg(target_os = "macos")]
    {
        let mut mods: HashMap<&str, &str> = HashMap::new();
        mods.insert("access-group", MACOS_KEYCHAIN_GROUP);
        return Ok(use_named_store_with_modifiers("protected", &mods)?);
    }
    #[cfg(target_os = "windows")]
    return OK(use_named_store("windows")?);
    #[cfg(target_os = "linux")]
    return OK(use_named_store("secret-service")?);
    Err(Box::from("no keychain implementation for current OS"))
}

pub fn service(name: &str) -> String {
    format!("io.goauthentik.agent.{name}")
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

pub enum Accessibility {
    Always,
    User,
}

fn entry_modifies(
    _service: &str,
    _user: &str,
    access: Accessibility,
) -> HashMap<&'static str, &'static str> {
    let mut mods: HashMap<&str, &str> = HashMap::new();
    #[cfg(all(target_os = "macos", not(test)))]
    {
        match access {
            Accessibility::User => {
                mods.insert("access-policy", "after-first-unlock");
            }
            Accessibility::Always => (),
        };
    }
    mods
}

pub async fn get(service: &str, user: &str, access: Accessibility) -> Result<String, KeyringError> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
        .map_err(|e| KeyringError::Other(e.into()))?;
    match e.get_password() {
        Ok(p) => Ok(p),
        Err(NoEntry) => Err(KeyringError::NotFound()),
        Err(e) => Err(KeyringError::Other(e.into())),
    }
}

pub async fn set(
    service: &str,
    user: &str,
    access: Accessibility,
    data: String,
) -> Result<(), KeyringError> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
        .map_err(|e| KeyringError::Other(e.into()))?;
    match e.set_password(&data) {
        Ok(()) => Ok(()),
        Err(NoEntry) => Err(KeyringError::NotFound()),
        Err(e) => Err(KeyringError::Other(e.into())),
    }
}

pub async fn delete(service: &str, user: &str, access: Accessibility) -> Result<(), KeyringError> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))
        .map_err(|e| KeyringError::Other(e.into()))?;
    match e.delete_credential() {
        Ok(()) => Ok(()),
        Err(NoEntry) => Ok(()),
        Err(e) => Err(KeyringError::Other(e.into())),
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
