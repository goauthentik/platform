use std::{collections::HashMap, error::Error};

use keyring::use_named_store;
use keyring_core::Entry;

pub fn init() -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    use_named_store("protected")?;
    #[cfg(target_os = "windows")]
    use_named_store("windows")?;
    #[cfg(target_os = "linux")]
    use_named_store("secret-service")?;
    Ok(())
}

pub fn service(name: &str) -> String {
    format!("io.goauthentik.agent.{name}")
}

pub enum Accessibility {
    Always,
    User,
}

const MACOS_KEYCHAIN_GROUP: &str = "group.232G855Y8N.io.goauthentik.platform.shared";

fn entry_modifies(
    _service: &str,
    _user: &str,
    access: Accessibility,
) -> HashMap<&'static str, &'static str> {
    let mut mods: HashMap<&str, &str> = HashMap::new();
    #[cfg(target_os = "macos")]
    {
        mods.insert("access-policy", MACOS_KEYCHAIN_GROUP);
        match access {
            Accessibility::User => {
                mods.insert("access-policy", "after-first-unlock");
            }
            Accessibility::Always => (),
        };
    }
    mods
}

pub async fn get(
    service: &str,
    user: &str,
    access: Accessibility,
) -> Result<String, Box<dyn Error>> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))?;
    match e.get_password() {
        Ok(p) => Ok(p),
        Err(e) => Err(Box::from(e)),
    }
}

pub async fn set(
    service: &str,
    user: &str,
    access: Accessibility,
    data: String,
) -> Result<(), Box<dyn Error>> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))?;
    e.set_password(&data)?;
    Ok(())
}

pub async fn delete(
    service: &str,
    user: &str,
    access: Accessibility,
) -> Result<(), Box<dyn Error>> {
    let e = Entry::new_with_modifiers(service, user, &entry_modifies(service, user, access))?;
    e.delete_credential()?;
    Ok(())
}
