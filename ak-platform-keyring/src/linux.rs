use std::collections::HashMap;

use secret_service::{EncryptionType, SecretService};

use super::{Accessibility, KeyringError, KeyringStore};

#[derive(Default)]
pub struct LinuxStore ;

impl LinuxStore {
    pub fn new() -> Self {
        LinuxStore
    }
}

impl KeyringStore for LinuxStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> Result<String, KeyringError> {
        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(other)?;
        let collection = ss.get_default_collection().await.map_err(other)?;
        collection.ensure_unlocked().await.map_err(other)?;

        let results = ss
            .search_items(attributes(service, user))
            .await
            .map_err(other)?;

        let item = if let Some(item) = results.unlocked.into_iter().next() {
            item
        } else if let Some(item) = results.locked.into_iter().next() {
            item.unlock().await.map_err(other)?;
            item
        } else {
            return Err(KeyringError::NotFound());
        };

        let bytes = item.get_secret().await.map_err(other)?;
        String::from_utf8(bytes).map_err(other)
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(other)?;
        let collection = ss.get_default_collection().await.map_err(other)?;
        collection.ensure_unlocked().await.map_err(other)?;

        collection
            .create_item(
                service,
                attributes(service, user),
                data.as_bytes(),
                true, // replace any existing item with matching attributes
                "text/plain",
            )
            .await
            .map_err(other)?;
        Ok(())
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> Result<(), KeyringError> {
        let ss = SecretService::connect(EncryptionType::Dh)
            .await
            .map_err(other)?;
        let collection = ss.get_default_collection().await.map_err(other)?;
        collection.ensure_unlocked().await.map_err(other)?;

        let results = ss
            .search_items(attributes(service, user))
            .await
            .map_err(other)?;

        for item in results.unlocked.into_iter().chain(results.locked) {
            item.delete().await.map_err(other)?;
        }
        Ok(())
    }
}

fn attributes<'a>(service: &'a str, user: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("service", service), ("username", user)])
}

fn other<E>(e: E) -> KeyringError
where
    E: std::error::Error + Send + Sync + 'static,
{
    KeyringError::Other(eyre::Report::from(e))
}
