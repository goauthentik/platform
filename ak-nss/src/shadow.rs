use ak_platform::generated::sys_directory::GetRequest;
use libnss::interop::Response;
use libnss::shadow::{Shadow, ShadowHooks};

use crate::AuthentikNSS;
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
    match bridge.list_users() {
        Ok(users) => {
            let entries = users.into_iter().map(|u| shadow_entry(u.name)).collect();
            Response::Success(entries)
        }
        Err(e) => {
            tracing::warn!("Failed to get users: {e:?}");
            Response::Unavail
        }
    }
}

fn get_entry_by_name_with(bridge: &impl DirectoryBridge, name: String) -> Response<Shadow> {
    match bridge.get_user(GetRequest {
        name: Some(name.clone()),
        id: None,
    }) {
        Ok(user) => Response::Success(shadow_entry(user.name)),
        Err(e) => {
            tracing::warn!("Failed to get user by name '{name}': {e:?}");
            Response::Unavail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ak_platform::generated::sys_directory::{Group as AKGroup, User};
    use eyre::Result;

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
                .find(|u| req.name.as_deref().map_or(false, |n| n == u.name))
                .cloned()
                .ok_or_else(|| eyre::eyre!("not found"))
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            unreachable!()
        }
        fn get_group(&self, _req: GetRequest) -> Result<AKGroup> {
            unreachable!()
        }
    }

    struct ErrorBridge;
    impl DirectoryBridge for ErrorBridge {
        fn list_users(&self) -> Result<Vec<User>> {
            Err(eyre::eyre!("unavailable"))
        }
        fn get_user(&self, _: GetRequest) -> Result<User> {
            Err(eyre::eyre!("unavailable"))
        }
        fn list_groups(&self) -> Result<Vec<AKGroup>> {
            unreachable!()
        }
        fn get_group(&self, _: GetRequest) -> Result<AKGroup> {
            unreachable!()
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
            get_all_entries_with(&ErrorBridge),
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
            get_entry_by_name_with(&ErrorBridge, "alice".to_owned()),
            Response::Unavail
        ));
    }
}
