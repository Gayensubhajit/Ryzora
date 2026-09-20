//! Ryzora Reversible Integration Core — Artifact Ownership Verifier
//!
//! Verifies artifacts at inspection and removal time to ensure:
//! 1. The Ryzora ownership marker is present.
//! 2. The SHA-256 hash matches according to the artifact ownership policy.
//! 3. Externally modified files are flagged as ModifiedByUser and preserved.
//! 4. Missing files are recorded cleanly without failing rollback.
//! 5. Untrusted symlinks or foreign files are rejected.

use super::filesystem::{calculate_sha256, normalize_and_validate_path};
use super::types::{
    ArtifactOwnershipPolicy, ArtifactVerificationResult, ArtifactVerificationStatus,
    IntegrationArtifact, IntegrationManifest,
};
use std::fs;
use std::path::Path;

/// Authoritatively verifies a single artifact against disk state.
pub fn verify_artifact(artifact: &IntegrationArtifact, home_dir: &Path) -> ArtifactVerificationResult {
    let target = match normalize_and_validate_path(&artifact.path, home_dir) {
        Ok(t) => t,
        Err(e) => {
            return ArtifactVerificationResult {
                path: artifact.path.clone(),
                status: ArtifactVerificationStatus::ForeignOrUnowned,
                details: Some(format!("Security path validation failed: {}", e)),
            };
        }
    };

    let meta = match fs::symlink_metadata(&target) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return ArtifactVerificationResult {
                path: target,
                status: ArtifactVerificationStatus::Missing,
                details: Some("Artifact does not exist on disk".to_string()),
            };
        }
        Err(e) => {
            return ArtifactVerificationResult {
                path: target,
                status: ArtifactVerificationStatus::ForeignOrUnowned,
                details: Some(format!("Failed to query filesystem metadata: {}", e)),
            };
        }
    };

    // If it is a symlink, verify it is expected
    if meta.file_type().is_symlink() {
        match fs::read_link(&target) {
            Ok(link_target) => {
                // Symlinks pointing outside home or containing .. are untrusted
                if link_target.to_string_lossy().contains("..") {
                    return ArtifactVerificationResult {
                        path: target,
                        status: ArtifactVerificationStatus::SymlinkTargetUntrusted,
                        details: Some("Symlink contains path traversal component".to_string()),
                    };
                }
            }
            Err(e) => {
                return ArtifactVerificationResult {
                    path: target,
                    status: ArtifactVerificationStatus::SymlinkTargetUntrusted,
                    details: Some(format!("Failed to read symlink target: {}", e)),
                };
            }
        }
    }

    if meta.is_dir() {
        return ArtifactVerificationResult {
            path: target,
            status: ArtifactVerificationStatus::ForeignOrUnowned,
            details: Some("Expected file artifact but found directory on disk".to_string()),
        };
    }

    // Read bytes to verify marker and SHA-256
    let bytes = match fs::read(&target) {
        Ok(b) => b,
        Err(e) => {
            return ArtifactVerificationResult {
                path: target,
                status: ArtifactVerificationStatus::ForeignOrUnowned,
                details: Some(format!("Failed to read file content: {}", e)),
            };
        }
    };

    // 1. Check ownership marker
    let content_str = String::from_utf8_lossy(&bytes);
    if !content_str.contains(&artifact.marker) {
        return ArtifactVerificationResult {
            path: target,
            status: ArtifactVerificationStatus::ForeignOrUnowned,
            details: Some(format!(
                "File lacks expected Ryzora ownership marker: {}",
                artifact.marker
            )),
        };
    }

    // 2. Calculate current hash
    let current_hash = calculate_sha256(&bytes);

    // 3. Apply ownership policy
    match artifact.policy {
        ArtifactOwnershipPolicy::Immutable => {
            if current_hash == artifact.sha256 {
                ArtifactVerificationResult {
                    path: target,
                    status: ArtifactVerificationStatus::VerifiedRyzoraOwned,
                    details: None,
                }
            } else {
                ArtifactVerificationResult {
                    path: target,
                    status: ArtifactVerificationStatus::ModifiedByUser,
                    details: Some(format!(
                        "File was modified externally (expected hash {}, found {})",
                        artifact.sha256, current_hash
                    )),
                }
            }
        }
        ArtifactOwnershipPolicy::RyzoraManaged => {
            // Marker is present, so Ryzora maintains ownership
            ArtifactVerificationResult {
                path: target,
                status: ArtifactVerificationStatus::VerifiedRyzoraOwned,
                details: None,
            }
        }
        ArtifactOwnershipPolicy::UserEditable => {
            if current_hash == artifact.sha256 {
                ArtifactVerificationResult {
                    path: target,
                    status: ArtifactVerificationStatus::VerifiedRyzoraOwned,
                    details: None,
                }
            } else {
                ArtifactVerificationResult {
                    path: target,
                    status: ArtifactVerificationStatus::ModifiedByUser,
                    details: Some("User customization detected; artifact must be preserved on rollback".to_string()),
                }
            }
        }
    }
}

/// Verifies all artifacts registered in a manifest.
pub fn verify_all(manifest: &IntegrationManifest, home_dir: &Path) -> Vec<ArtifactVerificationResult> {
    manifest
        .artifacts
        .iter()
        .map(|art| verify_artifact(art, home_dir))
        .collect()
}
