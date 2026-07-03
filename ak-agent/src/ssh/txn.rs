use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ak_platform::{net::server::creds::ProcCredentials, string::PlatformString};
use eyre::{Result, WrapErr, bail};
use ak_platform_authz::AuthorizeAction;
use authentik_client::apis::endpoints_api::endpoints_agents_connectors_auth_fed_create;
use ssh_key::{Certificate, PrivateKey, public::KeyData};
use uuid::Uuid;

use crate::Agent;
use crate::ssh::txn_keys::generate_cert;

pub struct SSHAgentTransaction {
    pub agent: Arc<Agent>,
    pub priv_key: Arc<PrivateKey>,
    pub creds: ProcCredentials,
    pub host_key: Option<KeyData>,
    pub session_id: Option<Vec<u8>>,
    pub cert: Option<Arc<Certificate>>,
    pub id: Uuid,
}

impl SSHAgentTransaction {
    pub(crate) async fn ensure_cert(&mut self) -> Option<Arc<Certificate>> {
        if let Some(ref c) = self.cert {
            return Some(Arc::clone(c));
        }

        let host_key = match &self.host_key {
            Some(k) => k.clone(),
            None => {
                tracing::debug!("ssh-agent: ensure_cert: no host key set yet");
                return None;
            }
        };

        let profile = self.agent.cfg.read().await.active_profile.clone();
        let token_mgr = match self.agent.gtm.for_profile(profile.clone()).await {
            Some(m) => m,
            None => {
                tracing::warn!(
                    profile = profile,
                    "ssh-agent: ensure_cert: profile not found"
                );
                return None;
            }
        };

        let root_token = match token_mgr.token().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("ssh-agent: ensure_cert: failed to get root token: {e:?}");
                return None;
            }
        };

        let claims = match root_token.claims() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("ssh-agent: ensure_cert: failed to parse token claims: {e:?}");
                return None;
            }
        };

        let (host_token_str, expires_in) = match self.get_host_token(&host_key).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("ssh-agent: ensure_cert: failed to get host token: {e:?}");
                return None;
            }
        };

        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let valid_before = now_secs.saturating_add(expires_in as u64);

        let cert = match generate_cert(
            &self.priv_key,
            &claims.preferred_username,
            &host_key,
            &host_token_str,
            valid_before,
        ) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("ssh-agent: ensure_cert: failed to generate cert: {e:?}");
                return None;
            }
        };

        let cert = Arc::new(cert);
        self.cert = Some(Arc::clone(&cert));
        Some(cert)
    }

    async fn get_host_token(&self, host_key: &KeyData) -> Result<(String, i64)> {
        let profile = {
            let profile = self.agent.cfg.read().await.active_profile.clone();
            let cfg = self.agent.cfg.read().await;
            cfg.profiles
                .get(&profile)
                .ok_or_else(|| eyre::eyre!("profile {} not found", profile))?
                .clone()
        };

        let pk = ssh_key::PublicKey::from(host_key.clone());
        let host_key_str = pk
            .to_openssh()
            .wrap_err("failed to serialize host public key")?;
        let host_key_trimmed = host_key_str.trim().to_string();

        self.authorize(&host_key_trimmed).await?;

        let device_name = format!("localhost {}", host_key_trimmed);
        let api_config = profile.api_config()?;

        let dt = endpoints_agents_connectors_auth_fed_create(&api_config, &device_name)
            .await
            .map_err(|e| eyre::eyre!("{e}"))?;

        Ok((dt.token, dt.expires_in.unwrap_or(0) as i64))
    }

    async fn authorize(&self, host_key_str: &str) -> Result<()> {
        let hk1 = host_key_str.to_string();
        let hk2 = host_key_str.to_string();

        let result = AuthorizeAction {
            message: Box::new(move |c| {
                let cmd = c.clone().proc_info()?.parent_cmdline()?;
                Ok(PlatformString::new()
                    .with_darwin(format!("authorize access device '{hk1}' in '{cmd}'"))
                    .with_windows(format!("'{hk1}' is attempting to access '{cmd}'"))
                    .with_linux(format!("'{hk1}' is attempting to access '{cmd}'")))
            }),
            uid: Box::new(move |c| {
                let pid = c.clone().proc_info()?.unique_process_id()?;
                Ok(format!("{hk2}:{pid}"))
            }),
            timeout_success: Duration::from_secs(30 * 60),
            timeout_denied: Duration::from_secs(5 * 60),
        }
        .prompt(self.creds.clone())
        .await?;

        if !result {
            bail!("authorization denied by user");
        }
        Ok(())
    }
}
