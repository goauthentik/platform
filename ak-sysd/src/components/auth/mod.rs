use crate::components::{Component, SysdContext};
use ak_platform::generated::sys_auth::{
    InteractiveAuthAsyncResponse, InteractiveAuthRequest, InteractiveChallenge,
    SystemAuthorizeRequest, SystemAuthorizeResponse,
    system_auth_authorize_server::{SystemAuthAuthorize, SystemAuthAuthorizeServer},
    system_auth_interactive_server::{SystemAuthInteractive, SystemAuthInteractiveServer},
    system_auth_token_server::SystemAuthTokenServer,
};
use ak_platform::generated::sys_auth_apple::{
    RegisterDeviceRequest, RegisterDeviceResponse, RegisterUserRequest, RegisterUserResponse,
    system_auth_apple_server::{SystemAuthApple, SystemAuthAppleServer},
};
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::sync::Arc;
use tonic::{Request, Response, Status};

mod apple;
mod authz;
mod interactive;
mod token;

pub struct AuthComponent {
    ctx: SysdContext,
    txns: interactive::Txns,
}

impl AuthComponent {
    pub fn new(ctx: SysdContext) -> AuthComponent {
        AuthComponent {
            ctx,
            txns: interactive::new_txns(),
        }
    }
}

#[tonic::async_trait]
impl Component for AuthComponent {
    fn id() -> &'static str {
        "auth"
    }

    async fn start(&self) -> Result<()> {
        // Reap interactive-auth transactions so the map does not grow without
        // bound (each entry pins a FlowExecutor, cookie jar and domain handle).
        let txns = Arc::clone(&self.txns);
        let cancel = self.ctx.cancel.clone();
        tokio::spawn(async move {
            use std::time::Duration;
            // Unfinished txns time out; finished ones linger briefly so a
            // retrying client can still fetch the result, then get evicted.
            const TICK: Duration = Duration::from_secs(60);
            const TTL: Duration = Duration::from_secs(600);
            const FINISHED_GRACE: Duration = Duration::from_secs(60);
            loop {
                tokio::select! {
                    _ = cancel.cancelled() => return,
                    _ = tokio::time::sleep(TICK) => {}
                }
                let now = tokio::time::Instant::now();
                let mut map = txns.write().await;
                let mut stale = vec![];
                for (id, txn) in map.iter() {
                    let txn = txn.lock().await;
                    let age = now.duration_since(txn.created_at);
                    let expired = if txn.result.is_some() {
                        age > FINISHED_GRACE
                    } else {
                        age > TTL
                    };
                    if expired {
                        stale.push(id.clone());
                    }
                }
                for id in stale {
                    map.remove(&id);
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
            routes.add_service(SystemAuthTokenServer::from_arc(Arc::clone(&self)));
            routes.add_service(SystemAuthInteractiveServer::from_arc(Arc::clone(&self)));
            routes.add_service(SystemAuthAuthorizeServer::from_arc(Arc::clone(&self)));
            routes.add_service(SystemAuthAppleServer::from_arc(self));
        }
    }
}

#[tonic::async_trait]
impl SystemAuthInteractive for AuthComponent {
    async fn interactive_auth(
        &self,
        request: Request<InteractiveAuthRequest>,
    ) -> Result<Response<InteractiveChallenge>, Status> {
        interactive::interactive_auth(&self.ctx, &self.txns, request.into_inner())
            .await
            .map(Response::new)
    }

    async fn interactive_auth_async(
        &self,
        _request: Request<()>,
    ) -> Result<Response<InteractiveAuthAsyncResponse>, Status> {
        interactive::interactive_auth_async(&self.ctx)
            .await
            .map(Response::new)
    }
}

#[tonic::async_trait]
impl SystemAuthAuthorize for AuthComponent {
    async fn authorize(
        &self,
        request: Request<SystemAuthorizeRequest>,
    ) -> Result<Response<SystemAuthorizeResponse>, Status> {
        authz::authorize(&self.ctx, request.into_inner())
            .await
            .map(Response::new)
    }
}

#[tonic::async_trait]
impl SystemAuthApple for AuthComponent {
    async fn register_user(
        &self,
        request: Request<RegisterUserRequest>,
    ) -> Result<Response<RegisterUserResponse>, Status> {
        apple::register_user(&self.ctx, request.into_inner())
            .await
            .map(Response::new)
    }

    async fn register_device(
        &self,
        request: Request<RegisterDeviceRequest>,
    ) -> Result<Response<RegisterDeviceResponse>, Status> {
        apple::register_device(&self.ctx, request.into_inner())
            .await
            .map(Response::new)
    }
}
