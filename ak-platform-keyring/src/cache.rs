use chrono::{DateTime, TimeDelta, Utc};
use eyre::{Result, WrapErr};
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::{KeyringError, KeyringStore};

/// How long before its expiry an entry already counts as expired
pub const EXPIRY_MARGIN: TimeDelta = TimeDelta::seconds(5);

fn is_expired(expiry: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    expiry <= now + EXPIRY_MARGIN
}

pub trait CacheData {
    fn expiry(&self) -> DateTime<Utc>;
}

#[derive(Debug)]
pub struct Cache<T> {
    uid: String,
    profile_name: String,
    _phantom: PhantomData<T>,
}

#[derive(Debug)]
pub enum CacheError {
    Other(eyre::Report),
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
    pub async fn set(self, val: T) -> Result<()> {
        tracing::debug!("Writing to cache");
        let serialized = serde_json::to_string(&val).wrap_err("failed to serialize cache value")?;
        crate::store()
            .set(
                &crate::service(&self.uid),
                &self.profile_name,
                crate::Accessibility::User,
                serialized,
            )
            .await
            .map_err(|e| eyre::eyre!("keyring set failed: {e}"))
    }

    #[tracing::instrument]
    pub async fn get(self) -> Result<T, CacheError> {
        tracing::debug!("Checking cache");
        let cached = match crate::store()
            .get(
                &crate::service(&self.uid),
                &self.profile_name,
                crate::Accessibility::User,
            )
            .await
        {
            Ok(c) => c.clone(),
            Err(KeyringError::NotFound()) => return Err(CacheError::NotFound()),
            Err(KeyringError::NotAvailable()) => return Err(CacheError::NotFound()),
            Err(KeyringError::NotAccessible()) => return Err(CacheError::NotFound()),
            Err(KeyringError::Other(e)) => return Err(CacheError::Other(e)),
        };
        let v: T =
            serde_json::from_str(&cached).map_err(|e| CacheError::Other(eyre::Report::from(e)))?;
        if is_expired(v.expiry(), Utc::now()) {
            // The entry is unusable either way, and the caller's next `set` replaces it, so a
            // failed delete shouldn't turn this into an error.
            if let Err(e) = crate::store()
                .delete(
                    &crate::service(&self.uid),
                    &self.profile_name,
                    crate::Accessibility::User,
                )
                .await
            {
                tracing::warn!("failed to delete expired cache entry: {e:?}");
            }
            return Err(CacheError::Expired());
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct Entry {
        exp: DateTime<Utc>,
    }

    impl CacheData for Entry {
        fn expiry(&self) -> DateTime<Utc> {
            self.exp
        }
    }

    // Tests run in parallel against the one in-memory store, so each needs its own key.
    fn cache(key: &str) -> Cache<Entry> {
        Cache::new(
            "profile".to_string(),
            vec!["cache-tests".to_string(), key.to_string()],
        )
    }

    async fn set_expiring_in(key: &str, left: TimeDelta) -> DateTime<Utc> {
        let exp = Utc::now() + left;
        cache(key).set(Entry { exp }).await.unwrap();
        exp
    }

    #[tokio::test]
    async fn valid_entry_is_returned() {
        let exp = set_expiring_in("valid", TimeDelta::hours(1)).await;
        assert_eq!(cache("valid").get().await.unwrap().exp, exp);
        // A read leaves a valid entry in place.
        assert_eq!(cache("valid").get().await.unwrap().exp, exp);
    }

    #[tokio::test]
    async fn expired_entry_is_deleted() {
        set_expiring_in("expired", TimeDelta::seconds(-1)).await;
        let res = cache("expired").get().await;
        assert!(matches!(res, Err(CacheError::Expired())), "{res:?}");
        let res = cache("expired").get().await;
        assert!(matches!(res, Err(CacheError::NotFound())), "{res:?}");
    }

    #[tokio::test]
    async fn entry_inside_margin_is_expired() {
        // Also covers an entry whose whole lifetime is shorter than the margin.
        set_expiring_in("margin", EXPIRY_MARGIN - TimeDelta::seconds(1)).await;
        let res = cache("margin").get().await;
        assert!(matches!(res, Err(CacheError::Expired())), "{res:?}");
        let res = cache("margin").get().await;
        assert!(matches!(res, Err(CacheError::NotFound())), "{res:?}");
    }

    #[tokio::test]
    async fn missing_entry_is_not_found() {
        let res = cache("missing").get().await;
        assert!(matches!(res, Err(CacheError::NotFound())), "{res:?}");
    }

    #[test]
    fn margin_boundary() {
        let now = Utc::now();
        assert!(is_expired(now - TimeDelta::seconds(1), now));
        assert!(is_expired(now, now));
        assert!(is_expired(now + EXPIRY_MARGIN, now));
        assert!(!is_expired(
            now + EXPIRY_MARGIN + TimeDelta::seconds(1),
            now
        ));
    }
}
