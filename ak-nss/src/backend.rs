use ak_platform::generated::sys_directory::{
    GetRequest, Group as AKGroup, User, system_directory_client::SystemDirectoryClient,
};
use ak_platform::grpc::grpc_request;
use eyre::Result;

pub trait DirectoryBridge {
    fn list_users(&self) -> Result<Vec<User>>;
    fn get_user(&self, req: GetRequest) -> Result<User>;
    fn list_groups(&self) -> Result<Vec<AKGroup>>;
    fn get_group(&self, req: GetRequest) -> Result<AKGroup>;
}

pub struct GrpcDirectoryBridge;

impl DirectoryBridge for GrpcDirectoryBridge {
    fn list_users(&self) -> Result<Vec<User>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_users(()).await?))
            .map(|r| r.into_inner().users)
    }

    fn get_user(&self, req: GetRequest) -> Result<User> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_user(req).await?) }
        })
        .map(|r| r.into_inner())
    }

    fn list_groups(&self) -> Result<Vec<AKGroup>> {
        grpc_request(async |ch| Ok(SystemDirectoryClient::new(ch).list_groups(()).await?))
            .map(|r| r.into_inner().groups)
    }

    fn get_group(&self, req: GetRequest) -> Result<AKGroup> {
        grpc_request(move |ch| {
            let req = req.clone();
            async move { Ok(SystemDirectoryClient::new(ch).get_group(req).await?) }
        })
        .map(|r| r.into_inner())
    }
}
