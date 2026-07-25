use std::sync::{Arc, Mutex};

use keyring_core::{CredentialStore, Error::NoEntry, api::CredentialStoreApi};

use super::{Accessibility, KeyringError, KeyringStore};

#[derive(Default)]
pub struct WindowsStore {
    store: Mutex<Option<Arc<CredentialStore>>>,
}

impl WindowsStore {
    pub fn new() -> Self {
        WindowsStore::default()
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
        let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new().map_err(other)?;
        *guard = Some(store.clone());
        Ok(store)
    }

    fn entry(&self, service: &str, user: &str) -> Result<keyring_core::Entry, KeyringError> {
        self.store()?.build(service, user, None).map_err(other)
    }
}

fn other<E>(e: E) -> KeyringError
where
    E: std::error::Error + Send + Sync + 'static,
{
    KeyringError::Other(eyre::Report::from(e))
}

impl KeyringStore for WindowsStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<String, KeyringError> {
        match self.entry(service, user)?.get_password() {
            Ok(p) => Ok(p),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(other(e)),
        }
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        match self.entry(service, user)?.set_password(&data) {
            Ok(()) => Ok(()),
            Err(NoEntry) => Err(KeyringError::NotFound()),
            Err(e) => Err(other(e)),
        }
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<(), KeyringError> {
        match self.entry(service, user)?.delete_credential() {
            Ok(()) => Ok(()),
            Err(NoEntry) => Ok(()),
            Err(e) => Err(other(e)),
        }
    }
}
