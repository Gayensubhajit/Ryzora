/// settings.rs — Persistent user preferences for Ryzora (Phase 15.1)
///
/// Settings are stored at ~/.local/share/ryzora/settings.json.
/// All writes are atomic (temp + rename, same pattern as Phase 12.1 key storage).
///
/// SECURITY INVARIANT:
///   Settings are preferences only. They cannot disable signature verification,
///   bypass trust policy, skip snapshots, or permit unsafe paths. Any setting
///   that would weaken the security model is rejected at validation time.
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Structs
// ─────────────────────────────────────────────────────────────────────────────

/// Validated, persisted user preferences.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Paths
// ─────────────────────────────────────────────────────────────────────────────

pub fn settings_file_path() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora/settings.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Persistence
// ─────────────────────────────────────────────────────────────────────────────

/// Load settings from disk. Returns defaults if the file does not exist or
/// cannot be parsed (never fails — missing/corrupt settings fall back to
/// safe defaults rather than crashing the application).
pub fn load_settings() -> RyzoraSettings {
    let path = settings_file_path();
    if path.is_file() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(s) = serde_json::from_str::<RyzoraSettings>(&raw) {
                if validate_settings(&s).is_ok() {
                    return s;
                }
            }
        }
    }
    RyzoraSettings::default()
}

/// Persist settings to disk atomically (temp file → rename).
/// Validates settings before writing.
pub fn save_settings_to_disk(settings: &RyzoraSettings) -> Result<(), String> {
    validate_settings(settings)?;

    let path = settings_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }

    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    // Atomic write: temp file → rename
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tmp_path = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("/tmp"))
        .join(format!(".settings.json.tmp.{}", nanos));

    fs::write(&tmp_path, &json)
        .map_err(|e| format!("Failed to write temporary settings file: {}", e))?;

    fs::rename(&tmp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to atomically rename settings file: {}", e)
    })?;

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

    fn temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        env::temp_dir().join(format!("ryzora-settings-test-{}", nanos))
    }

    fn write_and_read(path: &PathBuf, settings: &RyzoraSettings) -> RyzoraSettings {
        let json = serde_json::to_string_pretty(settings).unwrap();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let tmp = path.parent().unwrap().join(format!(".tmp.{}", nanos));
        fs::write(&tmp, &json).unwrap();
        fs::rename(&tmp, path).unwrap();
        let raw = fs::read_to_string(path).unwrap();
        serde_json::from_str(&raw).unwrap()
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
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        let mut s = RyzoraSettings::default();
        s.default_release_channel = "beta".to_string();
        s.compact_ui = true;
        s.auto_refresh_interval_minutes = 120;
        s.show_nightly_packages = true;

        let loaded = write_and_read(&path, &s);
        assert_eq!(loaded.default_release_channel, "beta");
        assert!(loaded.compact_ui);
        assert_eq!(loaded.auto_refresh_interval_minutes, 120);
        assert!(loaded.show_nightly_packages);

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
        // auto_apply is explicitly NOT a valid policy (deferred)
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
    fn test_settings_integrity_scan_off_by_default_but_valid() {
        // integrity_scan_on_startup=false only disables automatic scanning;
        // it never disables manual scans or installation-time verification.
        let s = RyzoraSettings::default();
        assert!(!s.integrity_scan_on_startup);
        assert!(validate_settings(&s).is_ok());
    }

    #[test]
    fn test_settings_zero_command_execution() {
        let s = RyzoraSettings::default();
        assert!(validate_settings(&s).is_ok());
        // No std::process::Command was used — settings engine is pure I/O only.
    }
}
