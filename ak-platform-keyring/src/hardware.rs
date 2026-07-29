//! Hardware-backed (Secure Enclave / TPM 2.0 / OS keyring) P-256 signing keys,
//! via the `hardware-enclave` crate.

use std::fmt::Display;

use hardware_enclave::{AccessPolicy, EnclaveConfig, Error, create_signer};

/// A P-256 signing key backed by the platform's hardware security module.
/// Private key material never leaves the enclave; only signing operations
/// and the public key are ever produced.
pub struct HardwareSigningKey {
    signer: hardware_enclave::SignerHandle,
    label: String,
}

#[derive(Debug)]
pub enum HardwareKeyError {
    /// No usable Secure Enclave / TPM / keyring backend exists on this device.
    NotAvailable,
    Other(eyre::Report),
}

impl Display for HardwareKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareKeyError::NotAvailable => write!(f, "hardware key storage not available"),
            HardwareKeyError::Other(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for HardwareKeyError {}

fn map_err(e: Error) -> HardwareKeyError {
    match e {
        Error::NotAvailable => HardwareKeyError::NotAvailable,
        other => HardwareKeyError::Other(eyre::eyre!(other)),
    }
}

impl HardwareSigningKey {
    /// Opens the key for `label` if it already exists, generating it otherwise.
    ///
    /// Generation uses `AccessPolicy::None` (no biometric/PIN prompt): this key
    /// signs silently on every background token refresh, so gating it behind
    /// user presence would break unattended renewal.
    pub fn open_or_generate(app_name: &str, label: &str) -> Result<Self, HardwareKeyError> {
        let signer = create_signer(&EnclaveConfig::new(app_name, label)).map_err(map_err)?;
        match signer.public_key(label) {
            Ok(_) => {}
            Err(Error::KeyNotFound { .. }) => {
                signer
                    .generate_key(label, AccessPolicy::None)
                    .map_err(map_err)?;
            }
            Err(e) => return Err(map_err(e)),
        }
        Ok(Self {
            signer,
            label: label.to_string(),
        })
    }

    /// The uncompressed SEC1 public key: `0x04 || X (32 bytes) || Y (32 bytes)`.
    pub fn public_key_sec1(&self) -> Result<Vec<u8>, HardwareKeyError> {
        self.signer.public_key(&self.label).map_err(map_err)
    }

    /// Sign `data` (SHA-256 applied internally by the enclave). Returns a
    /// DER-encoded ECDSA P-256 signature.
    pub fn sign_der(&self, data: &[u8]) -> Result<Vec<u8>, HardwareKeyError> {
        self.signer.sign(&self.label, data).map_err(map_err)
    }
}
