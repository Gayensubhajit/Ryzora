//! Ryzora Reversible Integration Core — Phase 1
//!
//! Provides the foundational, provider-agnostic integration engine:
//! - Additive, non-destructive integration touchpoints.
//! - Cryptographic hash (SHA-256) and ownership marker verification.
//! - Reversible rollback without snapshots.
//! - User modification preservation and idempotent cleanup.

pub mod filesystem;
pub mod lifecycle;
pub mod manifest;
pub mod types;
pub mod verifier;
pub mod session_lock;
pub mod hypridle;

pub use filesystem::*;
pub use lifecycle::*;
pub use manifest::*;
pub use types::*;
pub use verifier::*;

use std::path::PathBuf;

fn get_effective_home() -> PathBuf {
    crate::snapshot::get_home_dir()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri IPC Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Lists all registered integration manifests.
#[tauri::command]
pub fn integration_list() -> Result<Vec<IntegrationManifest>, String> {
    let home = get_effective_home();
    manifest::list_manifests(&home).map_err(|e| e.to_string())
}

/// Retrieves a specific integration manifest by its unique ID.
#[tauri::command]
pub fn integration_get(integration_id: String) -> Result<IntegrationManifest, String> {
    let home = get_effective_home();
    manifest::load_manifest(&integration_id, &home).map_err(|e| e.to_string())
}

/// Verifies the current health and on-disk state of an integration.
#[tauri::command]
pub fn integration_verify(
    integration_id: String,
) -> Result<Vec<ArtifactVerificationResult>, String> {
    let home = get_effective_home();
    lifecycle::verify_integration(&integration_id, &home).map_err(|e| e.to_string())
}

/// Disables an integration, strictly removing verified Ryzora artifacts and preserving user edits.
/// Requires an explicit integration_id; never accepts arbitrary file paths from the frontend.
#[tauri::command]
pub fn integration_disable(integration_id: String) -> Result<DisableReport, String> {
    let home = get_effective_home();
    lifecycle::disable_integration(&integration_id, &home).map_err(|e| e.to_string())
}

/// Gets the live status of the Hyprland session lock integration.
#[tauri::command]
pub fn session_lock_get_status() -> session_lock::SessionLockStatus {
    let home = get_effective_home();
    session_lock::get_status(&home)
}

/// Enables the Hyprland session lock integration.
#[tauri::command]
pub fn session_lock_enable() -> Result<session_lock::SessionLockStatus, String> {
    let home = get_effective_home();
    session_lock::enable_session_lock(&home, true).map_err(|e| e.to_string())
}

/// Disables the Hyprland session lock integration.
#[tauri::command]
pub fn session_lock_disable() -> Result<DisableReport, String> {
    let home = get_effective_home();
    session_lock::disable_session_lock(&home, true).map_err(|e| e.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Comprehensive Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn test_home(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ryzora_integ_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn cleanup(p: &Path) {
        let _ = fs::remove_dir_all(p);
    }

    #[test]
    fn test_plan_rejects_unapproved_roots() {
        let home = test_home("unapproved");
        let forbidden = home.join(".bashrc");

        let planned_arts = vec![PlannedArtifact {
            path: forbidden,
            content: "export FOO=1".to_string(),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker: "# Ryzora-Test".to_string(),
            permissions: None,
        }];

        let res = plan_integration(
            "test-feat",
            "test-unapproved",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        );

        assert!(res.is_err(), "Must reject writes to unapproved roots");
        let err = res.unwrap_err().to_string();
        assert!(err.contains("outside approved"), "Error must mention approved roots: {}", err);

        cleanup(&home);
    }

    #[test]
    fn test_plan_rejects_preexisting_unowned_file() {
        let home = test_home("preexisting");
        let target_dir = home.join(".config/ryzora");
        fs::create_dir_all(&target_dir).unwrap();
        let target_file = target_dir.join("my_custom.conf");
        fs::write(&target_file, "# User preexisting config without marker").unwrap();

        let planned_arts = vec![PlannedArtifact {
            path: target_file,
            content: "managed content".to_string(),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker: "# Ryzora-Managed-Integration: test-conflict".to_string(),
            permissions: None,
        }];

        let res = plan_integration(
            "test-feat",
            "test-conflict",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        );

        assert!(res.is_err(), "Must refuse to overwrite existing unowned file");
        let err = res.unwrap_err().to_string();
        assert!(err.contains("Refusing to overwrite"), "Error: {}", err);

        cleanup(&home);
    }

    #[test]
    fn test_apply_and_verify_clean_integration() {
        let home = test_home("apply-verify");
        let dropin_dir = home.join(".config/systemd/user/test.service.d");
        let dropin_file = dropin_dir.join("ryzora.conf");

        let marker = "# Ryzora-Managed-Integration: test-clean".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: dropin_file.clone(),
            content: format!("{}
[Service]
ExecStart=/usr/bin/test", marker),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker: marker.clone(),
            permissions: Some(0o644),
        }];

        let plan = plan_integration(
            "test-feat",
            "test-clean",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        let manifest = apply_integration(plan, &home).unwrap();
        assert!(dropin_file.exists());
        assert!(manifest.created_directories.contains(&dropin_dir));

        // Verify status
        let verification = verify_integration("test-clean", &home).unwrap();
        assert_eq!(verification.len(), 1);
        assert_eq!(verification[0].status, ArtifactVerificationStatus::VerifiedRyzoraOwned);

        cleanup(&home);
    }

    #[test]
    fn test_disable_clean_removes_artifacts_and_created_empty_dirs() {
        let home = test_home("disable-clean");
        let dropin_dir = home.join(".config/systemd/user/cleanup.service.d");
        let dropin_file = dropin_dir.join("ryzora.conf");

        let marker = "# Ryzora-Managed-Integration: test-cleanup".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: dropin_file.clone(),
            content: format!("{}
[Service]
ExecStart=/bin/true", marker),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: None,
        }];

        let plan = plan_integration(
            "test-feat",
            "test-cleanup",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        apply_integration(plan, &home).unwrap();
        assert!(dropin_file.exists());
        assert!(dropin_dir.exists());

        let report = disable_integration("test-cleanup", &home).unwrap();
        assert!(report.fully_reverted);
        assert_eq!(report.removed_artifacts.len(), 1);
        assert!(!dropin_file.exists(), "Artifact file must be removed");
        assert!(!dropin_dir.exists(), "Created empty parent dir must be cleaned up");

        // Manifest must be deleted on clean revert
        assert!(load_manifest("test-cleanup", &home).is_err());

        cleanup(&home);
    }

    #[test]
    fn test_disable_preserves_externally_modified_file() {
        let home = test_home("preserve-modified");
        let target_file = home.join(".config/ryzora/user_tinkered.conf");

        let marker = "# Ryzora-Managed-Integration: test-preserve".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: target_file.clone(),
            content: format!("{}
initial_setting=10", marker),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: None,
        }];

        let plan = plan_integration(
            "test-feat",
            "test-preserve",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        apply_integration(plan, &home).unwrap();

        // User edits the file externally
        fs::write(
            &target_file,
            "# Ryzora-Managed-Integration: test-preserve
user_customized=999
",
        )
        .unwrap();

        let report = disable_integration("test-preserve", &home).unwrap();
        assert!(!report.fully_reverted, "Must not report fully_reverted when files are preserved");
        assert_eq!(report.preserved_artifacts.len(), 1);
        assert!(report.removed_artifacts.is_empty());
        assert!(target_file.exists(), "User-modified file must NOT be deleted!");

        let content = fs::read_to_string(&target_file).unwrap();
        assert!(content.contains("user_customized=999"));

        cleanup(&home);
    }

    #[test]
    fn test_disable_preserves_non_ryzora_created_dir_even_if_empty() {
        let home = test_home("preserve-dir");
        // Pre-create directory before Ryzora touches it
        let pre_existing_dir = home.join(".config/systemd/user/preexisting.service.d");
        fs::create_dir_all(&pre_existing_dir).unwrap();

        let target_file = pre_existing_dir.join("ryzora.conf");
        let marker = "# Ryzora-Managed-Integration: test-dir-preserve".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: target_file.clone(),
            content: format!("{}
[Service]", marker),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: None,
        }];

        let plan = plan_integration(
            "test-feat",
            "test-dir-preserve",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        let manifest = apply_integration(plan, &home).unwrap();
        // Since preexisting.service.d already existed, it was NOT created by Ryzora
        assert!(!manifest.created_directories.contains(&pre_existing_dir));

        let report = disable_integration("test-dir-preserve", &home).unwrap();
        assert!(report.fully_reverted);
        assert!(!target_file.exists(), "File was removed");
        assert!(
            pre_existing_dir.exists(),
            "Pre-existing directory must be preserved even though it is now empty!"
        );

        cleanup(&home);
    }

    #[test]
    fn test_disable_missing_artifact_idempotence() {
        let home = test_home("missing-idempotence");
        let target_file = home.join(".local/bin/my_shim");

        let marker = "# Ryzora-Managed-Integration: test-missing".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: target_file.clone(),
            content: format!("#!/bin/bash
{}
exit 0", marker),
            artifact_type: ArtifactType::WrapperShim,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: Some(0o755),
        }];

        let plan = plan_integration(
            "test-feat",
            "test-missing",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        apply_integration(plan, &home).unwrap();

        // User or external process deleted the shim manually
        fs::remove_file(&target_file).unwrap();

        let report = disable_integration("test-missing", &home).unwrap();
        assert!(report.fully_reverted, "Missing file should be treated as cleanly removed");
        assert_eq!(report.missing_artifacts.len(), 1);

        cleanup(&home);
    }

    #[test]
    fn test_disable_twice_idempotence() {
        let home = test_home("double-disable");
        let target_file = home.join(".config/ryzora/once.conf");

        let marker = "# Ryzora-Managed-Integration: test-twice".to_string();
        let planned_arts = vec![PlannedArtifact {
            path: target_file,
            content: format!("{}
key=value", marker),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: None,
        }];

        let plan = plan_integration(
            "test-feat",
            "test-twice",
            1,
            planned_arts,
            std::collections::HashMap::new(),
            &home,
        )
        .unwrap();

        apply_integration(plan, &home).unwrap();

        // First disable
        let rep1 = disable_integration("test-twice", &home).unwrap();
        assert!(rep1.fully_reverted);

        // Second disable
        let rep2 = disable_integration("test-twice", &home);
        assert!(rep2.is_err(), "Second disable must report manifest not found");

        cleanup(&home);
    }

    #[test]
    fn test_symlink_security_does_not_traverse() {
        let home = test_home("symlink-security");
        let secret_file = home.join("secret_user_doc.txt");
        fs::write(&secret_file, "IMPORTANT USER DATA DO NOT DELETE").unwrap();

        // Symlink pointing to the secret file
        let symlink_path = home.join(".local/bin/malicious_link");
        fs::create_dir_all(symlink_path.parent().unwrap()).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&secret_file, &symlink_path).unwrap();

        let probe = IntegrationArtifact {
            path: symlink_path.clone(),
            artifact_type: ArtifactType::WrapperShim,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256: "fakehash".to_string(),
            marker: "# Ryzora-Managed".to_string(),
            created_at: 0,
            permissions: None,
        };

        let ver = verify_artifact(&probe, &home);
        // Because the secret file lacks the marker, it is ForeignOrUnowned
        assert_ne!(ver.status, ArtifactVerificationStatus::VerifiedRyzoraOwned);

        // Now test safe_remove_artifact_file on a symlink:
        // Even if explicitly removed, it must remove the symlink, NOT the target file!
        let _ = safe_remove_artifact_file(&symlink_path, &home);
        assert!(!symlink_path.exists(), "Symlink was removed");
        assert!(secret_file.exists(), "Target secret file was NOT deleted!");
        let secret_content = fs::read_to_string(&secret_file).unwrap();
        assert_eq!(secret_content, "IMPORTANT USER DATA DO NOT DELETE");

        cleanup(&home);
    }

    #[test]
    fn test_manifest_corruption_resilience() {
        let home = test_home("corrupt-manifest");
        let storage = get_manifest_storage_dir(&home);
        fs::create_dir_all(&storage).unwrap();
        let corrupt_file = storage.join("corrupted_one.json");
        fs::write(&corrupt_file, "{ invalid json ---").unwrap();

        let res = load_manifest("corrupted_one", &home);
        assert!(res.is_err());
        match res.unwrap_err() {
            IntegrationError::ManifestCorrupted(_) => {}
            other => panic!("Expected ManifestCorrupted, got: {:?}", other),
        }

        // list_manifests should not panic when a corrupted manifest is present
        let list = list_manifests(&home).unwrap();
        assert_eq!(list.len(), 0);

        cleanup(&home);
    }
}
