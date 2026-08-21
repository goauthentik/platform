use eyre::Result;
use tonic::transport::Channel;

use crate::{
    generated::{
        ping::ping_client::PingClient, session::session_manager_client::SessionManagerClient,
        sys_auth::system_auth_token_client::SystemAuthTokenClient,
        sys_ctrl::system_ctrl_client::SystemCtrlClient,
        sys_directory::system_directory_client::SystemDirectoryClient,
        sys_platform::system_platform_client::SystemPlatformClient,
    },
    grpc::grpc_endpoint,
    paths::{SysdSocketID, sysd_socket_path},
};

#[derive(Clone)]
pub struct Client {
    c: Channel,
}

impl Client {
    pub async fn new(id: SysdSocketID) -> Result<Self> {
        let c = grpc_endpoint(sysd_socket_path(id).for_current()).await?;
        Ok(Client { c })
    }

    /// Connects to the CTRL socket via [`crate::net::elevate`] instead of
    /// dialing it directly — for unprivileged callers (the desktop app) that
    /// can't open `SysdSocketID::CTRL` themselves. Triggers the platform's
    /// native elevation prompt (or, on macOS, requires the helper daemon to
    /// already be registered/approved — see `net::elevate::macos`).
    pub async fn new_elevated_ctrl() -> Result<Self> {
        let c = crate::net::elevate::elevated_sysd_ctrl_channel().await?;
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

    pub fn platform(self) -> SystemPlatformClient<Channel> {
        SystemPlatformClient::new(self.c)
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
