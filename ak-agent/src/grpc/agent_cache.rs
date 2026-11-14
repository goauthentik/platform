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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ak_platform::generated::agent::RequestHeader;
    use ak_platform::storage::cfgmgr::testutils::test_config_manager;
    use ak_platform_keyring::cache::EXPIRY_MARGIN;
    use chrono::TimeDelta;

    use super::*;
    use crate::Agent;
    use crate::config::ConfigV1;
    use crate::token::global::GlobalTokenManager;
    use crate::token::testutils::gtm_test_lock;

    fn header() -> Option<RequestHeader> {
        Some(RequestHeader {
            profile: "profile".to_string(),
        })
    }

    /// Caches a value under `key` that expires at `exp`, then reads it back. The key prefix
    /// keeps it apart from other tests sharing the in-memory keyring.
    async fn set_then_get(
        server: &AgentGRPCServer,
        key: &str,
        exp: DateTime<Utc>,
    ) -> CacheGetResponse {
        let keys = vec!["agent-cache-tests".to_string(), key.to_string()];
        server
            .cache_set(Request::new(CacheSetRequest {
                header: header(),
                keys: keys.clone(),
                expiry: Some(exp.into()),
                value: "value".to_string(),
            }))
            .await
            .unwrap();
        server
            .cache_get(Request::new(CacheGetRequest {
                header: header(),
                keys,
            }))
            .await
            .unwrap()
            .into_inner()
    }

    #[tokio::test]
    async fn cache_get_only_returns_valid_entries() {
        let _guard = gtm_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let cfg = test_config_manager::<ConfigV1>();
        let gtm = GlobalTokenManager::new(Arc::clone(&cfg)).await.unwrap();
        let server = AgentGRPCServer::new(Arc::new(Agent {
            cfg,
            gtm: Arc::new(gtm),
        }))
        .await
        .unwrap();

        let exp = Utc::now() + TimeDelta::hours(1);
        let res = set_then_get(&server, "valid", exp).await;
        assert_eq!(res.status(), CacheStatus::Valid);
        assert_eq!(res.value, "value");
        assert_eq!(res.expiry, Some(exp.into()));

        for (key, left) in [
            ("expired", TimeDelta::seconds(-1)),
            ("expiring", EXPIRY_MARGIN - TimeDelta::seconds(1)),
        ] {
            let res = set_then_get(&server, key, Utc::now() + left).await;
            assert_eq!(res.status(), CacheStatus::NotFound, "{key}");
            assert_eq!(res.value, "", "{key}");
        }
    }
}
