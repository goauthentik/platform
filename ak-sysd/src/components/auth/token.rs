use crate::components::auth::AuthComponent;
use crate::util::to_status;
use ak_platform::generated::agent::Token;
use ak_platform::generated::sys_auth::system_auth_token_server::SystemAuthToken;
use ak_platform::generated::sys_auth::{
    SshCertAuthRequest, SshCertAuthResponse, TokenAuthRequest, TokenAuthResponse,
};
use ak_platform::shared::{
    AuthentikClaims, EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY, EXT_AUTHENTIK_PLATFORM_SSH_TOKEN,
};
use authentik_client::models::AgentConfig;
use eyre::Result;
use jsonwebtoken::TokenData;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use ssh_key::{Certificate, PublicKey};
use subtle::ConstantTimeEq;
use tonic::{Request, Response, Status};

impl AuthComponent {
    /// Returns whether the cert's embedded host-key extension matches one of the
    /// locally-trusted SSH host public keys.
    fn verify_cert_host_key(&self, cert: &Certificate) -> Result<bool> {
        let ext_host_key = cert
            .extensions()
            .0
            .get(EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY)
            .ok_or_else(|| eyre::eyre!("invalid cert (no host key ext)"))?;
        let given = PublicKey::from_openssh(ext_host_key)?.to_bytes()?;
        Ok(self
            .local_host_keys()?
            .iter()
            .any(|hk| bool::from(hk.as_slice().ct_eq(given.as_slice()))))
    }

    /// Reads locally-trusted SSH host public keys.
    fn local_host_keys(&self) -> Result<Vec<Vec<u8>>> {
        let vendor = ak_platform_facts::vendor::gather();
        let Some(serde_json::Value::Array(keys)) = vendor.get("ssh_host_keys") else {
            tracing::debug!("No ssh_host_keys!");
            return Ok(vec![]);
        };
        let mut out = Vec::with_capacity(keys.len());
        for k in keys {
            let Some(k) = k.as_str() else { continue };
            let kk = k.strip_prefix("localhost ").unwrap_or(k);
            match PublicKey::from_openssh(kk) {
                Ok(pk) => out.push(pk.to_bytes()?),
                Err(e) => tracing::warn!("failed to parse local host key: {e:?}"),
            }
        }
        Ok(out)
    }

    pub async fn validate_token(
        &self,
        raw_token: String,
        remote: Option<AgentConfig>,
    ) -> Result<TokenData<AuthentikClaims>> {
        let remote = match remote {
            Some(r) => r,
            None => {
                let active = self.ctx.domains.active().await.map_err(to_status)?;
                active.remote.read().await.clone().ok_or_else(|| {
                    Status::failed_precondition("domain remote config not loaded yet")
                })?
            }
        };

        let jwks_value = serde_json::to_value(&remote.jwks_auth).map_err(to_status)?;
        let jwks: JwkSet = serde_json::from_value(jwks_value).map_err(to_status)?;

        let header = decode_header(&raw_token).map_err(to_status)?;
        let kid = header
            .kid
            .ok_or_else(|| Status::invalid_argument("token is missing a kid"))?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| Status::invalid_argument("unknown signing key"))?;
        let key = DecodingKey::from_jwk(jwk).map_err(to_status)?;

        let mut validation = Validation::new(header.alg);
        validation.validate_aud = false;
        validation.validate_nbf = true;
        let data = decode::<AuthentikClaims>(&raw_token, &key, &validation).map_err(to_status)?;
        Ok(data)
    }

    pub fn extract_ssh_cert_token(
        &self,
        ssh_auth: String,
    ) -> Result<(Certificate, String), Status> {
        let cert = Certificate::from_openssh(&ssh_auth).map_err(to_status)?;

        let ext_token = cert
            .extensions()
            .0
            .get(EXT_AUTHENTIK_PLATFORM_SSH_TOKEN)
            .ok_or_else(|| Status::invalid_argument("invalid cert (no token ext)"))?
            .clone();

        if !self.verify_cert_host_key(&cert).map_err(to_status)? {
            return Err(Status::permission_denied("certificate has wrong host-key"));
        }
        Ok((cert, ext_token))
    }
}

#[tonic::async_trait]
impl SystemAuthToken for AuthComponent {
    async fn ssh_cert_auth(
        &self,
        request: Request<SshCertAuthRequest>,
    ) -> Result<Response<SshCertAuthResponse>, Status> {
        let req = request.into_inner();
        let (cert, ext_token) =
            self.extract_ssh_cert_token(format!("{} {}", req.r#type, req.b64key))?;

        let res = self
            .token_auth(Request::new(TokenAuthRequest {
                username: req.user,
                token: ext_token,
            }))
            .await?;
        let token = match res.into_inner().token {
            Some(t) => t,
            None => {
                return Err(Status::unauthenticated("failed token authentication"));
            }
        };

        let pubkey_line = PublicKey::from(cert.signature_key().clone())
            .to_openssh()
            .map_err(|e| Status::from_error(e.into()))?
            .trim()
            .to_string();

        let principal = &token.preferred_username;
        if principal.contains(['"', '\n', '\r', '\\']) {
            return Err(Status::invalid_argument("invalid characters in username"));
        }
        let lines = vec![format!(
            "cert-authority,principals=\"{principal}\" {pubkey_line}"
        )];
        Ok(Response::new(SshCertAuthResponse { lines }))
    }

    async fn token_auth(
        &self,
        request: Request<TokenAuthRequest>,
    ) -> Result<Response<TokenAuthResponse>, Status> {
        let req = request.into_inner();
        let active = self.ctx.domains.active().await.map_err(to_status)?;
        let remote =
            active.remote.read().await.clone().ok_or_else(|| {
                Status::failed_precondition("domain remote config not loaded yet")
            })?;

        let token = self
            .validate_token(req.token, Some(remote.clone()))
            .await
            .map_err(|e| Status::from_error(e.into()))?;
        if !token
            .claims
            .aud
            .clone()
            .iter()
            .any(|v| v.as_str() == remote.device_id)
        {
            return Err(Status::permission_denied("token audience mismatch"));
        }
        if !req.username.is_empty() && req.username != token.claims.preferred_username {
            return Err(Status::permission_denied("token username mismatch"));
        }

        #[cfg_attr(
            not(any(target_os = "linux", target_os = "windows")),
            allow(unused_mut)
        )]
        let mut session_id = String::new();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            if let Some(session) = self
                .ctx
                .registry
                .get::<crate::components::session::SessionComponent>("session")
            {
                match session
                    .new_session(
                        data.claims.preferred_username.clone(),
                        req.token.clone(),
                        Some(data.claims.exp.timestamp()),
                    )
                    .await
                {
                    Ok(rec) => session_id = rec.id,
                    Err(e) => {
                        tracing::warn!("failed to create session: {e:?}");
                        return Err(Status::not_found("unable to create session"));
                    }
                }
            } else {
                tracing::debug!("session component not registered, skipping session creation");
            }
        }

        Ok(Response::new(TokenAuthResponse {
            successful: true,
            token: Some(Token {
                preferred_username: token.claims.preferred_username,
                iss: token.claims.iss,
                sub: token.claims.sub.unwrap_or_default(),
                aud: token.claims.aud,
                exp: Some(token.claims.exp.into()),
                nbf: None,
                iat: Some(token.claims.iat.into()),
                jti: token.claims.jti.unwrap_or_default(),
            }),
            session_id,
        }))
    }
}

#[cfg(test)]
mod test {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::context::testutils::test_context;
    use ak_agent::ssh::txn_keys::{generate_cert, generate_ssh_private_key};

    fn valid_before() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .saturating_add(5000)
    }

    #[tokio::test]
    async fn test_local_keys() {
        let auth = AuthComponent::new(test_context().await);
        let local = auth.local_host_keys().unwrap();
        assert!(local.len() > 0);
    }

    #[tokio::test]
    async fn test_host_key_trusted() {
        let auth = AuthComponent::new(test_context().await);
        let ca = generate_ssh_private_key().expect("ca key");

        // A real local host key is trusted.
        let local = auth.local_host_keys().expect("local host keys");
        let host_key_bytes = local.first().expect("at least one local host key");
        let host_pub = PublicKey::from_bytes(host_key_bytes).expect("decode local host key");
        let cert = generate_cert(&ca, "test-user", host_pub.key_data(), "tok", valid_before())
            .expect("cert gen");
        assert!(
            auth.verify_cert_host_key(&cert).expect("host key check"),
            "real local host key should be trusted"
        );

        // A throwaway host key is not trusted.
        let bogus = generate_ssh_private_key().expect("bogus host key");
        let cert = generate_cert(
            &ca,
            "test-user",
            bogus.public_key().key_data(),
            "tok",
            valid_before(),
        )
        .expect("cert gen");
        assert!(
            !auth.verify_cert_host_key(&cert).expect("host key check"),
            "throwaway host key should not be trusted"
        );
    }
}
