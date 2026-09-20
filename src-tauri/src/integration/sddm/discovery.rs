//! SilentSDDM Discovery — Phase S1
//!
//! Read-only host report covering:
//!   - SDDM installation + version (must be >= 0.21)
//!   - Qt version + multimedia capability (required for video backgrounds)
//!   - Current effective SDDM theme (from conf.d ordering)
//!   - SilentSDDM engine presence + Ryzora ownership
//!   - Cached upstream wallpapers + custom videos
//!   - Active wallpaper / target state from Ryzora manifest
//!
//! All functions accept an injectable `home: &Path` and `sys_root: Option<&Path>`
//! for full testability without system modification.

use crate::sddm_helper::resolve_effective_sddm_theme_in;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// The Ryzora-owned theme slug for SilentSDDM.
pub const RYZORA_SILENT_SLUG: &str = "ryzora-silent";

/// Upstream SilentSDDM pinned revision metadata.
pub const UPSTREAM_SHA: &str = "73380d331fe0f8b8a3b17991bb917a0d438a4d34";
pub const UPSTREAM_VERSION: &str = "1.5.0";
pub const UPSTREAM_REPO: &str = "uiriansan/SilentSDDM";

/// Supported background extensions (from SilentSDDM documentation).
pub const SUPPORTED_IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png"];
pub const SUPPORTED_VIDEO_EXTS: &[&str] = &["avi", "mp4", "mov", "mkv", "m4v", "webm"];
/// GIF is explicitly rejected — crashes SDDM.
pub const REJECTED_EXTS: &[&str] = &["gif"];

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Complete read-only report of the host's SilentSDDM state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentSddmHostReport {
    // ── SDDM host capabilities ────────────────────────────────────────────────
    pub sddm_installed: bool,
    /// Parsed from `sddm --version`. None if not installed or parse failed.
    pub sddm_version: Option<SemVer>,
    /// Whether sddm_version >= 0.21.0 (required by SilentSDDM).
    pub sddm_version_ok: bool,

    // ── Qt capabilities ───────────────────────────────────────────────────────
    pub qt_version: Option<SemVer>,
    /// Whether qt_version >= 6.5.0 (required by SilentSDDM).
    pub qt_version_ok: bool,
    /// Whether Qt Multimedia module is available (required for video backgrounds).
    pub qt_multimedia_ok: bool,

    // ── Effective SDDM theme state ────────────────────────────────────────────
    /// Currently active theme name, from /etc/sddm.conf.d/ resolution.
    pub current_theme: Option<String>,
    /// The conf.d file that is setting the current theme.
    pub current_theme_file: Option<PathBuf>,
    /// Whether Ryzora currently owns the active SDDM theme.
    pub ryzora_owns_sddm: bool,

    // ── SilentSDDM engine ─────────────────────────────────────────────────────
    /// Whether /usr/share/sddm/themes/ryzora-silent/ exists.
    pub engine_installed: bool,
    /// Install path on this host.
    pub engine_path: Option<PathBuf>,
    /// Version parsed from metadata.desktop inside the installed theme.
    pub engine_version: Option<String>,
    /// Whether Ryzora's manifest claims ownership of the installed engine.
    pub engine_owned_by_ryzora: bool,

    // ── Ryzora manifest state ─────────────────────────────────────────────────
    /// The Ryzora SilentSDDM manifest if present.
    pub manifest: Option<SilentSddmManifest>,

    // ── Cached assets ─────────────────────────────────────────────────────────
    pub cached_wallpapers: Vec<CachedAsset>,
    pub cached_custom: Vec<CachedAsset>,

    // ── Validation warnings ───────────────────────────────────────────────────
    /// Non-fatal issues discovered during the scan.
    pub warnings: Vec<String>,
}

/// Three-part semantic version for comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub raw: String,
}

impl SemVer {
    pub fn parse(s: &str) -> Option<Self> {
        // Accepts "0.21.0", "6.7", "6.7.2", strips leading "v"
        let s = s.trim().trim_start_matches('v');
        let parts: Vec<&str> = s.split('.').collect();
        let major = parts.first().and_then(|p| p.parse().ok())?;
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(Self { major, minor, patch, raw: s.to_string() })
    }

    pub fn at_least(&self, major: u32, minor: u32, patch: u32) -> bool {
        (self.major, self.minor, self.patch) >= (major, minor, patch)
    }
}

/// Ryzora-owned manifest stored at:
/// ~/.local/share/ryzora/lockscreens/silentsddm/manifest.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilentSddmManifest {
    pub provider: String,           // always "silentsddm"
    pub upstream_sha: String,
    pub version: String,
    pub engine_path: String,        // absolute path
    pub installed_at: String,       // ISO 8601
    pub active_wallpaper_id: Option<String>,
    pub active_targets: SilentSddmTargets,
    pub sddm_active: bool,
    pub previous_sddm_theme: Option<String>,
}

/// Which SDDM screens have Ryzora-managed backgrounds applied.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SilentSddmTargets {
    /// [LoginScreen] background managed by Ryzora.
    pub login_screen: bool,
    /// [LockScreen] background managed by Ryzora (SDDM QML lock mode, NOT hyprland session lock).
    pub lock_screen: bool,
}

/// Metadata for a single cached asset (upstream wallpaper or custom video).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAsset {
    /// Provider-scoped ID, e.g. "silentsddm-ken" or "custom:anime.mp4"
    pub id: String,
    pub filename: String,
    pub asset_type: AssetType,
    pub media_type: MediaType,
    /// SHA-256 hex of the file content (computed at download/import time).
    pub sha256: String,
    pub size_bytes: u64,
    pub installed_at: String,
    /// Only present for upstream assets.
    pub upstream_url: Option<String>,
    /// Only present for custom assets — the original import path (may no longer exist).
    pub original_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Upstream,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Video,
}

// ─────────────────────────────────────────────────────────────────────────────
// Custom Video Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Result of validating a user-supplied video path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomVideoValidation {
    pub valid: bool,
    pub path: PathBuf,
    pub filename: String,
    pub extension: String,
    pub media_type: Option<MediaType>,
    pub size_bytes: Option<u64>,
    /// Human-readable rejection reason if valid == false.
    pub error: Option<String>,
}

/// Validate that a user-supplied file is an acceptable SilentSDDM background.
/// Does NOT copy or modify any file.
pub fn validate_custom_video(path: &Path) -> CustomVideoValidation {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Reject GIF explicitly (crashes SDDM)
    if REJECTED_EXTS.contains(&ext.as_str()) {
        return CustomVideoValidation {
            valid: false,
            path: path.to_path_buf(),
            filename,
            extension: ext,
            media_type: None,
            size_bytes: None,
            error: Some(format!(
                ".gif files are not supported by SilentSDDM — they can crash SDDM. \
                 Please use .mp4, .webm, .mkv, .mov, .avi, or .m4v instead."
            )),
        };
    }

    // Check extension is in supported lists
    let media_type = if SUPPORTED_VIDEO_EXTS.contains(&ext.as_str()) {
        MediaType::Video
    } else if SUPPORTED_IMAGE_EXTS.contains(&ext.as_str()) {
        MediaType::Image
    } else {
        return CustomVideoValidation {
            valid: false,
            path: path.to_path_buf(),
            filename,
            extension: ext.clone(),
            media_type: None,
            size_bytes: None,
            error: Some(format!(
                "Unsupported file type '.{}'. SilentSDDM supports: {}",
                ext,
                [SUPPORTED_VIDEO_EXTS, SUPPORTED_IMAGE_EXTS].concat().join(", ")
            )),
        };
    };

    // Check file exists and is readable
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            return CustomVideoValidation {
                valid: false,
                path: path.to_path_buf(),
                filename,
                extension: ext,
                media_type: Some(media_type),
                size_bytes: None,
                error: Some(format!("Cannot read file: {}", e)),
            };
        }
    };

    if !meta.is_file() {
        return CustomVideoValidation {
            valid: false,
            path: path.to_path_buf(),
            filename,
            extension: ext,
            media_type: Some(media_type),
            size_bytes: None,
            error: Some("Path is not a file.".to_string()),
        };
    }

    CustomVideoValidation {
        valid: true,
        path: path.to_path_buf(),
        filename,
        extension: ext,
        media_type: Some(media_type),
        size_bytes: Some(meta.len()),
        error: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Host Discovery
// ─────────────────────────────────────────────────────────────────────────────

/// Build a complete, read-only SilentSDDM host report.
///
/// # Arguments
/// * `home`     - The user's home directory (injectable for tests).
/// * `sys_root` - Optional override for the system root (injectable for tests).
///                Pass `None` for real-machine operation.
pub fn discover_silentsddm_in(home: &Path, sys_root: Option<&Path>) -> SilentSddmHostReport {
    let mut warnings = Vec::new();

    // ── SDDM version ──────────────────────────────────────────────────────────
    let (sddm_installed, sddm_version, sddm_version_ok) = detect_sddm_version(&mut warnings);

    // ── Qt version + Multimedia ───────────────────────────────────────────────
    let (qt_version, qt_version_ok, qt_multimedia_ok) =
        detect_qt_capabilities(&mut warnings);

    // ── Effective SDDM theme ──────────────────────────────────────────────────
    let sddm_resolution = resolve_effective_sddm_theme_in(sys_root);
    let current_theme = sddm_resolution.effective_theme.clone();
    let current_theme_file = sddm_resolution.effective_file.clone();
    let ryzora_owns_sddm = sddm_resolution.is_ryzora;

    // ── Engine presence ───────────────────────────────────────────────────────
    let engine_path = resolve_engine_path(sys_root);
    let engine_installed = engine_path.exists();
    let engine_version = if engine_installed {
        read_metadata_desktop_version(&engine_path)
    } else {
        None
    };

    // ── Ryzora manifest ───────────────────────────────────────────────────────
    let manifest = load_ryzora_manifest(home);
    let engine_owned_by_ryzora = manifest
        .as_ref()
        .map(|m| m.engine_path == engine_path.to_string_lossy())
        .unwrap_or(false);

    // ── Cached assets ─────────────────────────────────────────────────────────
    let silentsddm_data_dir = silentsddm_data_dir(home);
    let cached_wallpapers = load_cached_assets(&silentsddm_data_dir.join("wallpapers_manifest.json"));
    let cached_custom = load_cached_assets(&silentsddm_data_dir.join("custom_manifest.json"));

    // ── Engine installed but not by Ryzora → warn ─────────────────────────────
    if engine_installed && !engine_owned_by_ryzora {
        warnings.push(format!(
            "Found SilentSDDM theme at {} but it was not installed by Ryzora. \
             Ryzora will not modify it.",
            engine_path.display()
        ));
    }

    SilentSddmHostReport {
        sddm_installed,
        sddm_version,
        sddm_version_ok,
        qt_version,
        qt_version_ok,
        qt_multimedia_ok,
        current_theme,
        current_theme_file,
        ryzora_owns_sddm,
        engine_installed,
        engine_path: if engine_installed { Some(engine_path) } else { None },
        engine_version,
        engine_owned_by_ryzora,
        manifest,
        cached_wallpapers,
        cached_custom,
        warnings,
    }
}

/// Convenience wrapper using the real home directory.
pub fn discover_silentsddm(sys_root: Option<&Path>) -> SilentSddmHostReport {
    let home = dirs_home();
    discover_silentsddm_in(&home, sys_root)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the Ryzora data directory for SilentSDDM:
/// ~/.local/share/ryzora/lockscreens/silentsddm/
pub fn silentsddm_data_dir(home: &Path) -> PathBuf {
    home.join(".local/share/ryzora/lockscreens/silentsddm")
}

/// Returns the expected engine install path:
/// {sys_root}/usr/share/sddm/themes/ryzora-silent/
pub fn resolve_engine_path(sys_root: Option<&Path>) -> PathBuf {
    let root = sys_root.unwrap_or_else(|| Path::new("/"));
    root.join("usr/share/sddm/themes").join(RYZORA_SILENT_SLUG)
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/root"))
}

/// Detect SDDM version by running `sddm --version`.
fn detect_sddm_version(warnings: &mut Vec<String>) -> (bool, Option<SemVer>, bool) {
    let output = Command::new("sddm").arg("--version").output();
    match output {
        Err(_) => (false, None, false),
        Ok(out) => {
            let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
            // `sddm --version` typically prints just the version number
            let version_str = raw.lines().last().unwrap_or("").trim().to_string();
            match SemVer::parse(&version_str) {
                Some(v) => {
                    let ok = v.at_least(0, 21, 0);
                    if !ok {
                        warnings.push(format!(
                            "SDDM version {} is below the required 0.21.0 for SilentSDDM.",
                            v.raw
                        ));
                    }
                    (true, Some(v), ok)
                }
                None => {
                    // SDDM is installed but version string wasn't parseable
                    warnings.push(format!(
                        "Could not parse SDDM version from output: '{}'",
                        version_str
                    ));
                    (true, None, false)
                }
            }
        }
    }
}

/// Detect Qt version and multimedia availability.
fn detect_qt_capabilities(warnings: &mut Vec<String>) -> (Option<SemVer>, bool, bool) {
    // Try qmake6 first, fall back to qmake
    let qt_version = detect_qt_version(warnings);
    let qt_ok = qt_version.as_ref().map_or(false, |v| v.at_least(6, 5, 0));
    if !qt_ok {
        if qt_version.is_some() {
            warnings.push(format!(
                "Qt version {} is below the required 6.5.0 for SilentSDDM.",
                qt_version.as_ref().unwrap().raw
            ));
        } else {
            warnings.push("Qt 6 (qmake6) not found. SilentSDDM requires Qt >= 6.5.".to_string());
        }
    }

    // Check qt6-multimedia via pacman -Q (Arch-specific)
    let multimedia_ok = check_qt_multimedia(warnings);

    (qt_version, qt_ok, multimedia_ok)
}

fn detect_qt_version(warnings: &mut Vec<String>) -> Option<SemVer> {
    for qmake in &["qmake6", "qmake"] {
        if let Ok(out) = Command::new(qmake).arg("-query").arg("QT_VERSION").output() {
            if out.status.success() {
                let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Some(v) = SemVer::parse(&raw) {
                    return Some(v);
                }
                warnings.push(format!(
                    "Could not parse Qt version from {} -query QT_VERSION: '{}'",
                    qmake, raw
                ));
            }
        }
    }
    None
}

fn check_qt_multimedia(warnings: &mut Vec<String>) -> bool {
    // Arch Linux: check qt6-multimedia package
    if let Ok(out) = Command::new("pacman").args(["-Q", "qt6-multimedia"]).output() {
        if out.status.success() {
            return true;
        }
    }
    // Fallback: check for the shared object directly
    let so_paths = [
        "/usr/lib/qt6/plugins/multimedia",
        "/usr/lib/x86_64-linux-gnu/qt6/plugins/multimedia",
    ];
    for p in &so_paths {
        if Path::new(p).exists() {
            return true;
        }
    }
    warnings.push(
        "Qt Multimedia (qt6-multimedia) does not appear to be installed. \
         Video backgrounds in SilentSDDM will not work without it."
            .to_string(),
    );
    false
}

/// Read the Version= field from a SilentSDDM metadata.desktop file.
fn read_metadata_desktop_version(engine_path: &Path) -> Option<String> {
    let meta_path = engine_path.join("metadata.desktop");
    let content = fs::read_to_string(&meta_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(ver) = trimmed.strip_prefix("Version=") {
            return Some(ver.trim().to_string());
        }
    }
    None
}

/// Load the Ryzora SilentSDDM manifest from:
/// ~/.local/share/ryzora/lockscreens/silentsddm/manifest.json
fn load_ryzora_manifest(home: &Path) -> Option<SilentSddmManifest> {
    let path = silentsddm_data_dir(home).join("manifest.json");
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Load cached assets from a JSON manifest file.
fn load_cached_assets(manifest_path: &Path) -> Vec<CachedAsset> {
    let content = match fs::read_to_string(manifest_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    // The manifest is a JSON object: { "id": CachedAsset, ... }
    let map: std::collections::HashMap<String, CachedAsset> =
        serde_json::from_str(&content).unwrap_or_default();
    let mut assets: Vec<CachedAsset> = map.into_values().collect();
    assets.sort_by(|a, b| a.installed_at.cmp(&b.installed_at));
    assets
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("ryzora-sddm-test-{}-{}-{}", label, std::process::id(), nanos));
            let _ = std::fs::create_dir_all(&path);
            Self { path }
        }
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn tmp() -> TestDir {
        TestDir::new("tmp")
    }

    // ── SemVer parsing ────────────────────────────────────────────────────────

    #[test]
    fn semver_parse_full() {
        let v = SemVer::parse("0.21.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 21);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn semver_parse_two_part() {
        let v = SemVer::parse("6.7").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (6, 7, 0));
    }

    #[test]
    fn semver_parse_with_v_prefix() {
        let v = SemVer::parse("v1.5.0").unwrap();
        assert_eq!((v.major, v.minor, v.patch), (1, 5, 0));
    }

    #[test]
    fn semver_parse_empty_is_none() {
        assert!(SemVer::parse("").is_none());
        assert!(SemVer::parse("not-a-version").is_none());
    }

    #[test]
    fn semver_at_least() {
        let v = SemVer::parse("0.21.0").unwrap();
        assert!(v.at_least(0, 21, 0));
        assert!(v.at_least(0, 20, 99));
        assert!(!v.at_least(0, 22, 0));
        assert!(!v.at_least(1, 0, 0));
    }

    #[test]
    fn semver_qt_at_least_6_5() {
        let v = SemVer::parse("6.7.2").unwrap();
        assert!(v.at_least(6, 5, 0));
        let v2 = SemVer::parse("6.4.0").unwrap();
        assert!(!v2.at_least(6, 5, 0));
    }

    // ── Custom video validation ───────────────────────────────────────────────

    #[test]
    fn validate_custom_gif_rejected() {
        let tmp = tmp();
        let path = tmp.path().join("animation.gif");
        fs::write(&path, b"fake gif content").unwrap();
        let result = validate_custom_video(&path);
        assert!(!result.valid);
        assert!(result.error.as_deref().unwrap_or("").contains("gif"));
    }

    #[test]
    fn validate_custom_mp4_accepted() {
        let tmp = tmp();
        let path = tmp.path().join("wallpaper.mp4");
        fs::write(&path, b"fake mp4").unwrap();
        let result = validate_custom_video(&path);
        assert!(result.valid, "error: {:?}", result.error);
        assert_eq!(result.media_type, Some(MediaType::Video));
    }

    #[test]
    fn validate_custom_webm_accepted() {
        let tmp = tmp();
        let path = tmp.path().join("clip.webm");
        fs::write(&path, b"fake webm").unwrap();
        let result = validate_custom_video(&path);
        assert!(result.valid);
        assert_eq!(result.media_type, Some(MediaType::Video));
    }

    #[test]
    fn validate_custom_jpg_accepted() {
        let tmp = tmp();
        let path = tmp.path().join("photo.jpg");
        fs::write(&path, b"fake jpg").unwrap();
        let result = validate_custom_video(&path);
        assert!(result.valid);
        assert_eq!(result.media_type, Some(MediaType::Image));
    }

    #[test]
    fn validate_custom_exe_rejected() {
        let tmp = tmp();
        let path = tmp.path().join("suspicious.exe");
        fs::write(&path, b"MZ").unwrap();
        let result = validate_custom_video(&path);
        assert!(!result.valid);
        assert!(result.error.as_deref().unwrap_or("").contains("Unsupported"));
    }

    #[test]
    fn validate_custom_missing_file() {
        let path = PathBuf::from("/nonexistent/path/file.mp4");
        let result = validate_custom_video(&path);
        assert!(!result.valid);
        assert!(result.error.as_deref().unwrap_or("").contains("Cannot read"));
    }

    #[test]
    fn validate_custom_directory_rejected() {
        let tmp = tmp();
        let result = validate_custom_video(tmp.path());
        assert!(!result.valid);
    }

    #[test]
    fn validate_all_supported_video_extensions() {
        let tmp = tmp();
        for ext in SUPPORTED_VIDEO_EXTS {
            let path = tmp.path().join(format!("file.{}", ext));
            fs::write(&path, b"data").unwrap();
            let result = validate_custom_video(&path);
            assert!(result.valid, "Expected {} to be valid", ext);
            assert_eq!(result.media_type, Some(MediaType::Video));
        }
    }

    #[test]
    fn validate_all_supported_image_extensions() {
        let tmp = tmp();
        for ext in SUPPORTED_IMAGE_EXTS {
            let path = tmp.path().join(format!("img.{}", ext));
            fs::write(&path, b"data").unwrap();
            let result = validate_custom_video(&path);
            assert!(result.valid, "Expected {} to be valid", ext);
            assert_eq!(result.media_type, Some(MediaType::Image));
        }
    }

    // ── Engine path resolution ────────────────────────────────────────────────

    #[test]
    fn engine_path_default_root() {
        let path = resolve_engine_path(None);
        assert_eq!(
            path,
            PathBuf::from("/usr/share/sddm/themes/ryzora-silent")
        );
    }

    #[test]
    fn engine_path_with_sys_root() {
        let tmp = tmp();
        let path = resolve_engine_path(Some(tmp.path()));
        assert_eq!(
            path,
            tmp.path().join("usr/share/sddm/themes/ryzora-silent")
        );
    }

    #[test]
    fn engine_not_installed_when_dir_absent() {
        let tmp = tmp();
        let path = resolve_engine_path(Some(tmp.path()));
        assert!(!path.exists());
    }

    #[test]
    fn engine_installed_detected() {
        let tmp = tmp();
        let engine_dir = tmp.path().join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(&engine_dir).unwrap();
        fs::write(
            engine_dir.join("metadata.desktop"),
            "[SddmGreeterTheme]\nVersion=1.5.0\nName=Silent\n",
        )
        .unwrap();

        let path = resolve_engine_path(Some(tmp.path()));
        assert!(path.exists());
        let version = read_metadata_desktop_version(&path);
        assert_eq!(version.as_deref(), Some("1.5.0"));
    }

    #[test]
    fn metadata_desktop_version_missing_field() {
        let tmp = tmp();
        let engine_dir = tmp.path().join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(&engine_dir).unwrap();
        fs::write(engine_dir.join("metadata.desktop"), "[SddmGreeterTheme]\nName=Silent\n").unwrap();
        assert!(read_metadata_desktop_version(&resolve_engine_path(Some(tmp.path()))).is_none());
    }

    // ── Ryzora manifest I/O ───────────────────────────────────────────────────

    #[test]
    fn manifest_absent_returns_none() {
        let tmp = tmp();
        assert!(load_ryzora_manifest(tmp.path()).is_none());
    }

    #[test]
    fn manifest_present_parsed() {
        let tmp = tmp();
        let data_dir = silentsddm_data_dir(tmp.path());
        fs::create_dir_all(&data_dir).unwrap();

        let manifest = SilentSddmManifest {
            provider: "silentsddm".to_string(),
            upstream_sha: UPSTREAM_SHA.to_string(),
            version: "1.5.0".to_string(),
            engine_path: "/usr/share/sddm/themes/ryzora-silent".to_string(),
            installed_at: "2026-09-21T00:00:00Z".to_string(),
            active_wallpaper_id: None,
            active_targets: SilentSddmTargets::default(),
            sddm_active: false,
            previous_sddm_theme: None,
        };

        fs::write(
            data_dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let loaded = load_ryzora_manifest(tmp.path()).unwrap();
        assert_eq!(loaded.provider, "silentsddm");
        assert_eq!(loaded.upstream_sha, UPSTREAM_SHA);
        assert_eq!(loaded.version, "1.5.0");
        assert!(!loaded.sddm_active);
    }

    #[test]
    fn manifest_corrupt_returns_none() {
        let tmp = tmp();
        let data_dir = silentsddm_data_dir(tmp.path());
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("manifest.json"), "{ not valid json ---").unwrap();
        assert!(load_ryzora_manifest(tmp.path()).is_none());
    }

    // ── Cached asset loading ──────────────────────────────────────────────────

    #[test]
    fn cached_assets_empty_when_manifest_absent() {
        let tmp = tmp();
        let result = load_cached_assets(&tmp.path().join("wallpapers_manifest.json"));
        assert!(result.is_empty());
    }

    #[test]
    fn cached_assets_parsed_from_json() {
        let tmp = tmp();
        let asset = CachedAsset {
            id: "silentsddm-ken".to_string(),
            filename: "ken.mp4".to_string(),
            asset_type: AssetType::Upstream,
            media_type: MediaType::Video,
            sha256: "abc123".to_string(),
            size_bytes: 6699629,
            installed_at: "2026-09-21T00:00:00Z".to_string(),
            upstream_url: Some("https://raw.githubusercontent.com/...".to_string()),
            original_path: None,
        };

        let mut map = std::collections::HashMap::new();
        map.insert("silentsddm-ken".to_string(), asset);

        let manifest_path = tmp.path().join("wallpapers_manifest.json");
        fs::write(&manifest_path, serde_json::to_string(&map).unwrap()).unwrap();

        let result = load_cached_assets(&manifest_path);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "silentsddm-ken");
        assert_eq!(result[0].size_bytes, 6699629);
        assert_eq!(result[0].media_type, MediaType::Video);
    }

    // ── Constants sanity ──────────────────────────────────────────────────────

    #[test]
    fn gif_is_in_rejected_list() {
        assert!(REJECTED_EXTS.contains(&"gif"));
    }

    #[test]
    fn gif_not_in_supported_lists() {
        assert!(!SUPPORTED_VIDEO_EXTS.contains(&"gif"));
        assert!(!SUPPORTED_IMAGE_EXTS.contains(&"gif"));
    }

    #[test]
    fn mp4_in_video_list_not_image() {
        assert!(SUPPORTED_VIDEO_EXTS.contains(&"mp4"));
        assert!(!SUPPORTED_IMAGE_EXTS.contains(&"mp4"));
    }

    #[test]
    fn upstream_sha_non_empty() {
        assert!(!UPSTREAM_SHA.is_empty());
        assert_eq!(UPSTREAM_SHA.len(), 40, "Should be a full 40-char git SHA");
    }

    #[test]
    fn ryzora_slug_prefix() {
        assert!(RYZORA_SILENT_SLUG.starts_with("ryzora-"),
            "Slug must be Ryzora-namespaced, got: {}", RYZORA_SILENT_SLUG);
    }
}
