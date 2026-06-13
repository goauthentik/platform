use ssh_key::{
    Algorithm, PrivateKey,
    certificate::{Builder, CertType},
    public::KeyData,
    rand_core::OsRng,
};

use ak_platform::prelude::*;

pub const EXT_AUTHENTIK_PLATFORM_SSH_TOKEN: &str = "goauthentik.io/platform/ssh/ssh/token";
pub const EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY: &str = "goauthentik.io/platform/ssh/host-key";

pub fn generate_ssh_private_key() -> Result<PrivateKey> {
    PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
        .map_err(|e| -> BoxError { Box::from(e.to_string()) })
}

pub fn generate_cert(
    priv_key: &PrivateKey,
    username: &str,
    host_key: &KeyData,
    host_token_str: &str,
    valid_before: u64,
) -> Result<ssh_key::Certificate> {
    let host_pubkey = ssh_key::PublicKey::from(host_key.clone());
    let host_key_openssh = host_pubkey
        .to_openssh()
        .map_err(|e| -> BoxError { Box::from(e.to_string()) })?;

    let mut builder = Builder::new_with_random_nonce(
        &mut OsRng,
        priv_key.public_key().key_data().clone(),
        0,
        valid_before,
    )
    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })?;

    builder
        .cert_type(CertType::User)?
        .valid_principal(username)?
        .key_id(username)?
        .extension("permit-X11-forwarding", "")?
        .extension("permit-agent-forwarding", "")?
        .extension("permit-port-forwarding", "")?
        .extension("permit-pty", "")?
        .extension("permit-user-rc", "")?
        .extension(EXT_AUTHENTIK_PLATFORM_SSH_TOKEN, host_token_str)?
        .extension(EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY, host_key_openssh.trim())?;

    builder
        .sign(priv_key)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_key() {
        let k = generate_ssh_private_key();
        assert_eq!(k.is_ok(), true);
    }
}
