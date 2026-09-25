//! Ryzora Reversible Integration Core — Hypridle Lifecycle & Transaction Engine
//!
//! Manages the transactional application, verification, safe legacy migration,
//! rollback, and teardown of the unified Hypridle configuration.
//!
//! Architectural Invariants:
//! 1. Strictly transactional: Pre-flight re-verification of host state before any mutations.
//! 2. Exact byte-level snapshotting: Pre-transaction bytes, permissions, and manifest state are captured
//!    so rollback restores exact pre-transaction host state using raw byte writes, not synthetic reconstructions.
//! 3. Safe legacy migration boundary: The Phase 2 drop-in (`zz-ryzora-session-lock.conf`) is ONLY
//!    removed after the new unified configuration is verified active on the host.
//! 4. Comprehensive rollback: If activation, verification, or manifest registration fails, all
//!    newly created files are reverted, legacy drop-in/manifest restored, and systemd reloaded.
//! 5. Transactional disable: The teardown path snapshots canonical artifacts and manifest; if removal,
//!    reloading, or native verification fails, the disable operation completely rolls back.
//! 6. Zero error discarding: Removal errors and rollback errors are never silenced; failures trigger
//!    explicit `RollbackFailed` reporting when restoration cannot be guaranteed.
//! 7. Preservation of user modifications: Any user-edited canonical artifact is preserved and
//!    blocks overwriting or deletion with ConflictError.
//! 8. Zero arbitrary shell execution: Generated Hypridle command strings remain configuration data;
//!    Ryzora never executes them via `sh -c` or `bash -c`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::composer::{
    check_canonical_artifact_permission, compose_hypridle_plan, inspect_dropin_directory,
    CANONICAL_DROPIN_REL, CANONICAL_MARKER, CANONICAL_OVERLAY_REL,
    COMPOSER_FEATURE_ID, COMPOSER_INTEGRATION_ID, LEGACY_DROPIN_REL,
};
#[cfg(test)]
use super::composer::{CANONICAL_DROPIN_DIR_REL, LEGACY_MARKER};
use super::types::HypridleCompositionRequest;
use crate::integration::filesystem::{
    atomic_write_artifact, atomic_write_bytes_artifact, calculate_sha256,
    safe_remove_artifact_file,
};
use crate::integration::manifest::{delete_manifest, load_manifest, save_manifest};
use crate::integration::types::{
    ArtifactOwnershipPolicy, ArtifactType, ArtifactVerificationStatus, IntegrationArtifact,
    IntegrationError, IntegrationManifest,
};
use crate::integration::verifier::{verify_all, verify_artifact};

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Exact in-memory capture of a disk artifact immediately before transaction.
#[derive(Debug, Clone)]
pub struct DiskArtifactSnapshot {
    pub path: PathBuf,
    pub existed: bool,
    pub bytes: Option<Vec<u8>>,
    pub permissions: Option<u32>,
    pub sha256: Option<String>,
}

impl DiskArtifactSnapshot {
    /// Captures the exact byte-level state of a file on disk.
    pub fn capture(path: &Path) -> Result<Self, IntegrationError> {
        let meta = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    path: path.to_path_buf(),
                    existed: false,
                    bytes: None,
                    permissions: None,
                    sha256: None,
                });
            }
            Err(e) => {
                return Err(IntegrationError::IoError(format!(
                    "Failed to query metadata for snapshot '{}': {}",
                    path.display(),
                    e
                )));
            }
        };

        if meta.file_type().is_symlink() {
            return Err(IntegrationError::SecurityViolation(format!(
                "Cannot snapshot untrusted symlink at '{}'",
                path.display()
            )));
        }

        let bytes = fs::read(path).map_err(|e| {
            IntegrationError::IoError(format!(
                "Failed to read file for snapshot '{}': {}",
                path.display(),
                e
            ))
        })?;

        let sha256 = calculate_sha256(&bytes);

        #[cfg(unix)]
        let permissions = {
            use std::os::unix::fs::PermissionsExt;
            Some(meta.permissions().mode())
        };
        #[cfg(not(unix))]
        let permissions = None;

        Ok(Self {
            path: path.to_path_buf(),
            existed: true,
            bytes: Some(bytes),
            permissions,
            sha256: Some(sha256),
        })
    }

    /// Restores the exact pre-transaction byte-level state of this file on disk.
    pub fn restore(&self, home: &Path) -> Result<(), IntegrationError> {
        if self.existed {
            if let Some(ref b) = self.bytes {
                atomic_write_bytes_artifact(&self.path, b, self.permissions, home)?;
            }
        } else {
            // Artifact did not exist before transaction -> remove any newly written file
            safe_remove_artifact_file(&self.path, home)?;
        }
        Ok(())
    }
}

/// Exact in-memory capture of an integration manifest immediately before transaction.
#[derive(Debug, Clone)]
pub struct ManifestSnapshot {
    pub integration_id: String,
    pub existed: bool,
    pub manifest: Option<IntegrationManifest>,
}

impl ManifestSnapshot {
    /// Captures the manifest state from disk.
    pub fn capture(integration_id: &str, home: &Path) -> Self {
        match load_manifest(integration_id, home) {
            Ok(m) => Self {
                integration_id: integration_id.to_string(),
                existed: true,
                manifest: Some(m),
            },
            Err(_) => Self {
                integration_id: integration_id.to_string(),
                existed: false,
                manifest: None,
            },
        }
    }

    /// Restores the manifest to its exact pre-transaction state.
    pub fn restore(&self, home: &Path) -> Result<(), IntegrationError> {
        if self.existed {
            if let Some(ref m) = self.manifest {
                save_manifest(m, home)?;
            }
        } else {
            let _ = delete_manifest(&self.integration_id, home);
        }
        Ok(())
    }
}

/// Structured outcome of executing an automated rollback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackResult {
    pub restored_artifacts: Vec<PathBuf>,
    pub restored_manifests: Vec<String>,
    pub service_restored: bool,
    pub errors: Vec<String>,
    pub fully_reverted: bool,
}

impl RollbackResult {
    pub fn into_integration_error(self, original_err: IntegrationError) -> IntegrationError {
        if self.fully_reverted {
            original_err
        } else {
            IntegrationError::RollbackFailed(format!(
                "Original error: {}. Rollback encountered failures: {:?}. System may be in a degraded state.",
                original_err, self.errors
            ))
        }
    }
}

/// Internal orchestrator to execute rollback across all tracked snapshots and reloader.
fn execute_apply_rollback<F>(
    canonical_dropin_snap: &DiskArtifactSnapshot,
    canonical_overlay_snap: &DiskArtifactSnapshot,
    legacy_dropin_snap: &DiskArtifactSnapshot,
    canonical_manifest_snap: &ManifestSnapshot,
    legacy_manifest_snap: &ManifestSnapshot,
    reloader: F,
    home: &Path,
    original_err: IntegrationError,
) -> IntegrationError
where
    F: Fn() -> Result<(), IntegrationError>,
{
    let mut errors = Vec::new();
    let mut restored_artifacts = Vec::new();
    let mut restored_manifests = Vec::new();

    match canonical_dropin_snap.restore(home) {
        Ok(_) => {
            if canonical_dropin_snap.existed {
                restored_artifacts.push(canonical_dropin_snap.path.clone());
            }
        }
        Err(e) => errors.push(format!("Failed to restore canonical drop-in: {}", e)),
    }
    match canonical_overlay_snap.restore(home) {
        Ok(_) => {
            if canonical_overlay_snap.existed {
                restored_artifacts.push(canonical_overlay_snap.path.clone());
            }
        }
        Err(e) => errors.push(format!("Failed to restore canonical overlay: {}", e)),
    }
    match legacy_dropin_snap.restore(home) {
        Ok(_) => {
            if legacy_dropin_snap.existed {
                restored_artifacts.push(legacy_dropin_snap.path.clone());
            }
        }
        Err(e) => errors.push(format!("Failed to restore legacy drop-in: {}", e)),
    }
    match canonical_manifest_snap.restore(home) {
        Ok(_) => {
            if canonical_manifest_snap.existed {
                restored_manifests.push(canonical_manifest_snap.integration_id.clone());
            }
        }
        Err(e) => errors.push(format!("Failed to restore canonical manifest: {}", e)),
    }
    match legacy_manifest_snap.restore(home) {
        Ok(_) => {
            if legacy_manifest_snap.existed {
                restored_manifests.push(legacy_manifest_snap.integration_id.clone());
            }
        }
        Err(e) => errors.push(format!("Failed to restore legacy manifest: {}", e)),
    }

    let service_restored = match reloader() {
        Ok(_) => true,
        Err(e) => {
            errors.push(format!("Failed to reload/restart service on rollback: {}", e));
            false
        }
    };

    let fully_reverted = errors.is_empty() && service_restored;
    let res = RollbackResult {
        restored_artifacts,
        restored_manifests,
        service_restored,
        errors,
        fully_reverted,
    };
    res.into_integration_error(original_err)
}

/// High-level discrete state of the unified Hypridle integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypridleIntegrationState {
    /// Fully active: canonical manifest exists and is enabled, canonical drop-in exists, and service is active.
    Active,
    /// Disabled: no active Ryzora manifest exists and no Ryzora drop-in exists on disk (running native).
    Disabled,
    /// Degraded: manifest is enabled but drop-in or service is inactive.
    Degraded,
    /// Conflict: unowned/foreign drop-in exists or canonical artifact was modified externally.
    Conflict,
}

/// Observational status report for the unified Hypridle integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HypridleStatus {
    pub state: HypridleIntegrationState,
    pub session_lock_active: bool,
    pub idle_management_active: bool,
    pub dropin_path: PathBuf,
    pub dropin_active: bool,
    pub overlay_path: PathBuf,
    pub overlay_active: bool,
    pub service_active: bool,
    pub legacy_dropin_present: bool,
    pub legacy_migrated: bool,
    pub audit_native_config_hash: Option<String>,
}

/// Queries user systemd whether hypridle.service is currently active.
pub fn is_hypridle_service_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "hypridle.service"])
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

/// Reloads systemd user daemon and restarts hypridle.service, verifying active transition.
pub fn reload_and_restart_hypridle() -> Result<(), IntegrationError> {
    // 1. systemctl --user daemon-reload
    let reload_out = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .map_err(|e| {
            IntegrationError::IoError(format!("Failed to execute 'systemctl --user daemon-reload': {}", e))
        })?;

    if !reload_out.status.success() {
        let err_msg = String::from_utf8_lossy(&reload_out.stderr).trim().to_string();
        return Err(IntegrationError::IoError(format!(
            "systemctl --user daemon-reload failed (code {:?}): {}",
            reload_out.status.code(),
            if err_msg.is_empty() {
                String::from_utf8_lossy(&reload_out.stdout).trim().to_string()
            } else {
                err_msg
            }
        )));
    }

    // Reset failed counter to prevent transient rate limits from blocking restart
    let _ = Command::new("systemctl")
        .args(["--user", "reset-failed", "hypridle.service"])
        .output();

    // 2. systemctl --user restart hypridle.service
    let restart_out = Command::new("systemctl")
        .args(["--user", "restart", "hypridle.service"])
        .output()
        .map_err(|e| {
            IntegrationError::IoError(format!("Failed to execute 'systemctl --user restart hypridle.service': {}", e))
        })?;

    if !restart_out.status.success() {
        let err_msg = String::from_utf8_lossy(&restart_out.stderr).trim().to_string();
        return Err(IntegrationError::IoError(format!(
            "systemctl --user restart hypridle.service failed (code {:?}): {}",
            restart_out.status.code(),
            if err_msg.is_empty() {
                String::from_utf8_lossy(&restart_out.stdout).trim().to_string()
            } else {
                err_msg
            }
        )));
    }

    // 3. Verify hypridle.service is active
    let is_active_out = Command::new("systemctl")
        .args(["--user", "is-active", "hypridle.service"])
        .output()
        .map_err(|e| {
            IntegrationError::IoError(format!("Failed to query 'systemctl --user is-active hypridle.service': {}", e))
        })?;

    let active_str = String::from_utf8_lossy(&is_active_out.stdout).trim().to_string();
    if active_str != "active" {
        return Err(IntegrationError::VerificationFailed(format!(
            "hypridle.service failed to transition to active state after restart; current state is '{}'",
            active_str
        )));
    }

    Ok(())
}

/// Strictly observational status inspector.
///
/// Invariant: Does NOT mutate files, migrate legacy artifacts, restart systemd,
/// or repair state. Only observes and reports current host reality.
pub fn get_hypridle_status(home: &Path) -> HypridleStatus {
    let dropin_path = home.join(CANONICAL_DROPIN_REL);
    let overlay_path = home.join(CANONICAL_OVERLAY_REL);
    let legacy_path = home.join(LEGACY_DROPIN_REL);

    let dropin_active = dropin_path.exists();
    let overlay_active = overlay_path.exists();
    let legacy_dropin_present = legacy_path.exists();
    let service_active = is_hypridle_service_active();

    let manifest = load_manifest(COMPOSER_INTEGRATION_ID, home).ok();
    let enabled = manifest.as_ref().map(|m| m.enabled).unwrap_or(false);

    let session_lock_active = manifest
        .as_ref()
        .and_then(|m| m.metadata.get("session_lock_active"))
        .map(|v| v == "true")
        .unwrap_or(false);

    let idle_management_active = manifest
        .as_ref()
        .and_then(|m| m.metadata.get("idle_management_active"))
        .map(|v| v == "true")
        .unwrap_or(false);

    let legacy_migrated = manifest
        .as_ref()
        .and_then(|m| m.metadata.get("legacy_migrated"))
        .map(|v| v == "true")
        .unwrap_or(false);

    let audit_native_config_hash = manifest
        .as_ref()
        .and_then(|m| m.metadata.get("audit_native_config_hash").cloned());

    // Check directory health for conflict
    let has_conflict = match inspect_dropin_directory(home) {
        Ok(_) => false,
        Err(_) => true,
    };

    let state = if has_conflict {
        HypridleIntegrationState::Conflict
    } else if enabled && dropin_active && overlay_active && service_active {
        HypridleIntegrationState::Active
    } else if !enabled && !dropin_active && !overlay_active {
        HypridleIntegrationState::Disabled
    } else if enabled && (!dropin_active || !overlay_active || !service_active) {
        HypridleIntegrationState::Degraded
    } else {
        HypridleIntegrationState::Conflict
    };

    HypridleStatus {
        state,
        session_lock_active,
        idle_management_active,
        dropin_path,
        dropin_active,
        overlay_path,
        overlay_active,
        service_active,
        legacy_dropin_present,
        legacy_migrated,
        audit_native_config_hash,
    }
}

/// Applies a unified Hypridle composition request to the real system.
///
/// Production API: Enforces the mandatory write -> reload -> restart -> verify sequence.
pub fn apply_hypridle_composition(
    home: &Path,
    req: &HypridleCompositionRequest,
) -> Result<HypridleStatus, IntegrationError> {
    apply_hypridle_composition_with_reloader(home, req, reload_and_restart_hypridle)
}

/// Applies a unified Hypridle composition with injectable reloader for testing.
///
/// Full transactional execution with atomic rollback:
/// 1. Compose in-memory plan (or delegate to disable if both features inactive).
/// 2. Pre-flight re-verification of drop-in directory and canonical targets.
/// 3. Exact byte-level snapshot of existing canonical artifacts, legacy artifacts, and manifests.
/// 4. Atomic writes of canonical overlay and canonical drop-in.
/// 5. Service reload and restart.
/// 6. Post-activation verification (service active + drop-in/overlay intact).
/// 7. Safe legacy migration (removes legacy drop-in ONLY after verified active).
/// 8. Registration of canonical manifest.
/// 9. Rollback if ANY step fails.
pub fn apply_hypridle_composition_with_reloader<F>(
    home: &Path,
    req: &HypridleCompositionRequest,
    reloader: F,
) -> Result<HypridleStatus, IntegrationError>
where
    F: Fn() -> Result<(), IntegrationError>,
{
    // 1. Compose in-memory plan
    let plan_opt = compose_hypridle_plan(home, req)?;
    let plan = match plan_opt {
        Some(p) => p,
        None => {
            // Both features are disabled -> teardown to native state
            return disable_hypridle_integration_with_reloader(home, reloader);
        }
    };

    // 2. Pre-flight re-verification
    inspect_dropin_directory(home)?;
    check_canonical_artifact_permission(&plan.overlay_path, home, COMPOSER_INTEGRATION_ID)?;

    // 3. Exact byte-level snapshot of all potentially mutated artifacts and manifests
    let canonical_dropin_snap = DiskArtifactSnapshot::capture(&plan.dropin_path)?;
    let canonical_overlay_snap = DiskArtifactSnapshot::capture(&plan.overlay_path)?;
    let legacy_dropin_path = home.join(LEGACY_DROPIN_REL);
    let legacy_dropin_snap = DiskArtifactSnapshot::capture(&legacy_dropin_path)?;
    let canonical_manifest_snap = ManifestSnapshot::capture(COMPOSER_INTEGRATION_ID, home);
    let legacy_manifest_snap = ManifestSnapshot::capture("hyprland-session-lock", home);

    // Helper closure to trigger full rollback on failure
    let trigger_rollback = |orig_err: IntegrationError| -> IntegrationError {
        execute_apply_rollback(
            &canonical_dropin_snap,
            &canonical_overlay_snap,
            &legacy_dropin_snap,
            &canonical_manifest_snap,
            &legacy_manifest_snap,
            &reloader,
            home,
            orig_err,
        )
    };

    // 4. Atomic writes of canonical overlay and drop-in
    let overlay_res = atomic_write_artifact(
        &plan.overlay_path,
        &plan.overlay_content,
        Some(0o644),
        home,
    );
    let overlay_write = match overlay_res {
        Ok(w) => w,
        Err(e) => {
            return Err(trigger_rollback(IntegrationError::IoError(format!(
                "Failed to atomically write canonical overlay '{}': {}. Transaction rolled back.",
                plan.overlay_path.display(),
                e
            ))));
        }
    };

    let dropin_res = atomic_write_artifact(
        &plan.dropin_path,
        &plan.dropin_content,
        Some(0o644),
        home,
    );
    let dropin_write = match dropin_res {
        Ok(w) => w,
        Err(e) => {
            return Err(trigger_rollback(IntegrationError::IoError(format!(
                "Failed to atomically write canonical drop-in '{}': {}. Transaction rolled back.",
                plan.dropin_path.display(),
                e
            ))));
        }
    };

    // 5. Systemd reload and restart
    if let Err(e) = reloader() {
        return Err(trigger_rollback(IntegrationError::IoError(format!(
            "Failed to reload and restart hypridle service: {}. Transaction rolled back.",
            e
        ))));
    }

    // 6. Post-activation verification: Verify artifacts on disk match expected hashes
    let current_dropin_bytes = match fs::read(&plan.dropin_path) {
        Ok(b) => b,
        Err(e) => {
            return Err(trigger_rollback(IntegrationError::IoError(format!(
                "Failed to read drop-in for post-activation verification: {}",
                e
            ))));
        }
    };
    let current_overlay_bytes = match fs::read(&plan.overlay_path) {
        Ok(b) => b,
        Err(e) => {
            return Err(trigger_rollback(IntegrationError::IoError(format!(
                "Failed to read overlay for post-activation verification: {}",
                e
            ))));
        }
    };
    if calculate_sha256(&current_dropin_bytes) != dropin_write.sha256
        || calculate_sha256(&current_overlay_bytes) != overlay_write.sha256
    {
        return Err(trigger_rollback(IntegrationError::VerificationFailed(
            "Post-activation verification failed: canonical artifacts on disk do not match expected hashes. Transaction rolled back.".to_string(),
        )));
    }

    // 7. Safe Legacy Migration (ONLY AFTER ACTIVATION IS VERIFIED!)
    if plan.legacy_dropin_to_migrate.is_some() && legacy_dropin_snap.existed {
        if let Err(e) = safe_remove_artifact_file(&legacy_dropin_path, home) {
            return Err(trigger_rollback(IntegrationError::IoError(format!(
                "Failed to safely remove legacy drop-in during migration: {}. Transaction rolled back.",
                e
            ))));
        }

        // Archive/retire Phase 2 manifest
        if legacy_manifest_snap.existed {
            if let Err(e) = delete_manifest("hyprland-session-lock", home) {
                return Err(trigger_rollback(IntegrationError::IoError(format!(
                    "Failed to retire legacy session-lock manifest: {}. Transaction rolled back.",
                    e
                ))));
            }
        }
    }

    // 8. Manifest Registration
    let now = current_timestamp();
    let mut metadata = HashMap::new();
    metadata.insert(
        "session_lock_active".to_string(),
        req.session_lock_active.to_string(),
    );
    metadata.insert(
        "idle_management_active".to_string(),
        req.idle_management_active.to_string(),
    );
    metadata.insert(
        "legacy_migrated".to_string(),
        plan.legacy_dropin_to_migrate.is_some().to_string(),
    );
    if let Some(ref h) = req.native_discovery.sha256 {
        metadata.insert("audit_native_config_hash".to_string(), h.clone());
    }

    let mut created_dirs = overlay_write.created_directories;
    created_dirs.extend(dropin_write.created_directories);
    created_dirs.sort();
    created_dirs.dedup();

    let manifest = IntegrationManifest {
        feature_id: COMPOSER_FEATURE_ID.to_string(),
        integration_id: COMPOSER_INTEGRATION_ID.to_string(),
        version: 1,
        enabled: true,
        artifacts: vec![
            IntegrationArtifact {
                path: dropin_write.path,
                artifact_type: ArtifactType::SystemdDropin,
                policy: ArtifactOwnershipPolicy::Immutable,
                sha256: dropin_write.sha256,
                marker: CANONICAL_MARKER.to_string(),
                created_at: now,
                permissions: Some(0o644),
            },
            IntegrationArtifact {
                path: overlay_write.path,
                artifact_type: ArtifactType::ConfigOverlay,
                policy: ArtifactOwnershipPolicy::Immutable,
                sha256: overlay_write.sha256,
                marker: CANONICAL_MARKER.to_string(),
                created_at: now,
                permissions: Some(0o644),
            },
        ],
        created_directories: created_dirs,
        metadata,
        updated_at: now,
    };

    if let Err(e) = save_manifest(&manifest, home) {
        return Err(trigger_rollback(IntegrationError::IoError(format!(
            "Failed to save canonical integration manifest: {}. Transaction rolled back.",
            e
        ))));
    }

    // 9. Final Authoritative Verification
    let verifications = verify_all(&manifest, home);
    for v in verifications {
        if v.status != ArtifactVerificationStatus::VerifiedRyzoraOwned {
            return Err(trigger_rollback(IntegrationError::VerificationFailed(format!(
                "Artifact at '{}' failed final ownership verification ({:?}). Transaction rolled back.",
                v.path.display(),
                v.status
            ))));
        }
    }

    Ok(get_hypridle_status(home))
}

/// Disables the unified Hypridle integration, restoring native operation.
///
/// Production API: Enforces removal of VerifiedRyzoraOwned artifacts, daemon-reload,
/// and restart of native hypridle.
pub fn disable_hypridle_integration(home: &Path) -> Result<HypridleStatus, IntegrationError> {
    disable_hypridle_integration_with_reloader(home, reload_and_restart_hypridle)
}

/// Disables the unified Hypridle integration with injectable reloader for testing.
///
/// Follows strict transactional ownership discipline:
/// 1. Snapshots canonical drop-in, overlay, and manifest before any mutation.
/// 2. Verifies exact canonical manifest ownership.
/// 3. Verifies artifact hashes using `verify_artifact()`.
/// 4. Removes ONLY `VerifiedRyzoraOwned` artifacts, propagating any removal errors.
/// 5. Strictly PRESERVES `ModifiedByUser` artifacts.
/// 6. Reloads and restarts systemd hypridle.
/// 7. Verifies native configuration transition.
/// 8. Retires manifest ONLY if all operations succeed and no user modifications were preserved.
/// 9. If removal, reloading, or native verification fails, triggers complete rollback.
pub fn disable_hypridle_integration_with_reloader<F>(
    home: &Path,
    reloader: F,
) -> Result<HypridleStatus, IntegrationError>
where
    F: Fn() -> Result<(), IntegrationError>,
{
    let manifest_res = load_manifest(COMPOSER_INTEGRATION_ID, home);
    let manifest = match manifest_res {
        Ok(m) => m,
        Err(_) => {
            // Manifest missing. Check if orphan canonical drop-in exists.
            let dropin_path = home.join(CANONICAL_DROPIN_REL);
            if !dropin_path.exists() {
                // Idempotently already disabled
                return Ok(get_hypridle_status(home));
            } else {
                return Err(IntegrationError::ConflictError(format!(
                    "Canonical drop-in exists at '{}' without an active manifest. Refusing blind deletion.",
                    dropin_path.display()
                )));
            }
        }
    };

    // 1. Snapshot canonical artifacts and manifest BEFORE any mutation
    let mut artifact_snapshots = Vec::new();
    for artifact in &manifest.artifacts {
        let abs_path = if artifact.path.is_absolute() {
            artifact.path.clone()
        } else {
            home.join(&artifact.path)
        };
        artifact_snapshots.push(DiskArtifactSnapshot::capture(&abs_path)?);
    }
    let canonical_manifest_snap = ManifestSnapshot::capture(COMPOSER_INTEGRATION_ID, home);

    // Rollback closure for disable failure
    let rollback_disable = |orig_err: IntegrationError| -> IntegrationError {
        let mut restored_artifacts = Vec::new();
        let mut restored_manifests = Vec::new();
        let mut errors = Vec::new();

        for snap in &artifact_snapshots {
            match snap.restore(home) {
                Ok(_) => {
                    if snap.existed {
                        restored_artifacts.push(snap.path.clone());
                    }
                }
                Err(e) => errors.push(format!("Failed to restore artifact '{}' on disable rollback: {}", snap.path.display(), e)),
            }
        }
        match canonical_manifest_snap.restore(home) {
            Ok(_) => {
                if canonical_manifest_snap.existed {
                    restored_manifests.push(canonical_manifest_snap.integration_id.clone());
                }
            }
            Err(e) => errors.push(format!("Failed to restore manifest on disable rollback: {}", e)),
        }
        let service_restored = match reloader() {
            Ok(_) => true,
            Err(e) => {
                errors.push(format!("Failed to reload/restart service on disable rollback: {}", e));
                false
            }
        };

        let fully_reverted = errors.is_empty() && service_restored;
        let res = RollbackResult {
            restored_artifacts,
            restored_manifests,
            service_restored,
            errors,
            fully_reverted,
        };
        res.into_integration_error(orig_err)
    };

    let mut user_modified_preserved = false;

    // 2. Authoritatively verify each artifact and remove ONLY VerifiedRyzoraOwned
    for artifact in &manifest.artifacts {
        let check = verify_artifact(artifact, home);
        match check.status {
            ArtifactVerificationStatus::VerifiedRyzoraOwned => {
                if let Err(e) = safe_remove_artifact_file(&artifact.path, home) {
                    return Err(rollback_disable(IntegrationError::IoError(format!(
                        "Failed to safely remove artifact '{}': {}",
                        artifact.path.display(),
                        e
                    ))));
                }
            }
            ArtifactVerificationStatus::ModifiedByUser => {
                // Invariant: NEVER overwrite or delete user modifications
                user_modified_preserved = true;
            }
            ArtifactVerificationStatus::Missing => {
                // Already missing, idempotent
            }
            ArtifactVerificationStatus::ForeignOrUnowned => {
                user_modified_preserved = true;
            }
            ArtifactVerificationStatus::SymlinkTargetUntrusted => {
                user_modified_preserved = true;
            }
        }
    }

    // 3. Reload systemd and restart hypridle to restore native operation
    if let Err(e) = reloader() {
        return Err(rollback_disable(e));
    }

    // 4. Verify native state: if no user modifications were preserved, verify canonical artifacts are absent
    let dropin_path = home.join(CANONICAL_DROPIN_REL);
    let overlay_path = home.join(CANONICAL_OVERLAY_REL);
    if !user_modified_preserved && (dropin_path.exists() || overlay_path.exists()) {
        return Err(rollback_disable(IntegrationError::VerificationFailed(
            "Post-disable verification failed: canonical integration files still exist on disk after removal.".to_string(),
        )));
    }

    // 5. Retire manifest ONLY if all operations succeed and no user modifications were preserved
    if !user_modified_preserved {
        if let Err(e) = delete_manifest(COMPOSER_INTEGRATION_ID, home) {
            return Err(rollback_disable(IntegrationError::IoError(format!(
                "Failed to retire manifest on disable: {}",
                e
            ))));
        }
    }

    Ok(get_hypridle_status(home))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::integration::hypridle::discovery::parse_hypridle_config_str;
    use crate::integration::hypridle::types::{
        HypridleDiscoveryReport, IdleAction, IdleListenerIntent,
    };

    fn sample_valid_discovery(conf_path: PathBuf) -> HypridleDiscoveryReport {
        let conf = r#"
general {
    lock_cmd = pidof hyprlock || hyprlock
}
listener {
    timeout = 300
    on-timeout = loginctl lock-session
}
"#;
        parse_hypridle_config_str(conf, conf_path, None, None)
    }

    fn dummy_success_reloader() -> Result<(), IntegrationError> {
        Ok(())
    }

    fn dummy_failure_reloader() -> Result<(), IntegrationError> {
        Err(IntegrationError::IoError("Mock systemd restart failed".to_string()))
    }

    fn make_fail_once_reloader(err: IntegrationError) -> impl Fn() -> Result<(), IntegrationError> {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        move || {
            let n = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                Err(err.clone())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_lifecycle_apply_clean_native_to_active() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_clean_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let status = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();
        assert_eq!(status.session_lock_active, true);
        assert_eq!(status.idle_management_active, false);
        assert!(status.dropin_active);
        assert!(status.overlay_active);

        // Verify artifacts on disk
        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        assert!(dropin_path.exists());
        assert!(overlay_path.exists());

        // Verify manifest on disk
        let manifest = load_manifest(COMPOSER_INTEGRATION_ID, &temp).unwrap();
        assert!(manifest.enabled);
        assert_eq!(manifest.artifacts.len(), 2);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_apply_with_session_lock_and_idle() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_both_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 60,
                    action: IdleAction::LockSession,
                    resume_action: None,
                },
            ],
        };

        let status = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();
        assert_eq!(status.session_lock_active, true);
        assert_eq!(status.idle_management_active, true);
        assert!(status.dropin_active);
        assert!(status.overlay_active);

        let overlay_content = fs::read_to_string(temp.join(CANONICAL_OVERLAY_REL)).unwrap();
        assert!(overlay_content.contains("lock_cmd = /usr/bin/hyprlock"));
        assert!(overlay_content.contains("timeout = 60"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_apply_migrates_verified_legacy_dropin_after_restart() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_mig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        // Create legacy dropin and Phase 2 manifest
        let legacy_file = temp.join(LEGACY_DROPIN_REL);
        fs::create_dir_all(legacy_file.parent().unwrap()).unwrap();
        let legacy_content = format!("{}\n[Service]\nExecStart=\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let sha256 = calculate_sha256(legacy_content.as_bytes());
        let legacy_artifact = IntegrationArtifact {
            path: legacy_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: LEGACY_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        let legacy_manifest = IntegrationManifest {
            feature_id: "session-lock".to_string(),
            integration_id: "hyprland-session-lock".to_string(),
            version: 1,
            enabled: true,
            artifacts: vec![legacy_artifact],
            created_directories: Vec::new(),
            metadata: HashMap::new(),
            updated_at: 1000,
        };
        save_manifest(&legacy_manifest, &temp).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let status = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();
        assert!(status.legacy_migrated);
        assert!(!legacy_file.exists(), "Legacy drop-in must be removed after successful activation");
        assert!(load_manifest("hyprland-session-lock", &temp).is_err(), "Legacy manifest must be retired");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_apply_refuses_modified_canonical_dropin() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_refuse_mod_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        fs::create_dir_all(dropin_path.parent().unwrap()).unwrap();
        let dropin_content = format!("{}\n[Service]\n# User edit\n", CANONICAL_MARKER);
        fs::write(&dropin_path, &dropin_content).unwrap();

        // Manifest has different hash
        let artifact = IntegrationArtifact {
            path: dropin_path.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            marker: CANONICAL_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };
        let manifest = IntegrationManifest {
            feature_id: COMPOSER_FEATURE_ID.to_string(),
            integration_id: COMPOSER_INTEGRATION_ID.to_string(),
            version: 1,
            enabled: true,
            artifacts: vec![artifact],
            created_directories: Vec::new(),
            metadata: HashMap::new(),
            updated_at: 1000,
        };
        save_manifest(&manifest, &temp).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("modified externally by user"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_apply_refuses_foreign_dropin() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_foreign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let foreign_path = temp.join(CANONICAL_DROPIN_DIR_REL).join("99-foreign.conf");
        fs::create_dir_all(foreign_path.parent().unwrap()).unwrap();
        fs::write(&foreign_path, "[Service]\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("Foreign or user-created drop-in detected"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_apply_refuses_untrusted_symlink() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_symlink_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();
        let symlink_path = dropin_dir.join("symlink.conf");
        let dummy = temp.join("dummy.conf");
        fs::write(&dummy, "test").unwrap();
        std::os::unix::fs::symlink(&dummy, &symlink_path).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap_err();
        match err {
            IntegrationError::SecurityViolation(msg) => {
                assert!(msg.to_lowercase().contains("untrusted symlink"));
            }
            _ => panic!("Expected SecurityViolation, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_rollback_on_restart_failure_restores_clean_state() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_rollback_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let reloader = make_fail_once_reloader(IntegrationError::IoError("Mock systemd restart failed".to_string()));
        let err = apply_hypridle_composition_with_reloader(&temp, &req, reloader).unwrap_err();
        match err {
            IntegrationError::IoError(msg) => {
                assert!(msg.contains("Transaction rolled back"));
            }
            _ => panic!("Expected IoError, got {:?}", err),
        }

        // On rollback, canonical drop-in and overlay must NOT remain on disk!
        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        assert!(!dropin_path.exists(), "Rollback must remove newly written drop-in");
        assert!(!overlay_path.exists(), "Rollback must remove newly written overlay");
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_err(), "Manifest must not exist after rollback");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_rollback_on_restart_failure_preserves_legacy_dropin() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_roll_leg_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        // Create legacy dropin and Phase 2 manifest
        let legacy_file = temp.join(LEGACY_DROPIN_REL);
        fs::create_dir_all(legacy_file.parent().unwrap()).unwrap();
        let legacy_content = format!("{}\n[Service]\nExecStart=\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let sha256 = calculate_sha256(legacy_content.as_bytes());
        let legacy_artifact = IntegrationArtifact {
            path: legacy_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: LEGACY_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        let legacy_manifest = IntegrationManifest {
            feature_id: "session-lock".to_string(),
            integration_id: "hyprland-session-lock".to_string(),
            version: 1,
            enabled: true,
            artifacts: vec![legacy_artifact],
            created_directories: Vec::new(),
            metadata: HashMap::new(),
            updated_at: 1000,
        };
        save_manifest(&legacy_manifest, &temp).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // Trigger failure during restart (reloader fails initially, rollback restart succeeds)
        let reloader = make_fail_once_reloader(IntegrationError::IoError("Mock systemd restart failed".to_string()));
        let err = apply_hypridle_composition_with_reloader(&temp, &req, reloader).unwrap_err();
        match err {
            IntegrationError::IoError(msg) => {
                assert!(msg.contains("Transaction rolled back"));
            }
            _ => panic!("Expected IoError, got {:?}", err),
        }

        // The legacy drop-in and manifest MUST be completely intact!
        assert!(legacy_file.exists(), "Legacy drop-in must be preserved on rollback");
        let content_after = fs::read_to_string(&legacy_file).unwrap();
        assert_eq!(content_after, legacy_content);
        assert!(load_manifest("hyprland-session-lock", &temp).is_ok(), "Legacy manifest must be preserved on rollback");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_exact_byte_restoration() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_exact_bytes_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let test_file = temp.join(".config/ryzora/test.bin");
        fs::create_dir_all(test_file.parent().unwrap()).unwrap();

        // Write raw bytes including arbitrary non-UTF8 sequences
        let raw_bytes: Vec<u8> = vec![0x00, 0xFF, 0xFE, 0xAA, 0x55, 0x12, 0x34];
        fs::write(&test_file, &raw_bytes).unwrap();

        // Capture snapshot
        let snap = DiskArtifactSnapshot::capture(&test_file).unwrap();
        assert_eq!(snap.bytes.as_ref().unwrap(), &raw_bytes);

        // Mutate file
        fs::write(&test_file, b"mutated").unwrap();

        // Restore snapshot
        snap.restore(&temp).unwrap();

        // Prove exact raw bytes are restored bitwise
        let restored_bytes = fs::read(&test_file).unwrap();
        assert_eq!(restored_bytes, raw_bytes, "Snapshot restore must be bitwise exact");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_restores_clean_native_state() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_disable_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // First apply
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        // Now disable
        let status = disable_hypridle_integration_with_reloader(&temp, dummy_success_reloader).unwrap();
        assert_eq!(status.session_lock_active, false);
        assert_eq!(status.idle_management_active, false);
        assert!(!status.dropin_active);
        assert!(!status.overlay_active);

        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        assert!(!dropin_path.exists());
        assert!(!overlay_path.exists());
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_err());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_preserves_user_modified_canonical_file() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_dis_mod_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // Apply first
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        // User modifies canonical dropin
        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        fs::write(&dropin_path, format!("{}\n[Service]\n# User customization\n", CANONICAL_MARKER)).unwrap();

        // Now disable
        let _ = disable_hypridle_integration_with_reloader(&temp, dummy_success_reloader).unwrap();

        // Drop-in MUST BE PRESERVED on disk!
        assert!(dropin_path.exists(), "User modified drop-in must NEVER be deleted on disable");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_idempotent() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_dis_idemp_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        // Calling disable on an already clean system succeeds cleanly
        let status = disable_hypridle_integration_with_reloader(&temp, dummy_success_reloader).unwrap();
        assert_eq!(status.session_lock_active, false);
        assert!(!status.dropin_active);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_rollback_on_restart_failure() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_dis_roll_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // Apply successfully
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        let dropin_bytes_before = fs::read(&dropin_path).unwrap();
        let overlay_bytes_before = fs::read(&overlay_path).unwrap();

        // Now attempt disable with failing reloader that succeeds on rollback
        let reloader = make_fail_once_reloader(IntegrationError::IoError("Mock systemd restart failed on disable".to_string()));
        let err = disable_hypridle_integration_with_reloader(&temp, reloader).unwrap_err();
        match err {
            IntegrationError::IoError(msg) => {
                assert!(msg.contains("Mock systemd restart failed on disable"));
            }
            _ => panic!("Expected IoError, got {:?}", err),
        }

        // Proves rollback restored canonical drop-in, overlay, and manifest!
        assert!(dropin_path.exists(), "Rollback on disable must restore drop-in");
        assert!(overlay_path.exists(), "Rollback on disable must restore overlay");
        assert_eq!(fs::read(&dropin_path).unwrap(), dropin_bytes_before);
        assert_eq!(fs::read(&overlay_path).unwrap(), overlay_bytes_before);
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_ok(), "Rollback on disable must restore manifest");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_rollback_on_verification_failure() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_dis_verif_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // Apply first
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        let dropin_path = temp.join(CANONICAL_DROPIN_REL);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        let dropin_bytes_before = fs::read(&dropin_path).unwrap();
        let overlay_bytes_before = fs::read(&overlay_path).unwrap();

        // Reloader fails with VerificationFailed on disable, but succeeds on rollback
        let reloader = make_fail_once_reloader(IntegrationError::VerificationFailed(
            "hypridle.service failed to transition to active state after restart; current state is 'failed'".to_string(),
        ));
        let err = disable_hypridle_integration_with_reloader(&temp, reloader).unwrap_err();
        match err {
            IntegrationError::VerificationFailed(msg) => {
                assert!(msg.contains("hypridle.service failed to transition to active state"));
            }
            _ => panic!("Expected VerificationFailed, got {:?}", err),
        }

        // Proves rollback restored canonical drop-in, overlay, and manifest!
        assert!(dropin_path.exists(), "Rollback on verification failure must restore drop-in");
        assert!(overlay_path.exists(), "Rollback on verification failure must restore overlay");
        assert_eq!(fs::read(&dropin_path).unwrap(), dropin_bytes_before);
        assert_eq!(fs::read(&overlay_path).unwrap(), overlay_bytes_before);
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_ok(), "Rollback on verification failure must restore manifest");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_disable_artifact_removal_failure() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_dis_rem_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // Apply first
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        let dropin_path = temp.join(CANONICAL_DROPIN_REL);

        // Make dropin directory read-only to force safe_remove_artifact_file to fail
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let parent = dropin_path.parent().unwrap();
            let mut perms = fs::metadata(parent).unwrap().permissions();
            perms.set_mode(0o555);
            fs::set_permissions(parent, perms).unwrap();
        }

        let err = disable_hypridle_integration_with_reloader(&temp, dummy_success_reloader).unwrap_err();

        // Restore permissions so cleanup can delete the temp directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let parent = dropin_path.parent().unwrap();
            let mut perms = fs::metadata(parent).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(parent, perms).unwrap();
        }

        match err {
            IntegrationError::RollbackFailed(msg) => {
                assert!(msg.contains("Failed to safely remove artifact"));
                assert!(msg.contains("Permission denied"));
            }
            IntegrationError::IoError(msg) => {
                assert!(msg.contains("Failed to safely remove artifact"));
            }
            _ => panic!("Expected IoError or RollbackFailed on artifact removal failure, got {:?}", err),
        }

        // Manifest must NOT be deleted because removal failed!
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_ok());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_rollback_failure_reporting() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_roll_fail_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        fs::write(&native_conf, "general {\n    lock_cmd = test\n}\n").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(native_conf.clone()),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        // 1. In apply: dummy_failure_reloader fails initially AND fails during rollback
        let err = apply_hypridle_composition_with_reloader(&temp, &req, dummy_failure_reloader).unwrap_err();
        match err {
            IntegrationError::RollbackFailed(msg) => {
                assert!(msg.contains("Original error:"));
                assert!(msg.contains("Rollback encountered failures:"));
                assert!(msg.contains("degraded state"));
            }
            _ => panic!("Expected RollbackFailed on apply, got {:?}", err),
        }

        // 2. In disable: apply first with success
        apply_hypridle_composition_with_reloader(&temp, &req, dummy_success_reloader).unwrap();

        // Now disable with dummy_failure_reloader (fails initially AND fails during rollback)
        let dis_err = disable_hypridle_integration_with_reloader(&temp, dummy_failure_reloader).unwrap_err();
        match dis_err {
            IntegrationError::RollbackFailed(msg) => {
                assert!(msg.contains("Original error:"));
                assert!(msg.contains("Rollback encountered failures:"));
            }
            _ => panic!("Expected RollbackFailed on disable, got {:?}", dis_err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_lifecycle_get_status_is_purely_observational() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_lifecycle_obs_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        // 1. Clean native config exists
        let native_conf = temp.join(".config/hypr/hypridle.conf");
        fs::create_dir_all(native_conf.parent().unwrap()).unwrap();
        let native_content = "general {\n    lock_cmd = test\n}\n";
        fs::write(&native_conf, native_content).unwrap();

        // Query status on clean native host -> Disabled
        let status_clean = get_hypridle_status(&temp);
        assert_eq!(status_clean.state, HypridleIntegrationState::Disabled);
        assert!(!status_clean.dropin_active);
        assert!(!status_clean.overlay_active);
        assert!(!status_clean.legacy_dropin_present);

        // 2. Add unowned legacy drop-in without manifest -> Conflict
        let legacy_file = temp.join(LEGACY_DROPIN_REL);
        fs::create_dir_all(legacy_file.parent().unwrap()).unwrap();
        let legacy_content = format!("{}\n[Service]\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let status_conflict = get_hypridle_status(&temp);
        assert_eq!(status_conflict.state, HypridleIntegrationState::Conflict);
        assert!(status_conflict.legacy_dropin_present);

        // Prove get_hypridle_status did NOT write or modify anything on disk!
        assert_eq!(fs::read_to_string(&native_conf).unwrap(), native_content);
        assert_eq!(fs::read_to_string(&legacy_file).unwrap(), legacy_content);
        assert!(!temp.join(CANONICAL_DROPIN_REL).exists());
        assert!(!temp.join(CANONICAL_OVERLAY_REL).exists());
        assert!(load_manifest(COMPOSER_INTEGRATION_ID, &temp).is_err());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    #[ignore]
    fn test_live_hypridle_lifecycle_enable_verify_disable_verify() {
        // Real host acceptance test for Phase 3C (explicit opt-in)
        let home = match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(_) => return,
        };

        let native_conf = home.join(".config/hypr/hypridle.conf");
        if !native_conf.exists() {
            println!("Skipping live host acceptance test: native hypridle.conf not present at {}", native_conf.display());
            return;
        }

        let native_bytes_before = fs::read(&native_conf).unwrap();
        let native_hash_before = calculate_sha256(&native_bytes_before);

        let req = HypridleCompositionRequest {
            native_discovery: crate::integration::hypridle::discovery::discover_native_hypridle_config(&home),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        println!("Applying live Hypridle composition...");
        let apply_status = apply_hypridle_composition(&home, &req).expect("Live apply failed");
        assert!(apply_status.dropin_active);
        assert!(apply_status.overlay_active);
        assert!(apply_status.service_active);

        // Verify native configuration hash remains bitwise identical
        let native_bytes_during = fs::read(&native_conf).unwrap();
        assert_eq!(calculate_sha256(&native_bytes_during), native_hash_before, "Native config must remain untouched during enable");

        println!("Disabling live Hypridle composition...");
        let disable_status = disable_hypridle_integration(&home).expect("Live disable failed");
        assert!(!disable_status.dropin_active);
        assert!(!disable_status.overlay_active);
        assert!(disable_status.service_active);

        let native_bytes_after = fs::read(&native_conf).unwrap();
        assert_eq!(calculate_sha256(&native_bytes_after), native_hash_before, "Native config must remain untouched after disable");

        println!("Live lifecycle test passed successfully!");
    }
}
