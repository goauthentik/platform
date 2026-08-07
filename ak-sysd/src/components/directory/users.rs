use crate::components::directory::{DirectoryComponent, PAGE_SIZE, attr_number};
use ak_platform::generated::sys_directory::{Group, User};
use authentik_client::apis::configuration::Configuration;
use authentik_client::models;
use eyre::Result;

impl DirectoryComponent {
    pub fn convert_user(&self, raw_user: &models::User) -> User {
        let name = self.clean_name(&raw_user.username);
        let uid = self.user_uid(raw_user);
        let gid = self.user_gid(raw_user);
        User {
            name: name.clone(),
            uid,
            gid,
            gecos: raw_user.name.clone(),
            homedir: format!("/home/{name}"),
            shell: "/bin/bash".to_string(),
        }
    }

    pub fn convert_user_to_group(&self, raw_user: &models::User) -> Group {
        let name = self.clean_name(&raw_user.username);
        let gid = self.user_gid(raw_user);
        Group {
            name,
            gid,
            members: vec![],
            passwd: "x".to_string(),
        }
    }

    pub fn user_uid(&self, u: &models::User) -> u32 {
        attr_number(&u.attributes, "uidNumber").unwrap_or((self.nss_uid_offset + u.pk) as u32)
    }

    /// A user's synthetic primary group defaults to the *same value* as their
    /// uid, not a separate offset — this looks like a typo but matches Go.
    pub fn user_gid(&self, u: &models::User) -> u32 {
        attr_number(&u.attributes, "gidNumber").unwrap_or_else(|| self.user_uid(u))
    }

    #[tracing::instrument(skip_all)]
    pub async fn fetch_all_users(&self, api: &Configuration) -> Result<Vec<models::User>> {
        let mut all = vec![];
        let mut page = 1;
        loop {
            let res = authentik_client::apis::core_api::core_users_list(
                api,
                None,
                None,
                None,
                None,
                None,
                None,
                None,       // attributes..groups_by_pk
                Some(true), // include_groups
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None, // include_roles..last_updated__lt
                None,
                None, // name, ordering
                Some(page),
                Some(PAGE_SIZE), // page, page_size
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None, // path..uuid
            )
            .await
            .map_err(|e| eyre::eyre!("core_users_list failed: {e}"))?;
            all.extend(res.results);
            if res.pagination.next == 0.0 {
                break;
            }
            page = res.pagination.next as i32;
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use ak_platform::generated::sys_directory::{
        GetRequest, User, system_directory_server::SystemDirectory,
    };
    use tonic::Request;

    use crate::{components::directory::DirectoryComponent, context::testutils};

    #[tokio::test]
    async fn test_list_users() {
        let dir = DirectoryComponent::new(testutils::test_context().await);
        dir.users.write().await.push(User {
            name: "test-user".to_string(),
            uid: 1000,
            gid: 1000,
            gecos: "My test user".to_string(),
            homedir: "".to_string(),
            shell: "".to_string(),
        });
        let res = dir
            .list_users(Request::new(()))
            .await
            .unwrap()
            .into_inner()
            .users;
        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_get_user() {
        let dir = DirectoryComponent::new(testutils::test_context().await);
        dir.users.write().await.push(User {
            name: "test-user".to_string(),
            uid: 1000,
            gid: 1000,
            gecos: "My test user".to_string(),
            homedir: "".to_string(),
            shell: "".to_string(),
        });

        let res = dir
            .get_user(Request::new(GetRequest {
                id: Some(1000),
                ..Default::default()
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.uid, 1000);

        let res = dir
            .get_user(Request::new(GetRequest {
                name: Some("test-user".to_string()),
                ..Default::default()
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.uid, 1000);

        assert!(
            dir.get_user(Request::new(GetRequest {
                name: Some("other-user".to_string()),
                ..Default::default()
            }))
            .await
            .is_err()
        );
    }
}
