use ak_platform::{
    generated::sys_directory::{GetRequest, system_directory_client::SystemDirectoryClient},
    grpc::grpc_request,
};
use pam::constants::PamResultCode;

use crate::PamError;

pub fn check_user_exists(username: String) -> Result<(), PamError> {
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
            tracing::debug!("User {} does not exist", username);
            Err(PamResultCode::PAM_IGNORE.into())
        }
    }
}
