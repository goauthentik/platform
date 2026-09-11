use ak_platform::generated::sys_directory::GetRequest;
use libnss::interop::Response;
use libnss::shadow::{Shadow, ShadowHooks};

use crate::AuthentikNSS;
use crate::backend::ErrMap;
use crate::backend::{DirectoryBridge, GrpcDirectoryBridge};
use crate::mapping::shadow_entry;

impl ShadowHooks for AuthentikNSS {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Shadow>> {
        get_all_entries_with(&GrpcDirectoryBridge)
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Shadow> {
        get_entry_by_name_with(&GrpcDirectoryBridge, name)
    }
}

fn get_all_entries_with(bridge: &impl DirectoryBridge) -> Response<Vec<Shadow>> {
    bridge
        .list_users()
        .map(|users| users.into_iter().map(|u| shadow_entry(u.name)).collect())
        .to_response("failed to list users")
}

fn get_entry_by_name_with(bridge: &impl DirectoryBridge, name: String) -> Response<Shadow> {
    bridge
        .get_user(GetRequest {
            name: Some(name.clone()),
            id: None,
        })
        .map(|user| shadow_entry(user.name))
        .to_response(format!("failed to get shadow entry for '{name}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::generated::sys_directory::{Group as AKGroup, User};
    use ak_platform::grpc::{GrpcResult, Status};

    struct MockBridge {
        users: Vec<User>,
    }

    impl DirectoryBridge for MockBridge {
        fn list_users(&self) -> GrpcResult<Vec<User>> {
            Ok(self.users.clone())
        }
        fn get_user(&self, req: GetRequest) -> GrpcResult<User> {
            self.users
                .iter()
                .find(|u| req.name.as_deref().map_or(false, |n| n == u.name))
                .cloned()
                .ok_or_else(|| Status::not_found("no such entry").into())
        }
        fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
            unreachable!()
        }
        fn get_group(&self, _req: GetRequest) -> GrpcResult<AKGroup> {
            unreachable!()
        }
    }

    struct UnavailBridge;
    impl DirectoryBridge for UnavailBridge {
        fn list_users(&self) -> GrpcResult<Vec<User>> {
            Err(Status::unavailable("connect failed").into())
        }
        fn get_user(&self, _: GetRequest) -> GrpcResult<User> {
            Err(Status::unavailable("connect failed").into())
        }
        fn list_groups(&self) -> GrpcResult<Vec<AKGroup>> {
            unreachable!()
        }
        fn get_group(&self, _: GetRequest) -> GrpcResult<AKGroup> {
            unreachable!()
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

    fn alice() -> User {
        User {
            name: "alice".to_owned(),
            uid: 1000,
            gid: 100,
            gecos: String::new(),
            homedir: "/home/alice".to_owned(),
            shell: "/bin/bash".to_owned(),
        }
    }

    #[test]
    fn get_all_entries_returns_shadow_for_each_user() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        match get_all_entries_with(&bridge) {
            Response::Success(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].name, "alice");
                assert_eq!(entries[0].passwd, "x");
                assert_eq!(entries[0].last_change, -1);
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
    fn get_entry_by_name_found() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        match get_entry_by_name_with(&bridge, "alice".to_owned()) {
            Response::Success(s) => {
                assert_eq!(s.name, "alice");
                assert_eq!(s.passwd, "x");
            }
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_entry_by_name_unavail_on_error() {
        assert!(matches!(
            get_entry_by_name_with(&UnavailBridge, "alice".to_owned()),
            Response::Unavail
        ));
    }

    /// A name that simply isn't in the directory is NOTFOUND, not UNAVAIL.
    #[test]
    fn get_entry_by_name_notfound_when_missing() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        assert!(matches!(
            get_entry_by_name_with(&bridge, "bob".to_owned()),
            Response::NotFound
        ));
        assert!(matches!(
            get_entry_by_name_with(&NotFoundBridge, "bob".to_owned()),
            Response::NotFound
        ));
    }
}
