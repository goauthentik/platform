use ak_platform::generated::sys_directory::{
    GetRequest, Group as AKGroup, User, system_directory_client::SystemDirectoryClient,
};
use ak_platform::grpc::{GrpcError, GrpcResult, grpc_request};
use eyre::Result;

pub trait DirectoryBridge {
    fn list_users(&self) -> GrpcResult<Vec<User>>;
    fn get_user(&self, req: GetRequest) -> GrpcResult<User>;
    fn list_groups(&self) -> GrpcResult<Vec<AKGroup>>;
    fn get_group(&self, req: GetRequest) -> GrpcResult<AKGroup>;
}

pub struct GrpcDirectoryBridge;

impl DirectoryBridge for GrpcDirectoryBridge {
    fn list_users(&self) -> GrpcResult<Vec<User>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_users(()).await?))
            .map(|r| r.into_inner().users)
    }

    fn get_user(&self, req: GetRequest) -> GrpcResult<User> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_user(req).await?) }
        })
        .map(|r| r.into_inner())
    }

    fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_groups(()).await?))
            .map(|r| r.into_inner().groups)
    }

    fn get_group(&self, req: GetRequest) -> GrpcResult<AKGroup> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_group(req).await?) }
        })
        .map(|r| r.into_inner())
    }
}
