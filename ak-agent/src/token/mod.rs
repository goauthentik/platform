use ak_platform::shared::AuthentikClaims;
use eyre::Result;
use jsonwebtoken::dangerous::insecure_decode;
use serde::{Deserialize, Serialize};

pub mod global;
pub mod profile;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
}

impl Token {
    pub fn claims(&self) -> Result<AuthentikClaims> {
        parse_unverified(&self.access_token)
    }
}

pub(crate) fn parse_unverified(token: &str) -> Result<AuthentikClaims> {
    let data = insecure_decode::<AuthentikClaims>(token).map_err(|e| eyre::eyre!("{e}"))?;
    Ok(data.claims)
}

/// Test-only HTTP mocking. No mocking crate is a workspace dependency, so
/// this hand-rolls a minimal single-shot server: good enough for our
/// callers, which only need a valid HTTP response, not real protocol
/// semantics.
#[cfg(test)]
pub(crate) mod testutils {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Binds to an ephemeral loopback port, accepts exactly one connection,
    /// ignores whatever request it sends, and writes back a fixed response.
    /// Returns the base URL to hit.
    pub(crate) async fn spawn_http_once(status_line: &'static str, body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let resp = format!(
                    "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });
        format!("http://{addr}")
    }

    /// `GlobalTokenManager::new` enforces a process-wide singleton
    /// (`GLOBAL_CREATED`), which would race if the test harness ran two
    /// tests that each construct one concurrently. Tests that do so should
    /// hold this lock for the manager's whole lifetime.
    pub(crate) fn gtm_test_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }
}
