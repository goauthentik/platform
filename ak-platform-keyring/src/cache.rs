use chrono::{DateTime, Utc};
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use std::marker::PhantomData;

use ak_platform::prelude::BoxError;

use crate::KeyringError;

pub trait CacheData {
    fn expiry(&self) -> DateTime<Utc>;
}

#[derive(Debug)]
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
    T: CacheData + Clone + Serialize + DeserializeOwned + Debug,
{
    pub fn new(profile_name: String, uid_parts: Vec<String>) -> Self {
        Cache {
            uid: uid_parts.join("-").replace("/", "_"),
            profile_name,
            _phantom: PhantomData,
        }
    }

    #[tracing::instrument]
    pub async fn set(self, val: T) -> Result<(), BoxError> {
        tracing::debug!("Writing to cache");
        let serialized = serde_json::to_string(&val).map_err(Box::new)?;
        crate::set(
            &crate::service(&self.uid),
            &self.profile_name,
            crate::Accessibility::User,
            serialized,
        )
        .await
        .map_err(Box::from)
    }

    #[tracing::instrument]
    pub async fn get(self) -> Result<T, CacheError> {
        tracing::debug!("Checking cache");
        let cached = match crate::get(
            &crate::service(&self.uid),
            &self.profile_name,
            crate::Accessibility::User,
        )
        .await
        {
            Ok(c) => c.clone(),
            Err(KeyringError::NotFound()) => return Err(CacheError::NotFound()),
            Err(KeyringError::Other(e)) => return Err(CacheError::Other(e)),
        };
        let v: T = serde_json::from_str(&cached).map_err(|e| CacheError::Other(e.into()))?;
        if v.expiry() < Utc::now() {
            crate::delete(
                &crate::service(&self.uid),
                &self.profile_name,
                crate::Accessibility::User,
            )
            .await
            .map_err(|e| CacheError::Other(e.into()))?;
        }
        Ok(v)
    }
}
