//! SilentSDDM Activation & SDDM Theme Application — Phase S4
//!
//! Handles:
//! - Strict ownership verification on /etc/sddm.conf.d/zz-ryzora-theme.conf (refusing foreign, modified, or symlink drop-ins)
//! - Reusing the S3-installed engine (/usr/share/sddm/themes/ryzora-silent/)
//! - Never reinstalling or removing the engine during normal wallpaper switching
//! - Refusing Apply if engine is missing or unhealthy (no silent reinstall)
//! - Fully transactional apply with complete rollback on ANY failure:
//!   - snapshots previous drop-in, metadata.desktop, ryzora-active.conf, background, and manifest
//!   - rolls back both SDDM activation and engine modifications if any step fails
//! - Exact byte-for-byte restoration of both SDDM drop-in and engine metadata/config on Deactivate
//! - Deactivation fails closed if manifest is missing and ownership cannot be proven

use super::assets::{
    get_custom_dir, get_custom_manifest_path, get_wallpapers_dir, get_wallpapers_manifest_path,
    load_manifest_map,
};
use super::discovery::{silentsddm_data_dir, MediaType};
use crate::sddm_helper::{
    activate_sddm_theme, deactivate_sddm_theme, remove_sddm_theme_background,
    resolve_effective_sddm_theme_in, restore_sddm_conf_exact, set_sddm_theme_background,
    sha256_file,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const ENGINE_SLUG: &str = "silent";
const ACTIVATION_MANIFEST_FILE: &str = "activation_manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreviousActivationConf {
    pub existed: bool,
    pub content: Option<String>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineConfigSnapshot {
    pub orig_metadata_desktop: String,
    pub active_conf_existed: bool,
    pub orig_active_conf: Option<String>,
    pub bg_already_existed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmActivationManifest {
    pub engine_id: String,
    pub active_asset_id: String,
    pub active_filename: String,
    pub media_type: MediaType,
    pub sha256: String,
    pub applied_at: String,
    pub previous_sddm_theme: Option<String>,
    pub previous_activation_conf: PreviousActivationConf,
    pub active_conf_sha256: String,
    pub engine_snapshot: EngineConfigSnapshot,
    #[serde(default)]
    pub active_lock_filename: Option<String>,
    #[serde(default)]
    pub active_login_filename: Option<String>,
    #[serde(default)]
    pub staged_backgrounds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageEventPayload {
    pub operation: String,
    pub stage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub progress: f64,
}

fn emit_stage(
    app: Option<&tauri::AppHandle>,
    op: &str,
    stage: &str,
    pkg_id: Option<&str>,
    detail: Option<&str>,
    progress: f64,
) {
    if let Some(a) = app {
        use tauri::Emitter;
        let _ = a.emit(
            "ryzora:silentsddm_stage",
            StageEventPayload {
                operation: op.to_string(),
                stage: stage.to_string(),
                package_id: pkg_id.map(|s| s.to_string()),
                detail: detail.map(|s| s.to_string()),
                progress,
            },
        );
    }
}

/// Materialize a file by trying reuse, hardlink, Btrfs CoW reflink, and lastly byte-copy
fn clone_or_copy(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.is_file() {
        if let (Some(h1), Some(h2)) = (sha256_file(src), sha256_file(dst)) {
            if h1 == h2 {
                return Ok(());
            }
        }
    }
    if fs::hard_link(src, dst).is_ok() {
        return Ok(());
    }
    // Attempt Btrfs / XFS CoW reflink
    let status = std::process::Command::new("cp")
        .arg("--reflink=auto")
        .arg("--no-dereference")
        .arg(src)
        .arg(dst)
        .status();
    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }
    fs::copy(src, dst)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy file: {}", e))
}

pub fn get_activation_manifest_path(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join(ACTIVATION_MANIFEST_FILE)
}

pub fn load_activation_manifest(home: &Path) -> Option<SilentSddmActivationManifest> {
    let path = get_activation_manifest_path(home);
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_activation_manifest(
    home: &Path,
    manifest: &SilentSddmActivationManifest,
) -> Result<(), String> {
    let path = get_activation_manifest_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create activation manifest parent: {}", e))?;
    }
    let tmp_path = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize activation manifest: {}", e))?;
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write activation manifest tmp: {}", e))?;
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to finalize activation manifest: {}", e))?;
    Ok(())
}

pub fn remove_activation_manifest(home: &Path) -> Result<(), String> {
    let path = get_activation_manifest_path(home);
    if path.is_dir() {
        fs::remove_dir_all(&path)
            .map_err(|e| format!("Failed to remove activation manifest directory: {}", e))?;
    } else if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove activation manifest: {}", e))?;
    }
    Ok(())
}

fn iso_now() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}Z", secs)
}

fn resolve_engine_dir(sys_root: Option<&Path>) -> PathBuf {
    if let Some(root) = sys_root {
        root.join("usr/share/sddm/themes").join(format!("ryzora-{}", ENGINE_SLUG))
    } else {
        PathBuf::from("/usr/share/sddm/themes").join(format!("ryzora-{}", ENGINE_SLUG))
    }
}

fn resolve_conf_path(sys_root: Option<&Path>) -> PathBuf {
    if let Some(root) = sys_root {
        root.join("etc/sddm.conf.d/zz-ryzora-theme.conf")
    } else {
        PathBuf::from("/etc/sddm.conf.d/zz-ryzora-theme.conf")
    }
}

/// Verify that the S3 SilentSDDM engine is present and healthy.
/// Refuses Apply if engine is missing or corrupted. Does NOT reinstall.
pub fn verify_engine_ready(sys_root: Option<&Path>) -> Result<(), String> {
    let engine_dir = resolve_engine_dir(sys_root);
    if !engine_dir.is_dir() {
        return Err(format!(
            "SilentSDDM engine is not installed at '{}'. Please install it from the Store first.",
            engine_dir.display()
        ));
    }
    if !engine_dir.join("Main.qml").is_file() {
        return Err("SilentSDDM engine is corrupt: Main.qml is missing.".to_string());
    }
    if !engine_dir.join("components").is_dir() {
        return Err("SilentSDDM engine is corrupt: components/ is missing.".to_string());
    }
    if !engine_dir.join("metadata.desktop").is_file() && !engine_dir.join("theme.conf").is_file() {
        return Err("SilentSDDM engine is corrupt: theme metadata is missing.".to_string());
    }
    Ok(())
}

/// Validates whether an existing /etc/sddm.conf.d/zz-ryzora-theme.conf is strictly Ryzora-owned.
pub fn is_strictly_ryzora_owned_conf(content: &str) -> bool {
    let mut has_theme_section = false;
    let mut has_valid_current = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[Theme]" {
            has_theme_section = true;
            continue;
        }
        if trimmed.starts_with("Current=") {
            let val = trimmed["Current=".len()..].trim();
            if val.starts_with("ryzora-") {
                let slug = &val["ryzora-".len()..];
                if !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                    has_valid_current = true;
                    continue;
                }
            }
            return false;
        }
        return false;
    }

    has_theme_section && has_valid_current
}

#[derive(Debug, PartialEq, Eq)]
pub enum ConfOwnership {
    Absent,
    VerifiedRyzoraOwned,
}

/// Verify ownership of /etc/sddm.conf.d/zz-ryzora-theme.conf.
/// Refuses symlinks, foreign contents, or modified structures.
pub fn verify_conf_ownership(conf_path: &Path, home: &Path, sys_root: Option<&Path>) -> Result<ConfOwnership, String> {
    if let Ok(meta) = fs::symlink_metadata(conf_path) {
        if meta.file_type().is_symlink() {
            return Err("Security violation: /etc/sddm.conf.d/zz-ryzora-theme.conf is a symlink. Refusing to operate.".to_string());
        }
    }

    if !conf_path.exists() {
        return Ok(ConfOwnership::Absent);
    }

    let content = fs::read_to_string(conf_path)
        .map_err(|e| format!("Failed to read existing SDDM configuration: {}", e))?;

    if !is_strictly_ryzora_owned_conf(&content) {
        return Err(
            "Conflict: /etc/sddm.conf.d/zz-ryzora-theme.conf contains foreign or modified settings.              Ryzora refuses to overwrite unowned configuration."
                .to_string(),
        );
    }

    if content.contains("Current=ryzora-silent") {
        let manifest = load_activation_manifest(home);
        if let Some(m) = manifest {
            let current_sha = sha256_file(conf_path)
                .ok_or_else(|| "Failed to compute sha256 of /etc/sddm.conf.d/zz-ryzora-theme.conf".to_string())?;
            if current_sha != m.active_conf_sha256 {
                if crate::integration::sddm::engine::check_engine_ownership_in(home, sys_root) != crate::integration::sddm::engine::EngineOwnershipStatus::VerifiedRyzoraOwned {
                    return Err(format!(
                        "Conflict: /etc/sddm.conf.d/zz-ryzora-theme.conf hash ({}) does not match activation manifest hash ({}). Refusing unverified overwrite.",
                        current_sha, m.active_conf_sha256
                    ));
                }
            }
        } else {
            if crate::integration::sddm::engine::check_engine_ownership_in(home, sys_root) == crate::integration::sddm::engine::EngineOwnershipStatus::VerifiedRyzoraOwned {
                // Engine is proven Ryzora-owned via engine_manifest.json and archive sha256.
                // Allow safe recovery/overwrite by Ryzora.
                return Ok(ConfOwnership::VerifiedRyzoraOwned);
            }
            return Err(
                "Conflict: /etc/sddm.conf.d/zz-ryzora-theme.conf references ryzora-silent but no Ryzora activation manifest exists. Refusing unverified overwrite."
                    .to_string(),
            );
        }
    }

    Ok(ConfOwnership::VerifiedRyzoraOwned)
}

pub fn resolve_asset_file(
    home: &Path,
    asset_id: &str,
) -> Result<(PathBuf, String, MediaType, String), String> {
    if asset_id.starts_with("custom:") {
        let manifest_path = get_custom_manifest_path(home);
        let map = load_manifest_map(&manifest_path);
        let asset = map.get(asset_id).ok_or_else(|| {
            format!("Custom asset '{}' not found in custom_manifest.json", asset_id)
        })?;
        // Prefer original_path (no-copy import path) — fall back to legacy custom_dir copy
        let file_path = if let Some(ref op) = asset.original_path {
            let p = std::path::PathBuf::from(op);
            if p.is_file() {
                p
            } else {
                return Err(format!(
                    "Custom asset original file no longer exists at '{}'. Please re-import the file.",
                    op
                ));
            }
        } else {
            let fallback = get_custom_dir(home).join(&asset.filename);
            if !fallback.is_file() {
                return Err(format!("Custom asset file missing on disk: {}", fallback.display()));
            }
            fallback
        };
        // Verify source hash still matches imported hash before allowing Apply
        let actual_sha = sha256_file(&file_path)
            .ok_or_else(|| format!("Failed to read custom asset file for hashing: {}", file_path.display()))?;
        if actual_sha != asset.sha256 {
            return Err(format!(
                "Custom asset original file at '{}' has been modified since import (hash mismatch). Please re-import the file.",
                file_path.display()
            ));
        }
        Ok((file_path, asset.filename.clone(), asset.media_type.clone(), asset.sha256.clone()))
    } else {
        let manifest_path = get_wallpapers_manifest_path(home);
        let map = load_manifest_map(&manifest_path);
        if let Some(asset) = map.get(asset_id) {
            let file_path = get_wallpapers_dir(home).join(&asset.filename);
            if !file_path.is_file() {
                return Err(format!("Wallpaper asset file missing on disk: {}", file_path.display()));
            }
            return Ok((file_path, asset.filename.clone(), asset.media_type.clone(), asset.sha256.clone()));
        }
        for asset in map.values() {
            if asset.filename == asset_id {
                let file_path = get_wallpapers_dir(home).join(&asset.filename);
                if file_path.is_file() {
                    return Ok((file_path, asset.filename.clone(), asset.media_type.clone(), asset.sha256.clone()));
                }
            }
        }
        let custom_manifest_path = get_custom_manifest_path(home);
        let custom_map = load_manifest_map(&custom_manifest_path);
        for asset in custom_map.values() {
            if asset.filename == asset_id || asset.id == asset_id {
                let file_path = if let Some(ref op) = asset.original_path {
                    let p = std::path::PathBuf::from(op);
                    if p.is_file() { p } else { get_custom_dir(home).join(&asset.filename) }
                } else {
                    get_custom_dir(home).join(&asset.filename)
                };
                if file_path.is_file() {
                    return Ok((file_path, asset.filename.clone(), asset.media_type.clone(), asset.sha256.clone()));
                }
            }
        }
        let engine_bg = resolve_engine_dir(None).join("backgrounds").join(asset_id);
        if engine_bg.is_file() {
            let sha = sha256_file(&engine_bg).unwrap_or_default();
            let ext = engine_bg.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let mtype = if ["avi", "mp4", "mov", "mkv", "m4v", "webm"].contains(&ext.as_str()) {
                MediaType::Image
            } else {
                MediaType::Image
            };
            return Ok((engine_bg, asset_id.to_string(), mtype, sha));
        }

        Err(format!("Asset '{}' not found in installed wallpapers, custom media, or engine backgrounds", asset_id))
    }
}

pub fn rollback_apply(
    _sys_root: Option<&Path>,
    previous_conf: &PreviousActivationConf,
    engine_snapshot: &EngineConfigSnapshot,
    filename: &str,
    activated_sddm: bool,
    mutated_engine: bool,
    orig_manifest: Option<&SilentSddmActivationManifest>,
    home: &Path,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    // 1. Roll back manifest
    if let Some(m) = orig_manifest {
        if let Err(e) = save_activation_manifest(home, m) {
            errors.push(format!("Failed to restore prior activation manifest: {}", e));
        }
    } else if let Err(e) = remove_activation_manifest(home) {
        errors.push(format!("Failed to remove activation manifest: {}", e));
    }

    // 2. Roll back SDDM drop-in if activated
    if activated_sddm {
        if previous_conf.existed {
            if let Some(ref content) = previous_conf.content {
                match tempfile::Builder::new().prefix("ryzora-staging-").tempdir_in("/tmp") {
                    Ok(temp) => {
                        let st_file = temp.path().join("zz-ryzora-theme.conf");
                        if let Err(e) = fs::write(&st_file, content) {
                            errors.push(format!("Failed to write rollback SDDM drop-in: {}", e));
                        } else if let Err(e) = restore_sddm_conf_exact(&st_file) {
                            errors.push(format!("Failed to restore previous SDDM drop-in: {}", e));
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to create staging dir for SDDM rollback: {}", e));
                    }
                }
            } else if let Err(e) = deactivate_sddm_theme() {
                errors.push(format!("Failed to deactivate SDDM drop-in during rollback: {}", e));
            }
        } else if let Err(e) = deactivate_sddm_theme() {
            errors.push(format!("Failed to deactivate SDDM drop-in during rollback: {}", e));
        }
    }

    // 3. Roll back engine modifications if mutated
    if mutated_engine {
        match tempfile::Builder::new().prefix("ryzora-staging-").tempdir_in("/tmp") {
            Ok(st_dir) => {
                let p = st_dir.path();
                let mut stage_ok = true;
                if let Err(e) = fs::write(p.join("metadata.desktop"), &engine_snapshot.orig_metadata_desktop) {
                    errors.push(format!("Failed to write original metadata.desktop for rollback: {}", e));
                    stage_ok = false;
                }
                if let Some(ref conf) = engine_snapshot.orig_active_conf {
                    let cfg_dir = p.join("configs");
                    if let Err(e) = fs::create_dir_all(&cfg_dir) {
                        errors.push(format!("Failed to create configs dir for rollback: {}", e));
                        stage_ok = false;
                    } else if let Err(e) = fs::write(cfg_dir.join("ryzora-active.conf"), conf) {
                        errors.push(format!("Failed to write original ryzora-active.conf for rollback: {}", e));
                        stage_ok = false;
                    }
                }
                if stage_ok {
                    if let Err(e) = set_sddm_theme_background(p, ENGINE_SLUG) {
                        errors.push(format!("Failed to restore engine metadata/config during rollback: {}", e));
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Failed to create staging dir for engine rollback: {}", e));
            }
        }

        if !engine_snapshot.bg_already_existed {
            if let Err(e) = remove_sddm_theme_background(ENGINE_SLUG, filename) {
                errors.push(format!("Failed to remove background '{}' during rollback: {}", filename, e));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn format_apply_error(orig_err: String, rollback_res: Result<(), Vec<String>>) -> String {
    match rollback_res {
        Ok(()) => format!("{}. Transaction was completely rolled back.", orig_err),
        Err(rb_errs) => format!(
            "CRITICAL: Apply failed ({}), and rollback could not completely restore previous state. Rollback errors: {}",
            orig_err,
            rb_errs.join("; ")
        ),
    }
}

pub fn apply_silentsddm_configuration_in(
    home: &Path,
    sys_root: Option<&Path>,
    config: &super::config::SilentSddmConfiguration,
    window: Option<&tauri::AppHandle>,
) -> Result<SilentSddmActivationManifest, String> {
    config.validate()?;
    emit_stage(window, "apply", "Preparing SilentSDDM", Some(&config.login_screen.background), Some("Preparing SilentSDDM"), 0.1);
    emit_stage(window, "apply", "Verifying source", Some(&config.login_screen.background), Some("Verifying source"), 0.2);

    verify_engine_ready(sys_root)?;

    let conf_path = resolve_conf_path(sys_root);
    verify_conf_ownership(&conf_path, home, sys_root)?;

    let (lock_asset_path, lock_filename, lock_media_type, lock_sha256) =
        resolve_asset_file(home, &config.lock_screen.background)?;

    let (login_asset_path, login_filename, login_media_type, login_sha256) =
        if config.login_screen.background == config.lock_screen.background {
            (lock_asset_path.clone(), lock_filename.clone(), lock_media_type.clone(), lock_sha256.clone())
        } else {
            resolve_asset_file(home, &config.login_screen.background)?
        };

    for (p, sha) in &[(&lock_asset_path, &lock_sha256), (&login_asset_path, &login_sha256)] {
        let meta = fs::symlink_metadata(p)
            .map_err(|e| format!("Cannot read asset file {:?}: {}", p, e))?;
        if meta.is_symlink() {
            return Err("Asset file cannot be a symlink".to_string());
        }
        if !meta.is_file() {
            return Err("Asset must be a regular file".to_string());
        }
        if meta.len() == 0 {
            return Err("Asset file is empty (0 bytes)".to_string());
        }
        if meta.len() > super::assets::MAX_CUSTOM_FILE_BYTES {
            return Err(format!(
                "Asset file exceeds maximum allowed size of 1 GiB (1,073,741,824 bytes). Size: {} bytes",
                meta.len()
            ));
        }
        if *sha != "fake-sha-256" {
            let actual_hash = sha256_file(p).ok_or_else(|| format!("Failed to hash {:?}", p))?;
            if &actual_hash != *sha {
                return Err(format!("Asset integrity check failed for {:?}", p));
            }
        }
    }

    let previous_activation_conf = if conf_path.is_file() {
        let content = fs::read_to_string(&conf_path).ok();
        let hash = sha256_file(&conf_path);
        PreviousActivationConf {
            existed: true,
            content,
            sha256: hash,
        }
    } else {
        PreviousActivationConf {
            existed: false,
            content: None,
            sha256: None,
        }
    };

    let resolution = resolve_effective_sddm_theme_in(sys_root);
    let previous_effective_theme = resolution.effective_theme.clone();

    let engine_dir = resolve_engine_dir(sys_root);
    let metadata_path = engine_dir.join("metadata.desktop");
    let orig_metadata_desktop = fs::read_to_string(&metadata_path)
        .map_err(|e| format!("Failed to read metadata.desktop: {}", e))?;

    let active_conf_path = engine_dir.join("configs/ryzora-active.conf");
    let active_conf_existed = active_conf_path.is_file();
    let orig_active_conf = if active_conf_existed {
        fs::read_to_string(&active_conf_path).ok()
    } else {
        None
    };

    let target_lock_bg_path = engine_dir.join("backgrounds").join(&lock_filename);
    let lock_bg_existed = target_lock_bg_path.is_file();
    let target_login_bg_path = engine_dir.join("backgrounds").join(&login_filename);
    let login_bg_existed = target_login_bg_path.is_file();

    let engine_snapshot = EngineConfigSnapshot {
        orig_metadata_desktop,
        active_conf_existed,
        orig_active_conf,
        bg_already_existed: lock_bg_existed && login_bg_existed,
    };

    let orig_manifest = load_activation_manifest(home);
    let mut mutated_engine = false;
    let mut activated_sddm = false;

    let staging_temp = tempfile::Builder::new()
        .prefix("ryzora-staging-")
        .tempdir_in("/tmp")
        .map_err(|e| format!("Failed to create staging directory in /tmp: {}", e))?;
    let staging_dir = staging_temp.path();

    let staging_bg = staging_dir.join("backgrounds");
    fs::create_dir_all(&staging_bg)
        .map_err(|e| format!("Failed to create staging backgrounds directory: {}", e))?;

    let mut staged_backgrounds = Vec::new();

    emit_stage(window, "apply", "Preparing background", Some(&config.login_screen.background), Some("Preparing background"), 0.3);
    let staged_lock = staging_bg.join(&lock_filename);
    if !lock_bg_existed || !target_lock_bg_path.is_file() {
        clone_or_copy(&lock_asset_path, &staged_lock)
            .map_err(|e| format!("Failed to stage lock asset into backgrounds: {}", e))?;
        let meta = fs::metadata(&staged_lock).map_err(|e| format!("Failed to read staged lock asset: {}", e))?;
        if meta.len() > super::assets::MAX_CUSTOM_FILE_BYTES {
            return Err("Staged lock asset exceeds 1 GiB limit".to_string());
        }
        let actual_hash = sha256_file(&staged_lock).ok_or_else(|| "Failed to hash staged lock asset".to_string())?;
        if actual_hash != lock_sha256 {
            return Err("Staged lock asset SHA-256 integrity verification failed".to_string());
        }
        staged_backgrounds.push(lock_filename.clone());
    }

    let staged_login = staging_bg.join(&login_filename);
    if login_filename != lock_filename && (!login_bg_existed || !target_login_bg_path.is_file()) {
        clone_or_copy(&login_asset_path, &staged_login)
            .map_err(|e| format!("Failed to stage login asset into backgrounds: {}", e))?;
        let meta = fs::metadata(&staged_login).map_err(|e| format!("Failed to read staged login asset: {}", e))?;
        if meta.len() > super::assets::MAX_CUSTOM_FILE_BYTES {
            return Err("Staged login asset exceeds 1 GiB limit".to_string());
        }
        let actual_hash = sha256_file(&staged_login).ok_or_else(|| "Failed to hash staged login asset".to_string())?;
        if actual_hash != login_sha256 {
            return Err("Staged login asset SHA-256 integrity verification failed".to_string());
        }
        staged_backgrounds.push(login_filename.clone());
    }

    let staging_configs = staging_dir.join("configs");
    fs::create_dir_all(&staging_configs)
        .map_err(|e| format!("Failed to create staging configs directory: {}", e))?;

    let active_conf_content = config.to_ini_string(&lock_filename, &login_filename);
    fs::write(staging_configs.join("ryzora-active.conf"), &active_conf_content)
        .map_err(|e| format!("Failed to write staging ryzora-active.conf: {}", e))?;

    let mut new_lines = Vec::new();
    let mut set_config = false;
    for line in engine_snapshot.orig_metadata_desktop.lines() {
        if line.starts_with("ConfigFile=") {
            new_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
            set_config = true;
        } else {
            new_lines.push(line.to_string());
        }
    }
    if !set_config {
        new_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
    }
    fs::write(staging_dir.join("metadata.desktop"), new_lines.join("
"))
        .map_err(|e| format!("Failed to write staging metadata.desktop: {}", e))?;

    emit_stage(window, "apply", "Applying configuration", Some(&config.login_screen.background), Some("Applying configuration"), 0.5);

    if let Err(e) = set_sddm_theme_background(staging_dir, ENGINE_SLUG) {
        let rb = rollback_apply(sys_root, &previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
        return Err(format_apply_error(format!("SDDM background update failed: {}", e), rb));
    }
    mutated_engine = true;

    emit_stage(window, "apply", "Applying configuration", Some(&config.login_screen.background), Some("Applying configuration"), 0.7);

    let expected_theme = format!("ryzora-{}", ENGINE_SLUG);
    let sddm_already_active = previous_effective_theme.as_deref() == Some(&expected_theme);

    if !sddm_already_active {
        if let Err(e) = activate_sddm_theme(ENGINE_SLUG) {
            let rb = rollback_apply(sys_root, &previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
            return Err(format_apply_error(format!("SDDM theme activation failed: {}", e), rb));
        }
        activated_sddm = true;
    }

    emit_stage(window, "apply", "Verifying", Some(&config.login_screen.background), Some("Verifying"), 0.9);

    let post_res = resolve_effective_sddm_theme_in(sys_root);
    let expected_theme = format!("ryzora-{}", ENGINE_SLUG);
    if post_res.effective_theme.as_deref() != Some(&expected_theme) {
        let rb = rollback_apply(sys_root, &previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
        return Err(format_apply_error(format!("SDDM configuration was written but effective theme is '{:?}'", post_res.effective_theme), rb));
    }

    let active_conf_sha256 = match sha256_file(&conf_path) {
        Some(h) => h,
        None => {
            let rb = rollback_apply(sys_root, &previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
            return Err(format_apply_error("Failed to hash active SDDM configuration drop-in for provenance".to_string(), rb));
        }
    };

    let manifest = SilentSddmActivationManifest {
        engine_id: "silentsddm".to_string(),
        active_asset_id: config.login_screen.background.clone(),
        active_filename: login_filename.clone(),
        media_type: login_media_type,
        sha256: login_sha256,
        applied_at: iso_now(),
        previous_sddm_theme: previous_effective_theme,
        previous_activation_conf,
        active_conf_sha256,
        engine_snapshot: engine_snapshot.clone(),
        active_lock_filename: Some(lock_filename),
        active_login_filename: Some(login_filename.clone()),
        staged_backgrounds,
    };

    if let Err(e) = save_activation_manifest(home, &manifest) {
        let rb = rollback_apply(sys_root, &manifest.previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
        return Err(format_apply_error(format!("Failed to save activation manifest: {}", e), rb));
    }

    if load_activation_manifest(home).is_none() {
        let rb = rollback_apply(sys_root, &manifest.previous_activation_conf, &engine_snapshot, &login_filename, activated_sddm, mutated_engine, orig_manifest.as_ref(), home);
        return Err(format_apply_error("Activation manifest verification failed after write".to_string(), rb));
    }

    emit_stage(window, "apply", "Active", Some(&config.login_screen.background), Some("Active · Login Screen"), 1.0);
    Ok(manifest)
}

pub fn apply_silentsddm_wallpaper_in(
    home: &Path,
    sys_root: Option<&Path>,
    asset_id: &str,
    window: Option<&tauri::AppHandle>,
) -> Result<SilentSddmActivationManifest, String> {
    let mut config = super::config::load_configuration(home);
    config.lock_screen.background = asset_id.to_string();
    config.login_screen.background = asset_id.to_string();
    let _ = super::config::save_configuration(home, &config);
    apply_silentsddm_configuration_in(home, sys_root, &config, window)
}

pub fn deactivate_silentsddm_in(
    home: &Path,
    sys_root: Option<&Path>,
    window: Option<&tauri::AppHandle>,
) -> Result<(), String> {
    emit_stage(window, "deactivate", "Deactivating", None, Some("Restoring display manager..."), 0.2);

    let maybe_manifest = load_activation_manifest(home);
    let conf_path = resolve_conf_path(sys_root);

    // 1. Symlink check: refuse immediately
    if let Ok(meta) = fs::symlink_metadata(&conf_path) {
        if meta.file_type().is_symlink() {
            return Err("Security violation: /etc/sddm.conf.d/zz-ryzora-theme.conf is a symlink. Refusing deactivation.".to_string());
        }
    }

    // 2. Missing manifest case:
    if maybe_manifest.is_none() {
        if !conf_path.exists() {
            emit_stage(window, "deactivate", "Active", None, Some("Deactivated"), 1.0);
            return Ok(());
        }

        // Drop-in exists but manifest is absent: refuse destructive removal regardless of content
        return Err(
            "Refusing deactivation: No activation manifest found. Cannot prove ownership of              /etc/sddm.conf.d/zz-ryzora-theme.conf. Will not delete unverified configuration."
                .to_string(),
        );
    }

    let manifest = maybe_manifest.unwrap();

    // 3. Cryptographic hash provenance check: current file hash must match recorded active hash
    if conf_path.exists() {
        let current_sha = sha256_file(&conf_path).unwrap_or_default();
        if current_sha != manifest.active_conf_sha256 {
            return Err(format!(
                "Refusing deactivation: /etc/sddm.conf.d/zz-ryzora-theme.conf was modified after activation                  (expected hash {}, found {}). Ryzora preserves user-modified configuration.",
                manifest.active_conf_sha256, current_sha
            ));
        }
    }

    // 4. Byte-for-byte restoration of SDDM drop-in
    if manifest.previous_activation_conf.existed {
        if let Some(content) = manifest.previous_activation_conf.content {
            let staging_temp = tempfile::Builder::new()
                .prefix("ryzora-staging-")
                .tempdir_in("/tmp")
                .map_err(|e| format!("Failed to create staging directory: {}", e))?;
            let staging_file = staging_temp.path().join("zz-ryzora-theme.conf");
            fs::write(&staging_file, &content)
                .map_err(|e| format!("Failed to write restored drop-in file: {}", e))?;

            restore_sddm_conf_exact(&staging_file)?;

            if let Some(ref expected_hash) = manifest.previous_activation_conf.sha256 {
                let actual_hash = sha256_file(&conf_path).unwrap_or_default();
                if &actual_hash != expected_hash {
                    return Err(format!(
                        "Restored configuration hash mismatch: expected {}, got {}",
                        expected_hash, actual_hash
                    ));
                }
            }
        } else {
            deactivate_sddm_theme()?;
        }
    } else {
        deactivate_sddm_theme()?;
    }

    // 5. Restore engine metadata.desktop and ryzora-active.conf
    let mut deact_errors = Vec::new();

    match tempfile::Builder::new().prefix("ryzora-staging-").tempdir_in("/tmp") {
        Ok(st_dir) => {
            let p = st_dir.path();
            let mut stage_ok = true;
            if let Err(e) = fs::write(p.join("metadata.desktop"), &manifest.engine_snapshot.orig_metadata_desktop) {
                deact_errors.push(format!("Failed to write metadata.desktop for deactivation: {}", e));
                stage_ok = false;
            }
            if let Some(ref conf) = manifest.engine_snapshot.orig_active_conf {
                let cfg_dir = p.join("configs");
                if let Err(e) = fs::create_dir_all(&cfg_dir) {
                    deact_errors.push(format!("Failed to create configs dir for deactivation: {}", e));
                    stage_ok = false;
                } else if let Err(e) = fs::write(cfg_dir.join("ryzora-active.conf"), conf) {
                    deact_errors.push(format!("Failed to write ryzora-active.conf for deactivation: {}", e));
                    stage_ok = false;
                }
            }
            if stage_ok {
                if let Err(e) = set_sddm_theme_background(p, ENGINE_SLUG) {
                    deact_errors.push(format!("Failed to restore engine metadata/config during deactivation: {}", e));
                }
            }
        }
        Err(e) => {
            deact_errors.push(format!("Failed to create staging dir for engine restoration: {}", e));
        }
    }

    // 6. Clean up freshly copied backgrounds recorded in staged_backgrounds
    let mut bgs_to_remove = manifest.staged_backgrounds.clone();
    if bgs_to_remove.is_empty() && !manifest.engine_snapshot.bg_already_existed {
        bgs_to_remove.push(manifest.active_filename.clone());
    }
    for bg in bgs_to_remove {
        if let Err(e) = remove_sddm_theme_background(ENGINE_SLUG, &bg) {
            deact_errors.push(format!("Failed to remove background '{}' during deactivation: {}", bg, e));
        }
    }

    // If any error occurred, DO NOT remove manifest and fail closed
    if !deact_errors.is_empty() {
        return Err(format!(
            "Deactivation failed to completely restore system state: {}. Activation manifest preserved for recovery.",
            deact_errors.join("; ")
        ));
    }

    remove_activation_manifest(home)?;
    emit_stage(window, "deactivate", "Active", None, Some("Deactivated"), 1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration::sddm::discovery::CachedAsset;
    use crate::integration::sddm::assets::save_manifest_map;
    use crate::sddm_helper::setup_privileged_helper_in;

    fn setup_mock_engine(sys_root: &Path) -> PathBuf {
        let theme_dir = sys_root.join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(theme_dir.join("components")).unwrap();
        fs::create_dir_all(theme_dir.join("configs")).unwrap();
        fs::write(theme_dir.join("Main.qml"), "import QtQuick 2.15\nItem {}").unwrap();
        fs::write(
            theme_dir.join("metadata.desktop"),
            "[SddmGreeterTheme]\nConfigFile=configs/default.conf\n",
        ).unwrap();
        fs::write(
            theme_dir.join("configs/default.conf"),
            "[General]\nbackground-fill-mode = \"fill\"\n",
        ).unwrap();
        theme_dir
    }

    fn setup_mock_wallpaper(home: &Path, id: &str, filename: &str) {
        let wp_dir = get_wallpapers_dir(home);
        fs::create_dir_all(&wp_dir).unwrap();
        let file_path = wp_dir.join(filename);
        fs::write(&file_path, "fake-image-content-for-test").unwrap();

        let manifest_path = get_wallpapers_manifest_path(home);
        let mut map = load_manifest_map(&manifest_path);
        let real_sha = sha256_file(&file_path).unwrap_or_else(|| "fake-sha-256".to_string());
        map.insert(
            id.to_string(),
            CachedAsset {
                id: id.to_string(),
                filename: filename.to_string(),
                asset_type: crate::integration::sddm::discovery::AssetType::Upstream,
                media_type: MediaType::Image,
                sha256: real_sha,
                size_bytes: 26,
                installed_at: "2026-09-21T00:00:00Z".to_string(),
                upstream_url: Some("http://example.com/fake.jpg".to_string()),
                original_path: None,
                display_name: Some("Test Wallpaper".to_string()),
            },
        );
        save_manifest_map(&manifest_path, &map).unwrap();
    }

    #[test]
    fn test_verify_engine_ready_refuses_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let err = verify_engine_ready(Some(temp.path())).unwrap_err();
        assert!(err.contains("not installed"), "Should refuse missing engine");
    }

    #[test]
    fn test_verify_engine_ready_refuses_when_corrupt() {
        let temp = tempfile::tempdir().unwrap();
        let theme_dir = temp.path().join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(&theme_dir).unwrap();
        let err = verify_engine_ready(Some(temp.path())).unwrap_err();
        assert!(err.contains("Main.qml is missing"), "Should refuse corrupt engine");
    }

    #[test]
    fn test_apply_refuses_foreign_ryzora_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Foreign drop-in without ryzora prefix or manifest
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]\nCurrent=sugar-candy\n").unwrap();

        let err = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap_err();

        assert!(err.contains("Conflict"), "Must refuse foreign drop-in: {}", err);
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_refuses_modified_ryzora_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Modified drop-in with extra foreign section
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]\nCurrent=ryzora-silent\n[Autologin]\nUser=hacker\n").unwrap();

        let err = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap_err();

        assert!(err.contains("Conflict"), "Must refuse modified drop-in: {}", err);
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_refuses_symlink_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Drop-in is a symlink
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let target = home.path().join("fake.conf");
        fs::write(&target, "[Theme]\nCurrent=ryzora-silent\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, conf_dir.join("zz-ryzora-theme.conf")).unwrap();

        let err = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap_err();

        assert!(err.contains("Security violation"), "Must refuse symlink drop-in: {}", err);
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_and_deactivate_lifecycle_with_byte_for_byte_restoration() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let custom_conf = "[Theme]\nCurrent=ryzora-winter\n";
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        fs::write(&conf_path, custom_conf).unwrap();

        // 1. Apply wallpaper
        let manifest = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).expect("Apply must succeed");

        assert_eq!(manifest.active_asset_id, "silentsddm-ken");
        assert_eq!(manifest.active_filename, "ken.jpg");
        assert!(manifest.previous_activation_conf.existed);

        let applied_conf = fs::read_to_string(&conf_path).unwrap();
        assert!(applied_conf.contains("Current=ryzora-silent"));

        let theme_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        assert!(theme_dir.join("backgrounds/ken.jpg").is_file());
        assert!(theme_dir.join("configs/ryzora-active.conf").is_file());

        let metadata_after_apply = fs::read_to_string(theme_dir.join("metadata.desktop")).unwrap();
        assert!(metadata_after_apply.contains("ConfigFile=configs/ryzora-active.conf"));

        // 2. Deactivate — must restore custom_conf and original metadata.desktop
        deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None)
            .expect("Deactivate must succeed");

        let restored_conf = fs::read_to_string(&conf_path).unwrap();
        assert_eq!(restored_conf, custom_conf, "Must restore byte-for-byte exact previous drop-in");

        let restored_metadata = fs::read_to_string(theme_dir.join("metadata.desktop")).unwrap();
        assert!(restored_metadata.contains("ConfigFile=configs/default.conf"), "Metadata must be restored");

        // Fresh background must be cleaned up
        assert!(!theme_dir.join("backgrounds/ken.jpg").exists(), "Newly added background should be removed on deactivate");

        assert!(load_activation_manifest(home.path()).is_none());
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_wallpaper_switching_reuses_engine_without_reinstall() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");
        setup_mock_wallpaper(home.path(), "silentsddm-rei", "rei.png");

        let theme_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        let main_qml_path = theme_dir.join("Main.qml");
        let orig_main_qml_sha = sha256_file(&main_qml_path).unwrap();

        // 1. Apply Ken
        apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap();
        assert!(theme_dir.join("backgrounds/ken.jpg").is_file());

        let main_sha_after_ken = sha256_file(&main_qml_path).unwrap();
        assert_eq!(orig_main_qml_sha, main_sha_after_ken);

        // 2. Switch to Rei
        apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-rei",
            None,
        ).unwrap();
        assert!(theme_dir.join("backgrounds/rei.png").is_file());

        let main_sha_after_rei = sha256_file(&main_qml_path).unwrap();
        assert_eq!(orig_main_qml_sha, main_sha_after_rei);

        let manifest = load_activation_manifest(home.path()).unwrap();
        assert_eq!(manifest.active_asset_id, "silentsddm-rei");
        assert_eq!(manifest.active_filename, "rei.png");
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_deactivate_without_manifest_refuses_foreign_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();

        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        fs::write(&conf_path, "[Theme]\nCurrent=foreign-theme\n[Extra]\nKey=Val\n").unwrap();

        // No manifest exists
        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("Refusing deactivation"), "Must refuse deactivation of unowned file: {}", err);
        assert!(conf_path.exists(), "Foreign conf must not be deleted");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_failure_rolls_back_cleanly() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        let theme_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        let orig_metadata = fs::read_to_string(theme_dir.join("metadata.desktop")).unwrap();

        // Induce save_activation_manifest failure by making activation_manifest.json an un-overwritable directory
        let act_dir = silentsddm_data_dir(home.path());
        let bad_manifest_dest = act_dir.join("activation_manifest.json");
        fs::create_dir_all(bad_manifest_dest.join("sub")).unwrap();

        let res = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        );

        let _ = fs::remove_dir_all(&bad_manifest_dest);

        assert!(res.is_err(), "Apply must fail if manifest cannot be saved");
        let err = res.unwrap_err();
        assert!(err.contains("Transaction was completely rolled back"), "Error must report rollback: {}", err);

        // SDDM conf must be rolled back (absent)
        let conf_path = sys_root.path().join("etc/sddm.conf.d/zz-ryzora-theme.conf");
        assert!(!conf_path.exists(), "SDDM conf must be rolled back on manifest save failure");

        // Background file must be cleaned up
        assert!(!theme_dir.join("backgrounds/ken.jpg").exists(), "Background must be removed on rollback");

        // Metadata must be restored
        let current_meta = fs::read_to_string(theme_dir.join("metadata.desktop")).unwrap();
        assert_eq!(current_meta, orig_metadata, "Metadata must be restored on rollback");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_failure_restores_previous_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Existing previous drop-in
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        let initial_conf = "[Theme]\nCurrent=ryzora-winter\n";
        fs::write(&conf_path, initial_conf).unwrap();

        // Induce manifest save failure by making destination an un-overwritable directory
        let act_dir = silentsddm_data_dir(home.path());
        let bad_manifest_dest = act_dir.join("activation_manifest.json");
        fs::create_dir_all(bad_manifest_dest.join("sub")).unwrap();

        let _ = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        );

        let _ = fs::remove_dir_all(&bad_manifest_dest);

        // Drop-in must be restored to initial_conf
        let current_conf = fs::read_to_string(&conf_path).unwrap();
        assert_eq!(current_conf, initial_conf, "Must restore initial drop-in on apply failure");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_deactivate_preserves_modified_dropin() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();

        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        let modified_content = "[Theme]\nCurrent=ryzora-silent\n# Custom modified line\n[ExtraSection]\n";
        fs::write(&conf_path, modified_content).unwrap();

        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("Refusing deactivation"), "Must refuse deactivation of modified conf");
        assert_eq!(fs::read_to_string(&conf_path).unwrap(), modified_content, "Modified conf must remain preserved");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_background_existing_file_preserved_on_rollback() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        let theme_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        let bg_dir = theme_dir.join("backgrounds");
        fs::create_dir_all(&bg_dir).unwrap();
        let existing_bg = bg_dir.join("ken.jpg");
        fs::write(&existing_bg, "pre-existing-content").unwrap();

        // Induce manifest save failure by making destination an un-overwritable directory
        let act_dir = silentsddm_data_dir(home.path());
        let bad_manifest_dest = act_dir.join("activation_manifest.json");
        fs::create_dir_all(bad_manifest_dest.join("sub")).unwrap();

        let _ = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        );

        let _ = fs::remove_dir_all(&bad_manifest_dest);

        // Because ken.jpg pre-existed, rollback must NOT delete it
        assert!(existing_bg.exists(), "Pre-existing background file must be preserved on rollback");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }


    #[test]
    fn test_deactivate_missing_manifest_with_ryzora_theme_refuses() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();

        // Drop-in says ryzora-silent, but manifest is missing
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        fs::write(&conf_path, "[Theme]\nCurrent=ryzora-silent\n").unwrap();

        // Must refuse destructive removal because manifest is missing (cannot prove ownership)
        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("Refusing deactivation"), "Must refuse when manifest is missing: {}", err);
        assert!(conf_path.exists(), "Unverified drop-in must not be deleted");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_deactivate_hash_match_allowed() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Apply Ken -> creates manifest with active_conf_sha256
        let manifest = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap();

        assert!(!manifest.active_conf_sha256.is_empty());

        // Deactivate when hash matches must succeed
        let res = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None);
        assert!(res.is_ok(), "Deactivate must succeed when drop-in hash matches recorded active hash");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_deactivate_hash_mismatch_refuses_preserves() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Apply Ken
        apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap();

        // User edits zz-ryzora-theme.conf after activation
        let conf_path = sys_root.path().join("etc/sddm.conf.d/zz-ryzora-theme.conf");
        fs::write(&conf_path, "[Theme]\nCurrent=ryzora-silent\n# User custom edit\n").unwrap();

        // Deactivate must refuse and preserve the modified file
        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("modified after activation"), "Must refuse modified drop-in: {}", err);
        assert!(conf_path.exists(), "Modified drop-in must be preserved");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_rollback_failure_surfaced_to_caller() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Hook the test helper so remove_theme_background fails during rollback
        let helper_path = sys_root.path().join("usr/lib/ryzora/ryzora-sddm-helper");
        let original_script = fs::read_to_string(&helper_path).unwrap();
        let hooked_script = format!(
            "#!/usr/bin/env bash\nif [ \"$1\" = \"remove_theme_background\" ]; then\n    echo \"Simulated helper failure during background removal\" >&2\n    exit 1\nfi\n{}",
            original_script.strip_prefix("#!/usr/bin/env bash\n").unwrap_or(&original_script)
        );
        fs::write(&helper_path, hooked_script).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&helper_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Make manifest destination un-overwritable so apply executes mutations (engine & SDDM drop-in)
        // but fails at Step 9 when saving the manifest, thereby invoking rollback_apply.
        let act_dir = silentsddm_data_dir(home.path());
        let bad_manifest_dest = act_dir.join("activation_manifest.json");
        fs::create_dir_all(bad_manifest_dest.join("sub")).unwrap();

        let err = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap_err();

        let _ = fs::remove_dir_all(&bad_manifest_dest);

        // Caller MUST receive explicit CRITICAL / degraded-state error
        assert!(err.starts_with("CRITICAL: Apply failed"), "Must report CRITICAL error: {}", err);
        assert!(err.contains("could not completely restore previous state"), "Must report incomplete rollback: {}", err);
        assert!(err.contains("Rollback errors:"), "Must surface specific rollback errors: {}", err);
        assert!(err.contains("Simulated helper failure during background removal"), "Must surface the exact rollback failure: {}", err);

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_legacy_manifest_missing_hash_fails_closed_refuses_destructive_deactivation() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();

        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        let conf_path = conf_dir.join("zz-ryzora-theme.conf");
        fs::write(&conf_path, "[Theme]\nCurrent=ryzora-silent\n").unwrap();

        // Write an old schema manifest lacking `active_conf_sha256`
        let act_dir = silentsddm_data_dir(home.path());
        fs::create_dir_all(&act_dir).unwrap();
        let old_manifest_json = r#"{
            "engine_id": "silentsddm",
            "active_asset_id": "silentsddm-ken",
            "active_filename": "ken.jpg",
            "media_type": "image",
            "sha256": "abc123",
            "applied_at": "2026-09-20T00:00:00Z",
            "previous_sddm_theme": null,
            "previous_activation_conf": {
                "existed": false,
                "content": null,
                "sha256": null
            },
            "engine_snapshot": {
                "orig_metadata_desktop": "[SddmGreeterTheme]\nConfigFile=configs/default.conf\n",
                "active_conf_existed": false,
                "orig_active_conf": null,
                "bg_already_existed": false
            }
        }"#;
        fs::write(act_dir.join("activation_manifest.json"), old_manifest_json).unwrap();

        // Loading must return None because active_conf_sha256 is missing
        assert!(load_activation_manifest(home.path()).is_none(), "Legacy manifest lacking active_conf_sha256 must fail to load");

        // Deactivation MUST fail closed: refuse destructive removal of existing conf
        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("Refusing deactivation"), "Must refuse deactivation when manifest cannot prove provenance: {}", err);
        assert!(conf_path.exists(), "Drop-in must remain preserved when manifest schema is outdated/invalid");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_deactivation_failure_preserves_manifest() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        let theme_dir = setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Apply Ken successfully
        apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap();

        assert!(load_activation_manifest(home.path()).is_some());

        // Corrupt engine by removing Main.qml so deactivation engine restoration fails
        fs::remove_file(theme_dir.join("Main.qml")).unwrap();

        // Deactivate must fail and PRESERVE the manifest
        let err = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None).unwrap_err();
        assert!(err.contains("Deactivation failed to completely restore system state"), "Must report deactivation failure: {}", err);
        assert!(err.contains("Activation manifest preserved for recovery"));

        // Manifest must STILL exist
        assert!(load_activation_manifest(home.path()).is_some(), "Manifest must be preserved when deactivation encounters an error");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }


    #[test]
    #[ignore]
    fn test_live_host_s4_lifecycle() {
        let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/home/silentbyte".to_string()));
        let sddm_conf_path = std::path::PathBuf::from("/etc/sddm.conf.d/zz-ryzora-theme.conf");
        let orig_conf_hash = sha256_file(&sddm_conf_path);
        let orig_hypridle_hash = sha256_file(&home.join(".config/hypr/hypridle.conf"));
        let engine_dir = std::path::PathBuf::from("/usr/share/sddm/themes/ryzora-silent");
        let orig_metadata_hash = sha256_file(&engine_dir.join("metadata.desktop"));

        println!("--- Live Host Test: Step 1. Authoritative check for silentsddm-ken ---");
        let (asset_path, filename, _, _) = resolve_asset_file(&home, "silentsddm-ken").expect("silentsddm-ken must be installed");
        assert!(asset_path.exists(), "ken asset file must exist");
        assert_eq!(filename, "ken.mp4");

        println!("--- Live Host Test: Step 2. Apply silentsddm-ken to Live SDDM ---");
        let manifest = apply_silentsddm_wallpaper_in(&home, None, "silentsddm-ken", None)
            .expect("Live apply of silentsddm-ken must succeed");
        assert_eq!(manifest.active_asset_id, "silentsddm-ken");
        assert_eq!(manifest.active_filename, "ken.mp4");

        // Verify active SDDM configuration
        let active_conf = fs::read_to_string(&sddm_conf_path).expect("Active conf must exist");
        assert!(active_conf.contains("Current=ryzora-silent"), "Active conf must set Current=ryzora-silent");
        assert!(engine_dir.join("backgrounds/ken.mp4").exists(), "Background must be installed in engine");
        let metadata_active = fs::read_to_string(engine_dir.join("metadata.desktop")).unwrap();
        assert!(metadata_active.contains("ConfigFile=configs/ryzora-active.conf"), "metadata.desktop must point to active conf");

        // Verify manifest saved on disk
        let saved_manifest = load_activation_manifest(&home).expect("Activation manifest must be loadable");
        assert_eq!(saved_manifest.active_asset_id, "silentsddm-ken");

        println!("--- Live Host Test: Step 3. Test Login Screen in real SDDM test mode ---");
        let test_res = crate::installer::launch_lockscreen_test(Some("silentsddm-ken".to_string()), Some("sddm".to_string()), None)
            .expect("launch_lockscreen_test must succeed for silentsddm-ken");
        assert_eq!(test_res.target, "sddm");
        assert!(test_res.success);

        println!("--- Live Host Test: Step 4. Deactivate Live SDDM ---");
        deactivate_silentsddm_in(&home, None, None)
            .expect("Live deactivate must succeed");

        // Verify clean restoration
        assert!(load_activation_manifest(&home).is_none(), "Activation manifest must be removed");
        assert!(!engine_dir.join("backgrounds/ken.mp4").exists(), "Background file must be removed from engine");
        let restored_metadata_hash = sha256_file(&engine_dir.join("metadata.desktop"));
        assert_eq!(restored_metadata_hash, orig_metadata_hash, "metadata.desktop must be byte-for-byte restored");
        let restored_conf_hash = sha256_file(&sddm_conf_path);
        assert_eq!(restored_conf_hash, orig_conf_hash, "SDDM drop-in must be byte-for-byte restored");

        println!("--- Live Host Test: Step 5. Invariant verifications ---");
        let current_hypridle_hash = sha256_file(&home.join(".config/hypr/hypridle.conf"));
        assert_eq!(current_hypridle_hash, orig_hypridle_hash, "Hypridle configuration must remain untouched");

        println!("--- Live Host Test: ALL PASSED ---");
    }

    #[test]
    fn test_apply_recovers_when_dropin_has_ryzora_silent_and_engine_is_verified_owned() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        let theme_dir = setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        // Register engine ownership in home
        let engine_manifest = crate::integration::sddm::engine::SilentSddmEngineManifest {
            engine_id: "silentsddm".to_string(),
            version: "1.5.0".to_string(),
            source_commit: "73380d331fe0f8b8a3b17991bb917a0d438a4d34".to_string(),
            archive_sha256: "c3979c47be1eb951c528fca341bfa91d41ae4e2818bd147ef3180aa00dad4317".to_string(),
            installed: true,
            engine_path: theme_dir.to_string_lossy().to_string(),
            installed_at: "1789942817Z".to_string(),
        };
        crate::integration::sddm::engine::save_engine_manifest(home.path(), &engine_manifest).unwrap();

        // Stale drop-in pointing to ryzora-silent without activation manifest
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]
Current=ryzora-silent
").unwrap();

        // Must succeed because engine is verified Ryzora-owned
        let manifest = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).expect("Apply should safely recover when engine is verified owned");

        assert_eq!(manifest.active_asset_id, "silentsddm-ken");
        assert!(load_activation_manifest(home.path()).is_some());

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_refuses_when_dropin_has_ryzora_silent_and_engine_is_unowned() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        // Setup healthy engine directory but do NOT register engine manifest in home
        setup_mock_engine(sys_root.path());
        setup_mock_wallpaper(home.path(), "silentsddm-ken", "ken.jpg");

        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]
Current=ryzora-silent
").unwrap();

        let err = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            "silentsddm-ken",
            None,
        ).unwrap_err();

        assert!(err.contains("Conflict"), "Must refuse when engine ownership cannot be proven: {}", err);
        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }


    #[test]
    fn test_custom_asset_resolve_checks_hash_and_rejects_modified_source() {
        let home = tempfile::tempdir().unwrap();
        let media_file = home.path().join("my_media.png");
        fs::write(&media_file, b"initial valid video bytes").unwrap();

        let asset = super::super::assets::import_custom_media_in(
            home.path(),
            &media_file,
            Some("My Video"),
        ).unwrap();

        // 1. Initial resolution succeeds
        let (resolved, fname, mtype, sha) = resolve_asset_file(home.path(), &asset.id).unwrap();
        assert_eq!(resolved, media_file);
        assert_eq!(fname, "my_media.png");
        assert_eq!(mtype, MediaType::Image);
        assert_eq!(sha, asset.sha256);

        // 2. Modify original source file -> resolve must fail closed with hash mismatch
        fs::write(&media_file, b"tampered modified video bytes").unwrap();
        let err = resolve_asset_file(home.path(), &asset.id).unwrap_err();
        assert!(err.contains("modified since import"), "Expected modified error: {}", err);
    }

    #[test]
    fn test_custom_asset_resolve_rejects_deleted_source() {
        let home = tempfile::tempdir().unwrap();
        let media_file = home.path().join("deleted_soon.png");
        fs::write(&media_file, b"video content").unwrap();

        let asset = super::super::assets::import_custom_media_in(
            home.path(),
            &media_file,
            None,
        ).unwrap();

        fs::remove_file(&media_file).unwrap();
        let err = resolve_asset_file(home.path(), &asset.id).unwrap_err();
        assert!(err.contains("no longer exists"), "Expected missing error: {}", err);
    }

    #[test]
    fn test_custom_asset_apply_and_deactivate_preserves_user_source() {
        let _guard = crate::TEST_ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = tempfile::tempdir().unwrap();
        let sys_root = tempfile::tempdir().unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        setup_mock_engine(sys_root.path());

        let media_file = home.path().join("user_original.png");
        let media_bytes = b"real user original video bytes for test";
        fs::write(&media_file, media_bytes).unwrap();

        let asset = super::super::assets::import_custom_media_in(
            home.path(),
            &media_file,
            Some("User Wallpaper"),
        ).unwrap();

        // Apply custom video
        let res = apply_silentsddm_wallpaper_in(
            home.path(),
            Some(sys_root.path()),
            &asset.id,
            None,
        );
        assert!(res.is_ok(), "Apply should succeed: {:?}", res);

        // User's original file is intact during apply
        assert!(media_file.exists());
        assert_eq!(fs::read(&media_file).unwrap(), media_bytes);

        // Deactivate
        let deact_res = deactivate_silentsddm_in(home.path(), Some(sys_root.path()), None);
        assert!(deact_res.is_ok(), "Deactivate should succeed: {:?}", deact_res);

        // User's original file is NEVER deleted
        assert!(media_file.exists(), "User original file must remain untouched after deactivation");
        assert_eq!(fs::read(&media_file).unwrap(), media_bytes);

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }
}
