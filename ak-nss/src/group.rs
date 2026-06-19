use ak_platform::generated::sys_directory::GetRequest;
use ak_platform::generated::sys_directory::system_directory_client::SystemDirectoryClient;
use ak_platform::grpc::grpc_request;
use libc::gid_t;
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;

use crate::AuthentikNSS;

impl GroupHooks for AuthentikNSS {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Group>> {
        match grpc_request(async |ch| {
            return Ok(SystemDirectoryClient::new(ch).list_groups(()).await?);
        }) {
            Ok(r) => {
                let groups = r
                    .into_inner()
                    .groups
                    .into_iter()
                    .map(Self::ak_group_to_group_entry)
                    .collect();
                Response::Success(groups)
            }
            Err(e) => {
                tracing::warn!("Failed to get groups: {e:?}");
                Response::Unavail
            }
        }
    }

    #[tracing::instrument(fields(gid))]
    fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
        match grpc_request(async |ch| {
            return Ok(SystemDirectoryClient::new(ch)
                .get_group(GetRequest {
                    name: None,
                    id: Some(gid),
                })
                .await?);
        }) {
            Ok(r) => Response::Success(Self::ak_group_to_group_entry(r.into_inner())),
            Err(e) => {
                tracing::warn!("error when getting group by ID '{gid}': {e:?}");
                Response::Unavail
            }
        }
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Group> {
        match grpc_request(async |ch| {
            return Ok(SystemDirectoryClient::new(ch)
                .get_group(GetRequest {
                    name: Some(name.clone()),
                    id: None,
                })
                .await?);
        }) {
            Ok(r) => Response::Success(Self::ak_group_to_group_entry(r.into_inner())),
            Err(e) => {
                tracing::warn!("error when getting group by name '{name}': {e:?}");
                Response::Unavail
            }
        }
    }
}
