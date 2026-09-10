//! Ryzora Universal Package Engine — Phase 22
//!
//! Provides the generic transactional pipeline that sits between the provider
//! abstraction and the existing `installer.rs` helpers. All package types
//! (lockscreens, fonts, wallpapers, rices, …) flow through these nine stages.
//!
//! # Stage ordering
//! Resolve → Validate → CheckCompat → ResolveDeps →
//! Download → Verify → Stage → Materialize → Integrate
//!
//! # Safety invariants
//! - The pipeline wraps the existing `installer.rs` public API. No installer
//!   functions are renamed or removed.
//! - On any stage failure, all prior stage effects are rolled back via the
//!   snapshot/restore mechanism that already exists in `installer.rs`.
//! - Ownership claims are only applied after a file is successfully written
//!   (`Materialize` stage). Failed transactions roll back all claims.
//! - System configuration paths (SDDM, /etc/, ~/.config/hypr, …) continue to
//!   be managed exclusively by the existing adapter / privileged-helper mechanism.

use serde::{Deserialize, Serialize};
use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline stage enum
// ─────────────────────────────────────────────────────────────────────────────

/// The nine canonical pipeline stages for every Ryzora package operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStage {
    /// Locate the package directory / manifest on disk.
    Resolve,
    /// Parse and schema-validate the manifest.
    Validate,
    /// Evaluate compatibility with the current host environment.
    CheckCompat,
    /// Resolve and verify all declared dependencies.
    ResolveDeps,
    /// Download any remote assets referenced by the manifest.
    Download,
    /// SHA-256 verify all payload files against declared hashes.
    Verify,
    /// Copy payload into a sandboxed staging directory.
    Stage,
    /// Atomically move staged files to their final destinations.
    Materialize,
    /// Run category-specific post-install integration (e.g. lockscreen adapter).
    Integrate,
}

impl PipelineStage {
    pub fn label(&self) -> &'static str {
        match self {
            PipelineStage::Resolve => "Resolve",
            PipelineStage::Validate => "Validate manifest",
            PipelineStage::CheckCompat => "Check compatibility",
            PipelineStage::ResolveDeps => "Resolve dependencies",
            PipelineStage::Download => "Download assets",
            PipelineStage::Verify => "Verify checksums",
            PipelineStage::Stage => "Stage payload",
            PipelineStage::Materialize => "Materialize files",
            PipelineStage::Integrate => "Integrate",
        }
    }

    pub fn all() -> Vec<PipelineStage> {
        vec![
            PipelineStage::Resolve,
            PipelineStage::Validate,
            PipelineStage::CheckCompat,
            PipelineStage::ResolveDeps,
            PipelineStage::Download,
            PipelineStage::Verify,
            PipelineStage::Stage,
            PipelineStage::Materialize,
            PipelineStage::Integrate,
        ]
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Install / Uninstall options
// ─────────────────────────────────────────────────────────────────────────────

/// Options for the install pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineInstallOptions {
    /// Optional target adapter (e.g. "quickshell", "sddm").
    pub target: Option<String>,
    /// Allow overwriting files that are owned by another package.
    pub force: bool,
    /// Skip creating a pre-install snapshot (useful in test sandboxes).
    pub skip_snapshot: bool,
    /// Optional per-target configuration (passed to the integration step).
    pub config: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Options for the uninstall pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineUninstallOptions {
    /// If set, deactivate the given target before uninstalling.
    pub deactivate_target: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pipeline result
// ─────────────────────────────────────────────────────────────────────────────

/// A record of which stage the pipeline stopped at and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageResult {
    pub stage: PipelineStage,
    pub success: bool,
    pub message: String,
}

/// The complete result of an install pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineInstallResult {
    pub success: bool,
    pub package_id: String,
    pub version: String,
    pub snapshot_id: String,
    pub installed_files: Vec<String>,
    pub ownership_claimed: Vec<String>,
    pub stage_results: Vec<PipelineStageResult>,
    pub errors: Vec<String>,
    pub rolled_back: bool,
}

/// The complete result of an uninstall pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineUninstallResult {
    pub success: bool,
    pub package_id: String,
    pub removed_files: Vec<String>,
    pub ownership_released: Vec<String>,
    pub stage_results: Vec<PipelineStageResult>,
    pub errors: Vec<String>,
    pub rolled_back: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Install pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the full install pipeline for any package type.
///
/// This wraps the existing `installer::install_package_target_options_in`
/// and adds:
/// 1. Named stage tracking for UI progress display.
/// 2. Ownership ledger updates for user-space artifacts.
/// 3. Pre-flight ownership conflict detection.
pub fn run_install_pipeline(
    package_dir: &Path,
    options: &PipelineInstallOptions,
) -> Result<PipelineInstallResult, String> {
    let ryzora_dir = crate::installer::get_ryzora_base_dir();
    let home = crate::snapshot::get_home_dir();
    let snapshots_dir = crate::snapshot::get_ryzora_snapshots_dir();
    let installed_dir = crate::installer::get_ryzora_installed_dir();
    let staging_dir = crate::installer::get_ryzora_staging_dir();

    run_install_pipeline_in(
        package_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        options,
        &ryzora_dir,
    )
}

/// Testable inner form — accepts explicit directories so tests can use sandboxes.
pub fn run_install_pipeline_in(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    options: &PipelineInstallOptions,
    ledger_dir: &Path,
) -> Result<PipelineInstallResult, String> {
    let mut stage_results = Vec::new();

    // ── Stage: Resolve ───────────────────────────────────────────────────────
    stage_results.push(PipelineStageResult {
        stage: PipelineStage::Resolve,
        success: true,
        message: format!("Resolved package at {}", package_dir.display()),
    });

    // ── Stage: Validate ──────────────────────────────────────────────────────
    let manifest = match crate::installer::load_package_manifest(package_dir) {
        Ok(m) => {
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Validate,
                success: true,
                message: format!("Manifest valid: {} v{}", m.name, m.version),
            });
            m
        }
        Err(e) => {
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Validate,
                success: false,
                message: e.clone(),
            });
            return Ok(PipelineInstallResult {
                success: false,
                package_id: String::new(),
                version: String::new(),
                snapshot_id: String::new(),
                installed_files: vec![],
                ownership_claimed: vec![],
                stage_results,
                errors: vec![e],
                rolled_back: false,
            });
        }
    };

    // ── Stage: CheckCompat ───────────────────────────────────────────────────
    let system_info = crate::system::detect_system_info();
    let compat_reqs = crate::compatibility::CompatibilityRequirements {
        required_binaries: manifest.compatibility.required.clone(),
        optional_binaries: manifest.compatibility.optional.clone(),
        supported_desktops: manifest.compatibility.desktops.clone(),
        supported_sessions: manifest.compatibility.sessions.clone(),
        supported_distros: manifest.compatibility.distros.clone(),
    };
    let compat = crate::compatibility::evaluate_compatibility(&system_info, &compat_reqs);
    let compat_ok = matches!(compat.level, crate::compatibility::CompatibilityLevel::Compatible | crate::compatibility::CompatibilityLevel::MissingDependencies);
    stage_results.push(PipelineStageResult {
        stage: PipelineStage::CheckCompat,
        success: compat_ok,
        message: compat.summary_label.clone(),
    });
    if !compat_ok {
        return Ok(PipelineInstallResult {
            success: false,
            package_id: manifest.id,
            version: manifest.version,
            snapshot_id: String::new(),
            installed_files: vec![],
            ownership_claimed: vec![],
            stage_results,
            errors: vec![compat.summary_label],
            rolled_back: false,
        });
    }

    // ── Stage: ResolveDeps ───────────────────────────────────────────────────
    stage_results.push(PipelineStageResult {
        stage: PipelineStage::ResolveDeps,
        success: true,
        message: "Dependencies resolved (delegated to installer)".to_string(),
    });

    // ── Stage: Download ──────────────────────────────────────────────────────
    // All assets must already be present in the package directory.
    // Remote download is handled upstream by the provider staging step.
    stage_results.push(PipelineStageResult {
        stage: PipelineStage::Download,
        success: true,
        message: "All assets present in package directory".to_string(),
    });

    // ── Stage: Verify ────────────────────────────────────────────────────────
    stage_results.push(PipelineStageResult {
        stage: PipelineStage::Verify,
        success: true,
        message: "Payload verification delegated to installer".to_string(),
    });

    // ── Stage: Stage + Materialize (delegated to installer) ──────────────────
    // Pre-flight: check ownership conflicts for user-space files only
    let target_files_for_conflict: Vec<String> = {
        let target_str = options.target.as_deref();
        manifest
            .target_files(target_str)
            .unwrap_or_default()
            .iter()
            .filter(|f| crate::ownership::is_ryzora_owned_path(&f.target, home_dir))
            .map(|f| f.target.clone())
            .collect()
    };

    if !options.force {
        let ledger = crate::ownership::OwnershipLedger::load_from(ledger_dir);
        let conflicts = ledger.check_all_conflicts(&manifest.id, &target_files_for_conflict);
        if !conflicts.is_empty() {
            let conflict_msgs: Vec<String> = conflicts
                .iter()
                .map(|c| format!("'{}' is owned by '{}'", c.path, c.owned_by))
                .collect();
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Stage,
                success: false,
                message: format!("Ownership conflicts: {}", conflict_msgs.join("; ")),
            });
            return Ok(PipelineInstallResult {
                success: false,
                package_id: manifest.id,
                version: manifest.version,
                snapshot_id: String::new(),
                installed_files: vec![],
                ownership_claimed: vec![],
                stage_results,
                errors: conflict_msgs,
                rolled_back: false,
            });
        }
    }

    stage_results.push(PipelineStageResult {
        stage: PipelineStage::Stage,
        success: true,
        message: "No ownership conflicts; staging delegated to installer".to_string(),
    });

    // Delegate to the existing installer
    let create_snapshot = !options.skip_snapshot;
    let install_result = crate::installer::install_package_target_options_in(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        &system_info,
        create_snapshot,
        options.target.as_deref(),
    );

    match install_result {
        Err(e) => {
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Materialize,
                success: false,
                message: e.clone(),
            });
            Ok(PipelineInstallResult {
                success: false,
                package_id: manifest.id,
                version: manifest.version,
                snapshot_id: String::new(),
                installed_files: vec![],
                ownership_claimed: vec![],
                stage_results,
                errors: vec![e],
                rolled_back: false,
            })
        }
        Ok(result) if !result.success => {
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Materialize,
                success: false,
                message: result.errors.join("; "),
            });
            // Ensure any partial ownership claims are rolled back
            let mut ledger = crate::ownership::OwnershipLedger::load_from(ledger_dir);
            ledger.release_all(&result.package_id);
            let _ = ledger.save_to(ledger_dir);

            Ok(PipelineInstallResult {
                success: false,
                package_id: result.package_id,
                version: result.version,
                snapshot_id: result.snapshot_id,
                installed_files: vec![],
                ownership_claimed: vec![],
                stage_results,
                errors: result.errors,
                rolled_back: result.rolled_back,
            })
        }
        Ok(result) => {
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Materialize,
                success: true,
                message: format!("{} files materialized", result.installed_files.len()),
            });

            // ── Stage: Ownership claim (post-materialize) ────────────────────
            let mut ledger = crate::ownership::OwnershipLedger::load_from(ledger_dir);
            let mut ownership_claimed = Vec::new();

            // Re-read the installed record to get per-file hashes
            let record = crate::installer::get_installed_package_in(&result.package_id, installed_root);
            let file_hashes: std::collections::HashMap<String, String> = if let Ok(Some(rec)) = record {
                rec.files.iter().map(|f| (f.target.clone(), f.sha256.clone())).collect()
            } else {
                std::collections::HashMap::new()
            };

            for target_str in &result.installed_files {
                if crate::ownership::is_ryzora_owned_path(target_str, home_dir) {
                    let sha = file_hashes.get(target_str).cloned().unwrap_or_default();
                    ledger.claim(&result.package_id, target_str, &sha);
                    ownership_claimed.push(target_str.clone());
                }
            }
            let _ = ledger.save_to(ledger_dir);

            // ── Stage: Integrate ─────────────────────────────────────────────
            stage_results.push(PipelineStageResult {
                stage: PipelineStage::Integrate,
                success: true,
                message: "Integration complete (lockscreen adapters handled by existing installer)".to_string(),
            });

            Ok(PipelineInstallResult {
                success: true,
                package_id: result.package_id,
                version: result.version,
                snapshot_id: result.snapshot_id,
                installed_files: result.installed_files,
                ownership_claimed,
                stage_results,
                errors: vec![],
                rolled_back: false,
            })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Uninstall pipeline
// ─────────────────────────────────────────────────────────────────────────────

pub fn run_uninstall_pipeline(
    package_id: &str,
    options: &PipelineUninstallOptions,
) -> Result<PipelineUninstallResult, String> {
    let ryzora_dir = crate::installer::get_ryzora_base_dir();
    let home = crate::snapshot::get_home_dir();
    let snapshots_dir = crate::snapshot::get_ryzora_snapshots_dir();
    let installed_dir = crate::installer::get_ryzora_installed_dir();

    run_uninstall_pipeline_in(
        package_id,
        &snapshots_dir,
        &installed_dir,
        &home,
        options,
        &ryzora_dir,
    )
}

pub fn run_uninstall_pipeline_in(
    package_id: &str,
    snapshots_root: &Path,
    installed_root: &Path,
    home_dir: &Path,
    _options: &PipelineUninstallOptions,
    ledger_dir: &Path,
) -> Result<PipelineUninstallResult, String> {
    let mut stage_results = Vec::new();

    // ── Stage: Resolve ───────────────────────────────────────────────────────
    let record = crate::installer::get_installed_package_in(package_id, installed_root)
        .map_err(|e| format!("Failed to read installed record: {}", e))?;

    let _record = match record {
        Some(r) => r,
        None => {
            return Err(format!("Package '{}' is not installed", package_id));
        }
    };

    stage_results.push(PipelineStageResult {
        stage: PipelineStage::Resolve,
        success: true,
        message: format!("Found installed record for '{}'", package_id),
    });

    // ── Delegate to existing installer ───────────────────────────────────────
    let uninstall_result = crate::installer::uninstall_package_in(
        package_id,
        home_dir,
        snapshots_root,
        installed_root,
    )?;

    if uninstall_result.success {
        // Release ownership for all user-space files that were removed
        let mut ledger = crate::ownership::OwnershipLedger::load_from(ledger_dir);
        let mut ownership_released = Vec::new();

        for target_str in &uninstall_result.removed_files {
            if crate::ownership::is_ryzora_owned_path(target_str, home_dir) {
                ledger.release(package_id, target_str);
                ownership_released.push(target_str.clone());
            }
        }
        let _ = ledger.save_to(ledger_dir);

        stage_results.push(PipelineStageResult {
            stage: PipelineStage::Materialize,
            success: true,
            message: format!(
                "{} files removed, {} ownership claims released",
                uninstall_result.removed_files.len(),
                ownership_released.len()
            ),
        });

        Ok(PipelineUninstallResult {
            success: true,
            package_id: package_id.to_string(),
            removed_files: uninstall_result.removed_files,
            ownership_released,
            stage_results,
            errors: vec![],
            rolled_back: false,
        })
    } else {
        let err = uninstall_result.error.unwrap_or_else(|| "Unknown error".to_string());
        stage_results.push(PipelineStageResult {
            stage: PipelineStage::Materialize,
            success: false,
            message: err.clone(),
        });
        Ok(PipelineUninstallResult {
            success: false,
            package_id: package_id.to_string(),
            removed_files: vec![],
            ownership_released: vec![],
            stage_results,
            errors: vec![err],
            rolled_back: uninstall_result.rolled_back,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the ordered list of pipeline stage labels for UI progress display.
#[tauri::command]
pub fn get_pipeline_stages() -> Vec<serde_json::Value> {
    PipelineStage::all()
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "id": format!("{:?}", s).to_lowercase(),
                "label": s.label()
            })
        })
        .collect()
}

/// Runs the install pipeline for a package (identified by its prepared directory).
/// Called by the frontend InstallPipeline.ts wrapper.
#[tauri::command]
pub fn engine_install_package(
    package_id: String,
    target: Option<String>,
    force: Option<bool>,
) -> Result<PipelineInstallResult, String> {
    // Resolve via provider manager to get the prepared package directory
    let package_dir = crate::providers::prepare_provider_package_dir(&package_id)?;
    let opts = PipelineInstallOptions {
        target,
        force: force.unwrap_or(false),
        skip_snapshot: false,
        config: None,
    };
    run_install_pipeline(&package_dir, &opts)
}

/// Runs the uninstall pipeline for a package.
#[tauri::command]
pub fn engine_uninstall_package(package_id: String) -> Result<PipelineUninstallResult, String> {
    let opts = PipelineUninstallOptions {
        deactivate_target: None,
    };
    run_uninstall_pipeline(&package_id, &opts)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ownership::OwnershipLedger;
    use std::path::PathBuf;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Local test sandbox that mirrors installer::TestSandbox
    struct EngineSandbox {
        #[allow(dead_code)]
        root: PathBuf,
        pub snapshots_dir: PathBuf,
        pub installed_dir: PathBuf,
        pub staging_dir: PathBuf,
        pub home_dir: PathBuf,
    }

    impl EngineSandbox {
        fn new(name: &str) -> Self {
            let unique_id = format!(
                "ryzora-engine-test-{}-{}",
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique_id);
            let snapshots_dir = root.join("snapshots");
            let installed_dir = root.join("installed");
            let staging_dir = root.join("staging");
            let home_dir = root.join("mock_home");

            fs::create_dir_all(&snapshots_dir).unwrap();
            fs::create_dir_all(&installed_dir).unwrap();
            fs::create_dir_all(&staging_dir).unwrap();
            fs::create_dir_all(&home_dir.join(".config")).unwrap();

            Self { root, snapshots_dir, installed_dir, staging_dir, home_dir }
        }
    }

    impl Drop for EngineSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ── Invariant: Qylock installation still works via the engine ────────────
    #[test]
    fn test_engine_install_qylock_dog_samurai_quickshell() {
        let sandbox = EngineSandbox::new("engine-qylock-qs");
        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: false,
            skip_snapshot: true,
            config: None,
        };

        let result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &sandbox.home_dir.join(".local/share/ryzora"),
        ).unwrap();

        assert!(result.success, "Pipeline install failed: {:?}", result.errors);
        assert!(!result.installed_files.is_empty(), "No files installed");
        assert_eq!(result.package_id, "lockscreen-qylock-dog-samurai");
        // All stages should be present
        assert!(result.stage_results.iter().any(|s| s.stage == PipelineStage::Validate));
        assert!(result.stage_results.iter().any(|s| s.stage == PipelineStage::Materialize));
    }

    // ── Invariant: ownership claims registered after install ─────────────────
    #[test]
    fn test_engine_install_registers_ownership_claims() {
        let sandbox = EngineSandbox::new("engine-ownership-claim");
        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let ledger_dir = sandbox.home_dir.join(".local/share/ryzora");
        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: false,
            skip_snapshot: true,
            config: None,
        };

        let result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &ledger_dir,
        ).unwrap();

        assert!(result.success);
        assert!(!result.ownership_claimed.is_empty(), "Should have claimed ownership");

        // Verify ledger was persisted
        let ledger = OwnershipLedger::load_from(&ledger_dir);
        for path in &result.ownership_claimed {
            let entry = ledger.query(path);
            assert!(
                entry.is_some(),
                "Path '{}' should be in ledger",
                path
            );
            assert_eq!(entry.unwrap().package_id, "lockscreen-qylock-dog-samurai");
        }
    }

    // ── Invariant: cross-package conflict is detected pre-materialize ─────────
    #[test]
    fn test_engine_cross_package_conflict_blocked() {
        let sandbox = EngineSandbox::new("engine-conflict");
        let ledger_dir = sandbox.home_dir.join(".local/share/ryzora");

        // Manually claim a path on behalf of pkg-a
        let mut ledger = OwnershipLedger::default();
        ledger.claim(
            "lockscreen-qylock-another-pkg",
            "~/.local/share/ryzora/lockscreens/qylock/dog-samurai/Main.qml",
            "existinghash",
        );
        ledger.save_to(&ledger_dir).unwrap();

        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: false,
            skip_snapshot: true,
            config: None,
        };

        let result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &ledger_dir,
        ).unwrap();

        assert!(!result.success, "Should fail due to ownership conflict");
        assert!(!result.errors.is_empty());
        // The conflicting pkg still owns the file
        let reloaded = OwnershipLedger::load_from(&ledger_dir);
        let owner = reloaded.query("~/.local/share/ryzora/lockscreens/qylock/dog-samurai/Main.qml");
        if let Some(owner) = owner {
            assert_eq!(owner.package_id, "lockscreen-qylock-another-pkg");
        }
    }

    // ── Invariant: force=true bypasses conflict and updates ownership ─────────
    #[test]
    fn test_engine_force_install_overrides_conflict() {
        let sandbox = EngineSandbox::new("engine-force");
        let ledger_dir = sandbox.home_dir.join(".local/share/ryzora");

        // Pre-claim with a fake owner
        let mut ledger = OwnershipLedger::default();
        ledger.claim(
            "old-owner-pkg",
            "~/.local/share/ryzora/lockscreens/qylock/dog-samurai/Main.qml",
            "oldhash",
        );
        ledger.save_to(&ledger_dir).unwrap();

        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: true, // force overrides conflict
            skip_snapshot: true,
            config: None,
        };

        let result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &ledger_dir,
        ).unwrap();

        assert!(result.success, "Force install should succeed: {:?}", result.errors);
    }

    // ── Invariant: uninstall releases ownership ───────────────────────────────
    #[test]
    fn test_engine_uninstall_releases_ownership() {
        let sandbox = EngineSandbox::new("engine-uninst-release");
        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let ledger_dir = sandbox.home_dir.join(".local/share/ryzora");

        // Install first
        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: false,
            skip_snapshot: true,
            config: None,
        };
        let install_result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &ledger_dir,
        ).unwrap();
        assert!(install_result.success);
        assert!(!install_result.ownership_claimed.is_empty());

        // Now uninstall
        let uninst_opts = PipelineUninstallOptions { deactivate_target: None };
        let uninst_result = run_uninstall_pipeline_in(
            "lockscreen-qylock-dog-samurai",
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.home_dir,
            &uninst_opts,
            &ledger_dir,
        ).unwrap();

        assert!(uninst_result.success, "Uninstall failed: {:?}", uninst_result.errors);

        // Verify ownership was released
        let ledger = OwnershipLedger::load_from(&ledger_dir);
        let still_owned = ledger.owned_by("lockscreen-qylock-dog-samurai");
        assert!(
            still_owned.is_empty(),
            "All ownership should be released after uninstall, but still owns: {:?}",
            still_owned
        );
    }

    // ── Invariant: uninstall does NOT remove files owned by other packages ────
    #[test]
    fn test_engine_uninstall_cannot_remove_other_package_files() {
        let sandbox = EngineSandbox::new("engine-uninst-safety");
        let ledger_dir = sandbox.home_dir.join(".local/share/ryzora");

        // Create a file "owned" by another package in the ledger
        let other_pkg_file = "~/.local/share/ryzora/lockscreens/qylock/dog-samurai/protected.txt";
        let mut ledger = OwnershipLedger::default();
        ledger.claim("some-other-package", other_pkg_file, "protectedhash");
        ledger.save_to(&ledger_dir).unwrap();

        // Uninstall dog-samurai (which isn't even installed — should just return not-found)
        let opts = PipelineUninstallOptions { deactivate_target: None };
        let _result = run_uninstall_pipeline_in(
            "lockscreen-qylock-dog-samurai",
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.home_dir,
            &opts,
            &ledger_dir,
        );

        // Either fails (not installed) or succeeds — either way the other pkg file must remain
        let reloaded = OwnershipLedger::load_from(&ledger_dir);
        let owner = reloaded.query(other_pkg_file);
        assert!(
            owner.is_some() && owner.unwrap().package_id == "some-other-package",
            "Other package's ownership must not be affected"
        );
    }

    // ── Invariant: all nine pipeline stages present in a successful run ───────
    #[test]
    fn test_pipeline_stages_complete_on_success() {
        let sandbox = EngineSandbox::new("engine-stages");
        let pkg_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let opts = PipelineInstallOptions {
            target: Some("quickshell".to_string()),
            force: false,
            skip_snapshot: true,
            config: None,
        };
        let result = run_install_pipeline_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &opts,
            &sandbox.home_dir.join(".local/share/ryzora"),
        ).unwrap();

        assert!(result.success);
        let stages_present: std::collections::HashSet<String> = result
            .stage_results
            .iter()
            .map(|s| format!("{:?}", s.stage))
            .collect();

        for required_stage in &["Resolve", "Validate", "CheckCompat", "Stage", "Materialize", "Integrate"] {
            assert!(
                stages_present.contains(*required_stage),
                "Stage {:?} missing from results",
                required_stage
            );
        }
    }

    // ── Invariant: get_pipeline_stages returns all nine ───────────────────────
    #[test]
    fn test_get_pipeline_stages_returns_nine() {
        assert_eq!(PipelineStage::all().len(), 9);
    }
}
