use ak_platform::generated::sys_directory::GetRequest;
use ak_platform::generated::sys_directory::system_directory_client::SystemDirectoryClient;
use ak_platform::grpc::grpc_request;
use libnss::interop::Response;
use libnss::shadow::{Shadow, ShadowHooks};

use crate::AuthentikNSS;

impl ShadowHooks for AuthentikNSS {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Shadow>> {
        match grpc_request(async |ch| {
            return Ok(SystemDirectoryClient::new(ch).list_users(()).await?);
        }) {
            Ok(r) => {
                let users: Vec<Shadow> = r
                    .into_inner()
                    .users
                    .into_iter()
                    .map(|user| Self::shadow_entry(user.name))
                    .collect();
                Response::Success(users)
            }
            Err(e) => {
                tracing::warn!("Failed to get users: {e:?}");
                Response::Unavail
            }
        }
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Shadow> {
        match grpc_request(async |ch| {
            return Ok(SystemDirectoryClient::new(ch)
                .get_user(GetRequest {
                    name: Some(name.clone()),
                    id: None,
                })
                .await?);
        }) {
            Ok(r) => Response::Success(Self::shadow_entry(r.into_inner().name)),
            Err(e) => {
                tracing::warn!("Failed to get user by name '{name}': {e:?}");
                Response::Unavail
            }
        }
    }
}
