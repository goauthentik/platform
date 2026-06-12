use ak_platform::prelude::*;
use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

pub mod global;
pub mod profile;

#[derive(Debug, Deserialize)]
pub struct AuthentikClaims {
    pub preferred_username: Option<String>,
    pub exp: u64,
    pub sub: Option<String>,
}

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
    let header = decode_header(token)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })?;
    let mut validation = Validation::new(header.alg);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    let data = decode::<AuthentikClaims>(token, &DecodingKey::from_secret(&[]), &validation)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::from(e.to_string()) })?;
    Ok(data.claims)
}
