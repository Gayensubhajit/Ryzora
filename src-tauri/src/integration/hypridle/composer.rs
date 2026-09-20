//! Ryzora Reversible Integration Core — Shared Hypridle Composer Engine
//!
//! Architectural Boundary:
//! - Pure in-memory planning engine: zero filesystem mutations, zero systemctl calls.
//! - Authoritative drop-in directory inspection: every drop-in entry is checked against its manifest
//!   (canonical -> `hyprland-hypridle`, legacy -> `hyprland-session-lock`, foreign -> Conflict, symlink -> SecurityViolation).
//! - Authoritative canonical overlay ownership verification via Phase 1 verifier.
//! - Option C: Strictly collision-free additive listener model (refuses duplicate or conflicting timeouts).
//! - Consumes validated high-level intents (`IdleAction`, `IdleResumeAction`) with parameterized,
//!   sanitized host capabilities rather than hardcoded device names or arbitrary shell strings.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{
    ComposedHypridlePlan, HypridleCompositionRequest, HypridleParseStatus,
};
use crate::integration::manifest::load_manifest;
use crate::integration::types::{ArtifactVerificationStatus, IntegrationError};
use crate::integration::verifier::verify_artifact;

pub const COMPOSER_FEATURE_ID: &str = "hypridle";
pub const COMPOSER_INTEGRATION_ID: &str = "hyprland-hypridle";
pub const LEGACY_DROPIN_REL: &str = ".config/systemd/user/hypridle.service.d/zz-ryzora-session-lock.conf";
pub const CANONICAL_DROPIN_REL: &str = ".config/systemd/user/hypridle.service.d/zz-ryzora-hypridle.conf";
pub const CANONICAL_DROPIN_DIR_REL: &str = ".config/systemd/user/hypridle.service.d";
pub const CANONICAL_OVERLAY_REL: &str = ".config/ryzora/hypridle/hypridle.conf";
pub const CANONICAL_MARKER: &str = "# Ryzora-Managed-Integration: hyprland-hypridle";
pub const LEGACY_MARKER: &str = "# Ryzora-Managed-Integration: hyprland-session-lock";

/// Checks if a string is a safe device identifier (alphanumeric, underscore, hyphen, colon).
pub fn is_safe_device_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':')
}

/// Inspects every drop-in in the hypridle.service.d directory.
///
/// Implements one authoritative ownership decision per artifact:
/// - Symlink (any symlink) -> SecurityViolation
/// - Directory -> ConflictError
/// - Canonical drop-in (`zz-ryzora-hypridle.conf`) -> verified against canonical manifest (`hyprland-hypridle`)
/// - Legacy drop-in (`zz-ryzora-session-lock.conf`) -> verified against Phase 2 manifest (`hyprland-session-lock`)
/// - Any other file -> foreign / ConflictError
///
/// Returns `Ok(Some(legacy_path))` if a verified Ryzora-owned legacy drop-in is present for migration.
/// Returns `Ok(None)` if no legacy drop-in is present.
pub fn inspect_dropin_directory(home: &Path) -> Result<Option<PathBuf>, IntegrationError> {
    let dropin_dir = home.join(CANONICAL_DROPIN_DIR_REL);
    if !dropin_dir.exists() {
        return Ok(None);
    }

    let entries = fs::read_dir(&dropin_dir).map_err(|e| {
        IntegrationError::IoError(format!(
            "Failed to read drop-in directory '{}': {}",
            dropin_dir.display(),
            e
        ))
    })?;

    let mut legacy_to_migrate: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                return Err(IntegrationError::IoError(format!(
                    "Failed to query metadata for drop-in entry '{}': {}",
                    path.display(),
                    e
                )));
            }
        };

        if meta.file_type().is_symlink() {
            return Err(IntegrationError::SecurityViolation(format!(
                "Untrusted symlink detected in drop-in directory at '{}'. Untrusted symlinks are prohibited.",
                path.display()
            )));
        }

        if meta.is_dir() {
            return Err(IntegrationError::ConflictError(format!(
                "Expected file artifact in drop-in directory at '{}', but found directory.",
                path.display()
            )));
        }

        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match file_name {
            "zz-ryzora-hypridle.conf" => {
                // Canonical drop-in: authoritatively verify against canonical manifest
                let manifest = match load_manifest(COMPOSER_INTEGRATION_ID, home) {
                    Ok(m) => m,
                    Err(_) => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Canonical drop-in exists at '{}' without an active integration manifest. Refusing to overwrite foreign/unowned file.",
                            path.display()
                        )));
                    }
                };

                let artifact = manifest
                    .artifacts
                    .iter()
                    .find(|a| a.path == path)
                    .ok_or_else(|| {
                        IntegrationError::ConflictError(format!(
                            "Canonical drop-in at '{}' exists but is not registered in active manifest '{}'. Refusing to overwrite.",
                            path.display(),
                            COMPOSER_INTEGRATION_ID
                        ))
                    })?;

                let verification = verify_artifact(artifact, home);
                match verification.status {
                    ArtifactVerificationStatus::VerifiedRyzoraOwned => {
                        // Permitted to replace existing Ryzora-owned canonical drop-in
                    }
                    ArtifactVerificationStatus::ModifiedByUser => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Canonical drop-in at '{}' was modified externally by user. Refusing to overwrite.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::SymlinkTargetUntrusted => {
                        return Err(IntegrationError::SecurityViolation(format!(
                            "Canonical drop-in at '{}' is an untrusted symlink.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::ForeignOrUnowned => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Canonical drop-in at '{}' failed ownership verification. Refusing to overwrite.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::Missing => {
                        // File was reported missing by verifier, safe to create
                    }
                }
            }
            "zz-ryzora-session-lock.conf" => {
                // Legacy Phase 2 drop-in: authoritatively verify against Phase 2 manifest
                let manifest = match load_manifest("hyprland-session-lock", home) {
                    Ok(m) => m,
                    Err(_) => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Legacy drop-in exists at '{}' but no active 'hyprland-session-lock' manifest was found. Refusing migration.",
                            path.display()
                        )));
                    }
                };

                let artifact = manifest
                    .artifacts
                    .iter()
                    .find(|a| a.path == path)
                    .ok_or_else(|| {
                        IntegrationError::ConflictError(format!(
                            "Legacy drop-in at '{}' is not registered in Phase 2 manifest. Refusing migration.",
                            path.display()
                        ))
                    })?;

                let verification = verify_artifact(artifact, home);
                match verification.status {
                    ArtifactVerificationStatus::VerifiedRyzoraOwned => {
                        legacy_to_migrate = Some(path.clone());
                    }
                    ArtifactVerificationStatus::ModifiedByUser => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Legacy drop-in at '{}' was modified externally by user. Refusing automatic migration.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::SymlinkTargetUntrusted => {
                        return Err(IntegrationError::SecurityViolation(format!(
                            "Legacy drop-in at '{}' is an untrusted symlink. Refusing migration.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::ForeignOrUnowned => {
                        return Err(IntegrationError::ConflictError(format!(
                            "Legacy drop-in at '{}' failed ownership verification. Refusing migration.",
                            path.display()
                        )));
                    }
                    ArtifactVerificationStatus::Missing => {
                        // File was reported missing by verifier
                    }
                }
            }
            _ => {
                // Any other file is foreign / unowned
                return Err(IntegrationError::ConflictError(format!(
                    "Foreign or user-created drop-in detected at '{}'. Ryzora refuses to overwrite or conflict with existing drop-ins.",
                    path.display()
                )));
            }
        }
    }

    Ok(legacy_to_migrate)
}

/// Checks if an existing canonical overlay artifact is safe to create or replace using the Phase 1 verifier.
pub fn check_canonical_artifact_permission(
    target_path: &Path,
    home: &Path,
    integration_id: &str,
) -> Result<(), IntegrationError> {
    let meta = match fs::symlink_metadata(target_path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(());
        }
        Err(e) => {
            return Err(IntegrationError::IoError(format!(
                "Failed to read metadata for '{}': {}",
                target_path.display(),
                e
            )));
        }
    };

    if meta.file_type().is_symlink() {
        return Err(IntegrationError::SecurityViolation(format!(
            "Canonical artifact at '{}' is an untrusted symlink. Refusing to touch.",
            target_path.display()
        )));
    }

    if meta.is_dir() {
        return Err(IntegrationError::ConflictError(format!(
            "Expected file artifact at '{}' but found directory.",
            target_path.display()
        )));
    }

    // Target exists on disk. Verify ownership via active manifest.
    let manifest = match load_manifest(integration_id, home) {
        Ok(m) => m,
        Err(_) => {
            return Err(IntegrationError::ConflictError(format!(
                "Canonical artifact exists at '{}' without an active integration manifest. Refusing to overwrite foreign/unowned file.",
                target_path.display()
            )));
        }
    };

    let artifact = manifest
        .artifacts
        .iter()
        .find(|a| a.path == target_path)
        .ok_or_else(|| {
            IntegrationError::ConflictError(format!(
                "Canonical artifact at '{}' exists but is not registered in active manifest '{}'. Refusing to overwrite.",
                target_path.display(),
                integration_id
            ))
        })?;

    let verification = verify_artifact(artifact, home);
    match verification.status {
        ArtifactVerificationStatus::VerifiedRyzoraOwned => Ok(()),
        ArtifactVerificationStatus::ModifiedByUser => Err(IntegrationError::ConflictError(format!(
            "Canonical artifact at '{}' was modified externally by user. Refusing to overwrite.",
            target_path.display()
        ))),
        ArtifactVerificationStatus::SymlinkTargetUntrusted => Err(IntegrationError::SecurityViolation(format!(
            "Canonical artifact at '{}' is an untrusted symlink.",
            target_path.display()
        ))),
        ArtifactVerificationStatus::ForeignOrUnowned => Err(IntegrationError::ConflictError(format!(
            "Canonical artifact at '{}' failed ownership verification. Refusing to overwrite.",
            target_path.display()
        ))),
        ArtifactVerificationStatus::Missing => Ok(()),
    }
}

/// Composes the unified Hypridle overlay and systemd drop-in from active feature intents.
///
/// Invariants:
/// - Pure planning engine: zero filesystem mutations, zero systemctl calls.
/// - Teardown state: If both features are inactive, returns Ok(None).
/// - Refuses composition if native discovery is Invalid.
/// - Refuses composition if native discovery has duplicate listener timeouts.
/// - Refuses composition if native discovery is missing or unreadable.
/// - Authoritatively inspects drop-in directory: each drop-in verified against its specific manifest.
/// - Reuses Phase 1 verifier/manifest to check canonical overlay and legacy drop-in migration.
/// - Refuses composition if requested idle timeouts collide with native listeners or each other.
/// - Consumes validated feature intent (IdleAction / IdleResumeAction) with sanitized host capabilities.
pub fn compose_hypridle_plan(
    home: &Path,
    req: &HypridleCompositionRequest,
) -> Result<Option<ComposedHypridlePlan>, IntegrationError> {
    // 1. Both features inactive -> no Ryzora integration needed
    if !req.session_lock_active && !req.idle_management_active {
        return Ok(None);
    }

    // 2. Refuse if native discovery is Invalid
    if req.native_discovery.parse_status == HypridleParseStatus::Invalid {
        return Err(IntegrationError::ConflictError(
            "Native Hypridle configuration has invalid or ambiguous syntax. Refusing to compose overlay.".to_string(),
        ));
    }

    // 3. Refuse if native config contains duplicate listener timeouts
    if !req.native_discovery.duplicate_listener_timeouts.is_empty() {
        return Err(IntegrationError::ConflictError(format!(
            "Native Hypridle configuration contains duplicate listener timeouts ({:?}). Refusing to compose overlay.",
            req.native_discovery.duplicate_listener_timeouts
        )));
    }

    // 4. Refuse if native config is missing or unreadable
    if !req.native_discovery.exists || !req.native_discovery.readable {
        return Err(IntegrationError::ConflictError(
            "Native Hypridle configuration is missing or unreadable. Refusing to compose overlay.".to_string(),
        ));
    }

    // 5. Authoritative inspection of drop-in directory: verifies canonical, legacy, foreign, symlinks
    let legacy_dropin_to_migrate = inspect_dropin_directory(home)?;

    // 6. Check canonical overlay permission using Phase 1 ownership verifier
    let overlay_path = home.join(CANONICAL_OVERLAY_REL);
    let dropin_path = home.join(CANONICAL_DROPIN_REL);
    check_canonical_artifact_permission(&overlay_path, home, COMPOSER_INTEGRATION_ID)?;

    // 7. Validate requested idle intents (Option C: additive-only, strictly collision-free)
    if req.idle_management_active {
        let native_timeouts: HashSet<u64> = req
            .native_discovery
            .listeners
            .iter()
            .map(|l| l.timeout)
            .collect();

        let mut req_timeouts = HashSet::new();
        for intent in &req.idle_intents {
            if intent.timeout == 0 {
                return Err(IntegrationError::ConflictError(
                    "Requested idle listener timeout must be greater than 0s.".to_string(),
                ));
            }

            // Validate device identifiers in intent to prevent command injection
            if let Some(dev) = intent.action.device() {
                if !is_safe_device_identifier(dev) {
                    return Err(IntegrationError::ConflictError(format!(
                        "Invalid device identifier '{}': must contain only alphanumeric characters, underscores, hyphens, and colons.",
                        dev
                    )));
                }
            }
            if let Some(ref resume) = intent.resume_action {
                if let Some(dev) = resume.device() {
                    if !is_safe_device_identifier(dev) {
                        return Err(IntegrationError::ConflictError(format!(
                            "Invalid resume device identifier '{}': must contain only alphanumeric characters, underscores, hyphens, and colons.",
                            dev
                        )));
                    }
                }
            }

            // Check collision with native listeners
            if native_timeouts.contains(&intent.timeout) {
                return Err(IntegrationError::ConflictError(format!(
                    "Requested idle listener timeout {}s collides with native listener timeout. Refusing to compose.",
                    intent.timeout
                )));
            }

            // Check collision among requested listeners
            if !req_timeouts.insert(intent.timeout) {
                return Err(IntegrationError::ConflictError(format!(
                    "Requested idle listeners contain duplicate timeout {}s. Refusing to compose.",
                    intent.timeout
                )));
            }
        }
    }

    // 8. Compose overlay content dynamically sourcing native config
    let mut overlay_content = String::new();
    overlay_content.push_str(CANONICAL_MARKER);
    overlay_content.push('\n');
    overlay_content.push_str(&format!("source = {}\n\n", req.native_discovery.config_path.display()));

    // Add Session Lock general block if active
    if req.session_lock_active {
        overlay_content.push_str("general {\n");
        overlay_content.push_str("    lock_cmd = /usr/bin/hyprlock\n");
        overlay_content.push_str("}\n\n");
    }

    // Add Idle Management listeners if active (sorted deterministically by timeout)
    if req.idle_management_active {
        let mut sorted_intents = req.idle_intents.clone();
        sorted_intents.sort_by_key(|l| l.timeout);

        for intent in sorted_intents {
            overlay_content.push_str("listener {\n");
            overlay_content.push_str(&format!("    timeout = {}\n", intent.timeout));
            overlay_content.push_str(&format!("    on-timeout = {}\n", intent.action.to_command_string()));
            if let Some(ref resume_action) = intent.resume_action {
                overlay_content.push_str(&format!("    on-resume = {}\n", resume_action.to_command_string()));
            }
            overlay_content.push_str("}\n\n");
        }
    }

    // 9. Compose systemd drop-in content
    let dropin_content = format!(
        "{}\n[Service]\nExecStart=\nExecStart=/usr/bin/hypridle -c {}\n",
        CANONICAL_MARKER,
        overlay_path.display()
    );

    Ok(Some(ComposedHypridlePlan {
        overlay_path,
        overlay_content,
        dropin_path,
        dropin_content,
        legacy_dropin_to_migrate,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::integration::filesystem::calculate_sha256;
    use crate::integration::hypridle::discovery::parse_hypridle_config_str;
    use crate::integration::hypridle::types::{
        HypridleDiscoveryReport, IdleAction, IdleListenerIntent, IdleResumeAction,
    };
    use crate::integration::manifest::save_manifest;
    use crate::integration::types::{
        ArtifactOwnershipPolicy, ArtifactType, IntegrationArtifact, IntegrationManifest,
    };

    fn sample_valid_discovery() -> HypridleDiscoveryReport {
        let conf = r#"
general {
    lock_cmd = pidof hyprlock || hyprlock
}
listener {
    timeout = 300
    on-timeout = loginctl lock-session
}
"#;
        parse_hypridle_config_str(conf, PathBuf::from("/home/test/.config/hypr/hypridle.conf"), None, None)
    }

    fn write_test_manifest(home: &Path, id: &str, artifacts: Vec<IntegrationArtifact>) {
        let manifest = IntegrationManifest {
            feature_id: "hypridle".to_string(),
            integration_id: id.to_string(),
            version: 1,
            enabled: true,
            artifacts,
            created_directories: Vec::new(),
            metadata: HashMap::new(),
            updated_at: 1000,
        };
        save_manifest(&manifest, home).unwrap();
    }

    #[test]
    fn test_both_features_off_returns_none() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let result = compose_hypridle_plan(&home, &req).unwrap();
        assert!(result.is_none(), "Both features off must return None signaling native state");
    }

    #[test]
    fn test_invalid_native_discovery_rejected() {
        let home = PathBuf::from("/home/test");
        let mut discovery = sample_valid_discovery();
        discovery.parse_status = HypridleParseStatus::Invalid;

        let req = HypridleCompositionRequest {
            native_discovery: discovery,
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("invalid or ambiguous syntax"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    #[test]
    fn test_duplicate_native_listener_timeouts_rejected() {
        let home = PathBuf::from("/home/test");
        let mut discovery = sample_valid_discovery();
        discovery.duplicate_listener_timeouts = vec![300];

        let req = HypridleCompositionRequest {
            native_discovery: discovery,
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("duplicate listener timeouts"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    #[test]
    fn test_session_lock_only_composition() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&home, &req).unwrap().unwrap();
        assert!(plan.overlay_content.contains("source = /home/test/.config/hypr/hypridle.conf"));
        assert!(plan.overlay_content.contains("lock_cmd = /usr/bin/hyprlock"));
        assert!(!plan.overlay_content.contains("listener {"), "No idle listeners when idle_management is off");
        assert!(plan.dropin_content.contains("ExecStart=/usr/bin/hypridle -c /home/test/.config/ryzora/hypridle/hypridle.conf"));
    }

    #[test]
    fn test_idle_management_only_composition_with_validated_intents() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 120,
                    action: IdleAction::dim_screen(10),
                    resume_action: Some(IdleResumeAction::restore_screen()),
                },
            ],
        };

        let plan = compose_hypridle_plan(&home, &req).unwrap().unwrap();
        assert!(plan.overlay_content.contains("source = /home/test/.config/hypr/hypridle.conf"));
        assert!(!plan.overlay_content.contains("lock_cmd"), "No lock_cmd when session_lock is off");
        assert!(plan.overlay_content.contains("timeout = 120"));
        assert!(plan.overlay_content.contains("on-timeout = brightnessctl -s set 10%"));
        assert!(plan.overlay_content.contains("on-resume = brightnessctl -r"));
    }

    #[test]
    fn test_both_features_composition_with_validated_intents() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 600,
                    action: IdleAction::Suspend,
                    resume_action: None,
                },
            ],
        };

        let plan = compose_hypridle_plan(&home, &req).unwrap().unwrap();
        assert!(plan.overlay_content.contains("source = /home/test/.config/hypr/hypridle.conf"));
        assert!(plan.overlay_content.contains("lock_cmd = /usr/bin/hyprlock"));
        assert!(plan.overlay_content.contains("timeout = 600"));
        assert!(plan.overlay_content.contains("on-timeout = systemctl suspend"));
    }

    #[test]
    fn test_host_validated_device_capabilities() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 60,
                    action: IdleAction::turn_off_keyboard_backlight_device("asus::kbd_backlight"),
                    resume_action: Some(IdleResumeAction::restore_keyboard_backlight_device("asus::kbd_backlight")),
                },
                IdleListenerIntent {
                    timeout: 120,
                    action: IdleAction::dim_screen_device(15, "intel_backlight"),
                    resume_action: Some(IdleResumeAction::restore_screen_device("intel_backlight")),
                },
            ],
        };

        let plan = compose_hypridle_plan(&home, &req).unwrap().unwrap();
        assert!(plan.overlay_content.contains("on-timeout = brightnessctl -sd asus::kbd_backlight set 0"));
        assert!(plan.overlay_content.contains("on-resume = brightnessctl -rd asus::kbd_backlight"));
        assert!(plan.overlay_content.contains("on-timeout = brightnessctl -sd intel_backlight set 15%"));
        assert!(plan.overlay_content.contains("on-resume = brightnessctl -rd intel_backlight"));
    }

    #[test]
    fn test_unsafe_device_identifier_rejected() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 60,
                    action: IdleAction::turn_off_keyboard_backlight_device("kbd; rm -rf /"),
                    resume_action: None,
                },
            ],
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("Invalid device identifier"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    #[test]
    fn test_listener_timeout_collision_with_native_rejected() {
        let home = PathBuf::from("/home/test");
        // Native has timeout = 300
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 300, // Collides with native 300s!
                    action: IdleAction::LockSession,
                    resume_action: None,
                },
            ],
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("timeout 300s collides with native listener timeout"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    #[test]
    fn test_duplicate_requested_listener_timeouts_rejected() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 120,
                    action: IdleAction::dim_screen(10),
                    resume_action: None,
                },
                IdleListenerIntent {
                    timeout: 120, // Duplicate among requested!
                    action: IdleAction::turn_off_keyboard_backlight(),
                    resume_action: None,
                },
            ],
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("Requested idle listeners contain duplicate timeout 120s"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    #[test]
    fn test_idle_timeout_zero_rejected() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: false,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 0,
                    action: IdleAction::LockSession,
                    resume_action: None,
                },
            ],
        };

        let err = compose_hypridle_plan(&home, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("must be greater than 0s"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }
    }

    // ==========================================
    // Authoritative Drop-in Directory Inspection Tests
    // ==========================================

    #[test]
    fn test_foreign_dropin_blocks_composition() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_foreign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();
        fs::write(dropin_dir.join("99-unowned-custom.conf"), "[Service]").unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("Foreign or user-created drop-in detected"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_dropin_absent_permitted() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_canon_absent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap();
        assert!(plan.is_some());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_dropin_verified_ryzora_owned_permitted_to_replace() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_canon_verified_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let dropin_file = dropin_dir.join("zz-ryzora-hypridle.conf");
        let dropin_content = format!("{}\n[Service]\nExecStart=\n", CANONICAL_MARKER);
        fs::write(&dropin_file, &dropin_content).unwrap();

        let sha256 = calculate_sha256(dropin_content.as_bytes());
        let artifact = IntegrationArtifact {
            path: dropin_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: CANONICAL_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, COMPOSER_INTEGRATION_ID, vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap();
        assert!(plan.is_some(), "Verified Ryzora-owned canonical artifact must be permitted to update");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_dropin_modified_rejected_as_conflict() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_canon_modified_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let dropin_file = dropin_dir.join("zz-ryzora-hypridle.conf");
        let dropin_content = format!("{}\n[Service]\nExecStart=\n# User modified line\n", CANONICAL_MARKER);
        fs::write(&dropin_file, &dropin_content).unwrap();

        let artifact = IntegrationArtifact {
            path: dropin_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            marker: CANONICAL_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, COMPOSER_INTEGRATION_ID, vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("modified externally by user"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_dropin_foreign_rejected_as_conflict() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_canon_foreign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let dropin_file = dropin_dir.join("zz-ryzora-hypridle.conf");
        fs::write(&dropin_file, "[Service]\nExecStart=/usr/bin/custom\n").unwrap();
        // No manifest written for this file

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("without an active integration manifest"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_dropin_symlink_rejected_as_security_violation() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_canon_sym_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let dropin_file = dropin_dir.join("zz-ryzora-hypridle.conf");
        let target_file = temp.join("some_target.conf");
        fs::write(&target_file, "content").unwrap();
        std::os::unix::fs::symlink(&target_file, &dropin_file).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::SecurityViolation(msg) => {
                assert!(msg.to_lowercase().contains("untrusted symlink"));
            }
            _ => panic!("Expected SecurityViolation, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    // ==========================================
    // Canonical Overlay Ownership Tests
    // ==========================================

    #[test]
    fn test_canonical_overlay_absent_permitted() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_overlay_absent_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap();
        assert!(plan.is_some());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_overlay_verified_ryzora_owned_permitted_to_replace() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_overlay_verified_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        fs::create_dir_all(overlay_path.parent().unwrap()).unwrap();

        let overlay_content = format!("{}\nsource = /tmp/native.conf\n", CANONICAL_MARKER);
        fs::write(&overlay_path, &overlay_content).unwrap();

        let sha256 = calculate_sha256(overlay_content.as_bytes());
        let artifact = IntegrationArtifact {
            path: overlay_path.clone(),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: CANONICAL_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, COMPOSER_INTEGRATION_ID, vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap();
        assert!(plan.is_some(), "Verified Ryzora-owned overlay must be permitted to update");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_overlay_modified_rejected_as_conflict() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_overlay_modified_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        fs::create_dir_all(overlay_path.parent().unwrap()).unwrap();

        let overlay_content = format!("{}\nsource = /tmp/native.conf\n# User custom setting\n", CANONICAL_MARKER);
        fs::write(&overlay_path, &overlay_content).unwrap();

        let artifact = IntegrationArtifact {
            path: overlay_path.clone(),
            artifact_type: ArtifactType::ConfigOverlay,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            marker: CANONICAL_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, COMPOSER_INTEGRATION_ID, vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("modified externally by user"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_overlay_foreign_rejected_as_conflict() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_overlay_foreign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        fs::create_dir_all(overlay_path.parent().unwrap()).unwrap();

        fs::write(&overlay_path, "general {\n    lock_cmd = foreign\n}\n").unwrap();
        // No manifest written for this file

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("without an active integration manifest"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_canonical_overlay_symlink_rejected_as_security_violation() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_overlay_sym_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let overlay_path = temp.join(CANONICAL_OVERLAY_REL);
        fs::create_dir_all(overlay_path.parent().unwrap()).unwrap();

        let target_file = temp.join("some_overlay_target.conf");
        fs::write(&target_file, "content").unwrap();
        std::os::unix::fs::symlink(&target_file, &overlay_path).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::SecurityViolation(msg) => {
                assert!(msg.to_lowercase().contains("untrusted symlink"));
            }
            _ => panic!("Expected SecurityViolation, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    // ==========================================
    // Legacy Migration Tests
    // ==========================================

    #[test]
    fn test_legacy_dropin_verified_ryzora_owned_permits_migration() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_legacy_verified_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let legacy_file = dropin_dir.join("zz-ryzora-session-lock.conf");
        let legacy_content = format!("{}\n[Service]\nExecStart=\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let sha256 = calculate_sha256(legacy_content.as_bytes());
        let artifact = IntegrationArtifact {
            path: legacy_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: LEGACY_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, "hyprland-session-lock", vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap().unwrap();
        assert_eq!(plan.legacy_dropin_to_migrate, Some(legacy_file));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_legacy_dropin_modified_rejects_migration() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_legacy_mod_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let legacy_file = dropin_dir.join("zz-ryzora-session-lock.conf");
        let legacy_content = format!("{}\n[Service]\nExecStart=\n# Modified\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let artifact = IntegrationArtifact {
            path: legacy_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string(),
            marker: LEGACY_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };

        write_test_manifest(&temp, "hyprland-session-lock", vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("modified externally by user"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_legacy_dropin_foreign_rejects_migration() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_legacy_foreign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let legacy_file = dropin_dir.join("zz-ryzora-session-lock.conf");
        fs::write(&legacy_file, "[Service]\nExecStart=/usr/bin/foreign\n").unwrap();
        // No manifest

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::ConflictError(msg) => {
                assert!(msg.contains("no active 'hyprland-session-lock' manifest was found"));
            }
            _ => panic!("Expected ConflictError, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_legacy_dropin_symlink_rejects_migration() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_legacy_sym_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();

        let legacy_file = dropin_dir.join("zz-ryzora-session-lock.conf");
        let dummy = temp.join("dummy.conf");
        fs::write(&dummy, "test").unwrap();
        std::os::unix::fs::symlink(&dummy, &legacy_file).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let err = compose_hypridle_plan(&temp, &req).unwrap_err();
        match err {
            IntegrationError::SecurityViolation(msg) => {
                assert!(msg.to_lowercase().contains("untrusted symlink"));
            }
            _ => panic!("Expected SecurityViolation, got {:?}", err),
        }

        let _ = fs::remove_dir_all(&temp);
    }

    // ==========================================
    // Determinism & Immutability / Purity Tests
    // ==========================================

    #[test]
    fn test_deterministic_composition_output() {
        let home = PathBuf::from("/home/test");
        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: true,
            idle_intents: vec![
                IdleListenerIntent {
                    timeout: 600,
                    action: IdleAction::Suspend,
                    resume_action: None,
                },
                IdleListenerIntent {
                    timeout: 100,
                    action: IdleAction::dim_screen(15),
                    resume_action: Some(IdleResumeAction::restore_screen()),
                },
            ],
        };

        let plan1 = compose_hypridle_plan(&home, &req).unwrap().unwrap();
        let plan2 = compose_hypridle_plan(&home, &req).unwrap().unwrap();

        assert_eq!(plan1.overlay_content, plan2.overlay_content);
        assert_eq!(plan1.dropin_content, plan2.dropin_content);

        let pos_100 = plan1.overlay_content.find("timeout = 100").unwrap();
        let pos_600 = plan1.overlay_content.find("timeout = 600").unwrap();
        assert!(pos_100 < pos_600, "Listeners must be sorted deterministically by timeout");
    }

    #[test]
    fn test_composer_pure_does_not_modify_filesystem() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_composer_pure_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let req = HypridleCompositionRequest {
            native_discovery: sample_valid_discovery(),
            session_lock_active: true,
            idle_management_active: false,
            idle_intents: Vec::new(),
        };

        let plan = compose_hypridle_plan(&temp, &req).unwrap().unwrap();

        // Composer plan is in memory; NO files should have been created on disk!
        assert!(!plan.overlay_path.exists(), "Composer must NOT write overlay file to disk");
        assert!(!plan.dropin_path.exists(), "Composer must NOT write drop-in file to disk");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_existing_artifacts_remain_unmodified_during_composition() {
        let mut temp = std::env::temp_dir();
        temp.push(format!("ryzora_test_composer_unmod_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        // Setup native config
        let native_conf_dir = temp.join(".config/hypr");
        fs::create_dir_all(&native_conf_dir).unwrap();
        let native_conf_file = native_conf_dir.join("hypridle.conf");
        let initial_native_content = "general {\n    lock_cmd = test\n}\n";
        fs::write(&native_conf_file, initial_native_content).unwrap();

        // Setup drop-in dir and legacy drop-in
        let dropin_dir = temp.join(CANONICAL_DROPIN_DIR_REL);
        fs::create_dir_all(&dropin_dir).unwrap();
        let legacy_file = dropin_dir.join("zz-ryzora-session-lock.conf");
        let legacy_content = format!("{}\n[Service]\nExecStart=\n", LEGACY_MARKER);
        fs::write(&legacy_file, &legacy_content).unwrap();

        let sha256 = calculate_sha256(legacy_content.as_bytes());
        let artifact = IntegrationArtifact {
            path: legacy_file.clone(),
            artifact_type: ArtifactType::SystemdDropin,
            policy: ArtifactOwnershipPolicy::Immutable,
            sha256,
            marker: LEGACY_MARKER.to_string(),
            created_at: 1000,
            permissions: Some(0o644),
        };
        write_test_manifest(&temp, "hyprland-session-lock", vec![artifact]);

        let req = HypridleCompositionRequest {
            native_discovery: parse_hypridle_config_str(
                initial_native_content,
                native_conf_file.clone(),
                None,
                None,
            ),
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

        // Run composition
        let plan = compose_hypridle_plan(&temp, &req).unwrap().unwrap();
        assert_eq!(plan.legacy_dropin_to_migrate, Some(legacy_file.clone()));

        // Verify existing disk artifacts were NOT touched or mutated in any way
        let native_after = fs::read_to_string(&native_conf_file).unwrap();
        assert_eq!(native_after, initial_native_content, "Native config must be untouched");

        let legacy_after = fs::read_to_string(&legacy_file).unwrap();
        assert_eq!(legacy_after, legacy_content, "Legacy dropin file must remain untouched by composer");

        assert!(!plan.overlay_path.exists(), "Canonical overlay must not be created by pure composer");
        assert!(!plan.dropin_path.exists(), "Canonical dropin must not be created by pure composer");

        let _ = fs::remove_dir_all(&temp);
    }
}
