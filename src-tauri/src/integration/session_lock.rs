//! Ryzora Reversible Integration — Phase 2: Session Lock (Hyprland / Hypridle)
//!
//! Implements the reversible session lock integration using the Phase 1 Integration Core:
//! - Purely additive: systemd user drop-in + composition overlay.
//! - Native configuration (`~/.config/hypr/hypridle.conf`) is dynamically sourced,
//!   never duplicated or modified.
//! - Only `lock_cmd` is overridden to `/usr/bin/hyprlock`.
//! - No shims, no wrappers in `~/.local/bin/hyprlock`, no snapshots, no Quickshell coupling.
//! - Baseline SHA-256 is an audit metadata field only; native config is user-owned and
//!   its modification never blocks rollback.
//! - Strict error handling: validates daemon-reload, service restart, and active state.
//! - Transactional rollback if service activation fails after artifact writing.
//! - Truly idempotent disable: repeated disables succeed cleanly.

use super::filesystem::calculate_sha256;
use super::lifecycle::{apply_integration, disable_integration, PlannedArtifact, PlannedIntegration};
use super::manifest::load_manifest;
use super::types::{ArtifactOwnershipPolicy, ArtifactType, DisableReport, IntegrationError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const FEATURE_ID: &str = "session-lock";
pub const INTEGRATION_ID: &str = "hyprland-session-lock";
pub const NATIVE_CONFIG_REL: &str = ".config/hypr/hypridle.conf";
pub const OVERLAY_CONFIG_REL: &str = ".config/ryzora/session-lock/hypridle.conf";
pub const DROPIN_REL: &str = ".config/systemd/user/hypridle.service.d/zz-ryzora-session-lock.conf";
pub const DROPIN_DIR_REL: &str = ".config/systemd/user/hypridle.service.d";

/// High-level discrete state of the session lock integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLockIntegrationState {
    /// Fully active: manifest exists, drop-in exists, and hypridle service is running.
    Active,
    /// Disabled: no manifest exists and no drop-in exists on disk.
    Disabled,
    /// Degraded: manifest is enabled but drop-in or service is inactive.
    Degraded,
    /// Conflict: unowned drop-in exists without an active Ryzora manifest.
    Conflict,
}

/// Status report for the Hyprland session lock integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLockStatus {
    pub state: SessionLockIntegrationState,
    pub enabled: bool,
    pub dropin_active: bool,
    pub service_active: bool,
    pub native_config_path: PathBuf,
    pub overlay_config_path: PathBuf,
    pub dropin_path: PathBuf,
    pub audit_native_config_hash: Option<String>,
}

/// Helper to reload daemon and restart hypridle.service in the user systemd manager,
/// validating that both commands succeed and the service transitions to active.
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
            if err_msg.is_empty() { String::from_utf8_lossy(&reload_out.stdout).trim().to_string() } else { err_msg }
        )));
    }

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
            if err_msg.is_empty() { String::from_utf8_lossy(&restart_out.stdout).trim().to_string() } else { err_msg }
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

/// Checks whether hypridle.service is currently active in user systemd.
pub fn is_hypridle_service_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "hypridle.service"])
        .output()
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

/// Checks whether any foreign/unowned drop-ins exist in hypridle.service.d/.
pub fn check_foreign_dropins(home: &Path) -> Result<Option<PathBuf>, IntegrationError> {
    let dropin_dir = home.join(DROPIN_DIR_REL);
    if !dropin_dir.exists() {
        return Ok(None);
    }

    let entries = fs::read_dir(&dropin_dir).map_err(|e| {
        IntegrationError::IoError(format!("Failed to read drop-in directory: {}", e))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name != "zz-ryzora-session-lock.conf" {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

/// Inspects current session lock integration status.
pub fn get_status(home: &Path) -> SessionLockStatus {
    let native_config = home.join(NATIVE_CONFIG_REL);
    let overlay_config = home.join(OVERLAY_CONFIG_REL);
    let dropin = home.join(DROPIN_REL);

    let manifest = load_manifest(INTEGRATION_ID, home).ok();
    let enabled = manifest.as_ref().map(|m| m.enabled).unwrap_or(false);
    let dropin_active = dropin.exists();
    let service_active = is_hypridle_service_active();
    let audit_hash = manifest.and_then(|m| m.metadata.get("audit_native_config_hash").cloned());

    let state = if enabled && dropin_active && service_active {
        SessionLockIntegrationState::Active
    } else if !enabled && !dropin_active {
        SessionLockIntegrationState::Disabled
    } else if enabled && (!dropin_active || !service_active) {
        SessionLockIntegrationState::Degraded
    } else {
        SessionLockIntegrationState::Conflict
    };

    SessionLockStatus {
        state,
        enabled,
        dropin_active,
        service_active,
        native_config_path: native_config,
        overlay_config_path: overlay_config,
        dropin_path: dropin,
        audit_native_config_hash: audit_hash,
    }
}

/// Enables the reversible session lock integration using Phase 1 Core.
///
/// Invariants:
/// - Pre-flight check: Rejects if foreign/user drop-ins exist.
/// - Sourcing: Overlay dynamically sources native config.
/// - Transactional Rollback: If service reload/verification fails, artifacts are safely removed.
pub fn enable_session_lock(home: &Path, restart_service: bool) -> Result<SessionLockStatus, IntegrationError> {
    let native_config = home.join(NATIVE_CONFIG_REL);
    let overlay_config = home.join(OVERLAY_CONFIG_REL);
    let dropin = home.join(DROPIN_REL);

    // 1. Verify native config exists
    if !native_config.exists() {
        return Err(IntegrationError::ConflictError(format!(
            "Native hypridle configuration not found at '{}'. Refusing to enable.",
            native_config.display()
        )));
    }

    // 2. Check for pre-existing unowned user drop-ins to protect user config
    if let Some(foreign) = check_foreign_dropins(home)? {
        return Err(IntegrationError::ConflictError(format!(
            "Foreign or user-created drop-in detected at '{}'. Ryzora refuses to overwrite or conflict with existing drop-ins.",
            foreign.display()
        )));
    }

    // 3. Record audit baseline hash (for informational tracking, NOT ownership gate)
    let native_bytes = fs::read(&native_config).map_err(|e| {
        IntegrationError::IoError(format!("Failed to read native config for baseline audit: {}", e))
    })?;
    let audit_native_hash = calculate_sha256(&native_bytes);

    let marker = format!("# Ryzora-Managed-Integration: {}", INTEGRATION_ID);

    // Artifact 1: Overlay hypridle configuration dynamically sourcing the native config
    let overlay_content = format!(
        "{}\n\
        source = {}\n\n\
        general {{\n\
            lock_cmd = /usr/bin/hyprlock\n\
        }}\n",
        marker,
        native_config.display()
    );

    // Artifact 2: Systemd user drop-in redirecting ExecStart
    let dropin_content = format!(
        "{}\n\
        [Service]\n\
        ExecStart=\n\
        ExecStart=/usr/bin/hypridle -c {}\n",
        marker,
        overlay_config.display()
    );

    let artifacts = vec![
        PlannedArtifact {
            path: overlay_config.clone(),
            content: overlay_content,
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker: marker.clone(),
            permissions: Some(0o644),
        },
        PlannedArtifact {
            path: dropin.clone(),
            content: dropin_content,
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            marker,
            permissions: Some(0o644),
        },
    ];

    let mut metadata = HashMap::new();
    metadata.insert("audit_native_config_hash".to_string(), audit_native_hash);
    metadata.insert("lock_target".to_string(), "/usr/bin/hyprlock".to_string());

    let plan = PlannedIntegration {
        feature_id: FEATURE_ID.to_string(),
        integration_id: INTEGRATION_ID.to_string(),
        version: 1,
        artifacts,
        metadata,
    };

    // Apply integration atomically through Phase 1 Core
    apply_integration(plan, home)?;

    // Reload service if requested (and in real environment)
    if restart_service {
        if let Err(restart_err) = reload_and_restart_hypridle() {
            // Service restart or verification failed: rollback artifacts atomically
            let _ = disable_integration(INTEGRATION_ID, home);
            return Err(IntegrationError::VerificationFailed(format!(
                "Failed to activate hypridle.service: {}. Transaction rolled back.",
                restart_err
            )));
        }
    }

    Ok(get_status(home))
}

/// Disables the reversible session lock integration using Phase 1 Core.
///
/// Invariants:
/// - Truly idempotent: repeated disables succeed cleanly.
/// - Native config is NEVER modified, deleted, or used to block rollback.
/// - Restores native systemd unit and verifies active state.
pub fn disable_session_lock(home: &Path, restart_service: bool) -> Result<DisableReport, IntegrationError> {
    let manifest_exists = load_manifest(INTEGRATION_ID, home).is_ok();

    let report = if manifest_exists {
        // Reversibly disable through Phase 1 Core
        disable_integration(INTEGRATION_ID, home)?
    } else {
        // Idempotent: already disabled, no artifacts to remove
        DisableReport {
            integration_id: INTEGRATION_ID.to_string(),
            removed_artifacts: Vec::new(),
            preserved_artifacts: Vec::new(),
            missing_artifacts: Vec::new(),
            removed_directories: Vec::new(),
            preserved_directories: Vec::new(),
            errors: Vec::new(),
            fully_reverted: true,
        }
    };

    // Reload and restart service to restore native systemd unit
    if restart_service {
        if let Err(restart_err) = reload_and_restart_hypridle() {
            return Err(IntegrationError::IoError(format!(
                "Artifacts were removed, but restoring native hypridle.service failed: {}",
                restart_err
            )));
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_home(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("ryzora_test_session_lock_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn setup_native_env(home: &Path) -> PathBuf {
        let hypr_dir = home.join(".config/hypr");
        fs::create_dir_all(&hypr_dir).unwrap();
        let native_conf = hypr_dir.join("hypridle.conf");
        fs::write(
            &native_conf,
            "general {\n\
                lock_cmd = pidof hyprlock || hyprlock\n\
                before_sleep_cmd = loginctl lock-session\n\
                after_sleep_cmd = hyprctl dispatch dpms on\n\
            }\n\
            listener {\n\
                timeout = 300\n\
                on-timeout = loginctl lock-session\n\
            }\n",
        )
        .unwrap();
        native_conf
    }

    #[test]
    fn test_enable_and_disable_lifecycle_clean() {
        let home = test_home("lifecycle_clean");
        let native_conf = setup_native_env(&home);
        let native_content_before = fs::read_to_string(&native_conf).unwrap();

        // 1. Initial status before enable
        let status_before = get_status(&home);
        assert_eq!(status_before.state, SessionLockIntegrationState::Disabled);
        assert!(!status_before.enabled);
        assert!(!status_before.dropin_active);

        // 2. Enable without triggering real systemctl restart in unit test
        let status_enabled = enable_session_lock(&home, false).unwrap();
        assert!(status_enabled.enabled);
        assert!(status_enabled.dropin_active);
        assert!(home.join(OVERLAY_CONFIG_REL).exists());
        assert!(home.join(DROPIN_REL).exists());

        // Native config must be completely untouched
        let native_content_after_enable = fs::read_to_string(&native_conf).unwrap();
        assert_eq!(native_content_before, native_content_after_enable);

        // 3. Disable
        let report = disable_session_lock(&home, false).unwrap();
        assert!(report.fully_reverted);
        assert_eq!(report.removed_artifacts.len(), 2);
        assert!(!home.join(OVERLAY_CONFIG_REL).exists());
        assert!(!home.join(DROPIN_REL).exists());

        // Manifest must be cleaned up
        assert!(load_manifest(INTEGRATION_ID, &home).is_err());

        // Native config must remain byte-for-byte identical
        let native_content_after_disable = fs::read_to_string(&native_conf).unwrap();
        assert_eq!(native_content_before, native_content_after_disable);

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_foreign_dropin_blocks_enable() {
        let home = test_home("foreign_dropin");
        setup_native_env(&home);

        // Simulate a pre-existing user drop-in
        let dropin_dir = home.join(DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();
        fs::write(dropin_dir.join("99-user-custom.conf"), "[Service]\nEnvironment=FOO=BAR\n").unwrap();

        let err = enable_session_lock(&home, false).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("Foreign or user-created drop-in detected"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        // Clean up
        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_native_config_change_does_not_block_disable() {
        let home = test_home("native_changed");
        let native_conf = setup_native_env(&home);

        enable_session_lock(&home, false).unwrap();

        // User / Dusky legitimately changes the native hypridle.conf while Ryzora is active
        fs::write(&native_conf, "# Dusky updated timeouts\nlistener { timeout = 120 }\n").unwrap();

        // Ryzora disable must STILL succeed and remove Ryzora's artifacts, preserving native config
        let report = disable_session_lock(&home, false).unwrap();
        assert!(report.fully_reverted);
        assert_eq!(report.removed_artifacts.len(), 2);

        // Native config must still have the user's updated content!
        let updated_native = fs::read_to_string(&native_conf).unwrap();
        assert!(updated_native.contains("Dusky updated timeouts"));

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_user_modified_ryzora_artifact_is_preserved_on_disable() {
        let home = test_home("user_modified");
        setup_native_env(&home);

        enable_session_lock(&home, false).unwrap();

        // User customized the overlay config
        let overlay_file = home.join(OVERLAY_CONFIG_REL);
        fs::write(&overlay_file, "# Ryzora-Managed-Integration: hyprland-session-lock\n# User custom edit\n").unwrap();

        let report = disable_session_lock(&home, false).unwrap();
        // Preserved, not fully reverted
        assert!(!report.fully_reverted);
        assert_eq!(report.preserved_artifacts.len(), 1);
        assert!(overlay_file.exists(), "User modified file must be preserved");

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_disable_twice_is_truly_idempotent() {
        let home = test_home("twice_idempotent");
        setup_native_env(&home);

        enable_session_lock(&home, false).unwrap();
        let rep1 = disable_session_lock(&home, false).unwrap();
        assert!(rep1.fully_reverted);

        // Disabling again succeeds cleanly (truly idempotent)
        let rep2 = disable_session_lock(&home, false).unwrap();
        assert!(rep2.fully_reverted);
        assert_eq!(rep2.removed_artifacts.len(), 0);

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn test_live_system_enable_verify_disable_revert() {
        let home_str = std::env::var("HOME").unwrap();
        let home = PathBuf::from(&home_str);

        let native_conf = home.join(NATIVE_CONFIG_REL);
        if !native_conf.exists() {
            eprintln!("Skipping live test: native config not present");
            return;
        }

        let baseline_hash = "9d117906fb409c9f707a7d1408dbf8c6006e0c804b83194e9ce9b041a1b8c402";
        let native_bytes_initial = fs::read(&native_conf).unwrap();
        assert_eq!(calculate_sha256(&native_bytes_initial), baseline_hash, "Native config must match baseline");

        // 1. Initial host status
        let initial_status = get_status(&home);
        assert_eq!(initial_status.state, SessionLockIntegrationState::Disabled);
        assert!(!initial_status.enabled);
        assert!(!initial_status.dropin_active);

        // 2. Enable integration on live system (with full reload/restart validation)
        let enabled_status = enable_session_lock(&home, true).unwrap();
        assert_eq!(enabled_status.state, SessionLockIntegrationState::Active);
        assert!(enabled_status.enabled);
        assert!(enabled_status.dropin_active);
        assert!(enabled_status.service_active);
        assert!(home.join(OVERLAY_CONFIG_REL).exists());
        assert!(home.join(DROPIN_REL).exists());

        // Verify systemd dropin is active in systemctl cat
        let cat_out = std::process::Command::new("systemctl")
            .args(["--user", "cat", "hypridle.service"])
            .output()
            .unwrap();
        let cat_str = String::from_utf8_lossy(&cat_out.stdout);
        assert!(cat_str.contains("zz-ryzora-session-lock.conf"), "Systemd must have loaded the Ryzora drop-in");

        // Verify hypridle service is running
        assert!(is_hypridle_service_active(), "hypridle.service must remain active after reload");

        // Native config must be completely untouched
        let native_bytes_during = fs::read(&native_conf).unwrap();
        assert_eq!(calculate_sha256(&native_bytes_during), baseline_hash, "Native config must remain byte-for-byte identical while enabled");

        // 3. Disable integration on live system
        let report = disable_session_lock(&home, true).unwrap();
        assert!(report.fully_reverted, "Disable must fully revert on clean state");
        assert_eq!(report.removed_artifacts.len(), 2);
        assert!(!home.join(OVERLAY_CONFIG_REL).exists(), "Overlay config must be removed");
        assert!(!home.join(DROPIN_REL).exists(), "Dropin config must be removed");

        // Verify systemd dropin is gone
        let cat_out_after = std::process::Command::new("systemctl")
            .args(["--user", "cat", "hypridle.service"])
            .output()
            .unwrap();
        let cat_str_after = String::from_utf8_lossy(&cat_out_after.stdout);
        assert!(!cat_str_after.contains("zz-ryzora-session-lock.conf"), "Systemd drop-in must be completely removed");

        // Verify service returned to native unit and is active
        assert!(is_hypridle_service_active(), "hypridle.service must be active and running native unit");

        // Final check: native config remains completely identical
        let native_bytes_final = fs::read(&native_conf).unwrap();
        assert_eq!(calculate_sha256(&native_bytes_final), baseline_hash, "Native config must remain byte-for-byte identical after disable");

        // 4. Test idempotence on live system
        let rep_repeat = disable_session_lock(&home, true).unwrap();
        assert!(rep_repeat.fully_reverted);
    }
}
