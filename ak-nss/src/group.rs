use ak_platform::generated::sys_directory::GetRequest;
use libc::gid_t;
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;

use crate::AuthentikNSS;
use crate::backend::{DirectoryBridge, GrpcDirectoryBridge};
use crate::error::response_for_error;
use crate::mapping::ak_group_to_group_entry;

impl GroupHooks for AuthentikNSS {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Group>> {
        get_all_entries_with(&GrpcDirectoryBridge)
    }

    #[tracing::instrument(fields(gid))]
    fn get_entry_by_gid(gid: gid_t) -> Response<Group> {
        get_entry_by_gid_with(&GrpcDirectoryBridge, gid)
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Group> {
        get_entry_by_name_with(&GrpcDirectoryBridge, name)
    }
}

fn get_all_entries_with(bridge: &impl DirectoryBridge) -> Response<Vec<Group>> {
    match bridge.list_groups() {
        Ok(groups) => Response::Success(groups.into_iter().map(ak_group_to_group_entry).collect()),
        Err(e) => response_for_error("Failed to get groups", &e),
    }
}

fn get_entry_by_gid_with(bridge: &impl DirectoryBridge, gid: gid_t) -> Response<Group> {
    match bridge.get_group(GetRequest {
        name: None,
        id: Some(gid),
    }) {
        Ok(group) => Response::Success(ak_group_to_group_entry(group)),
        Err(e) => response_for_error(&format!("error when getting group by ID '{gid}'"), &e),
    }
}

fn get_entry_by_name_with(bridge: &impl DirectoryBridge, name: String) -> Response<Group> {
    match bridge.get_group(GetRequest {
        name: Some(name.clone()),
        id: None,
    }) {
        Ok(group) => Response::Success(ak_group_to_group_entry(group)),
        Err(e) => response_for_error(&format!("error when getting group by name '{name}'"), &e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::generated::sys_directory::{Group as AKGroup, User};
    use eyre::Result;

    struct MockBridge {
        groups: Vec<AKGroup>,
    }

    impl DirectoryBridge for MockBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            unreachable!()
        }
        fn get_user(&self, _req: GetRequest) -> Result<User> {
            unreachable!()
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            Ok(self.groups.clone())
        }
        fn get_group(&self, req: GetRequest) -> Result<AKGroup> {
            self.groups
                .iter()
                .find(|g| {
                    req.id.map_or(false, |id| id == g.gid)
                        || req.name.as_deref().map_or(false, |n| n == g.name)
                })
                .cloned()
                .ok_or_else(|| eyre::eyre!("not found"))
        }
    }

    struct ErrorBridge;
    impl DirectoryBridge for ErrorBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            unreachable!()
        }
        fn get_user(&self, _: GetRequest) -> Result<User> {
            unreachable!()
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            Err(eyre::eyre!("unavailable"))
        }
        fn get_group(&self, _: GetRequest) -> Result<AKGroup> {
            Err(eyre::eyre!("unavailable"))
        }
    }

    struct NotFoundBridge;
    impl DirectoryBridge for NotFoundBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            unreachable!()
        }
        fn get_user(&self, _: GetRequest) -> Result<User> {
            unreachable!()
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            unreachable!()
        }
        fn get_group(&self, _: GetRequest) -> Result<AKGroup> {
            Err(tonic::Status::not_found("no such group").into())
        }
    }

    fn admins() -> AKGroup {
        AKGroup {
            name: "admins".to_owned(),
            gid: 200,
            passwd: "x".to_owned(),
            members: vec!["alice".to_owned()],
        }
    }

    #[test]
    fn get_all_entries_returns_mapped_groups() {
        let bridge = MockBridge {
            groups: vec![admins()],
        };
        match get_all_entries_with(&bridge) {
            Response::Success(groups) => {
                assert_eq!(groups.len(), 1);
                assert_eq!(groups[0].name, "admins");
                assert_eq!(groups[0].gid, 200);
                assert_eq!(groups[0].members, vec!["alice".to_owned()]);
            }
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_all_entries_unavail_on_error() {
        assert!(matches!(
            get_all_entries_with(&ErrorBridge),
            Response::Unavail
        ));
    }

    #[test]
    fn get_entry_by_gid_found() {
        let bridge = MockBridge {
            groups: vec![admins()],
        };
        match get_entry_by_gid_with(&bridge, 200) {
            Response::Success(g) => assert_eq!(g.name, "admins"),
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_entry_by_gid_unavail_on_error() {
        assert!(matches!(
            get_entry_by_gid_with(&ErrorBridge, 200),
            Response::Unavail
        ));
    }

    #[test]
    fn get_entry_by_name_found() {
        let bridge = MockBridge {
            groups: vec![admins()],
        };
        match get_entry_by_name_with(&bridge, "admins".to_owned()) {
            Response::Success(g) => assert_eq!(g.gid, 200),
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_entry_by_name_unavail_on_error() {
        assert!(matches!(
            get_entry_by_name_with(&ErrorBridge, "admins".to_owned()),
            Response::Unavail
        ));
    }

    #[test]
    fn get_entry_by_gid_not_found_on_grpc_not_found() {
        assert!(matches!(
            get_entry_by_gid_with(&NotFoundBridge, 200),
            Response::NotFound
        ));
    }

    #[test]
    fn get_entry_by_name_not_found_on_grpc_not_found() {
        assert!(matches!(
            get_entry_by_name_with(&NotFoundBridge, "admins".to_owned()),
            Response::NotFound
        ));
    }
}
