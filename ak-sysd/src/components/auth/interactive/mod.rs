mod fido;
mod interactive_txn;
mod redirect_scheme;

use std::collections::HashMap;
use std::sync::Arc;

use ak_platform::generated::sys_auth::{
    InteractiveAuthAsyncResponse, InteractiveAuthContinueRequest, InteractiveAuthInitRequest,
    InteractiveAuthRequest, InteractiveChallenge, interactive_auth_request::InteractiveAuth,
};
use authentik_client::apis::endpoints_api::endpoints_agents_connectors_auth_ia_create;
use authentik_client::models::LicenseStatusEnum;
use base64::{Engine, prelude::BASE64_STANDARD};
use rand::Rng;
use tokio::sync::{Mutex, RwLock};
use tonic::Status;

use interactive_txn::{InteractiveAuthTransaction, device_token_hash};

use crate::components::SysdContext;
use crate::util::to_status;

pub type Txns = Arc<RwLock<HashMap<String, Arc<Mutex<InteractiveAuthTransaction>>>>>;

pub fn new_txns() -> Txns {
    Arc::new(RwLock::new(HashMap::new()))
}

fn random_id(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    BASE64_STANDARD.encode(bytes)
}

// Matches the AUTH_INTERACTIVE capability the ping component advertises.
async fn interactive_supported(ctx: &SysdContext) -> bool {
    if let Ok(active) = ctx.domains.active().await
        && let Some(remote) = active.remote.read().await.as_ref()
        && let Some(status) = remote.license_status
    {
        return status != LicenseStatusEnum::Unlicensed;
    }
    false
}

pub async fn interactive_auth(
    ctx: &SysdContext,
    txns: &Txns,
    req: InteractiveAuthRequest,
) -> Result<InteractiveChallenge, Status> {
    if !interactive_supported(ctx).await {
        return Err(Status::unavailable(
            "interactive authentication is not licensed for this domain",
        ));
    }
    match req.interactive_auth {
        Some(InteractiveAuth::Init(init)) => interactive_auth_init(ctx, txns, init).await,
        Some(InteractiveAuth::Continue(cont)) => interactive_auth_continue(txns, cont).await,
        None => Err(Status::invalid_argument("missing interactive auth request")),
    }
}

async fn interactive_auth_init(
    ctx: &SysdContext,
    txns: &Txns,
    init: InteractiveAuthInitRequest,
) -> Result<InteractiveChallenge, Status> {
    let active = ctx.domains.active().await.map_err(to_status)?;

    let flow_slug = active
        .brand
        .read()
        .await
        .as_ref()
        .and_then(|b| b.flow_authentication.clone())
        .filter(|slug| !slug.is_empty())
        .ok_or_else(|| Status::internal("no authentication flow configured for domain brand"))?;

    let id = random_id(64);
    let mut txn = InteractiveAuthTransaction::new(
        id.clone(),
        ctx.clone(),
        Arc::clone(&active),
        flow_slug,
        init.username,
    )
    .await?;

    let challenge = txn.get_next_challenge().await?;
    txns.write().await.insert(id, Arc::new(Mutex::new(txn)));
    Ok(challenge)
}

async fn interactive_auth_continue(
    txns: &Txns,
    cont: InteractiveAuthContinueRequest,
) -> Result<InteractiveChallenge, Status> {
    let txn = txns.read().await.get(&cont.txid).cloned();
    let Some(txn) = txn else {
        return Err(Status::not_found("unknown interactive auth transaction"));
    };
    let mut txn = txn.lock().await;

    // Already finished: re-return the outcome with the real session id (empty
    // for non-success results, which never mint a session). Go returns a fresh
    // random id here, which is useless to a retrying client; we diverge.
    if let Some(result) = txn.result {
        return Ok(InteractiveChallenge {
            txid: cont.txid,
            finished: true,
            result: result as i32,
            session_id: txn.session_id.clone().unwrap_or_default(),
            ..Default::default()
        });
    }

    if let Some(err) = txn.solve_challenge(cont.value).await? {
        return Ok(err);
    }
    txn.get_next_challenge().await
}

pub async fn interactive_auth_async(
    ctx: &SysdContext,
    username: Option<String>,
) -> Result<InteractiveAuthAsyncResponse, Status> {
    if !interactive_supported(ctx).await {
        return Err(Status::unavailable(
            "interactive authentication is not licensed for this domain",
        ));
    }
    let active = ctx.domains.active().await.map_err(to_status)?;
    let ia = endpoints_agents_connectors_auth_ia_create(&active.api, username.as_deref())
        .await
        .map_err(|e| Status::internal(format!("failed to start interactive auth: {e}")))?;
    Ok(InteractiveAuthAsyncResponse {
        url: ia.url,
        header_token: device_token_hash(&active.cfg.token),
    })
}
