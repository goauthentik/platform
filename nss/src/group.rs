use authentik_sys::generated::grpc_request;
use authentik_sys::generated::sys_directory::system_directory_client::SystemDirectoryClient;
use authentik_sys::generated::sys_directory::{GetRequest, Group as AKGroup};
use authentik_sys::logger::log_hook;
use libc::gid_t;
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;

pub struct AuthentikGroupHooks;
impl GroupHooks for AuthentikGroupHooks {
    fn get_all_entries() -> Response<Vec<Group>> {
        log_hook("group::get_all_entries");
        get_all_entries()
    }

    fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
        log_hook("group::get_entry_by_gid");
        get_entry_by_gid(gid)
    }

    fn get_entry_by_name(name: String) -> Response<Group> {
        log_hook("group::get_entry_by_name");
        get_entry_by_name(name)
    }
}

fn get_all_entries() -> Response<Vec<Group>> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch).list_groups(()).await?);
    }) {
        Ok(r) => {
            let groups = r
                .into_inner()
                .groups
                .into_iter()
                .map(ak_group_to_group_entry)
                .collect();
            Response::Success(groups)
        }
        Err(e) => {
            log::warn!("Failed to get groups: {e}");
            Response::Unavail
        }
    }
}

fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch)
            .get_group(GetRequest {
                name: None,
                id: Some(gid),
            })
            .await?);
    }) {
        Ok(r) => Response::Success(ak_group_to_group_entry(r.into_inner())),
        Err(e) => {
            log::warn!("error when getting group by ID '{gid}': {e}");
            Response::Unavail
        }
    }
}

fn get_entry_by_name(name: String) -> Response<Group> {
    match grpc_request(async |ch| {
        return Ok(SystemDirectoryClient::new(ch)
            .get_group(GetRequest {
                name: Some(name.clone()),
                id: None,
            })
            .await?);
    }) {
        Ok(r) => Response::Success(ak_group_to_group_entry(r.into_inner())),
        Err(e) => {
            log::warn!("error when getting group by name '{name}': {e}");
            Response::Unavail
        }
    }
}

fn ak_group_to_group_entry(group: AKGroup) -> Group {
    Group {
        name: group.name,
        passwd: group.passwd,
        gid: group.gid,
        members: group.members,
    }
}
