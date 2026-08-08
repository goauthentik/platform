use std::{
    ffi::c_void,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use ak_platform::paths::xdg_data_path;
use keyring_core::{CredentialStore, Error::NoEntry};
use windows::Win32::{
    Foundation::{HLOCAL, LocalFree},
    Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
    },
};

use super::{Accessibility, KeyringError, KeyringStore};

#[derive(Default)]
pub struct WindowsStore {
    store: Mutex<Option<Arc<CredentialStore>>>,
}

impl WindowsStore {
    pub fn new() -> Self {
        WindowsStore::default()
    }

    fn storage_dir(&self) -> Result<PathBuf, KeyringError> {
        let path = xdg_data_path("tokens")
            .map_err(|e| KeyringError::Other(eyre::Report::from(e)))?;
        let dir = PathBuf::from(path);
        fs::create_dir_all(&dir)
            .map_err(|e| KeyringError::Other(eyre::Report::from(e)))?;
        Ok(dir)
    }

    fn file_path(&self, service: &str, user: &str) -> Result<PathBuf, KeyringError> {
        let dir = self.storage_dir()?;
        let filename = format!("{}-{}.bin", sanitize_path_component(service), sanitize_path_component(user));
        Ok(dir.join(filename))
    }

    fn read_file(&self, service: &str, user: &str) -> Result<Option<String>, KeyringError> {
        let path = self.file_path(service, user)?;
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => {
                return Err(KeyringError::Other(eyre::Report::from(e)));
            }
        };
        let decrypted = decrypt_bytes(&bytes)?;
        String::from_utf8(decrypted)
            .map_err(|e| KeyringError::Other(eyre::Report::from(e)))
            .map(Some)
    }

    fn write_file(&self, service: &str, user: &str, data: &str) -> Result<(), KeyringError> {
        let path = self.file_path(service, user)?;
        let encrypted = encrypt_bytes(data.as_bytes())?;
        fs::write(&path, encrypted)
            .map_err(|e| KeyringError::Other(eyre::Report::from(e)))
    }

    fn delete_file(&self, service: &str, user: &str) -> Result<(), KeyringError> {
        let path = self.file_path(service, user)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(KeyringError::Other(eyre::Report::from(e))),
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
        self.store()?.build(service, user, None).map_err(map_kc)
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

fn sanitize_path_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

fn encrypt_bytes(data: &[u8]) -> Result<Vec<u8>, KeyringError> {
    let mut data_in = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into().unwrap_or(u32::MAX),
        pbData: data.as_ptr() as *mut u8,
    };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    let result = unsafe {
        CryptProtectData(
            &mut data_in,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
    };
    if let Err(e) = result {
        return Err(KeyringError::Other(eyre::Report::from(e)));
    }
    let bytes = unsafe {
        std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec()
    };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(data_out.pbData as *mut c_void)));
    }
    Ok(bytes)
}

fn decrypt_bytes(data: &[u8]) -> Result<Vec<u8>, KeyringError> {
    let mut data_in = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into().unwrap_or(u32::MAX),
        pbData: data.as_ptr() as *mut u8,
    };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    let result = unsafe {
        CryptUnprotectData(
            &mut data_in,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
    };
    if let Err(e) = result {
        return Err(KeyringError::Other(eyre::Report::from(e)));
    }
    let bytes = unsafe {
        std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize).to_vec()
    };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(data_out.pbData as *mut c_void)));
    }
    Ok(bytes)
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
            Err(NoEntry) => match self.read_file(service, user)? {
                Some(p) => Ok(p),
                None => Err(KeyringError::NotFound()),
            },
            Err(e) => Err(map_kc(e)),
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
            Err(e) => {
                let message = e.to_string();
                if message.contains("longer than the platform limit") {
                    self.write_file(service, user, &data)
                } else {
                    Err(map_kc(e))
                }
            }
        }
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<(), KeyringError> {
        match self.entry(service, user)?.delete_credential() {
            Ok(()) => {}
            Err(NoEntry) => {}
            Err(e) => return Err(map_kc(e)),
        }
        self.delete_file(service, user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs};

    #[tokio::test]
    async fn large_payload_falls_back_to_file() {
        let store = WindowsStore::new();
        let service = "test-service";
        let user = "test-user";
        let payload = "x".repeat(5000);

        store
            .set(service, user, Accessibility::User, payload.clone())
            .await
            .unwrap();
        let loaded = store.get(service, user, Accessibility::User).await.unwrap();
        assert_eq!(loaded, payload);
        store.delete(service, user, Accessibility::User).await.unwrap();
    }

    #[tokio::test]
    async fn payload_below_windows_limit_uses_native_store() {
        let service = "test-service-native";
        let user = "test-user-native";
        let payload = "x".repeat(1000);
        let temp_dir = env::temp_dir().join(format!("ak-platform-keyring-{}", std::process::id()));
        let old_localappdata = env::var_os("LOCALAPPDATA");
        let old_appdata = env::var_os("APPDATA");

        fs::create_dir_all(&temp_dir).unwrap();
        unsafe {
            env::set_var("LOCALAPPDATA", &temp_dir);
            env::set_var("APPDATA", &temp_dir);
        }

        let store = WindowsStore::new();
        store
            .set(service, user, Accessibility::User, payload.clone())
            .await
            .unwrap();

        let file_path = temp_dir.join(format!("{}-{}.bin", sanitize_path_component(service), sanitize_path_component(user)));
        assert!(!file_path.exists(), "payload should not fall back to file storage below the platform limit");

        let loaded = store.get(service, user, Accessibility::User).await.unwrap();
        assert_eq!(loaded, payload);

        store.delete(service, user, Accessibility::User).await.unwrap();
        let _ = fs::remove_file(file_path);
        let _ = fs::remove_dir_all(&temp_dir);

        unsafe {
            match old_localappdata {
                Some(v) => env::set_var("LOCALAPPDATA", v),
                None => env::remove_var("LOCALAPPDATA"),
            }
            match old_appdata {
                Some(v) => env::set_var("APPDATA", v),
                None => env::remove_var("APPDATA"),
            }
        }
    }
}
