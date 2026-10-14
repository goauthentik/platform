use std::{collections::HashMap, io::ErrorKind};

use secret_service::{EncryptionType, SecretService};
use tokio::sync::OnceCell;

use super::{Accessibility, KeyringError, KeyringStore};

#[derive(Default)]
pub struct LinuxStore {
    service: OnceCell<SecretService<'static>>,
}

impl LinuxStore {
    pub fn new() -> Self {
        LinuxStore::default()
    }

    // Returns the shared connection, connecting on first use.
    async fn service(&self) -> Result<&SecretService<'static>, KeyringError> {
        self.service
            .get_or_try_init(|| async {
                SecretService::connect(EncryptionType::Dh)
                    .await
                    .map_err(map_ss)
            })
            .await
    }

    fn attributes<'a>(&self, service: &'a str, user: &'a str) -> HashMap<&'a str, &'a str> {
        HashMap::from([("service", service), ("username", user)])
    }
}

impl KeyringStore for LinuxStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<String, KeyringError> {
        let ss = self.service().await?;
        let collection = ss.get_default_collection().await.map_err(map_ss)?;
        collection.ensure_unlocked().await.map_err(map_ss)?;

        let results = ss
            .search_items(self.attributes(service, user))
            .await
            .map_err(map_ss)?;

        let item = if let Some(item) = results.unlocked.into_iter().next() {
            item
        } else if let Some(item) = results.locked.into_iter().next() {
            item.unlock().await.map_err(map_ss)?;
            item
        } else {
            return Err(KeyringError::NotFound());
        };

        let bytes = item.get_secret().await.map_err(map_ss)?;
        String::from_utf8(bytes).map_err(|e| KeyringError::Other(eyre::Report::from(e)))
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        let ss = self.service().await?;
        let collection = ss.get_default_collection().await.map_err(map_ss)?;
        collection.ensure_unlocked().await.map_err(map_ss)?;

        collection
            .create_item(
                service,
                self.attributes(service, user),
                data.as_bytes(),
                true, // replace any existing item with matching attributes
                "text/plain",
            )
            .await
            .map_err(map_ss)?;
        Ok(())
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        _access: Accessibility,
    ) -> Result<(), KeyringError> {
        let ss = self.service().await?;
        let collection = ss.get_default_collection().await.map_err(map_ss)?;
        collection.ensure_unlocked().await.map_err(map_ss)?;

        let results = ss
            .search_items(self.attributes(service, user))
            .await
            .map_err(map_ss)?;

        for item in results.unlocked.into_iter().chain(results.locked) {
            item.delete().await.map_err(map_ss)?;
        }
        Ok(())
    }
}

// Maps a Secret Service error, translating "no provider / no D-Bus session" into
// `NotAvailable` so callers can distinguish a missing backend from other failures.
fn map_ss(e: secret_service::Error) -> KeyringError {
    let Ok(method_not_available) =
        zbus_names::OwnedErrorName::try_from("org.freedesktop.DBus.Error.ServiceUnknown")
    else {
        return KeyringError::NotAvailable();
    };
    match e {
        secret_service::Error::Unavailable => KeyringError::NotAvailable(),
        secret_service::Error::Zbus(e) => {
            match e.clone() {
                // Handle error when DBus is available, but no secret service is registered
                zbus::Error::MethodError(err_name, _, _) => {
                    if err_name == method_not_available {
                        KeyringError::NotAvailable()
                    } else {
                        KeyringError::Other(eyre::Report::from(e))
                    }
                }
                // Handle error when DBus socket does not exist
                zbus::Error::Connection(e, _) => {
                    if e.kind() == ErrorKind::NotFound {
                        KeyringError::NotAvailable()
                    } else {
                        KeyringError::Other(eyre::Report::from(e))
                    }
                }
                _ => KeyringError::Other(eyre::Report::from(e)),
            }
        }
        e => KeyringError::Other(eyre::Report::from(e)),
    }
}

#[cfg(all(test, target_os = "linux"))]
pub mod tests {
    use std::env;

    use super::*;
    use crate::service;

    #[tokio::test]
    async fn error_not_available() {
        let ls = LinuxStore::new();
        let err = ls
            .set(
                &service("foo"),
                "foo",
                Accessibility::Always,
                "bar".to_string(),
            )
            .await
            .err()
            .expect("should error");
        assert!(matches!(err, KeyringError::NotAvailable()));
    }

    #[tokio::test]
    async fn test_no_dbus() {
        unsafe {
            env::set_var("DBUS_SESSION_BUS_ADDRESS", "unix:path=/tmp/foo");
        };
        let ls = LinuxStore::new();
        let err = ls
            .set(
                &service("foo"),
                "foo",
                Accessibility::Always,
                "bar".to_string(),
            )
            .await
            .err()
            .expect("should error");
        assert!(matches!(err, KeyringError::NotAvailable()));
    }
}
