//! Ryzora cryptographic signature and verification data models.

use serde::{Deserialize, Serialize};

/// Author / Signer identity recorded inside signature metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignerIdentity {
    pub author_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

/// Ryzora v1 cryptographic package signature container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSignatureMetadata {
    pub schema_version: u32,
    pub algorithm: String,
    pub key_id: String,
    pub public_key: String,
    pub signed_tree_hash: String,
    pub signature: String,
    pub signer_identity: SignerIdentity,
    pub signed_at: String,
}

/// Cryptographic verification status evaluated for catalog & installation gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptographicStatus {
    /// Mathematically verified with Ryzora Core Root key -> Grants Official tier.
    OfficialVerified,
    /// Mathematically verified with key in Trusted Community Keyring -> Grants Verified tier.
    AuthorVerified,
    /// Mathematically valid Ed25519 signature, but key is unvetted -> Remains Community.
    SelfSignedUnvetted,
    /// Package has no signature metadata -> Remains Community or Untrusted.
    Unsigned,
    /// Mathematical signature check failed (corrupted or tampered payload) -> BLOCKED.
    InvalidSignature,
    /// Signing key is listed on Key Revocation List (fail closed) -> BLOCKED.
    RevokedKey,
    /// Package claims Official tier without Ryzora Core Root signature -> BLOCKED.
    OfficialImpersonation,
}

impl Default for CryptographicStatus {
    fn default() -> Self {
        CryptographicStatus::Unsigned
    }
}
