use ak_platform::client::sysd::Client;
use ak_platform::generated::sys_ctrl::DomainEnrollRequest;
use ak_platform::paths::SysdSocketID;
use eyre::Result;
use std::io::IsTerminal;

/// Reads an enrollment token: `AK_SYS_INSECURE_ENV_TOKEN` first, otherwise an
/// interactive password-style prompt (or a plain stdin line if not a TTY)
fn read_token() -> Result<String> {
    if let Ok(env_token) = std::env::var("AK_SYS_INSECURE_ENV_TOKEN")
        && !env_token.is_empty()
    {
        return Ok(env_token);
    }

    if std::io::stdin().is_terminal() {
        Ok(rpassword::prompt_password(
            "Enter authentik enrollment token: ",
        )?)
    } else {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line.trim_end_matches(['\n', '\r']).to_string())
    }
}

pub async fn join(name: String, authentik_url: String) -> Result<()> {
    let token = read_token()?;
    let client = Client::new(SysdSocketID::CTRL).await?;
    client
        .ctrl()
        .domain_enroll(DomainEnrollRequest {
            name,
            authentik_url,
            token,
        })
        .await?;
    Ok(())
}
