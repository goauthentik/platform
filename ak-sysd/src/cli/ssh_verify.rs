use ak_platform::client::sysd::Client;
use ak_platform::generated::sys_auth::SshCertAuthRequest;
use ak_platform::paths::SysdSocketID;
use eyre::Result;

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
    Client::new(SysdSocketID::Default)
        .await?
        .auth_token()
        .ssh_cert_auth(SshCertAuthRequest {
            user: user.to_string(),
            b64key: b64key.to_string(),
            r#type: typ.to_string(),
        })
        .await?
        .into_inner()
        .lines
        .iter()
        .for_each(|l| {
            println!("{}", l);
        });
    Ok(())
}
