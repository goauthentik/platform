use crate::components::{Component, SysdContext};
use crate::events::SysdEvent;
use ak_platform::generated::sys_directory::{
    GetRequest, Group, Groups, User, Users,
    system_directory_server::{SystemDirectory, SystemDirectoryServer},
};
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
pub mod groups;
pub mod users;

const DEFAULT_REFRESH_INTERVAL_SECS: u64 = 30 * 60;
pub const PAGE_SIZE: i32 = 100;

pub struct DirectoryComponent {
    ctx: SysdContext,
    users: Arc<RwLock<Vec<User>>>,
    groups: Arc<RwLock<Vec<Group>>>,
    nss_uid_offset: i32,
    nss_gid_offset: i32,
}

impl DirectoryComponent {
    pub fn new(ctx: SysdContext) -> DirectoryComponent {
        DirectoryComponent {
            ctx,
            users: Arc::new(RwLock::new(vec![])),
            groups: Arc::new(RwLock::new(vec![])),
            nss_uid_offset: 10000,
            nss_gid_offset: 10000,
        }
    }
}

pub fn attr_number(attrs: &Option<HashMap<String, serde_json::Value>>, key: &str) -> Option<u32> {
    attrs.as_ref()?.get(key)?.as_str()?.parse().ok()
}

impl DirectoryComponent {
    async fn fetch(&mut self) -> Result<()> {
        tracing::info!("Fetching users & groups");
        let domain = self.ctx.domains.active().await?;
        self.nss_uid_offset = domain
            .remote
            .read()
            .await
            .as_ref()
            .map(|r| r.nss_uid_offset)
            .unwrap_or(10000);
        self.nss_gid_offset = domain
            .remote
            .read()
            .await
            .as_ref()
            .map(|r| r.nss_gid_offset)
            .unwrap_or(10000);

        let raw_users = self.fetch_all_users(&domain.api).await?;
        let raw_groups = self.fetch_all_groups(&domain.api).await?;

        let mut users = vec![];
        let mut synthetic_groups = vec![];
        for u in &raw_users {
            users.push(self.convert_user(u));
            synthetic_groups.push(self.convert_user_to_group(u));
        }
        users.sort_by_key(|u| u.uid);

        let mut groups: Vec<Group> = raw_groups.iter().map(|g| self.convert_group(g)).collect();
        groups.extend(synthetic_groups);
        groups.sort_by_key(|g| g.gid);

        *self.users.write().await = users;
        *self.groups.write().await = groups;

        self.ctx.events.dispatch(SysdEvent::DirectoryFetched {
            domain: domain.cfg.domain.clone(),
        });
        Ok(())
    }

    /// NSS-safe username cleaning: lowercase, `@`/`:` replaced with `-`. The
    /// exact regex Go uses (`pkg/agent_system/directory/user.go`, not read in
    /// this pass) may differ in edge cases — this is a best-effort approximation.
    pub fn clean_name(&self, name: &str) -> String {
        name.to_lowercase()
            .replace(['@', ':'], "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect()
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
        let nss_uid_offset = self.nss_uid_offset;
        let nss_gid_offset = self.nss_gid_offset;
        tokio::spawn(async move {
            let mut this = DirectoryComponent {
                ctx: ctx.clone(),
                users,
                groups,
                nss_uid_offset,
                nss_gid_offset,
            };
            let jitter = rand::random::<u64>() % 30;
            tracing::info!(jitter, "Waiting seconds before fetching...");
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(jitter)) => {}
                _ = ctx.cancel.cancelled() => return,
            }
            loop {
                if let Err(e) = this.fetch().await {
                    tracing::warn!("directory fetch failed: {e:?}");
                }
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(DEFAULT_REFRESH_INTERVAL_SECS)) => {}
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
