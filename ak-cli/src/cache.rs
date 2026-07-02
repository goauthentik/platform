use eyre::{Result, WrapErr};
use ak_platform::{
    client::user::{AnyService, Client},
    generated::{
        agent::RequestHeader,
        agent_cache::{CacheGetRequest, CacheSetRequest},
    },
    grpc::assert_response_valid,
};
use pbjson_types::Timestamp;
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;

pub trait CacheData {
    fn expiry(&self) -> Timestamp;
}

pub struct ClientCache<T: CacheData> {
    keys: Vec<String>,
    header: RequestHeader,

    c: Client<AnyService>,
    _phantom: PhantomData<T>,
}

impl<T> ClientCache<T>
where
    T: CacheData + Serialize + DeserializeOwned,
{
    pub fn new(c: Client<AnyService>, header: RequestHeader, keys: Vec<String>) -> Self {
        Self {
            keys,
            header,
            c,
            _phantom: PhantomData,
        }
    }

    pub async fn get(&self) -> Result<T> {
        let res = self
            .c
            .clone()
            .cache()
            .cache_get(CacheGetRequest {
                header: Some(self.header.clone()),
                keys: self.keys.clone(),
            })
            .await
            .wrap_err("cache get RPC failed")?
            .into_inner();
        assert_response_valid(res.header).map_err(|e| eyre::eyre!("{e}"))?;
        let value: T = serde_json::from_str(&res.value).wrap_err("failed to deserialize cached value")?;
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
                keys: self.keys.clone(),
                expiry: Some(expiry_ts),
                value: json,
            })
            .await
            .wrap_err("cache set RPC failed")?
            .into_inner();
        assert_response_valid(res.header).map_err(|e| eyre::eyre!("{e}"))?;
        Ok(())
    }
}
