use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use keyring_core::{CredentialStore, Error::NoEntry};

use super::{Accessibility, KeyringError, KeyringStore};

/// Roams with the user profile on a domain. The historic default.
const PERSIST_ENTERPRISE: &str = "Enterprise";
/// `CRED_PERSIST_LOCAL_MACHINE` — stays on this machine.
const PERSIST_LOCAL_MACHINE: &str = "Local";

pub struct WindowsStore {
    store: Mutex<Option<Arc<CredentialStore>>>,
    persistence: &'static str,
}

impl Default for WindowsStore {
    fn default() -> Self {
        WindowsStore::new()
    }
}

impl WindowsStore {
    pub fn new() -> Self {
        WindowsStore {
            store: Mutex::new(None),
            persistence: PERSIST_ENTERPRISE,
        }
    }

    /// A store whose credentials never leave this machine. For secrets that are
    /// meaningless elsewhere — a machine-local account password, say — roaming
    /// them to a domain is pure exposure with no benefit.
    pub fn new_local_machine() -> Self {
        WindowsStore {
            store: Mutex::new(None),
            persistence: PERSIST_LOCAL_MACHINE,
        }
    }

    // Returns the cached native store, creating it on first use.
    fn store(&self) -> Result<Arc<CredentialStore>, KeyringError> {
        let mut guard = self
            .store
            .lock()
            .map_err(|_| KeyringError::Other(eyre::eyre!("windows keyring lock poisoned")))?;
        if let Some(store) = guard.as_ref() {
            return Ok(store.clone());
        }
        let store: Arc<CredentialStore> =
            windows_native_keyring_store::Store::new().map_err(map_kc)?;
        *guard = Some(store.clone());
        Ok(store)
    }

    fn entry(&self, service: &str, user: &str) -> Result<keyring_core::Entry, KeyringError> {
        // "persistence" is one of only two modifiers the backend accepts; it
        // defaults to Enterprise when absent.
        let modifiers = HashMap::from([("persistence", self.persistence)]);
        self.store()?
            .build(service, user, Some(&modifiers))
            .map_err(map_kc)
    }

    /// Credential Manager is a synchronous API, so these are the real
    /// implementations and the [`KeyringStore`] futures are thin wrappers.
    /// Callers without an async runtime — the credential provider DLL, which is
    /// loaded into LogonUI — use these directly.
    pub fn get_blocking(&self, service: &str, user: &str) -> Result<String, KeyringError> {
        match self.entry(service, user)?.get_password() {
            Ok(p) => Ok(p),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(map_kc(e)),
        }
    }

    pub fn set_blocking(&self, service: &str, user: &str, data: &str) -> Result<(), KeyringError> {
        match self.entry(service, user)?.set_password(data) {
            Ok(()) => Ok(()),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(map_kc(e)),
        }
    }

    pub fn delete_blocking(&self, service: &str, user: &str) -> Result<(), KeyringError> {
        match self.entry(service, user)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(NoEntry) => Ok(()),
            Err(e) => Err(map_kc(e)),
        }
    }
}

// Maps a keyring-core error, translating "no storage access" (the Credential Manager
// is unreachable) into `NotAvailable` so callers can distinguish a missing backend.
fn map_kc(e: keyring_core::Error) -> KeyringError {
    match e {
        keyring_core::Error::NoStorageAccess(_) => KeyringError::NotAvailable(),
        e => KeyringError::Other(eyre::Report::from(e)),
    }
}

impl KeyringStore for WindowsStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<String, KeyringError> {
        self.get_blocking(service, user)
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        self.set_blocking(service, user, &data)
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<(), KeyringError> {
        self.delete_blocking(service, user)
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::service;

    // These hit the real Credential Manager, like the macOS tests hit the real
    // keychain. Unique service names keep concurrent runs from colliding.
    fn unique_service(tag: &str) -> String {
        service(&format!("keyring-test-{tag}-{}", std::process::id()))
    }

    #[test]
    fn enterprise_store_round_trips() {
        let store = WindowsStore::new();
        let service = unique_service("enterprise");
        store.set_blocking(&service, "user", "secret").unwrap();
        assert_eq!(store.get_blocking(&service, "user").unwrap(), "secret");
        store.delete_blocking(&service, "user").unwrap();
        assert!(matches!(
            store.get_blocking(&service, "user"),
            Err(KeyringError::NotFound())
        ));
    }

    #[test]
    fn local_machine_store_round_trips() {
        let store = WindowsStore::new_local_machine();
        let service = unique_service("local");
        store.set_blocking(&service, "user", "secret").unwrap();
        assert_eq!(store.get_blocking(&service, "user").unwrap(), "secret");
        store.delete_blocking(&service, "user").unwrap();
    }

    #[test]
    fn deleting_a_missing_entry_succeeds() {
        let store = WindowsStore::new_local_machine();
        let service = unique_service("absent");
        store.delete_blocking(&service, "user").unwrap();
    }
}
