use crate::components::{Component, SysdContext};
use crate::state::SessionRecord;
use ak_platform::generated::session::{
    CloseSessionRequest, CloseSessionResponse, OpenSessionRequest, OpenSessionResponse,
    SessionStatusRequest, SessionStatusResponse,
    session_manager_server::{SessionManager, SessionManagerServer},
};
use ak_platform::paths::SysdSocketID;
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use eyre::Result;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tonic::{Request, Response, Status};

mod terminate;

pub struct SessionComponent {
    ctx: SysdContext,
}

impl SessionComponent {
    pub fn new(ctx: SysdContext) -> SessionComponent {
        SessionComponent { ctx }
    }

    /// Creates a new session record, mirroring Go's `NewSession`.
    ///
    /// `expires_at` (unix seconds): the active domain's
    /// `auth_terminate_session_on_expiry` flag reads backwards from its
    /// name in the Go source — ported literally rather than "fixed" without
    /// confirming actual running behavior first.
    pub async fn new_session(
        &self,
        username: String,
        raw_token: String,
        expires_at: Option<i64>,
    ) -> Result<SessionRecord> {
        let mut id_bytes = [0u8; 48];
        rand::rng().fill_bytes(&mut id_bytes);
        // URL-safe alphabet: ak-pam embeds this id verbatim into a filesystem
        // path (session_data.rs), so it must never contain '/' or '+'.
        let id = BASE64_URL_SAFE_NO_PAD.encode(id_bytes);

        let token_hash = {
            let mut hasher = Sha256::new();
            hasher.update(raw_token.as_bytes());
            hex::encode(hasher.finalize())
        };

        let terminate_on_expiry = self
            .ctx
            .domains
            .active()
            .await
            .ok()
            .and_then(|d| {
                let remote = d.remote.try_read().ok()?;
                remote.as_ref().map(|r| r.auth_terminate_session_on_expiry)
            })
            .unwrap_or(false);

        let record = SessionRecord {
            id: id.clone(),
            username,
            token_hash,
            expires_at: if terminate_on_expiry {
                expires_at
            } else {
                None
            },
            created_at: chrono::Utc::now().timestamp(),
            pid: None,
            ppid: None,
            local_socket: None,
            opened: false,
        };
        self.ctx.state.sessions().insert(&record).await?;
        Ok(record)
    }

    async fn check_expired_sessions(&self) {
        let now = chrono::Utc::now().timestamp();
        let sessions = match self.ctx.state.sessions().all_opened().await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("failed to list opened sessions: {e:?}");
                return;
            }
        };
        for session in sessions {
            let Some(expires_at) = session.expires_at else {
                continue;
            };
            if expires_at > now {
                continue;
            }
            if let Err(e) = terminate::terminate_session(&session).await {
                tracing::warn!(session = session.id, "failed to terminate session: {e:?}");
            }
            if let Err(e) = self.ctx.state.sessions().delete(&session.id).await {
                tracing::warn!(session = session.id, "failed to delete session: {e:?}");
            }
        }
    }
}

#[tonic::async_trait]
impl Component for SessionComponent {
    fn id() -> &'static str {
        "session"
    }

    async fn start(&self) -> Result<()> {
        let ctx = self.ctx.clone();
        let this = SessionComponent { ctx: ctx.clone() };
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                tokio::select! {
                    _ = interval.tick() => this.check_expired_sessions().await,
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
            routes.add_service(SessionManagerServer::from_arc(self));
        }
    }
}

#[tonic::async_trait]
impl SessionManager for SessionComponent {
    async fn session_status(
        &self,
        request: Request<SessionStatusRequest>,
    ) -> Result<Response<SessionStatusResponse>, Status> {
        let req = request.into_inner();
        match self
            .ctx
            .state
            .sessions()
            .get(&req.session_id)
            .await
            .map_err(crate::util::to_status)?
        {
            Some(session) => Ok(Response::new(SessionStatusResponse {
                success: true,
                error: String::new(),
                expiry: session.expires_at.map(|s| pbjson_types::Timestamp {
                    seconds: s,
                    nanos: 0,
                }),
            })),
            None => Ok(Response::new(SessionStatusResponse {
                success: false,
                error: "session not found".to_string(),
                expiry: None,
            })),
        }
    }

    async fn open_session(
        &self,
        request: Request<OpenSessionRequest>,
    ) -> Result<Response<OpenSessionResponse>, Status> {
        let req = request.into_inner();
        let mut session = self
            .ctx
            .state
            .sessions()
            .get(&req.session_id)
            .await
            .map_err(crate::util::to_status)?
            .ok_or_else(|| Status::not_found("session not found"))?;

        session.opened = true;
        session.pid = Some(req.pid);
        session.ppid = Some(req.ppid);
        session.local_socket = Some(req.local_socket);
        self.ctx
            .state
            .sessions()
            .update(&session)
            .await
            .map_err(crate::util::to_status)?;

        self.ctx
            .events
            .dispatch(crate::events::SysdEvent::SessionOpened {
                session_id: session.id.clone(),
                pid: req.pid,
            });

        Ok(Response::new(OpenSessionResponse {
            success: true,
            session_id: session.id,
        }))
    }

    async fn close_session(
        &self,
        request: Request<CloseSessionRequest>,
    ) -> Result<Response<CloseSessionResponse>, Status> {
        let req = request.into_inner();
        self.ctx
            .state
            .sessions()
            .delete(&req.session_id)
            .await
            .map_err(crate::util::to_status)?;
        Ok(Response::new(CloseSessionResponse { success: true }))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::context::testutils::test_context;

    // ak-pam embeds the session id verbatim into a filesystem path
    // (`/tmp/.aksm-{id}`); a standard-alphabet base64 id can contain '/' or
    // '+', which previously broke that file creation intermittently
    // (~63% of the time, since 48 random bytes -> 64 base64 chars).
    #[tokio::test]
    async fn test_session_id_is_filesystem_safe() {
        let session = SessionComponent::new(test_context().await);
        for _ in 0..100 {
            let record = session
                .new_session("akadmin".to_string(), "token".to_string(), None)
                .await
                .expect("create session");
            assert!(
                record
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
                "session id contains filesystem-unsafe characters: {}",
                record.id
            );
        }
    }
}
