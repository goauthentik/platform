use ak_platform::generated::agent_cache::{
    CacheGetRequest, CacheGetResponse, CacheSetRequest, CacheSetResponse,
    agent_cache_server::AgentCache,
};
use tonic::{Request, Response, Status};

use crate::grpc::AgentGRPCServer;

#[tonic::async_trait]
impl AgentCache for AgentGRPCServer {
    async fn cache_get(
        &self,
        _request: Request<CacheGetRequest>,
    ) -> Result<Response<CacheGetResponse>, Status> {
        todo!()
    }

    async fn cache_set(
        &self,
        _request: Request<CacheSetRequest>,
    ) -> Result<Response<CacheSetResponse>, Status> {
        todo!()
    }
}
