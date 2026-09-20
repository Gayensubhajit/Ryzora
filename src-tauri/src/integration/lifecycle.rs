//! Ryzora Reversible Integration Core — Lifecycle Orchestration Engine
//!
//! Manages the plan -> apply -> verify -> disable lifecycle with strict safety invariants:
//! 1. Never overwrite pre-existing unowned files during apply.
//! 2. Authoritatively verify artifact ownership at removal time.
//! 3. Strictly preserve files modified by the user externally.
//! 4. Handle missing artifacts idempotently.
//! 5. Clean up only empty directories that Ryzora explicitly created.

use super::filesystem::{
    atomic_write_artifact, is_approved_integration_root, normalize_and_validate_path,
    safe_remove_artifact_file, safe_remove_created_directory,
};
use super::manifest::{delete_manifest, load_manifest, save_manifest};
use super::types::{
    ArtifactOwnershipPolicy, ArtifactType, ArtifactVerificationResult,
    ArtifactVerificationStatus, DisableReport, IntegrationArtifact, IntegrationError,
    IntegrationManifest,
};
use super::verifier::{verify_all, verify_artifact};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// An artifact specification ready to be planned and applied.
#[derive(Debug, Clone)]
pub struct PlannedArtifact {
    pub path: PathBuf,
    pub content: String,
    pub artifact_type: ArtifactType,
    pub policy: ArtifactOwnershipPolicy,
    pub marker: String,
    pub permissions: Option<u32>,
}

/// A validated integration plan ready for atomic application.
#[derive(Debug, Clone)]
pub struct PlannedIntegration {
    pub feature_id: String,
    pub integration_id: String,
    pub version: u32,
    pub artifacts: Vec<PlannedArtifact>,
    pub metadata: HashMap<String, String>,
}

/// Validates that an integration plan will not overwrite user-owned files or violate path security.
pub fn plan_integration(
    feature_id: &str,
    integration_id: &str,
    version: u32,
    artifacts: Vec<PlannedArtifact>,
    metadata: HashMap<String, String>,
    home_dir: &Path,
) -> Result<PlannedIntegration, IntegrationError> {
    if feature_id.trim().is_empty() {
        return Err(IntegrationError::ConflictError("feature_id cannot be empty".to_string()));
    }
    if integration_id.trim().is_empty() {
        return Err(IntegrationError::ConflictError("integration_id cannot be empty".to_string()));
    }

    // Check if integration is already recorded as active
    if let Ok(existing) = load_manifest(integration_id, home_dir) {
        if existing.enabled {
            return Err(IntegrationError::AlreadyActive(format!(
                "Integration '{}' is already active on this host",
                integration_id
            )));
        }
    }

    for art in &artifacts {
        let norm_path = normalize_and_validate_path(&art.path, home_dir)?;

        if !is_approved_integration_root(&norm_path, home_dir)? {
            return Err(IntegrationError::SecurityViolation(format!(
                "Artifact path '{}' is outside approved integration roots",
                norm_path.display()
            )));
        }

        // Reject if target already exists on disk
        if norm_path.exists() {
            // Check if this existing file was already verified as Ryzora-owned
            let probe = IntegrationArtifact {
                path: norm_path.clone(),
                artifact_type: art.artifact_type,
                policy: art.policy,
                sha256: String::new(),
                marker: art.marker.clone(),
                created_at: 0,
                permissions: art.permissions,
            };
            let status = verify_artifact(&probe, home_dir);
            if status.status != ArtifactVerificationStatus::VerifiedRyzoraOwned {
                return Err(IntegrationError::ConflictError(format!(
                    "Target path '{}' already exists and is not owned by Ryzora. Refusing to overwrite.",
                    norm_path.display()
                )));
            }
        }
    }

    Ok(PlannedIntegration {
        feature_id: feature_id.to_string(),
        integration_id: integration_id.to_string(),
        version,
        artifacts,
        metadata,
    })
}

/// Materializes planned integration artifacts and registers durable ownership.
pub fn apply_integration(
    planned: PlannedIntegration,
    home_dir: &Path,
) -> Result<IntegrationManifest, IntegrationError> {
    let now = current_timestamp();
    let mut installed_artifacts: Vec<IntegrationArtifact> = Vec::new();
    let mut created_dirs: Vec<PathBuf> = Vec::new();

    for planned_art in &planned.artifacts {
        // Ensure content includes the marker string
        let final_content = if !planned_art.content.contains(&planned_art.marker) {
            format!("{}\n{}", planned_art.marker, planned_art.content)
        } else {
            planned_art.content.clone()
        };

        let write_res = match atomic_write_artifact(
            &planned_art.path,
            &final_content,
            planned_art.permissions,
            home_dir,
        ) {
            Ok(res) => res,
            Err(e) => {
                // Rollback any artifacts written so far during this transaction
                for written in &installed_artifacts {
                    let _ = safe_remove_artifact_file(&written.path, home_dir);
                }
                for dir in created_dirs.iter().rev() {
                    let _ = safe_remove_created_directory(dir, home_dir);
                }
                return Err(IntegrationError::IoError(format!(
                    "Failed to write artifact '{}': {}. Transaction rolled back.",
                    planned_art.path.display(),
                    e
                )));
            }
        };

        created_dirs.extend(write_res.created_directories);

        installed_artifacts.push(IntegrationArtifact {
            path: write_res.path,
            artifact_type: planned_art.artifact_type,
            policy: planned_art.policy,
            sha256: write_res.sha256,
            marker: planned_art.marker.clone(),
            created_at: now,
            permissions: planned_art.permissions,
        });
    }

    // Deduplicate created directories
    created_dirs.sort();
    created_dirs.dedup();

    let manifest = IntegrationManifest {
        feature_id: planned.feature_id,
        integration_id: planned.integration_id,
        version: planned.version,
        enabled: true,
        artifacts: installed_artifacts,
        created_directories: created_dirs,
        metadata: planned.metadata,
        updated_at: now,
    };

    save_manifest(&manifest, home_dir)?;
    Ok(manifest)
}

/// Verifies the current health and status of an active integration.
pub fn verify_integration(
    integration_id: &str,
    home_dir: &Path,
) -> Result<Vec<ArtifactVerificationResult>, IntegrationError> {
    let manifest = load_manifest(integration_id, home_dir)?;
    Ok(verify_all(&manifest, home_dir))
}

/// Disables an integration, removing ONLY verified Ryzora-owned artifacts and preserving user modifications.
pub fn disable_integration(
    integration_id: &str,
    home_dir: &Path,
) -> Result<DisableReport, IntegrationError> {
    let manifest = load_manifest(integration_id, home_dir)?;

    let mut removed_artifacts = Vec::new();
    let mut preserved_artifacts = Vec::new();
    let mut missing_artifacts = Vec::new();
    let mut errors = Vec::new();

    for artifact in &manifest.artifacts {
        let check = verify_artifact(artifact, home_dir);
        match check.status {
            ArtifactVerificationStatus::VerifiedRyzoraOwned => {
                match safe_remove_artifact_file(&artifact.path, home_dir) {
                    Ok(_) => removed_artifacts.push(artifact.path.clone()),
                    Err(e) => errors.push(format!("Failed to remove {}: {}", artifact.path.display(), e)),
                }
            }
            ArtifactVerificationStatus::ModifiedByUser => {
                // Invariant: NEVER overwrite or delete user modifications
                preserved_artifacts.push(artifact.path.clone());
            }
            ArtifactVerificationStatus::Missing => {
                // Invariant: missing is idempotently complete
                missing_artifacts.push(artifact.path.clone());
            }
            ArtifactVerificationStatus::ForeignOrUnowned => {
                preserved_artifacts.push(artifact.path.clone());
                errors.push(format!(
                    "Artifact {} no longer possesses Ryzora ownership marker; preserved.",
                    artifact.path.display()
                ));
            }
            ArtifactVerificationStatus::SymlinkTargetUntrusted => {
                preserved_artifacts.push(artifact.path.clone());
                errors.push(format!(
                    "Artifact {} is an untrusted symlink; refusing removal to protect target.",
                    artifact.path.display()
                ));
            }
        }
    }

    let mut removed_directories = Vec::new();
    let mut preserved_directories = Vec::new();

    // Clean up empty directories that Ryzora explicitly created (in reverse order: innermost first)
    let mut created_dirs_sorted = manifest.created_directories.clone();
    created_dirs_sorted.sort_by(|a, b| b.components().count().cmp(&a.components().count()));

    for dir in created_dirs_sorted {
        match safe_remove_created_directory(&dir, home_dir) {
            Ok(true) => removed_directories.push(dir),
            Ok(false) => preserved_directories.push(dir),
            Err(e) => {
                errors.push(format!("Failed cleaning up empty directory {}: {}", dir.display(), e));
                preserved_directories.push(dir);
            }
        }
    }

    let fully_reverted = preserved_artifacts.is_empty() && errors.is_empty();

    if fully_reverted {
        delete_manifest(integration_id, home_dir)?;
    } else {
        // Keep manifest on disk marked as disabled with remaining preserved artifacts
        let mut updated = manifest;
        updated.enabled = false;
        updated.artifacts.retain(|a| preserved_artifacts.contains(&a.path));
        updated.updated_at = current_timestamp();
        let _ = save_manifest(&updated, home_dir);
    }

    Ok(DisableReport {
        integration_id: integration_id.to_string(),
        removed_artifacts,
        preserved_artifacts,
        missing_artifacts,
        removed_directories,
        preserved_directories,
        errors,
        fully_reverted,
    })
}
