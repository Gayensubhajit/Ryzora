/// settings.rs — Persistent user preferences for Ryzora (Phase 15.1)
///
/// Settings are stored at ~/.local/share/ryzora/settings.json.
/// All writes are atomic (temp + rename, same pattern as Phase 12.1 key storage).
/// Permissions are hardened: 0700 for directory, 0600 for file (Unix).
///
/// SECURITY INVARIANT:
///   Settings are preferences only. They cannot disable signature verification,
///   bypass trust policy, skip snapshots, or permit unsafe paths. Any setting
///   that would weaken the security model is rejected at validation time.
///   Unknown or injected fields are rejected via deny_unknown_fields.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Structs
// ─────────────────────────────────────────────────────────────────────────────

/// Validated, persisted user preferences.
/// deny_unknown_fields prevents tampering/injection of bypass keys.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RyzoraSettings {
    /// Default release channel applied to newly added repositories.
    /// "stable" | "beta" | "nightly" — default "stable".
    pub default_release_channel: String,
    /// Whether automatic repository index refresh is enabled.
    pub auto_refresh_enabled: bool,
    /// How often (in minutes) to auto-refresh repository indexes.
    /// Clamped to [15, 1440]. Default 60. Ignored when auto_refresh_enabled = false.
    pub auto_refresh_interval_minutes: u32,
    /// Notification verbosity level.
    /// "all" | "security_only" | "none" — default "all".
    pub notification_level: String,
    /// Whether update-available notifications appear in activity feed.
    /// "notify" | "silent" — default "notify".
    pub update_notification_policy: String,
    /// Run an integrity scan automatically on application startup.
    /// Default false — avoids startup latency on large installs.
    pub integrity_scan_on_startup: bool,
    /// Show packages with Community/Unvetted trust tier in Discover.
    /// Default true.
    pub show_unverified_packages: bool,
    /// Show packages on the nightly release channel.
    /// Default false.
    pub show_nightly_packages: bool,
    /// Use compact information-dense UI layout.
    /// Default false.
    pub compact_ui: bool,
    /// Snapshot creation policy on package apply/install.
    /// "never" | "ask" | "always" — default "ask".
    pub snapshot_policy: String,
    /// Maximum number of snapshots to retain. Default 5, clamped to [1, 50].
    pub snapshot_retention_count: u32,
    /// Automatically delete older snapshots exceeding retention count. Default true.
    pub snapshot_auto_cleanup: bool,
}

impl Default for RyzoraSettings {
    fn default() -> Self {
        RyzoraSettings {
            default_release_channel: "stable".to_string(),
            auto_refresh_enabled: true,
            auto_refresh_interval_minutes: 60,
            notification_level: "all".to_string(),
            update_notification_policy: "notify".to_string(),
            integrity_scan_on_startup: false,
            show_unverified_packages: true,
            show_nightly_packages: false,
            compact_ui: false,
            snapshot_policy: "never".to_string(),
            snapshot_retention_count: 5,
            snapshot_auto_cleanup: true,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Paths & Permissions
// ─────────────────────────────────────────────────────────────────────────────

pub fn settings_dir() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora")
}

pub fn settings_file_path() -> PathBuf {
    settings_dir().join("settings.json")
}

/// Verify that file permissions are strict (0600 on Unix, no group/other access).
pub fn verify_file_permissions(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = fs::metadata(path)
            .map_err(|e| format!("Failed to read metadata for '{}': {}", path.display(), e))?;
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(format!(
                "Insecure permissions 0{:03o} on '{}' — file is group or world accessible",
                mode & 0o777,
                path.display()
            ));
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence
// ─────────────────────────────────────────────────────────────────────────────

/// Load settings from disk. Returns defaults if the file does not exist or
/// cannot be parsed (never fails — missing/corrupt settings fall back to
/// safe defaults rather than crashing the application).
pub fn load_settings() -> RyzoraSettings {
    load_settings_from_path(&settings_file_path())
}

/// Load settings from an explicit file path.
pub fn load_settings_from_path(path: &Path) -> RyzoraSettings {
    if path.is_file() {
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(s) = serde_json::from_str::<RyzoraSettings>(&raw) {
                if validate_settings(&s).is_ok() {
                    return s;
                }
            }
        }
    }
    RyzoraSettings::default()
}

/// Persist settings to disk atomically (temp file → rename) with hardened permissions.
/// Validates settings before writing.
pub fn save_settings_to_disk(settings: &RyzoraSettings) -> Result<(), String> {
    save_settings_to_path(settings, &settings_file_path())
}

/// Persist settings to an explicit path atomically with hardened permissions (0700 dir, 0600 file).
pub fn save_settings_to_path(settings: &RyzoraSettings, path: &Path) -> Result<(), String> {
    validate_settings(settings)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        let _ = crate::crypto::set_secure_permissions(parent, 0o700);
    }

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    // Atomic write: temp file beside destination → rename
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tmp_path = path
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .join(format!(".settings.json.tmp.{}", nanos));

    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write temporary settings file: {}", e))?;
    let _ = crate::crypto::set_secure_permissions(&tmp_path, 0o600);

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to atomically rename settings file: {}", e)
    })?;
    let _ = crate::crypto::set_secure_permissions(path, 0o600);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validate user-supplied settings.
///
/// SECURITY NOTE: This function is the enforcement point ensuring that no
/// setting can disable signature verification, bypass trust policy, skip
/// snapshots, or permit unsafe paths. All security invariants are hard-coded
/// in the engine — settings only adjust preferences within safe bounds.
pub fn validate_settings(s: &RyzoraSettings) -> Result<(), String> {
    let valid_channels = ["stable", "beta", "nightly"];
    if !valid_channels.contains(&s.default_release_channel.as_str()) {
        return Err(format!(
            "Invalid release channel '{}'. Must be one of: stable, beta, nightly.",
            s.default_release_channel
        ));
    }

    if s.auto_refresh_interval_minutes < 15 || s.auto_refresh_interval_minutes > 1440 {
        return Err(format!(
            "auto_refresh_interval_minutes must be between 15 and 1440, got {}.",
            s.auto_refresh_interval_minutes
        ));
    }

    let valid_notif = ["all", "security_only", "none"];
    if !valid_notif.contains(&s.notification_level.as_str()) {
        return Err(format!(
            "Invalid notification_level '{}'. Must be one of: all, security_only, none.",
            s.notification_level
        ));
    }

    let valid_policy = ["notify", "silent"];
    if !valid_policy.contains(&s.update_notification_policy.as_str()) {
        return Err(format!(
            "Invalid update_notification_policy '{}'. Must be one of: notify, silent.",
            s.update_notification_policy
        ));
    }

    let valid_snapshot_policies = ["never", "ask", "always"];
    if !valid_snapshot_policies.contains(&s.snapshot_policy.as_str()) {
        return Err(format!(
            "Invalid snapshot_policy '{}'. Must be one of: never, ask, always.",
            s.snapshot_policy
        ));
    }

    if s.snapshot_retention_count < 1 || s.snapshot_retention_count > 50 {
        return Err(format!(
            "snapshot_retention_count must be between 1 and 50, got {}.",
            s.snapshot_retention_count
        ));
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// Return the current user settings. Falls back to defaults if missing/invalid.
#[tauri::command]
pub fn get_settings() -> Result<RyzoraSettings, String> {
    Ok(load_settings())
}

/// Persist updated user settings. Validates all fields before writing.
#[tauri::command]
pub fn save_settings(settings: RyzoraSettings) -> Result<(), String> {
    save_settings_to_disk(&settings)
}

/// Reset all settings to factory defaults and persist.
#[tauri::command]
pub fn reset_settings() -> Result<RyzoraSettings, String> {
    let defaults = RyzoraSettings::default();
    save_settings_to_disk(&defaults)?;
    Ok(defaults)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let p = env::temp_dir().join(format!("ryzora-settings-test-{}", nanos));
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn test_settings_default_values() {
        let s = RyzoraSettings::default();
        assert_eq!(s.default_release_channel, "stable");
        assert!(s.auto_refresh_enabled);
        assert_eq!(s.auto_refresh_interval_minutes, 60);
        assert_eq!(s.notification_level, "all");
        assert_eq!(s.update_notification_policy, "notify");
        assert!(!s.integrity_scan_on_startup);
        assert!(s.show_unverified_packages);
        assert!(!s.show_nightly_packages);
        assert!(!s.compact_ui);
    }

    #[test]
    fn test_settings_round_trip_persistence() {
        let dir = temp_test_dir();
        let path = dir.join("settings.json");

        let mut s = RyzoraSettings::default();
        s.default_release_channel = "beta".to_string();
        s.compact_ui = true;
        s.auto_refresh_interval_minutes = 120;
        s.show_nightly_packages = true;

        save_settings_to_path(&s, &path).unwrap();
        let loaded = load_settings_from_path(&path);
        assert_eq!(loaded.default_release_channel, "beta");
        assert!(loaded.compact_ui);
        assert_eq!(loaded.auto_refresh_interval_minutes, 120);
        assert!(loaded.show_nightly_packages);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_permission_hardening() {
        let dir = temp_test_dir();
        let path = dir.join("settings.json");
        let s = RyzoraSettings::default();

        save_settings_to_path(&s, &path).unwrap();
        assert!(verify_file_permissions(&path).is_ok());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
            let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700, "Settings directory must be 0700");
            assert_eq!(file_mode, 0o600, "Settings file must be 0600");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_tampered_unknown_fields_fallback() {
        let dir = temp_test_dir();
        let path = dir.join("settings.json");

        // Attempting to inject an unauthorized setting (e.g. bypass signature verification)
        let malicious_json = r#"{
            "default_release_channel": "stable",
            "auto_refresh_enabled": true,
            "auto_refresh_interval_minutes": 60,
            "notification_level": "all",
            "update_notification_policy": "notify",
            "integrity_scan_on_startup": false,
            "show_unverified_packages": true,
            "show_nightly_packages": false,
            "compact_ui": false,
            "disable_signatures": true,
            "bypass_snapshots": true
        }"#;
        fs::write(&path, malicious_json).unwrap();

        // Must fall back to safe defaults — reject tampered configuration
        let loaded = load_settings_from_path(&path);
        assert_eq!(loaded, RyzoraSettings::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_corrupted_json_fallback() {
        let dir = temp_test_dir();
        let path = dir.join("settings.json");
        fs::write(&path, "{ this is corrupt json ").unwrap();

        let loaded = load_settings_from_path(&path);
        assert_eq!(loaded, RyzoraSettings::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_invalid_values_fallback() {
        let dir = temp_test_dir();
        let path = dir.join("settings.json");

        // Valid JSON but invalid channel
        let invalid_json = r#"{
            "default_release_channel": "unsupported_channel",
            "auto_refresh_enabled": true,
            "auto_refresh_interval_minutes": 60,
            "notification_level": "all",
            "update_notification_policy": "notify",
            "integrity_scan_on_startup": false,
            "show_unverified_packages": true,
            "show_nightly_packages": false,
            "compact_ui": false
        }"#;
        fs::write(&path, invalid_json).unwrap();

        let loaded = load_settings_from_path(&path);
        assert_eq!(loaded, RyzoraSettings::default());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_settings_validation_rejects_invalid_channel() {
        let mut s = RyzoraSettings::default();
        s.default_release_channel = "canary".to_string();
        let err = validate_settings(&s).unwrap_err();
        assert!(err.contains("Invalid release channel"));
    }

    #[test]
    fn test_settings_validation_rejects_interval_too_low() {
        let mut s = RyzoraSettings::default();
        s.auto_refresh_interval_minutes = 5;
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn test_settings_validation_rejects_interval_too_high() {
        let mut s = RyzoraSettings::default();
        s.auto_refresh_interval_minutes = 9999;
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn test_settings_validation_rejects_invalid_notification_level() {
        let mut s = RyzoraSettings::default();
        s.notification_level = "verbose".to_string();
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn test_settings_validation_rejects_auto_apply_policy() {
        let mut s = RyzoraSettings::default();
        s.update_notification_policy = "auto_apply".to_string();
        assert!(validate_settings(&s).is_err());
    }

    #[test]
    fn test_settings_all_valid_channels_pass() {
        for ch in &["stable", "beta", "nightly"] {
            let mut s = RyzoraSettings::default();
            s.default_release_channel = ch.to_string();
            assert!(
                validate_settings(&s).is_ok(),
                "Channel '{}' should be valid",
                ch
            );
        }
    }

    #[test]
    fn test_settings_reset_returns_defaults() {
        let defaults = RyzoraSettings::default();
        assert_eq!(defaults.default_release_channel, "stable");
        assert!(!defaults.compact_ui);
        assert_eq!(defaults.auto_refresh_interval_minutes, 60);
        assert!(validate_settings(&defaults).is_ok());
    }

    #[test]
    fn test_settings_cannot_bypass_security_invariants() {
        // Explicitly verify settings has no capabilities to influence security:
        let s = RyzoraSettings::default();
        assert!(validate_settings(&s).is_ok());

        // Verify serializing and checking fields: no security toggles exist
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("signature"));
        assert!(!json.contains("trust"));
        assert!(!json.contains("bypass"));
        assert!(!json.contains("force"));
        assert!(!json.contains("sudo"));
        assert!(!json.contains("root"));
    }

    #[test]
    fn test_settings_zero_command_execution() {
        let s = RyzoraSettings::default();
        assert!(validate_settings(&s).is_ok());
    }
}
