use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};

use super::{Accessibility, KeyringError, KeyringStore};

// ---------------------------------------------------------------------------
// In-memory store for development (test/debug) builds.
//
// State is held on the struct; the shared `LazyLock` instance in `lib.rs` keeps
// it alive so values persist across `get()` calls within a single run, but they
// never touch the OS keychain or disk. `Accessibility` is ignored. This replaces
// the third-party `keyring-core` sample store.
// ---------------------------------------------------------------------------

type Key = (String, String);

#[derive(Default)]
pub struct MemoryStore {
    data: Mutex<HashMap<Key, String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        MemoryStore::default()
    }

    fn lock(&self) -> Result<MutexGuard<'_, HashMap<Key, String>>, KeyringError> {
        self.data
            .lock()
            .map_err(|_| KeyringError::Other(eyre::eyre!("in-memory keyring lock poisoned")))
    }
}

impl KeyringStore for MemoryStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<String, KeyringError> {
        let key = (service.to_string(), user.to_string());
        match self.lock()?.get(&key) {
            Some(v) => Ok(v.clone()),
            None => Err(KeyringError::NotFound()),
        }
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        self.lock()?
            .insert((service.to_string(), user.to_string()), data);
        Ok(())
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<(), KeyringError> {
        self.lock()?
            .remove(&(service.to_string(), user.to_string()));
        Ok(())
    }
}
