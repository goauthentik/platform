//! RFC 9449 DPoP (Demonstrating Proof-of-Possession) proof generation, as
//! required by authentik's OpenID Key Binding feature.

use ak_platform_keyring::hardware::HardwareSigningKey;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use eyre::{Result, eyre};
use jsonwebtoken::jwk::Jwk;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding};
use p256::{PublicKey, SecretKey};
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
        jwk_from_public_key(&self.0.public_key())
    }

    /// RFC 7638 JWK thumbprint (`dpop_jkt`): base64url(SHA-256(canonical JWK)).
    pub fn thumbprint(&self) -> Result<String> {
        thumbprint_from_public_key(&self.0.public_key())
    }

    /// Sign `data`, returning the raw (R || S) 64-byte ECDSA P-256 signature
    /// JOSE/JWS expects (SHA-256 is applied internally by the signing key).
    fn sign_raw(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signing_key = SigningKey::from(&self.0);
        let sig: Signature = signing_key.sign(data);
        Ok(sig.to_bytes().to_vec())
    }
}

/// Either a locally-held software keypair or a hardware-enclave-backed key
/// (Secure Enclave / TPM 2.0 / OS keyring) used to sign DPoP proofs. Both
/// variants are driven through the same manually-assembled JWS in
/// [`build_proof`], since a hardware-backed key's private material can never
/// be exported into a `jsonwebtoken::EncodingKey`.
pub enum DpopSigner {
    Software(DpopKeyPair),
    Hardware(HardwareSigningKey),
}

impl DpopSigner {
    pub fn public_jwk(&self) -> Result<Jwk> {
        match self {
            Self::Software(kp) => kp.public_jwk(),
            Self::Hardware(hk) => {
                jwk_from_public_key(&public_key_from_sec1(&hk.public_key_sec1()?)?)
            }
        }
    }

    pub fn thumbprint(&self) -> Result<String> {
        match self {
            Self::Software(kp) => kp.thumbprint(),
            Self::Hardware(hk) => {
                thumbprint_from_public_key(&public_key_from_sec1(&hk.public_key_sec1()?)?)
            }
        }
    }

    fn sign_raw(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::Software(kp) => kp.sign_raw(data),
            Self::Hardware(hk) => {
                let der = hk.sign_der(data)?;
                Ok(Signature::from_der(&der)?.to_bytes().to_vec())
            }
        }
    }
}

fn public_key_from_sec1(sec1: &[u8]) -> Result<PublicKey> {
    Ok(PublicKey::from_sec1_bytes(sec1)?)
}

fn jwk_from_public_key(pubkey: &PublicKey) -> Result<Jwk> {
    let jwk_ec = pubkey.to_jwk();
    let value = serde_json::to_value(&jwk_ec)?;
    Ok(serde_json::from_value(value)?)
}

fn thumbprint_from_public_key(pubkey: &PublicKey) -> Result<String> {
    let jwk_ec = pubkey.to_jwk();
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

#[derive(Serialize)]
struct DpopHeader {
    typ: &'static str,
    alg: &'static str,
    jwk: Jwk,
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
///
/// The JWS is assembled by hand (rather than via `jsonwebtoken::encode`) so
/// that a hardware-backed [`DpopSigner::Hardware`] key — whose private
/// material never leaves the enclave — can sign the same way a software key
/// does: only the raw signing-input bytes cross the `DpopSigner` boundary.
pub fn build_proof(
    signer: &DpopSigner,
    htm: &str,
    htu: &str,
    code_for_c_s256: Option<&str>,
) -> Result<String> {
    let header = DpopHeader {
        typ: DPOP_JWT_TYP,
        alg: "ES256",
        jwk: signer.public_jwk()?,
    };
    let claims = DpopClaims {
        htm,
        htu,
        iat: Utc::now().timestamp(),
        jti: Uuid::new_v4().to_string(),
        c_s256: code_for_c_s256.map(|code| URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))),
    };

    let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
    let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let signing_input = format!("{header_b64}.{claims_b64}");
    let signature_b64 = URL_SAFE_NO_PAD.encode(signer.sign_raw(signing_input.as_bytes())?);

    Ok(format!("{signing_input}.{signature_b64}"))
}

/// Produces a DPoP proof for a single request, without exposing whatever key
/// material (or lack thereof) backs it.
///
/// The device flow in [`crate::oauth::device_flow`] runs from `ak-cli`, which
/// never holds a `DpopSigner` itself — DPoP keys live in `ak-agent` (and may be
/// hardware-backed, i.e. non-exportable). Implementations there proxy this
/// call over gRPC to whichever process actually holds the key; [`LocalDpopProver`]
/// is the trivial implementation for callers (like `ak-agent`'s own token
/// refresh) that already hold a `DpopSigner` locally.
#[tonic::async_trait]
pub trait DpopProver: Send + Sync {
    async fn prove(&self, htm: &str, htu: &str, code_for_c_s256: Option<&str>) -> Result<String>;
}

/// Trivial [`DpopProver`] wrapping a locally-held [`DpopSigner`].
pub struct LocalDpopProver<'a>(pub &'a DpopSigner);

#[tonic::async_trait]
impl DpopProver for LocalDpopProver<'_> {
    async fn prove(&self, htm: &str, htu: &str, code_for_c_s256: Option<&str>) -> Result<String> {
        build_proof(self.0, htm, htu, code_for_c_s256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn software(key: DpopKeyPair) -> DpopSigner {
        DpopSigner::Software(key)
    }

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
        let signer = software(key);
        let jwt = build_proof(
            &signer,
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
        let signer = software(key);
        let jwt = build_proof(
            &signer,
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
        let signer = software(DpopKeyPair::generate());
        let jwt1 = build_proof(
            &signer,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;
        let jwt2 = build_proof(
            &signer,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;
        assert_ne!(jwt1, jwt2);
        Ok(())
    }

    #[test]
    fn build_proof_verifies_against_jsonwebtoken() -> Result<()> {
        // The JWS is now hand-assembled rather than produced by
        // `jsonwebtoken::encode` — confirm a standard JOSE decoder still
        // accepts it and that the signature verifies against the embedded jwk.
        let key = DpopKeyPair::generate();
        let jwk = key.public_jwk()?;
        let signer = software(key);
        let jwt = build_proof(
            &signer,
            "POST",
            "https://example.com/application/o/token/",
            None,
        )?;

        let decoding_key = jsonwebtoken::DecodingKey::from_jwk(&jwk)?;
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        let data = jsonwebtoken::decode::<serde_json::Value>(&jwt, &decoding_key, &validation)?;
        assert_eq!(data.claims["htm"], "POST");
        Ok(())
    }
}
