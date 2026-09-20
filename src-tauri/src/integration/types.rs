//! Ryzora Reversible Integration Core — Data Models & Types
//!
//! Defines the contracts for integration manifests, ownership policies,
//! artifact verification, and safe rollback reports.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// The functional role of an integration artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Systemd user service drop-in (e.g. ~/.config/systemd/user/*.service.d/*.conf)
    SystemdDropin,
    /// Binary or script dispatcher shim (e.g. ~/.local/bin/*)
    WrapperShim,
    /// Application or daemon configuration overlay (e.g. ~/.config/ryzora/*.conf)
    ConfigOverlay,
    /// Environment variable declaration file (e.g. ~/.config/environment.d/*.conf)
    EnvironmentFile,
    /// Standalone script hook
    SessionScript,
}

/// The ownership and mutability policy governing an artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOwnershipPolicy {
    /// Both the Ryzora ownership marker and SHA-256 content hash must match.
    /// Any external modification flags a conflict and prevents deletion.
    Immutable,
    /// Ryzora owns the content format; the ownership marker must be present,
    /// but content hash changes across Ryzora versions are handled by the engine.
    RyzoraManaged,
    /// Ryzora created the initial file, but user customization is expected.
    /// On disable/rollback, user modifications are strictly preserved.
    UserEditable,
}

/// A specific file or link created as part of a Ryzora integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationArtifact {
    /// Canonical path to the artifact on disk.
    pub path: PathBuf,
    /// The structural category of the artifact.
    pub artifact_type: ArtifactType,
    /// The verification policy determining deletion safety.
    pub policy: ArtifactOwnershipPolicy,
    /// SHA-256 hash of the content at creation time.
    pub sha256: String,
    /// Unique ownership marker string embedded in the file.
    pub marker: String,
    /// Unix timestamp of creation.
    pub created_at: u64,
    /// Unix file mode permissions (e.g. 0o755 for shims, 0o644 for configs).
    pub permissions: Option<u32>,
}

/// Manifest tracking all artifacts and directories owned by a specific integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationManifest {
    /// High-level feature domain (e.g. "session-lock", "idle-management", "waybar").
    pub feature_id: String,
    /// Unique identifier for this specific integration implementation.
    pub integration_id: String,
    /// Schema version for forward/backward compatibility.
    pub version: u32,
    /// Whether this integration is currently active.
    pub enabled: bool,
    /// All individual file artifacts owned by this integration.
    pub artifacts: Vec<IntegrationArtifact>,
    /// Directories explicitly created by Ryzora when applying this integration.
    /// Only directories in this list may be considered for empty cleanup on disable.
    pub created_directories: Vec<PathBuf>,
    /// Arbitrary metadata key-values (e.g. detected compositor, target service).
    pub metadata: HashMap<String, String>,
    /// Unix timestamp of last update.
    pub updated_at: u64,
}

/// The verification verdict for an artifact at inspection or removal time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactVerificationStatus {
    /// Artifact exists, contains the valid marker, and matches the recorded hash.
    /// Verified Ryzora-owned and safe to remove.
    VerifiedRyzoraOwned,
    /// Artifact was modified externally by the user. Must be preserved.
    ModifiedByUser,
    /// Artifact does not exist on disk (already removed or missing).
    Missing,
    /// File exists but lacks the required Ryzora ownership marker.
    ForeignOrUnowned,
    /// File is a symlink pointing to an unexpected or untrusted location.
    SymlinkTargetUntrusted,
}

/// Result of verifying an individual artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactVerificationResult {
    pub path: PathBuf,
    pub status: ArtifactVerificationStatus,
    pub details: Option<String>,
}

/// Comprehensive report generated when disabling/reverting an integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisableReport {
    pub integration_id: String,
    /// Artifacts positively verified and successfully deleted.
    pub removed_artifacts: Vec<PathBuf>,
    /// Artifacts preserved because they were modified by the user or foreign.
    pub preserved_artifacts: Vec<PathBuf>,
    /// Artifacts that were already missing on disk.
    pub missing_artifacts: Vec<PathBuf>,
    /// Directories created by Ryzora that were empty and successfully cleaned up.
    pub removed_directories: Vec<PathBuf>,
    /// Directories that were not removed (non-empty or not created by Ryzora).
    pub preserved_directories: Vec<PathBuf>,
    /// Any non-fatal errors or warnings encountered during rollback.
    pub errors: Vec<String>,
    /// True if all owned artifacts were cleanly removed without conflicts.
    pub fully_reverted: bool,
}

/// Errors produced by the reversible integration subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntegrationError {
    IoError(String),
    SecurityViolation(String),
    ManifestNotFound(String),
    ManifestCorrupted(String),
    ConflictError(String),
    AlreadyActive(String),
    AlreadyDisabled(String),
    VerificationFailed(String),
    RollbackFailed(String),
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IntegrationError::IoError(msg) => write!(f, "Integration I/O Error: {}", msg),
            IntegrationError::SecurityViolation(msg) => write!(f, "Integration Security Violation: {}", msg),
            IntegrationError::ManifestNotFound(id) => write!(f, "Integration manifest not found: {}", id),
            IntegrationError::ManifestCorrupted(msg) => write!(f, "Integration manifest corrupted: {}", msg),
            IntegrationError::ConflictError(msg) => write!(f, "Integration conflict: {}", msg),
            IntegrationError::AlreadyActive(id) => write!(f, "Integration already active: {}", id),
            IntegrationError::AlreadyDisabled(id) => write!(f, "Integration already disabled: {}", id),
            IntegrationError::VerificationFailed(msg) => write!(f, "Integration verification failed: {}", msg),
            IntegrationError::RollbackFailed(msg) => write!(f, "Integration rollback failed: {}", msg),
        }
    }
}

impl std::error::Error for IntegrationError {}
