use ssh_key::{
    Algorithm, PrivateKey,
    certificate::{Builder, CertType},
    public::KeyData,
    rand_core::OsRng,
};

use eyre::{Result, WrapErr};

pub const EXT_AUTHENTIK_PLATFORM_SSH_TOKEN: &str = "goauthentik.io/platform/ssh/ssh/token";
pub const EXT_AUTHENTIK_PLATFORM_SSH_HOST_KEY: &str = "goauthentik.io/platform/ssh/host-key";

pub fn generate_ssh_private_key() -> Result<PrivateKey> {
    PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
        .wrap_err("failed to generate SSH private key")
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
        .wrap_err("failed to serialize host public key")?;

    let mut builder = Builder::new_with_random_nonce(
        &mut OsRng,
        priv_key.public_key().key_data().clone(),
        0,
        valid_before,
    )
    .wrap_err("failed to create certificate builder")?;

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
        .wrap_err("failed to sign SSH certificate")
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
