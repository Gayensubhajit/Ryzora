pub use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::authoring::expand_user_path;
use crate::distribution::TrustTier;
use crate::manifest::RyzoraManifest;
use crate::repository::compute_package_tree_hash;

// ─────────────────────────────────────────────────────────────────────────────
// Constants & Domain Separation
// ─────────────────────────────────────────────────────────────────────────────

/// Domain separation prefix for Ryzora v1 package signatures.
/// Prevents cross-protocol attacks and signature replay across package IDs.
pub const RYZORA_SIGNING_DOMAIN_V1: &str = "ryzora-v1";

/// Dedicated Development/Test Root Public Key used in unit testing, local development,
/// and automated CI integration tests.
/// Hex: 32 bytes (64 hex characters).
/// Fingerprint: ed25519:992aaf4748a8fd60
pub const RYZORA_DEV_TEST_ROOT_PUBKEY_HEX: &str =
    "d1a9eda16bd08fae78c922688650008b4bce235a1171044f3b68d1cb109dc0fb";

/// Fingerprint of the development/test root public key.
pub const RYZORA_DEV_TEST_ROOT_FINGERPRINT: &str = "ed25519:992aaf4748a8fd60";

/// Backward-compatible alias for the official root public key.
/// In production distribution builds, the root key is provisioned via the RYZORA_OFFICIAL_ROOT_KEY
/// compile-time environment variable or the root key ceremony (see docs/ROOT_KEY_CEREMONY.md).
pub const RYZORA_OFFICIAL_ROOT_PUBKEY_HEX: &str = RYZORA_DEV_TEST_ROOT_PUBKEY_HEX;

/// Backward-compatible alias for the official root fingerprint.
pub const RYZORA_OFFICIAL_ROOT_FINGERPRINT: &str = RYZORA_DEV_TEST_ROOT_FINGERPRINT;

/// Recognized active Ryzora Core Root Public Keys for vetting official distribution releases.
/// Designed for root key rotation: additional authorized root public keys can be appended here.
pub const RYZORA_RECOGNIZED_OFFICIAL_ROOT_KEYS: &[&str] = &[RYZORA_DEV_TEST_ROOT_PUBKEY_HEX];

// ─────────────────────────────────────────────────────────────────────────────
// Cryptographic Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Author / Signer identity recorded inside signature metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignerIdentity {
    pub author_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<String>,
}

/// Self-contained cryptographic signature metadata for a package release.
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

/// Cryptographic verification status evaluated through the Ryzora Trust Chain.
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

/// Full cryptographic evaluation report used for gating installation and display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptographicEvaluation {
    pub status: CryptographicStatus,
    pub key_id: Option<String>,
    pub public_key: Option<String>,
    pub signer_name: Option<String>,
    pub is_valid: bool,
    pub is_trusted: bool,
    pub can_install: bool,
    pub error_message: Option<String>,
}

/// Local author keypair summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorKeyPairInfo {
    pub key_id: String,
    pub public_key_hex: String,
    pub author_name: String,
    pub private_key_path: String,
    pub public_key_path: String,
}

/// Key entry in the trusted keyring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedKeyEntry {
    pub key_id: String,
    pub public_key: String,
    pub author_name: String,
    pub role: String, // "official" | "community_verified" | "user_pinned"
    pub added_at: String,
    pub notes: String,
}

/// Revoked key entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokedKeyEntry {
    pub key_id: String,
    pub public_key: String,
    pub reason: String,
    pub revoked_at: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Core Cryptographic Primitives (Pure Rust, Zero Subprocess)
// ─────────────────────────────────────────────────────────────────────────────

/// Computes the exact canonical byte payload signed for a package.
/// Canonical format: `ryzora-v1:<package_id>:<canonical_tree_hash>`
pub fn compute_signing_payload(package_id: &str, tree_hash: &str) -> Vec<u8> {
    format!(
        "{}:{}:{}",
        RYZORA_SIGNING_DOMAIN_V1,
        package_id.trim(),
        tree_hash.trim()
    )
    .into_bytes()
}

/// Computes a concise key fingerprint for an Ed25519 verifying key.
/// Format: `ed25519:<first-16-hex-chars-of-sha256(public_key)>`
pub fn compute_key_fingerprint(public_key: &VerifyingKey) -> String {
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let digest = hasher.finalize();
    let hex_full = hex::encode(digest);
    format!("ed25519:{}", &hex_full[..16])
}

/// Generates a fresh random Ed25519 keypair in pure Rust memory.
pub fn generate_ed25519_keypair() -> (SigningKey, VerifyingKey) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Signs a package's canonical tree hash with an Ed25519 signing key.
pub fn sign_package_tree_hash(
    signing_key: &SigningKey,
    package_id: &str,
    tree_hash: &str,
    identity: SignerIdentity,
) -> Result<PackageSignatureMetadata, String> {
    if package_id.trim().is_empty() {
        return Err("Package ID cannot be empty for signing".to_string());
    }
    if tree_hash.trim().is_empty() {
        return Err("Package tree hash cannot be empty for signing".to_string());
    }

    let payload = compute_signing_payload(package_id, tree_hash);
    let signature = signing_key.sign(&payload);

    let verifying_key = signing_key.verifying_key();
    let public_key_hex = hex::encode(verifying_key.as_bytes());
    let key_id = compute_key_fingerprint(&verifying_key);
    let signature_hex = hex::encode(signature.to_bytes());

    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    Ok(PackageSignatureMetadata {
        schema_version: 1,
        algorithm: "ed25519".to_string(),
        key_id,
        public_key: public_key_hex,
        signed_tree_hash: tree_hash.to_string(),
        signature: signature_hex,
        signer_identity: identity,
        signed_at: format!("{}", now_ts),
    })
}

/// Mathematically verifies an Ed25519 signature against the expected package ID and tree hash.
pub fn verify_signature_bytes(
    public_key_hex: &str,
    package_id: &str,
    expected_tree_hash: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let pubkey_bytes = hex::decode(public_key_hex.trim())
        .map_err(|e| format!("Invalid public key hex encoding: {}", e))?;

    if pubkey_bytes.len() != 32 {
        return Err(format!(
            "Invalid public key length {} bytes (expected 32 bytes)",
            pubkey_bytes.len()
        ));
    }

    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&pubkey_bytes);
    let verifying_key = VerifyingKey::from_bytes(&key_arr)
        .map_err(|e| format!("Invalid Ed25519 public key: {}", e))?;

    let sig_bytes = hex::decode(signature_hex.trim())
        .map_err(|e| format!("Invalid signature hex encoding: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err(format!(
            "Invalid signature length {} bytes (expected 64 bytes)",
            sig_bytes.len()
        ));
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    let expected_payload = compute_signing_payload(package_id, expected_tree_hash);
    verifying_key
        .verify(&expected_payload, &signature)
        .map_err(|e| format!("Ed25519 signature verification failed: {}", e))
}

// ─────────────────────────────────────────────────────────────────────────────
// Trust Store & Keyring Management
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_user_keyring_dir() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora")
}

pub fn get_trusted_keyring_file() -> PathBuf {
    get_user_keyring_dir().join("trusted_keyring.json")
}

pub fn get_revoked_keys_file() -> PathBuf {
    get_user_keyring_dir().join("revoked_keys.json")
}

pub fn get_author_keys_dir() -> PathBuf {
    crate::snapshot::get_home_dir().join(".config/ryzora/keys")
}

/// Persistent store managing trusted keys and revocation lists.
#[derive(Debug, Clone)]
pub struct TrustStore {
    trusted_keys: Vec<TrustedKeyEntry>,
    revoked_keys: Vec<RevokedKeyEntry>,
    official_core_keys: Vec<String>,
}

impl TrustStore {
    /// Loads the trust store combining official hardcoded roots, user keyring, and revocation lists.
    pub fn load_default() -> Self {
        let mut trusted_keys = Vec::new();
        let keyring_path = get_trusted_keyring_file();
        if keyring_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&keyring_path) {
                if let Ok(entries) = serde_json::from_str::<Vec<TrustedKeyEntry>>(&raw) {
                    trusted_keys = entries;
                }
            }
        }

        let mut revoked_keys = Vec::new();
        let revoked_path = get_revoked_keys_file();
        if revoked_path.is_file() {
            if let Ok(raw) = fs::read_to_string(&revoked_path) {
                if let Ok(entries) = serde_json::from_str::<Vec<RevokedKeyEntry>>(&raw) {
                    revoked_keys = entries;
                }
            }
        }

        let mut official_keys: Vec<String> = Vec::new();
        // 1. Check if compile-time production official root key was provisioned
        if let Some(prod_key) = option_env!("RYZORA_OFFICIAL_ROOT_KEY") {
            let clean = prod_key.trim();
            if !clean.is_empty() {
                official_keys.push(clean.to_lowercase());
            }
        }
        // 2. In debug / test builds, fallback to the recognized dev/test keys if no production key was provided
        #[cfg(any(debug_assertions, test))]
        if official_keys.is_empty() {
            official_keys = RYZORA_RECOGNIZED_OFFICIAL_ROOT_KEYS
                .iter()
                .map(|k| k.to_string())
                .collect();
        }

        Self {
            trusted_keys,
            revoked_keys,
            official_core_keys: official_keys,
        }
    }

    /// Creates an in-memory trust store for deterministic unit testing.
    pub fn new_test_store(
        official_keys: &[&str],
        trusted: &[TrustedKeyEntry],
        revoked: &[RevokedKeyEntry],
    ) -> Self {
        Self {
            trusted_keys: trusted.to_vec(),
            revoked_keys: revoked.to_vec(),
            official_core_keys: official_keys.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn is_official_core_key(&self, public_key_hex: &str) -> bool {
        let clean = public_key_hex.trim().to_lowercase();
        self.official_core_keys
            .iter()
            .any(|k| k.to_lowercase() == clean)
    }

    /// Returns true if an authentic production root key has been provisioned
    /// (i.e. official keys are present and not the dev/test placeholder).
    pub fn is_production_ready(&self) -> bool {
        !self.official_core_keys.is_empty()
            && !self
                .official_core_keys
                .iter()
                .any(|k| k.trim().to_lowercase() == RYZORA_DEV_TEST_ROOT_PUBKEY_HEX.to_lowercase())
    }

    /// Access the recognized official root public keys.
    pub fn official_core_keys(&self) -> &[String] {
        &self.official_core_keys
    }

    /// Appends an authorized official root key for key rotation.
    pub fn add_official_core_key(&mut self, public_key_hex: &str) {
        let clean = public_key_hex.trim().to_lowercase();
        if !clean.is_empty()
            && !self
                .official_core_keys
                .iter()
                .any(|k| k.to_lowercase() == clean)
        {
            self.official_core_keys.push(clean);
        }
    }

    pub fn is_revoked(&self, key_id: &str, public_key_hex: &str) -> bool {
        let clean_id = key_id.trim();
        let clean_pub = public_key_hex.trim().to_lowercase();
        self.revoked_keys
            .iter()
            .any(|r| r.key_id == clean_id || r.public_key.to_lowercase() == clean_pub)
    }

    pub fn is_trusted_author(&self, key_id: &str, public_key_hex: &str) -> bool {
        let clean_id = key_id.trim();
        let clean_pub = public_key_hex.trim().to_lowercase();
        self.trusted_keys.iter().any(|k| {
            (k.key_id == clean_id || k.public_key.to_lowercase() == clean_pub)
                && !self.is_revoked(&k.key_id, &k.public_key)
        })
    }

    pub fn add_trusted_key(&mut self, entry: TrustedKeyEntry) -> Result<(), String> {
        if self.is_revoked(&entry.key_id, &entry.public_key) {
            return Err("Cannot add key: this key is present in the revocation list".to_string());
        }
        if let Some(pos) = self
            .trusted_keys
            .iter()
            .position(|k| k.key_id == entry.key_id)
        {
            self.trusted_keys[pos] = entry;
        } else {
            self.trusted_keys.push(entry);
        }
        self.save_keyring()
    }

    pub fn revoke_key(
        &mut self,
        key_id: &str,
        public_key: &str,
        reason: &str,
    ) -> Result<(), String> {
        let entry = RevokedKeyEntry {
            key_id: key_id.to_string(),
            public_key: public_key.to_string(),
            reason: reason.to_string(),
            revoked_at: format!(
                "{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            ),
        };
        self.revoked_keys.push(entry);
        self.trusted_keys
            .retain(|k| k.key_id != key_id && k.public_key != public_key);
        self.save_keyring()?;
        self.save_revoked()
    }

    fn save_keyring(&self) -> Result<(), String> {
        let path = get_trusted_keyring_file();
        if let Some(p) = path.parent() {
            let _ = fs::create_dir_all(p);
        }
        let json = serde_json::to_string_pretty(&self.trusted_keys)
            .map_err(|e| format!("Failed to serialize keyring: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write keyring: {}", e))
    }

    fn save_revoked(&self) -> Result<(), String> {
        let path = get_revoked_keys_file();
        if let Some(p) = path.parent() {
            let _ = fs::create_dir_all(p);
        }
        let json = serde_json::to_string_pretty(&self.revoked_keys)
            .map_err(|e| format!("Failed to serialize revocation list: {}", e))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write revocation list: {}", e))
    }

    pub fn list_trusted_keys(&self) -> Vec<TrustedKeyEntry> {
        self.trusted_keys.clone()
    }

    pub fn list_revoked_keys(&self) -> Vec<RevokedKeyEntry> {
        self.revoked_keys.clone()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Trust Chain Evaluator (The Core Rule of Phase 12)
// ─────────────────────────────────────────────────────────────────────────────

/// Evaluates a package's cryptographic signature against Ryzora's multi-layered Trust Chain.
///
/// Invariant:
/// Signature validity alone NEVER equates to Verified or Official.
///
/// Trust Chain:
/// 1. Signature mathematically valid?
/// 2. Key not revoked? (fail-closed)
/// 3. Key in official core root? -> OfficialVerified
/// 4. Key in trusted community keyring? -> AuthorVerified
/// 5. Valid signature, unvetted key? -> SelfSignedUnvetted (remains Community)
/// 6. No signature? -> Unsigned (permitted as Community, rejected if claims Official)
pub fn evaluate_trust_chain(
    signature_opt: Option<&PackageSignatureMetadata>,
    package_id: &str,
    expected_tree_hash: &str,
    claimed_trust_tier: Option<TrustTier>,
    trust_store: &TrustStore,
) -> CryptographicEvaluation {
    let claimed = claimed_trust_tier.unwrap_or(TrustTier::Community);

    // Case 1: Unsigned Package
    let sig = match signature_opt {
        None => {
            if claimed == TrustTier::Official {
                return CryptographicEvaluation {
                    status: CryptographicStatus::OfficialImpersonation,
                    key_id: None,
                    public_key: None,
                    signer_name: None,
                    is_valid: false,
                    is_trusted: false,
                    can_install: false,
                    error_message: Some(format!(
                        "Package '{}' claims Official trust tier but has no cryptographic signature. Installation refused.",
                        package_id
                    )),
                };
            }
            return CryptographicEvaluation {
                status: CryptographicStatus::Unsigned,
                key_id: None,
                public_key: None,
                signer_name: None,
                is_valid: false,
                is_trusted: false,
                can_install: true, // Community unsigned packages remain permitted
                error_message: None,
            };
        }
        Some(s) => s,
    };

    // Case 1.5: Algorithm Check (Ed25519 only)
    if sig.algorithm.to_lowercase() != "ed25519" {
        return CryptographicEvaluation {
            status: CryptographicStatus::InvalidSignature,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: false,
            is_trusted: false,
            can_install: false,
            error_message: Some(format!(
                "Unsupported signature algorithm '{}'. Ryzora exclusively supports 'ed25519'.",
                sig.algorithm
            )),
        };
    }

    // Case 2: Tree Hash Check
    if sig.signed_tree_hash.trim() != expected_tree_hash.trim() {
        return CryptographicEvaluation {
            status: CryptographicStatus::InvalidSignature,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: false,
            is_trusted: false,
            can_install: false,
            error_message: Some(format!(
                "Signed tree hash '{}' does not match payload tree hash '{}'. Tampering detected.",
                sig.signed_tree_hash, expected_tree_hash
            )),
        };
    }

    // Case 3: Mathematical Ed25519 Verification
    if let Err(e) = verify_signature_bytes(
        &sig.public_key,
        package_id,
        expected_tree_hash,
        &sig.signature,
    ) {
        return CryptographicEvaluation {
            status: CryptographicStatus::InvalidSignature,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: false,
            is_trusted: false,
            can_install: false,
            error_message: Some(format!(
                "Mathematical Ed25519 signature check failed: {}",
                e
            )),
        };
    }

    // Case 4: Revocation Check (Fail Closed)
    if trust_store.is_revoked(&sig.key_id, &sig.public_key) {
        return CryptographicEvaluation {
            status: CryptographicStatus::RevokedKey,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: false,
            is_trusted: false,
            can_install: false,
            error_message: Some(format!(
                "Signing key '{}' has been revoked. Installation refused.",
                sig.key_id
            )),
        };
    }

    // Case 5: Official Core Root Authority
    if trust_store.is_official_core_key(&sig.public_key) {
        return CryptographicEvaluation {
            status: CryptographicStatus::OfficialVerified,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: true,
            is_trusted: true,
            can_install: true,
            error_message: None,
        };
    }

    // Check Official Impersonation: Package claims Official, but key is not Core Root
    if claimed == TrustTier::Official {
        return CryptographicEvaluation {
            status: CryptographicStatus::OfficialImpersonation,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: true,
            is_trusted: false,
            can_install: false,
            error_message: Some(format!(
                "Package '{}' claims Official status but signing key '{}' is not a Ryzora Core root key. Installation refused.",
                package_id, sig.key_id
            )),
        };
    }

    // Case 6: Vetted Community Author in Keyring
    if trust_store.is_trusted_author(&sig.key_id, &sig.public_key) {
        return CryptographicEvaluation {
            status: CryptographicStatus::AuthorVerified,
            key_id: Some(sig.key_id.clone()),
            public_key: Some(sig.public_key.clone()),
            signer_name: Some(sig.signer_identity.name.clone()),
            is_valid: true,
            is_trusted: true,
            can_install: true,
            error_message: None,
        };
    }

    // Case 7: Valid Signature with Unknown / Self-Signed Key (Community)
    CryptographicEvaluation {
        status: CryptographicStatus::SelfSignedUnvetted,
        key_id: Some(sig.key_id.clone()),
        public_key: Some(sig.public_key.clone()),
        signer_name: Some(sig.signer_identity.name.clone()),
        is_valid: true,
        is_trusted: false, // Mathematically valid, but NOT trusted as Verified author
        can_install: true, // Permitted in Community tier with unvetted status
        error_message: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Package Inspection & Signature Extraction
// ─────────────────────────────────────────────────────────────────────────────

/// Inspects a package directory on disk and attempts to extract its signature metadata.
/// Searches for:
/// 1. `release.sig` (standalone signature metadata JSON)
/// 2. Embedded `signature` object in `release.json`
pub fn load_package_signature(package_dir: &Path) -> Option<PackageSignatureMetadata> {
    let sig_file = package_dir.join("release.sig");
    if sig_file.is_file() {
        if let Ok(raw) = fs::read_to_string(&sig_file) {
            if let Ok(meta) = serde_json::from_str::<PackageSignatureMetadata>(&raw) {
                return Some(meta);
            }
        }
    }

    let release_json = package_dir.join("release.json");
    if release_json.is_file() {
        if let Ok(raw) = fs::read_to_string(&release_json) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(sig_val) = val.get("signature") {
                    if let Ok(meta) =
                        serde_json::from_value::<PackageSignatureMetadata>(sig_val.clone())
                    {
                        return Some(meta);
                    }
                }
            }
        }
    }

    None
}

/// Evaluates the complete cryptographic status of a package directory before installation.
pub fn evaluate_package_directory_crypto(
    package_dir: &Path,
    manifest: &RyzoraManifest,
    claimed_tier: Option<TrustTier>,
    trust_store: &TrustStore,
) -> Result<CryptographicEvaluation, String> {
    let tree_hash = compute_package_tree_hash(package_dir, manifest)?;
    let sig_meta = load_package_signature(package_dir);
    Ok(evaluate_trust_chain(
        sig_meta.as_ref(),
        &manifest.id,
        &tree_hash,
        claimed_tier,
        trust_store,
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Author Keypair Management
// ─────────────────────────────────────────────────────────────────────────────

/// Gets or creates a local persistent author Ed25519 keypair for signing releases.
/// Sets secure Unix file permissions (e.g. 0600 for private keys or 0700 for directories).
#[allow(unused_variables)]
pub fn set_secure_permissions(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        let permissions = fs::Permissions::from_mode(mode);
        fs::set_permissions(path, permissions).map_err(|e| {
            format!(
                "Failed to set permissions 0{:03o} on '{}': {}",
                mode,
                path.display(),
                e
            )
        })?;
    }
    Ok(())
}

/// Verifies that a private key file has strict, secure permissions (0600 on Unix).
/// Fails closed if the file is readable or writable by other users (group/world access).
pub fn verify_private_key_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let metadata = fs::metadata(path).map_err(|e| {
            format!(
                "Failed to read metadata for private key file '{}': {}",
                path.display(),
                e
            )
        })?;
        let mode = metadata.permissions().mode();
        let insecure_bits = mode & 0o077;
        if insecure_bits != 0 {
            return Err(format!(
                "Insecure permissions on private key file '{}': mode is 0{:04o} (group/world bits: 0{:03o}). Private key files must have strict 0600 permissions (user read/write only). Installation or signing refused.",
                path.display(),
                mode & 0o777,
                insecure_bits
            ));
        }
    }
    Ok(())
}

/// Writes a private key atomically to disk with strict 0600 permissions.
pub fn write_private_key_atomic(path: &Path, private_key_hex: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Invalid private key path".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("Failed to create directory '{}': {}", parent.display(), e))?;
    set_secure_permissions(parent, 0o700)?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = parent.join(format!(".author.key.tmp.{}", nanos));

    // Write temp file
    fs::write(&tmp_path, private_key_hex).map_err(|e| {
        format!(
            "Failed to write temporary private key file '{}': {}",
            tmp_path.display(),
            e
        )
    })?;

    // Enforce 0600 on temp file before rename
    set_secure_permissions(&tmp_path, 0o600)?;

    // Atomically rename into place
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "Failed to atomically rename private key to '{}': {}",
            path.display(),
            e
        )
    })?;

    // Verify final permissions
    verify_private_key_permissions(path)?;

    Ok(())
}

/// Loads an Ed25519 author signing key from disk, strictly verifying permissions.
pub fn load_author_signing_key(priv_path: &Path) -> Result<SigningKey, String> {
    if !priv_path.is_file() {
        return Err(format!(
            "Private key file '{}' does not exist",
            priv_path.display()
        ));
    }

    // Step 1: Strict permission check (fails closed on group/world access)
    verify_private_key_permissions(priv_path)?;

    // Step 2: Read and decode
    let hex_raw = fs::read_to_string(priv_path).map_err(|e| {
        format!(
            "Failed to read private key file '{}': {}",
            priv_path.display(),
            e
        )
    })?;
    let bytes = hex::decode(hex_raw.trim()).map_err(|e| {
        format!(
            "Malformed private key hex in '{}': {}",
            priv_path.display(),
            e
        )
    })?;
    if bytes.len() != 32 {
        return Err(format!(
            "Invalid private key length ({}) in '{}', expected 32 bytes",
            bytes.len(),
            priv_path.display()
        ));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(SigningKey::from_bytes(&arr))
}

/// Gets or creates a local persistent author Ed25519 keypair for signing releases.
pub fn get_or_create_author_keypair(author_name: &str) -> Result<AuthorKeyPairInfo, String> {
    let keys_dir = get_author_keys_dir();
    fs::create_dir_all(&keys_dir)
        .map_err(|e| format!("Failed to create author keys directory: {}", e))?;
    set_secure_permissions(&keys_dir, 0o700)?;

    let priv_path = keys_dir.join("author.key");
    let pub_path = keys_dir.join("author.pub");

    let signing_key = if priv_path.is_file() {
        load_author_signing_key(&priv_path)?
    } else {
        let (sign, _) = generate_ed25519_keypair();
        let priv_hex = hex::encode(sign.to_bytes());
        write_private_key_atomic(&priv_path, &priv_hex)?;
        sign
    };

    let verifying = signing_key.verifying_key();
    let pub_hex = hex::encode(verifying.as_bytes());
    let _ = fs::write(&pub_path, &pub_hex);
    let _ = set_secure_permissions(&pub_path, 0o644);

    let key_id = compute_key_fingerprint(&verifying);

    Ok(AuthorKeyPairInfo {
        key_id,
        public_key_hex: pub_hex,
        author_name: author_name.to_string(),
        private_key_path: priv_path.to_string_lossy().to_string(),
        public_key_path: pub_path.to_string_lossy().to_string(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_author_keypair(author_name: Option<String>) -> Result<AuthorKeyPairInfo, String> {
    let name = author_name.unwrap_or_else(|| "Local Author".to_string());
    get_or_create_author_keypair(&name)
}

#[tauri::command]
pub fn list_trusted_keys() -> Result<Vec<TrustedKeyEntry>, String> {
    let store = TrustStore::load_default();
    Ok(store.list_trusted_keys())
}

#[tauri::command]
pub fn list_revoked_keys() -> Result<Vec<RevokedKeyEntry>, String> {
    let store = TrustStore::load_default();
    Ok(store.list_revoked_keys())
}

#[tauri::command]
pub fn add_trusted_key(
    key_id: String,
    public_key: String,
    author_name: String,
    notes: Option<String>,
) -> Result<(), String> {
    let mut store = TrustStore::load_default();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = TrustedKeyEntry {
        key_id,
        public_key,
        author_name,
        role: "user_pinned".to_string(),
        added_at: format!("{}", now),
        notes: notes.unwrap_or_default(),
    };
    store.add_trusted_key(entry)
}

#[tauri::command]
pub fn revoke_trusted_key(
    key_id: String,
    public_key: String,
    reason: String,
) -> Result<(), String> {
    let mut store = TrustStore::load_default();
    store.revoke_key(&key_id, &public_key, &reason)
}

#[tauri::command]
pub fn verify_package_cryptography(package_dir: String) -> Result<CryptographicEvaluation, String> {
    let resolved = expand_user_path(&package_dir);
    let manifest = crate::installer::load_package_manifest(&resolved)?;
    let store = TrustStore::load_default();
    evaluate_package_directory_crypto(&resolved, &manifest, None, &store)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests & Canonicalization Attack Surface Verification
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::manifest::{ManifestCompatibility, ManifestFile, PackageType, RyzoraManifest};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestCryptoSandbox {
        root: PathBuf,
    }

    impl TestCryptoSandbox {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ryzora-crypto-test-{}-{}", label, nanos));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn create_test_package(
            &self,
            id: &str,
            files: &[(&str, &str)],
        ) -> (PathBuf, RyzoraManifest) {
            let pkg_dir = self.root.join(id);
            fs::create_dir_all(&pkg_dir).unwrap();

            let mut manifest_files = Vec::new();
            for (rel_source, content) in files {
                let file_path = pkg_dir.join(rel_source);
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(&file_path, content).unwrap();

                manifest_files.push(ManifestFile {
                    source: rel_source.to_string(),
                    target: format!(
                        "~/.config/{}",
                        rel_source.strip_prefix("files/").unwrap_or(rel_source)
                    ),
                    description: "Test config".to_string(),
                });
            }

            let manifest = RyzoraManifest {
                id: id.to_string(),
                name: "Crypto Test Package".to_string(),
                version: "1.0.0".to_string(),
                ryzora_spec: "1".to_string(),
                author: "TestSigner".to_string(),
                package_type: PackageType::Theme,
                description: "Test theme for cryptographic trust verification".to_string(),
                tags: vec!["crypto".to_string(), "test".to_string()],
                color_palette: vec!["#1e1e2e".to_string(), "#cba6f7".to_string()],
                compatibility: ManifestCompatibility {
                    desktops: vec!["hyprland".to_string()],
                    sessions: vec!["wayland".to_string()],
                    distros: vec![],
                    required: vec![],
                    optional: vec![],
                },
                dependencies: vec![],
                files: manifest_files,
            };

            fs::write(
                pkg_dir.join("ryzora.json"),
                serde_json::to_string_pretty(&manifest).unwrap(),
            )
            .unwrap();

            (pkg_dir, manifest)
        }
    }

    impl Drop for TestCryptoSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn test_crypto_keypair_generation_and_fingerprint() {
        let (_sign, verify) = generate_ed25519_keypair();
        let fp = compute_key_fingerprint(&verify);
        assert!(fp.starts_with("ed25519:"));
        assert_eq!(fp.len(), 24); // "ed25519:" (8 chars) + 16 hex chars

        // Deterministic fingerprint for same public key
        let fp2 = compute_key_fingerprint(&verify);
        assert_eq!(fp, fp2);
    }

    #[test]
    fn test_crypto_canonical_tree_hash_determinism() {
        let sandbox = TestCryptoSandbox::new("tree-hash-det");
        let (pkg_dir1, m1) = sandbox.create_test_package(
            "det-pkg-1",
            &[
                ("files/style.css", "body { color: red; }"),
                ("files/bar.conf", "height = 30"),
            ],
        );
        let (pkg_dir2, m2) = sandbox.create_test_package(
            "det-pkg-2",
            &[
                ("files/style.css", "body { color: red; }"),
                ("files/bar.conf", "height = 30"),
            ],
        );

        let h1 = compute_package_tree_hash(&pkg_dir1, &m1).unwrap();
        let h2 = compute_package_tree_hash(&pkg_dir2, &m2).unwrap();
        assert_eq!(
            h1, h2,
            "Identical files must yield identical canonical tree hash"
        );
    }

    #[test]
    fn test_crypto_changed_file_content_changes_tree_hash() {
        let sandbox = TestCryptoSandbox::new("changed-content");
        let (pkg_dir1, m1) =
            sandbox.create_test_package("pkg-1", &[("files/style.css", "body { color: red; }")]);
        let (pkg_dir2, m2) =
            sandbox.create_test_package("pkg-2", &[("files/style.css", "body { color: blue; }")]);

        let h1 = compute_package_tree_hash(&pkg_dir1, &m1).unwrap();
        let h2 = compute_package_tree_hash(&pkg_dir2, &m2).unwrap();
        assert_ne!(h1, h2, "Changed file content must alter tree hash");
    }

    #[test]
    fn test_crypto_changed_filename_changes_tree_hash() {
        let sandbox = TestCryptoSandbox::new("changed-filename");
        let (pkg_dir1, m1) =
            sandbox.create_test_package("pkg-1", &[("files/style.css", "body { margin: 0; }")]);
        let (pkg_dir2, m2) =
            sandbox.create_test_package("pkg-2", &[("files/other.css", "body { margin: 0; }")]);

        let h1 = compute_package_tree_hash(&pkg_dir1, &m1).unwrap();
        let h2 = compute_package_tree_hash(&pkg_dir2, &m2).unwrap();
        assert_ne!(h1, h2, "Changed relative source path must alter tree hash");
    }

    #[test]
    fn test_crypto_added_file_changes_tree_hash() {
        let sandbox = TestCryptoSandbox::new("added-file");
        let (pkg_dir1, m1) =
            sandbox.create_test_package("pkg-1", &[("files/style.css", "body {}")]);
        let (pkg_dir2, m2) = sandbox.create_test_package(
            "pkg-2",
            &[("files/style.css", "body {}"), ("files/extra.css", "a {}")],
        );

        let h1 = compute_package_tree_hash(&pkg_dir1, &m1).unwrap();
        let h2 = compute_package_tree_hash(&pkg_dir2, &m2).unwrap();
        assert_ne!(h1, h2, "Adding a file must alter tree hash");
    }

    #[test]
    fn test_crypto_deleted_file_fails_tree_hash() {
        let sandbox = TestCryptoSandbox::new("deleted-file");
        let (pkg_dir, m) =
            sandbox.create_test_package("pkg-del", &[("files/style.css", "body {}")]);

        // Delete payload file from disk
        fs::remove_file(pkg_dir.join("files/style.css")).unwrap();

        let res = compute_package_tree_hash(&pkg_dir, &m);
        assert!(
            res.is_err(),
            "Missing payload file declared in manifest must fail tree hash"
        );
    }

    #[test]
    fn test_crypto_cross_package_signature_replay_prevented_by_domain_separation() {
        let sandbox = TestCryptoSandbox::new("replay-prevention");
        let (pkg_dir1, m1) = sandbox
            .create_test_package("pkg-alpha", &[("files/theme.css", "/* identical bytes */")]);
        let (_pkg_dir2, _m2) = sandbox
            .create_test_package("pkg-beta", &[("files/theme.css", "/* identical bytes */")]);

        let tree_hash = compute_package_tree_hash(&pkg_dir1, &m1).unwrap();
        let (sign, _) = generate_ed25519_keypair();
        let identity = SignerIdentity {
            author_id: "pkg-alpha".to_string(),
            name: "Author".to_string(),
            handle: None,
        };

        // Sign for pkg-alpha
        let sig_meta = sign_package_tree_hash(&sign, "pkg-alpha", &tree_hash, identity).unwrap();

        // Attempt to verify the exact same signature and tree hash for pkg-beta -> Must fail!
        let replay_check = verify_signature_bytes(
            &sig_meta.public_key,
            "pkg-beta",
            &tree_hash,
            &sig_meta.signature,
        );
        assert!(
            replay_check.is_err(),
            "Signature created for pkg-alpha must NOT be valid for pkg-beta (domain separation)"
        );
    }

    #[test]
    fn test_crypto_signature_with_wrong_public_key_rejected() {
        let (sign1, _) = generate_ed25519_keypair();
        let (_, verify2) = generate_ed25519_keypair();

        let identity = SignerIdentity {
            author_id: "test-pkg".to_string(),
            name: "Author".to_string(),
            handle: None,
        };
        let sig = sign_package_tree_hash(&sign1, "test-pkg", "hash123", identity).unwrap();

        let wrong_pubkey_hex = hex::encode(verify2.as_bytes());
        let res = verify_signature_bytes(&wrong_pubkey_hex, "test-pkg", "hash123", &sig.signature);
        assert!(
            res.is_err(),
            "Verification with mismatched public key must fail"
        );
    }

    #[test]
    fn test_crypto_corrupted_signature_bytes_rejected() {
        let (sign, verify) = generate_ed25519_keypair();
        let identity = SignerIdentity {
            author_id: "test-pkg".to_string(),
            name: "Author".to_string(),
            handle: None,
        };
        let sig = sign_package_tree_hash(&sign, "test-pkg", "hash123", identity).unwrap();

        // Mutate signature hex
        let mut chars: Vec<char> = sig.signature.chars().collect();
        chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
        let corrupted_sig: String = chars.into_iter().collect();

        let pub_hex = hex::encode(verify.as_bytes());
        let res = verify_signature_bytes(&pub_hex, "test-pkg", "hash123", &corrupted_sig);
        assert!(
            res.is_err(),
            "Corrupted signature bytes must fail verification"
        );
    }

    #[test]
    fn test_crypto_trust_chain_self_signed_unvetted_remains_community() {
        // Crucial invariant: Valid signature with unknown key does NOT grant Verified tier!
        let (sign, _verify) = generate_ed25519_keypair();
        let identity = SignerIdentity {
            author_id: "self-pkg".to_string(),
            name: "Unknown Creator".to_string(),
            handle: None,
        };
        let sig = sign_package_tree_hash(&sign, "self-pkg", "treehash-abc", identity).unwrap();

        let trust_store = TrustStore::new_test_store(&[], &[], &[]);
        let eval = evaluate_trust_chain(
            Some(&sig),
            "self-pkg",
            "treehash-abc",
            Some(TrustTier::Community),
            &trust_store,
        );

        assert!(eval.is_valid, "Signature is mathematically valid");
        assert!(!eval.is_trusted, "Unvetted key must NOT be marked trusted");
        assert_eq!(eval.status, CryptographicStatus::SelfSignedUnvetted);
        assert!(
            eval.can_install,
            "Community unvetted packages can be installed"
        );
    }

    #[test]
    fn test_crypto_trust_chain_vetted_author_grants_verified_tier() {
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());
        let key_id = compute_key_fingerprint(&verify);

        let identity = SignerIdentity {
            author_id: "verified-pkg".to_string(),
            name: "Trusted Creator".to_string(),
            handle: Some("@trusted".to_string()),
        };
        let sig =
            sign_package_tree_hash(&sign, "verified-pkg", "treehash-vetted", identity).unwrap();

        let trusted_entry = TrustedKeyEntry {
            key_id: key_id.clone(),
            public_key: pub_hex.clone(),
            author_name: "Trusted Creator".to_string(),
            role: "community_verified".to_string(),
            added_at: "1000".to_string(),
            notes: "Vetted via PR #42".to_string(),
        };

        let trust_store = TrustStore::new_test_store(&[], &[trusted_entry], &[]);
        let eval = evaluate_trust_chain(
            Some(&sig),
            "verified-pkg",
            "treehash-vetted",
            Some(TrustTier::Community),
            &trust_store,
        );

        assert!(eval.is_valid);
        assert!(eval.is_trusted);
        assert_eq!(eval.status, CryptographicStatus::AuthorVerified);
        assert!(eval.can_install);
    }

    #[test]
    fn test_crypto_trust_chain_official_core_key_grants_official_tier() {
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());

        let identity = SignerIdentity {
            author_id: "core-pkg".to_string(),
            name: "Ryzora Core Team".to_string(),
            handle: None,
        };
        let sig = sign_package_tree_hash(&sign, "core-pkg", "treehash-official", identity).unwrap();

        // Register public key as Official Core Root in store
        let trust_store = TrustStore::new_test_store(&[&pub_hex], &[], &[]);
        let eval = evaluate_trust_chain(
            Some(&sig),
            "core-pkg",
            "treehash-official",
            Some(TrustTier::Official),
            &trust_store,
        );

        assert!(eval.is_valid);
        assert!(eval.is_trusted);
        assert_eq!(eval.status, CryptographicStatus::OfficialVerified);
        assert!(eval.can_install);
    }

    #[test]
    fn test_crypto_trust_chain_revocation_fails_closed() {
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());
        let key_id = compute_key_fingerprint(&verify);

        let identity = SignerIdentity {
            author_id: "compromised-pkg".to_string(),
            name: "Compromised Key".to_string(),
            handle: None,
        };
        let sig =
            sign_package_tree_hash(&sign, "compromised-pkg", "treehash-revoked", identity).unwrap();

        let revoked_entry = RevokedKeyEntry {
            key_id: key_id.clone(),
            public_key: pub_hex.clone(),
            reason: "Private key leaked publicly".to_string(),
            revoked_at: "2000".to_string(),
        };

        let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
        let eval = evaluate_trust_chain(
            Some(&sig),
            "compromised-pkg",
            "treehash-revoked",
            Some(TrustTier::Community),
            &trust_store,
        );

        assert!(!eval.is_trusted);
        assert!(
            !eval.can_install,
            "Revoked key must be BLOCKED from installation (fail closed)"
        );
        assert_eq!(eval.status, CryptographicStatus::RevokedKey);
        assert!(eval.error_message.unwrap().contains("revoked"));
    }

    #[test]
    fn test_crypto_trust_chain_official_impersonation_blocked() {
        let (sign, _) = generate_ed25519_keypair();
        let identity = SignerIdentity {
            author_id: "imposter-pkg".to_string(),
            name: "Malicious Actor".to_string(),
            handle: None,
        };
        let sig =
            sign_package_tree_hash(&sign, "imposter-pkg", "treehash-imposter", identity).unwrap();

        // Trust store has an official core key, but NOT the signer's key
        let trust_store = TrustStore::new_test_store(
            &["deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"],
            &[],
            &[],
        );

        // Package claims Official trust tier
        let eval = evaluate_trust_chain(
            Some(&sig),
            "imposter-pkg",
            "treehash-imposter",
            Some(TrustTier::Official),
            &trust_store,
        );

        assert!(
            !eval.can_install,
            "Official impersonation must be BLOCKED from installation"
        );
        assert_eq!(eval.status, CryptographicStatus::OfficialImpersonation);
    }

    #[test]
    fn test_crypto_trust_chain_unsigned_community_package_permitted() {
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);
        let eval = evaluate_trust_chain(
            None,
            "legacy-community-pkg",
            "treehash-unsigned",
            Some(TrustTier::Community),
            &trust_store,
        );

        assert_eq!(eval.status, CryptographicStatus::Unsigned);
        assert!(
            eval.can_install,
            "Existing unsigned community packages remain installable"
        );
        assert!(!eval.is_trusted);
    }

    #[test]
    fn test_crypto_trust_chain_unsigned_claiming_official_blocked() {
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);
        let eval = evaluate_trust_chain(
            None,
            "fake-official-pkg",
            "treehash-unsigned",
            Some(TrustTier::Official),
            &trust_store,
        );

        assert!(
            !eval.can_install,
            "Unsigned package claiming Official tier must be BLOCKED"
        );
        assert_eq!(eval.status, CryptographicStatus::OfficialImpersonation);
    }

    #[test]
    fn test_crypto_zero_command_execution() {
        let src_dir = std::path::Path::new("src");
        let forbidden = [
            "std::process::Command",
            "process::Command",
            "Command::new",
            "\"sh\"",
            "\"bash\"",
            "\"sudo\"",
            "\"pkexec\"",
            "\"pacman\"",
            "\"yay\"",
            "\"gpg\"",
            "\"openssl\"",
        ];

        let files = [
            "crypto.rs",
            "distribution.rs",
            "authoring.rs",
            "repository.rs",
            "installer.rs",
            "manifest.rs",
            "snapshot.rs",
            "lib.rs",
        ];

        for file_name in &files {
            let file_path = src_dir.join(file_name);
            if file_path.is_file() {
                let code = fs::read_to_string(&file_path).unwrap();
                let prod_code = code.split("#[cfg(test)]").next().unwrap_or(&code);
                for pat in &forbidden {
                    assert!(
                        !prod_code.contains(pat),
                        "File '{}' contains forbidden execution call '{}'",
                        file_name,
                        pat
                    );
                }
            }
        }
    }

    #[test]
    fn test_crypto_official_root_key_authenticity_and_fingerprint() {
        let bytes = hex::decode(RYZORA_OFFICIAL_ROOT_PUBKEY_HEX).expect("Must be valid hex");
        assert_eq!(
            bytes.len(),
            32,
            "Ed25519 root public key must be exactly 32 bytes"
        );
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        let verifying_key =
            VerifyingKey::from_bytes(&arr).expect("Must be authentic Ed25519 public key");
        let computed_fp = compute_key_fingerprint(&verifying_key);
        assert_eq!(computed_fp, RYZORA_OFFICIAL_ROOT_FINGERPRINT);
        assert_eq!(computed_fp, "ed25519:992aaf4748a8fd60");

        // Must be recognized by default TrustStore
        let store = TrustStore::load_default();
        assert!(store.is_official_core_key(RYZORA_OFFICIAL_ROOT_PUBKEY_HEX));
        assert!(!store.is_official_core_key(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn test_crypto_production_root_unprovisioned_safety() {
        // When unprovisioned (empty official keys), an absent root CANNOT validate arbitrary packages as official
        let unprovisioned_store = TrustStore::new_test_store(&[], &[], &[]);
        assert!(!unprovisioned_store.is_production_ready());
        assert_eq!(unprovisioned_store.official_core_keys().len(), 0);
        assert!(!unprovisioned_store.is_official_core_key(RYZORA_DEV_TEST_ROOT_PUBKEY_HEX));
        assert!(!unprovisioned_store.is_official_core_key(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
    }

    #[test]
    fn test_crypto_production_root_provisioning_and_rotation() {
        let (_signing1, verifying1) = generate_ed25519_keypair();
        let pubkey1_hex = hex::encode(verifying1.as_bytes());

        let (_signing2, verifying2) = generate_ed25519_keypair();
        let pubkey2_hex = hex::encode(verifying2.as_bytes());

        // Provisioned with single root key
        let mut store = TrustStore::new_test_store(&[&pubkey1_hex], &[], &[]);
        assert!(store.is_production_ready());
        assert!(store.is_official_core_key(&pubkey1_hex));
        assert!(!store.is_official_core_key(&pubkey2_hex));

        // Rotate root key: append authorized key2
        store.add_official_core_key(&pubkey2_hex);
        assert_eq!(store.official_core_keys().len(), 2);
        assert!(store.is_official_core_key(&pubkey1_hex));
        assert!(store.is_official_core_key(&pubkey2_hex));
    }

    #[test]
    fn test_crypto_author_private_key_permissions_hardening_0600() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ryzora-perm-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let priv_path = temp_dir.join("author.key");

        let (sign, _) = generate_ed25519_keypair();
        let priv_hex = hex::encode(sign.to_bytes());

        // Write atomically with 0600
        write_private_key_atomic(&priv_path, &priv_hex).unwrap();

        #[cfg(unix)]
        {
            let mode = fs::metadata(&priv_path).unwrap().permissions().mode();
            assert_eq!(
                mode & 0o777,
                0o600,
                "Private key file must have strict 0600 permissions"
            );
            let parent_mode = fs::metadata(&temp_dir).unwrap().permissions().mode();
            assert_eq!(
                parent_mode & 0o777,
                0o700,
                "Parent key directory must have strict 0700 permissions"
            );
        }

        // Loading key must succeed
        let loaded = load_author_signing_key(&priv_path).unwrap();
        assert_eq!(loaded.to_bytes(), sign.to_bytes());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    #[cfg(unix)]
    fn test_crypto_author_private_key_rejects_insecure_permissions() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ryzora-insecure-perm-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        let priv_path = temp_dir.join("author.key");

        let (sign, _) = generate_ed25519_keypair();
        let priv_hex = hex::encode(sign.to_bytes());
        fs::write(&priv_path, &priv_hex).unwrap();

        // Test 1: group/world readable (0644)
        set_secure_permissions(&priv_path, 0o644).unwrap();
        let err644 = load_author_signing_key(&priv_path).unwrap_err();
        assert!(err644.contains("Insecure permissions"));
        assert!(err644.contains("0600"));

        // Test 2: world writable (0666)
        set_secure_permissions(&priv_path, 0o666).unwrap();
        let err666 = load_author_signing_key(&priv_path).unwrap_err();
        assert!(err666.contains("Insecure permissions"));

        // Test 3: group readable only (0640)
        set_secure_permissions(&priv_path, 0o640).unwrap();
        let err640 = load_author_signing_key(&priv_path).unwrap_err();
        assert!(err640.contains("Insecure permissions"));

        // Test 4: fixed to 0600 -> succeeds
        set_secure_permissions(&priv_path, 0o600).unwrap();
        assert!(load_author_signing_key(&priv_path).is_ok());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_crypto_signature_algorithm_enforcement_ed25519_only() {
        let (sign, _) = generate_ed25519_keypair();
        let mut sig = sign_package_tree_hash(
            &sign,
            "algo-pkg",
            "treehash-algo",
            SignerIdentity {
                author_id: "algo-pkg".to_string(),
                name: "Algo Tester".to_string(),
                handle: None,
            },
        )
        .unwrap();

        let store = TrustStore::new_test_store(&[], &[], &[]);

        // Valid with ed25519
        assert_eq!(sig.algorithm, "ed25519");
        assert!(
            evaluate_trust_chain(Some(&sig), "algo-pkg", "treehash-algo", None, &store).is_valid
        );

        // Corrupted / forbidden algorithms must fail closed
        sig.algorithm = "rsa".to_string();
        let eval_rsa = evaluate_trust_chain(Some(&sig), "algo-pkg", "treehash-algo", None, &store);
        assert!(!eval_rsa.is_valid);
        assert_eq!(eval_rsa.status, CryptographicStatus::InvalidSignature);
        assert!(eval_rsa
            .error_message
            .unwrap()
            .contains("Unsupported signature algorithm"));

        sig.algorithm = "ecdsa".to_string();
        let eval_ecdsa =
            evaluate_trust_chain(Some(&sig), "algo-pkg", "treehash-algo", None, &store);
        assert!(!eval_ecdsa.is_valid);
        assert_eq!(eval_ecdsa.status, CryptographicStatus::InvalidSignature);
    }

    #[test]
    fn test_crypto_signature_metadata_tampering_rejected() {
        let (sign, _) = generate_ed25519_keypair();
        let orig_sig = sign_package_tree_hash(
            &sign,
            "tamper-meta-pkg",
            "treehash-original",
            SignerIdentity {
                author_id: "tamper-meta-pkg".to_string(),
                name: "Tamper Tester".to_string(),
                handle: None,
            },
        )
        .unwrap();

        let store = TrustStore::new_test_store(&[], &[], &[]);

        // Tamper 1: signed_tree_hash mismatch
        let mut bad_tree = orig_sig.clone();
        bad_tree.signed_tree_hash = "treehash-tampered".to_string();
        let eval1 = evaluate_trust_chain(
            Some(&bad_tree),
            "tamper-meta-pkg",
            "treehash-original",
            None,
            &store,
        );
        assert!(!eval1.is_valid);
        assert_eq!(eval1.status, CryptographicStatus::InvalidSignature);
        assert!(eval1.error_message.unwrap().contains("Tampering detected"));

        // Tamper 2: signature bytes corrupted
        let mut bad_sig_bytes = orig_sig.clone();
        bad_sig_bytes.signature = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        let eval2 = evaluate_trust_chain(
            Some(&bad_sig_bytes),
            "tamper-meta-pkg",
            "treehash-original",
            None,
            &store,
        );
        assert!(!eval2.is_valid);
        assert_eq!(eval2.status, CryptographicStatus::InvalidSignature);
        assert!(eval2
            .error_message
            .unwrap()
            .contains("Mathematical Ed25519 signature check failed"));

        // Tamper 3: public key substituted
        let (_, other_verify) = generate_ed25519_keypair();
        let mut bad_pubkey = orig_sig.clone();
        bad_pubkey.public_key = hex::encode(other_verify.as_bytes());
        let eval3 = evaluate_trust_chain(
            Some(&bad_pubkey),
            "tamper-meta-pkg",
            "treehash-original",
            None,
            &store,
        );
        assert!(!eval3.is_valid);
        assert_eq!(eval3.status, CryptographicStatus::InvalidSignature);
    }
}
