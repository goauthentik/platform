use ak_platform::generated::sys_directory::GetRequest;
use libc::uid_t;
use libnss::interop::Response;
use libnss::passwd::{Passwd, PasswdHooks};

use crate::AuthentikNSS;
use crate::backend::{DirectoryBridge, GrpcDirectoryBridge};
use crate::mapping::user_to_passwd_entry;

impl PasswdHooks for AuthentikNSS {
    #[tracing::instrument]
    fn get_all_entries() -> Response<Vec<Passwd>> {
        get_all_entries_with(&GrpcDirectoryBridge)
    }

    #[tracing::instrument(fields(uid))]
    fn get_entry_by_uid(uid: uid_t) -> Response<Passwd> {
        get_entry_by_uid_with(&GrpcDirectoryBridge, uid)
    }

    #[tracing::instrument(fields(name))]
    fn get_entry_by_name(name: String) -> Response<Passwd> {
        get_entry_by_name_with(&GrpcDirectoryBridge, name)
    }
}

fn get_all_entries_with(bridge: &impl DirectoryBridge) -> Response<Vec<Passwd>> {
    match bridge.list_users() {
        Ok(users) => Response::Success(users.into_iter().map(user_to_passwd_entry).collect()),
        Err(e) => {
            tracing::warn!("error getting users: {e:?}");
            Response::Unavail
        }
    }
}

fn get_entry_by_uid_with(bridge: &impl DirectoryBridge, uid: uid_t) -> Response<Passwd> {
    match bridge.get_user(GetRequest {
        id: Some(uid),
        name: None,
    }) {
        Ok(user) => Response::Success(user_to_passwd_entry(user)),
        Err(e) => {
            tracing::warn!("error when getting user by ID '{uid}': {e:?}");
            Response::Unavail
        }
    }
}

fn get_entry_by_name_with(bridge: &impl DirectoryBridge, name: String) -> Response<Passwd> {
    // This is a fake call done by PAM to avoid attacks, so we need to special case it to avoid
    // spamming logs with "Not Found" messages, as this call is done quite frequently.
    if name == "pam_unix_non_existent:" {
        return Response::NotFound;
    }
    match bridge.get_user(GetRequest {
        name: Some(name.clone()),
        id: None,
    }) {
        Ok(user) => Response::Success(user_to_passwd_entry(user)),
        Err(e) => {
            tracing::warn!("error when getting user by name '{name}': {e:?}");
            Response::Unavail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::generated::sys_directory::{Group as AKGroup, User};
    use ak_platform::prelude::Result;

    struct MockBridge {
        users: Vec<User>,
    }

    impl DirectoryBridge for MockBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            Ok(self.users.clone())
        }
        fn get_user(&self, req: GetRequest) -> Result<User> {
            self.users
                .iter()
                .find(|u| {
                    req.id.map_or(false, |id| id == u.uid)
                        || req.name.as_deref().map_or(false, |n| n == u.name)
                })
                .cloned()
                .ok_or_else(|| "not found".into())
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            Ok(vec![])
        }
        fn get_group(&self, _req: GetRequest) -> Result<AKGroup> {
            Err("not found".into())
        }
    }

    struct ErrorBridge;
    impl DirectoryBridge for ErrorBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            Err("unavailable".into())
        }
        fn get_user(&self, _: GetRequest) -> Result<User> {
            Err("unavailable".into())
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            Err("unavailable".into())
        }
        fn get_group(&self, _: GetRequest) -> Result<AKGroup> {
            Err("unavailable".into())
        }
    }

    fn alice() -> User {
        User {
            name: "alice".to_owned(),
            uid: 1000,
            gid: 100,
            gecos: "Alice Smith".to_owned(),
            homedir: "/home/alice".to_owned(),
            shell: "/bin/bash".to_owned(),
        }
    }

    #[test]
    fn get_all_entries_returns_mapped_users() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        match get_all_entries_with(&bridge) {
            Response::Success(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].name, "alice");
                assert_eq!(entries[0].uid, 1000);
                assert_eq!(entries[0].passwd, "x");
            }
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_all_entries_unavail_on_error() {
        assert!(matches!(get_all_entries_with(&ErrorBridge), Response::Unavail));
    }

    #[test]
    fn get_entry_by_uid_found() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        match get_entry_by_uid_with(&bridge, 1000) {
            Response::Success(p) => assert_eq!(p.name, "alice"),
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_entry_by_uid_unavail_on_error() {
        assert!(matches!(
            get_entry_by_uid_with(&ErrorBridge, 1000),
            Response::Unavail
        ));
    }

    #[test]
    fn get_entry_by_name_found() {
        let bridge = MockBridge {
            users: vec![alice()],
        };
        match get_entry_by_name_with(&bridge, "alice".to_owned()) {
            Response::Success(p) => assert_eq!(p.uid, 1000),
            _ => panic!("expected Response::Success"),
        }
    }

    #[test]
    fn get_entry_by_name_unavail_on_error() {
        assert!(matches!(
            get_entry_by_name_with(&ErrorBridge, "alice".to_owned()),
            Response::Unavail
        ));
    }

    #[test]
    fn pam_unix_non_existent_short_circuits_before_bridge() {
        struct PanicBridge;
        impl DirectoryBridge for PanicBridge {
            fn list_users(&self) -> Result<Vec<User>> {
                panic!("bridge must not be called")
            }
            fn get_user(&self, _: GetRequest) -> Result<User> {
                panic!("bridge must not be called")
            }
            fn list_groups(&self) -> Result<Vec<AKGroup>> {
                panic!("bridge must not be called")
            }
            fn get_group(&self, _: GetRequest) -> Result<AKGroup> {
                panic!("bridge must not be called")
            }
        }
        assert!(matches!(
            get_entry_by_name_with(&PanicBridge, "pam_unix_non_existent:".to_owned()),
            Response::NotFound
        ));
    }
}
