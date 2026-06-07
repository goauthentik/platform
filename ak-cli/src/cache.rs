use std::{error::Error, marker::PhantomData};

use authentik_sys::{
    generated::{
        agent::RequestHeader,
        agent_cache::{CacheGetRequest, CacheSetRequest, agent_cache_client::AgentCacheClient},
    },
    grpc::{assert_response_valid, grpc_endpoint},
    platform::paths::{AgentSocketID, agent_socket_path},
};
use pbjson_types::Timestamp;
use serde::{Serialize, de::DeserializeOwned};

pub trait CacheData {
    fn expiry(&self) -> Timestamp;
}

pub struct ClientCache<T: CacheData> {
    keys: Vec<String>,
    header: RequestHeader,

    _phantom: PhantomData<T>,
}

impl<T: CacheData + Serialize + DeserializeOwned> ClientCache<T> {
    pub fn new(header: RequestHeader, keys: Vec<String>) -> Self {
        Self {
            keys,
            header,
            _phantom: PhantomData,
        }
    }

    pub async fn get(&self) -> Result<T, Box<dyn Error>> {
        let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
        let res = AgentCacheClient::new(c)
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

        let c = grpc_endpoint(agent_socket_path(AgentSocketID::Default)?.for_current()).await?;
        let res = AgentCacheClient::new(c)
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
