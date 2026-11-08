//! RFC 9449 DPoP (Demonstrating Proof-of-Possession) proof generation, as
//! required by authentik's OpenID Key Binding feature.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use eyre::{Result, eyre};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, jwk::Jwk};
use p256::SecretKey;
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// JOSE `typ` header value for DPoP proof JWTs (RFC 9449 section 4.2).
pub const DPOP_JWT_TYP: &str = "dpop+jwt";

/// An EC (P-256) keypair used to prove possession of a DPoP-bound token.
///
/// The same keypair must be reused across the authorization, token and
/// subsequent refresh requests of a single key-bound session: the server
/// recomputes the JWK thumbprint from each proof and compares it against the
/// one recorded at authorization time.
pub struct DpopKeyPair(SecretKey);

impl DpopKeyPair {
    /// Generate a new random P-256 keypair.
    pub fn generate() -> Self {
        Self(SecretKey::random(
            &mut p256::elliptic_curve::rand_core::OsRng,
        ))
    }

    /// Serialize the private key as a PKCS#8 PEM string, for storage.
    pub fn to_pkcs8_pem(&self) -> Result<String> {
        Ok(self.0.to_pkcs8_pem(LineEnding::LF)?.to_string())
    }

    /// Parse a keypair previously serialized with [`Self::to_pkcs8_pem`].
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self> {
        Ok(Self(SecretKey::from_pkcs8_pem(pem)?))
    }

    /// The public key as a JOSE JWK, suitable for the DPoP proof header.
    pub fn public_jwk(&self) -> Result<Jwk> {
        let jwk_ec = self.0.public_key().to_jwk();
        let value = serde_json::to_value(&jwk_ec)?;
        Ok(serde_json::from_value(value)?)
    }

    /// RFC 7638 JWK thumbprint (`dpop_jkt`): base64url(SHA-256(canonical JWK)).
    pub fn thumbprint(&self) -> Result<String> {
        let jwk_ec = self.0.public_key().to_jwk();
        let value = serde_json::to_value(&jwk_ec)?;
        let x = value
            .get("x")
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre!("EC JWK missing x coordinate"))?;
        let y = value
            .get("y")
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre!("EC JWK missing y coordinate"))?;
        // RFC 7638 requires the exact required member set for an EC key,
        // in lexicographic order, with no insignificant whitespace. Built by
        // hand rather than via a serde_json::Map, since key ordering there is
        // an implementation detail this computation must not depend on.
        let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
        Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes())))
    }

    /// A signing key usable with `jsonwebtoken::encode`.
    fn encoding_key(&self) -> Result<EncodingKey> {
        let pem = self.to_pkcs8_pem()?;
        Ok(EncodingKey::from_ec_pem(pem.as_bytes())?)
    }
}

#[derive(Serialize)]
struct DpopClaims<'a> {
    htm: &'a str,
    htu: &'a str,
    iat: i64,
    jti: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    c_s256: Option<String>,
}

/// Build a signed DPoP proof JWT (RFC 9449 section 4.2) for a single request.
///
/// `code_for_c_s256` must be `Some(raw authorization code or device code)` when
/// proving possession during the authorization_code/device_code token
/// exchange, and `None` for a refresh_token grant, which carries no `c_s256`
/// claim. `iat` and `jti` are generated fresh on every call; callers cannot
/// (and must not be able to) reuse them, since that would defeat the replay
/// protection the claims exist for.
pub fn build_proof(
    key: &DpopKeyPair,
    htm: &str,
    htu: &str,
    code_for_c_s256: Option<&str>,
) -> Result<String> {
    let mut header = Header::new(Algorithm::ES256);
    header.typ = Some(DPOP_JWT_TYP.to_string());
    header.jwk = Some(key.public_jwk()?);

    let claims = DpopClaims {
        htm,
        htu,
        iat: Utc::now().timestamp(),
        jti: Uuid::new_v4().to_string(),
        c_s256: code_for_c_s256.map(|code| URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))),
    };

    Ok(encode(&header, &claims, &key.encoding_key()?)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbprint_is_well_formed() -> Result<()> {
        let key = DpopKeyPair::generate();
        let jkt = key.thumbprint()?;
        assert_eq!(jkt.len(), 43);
        assert!(
            jkt.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        Ok(())
    }

    #[test]
    fn pkcs8_pem_round_trips() -> Result<()> {
        let key = DpopKeyPair::generate();
        let pem = key.to_pkcs8_pem()?;
        let key2 = DpopKeyPair::from_pkcs8_pem(&pem)?;
        assert_eq!(key.thumbprint()?, key2.thumbprint()?);
        Ok(())
    }

    #[test]
    fn different_keys_have_different_thumbprints() -> Result<()> {
        let a = DpopKeyPair::generate();
        let b = DpopKeyPair::generate();
        assert_ne!(a.thumbprint()?, b.thumbprint()?);
        Ok(())
    }

    #[test]
    fn public_jwk_has_no_private_material() -> Result<()> {
        let key = DpopKeyPair::generate();
        let jwk = key.public_jwk()?;
        let value = serde_json::to_value(&jwk)?;
        assert!(value.get("d").is_none());
        Ok(())
    }

    #[test]
    fn build_proof_has_expected_shape() -> Result<()> {
        let key = DpopKeyPair::generate();
        let jwt = build_proof(
            &key,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);

        let header: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0])?)?;
        assert_eq!(header["typ"], DPOP_JWT_TYP);
        assert_eq!(header["alg"], "ES256");
        assert!(header["jwk"]["x"].is_string());
        assert!(header["jwk"]["d"].is_null());

        let payload: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1])?)?;
        assert_eq!(payload["htm"], "POST");
        assert_eq!(payload["htu"], "https://example.com/application/o/token/");
        assert!(payload["c_s256"].is_null());
        assert!(payload["jti"].is_string());
        Ok(())
    }

    #[test]
    fn build_proof_includes_c_s256_when_code_given() -> Result<()> {
        let key = DpopKeyPair::generate();
        let jwt = build_proof(
            &key,
            "POST",
            "https://example.com/application/o/token/",
            Some("abc123"),
        )?;
        let parts: Vec<&str> = jwt.split('.').collect();
        let payload: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1])?)?;
        assert!(payload["c_s256"].is_string());
        Ok(())
    }

    #[test]
    fn build_proof_generates_fresh_jti_each_time() -> Result<()> {
        let key = DpopKeyPair::generate();
        let jwt1 = build_proof(
            &key,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;
        let jwt2 = build_proof(
            &key,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;
        assert_ne!(jwt1, jwt2);
        Ok(())
    }
}
