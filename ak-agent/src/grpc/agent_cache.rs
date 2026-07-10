use std::fmt::Debug;

use crate::grpc::AgentGRPCServer;
use ak_platform::generated::{
    agent::ResponseHeader,
    agent_cache::{
        CacheGetRequest, CacheGetResponse, CacheSetRequest, CacheSetResponse, CacheStatus,
        agent_cache_server::AgentCache,
    },
};
use ak_platform_keyring::cache::{Cache, CacheData, CacheError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentCacheEntry {
    pub body: String,
    pub exp: DateTime<Utc>,
}

impl Debug for AgentCacheEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentCacheEntry")
            .field("body", &self.body.len())
            .field("exp", &self.exp)
            .finish()
    }
}

impl CacheData for AgentCacheEntry {
    fn expiry(&self) -> DateTime<Utc> {
        self.exp
    }
}

#[tonic::async_trait]
impl AgentCache for AgentGRPCServer {
    async fn cache_get(
        &self,
        request: Request<CacheGetRequest>,
    ) -> Result<Response<CacheGetResponse>, Status> {
        let b = request.into_inner();
        let header = match b.header {
            Some(h) => h,
            None => return Err(Status::invalid_argument("missing header")),
        };
        let cache: Cache<AgentCacheEntry> = Cache::new(header.profile, b.keys);
        let res = match cache.get().await {
            Ok(r) => r,
            Err(CacheError::NotFound()) => {
                return Ok(Response::new(CacheGetResponse {
                    header: Some(ResponseHeader { successful: false }),
                    status: CacheStatus::NotFound.into(),
                    expiry: None,
                    value: "".to_string(),
                }));
            }
            Err(CacheError::Expired()) => {
                return Ok(Response::new(CacheGetResponse {
                    header: Some(ResponseHeader { successful: false }),
                    status: CacheStatus::NotFound.into(),
                    expiry: None,
                    value: "".to_string(),
                }));
            }
            Err(CacheError::Other(e)) => {
                return Err(Status::from_error(e.into()));
            }
        };
        Ok(Response::new(CacheGetResponse {
            header: Some(ResponseHeader { successful: true }),
            status: CacheStatus::Valid.into(),
            expiry: Some(res.exp.into()),
            value: res.body,
        }))
    }

    async fn cache_set(
        &self,
        request: Request<CacheSetRequest>,
    ) -> Result<Response<CacheSetResponse>, Status> {
        let b = request.into_inner();
        let header = match b.header {
            Some(h) => h,
            None => return Err(Status::invalid_argument("missing header")),
        };
        let cache: Cache<AgentCacheEntry> = Cache::new(header.profile, b.keys);
        match cache
            .set(AgentCacheEntry {
                body: b.value,
                exp: b
                    .expiry
                    .unwrap_or_default()
                    .try_into()
                    .map_err(|e| Status::from_error(Box::from(e)))?,
            })
            .await
        {
            Ok(()) => Ok(Response::new(CacheSetResponse {
                header: Some(ResponseHeader { successful: true }),
            })),
            Err(e) => Err(Status::from_error(e.into())),
        }
    }
}
