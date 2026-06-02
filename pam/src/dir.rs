use authentik_sys::{
    generated::sys_directory::{GetRequest, system_directory_client::SystemDirectoryClient},
    grpc::grpc_request,
};
use pam::constants::PamResultCode::{self, PAM_IGNORE};

pub fn check_user_exists(username: String) -> Result<(), PamResultCode> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch)
            .get_user(GetRequest {
                name: Some(username.clone()),
                id: None,
            })
            .await?);
    }) {
        Ok(_) => Ok(()),
        Err(_) => {
            log::debug!("User {} does not exist", username);
            Err(PAM_IGNORE)
        }
    }
}
