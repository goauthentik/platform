//! A minimal RFC 8628 OAuth 2.0 Device Authorization Grant client.
//!
//! Hand-rolled rather than pulled from a third-party crate because DPoP
//! (RFC 9449) needs to inject a `dpop_jkt` form parameter into the initial
//! device-authorization request and a freshly-signed `DPoP` header into every
//! token poll — neither of which a generic device-flow crate exposes a hook
//! for.

use std::time::{Duration, Instant};

use eyre::{Result, bail, eyre};
use serde::Deserialize;
use url::Url;

use crate::dpop::DpopProver;

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(5);
const SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);

/// The result of a successful device authorization request (RFC 8628 section 3.2).
pub struct DeviceAuthorization {
    /// Never shown to the user; used to poll the token endpoint and, when
    /// DPoP is enabled, as the input to the proof's `c_s256` claim.
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: Url,
    pub verification_uri_complete: Option<Url>,
    pub interval: Duration,
    pub expires_at: Instant,
}

#[derive(Deserialize)]
struct DeviceAuthorizationResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    #[serde(default)]
    interval: Option<u64>,
}

/// Start a device authorization grant (RFC 8628 section 3.1/3.2).
///
/// `dpop_jkt`, when set, is sent as the `dpop_jkt` form parameter and the
/// caller is expected to have also added the `bound_key` scope to `scopes`.
pub async fn request_device_authorization(
    device_code_url: &Url,
    client_id: &str,
    scopes: &[&str],
    dpop_jkt: Option<&str>,
    user_agent: &str,
) -> Result<DeviceAuthorization> {
    let mut form = url::form_urlencoded::Serializer::new(String::new());
    form.append_pair("client_id", client_id);
    form.append_pair("scope", &scopes.join(" "));
    if let Some(jkt) = dpop_jkt {
        form.append_pair("dpop_jkt", jkt);
    }
    let body = form.finish();

    let res = reqwest::Client::new()
        .post(device_code_url.clone())
        .header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .header(reqwest::header::USER_AGENT, user_agent)
        .body(body)
        .send()
        .await?;

    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        bail!("device authorization request failed: {body}");
    }

    let parsed: DeviceAuthorizationResponse = res.json().await?;
    Ok(DeviceAuthorization {
        device_code: parsed.device_code,
        user_code: parsed.user_code,
        verification_uri: Url::parse(&parsed.verification_uri)?,
        verification_uri_complete: parsed
            .verification_uri_complete
            .map(|u| Url::parse(&u))
            .transpose()?,
        interval: parsed
            .interval
            .map(Duration::from_secs)
            .unwrap_or(DEFAULT_POLL_INTERVAL),
        expires_at: Instant::now() + Duration::from_secs(parsed.expires_in),
    })
}

/// The token response of a completed device authorization grant.
pub struct DeviceTokenResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct TokenSuccessResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Deserialize)]
struct TokenErrorResponse {
    error: String,
}

/// Poll the token endpoint until the user completes authorization (RFC 8628
/// section 3.4/3.5), or bail out on denial/expiry.
///
/// `dpop_prover`, when set, attaches a fresh `DPoP` proof header (with
/// `c_s256` bound to `auth.device_code`) to every poll attempt.
pub async fn poll_for_device_token(
    token_url: &Url,
    client_id: &str,
    auth: &DeviceAuthorization,
    dpop_prover: Option<&dyn DpopProver>,
    user_agent: &str,
) -> Result<DeviceTokenResult> {
    let mut interval = auth.interval;
    let client = reqwest::Client::new();

    loop {
        if Instant::now() >= auth.expires_at {
            bail!("device code expired before authorization was completed");
        }
        tokio::time::sleep(interval).await;

        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "urn:ietf:params:oauth:grant-type:device_code")
            .append_pair("device_code", &auth.device_code)
            .append_pair("client_id", client_id)
            .finish();

        let mut req = client
            .post(token_url.clone())
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .header(reqwest::header::USER_AGENT, user_agent);

        if let Some(prover) = dpop_prover {
            let proof = prover
                .prove("POST", token_url.as_str(), Some(&auth.device_code))
                .await?;
            req = req.header("DPoP", proof);
        }

        let res = req.body(body).send().await?;
        let status = res.status();
        let text = res.text().await?;

        if status.is_success() {
            let parsed: TokenSuccessResponse = serde_json::from_str(&text)
                .map_err(|e| eyre!("malformed token response: {e}: {text}"))?;
            return Ok(DeviceTokenResult {
                access_token: parsed.access_token,
                refresh_token: parsed.refresh_token,
                expires_in: parsed.expires_in,
            });
        }

        let error: TokenErrorResponse = serde_json::from_str(&text)
            .map_err(|_| eyre!("device token poll failed with status {status}: {text}"))?;

        match error.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += SLOW_DOWN_INCREMENT;
                continue;
            }
            "access_denied" => bail!("authorization was denied"),
            "expired_token" => bail!("device code expired before authorization was completed"),
            other => bail!("device token poll failed: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dpop::{DpopKeyPair, DpopSigner, LocalDpopProver};
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_auth(interval: Duration) -> Result<DeviceAuthorization> {
        Ok(DeviceAuthorization {
            device_code: "test-device-code".to_string(),
            user_code: "ABCD-EFGH".to_string(),
            verification_uri: Url::parse("https://example.com/device")?,
            verification_uri_complete: None,
            interval,
            expires_at: Instant::now() + Duration::from_secs(600),
        })
    }

    #[tokio::test]
    async fn request_device_authorization_parses_response() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/device/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code": "devcode123",
                "user_code": "WXYZ-1234",
                "verification_uri": "https://example.com/verify",
                "verification_uri_complete": "https://example.com/verify?code=WXYZ-1234",
                "expires_in": 600,
                "interval": 5
            })))
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/device/", server.uri()))?;
        let auth = request_device_authorization(
            &url,
            "client-id",
            &["openid"],
            Some("thumbprint"),
            "test-agent",
        )
        .await?;

        assert_eq!(auth.device_code, "devcode123");
        assert_eq!(auth.user_code, "WXYZ-1234");
        assert_eq!(auth.interval, Duration::from_secs(5));
        assert!(auth.verification_uri_complete.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn poll_retries_on_authorization_pending_then_succeeds() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token/"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({"error": "authorization_pending"})),
            )
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/token/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "at123",
                "refresh_token": "rt123",
                "expires_in": 3600
            })))
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/token/", server.uri()))?;
        let auth = test_auth(Duration::from_millis(10))?;
        let result = poll_for_device_token(&url, "client-id", &auth, None, "test-agent").await?;

        assert_eq!(result.access_token, "at123");
        assert_eq!(result.refresh_token.as_deref(), Some("rt123"));
        Ok(())
    }

    #[tokio::test]
    async fn poll_bails_on_access_denied() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token/"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({"error": "access_denied"})),
            )
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/token/", server.uri()))?;
        let auth = test_auth(Duration::from_millis(10))?;
        let result = poll_for_device_token(&url, "client-id", &auth, None, "test-agent").await;

        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn poll_attaches_dpop_header_with_c_s256_of_device_code() -> Result<()> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "at123",
                "expires_in": 3600
            })))
            .mount(&server)
            .await;

        let url = Url::parse(&format!("{}/token/", server.uri()))?;
        let auth = test_auth(Duration::from_millis(10))?;
        let signer = DpopSigner::Software(DpopKeyPair::generate());
        let prover = LocalDpopProver(&signer);
        let result =
            poll_for_device_token(&url, "client-id", &auth, Some(&prover), "test-agent").await?;

        assert_eq!(result.access_token, "at123");

        let requests = server
            .received_requests()
            .await
            .ok_or_else(|| eyre!("request recording disabled"))?;
        let req = requests
            .first()
            .ok_or_else(|| eyre!("no request received"))?;
        let dpop_header = req
            .headers
            .get("dpop")
            .ok_or_else(|| eyre!("missing DPoP header"))?
            .to_str()?;
        let parts: Vec<&str> = dpop_header.split('.').collect();
        assert_eq!(parts.len(), 3);
        Ok(())
    }
}
