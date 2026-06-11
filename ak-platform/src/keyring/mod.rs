use std::{collections::HashMap, error::Error};

#[cfg(not(target_os = "macos"))]
use keyring::use_named_store;
#[cfg(target_os = "macos")]
use keyring::use_named_store_with_modifiers;
use keyring_core::Entry;

#[cfg(target_os = "macos")]
const MACOS_KEYCHAIN_GROUP: &str = "group.232G855Y8N.io.goauthentik.platform.shared";

pub fn init() -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "macos")]
    {
        let mut mods: HashMap<&str, &str> = HashMap::new();
        mods.insert("access-group", MACOS_KEYCHAIN_GROUP);
        use_named_store_with_modifiers("protected", &mods)?;
    }
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

fn entry_modifies(
    _service: &str,
    _user: &str,
    access: Accessibility,
) -> HashMap<&'static str, &'static str> {
    let mut mods: HashMap<&str, &str> = HashMap::new();
    #[cfg(target_os = "macos")]
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
