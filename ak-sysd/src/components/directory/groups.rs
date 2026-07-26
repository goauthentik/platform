use crate::components::directory::{DirectoryComponent, PAGE_SIZE, attr_number};
use ak_platform::generated::sys_directory::Group;
use authentik_client::apis::configuration::Configuration;
use authentik_client::models;
use eyre::Result;

impl DirectoryComponent {
    pub fn convert_group(&self, raw_group: &models::Group) -> Group {
        Group {
            name: self.clean_name(&raw_group.name),
            gid: self.group_gid(raw_group),
            members: raw_group
                .users_obj
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|u| self.clean_name(&u.username))
                .collect(),
            passwd: "x".to_string(),
        }
    }

    pub fn group_gid(&self, g: &models::Group) -> u32 {
        attr_number(&g.attributes, "gidNumber").unwrap_or((self.nss_gid_offset + g.num_pk) as u32)
    }

    #[tracing::instrument(skip_all)]
    pub async fn fetch_all_groups(&self, api: &Configuration) -> Result<Vec<models::Group>> {
        let mut all = vec![];
        let mut page = 1;
        loop {
            let res = authentik_client::apis::core_api::core_groups_list(
                api,
                None,
                None,
                None,
                None,
                Some(true),
                None,
                None,
                None,
                None,
                None,
                Some(page),
                Some(PAGE_SIZE),
                None,
            )
            .await
            .map_err(|e| eyre::eyre!("core_groups_list failed: {e}"))?;
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
        GetRequest, Group, system_directory_server::SystemDirectory,
    };
    use tonic::Request;

    use crate::{components::directory::DirectoryComponent, context::testutils};

    #[tokio::test]
    async fn test_list_groups() {
        let dir = DirectoryComponent::new(testutils::test_context().await);
        dir.groups.write().await.push(Group {
            name: "test-group".to_string(),
            gid: 1000,
            members: vec![],
            passwd: "x".to_string(),
        });
        let res = dir
            .list_groups(Request::new(()))
            .await
            .unwrap()
            .into_inner()
            .groups;
        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn test_get_group() {
        let dir = DirectoryComponent::new(testutils::test_context().await);
        dir.groups.write().await.push(Group {
            name: "test-group".to_string(),
            gid: 1000,
            members: vec![],
            passwd: "x".to_string(),
        });

        let res = dir
            .get_group(Request::new(GetRequest {
                id: Some(1000),
                ..Default::default()
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.gid, 1000);

        let res = dir
            .get_group(Request::new(GetRequest {
                name: Some("test-group".to_string()),
                ..Default::default()
            }))
            .await
            .unwrap()
            .into_inner();
        assert_eq!(res.gid, 1000);

        assert!(
            dir.get_group(Request::new(GetRequest {
                name: Some("other-user".to_string()),
                ..Default::default()
            }))
            .await
            .is_err()
        );
    }
}
