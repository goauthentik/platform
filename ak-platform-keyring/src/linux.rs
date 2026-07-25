use std::collections::HashMap;

use secret_service::{EncryptionType, SecretService};

use super::{Accessibility, KeyringError};

// ---------------------------------------------------------------------------
// Linux release: talk to the freedesktop Secret Service (GNOME Keyring /
// KWallet) over D-Bus via the `secret-service` crate.
//
// This crate is NOT a `keyring-core::Store`, so — like the macOS module — we
// call it directly rather than registering a default store.  Secrets land in
// the desktop's default collection, are persisted to disk, and survive logout
// (unlike the previous keyutils kernel-keyring backend).
//
// `Accessibility` has no Secret Service equivalent (there is no
// after-first-unlock vs. when-unlocked distinction), so it is ignored here,
// matching the previous keyutils behavior.
// ---------------------------------------------------------------------------

fn attributes<'a>(service: &'a str, user: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("service", service), ("username", user)])
}

fn other<E>(e: E) -> KeyringError
where
    E: std::error::Error + Send + Sync + 'static,
{
    KeyringError::Other(eyre::Report::from(e))
}

pub async fn get(service: &str, user: &str, _access: &Accessibility) -> Result<String, KeyringError> {
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

pub async fn set(
    service: &str,
    user: &str,
    _access: &Accessibility,
    data: &str,
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

pub async fn delete(service: &str, user: &str, _access: &Accessibility) -> Result<(), KeyringError> {
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
