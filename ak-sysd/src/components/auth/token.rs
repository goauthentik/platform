use crate::components::SysdContext;
use crate::util::to_status;
use ak_platform::generated::agent::Token;
use ak_platform::generated::sys_auth::{TokenAuthRequest, TokenAuthResponse};
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use serde::Deserialize;
use tonic::Status;

#[derive(Deserialize)]
struct TokenClaims {
    #[serde(default)]
    preferred_username: String,
    #[serde(default)]
    iss: String,
    #[serde(default)]
    sub: String,
    #[serde(default)]
    aud: serde_json::Value,
    exp: i64,
    #[serde(default)]
    iat: Option<i64>,
    #[serde(default)]
    jti: String,
}

fn aud_contains(aud: &serde_json::Value, target: &str) -> bool {
    match aud {
        serde_json::Value::String(s) => s == target,
        serde_json::Value::Array(arr) => arr.iter().any(|v| v.as_str() == Some(target)),
        _ => false,
    }
}

/// Validates the token in `req` against the active domain's cached
/// `jwks_auth`, checking that its audience includes the domain's device id.
/// On success, creates a session via the `session` component if one is
/// registered for this platform (absent on macOS — that's expected, not an
/// error).
pub async fn token_auth(
    ctx: &SysdContext,
    req: TokenAuthRequest,
) -> Result<TokenAuthResponse, Status> {
    let active = ctx.domains.active().await.map_err(to_status)?;
    let remote = active
        .remote
        .read()
        .await
        .clone()
        .ok_or_else(|| Status::failed_precondition("domain remote config not loaded yet"))?;

    let jwks_value = serde_json::to_value(&remote.jwks_auth).map_err(to_status)?;
    let jwks: JwkSet = serde_json::from_value(jwks_value).map_err(to_status)?;

    let header = decode_header(&req.token).map_err(to_status)?;
    let kid = header
        .kid
        .ok_or_else(|| Status::invalid_argument("token is missing a kid"))?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| Status::invalid_argument("unknown signing key"))?;
    let key = DecodingKey::from_jwk(jwk).map_err(to_status)?;

    let mut validation = Validation::new(header.alg);
    validation.validate_aud = false;
    let data = decode::<TokenClaims>(&req.token, &key, &validation).map_err(to_status)?;

    if !aud_contains(&data.claims.aud, &remote.device_id) {
        return Err(Status::permission_denied("token audience mismatch"));
    }
    if !req.username.is_empty() && req.username != data.claims.preferred_username {
        return Err(Status::permission_denied("token username mismatch"));
    }

    #[cfg_attr(
        not(any(target_os = "linux", target_os = "windows")),
        allow(unused_mut)
    )]
    let mut session_id = String::new();
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        if let Some(session) = ctx
            .registry
            .get::<crate::components::session::SessionComponent>("session")
        {
            match session
                .new_session(
                    data.claims.preferred_username.clone(),
                    req.token.clone(),
                    Some(data.claims.exp),
                )
                .await
            {
                Ok(rec) => session_id = rec.id,
                Err(e) => tracing::warn!("failed to create session: {e:?}"),
            }
        } else {
            tracing::debug!("session component not registered, skipping session creation");
        }
    }

    Ok(TokenAuthResponse {
        successful: true,
        token: Some(Token {
            preferred_username: data.claims.preferred_username,
            iss: data.claims.iss,
            sub: data.claims.sub,
            aud: match &data.claims.aud {
                serde_json::Value::String(s) => vec![s.clone()],
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => vec![],
            },
            exp: Some(pbjson_types::Timestamp {
                seconds: data.claims.exp,
                nanos: 0,
            }),
            nbf: None,
            iat: data.claims.iat.map(|s| pbjson_types::Timestamp {
                seconds: s,
                nanos: 0,
            }),
            jti: data.claims.jti,
        }),
        session_id,
    })
}
