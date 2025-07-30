use libc::gid_t;
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;
use tokio::runtime::Runtime;

use crate::config::Config;
use crate::generated::create_grpc_client;
use crate::generated::nss::Group as AKGroup;
use crate::generated::nss::{Empty, GetRequest};
use crate::grpc_status_to_nss_response;
use crate::logger::log_hook;

pub struct AuthentikGroupHooks;
impl GroupHooks for AuthentikGroupHooks {
    /// get_all_entries returns all group entries.
    fn get_all_entries() -> Response<Vec<Group>> {
        log_hook("group@get_all_entries");
        get_all_entries()
    }

    /// get_entry_by_gid returns the group entry for the given gid.
    fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
        log_hook("group@get_entry_by_gid");
        get_entry_by_gid(gid)
    }

    /// get_entry_by_name returns the group entry for the given name.
    fn get_entry_by_name(name: String) -> Response<Group> {
        log_hook("group@get_entry_by_name");
        get_entry_by_name(name)
    }
}

/// get_all_entries connects to the grpc server and asks for all group entries.
fn get_all_entries() -> Response<Vec<Group>> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };

    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };
        match client.list_groups(Empty {}).await {
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
                log::info!("error when listing groups: {}", e.code());
                grpc_status_to_nss_response(e)
            }
        }
    })
}

/// get_entry_by_gid connects to the grpc server and asks for the group entry with the given gid.
fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };
    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };
        match client
            .get_group(GetRequest {
                name: None,
                id: Some(gid),
            })
            .await
        {
            Ok(r) => Response::Success(ak_group_to_group_entry(r.into_inner())),
            Err(e) => {
                log::info!("error when getting group by ID '{}': {}", gid, e.code());
                grpc_status_to_nss_response(e)
            }
        }
    })
}

/// get_entry_by_name connects to the grpc server and asks for the group entry with the given name.
fn get_entry_by_name(name: String) -> Response<Group> {
    let config = Config::from_file("/etc/authentik/host.yaml").expect("Failed to load config");

    let rt = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            log::warn!("Failed to create runtime: {}", e);
            return Response::Unavail;
        }
    };

    rt.block_on(async {
        let mut client = match create_grpc_client(config).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to create grpc client: {}", e);
                return Response::Unavail;
            }
        };
        match client
            .get_group(GetRequest {
                name: Some(name.to_owned()),
                id: None,
            })
            .await
        {
            Ok(r) => Response::Success(ak_group_to_group_entry(r.into_inner())),
            Err(e) => {
                log::info!(
                    "error when getting group by name '{}': {}",
                    name,
                    e.code().description()
                );
                grpc_status_to_nss_response(e)
            }
        }
    })
}

fn ak_group_to_group_entry(group: AKGroup) -> Group {
    Group {
        name: group.name,
        passwd: group.passwd,
        gid: group.gid,
        members: group.members,
    }
}
