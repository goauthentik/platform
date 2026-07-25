use crate::components::{Component, SysdContext};
use crate::events::SysdEvent;
use ak_platform::generated::sys_directory::{
    GetRequest, Group, Groups, User, Users,
    system_directory_server::{SystemDirectory, SystemDirectoryServer},
};
use ak_platform::paths::SysdSocketID;
use authentik_client::apis::configuration::Configuration;
use authentik_client::models;
use eyre::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};

const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 30 * 60;
const PAGE_SIZE: i32 = 100;

pub struct DirectoryComponent {
    ctx: SysdContext,
    users: Arc<RwLock<Vec<User>>>,
    groups: Arc<RwLock<Vec<Group>>>,
}

impl DirectoryComponent {
    pub fn new(ctx: SysdContext) -> DirectoryComponent {
        DirectoryComponent {
            ctx,
            users: Arc::new(RwLock::new(vec![])),
            groups: Arc::new(RwLock::new(vec![])),
        }
    }
}

/// NSS-safe username cleaning: lowercase, `@`/`:` replaced with `-`. The
/// exact regex Go uses (`pkg/agent_system/directory/user.go`, not read in
/// this pass) may differ in edge cases — this is a best-effort approximation.
fn clean_name(name: &str) -> String {
    name.to_lowercase()
        .replace(['@', ':'], "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

fn attr_number(
    attrs: &Option<std::collections::HashMap<String, serde_json::Value>>,
    key: &str,
) -> Option<u32> {
    attrs.as_ref()?.get(key)?.as_str()?.parse().ok()
}

fn user_uid(u: &models::User, nss_uid_offset: i32) -> u32 {
    attr_number(&u.attributes, "uidNumber").unwrap_or((nss_uid_offset + u.pk) as u32)
}

/// A user's synthetic primary group defaults to the *same value* as their
/// uid, not a separate offset — this looks like a typo but matches Go.
fn user_gid(u: &models::User, nss_uid_offset: i32) -> u32 {
    attr_number(&u.attributes, "gidNumber").unwrap_or_else(|| user_uid(u, nss_uid_offset))
}

fn group_gid(g: &models::Group, nss_gid_offset: i32) -> u32 {
    attr_number(&g.attributes, "gidNumber").unwrap_or((nss_gid_offset + g.num_pk) as u32)
}

async fn fetch_all_users(api: &Configuration) -> Result<Vec<models::User>> {
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

async fn fetch_all_groups(api: &Configuration) -> Result<Vec<models::Group>> {
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

impl DirectoryComponent {
    async fn fetch(&self) -> Result<()> {
        let domain = self.ctx.domains.active().await?;
        let nss_uid_offset = domain
            .remote
            .read()
            .await
            .as_ref()
            .map(|r| r.nss_uid_offset)
            .unwrap_or(10000);
        let nss_gid_offset = domain
            .remote
            .read()
            .await
            .as_ref()
            .map(|r| r.nss_gid_offset)
            .unwrap_or(10000);

        let raw_users = fetch_all_users(&domain.api).await?;
        let raw_groups = fetch_all_groups(&domain.api).await?;

        let mut users = vec![];
        let mut synthetic_groups = vec![];
        for u in &raw_users {
            let name = clean_name(&u.username);
            let uid = user_uid(u, nss_uid_offset);
            let gid = user_gid(u, nss_uid_offset);
            users.push(User {
                name: name.clone(),
                uid,
                gid,
                gecos: u.name.clone(),
                homedir: format!("/home/{name}"),
                shell: "/bin/bash".to_string(),
            });
            synthetic_groups.push(Group {
                name,
                gid,
                members: vec![],
                passwd: "x".to_string(),
            });
        }
        users.sort_by_key(|u| u.uid);

        let mut groups: Vec<Group> = raw_groups
            .iter()
            .map(|g| Group {
                name: clean_name(&g.name),
                gid: group_gid(g, nss_gid_offset),
                members: g
                    .users_obj
                    .clone()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|u| clean_name(&u.username))
                    .collect(),
                passwd: "x".to_string(),
            })
            .collect();
        groups.extend(synthetic_groups);
        groups.sort_by_key(|g| g.gid);

        *self.users.write().await = users;
        *self.groups.write().await = groups;

        self.ctx.events.dispatch(SysdEvent::DirectoryFetched {
            domain: domain.cfg.domain.clone(),
        });
        Ok(())
    }
}

#[tonic::async_trait]
impl Component for DirectoryComponent {
    fn id() -> &'static str {
        "directory"
    }

    async fn start(&self) -> Result<()> {
        let ctx = self.ctx.clone();
        let users = Arc::clone(&self.users);
        let groups = Arc::clone(&self.groups);
        tokio::spawn(async move {
            let this = DirectoryComponent {
                ctx: ctx.clone(),
                users,
                groups,
            };
            let jitter = rand::random::<u64>() % 30;
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(jitter)) => {}
                _ = ctx.cancel.cancelled() => return,
            }
            loop {
                if let Err(e) = this.fetch().await {
                    tracing::warn!("directory fetch failed: {e:?}");
                }
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS)) => {}
                    _ = ctx.cancel.cancelled() => return,
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    fn register(self: Arc<Self>, socket: SysdSocketID, routes: &mut tonic::service::RoutesBuilder) {
        if matches!(socket, SysdSocketID::Default) {
            routes.add_service(SystemDirectoryServer::from_arc(self));
        }
    }
}

#[tonic::async_trait]
impl SystemDirectory for DirectoryComponent {
    async fn list_users(&self, _request: Request<()>) -> Result<Response<Users>, Status> {
        Ok(Response::new(Users {
            users: self.users.read().await.clone(),
        }))
    }

    async fn get_user(&self, request: Request<GetRequest>) -> Result<Response<User>, Status> {
        let req = request.into_inner();
        let users = self.users.read().await;
        let found = users
            .iter()
            .find(|u| Some(u.uid) == req.id || Some(&u.name) == req.name.as_ref());
        found
            .cloned()
            .map(Response::new)
            .ok_or_else(|| Status::not_found("user not found"))
    }

    async fn list_groups(&self, _request: Request<()>) -> Result<Response<Groups>, Status> {
        Ok(Response::new(Groups {
            groups: self.groups.read().await.clone(),
        }))
    }

    async fn get_group(&self, request: Request<GetRequest>) -> Result<Response<Group>, Status> {
        let req = request.into_inner();
        let groups = self.groups.read().await;
        let found = groups
            .iter()
            .find(|g| Some(g.gid) == req.id || Some(&g.name) == req.name.as_ref());
        found
            .cloned()
            .map(Response::new)
            .ok_or_else(|| Status::not_found("group not found"))
    }
}
