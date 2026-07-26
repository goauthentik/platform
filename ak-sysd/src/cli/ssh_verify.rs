use ak_platform::generated::sys_auth::TokenAuthRequest;
use ak_platform::paths::SysdSocketID;
use ak_platform::shared::EXT_AUTHENTIK_PLATFORM_SSH_TOKEN;
use ak_platform::{client::sysd::Client, shared::EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY};
use eyre::{Result, bail};
use ssh_key::{Certificate, PublicKey};
use subtle::ConstantTimeEq;

/// Used as an OpenSSH `AuthorizedPrincipalsCommand`: on any failure, prints
/// nothing and returns without propagating an error — sshd must never be
/// blocked by this command failing, mirroring Go's `ssh_verify.go` `Run`
/// (which only logs a warning and returns nil).
pub async fn verify(user: String, b64key: String, typ: String) {
    if let Err(e) = validate(&user, &b64key, &typ).await {
        tracing::warn!("failed to verify ssh cert: {e:?}");
    }
}

async fn validate(user: &str, b64key: &str, typ: &str) -> Result<()> {
    let cert = Certificate::from_openssh(&format!("{typ} {b64key}"))?;

    let ext_host_key = cert
        .extensions()
        .0
        .get(EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY)
        .ok_or_else(|| eyre::eyre!("invalid cert (no host key ext)"))?;
    let ext_token = cert
        .extensions()
        .0
        .get(EXT_AUTHENTIK_PLATFORM_SSH_TOKEN)
        .ok_or_else(|| eyre::eyre!("invalid cert (no token ext)"))?;

    let given_host_key = PublicKey::from_openssh(ext_host_key)?.to_bytes()?;
    let local_host_keys = local_host_keys()?;
    let found = local_host_keys
        .iter()
        .any(|hk| bool::from(hk.as_slice().ct_eq(given_host_key.as_slice())));
    if !found {
        bail!("certificate has wrong host-key");
    }

    let client = Client::new(SysdSocketID::Default).await?;
    let res = client
        .auth_token()
        .token_auth(TokenAuthRequest {
            username: user.to_string(),
            token: ext_token.clone(),
        })
        .await?
        .into_inner();
    if !res.successful {
        bail!("unsuccessful token validation");
    }
    let preferred_username = res
        .token
        .ok_or_else(|| eyre::eyre!("token auth response missing token"))?
        .preferred_username;

    let pubkey_line = PublicKey::from(cert.signature_key().clone())
        .to_openssh()?
        .trim()
        .to_string();
    println!("cert-authority,principals=\"{preferred_username}\" {pubkey_line}");
    Ok(())
}

/// Reads locally-trusted SSH host public keys
fn local_host_keys() -> Result<Vec<Vec<u8>>> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_verify() {
        let local = local_host_keys().unwrap();
        assert!(local.len() > 0);
    }
}
