use chrono::{DateTime, Utc};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

use crate::{
    keyring::{self, KeyringError},
    prelude::BoxError,
};
pub trait CacheData {
    fn expiry(&self) -> DateTime<Utc>;
}

pub struct Cache<T> {
    uid: String,
    profile_name: String,
    _phantom: PhantomData<T>,
}

pub enum CacheError {
    Other(BoxError),
    Expired(),
    NotFound(),
}

impl<T> Cache<T>
where
    T: CacheData + Clone + Serialize + DeserializeOwned,
{
    pub fn new(profile_name: String, uid_parts: Vec<String>) -> Self {
        Cache {
            uid: uid_parts.join("-").replace("/", "_"),
            profile_name,
            _phantom: PhantomData,
        }
    }

    pub async fn set(self, val: T) -> Result<(), BoxError> {
        log::debug!("Writing to cache");
        let serialized = serde_json::to_string(&val).map_err(Box::new)?;
        keyring::set(
            &keyring::service(&self.uid),
            &self.profile_name,
            keyring::Accessibility::User,
            serialized,
        )
        .await
        .map_err(Box::from)
    }

    pub async fn get(self) -> Result<T, CacheError> {
        log::debug!("Checking cache");
        let cached = match keyring::get(
            &keyring::service(&self.uid),
            &self.profile_name,
            keyring::Accessibility::User,
        )
        .await
        {
            Ok(c) => c.clone(),
            Err(KeyringError::NotFound()) => return Err(CacheError::NotFound()),
            Err(KeyringError::Other(e)) => return Err(CacheError::Other(e)),
        };
        let v: T = serde_json::from_str(&cached).map_err(|e| CacheError::Other(e.into()))?;
        if v.expiry() < Utc::now() {
            keyring::delete(
                &keyring::service(&self.uid),
                &self.profile_name,
                keyring::Accessibility::User,
            )
            .await
            .map_err(|e| CacheError::Other(e.into()))?;
        }
        Ok(v)
    }
}
