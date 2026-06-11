use std::error::Error;

use tonic::transport::Channel;

use crate::{
    generated::{
        agent_platform::agent_platform_client::AgentPlatformClient, ping::ping_client::PingClient,
        session::session_manager_client::SessionManagerClient,
        sys_auth::system_auth_token_client::SystemAuthTokenClient,
        sys_ctrl::system_ctrl_client::SystemCtrlClient,
        sys_directory::system_directory_client::SystemDirectoryClient,
    },
    grpc::grpc_endpoint,
    paths::{SysdSocketID, sysd_socket_path},
};

#[derive(Clone)]
pub struct Client {
    c: Channel,
}

impl Client {
    pub async fn new(id: SysdSocketID) -> Result<Self, Box<dyn Error>> {
        let c = grpc_endpoint(sysd_socket_path(id).for_current()).await?;
        Ok(Client { c })
    }

    pub fn new_channel(c: Channel) -> Self {
        Client { c }
    }

    pub fn auth_token(self) -> SystemAuthTokenClient<Channel> {
        SystemAuthTokenClient::new(self.c)
    }

    pub fn session(self) -> SessionManagerClient<Channel> {
        SessionManagerClient::new(self.c)
    }

    pub fn platform(self) -> AgentPlatformClient<Channel> {
        AgentPlatformClient::new(self.c)
    }

    pub fn ping(self) -> PingClient<Channel> {
        PingClient::new(self.c)
    }

    pub fn ctrl(self) -> SystemCtrlClient<Channel> {
        SystemCtrlClient::new(self.c)
    }

    pub fn directory(self) -> SystemDirectoryClient<Channel> {
        SystemDirectoryClient::new(self.c)
    }
}
