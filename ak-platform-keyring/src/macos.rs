use core_foundation::base::TCFType;
use core_foundation::string::{CFString, CFStringRef};
use security_framework::passwords::{
    PasswordOptions, delete_generic_password_options, generic_password,
    set_generic_password_options,
};

const MACOS_KEYCHAIN_GROUP: &str = "group.232G855Y8N.io.goauthentik.platform.shared";

// ---------------------------------------------------------------------------
// macOS release: call Security.framework directly via security-framework.
//
// The "protected" store (apple-native-keyring-store) sets
// kSecUseDataProtectionKeychain=true, which routes items to the Data
// Protection Keychain and attaches a SecAccessControl object.
// ---------------------------------------------------------------------------

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecAttrAccessibleAfterFirstUnlock: CFStringRef;
    static kSecAttrAccessibleWhenUnlocked: CFStringRef;
}

use super::{Accessibility, KeyringError, KeyringStore};

#[derive(Default)]
pub struct MacosStore;

impl MacosStore {
    pub fn new() -> Self {
        MacosStore
    }

    fn build_options(&self, service: &str, user: &str, access: &Accessibility) -> PasswordOptions {
        let mut options = PasswordOptions::new_generic_password(service, user);
        options.set_access_group(MACOS_KEYCHAIN_GROUP);
        // Set kSecAttrAccessible (the attribute key is "pdmn" internally).
        // Using this simple attribute — rather than kSecAttrAccessControl with a
        // SecAccessControl object — means no TouchID prompt is ever attached to
        // the item, matching what go-keychain does.
        let accessible_val = unsafe {
            match access {
                Accessibility::User => kSecAttrAccessibleAfterFirstUnlock,
                Accessibility::Always => kSecAttrAccessibleWhenUnlocked,
            }
        };
        #[allow(deprecated)]
        unsafe {
            options.query.push((
                CFString::from("pdmn"),
                CFString::wrap_under_get_rule(accessible_val).into_CFType(),
            ));
        }
        options
    }
}

impl KeyringStore for MacosStore {
    async fn get(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> Result<String, KeyringError> {
        let options = self.build_options(service, user, &access);
        match generic_password(options) {
            Ok(bytes) => {
                String::from_utf8(bytes).map_err(|e| KeyringError::Other(eyre::Report::from(e)))
            }
            Err(e) if e.code() == -25300 => Err(KeyringError::NotFound()),
            Err(e) if e.code() == -25291 => Err(KeyringError::NotAvailable()),
            Err(e) => Err(KeyringError::Other(eyre::Report::from(e))),
        }
    }

    async fn set(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
        data: String,
    ) -> Result<(), KeyringError> {
        let options = self.build_options(service, user, &access);
        match set_generic_password_options(data.as_bytes(), options) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == -25291 => Err(KeyringError::NotAvailable()),
            Err(e) => Err(KeyringError::Other(eyre::Report::from(e))),
        }
    }

    async fn delete(
        &self,
        service: &str,
        user: &str,
        access: Accessibility,
    ) -> Result<(), KeyringError> {
        let options = self.build_options(service, user, &access);
        match delete_generic_password_options(options) {
            Ok(()) => Ok(()),
            Err(e) if e.code() == -25300 => Ok(()),
            Err(e) if e.code() == -25291 => Err(KeyringError::NotAvailable()),
            Err(e) => Err(KeyringError::Other(eyre::Report::from(e))),
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
pub mod tests {
    use super::*;
    use crate::service;

    #[tokio::test]
    async fn full() {
        let store = MacosStore::new();
        store
            .set(
                &service("foo"),
                "bar",
                Accessibility::User,
                "baz".to_owned(),
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .get(&service("foo"), "bar", Accessibility::User)
                .await
                .unwrap(),
            "baz"
        );
    }
}
