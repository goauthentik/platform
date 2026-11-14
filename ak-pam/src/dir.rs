use ak_platform::{
    generated::sys_directory::{GetRequest, system_directory_client::SystemDirectoryClient},
    grpc::{Code, GrpcError, grpc_request},
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
        Err(GrpcError::Status(s)) if s.code() == Code::NotFound => {
            tracing::debug!("User {} does not exist", username);
            Err(PamResultCode::PAM_IGNORE.into())
        }
        Err(e) => {
            tracing::warn!("failed to look up user {}: {e}", username);
            Err(PamResultCode::PAM_AUTHINFO_UNAVAIL.into())
        }
    }
}
