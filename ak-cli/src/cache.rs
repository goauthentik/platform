use std::{error::Error, marker::PhantomData};

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

pub trait CacheData {
    fn expiry(&self) -> Timestamp;
}

pub struct ClientCache<T: CacheData> {
    keys: Vec<String>,
    header: RequestHeader,

    c: Client<AnyService>,
    _phantom: PhantomData<T>,
}

impl<T: CacheData + Serialize + DeserializeOwned> ClientCache<T> {
    pub fn new(c: Client<AnyService>, header: RequestHeader, keys: Vec<String>) -> Self {
        Self {
            keys,
            header,
            c,
            _phantom: PhantomData,
        }
    }

    pub async fn get(&self) -> Result<T, Box<dyn Error>> {
        let res = self
            .c
            .clone()
            .cache()
            .cache_get(CacheGetRequest {
                header: Some(self.header.clone()),
                keys: self.keys.clone(),
            })
            .await?
            .into_inner();
        assert_response_valid(res.header)?;
        let value: T = serde_json::from_str(&res.value)?;
        Ok(value)
    }

    pub async fn set(&self, value: T) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string(&value)?;

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
            .await?
            .into_inner();
        assert_response_valid(res.header)?;
        Ok(())
    }
}
