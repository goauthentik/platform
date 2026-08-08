use ak_platform::generated::sys_directory::GetRequest;
use libc::gid_t;
use libnss::group::{Group, GroupHooks};
use libnss::interop::Response;

use crate::AuthentikNSS;
use crate::backend::ErrMap;
use crate::backend::{DirectoryBridge, GrpcDirectoryBridge};
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
    bridge
        .list_groups()
        .map(|groups| groups.into_iter().map(ak_group_to_group_entry).collect())
        .to_response("failed to list groups")
}

fn get_entry_by_gid_with(bridge: &impl DirectoryBridge, gid: gid_t) -> Response<Group> {
    bridge
        .get_group(GetRequest {
            name: None,
            id: Some(gid),
        })
        .map(ak_group_to_group_entry)
        .to_response(format!("failed to get group by ID '{gid}'"))
}

fn get_entry_by_name_with(bridge: &impl DirectoryBridge, name: String) -> Response<Group> {
    bridge
        .get_group(GetRequest {
            name: Some(name.clone()),
            id: None,
        })
        .map(ak_group_to_group_entry)
        .to_response(format!("failed to get group by name '{name}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::generated::sys_directory::{Group as AKGroup, User};
    use ak_platform::grpc::{GrpcResult, Status};

    struct MockBridge {
        groups: Vec<AKGroup>,
    }

    impl DirectoryBridge for MockBridge {
        fn list_users(&self) -> GrpcResult<Vec<User>> {
            unreachable!()
        }
        fn get_user(&self, _req: GetRequest) -> GrpcResult<User> {
            unreachable!()
        }
        fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
            Ok(self.groups.clone())
        }
        fn get_group(&self, req: GetRequest) -> GrpcResult<AKGroup> {
            self.groups
                .iter()
                .find(|g| {
                    req.id.map_or(false, |id| id == g.gid)
                        || req.name.as_deref().map_or(false, |n| n == g.name)
                })
                .cloned()
                .ok_or_else(|| Status::not_found("no such entry").into())
        }
    }

    struct UnavailBridge;
    impl DirectoryBridge for UnavailBridge {
        fn list_users(&self) -> GrpcResult<Vec<User>> {
            unreachable!()
        }
        fn get_user(&self, _: GetRequest) -> GrpcResult<User> {
            unreachable!()
        }
        fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
            Err(Status::unavailable("connect failed").into())
        }
        fn get_group(&self, _: GetRequest) -> GrpcResult<AKGroup> {
            Err(Status::unavailable("connect failed").into())
        }
    }

    /// sysd answered, and there is no such entry.
    struct NotFoundBridge;
    impl DirectoryBridge for NotFoundBridge {
        fn list_users(&self) -> GrpcResult<Vec<User>> {
            Err(Status::not_found("no such entry").into())
        }
        fn get_user(&self, _: GetRequest) -> GrpcResult<User> {
            Err(Status::not_found("no such entry").into())
        }
        fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
            Err(Status::not_found("no such entry").into())
        }
        fn get_group(&self, _: GetRequest) -> GrpcResult<AKGroup> {
            Err(Status::not_found("no such entry").into())
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
            get_all_entries_with(&UnavailBridge),
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
            get_entry_by_gid_with(&UnavailBridge, 200),
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
            get_entry_by_name_with(&UnavailBridge, "admins".to_owned()),
            Response::Unavail
        ));
    }

    /// A gid that simply isn't in the directory is NOTFOUND, not UNAVAIL.
    #[test]
    fn get_entry_by_gid_notfound_when_missing() {
        let bridge = MockBridge {
            groups: vec![admins()],
        };
        assert!(matches!(
            get_entry_by_gid_with(&bridge, 4242),
            Response::NotFound
        ));
        assert!(matches!(
            get_entry_by_gid_with(&NotFoundBridge, 4242),
            Response::NotFound
        ));
    }

    /// A group name that simply isn't in the directory is NOTFOUND, not UNAVAIL.
    #[test]
    fn get_entry_by_name_notfound_when_missing() {
        let bridge = MockBridge {
            groups: vec![admins()],
        };
        assert!(matches!(
            get_entry_by_name_with(&bridge, "nosuchgroup".to_owned()),
            Response::NotFound
        ));
        assert!(matches!(
            get_entry_by_name_with(&NotFoundBridge, "nosuchgroup".to_owned()),
            Response::NotFound
        ));
    }
}
