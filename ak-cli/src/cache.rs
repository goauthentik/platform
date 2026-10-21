use ak_platform::{
    client::user::{AnyService, Client},
    generated::{
        agent::RequestHeader,
        agent_cache::{CacheGetRequest, CacheSetRequest},
    },
    grpc::assert_response_valid,
};
use eyre::{Result, WrapErr};
use pbjson_types::Timestamp;
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::marker::PhantomData;

pub trait CacheData {
    fn expiry(&self) -> Timestamp;
}

pub struct ClientCache<T: CacheData> {
    additional_keys: Vec<String>,
    header: RequestHeader,
    client_id: String,

    c: Client<AnyService>,
    _phantom: PhantomData<T>,
}

impl<T> ClientCache<T>
where
    T: CacheData + Serialize + DeserializeOwned,
{
    pub fn new(
        c: Client<AnyService>,
        header: RequestHeader,
        client_id: String,
        additional_keys: Vec<String>,
    ) -> Self {
        Self {
            additional_keys,
            header,
            c,
            client_id,
            _phantom: PhantomData,
        }
    }

    pub fn keys(&self) -> Vec<String> {
        let mut hasher = Sha256::new();
        for key in self.additional_keys.clone() {
            hasher.update(key);
        }
        vec![
            // v2 to discern the new format which always contains the client_id at the start
            "v2".to_string(),
            self.client_id.clone(),
            hex::encode(hasher.finalize()),
        ]
    }

    pub async fn get(&self) -> Result<T> {
        let res = self
            .c
            .clone()
            .cache()
            .cache_get(CacheGetRequest {
                header: Some(self.header.clone()),
                keys: self.keys(),
            })
            .await
            .wrap_err("cache get RPC failed")?
            .into_inner();
        assert_response_valid(res.header)?;
        let value: T =
            serde_json::from_str(&res.value).wrap_err("failed to deserialize cached value")?;
        Ok(value)
    }

    pub async fn set(&self, value: T) -> Result<()> {
        let json = serde_json::to_string(&value).wrap_err("failed to serialize value for cache")?;

        let expiry_ts = value.expiry();

        let res = self
            .c
            .clone()
            .cache()
            .cache_set(CacheSetRequest {
                header: Some(self.header.clone()),
                keys: self.keys(),
                expiry: Some(expiry_ts),
                value: json,
            })
            .await
            .wrap_err("cache set RPC failed")?
            .into_inner();
        assert_response_valid(res.header)?;
        Ok(())
    }
}
