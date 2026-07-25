use ak_platform::generated::ic_pam_fido::{FidoRequest, FidoResponse};
use ak_platform::generated::sys_auth::{InteractiveChallenge, interactive_challenge::PromptMeta};
use ak_platform::grpc::{decode_pb, encode_pb};
use authentik_client::models::{
    AuthenticatorValidationChallengeResponseRequest, DeviceChallenge, FlowChallengeResponseRequest,
};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use serde::Deserialize;
use serde_json::json;
use tonic::Status;

const COMPONENT: &str = "ak-stage-authenticator-validate";

#[derive(Deserialize)]
struct WebauthnChallenge {
    challenge: String,
    #[serde(rename = "rpId")]
    rp_id: String,
    #[serde(rename = "allowCredentials", default)]
    allow_credentials: Vec<AllowCredential>,
    #[serde(rename = "userVerification", default)]
    user_verification: String,
}

#[derive(Deserialize)]
struct AllowCredential {
    id: String,
}

/// clientDataJSON as the WebAuthn client would produce it. Built and verified
/// through this one helper so both sides stay byte-identical.
fn build_client_data_json(challenge: &str, origin: &str) -> Result<Vec<u8>, Status> {
    serde_json::to_vec(&json!({
        "type": "webauthn.get",
        "challenge": challenge,
        "origin": origin,
    }))
    .map_err(|e| Status::internal(format!("failed to build clientDataJSON: {e}")))
}

fn parse_challenge(dc: &DeviceChallenge) -> Result<WebauthnChallenge, Status> {
    let value = serde_json::to_value(&dc.challenge)
        .map_err(|e| Status::internal(format!("invalid webauthn challenge: {e}")))?;
    serde_json::from_value(value)
        .map_err(|e| Status::internal(format!("invalid webauthn challenge: {e}")))
}

/// Turns an authentik webauthn device challenge into a binary PAM prompt
/// carrying a base64 `FidoRequest`.
pub fn parse_webauthn_request(
    txid: &str,
    dc: &DeviceChallenge,
    origin: &str,
) -> Result<InteractiveChallenge, Status> {
    let ch = parse_challenge(dc)?;
    let client_data = build_client_data_json(&ch.challenge, origin)?;

    let mut credential_ids = Vec::with_capacity(ch.allow_credentials.len());
    for cred in &ch.allow_credentials {
        credential_ids.push(
            BASE64_URL_SAFE_NO_PAD
                .decode(&cred.id)
                .map_err(|e| Status::internal(format!("invalid credential id: {e}")))?,
        );
    }

    let req = FidoRequest {
        rp_id: ch.rp_id,
        challenge: client_data,
        credential_ids,
        uv: matches!(ch.user_verification.as_str(), "preferred" | "required"),
    };

    Ok(InteractiveChallenge {
        txid: txid.to_string(),
        prompt: encode_pb(req).map_err(|e| Status::internal(e.to_string()))?,
        prompt_meta: PromptMeta::PamBinaryPrompt as i32,
        component: COMPONENT.to_string(),
        ..Default::default()
    })
}

/// Turns the client's base64 `FidoResponse` into the flow challenge response.
pub fn parse_webauthn_response(
    raw: &str,
    dc: &DeviceChallenge,
    origin: &str,
) -> Result<FlowChallengeResponseRequest, Status> {
    let resp = decode_pb::<FidoResponse>(raw.to_string())
        .map_err(|e| Status::internal(format!("invalid fido response: {e}")))?;
    let ch = parse_challenge(dc)?;
    let client_data = build_client_data_json(&ch.challenge, origin)?;

    let cred_id = BASE64_URL_SAFE_NO_PAD.encode(&resp.credential_id);
    let webauthn = json!({
        "id": cred_id,
        "rawId": cred_id,
        "type": "public-key",
        "response": {
            "clientDataJSON": BASE64_URL_SAFE_NO_PAD.encode(&client_data),
            "authenticatorData": BASE64_URL_SAFE_NO_PAD.encode(&resp.authenticator_data),
            "signature": BASE64_URL_SAFE_NO_PAD.encode(&resp.signature),
            "userHandle": serde_json::Value::Null,
        },
    });

    let webauthn = match webauthn {
        serde_json::Value::Object(map) => map.into_iter().collect(),
        _ => return Err(Status::internal("failed to build webauthn response")),
    };

    Ok(FlowChallengeResponseRequest::AkStageAuthenticatorValidate(
        AuthenticatorValidationChallengeResponseRequest {
            component: Some(COMPONENT.to_string()),
            webauthn: Some(webauthn),
            ..AuthenticatorValidationChallengeResponseRequest::new()
        },
    ))
}
