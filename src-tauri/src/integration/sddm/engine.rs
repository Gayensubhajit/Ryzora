//! SilentSDDM Theme Engine — Phase S3
//!
//! Manages the shared SilentSDDM SDDM theme engine:
//! - Pinned archive verification (SHA-256: c3979c47... and 20,807,064 bytes)
//! - Safe decompression with path traversal, symlink, and hardlink rejection
//! - Strict pruning: backgrounds/ is excluded from the installed engine payload
//! - Ownership check: refuses to clobber foreign or unowned theme directories
//! - Transactional installation with full rollback via InstallRollbackResult
//! - Uninstall safety: refuses removal if SDDM is currently configured to use ryzora-silent

use super::discovery::{resolve_engine_path, silentsddm_data_dir, RYZORA_SILENT_SLUG};
use crate::sddm_helper::{install_sddm_theme, remove_sddm_theme, resolve_effective_sddm_theme_in};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Pinned Engine Constants
// ─────────────────────────────────────────────────────────────────────────────

pub const ENGINE_COMMIT: &str = "73380d331fe0f8b8a3b17991bb917a0d438a4d34";
pub const ENGINE_VERSION: &str = "1.5.0";
pub const ENGINE_ARCHIVE_SHA256: &str =
    "c3979c47be1eb951c528fca341bfa91d41ae4e2818bd147ef3180aa00dad4317";
pub const ENGINE_ARCHIVE_SIZE_BYTES: u64 = 20807064;
pub const ENGINE_SLUG: &str = "silent";
pub const ENGINE_ARCHIVE_URL: &str =
    "https://github.com/uiriansan/SilentSDDM/archive/73380d331fe0f8b8a3b17991bb917a0d438a4d34.tar.gz";

// Limits for archive extraction safety
pub const MAX_ARCHIVE_ENTRIES: usize = 2000;
pub const MAX_DECOMPRESSED_BYTES: usize = 150 * 1024 * 1024; // 150 MB

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SilentSddmEngineManifest {
    pub engine_id: String,
    pub version: String,
    pub source_commit: String,
    pub archive_sha256: String,
    pub installed: bool,
    pub engine_path: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InstallRollbackResult {
    pub restored_engine: bool,
    pub restored_manifest: bool,
    pub removed_staging: bool,
    pub errors: Vec<String>,
    pub fully_reverted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineOwnershipStatus {
    Absent,
    VerifiedRyzoraOwned,
    ForeignOrUnowned(String),
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn engine_manifest_path(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("engine_manifest.json")
}

pub fn load_engine_manifest(home: &Path) -> Option<SilentSddmEngineManifest> {
    let path = engine_manifest_path(home);
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_engine_manifest(home: &Path, manifest: &SilentSddmEngineManifest) -> Result<(), String> {
    let dir = silentsddm_data_dir(home);
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create silentsddm data dir: {}", e))?;
    let path = engine_manifest_path(home);
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| format!("Failed to serialize engine manifest: {}", e))?;
    let tmp = dir.join(".tmp_engine_manifest.json");
    fs::write(&tmp, json)
        .map_err(|e| format!("Failed to write temp engine manifest: {}", e))?;
    fs::rename(&tmp, &path)
        .map_err(|e| format!("Failed to atomically replace engine manifest: {}", e))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Ownership Verification
// ─────────────────────────────────────────────────────────────────────────────

/// Checks ownership of the destination `/usr/share/sddm/themes/ryzora-silent`.
/// Refuses installation if the directory exists without a valid Ryzora manifest.
pub fn check_engine_ownership_in(home: &Path, sys_root: Option<&Path>) -> EngineOwnershipStatus {
    let engine_dir = resolve_engine_path(sys_root);
    if !engine_dir.exists() {
        return EngineOwnershipStatus::Absent;
    }

    // Destination exists: check if Ryzora owns it via engine_manifest.json
    if let Some(manifest) = load_engine_manifest(home) {
        if manifest.installed
            && (manifest.engine_path == engine_dir.to_string_lossy()
                || manifest.engine_id == "silentsddm")
        {
            return EngineOwnershipStatus::VerifiedRyzoraOwned;
        }
    }

    EngineOwnershipStatus::ForeignOrUnowned(format!(
        "Theme directory '{}' exists but is not registered to Ryzora",
        engine_dir.display()
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Archive Download & Extraction (with backgrounds/ pruning)
// ─────────────────────────────────────────────────────────────────────────────

/// Downloads the pinned GitHub archive and verifies SHA-256 and size.
pub fn download_engine_archive(dest_file: &Path) -> Result<(), String> {
    if let Some(parent) = dest_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create download dir: {}", e))?;
    }

    let tmp_dest = dest_file.with_extension("download_tmp");
    let resp = ureq::get(ENGINE_ARCHIVE_URL)
        .call()
        .map_err(|e| format!("Failed to download SilentSDDM archive from {}: {}", ENGINE_ARCHIVE_URL, e))?;

    let mut reader = resp.into_reader();
    let mut file = File::create(&tmp_dest)
        .map_err(|e| format!("Failed to create archive file: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32768];
    let mut total_bytes: u64 = 0;

    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|e| format!("Error streaming archive download: {}", e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])
            .map_err(|e| format!("Error writing archive file: {}", e))?;
        hasher.update(&buffer[..n]);
        total_bytes += n as u64;

        if total_bytes > ENGINE_ARCHIVE_SIZE_BYTES * 2 {
            let _ = fs::remove_file(&tmp_dest);
            return Err("Archive download exceeded maximum allowed byte limit".to_string());
        }
    }

    file.flush()
        .map_err(|e| format!("Error flushing archive file: {}", e))?;
    drop(file);

    let calculated_sha256 = format!("{:x}", hasher.finalize());

    if calculated_sha256 != ENGINE_ARCHIVE_SHA256 {
        let _ = fs::remove_file(&tmp_dest);
        return Err(format!(
            "Archive integrity check failed: expected SHA-256 {}, got {}",
            ENGINE_ARCHIVE_SHA256, calculated_sha256
        ));
    }

    if total_bytes != ENGINE_ARCHIVE_SIZE_BYTES {
        let _ = fs::remove_file(&tmp_dest);
        return Err(format!(
            "Archive byte count mismatch: expected {} bytes, got {}",
            ENGINE_ARCHIVE_SIZE_BYTES, total_bytes
        ));
    }

    fs::rename(&tmp_dest, dest_file)
        .map_err(|e| format!("Failed to finalize archive file: {}", e))?;

    Ok(())
}

/// Safely extracts the engine archive into `staging_dir`.
/// Explicitly PRUNES / EXCLUDES `backgrounds/` so no wallpapers are bundled into the engine.
pub fn extract_and_prune_engine<R: Read>(reader: R, staging_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    let gz = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);

    let mut total_uncompressed: usize = 0;
    let mut entry_count: usize = 0;

    let entries = archive
        .entries()
        .map_err(|e| format!("Malformed tarball stream: {}", e))?;

    for entry_res in entries {
        entry_count += 1;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(format!(
                "Archive exceeds maximum allowed entry count ({})",
                MAX_ARCHIVE_ENTRIES
            ));
        }

        let mut entry = entry_res.map_err(|e| format!("Failed to read archive entry: {}", e))?;
        let entry_type = entry.header().entry_type();

        // 1. Unconditional rejection of symlinks and hardlinks
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(format!(
                "Archive contains rejected link entry: {:?}",
                entry.path()
            ));
        }

        // 2. Reject special device files, sockets, FIFOs
        if entry_type.is_character_special()
            || entry_type.is_block_special()
            || entry_type.is_fifo()
        {
            return Err(format!(
                "Archive contains rejected special device entry: {:?}",
                entry.path()
            ));
        }

        let raw_path = entry
            .path()
            .map_err(|e| format!("Invalid path in archive entry: {}", e))?
            .to_path_buf();

        // 3. Traversal protection
        if raw_path.is_absolute()
            || raw_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(format!(
                "Archive entry path contains forbidden traversal: {:?}",
                raw_path
            ));
        }

        // 4. Strip single top-level directory (e.g. `SilentSDDM-73380d.../`)
        let mut components = raw_path.components();
        let _root = match components.next() {
            Some(std::path::Component::Normal(r)) => r,
            _ => continue,
        };

        let stripped: PathBuf = components.collect();
        if stripped.as_os_str().is_empty() {
            continue; // Skip the root directory entry itself
        }

        // 5. CRITICAL PRUNING: Exclude `backgrounds/` directory entirely
        if stripped == Path::new("backgrounds") || stripped.starts_with("backgrounds") {
            continue; // Do NOT extract backgrounds into the engine payload!
        }

        let target_path = staging_dir.join(&stripped);

        // Canonical confinement check
        if !target_path.starts_with(staging_dir) {
            return Err(format!(
                "Path escapes staging directory: {:?}",
                target_path
            ));
        }

        let size = entry.header().size().unwrap_or(0) as usize;
        total_uncompressed += size;
        if total_uncompressed > MAX_DECOMPRESSED_BYTES {
            return Err(format!(
                "Uncompressed archive exceeds size limit ({} bytes)",
                MAX_DECOMPRESSED_BYTES
            ));
        }

        if entry_type.is_dir() {
            fs::create_dir_all(&target_path)
                .map_err(|e| format!("Failed to create directory {:?}: {}", target_path, e))?;
        } else if entry_type.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir for {:?}: {}", target_path, e))?;
            }
            entry
                .unpack(&target_path)
                .map_err(|e| format!("Failed to unpack file {:?}: {}", target_path, e))?;
        }
    }

    // Post-extraction validation: required engine files must exist
    validate_staged_engine(staging_dir)?;

    Ok(())
}

/// Validates that an engine staging directory contains all required infrastructure
/// and confirms that `backgrounds/` was strictly excluded.
pub fn validate_staged_engine(staging_dir: &Path) -> Result<(), String> {
    let required_files = [
        "Main.qml",
        "metadata.desktop",
        "qmldir",
    ];

    for f in required_files {
        let p = staging_dir.join(f);
        if !p.is_file() {
            return Err(format!(
                "Engine validation failed: missing essential file '{}'",
                f
            ));
        }
    }

    let required_dirs = [
        "components",
        "configs",
        "fonts",
        "icons",
    ];

    for d in required_dirs {
        let p = staging_dir.join(d);
        if !p.is_dir() {
            return Err(format!(
                "Engine validation failed: missing essential directory '{}'",
                d
            ));
        }
    }

    // Invariant: backgrounds/ MUST NOT exist in the staged engine
    let bg_dir = staging_dir.join("backgrounds");
    if bg_dir.exists() {
        return Err("Engine validation failed: backgrounds/ must not be bundled in the engine payload".to_string());
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Health Verification
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the installed engine is present, healthy, and does not bundle catalog wallpapers.
pub fn verify_engine_health(sys_root: Option<&Path>) -> Result<bool, String> {
    let dest = resolve_engine_path(sys_root);
    if !dest.exists() {
        return Ok(false);
    }

    if !dest.join("Main.qml").exists() {
        return Err("Installed engine corrupted: Main.qml missing".to_string());
    }
    if !dest.join("metadata.desktop").exists() {
        return Err("Installed engine corrupted: metadata.desktop missing".to_string());
    }
    if !dest.join("qmldir").exists() {
        return Err("Installed engine corrupted: qmldir missing".to_string());
    }
    if !dest.join("components").exists() {
        return Err("Installed engine corrupted: components/ missing".to_string());
    }

    // Ensure catalog wallpapers were not accidentally bundled
    let bundled_ken = dest.join("backgrounds/ken.mp4");
    if bundled_ken.exists() {
        return Err("Installed engine invalid: contains bundled catalog wallpaper".to_string());
    }

    Ok(true)
}

// ─────────────────────────────────────────────────────────────────────────────
// Transactional Engine Installation & Rollback
// ─────────────────────────────────────────────────────────────────────────────

/// Performs a fully transactional installation of the SilentSDDM engine.
/// If `archive_override` is provided, reads from that archive file instead of downloading.
pub fn install_engine_transactional_in(
    home: &Path,
    sys_root: Option<&Path>,
    archive_override: Option<&Path>,
) -> Result<SilentSddmEngineManifest, (String, InstallRollbackResult)> {
    let mut rollback = InstallRollbackResult::default();

    // 1. Ownership verification: refuse to clobber foreign / unowned directories
    match check_engine_ownership_in(home, sys_root) {
        EngineOwnershipStatus::ForeignOrUnowned(reason) => {
            rollback.fully_reverted = true;
            return Err((reason, rollback));
        }
        EngineOwnershipStatus::Absent | EngineOwnershipStatus::VerifiedRyzoraOwned => {}
    }

    let data_dir = silentsddm_data_dir(home);
    // Secure temporary staging directory matching ryzora-staging-* under /tmp
    let staging_temp = match tempfile::Builder::new()
        .prefix("ryzora-staging-")
        .tempdir_in("/tmp")
    {
        Ok(t) => t,
        Err(e) => {
            rollback.fully_reverted = true;
            return Err((format!("Failed to create secure staging directory in /tmp: {}", e), rollback));
        }
    };
    let staging_base = staging_temp.path().to_path_buf();
    rollback.removed_staging = false;

    // 2. Snapshot existing engine if present (for rollback on upgrade)
    let dest = resolve_engine_path(sys_root);
    let backup_dir = data_dir.join(".engine_backup");
    let _ = fs::remove_dir_all(&backup_dir);
    let previous_engine_existed = dest.exists();

    if previous_engine_existed {
        if let Err(e) = copy_dir_all(&dest, &backup_dir) {
            let _ = fs::remove_dir_all(&staging_base);
            rollback.fully_reverted = true;
            return Err((format!("Failed to snapshot existing engine: {}", e), rollback));
        }
    }

    // Snapshot existing manifest if present
    let previous_manifest = load_engine_manifest(home);

    // 3. Obtain and extract archive into staging
    let extract_res = match archive_override {
        Some(path) => {
            File::open(path)
                .map_err(|e| format!("Failed to open override archive: {}", e))
                .and_then(|f| extract_and_prune_engine(f, &staging_base))
        }
        None => {
            let archive_file = data_dir.join(".engine_archive.tar.gz");
            download_engine_archive(&archive_file)
                .and_then(|()| {
                    File::open(&archive_file)
                        .map_err(|e| format!("Failed to open downloaded archive: {}", e))
                        .and_then(|f| extract_and_prune_engine(f, &staging_base))
                })
                .map(|()| {
                    let _ = fs::remove_file(&archive_file);
                })
        }
    };

    if let Err(e) = extract_res {
        let _ = fs::remove_dir_all(&staging_base);
        let _ = fs::remove_dir_all(&backup_dir);
        rollback.removed_staging = true;
        rollback.fully_reverted = true;
        return Err((e, rollback));
    }

    // 4. Install system engine via privileged helper
    let install_res = install_sddm_theme(&staging_base, ENGINE_SLUG);

    if let Err(e) = install_res {
        rollback.errors.push(format!("Helper install failed: {}", e));
        execute_engine_rollback(
            home,
            sys_root,
            &backup_dir,
            previous_engine_existed,
            previous_manifest.as_ref(),
            &staging_base,
            &mut rollback,
        );
        return Err((format!("SDDM engine installation failed: {}", e), rollback));
    }

    // 5. Post-installation physical verification
    if let Err(e) = verify_engine_health(sys_root) {
        rollback.errors.push(format!("Post-install health check failed: {}", e));
        execute_engine_rollback(
            home,
            sys_root,
            &backup_dir,
            previous_engine_existed,
            previous_manifest.as_ref(),
            &staging_base,
            &mut rollback,
        );
        return Err((format!("Installed engine failed health verification: {}", e), rollback));
    }

    // 6. Write engine_manifest.json
    let now = chrono_now_iso();
    let manifest = SilentSddmEngineManifest {
        engine_id: "silentsddm".to_string(),
        version: ENGINE_VERSION.to_string(),
        source_commit: ENGINE_COMMIT.to_string(),
        archive_sha256: ENGINE_ARCHIVE_SHA256.to_string(),
        installed: true,
        engine_path: dest.to_string_lossy().to_string(),
        installed_at: now,
    };

    if let Err(e) = save_engine_manifest(home, &manifest) {
        rollback.errors.push(format!("Failed to save engine manifest: {}", e));
        execute_engine_rollback(
            home,
            sys_root,
            &backup_dir,
            previous_engine_existed,
            previous_manifest.as_ref(),
            &staging_base,
            &mut rollback,
        );
        return Err((format!("Failed to record engine manifest: {}", e), rollback));
    }

    // 7. Success: clean up temporary staging and backup
    let _ = fs::remove_dir_all(&staging_base);
    let _ = fs::remove_dir_all(&backup_dir);

    Ok(manifest)
}

fn execute_engine_rollback(
    home: &Path,
    sys_root: Option<&Path>,
    backup_dir: &Path,
    previous_engine_existed: bool,
    previous_manifest: Option<&SilentSddmEngineManifest>,
    staging_base: &Path,
    rollback: &mut InstallRollbackResult,
) {
    let _ = fs::remove_dir_all(staging_base);
    rollback.removed_staging = true;

    // Restore or remove engine
    if previous_engine_existed && backup_dir.exists() {
        let dest = resolve_engine_path(sys_root);
        let _ = fs::remove_dir_all(&dest);
        if let Err(e) = copy_dir_all(backup_dir, &dest) {
            rollback.errors.push(format!("Failed to restore engine from backup: {}", e));
            rollback.restored_engine = false;
        } else {
            rollback.restored_engine = true;
        }
    } else {
        let _ = remove_sddm_theme(ENGINE_SLUG);
        rollback.restored_engine = true;
    }

    // Restore manifest
    if let Some(pm) = previous_manifest {
        if let Err(e) = save_engine_manifest(home, pm) {
            rollback.errors.push(format!("Failed to restore previous manifest: {}", e));
            rollback.restored_manifest = false;
        } else {
            rollback.restored_manifest = true;
        }
    } else {
        let p = engine_manifest_path(home);
        let _ = fs::remove_file(p);
        rollback.restored_manifest = true;
    }

    let _ = fs::remove_dir_all(backup_dir);
    rollback.fully_reverted = rollback.restored_engine && rollback.restored_manifest;
}

// ─────────────────────────────────────────────────────────────────────────────
// Engine Uninstallation
// ─────────────────────────────────────────────────────────────────────────────

/// Uninstalls the SilentSDDM engine.
/// Checks that SDDM is not currently configured to use `ryzora-silent`.
/// Invariant: Leaves `custom/` and `blobs/` intact.
pub fn uninstall_engine_in(home: &Path, sys_root: Option<&Path>) -> Result<(), String> {
    // 1. Read-only safety check: refuse removal if SDDM is currently using ryzora-silent
    let resolution = resolve_effective_sddm_theme_in(sys_root);
    if let Some(current) = resolution.effective_theme {
        if current == RYZORA_SILENT_SLUG || current == "silent" {
            return Err(format!(
                "Cannot uninstall SilentSDDM engine while SDDM is actively using it (Current={}). Deactivate SDDM first.",
                current
            ));
        }
    }

    // 2. Remove theme directory via privileged helper
    remove_sddm_theme(ENGINE_SLUG)?;

    // 3. Remove engine manifest
    let manifest_file = engine_manifest_path(home);
    if manifest_file.exists() {
        let _ = fs::remove_file(manifest_file);
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility Functions
// ─────────────────────────────────────────────────────────────────────────────

fn chrono_now_iso() -> String {
    let now = std::time::SystemTime::now();
    let dt: ChronoMock = now.into();
    dt.to_iso()
}

struct ChronoMock {
    secs: u64,
}

impl From<std::time::SystemTime> for ChronoMock {
    fn from(t: std::time::SystemTime) -> Self {
        let secs = t
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self { secs }
    }
}

impl ChronoMock {
    fn to_iso(&self) -> String {
        format!("{}Z", self.secs)
    }
}

/// Recursively copies a directory tree, rejecting symlinks.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_symlink() {
            continue; // Ignore symlinks
        }
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ft.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ryzora-sddm-eng-test-{}-{}-{}",
                label,
                std::process::id(),
                nanos
            ));
            let _ = fs::create_dir_all(&path);
            Self { path }
        }
        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    /// Creates a mock SilentSDDM tarball archive containing valid engine files + backgrounds/
    fn create_mock_archive(archive_path: &Path, include_backgrounds: bool) {
        let tar_gz = File::create(archive_path).unwrap();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        let root = "SilentSDDM-73380d331fe0f8b8a3b17991bb917a0d438a4d34";

        let mut add_file = |rel: &str, content: &[u8]| {
            let full = format!("{}/{}", root, rel);
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, full, content).unwrap();
        };

        add_file("Main.qml", b"import QtQuick; Rectangle {}");
        add_file("metadata.desktop", b"[SddmGreeterTheme]\nVersion=1.5.0\nName=Silent\n");
        add_file("qmldir", b"module SilentSDDM\n");
        add_file("components/LoginScreen.qml", b"Item {}");
        add_file("configs/default.conf", b"[General]\n");
        add_file("fonts/OFL.txt", b"Open Font License");
        add_file("icons/enter.svg", b"<svg></svg>");

        if include_backgrounds {
            add_file("backgrounds/default.jpg", b"fake image");
            add_file("backgrounds/ken.mp4", b"fake video");
        }

        tar.finish().unwrap();
    }

    #[test]
    fn test_extract_and_prune_engine_excludes_backgrounds() {
        let tmp = TestDir::new("prune-bg");
        let archive = tmp.path().join("mock.tar.gz");
        create_mock_archive(&archive, true);

        let staging = tmp.path().join("staging");
        let file = File::open(&archive).unwrap();
        extract_and_prune_engine(file, &staging).unwrap();

        assert!(staging.join("Main.qml").exists());
        assert!(staging.join("metadata.desktop").exists());
        assert!(staging.join("components/LoginScreen.qml").exists());
        assert!(staging.join("configs/default.conf").exists());

        // Invariant: backgrounds/ MUST NOT exist
        assert!(!staging.join("backgrounds").exists(), "backgrounds/ must be strictly excluded from engine staging!");
    }

    #[test]
    fn test_extract_rejects_path_traversal() {
        let tmp = TestDir::new("traversal");
        let archive = tmp.path().join("evil.tar.gz");

        let tar_gz = File::create(&archive).unwrap();
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o644);
        let path_bytes = b"root/../../evil.txt";
        header.as_mut_bytes()[..path_bytes.len()].copy_from_slice(path_bytes);
        header.set_cksum();
        tar.append(&header, &b"evil"[..]).unwrap();
        tar.into_inner().unwrap().finish().unwrap();

        let staging = tmp.path().join("staging");
        let file = File::open(&archive).unwrap();
        let res = extract_and_prune_engine(file, &staging);
        assert!(res.is_err(), "Must reject path traversal");
        let err = res.unwrap_err();
        assert!(err.contains("traversal") || err.contains("Invalid path") || err.contains(".."));
    }

    #[test]
    fn test_ownership_check_absent_ok() {
        let home = TestDir::new("home");
        let sys_root = TestDir::new("sys");
        let status = check_engine_ownership_in(home.path(), Some(sys_root.path()));
        assert_eq!(status, EngineOwnershipStatus::Absent);
    }

    #[test]
    fn test_ownership_check_foreign_directory_refused() {
        let home = TestDir::new("home");
        let sys_root = TestDir::new("sys");

        // Foreign directory exists without Ryzora manifest
        let foreign_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(&foreign_dir).unwrap();
        fs::write(foreign_dir.join("Main.qml"), "alien").unwrap();

        let status = check_engine_ownership_in(home.path(), Some(sys_root.path()));
        match status {
            EngineOwnershipStatus::ForeignOrUnowned(msg) => {
                assert!(msg.contains("not registered to Ryzora"));
            }
            other => panic!("Expected ForeignOrUnowned, got {:?}", other),
        }
    }

    #[test]
    fn test_ownership_check_verified_owned() {
        let home = TestDir::new("home");
        let sys_root = TestDir::new("sys");

        let engine_dir = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        fs::create_dir_all(&engine_dir).unwrap();

        let manifest = SilentSddmEngineManifest {
            engine_id: "silentsddm".to_string(),
            version: "1.5.0".to_string(),
            source_commit: ENGINE_COMMIT.to_string(),
            archive_sha256: ENGINE_ARCHIVE_SHA256.to_string(),
            installed: true,
            engine_path: engine_dir.to_string_lossy().to_string(),
            installed_at: "2026-09-21T00:00:00Z".to_string(),
        };
        save_engine_manifest(home.path(), &manifest).unwrap();

        let status = check_engine_ownership_in(home.path(), Some(sys_root.path()));
        assert_eq!(status, EngineOwnershipStatus::VerifiedRyzoraOwned);
    }

    #[test]
    fn test_uninstall_refuses_when_sddm_active() {
        let home = TestDir::new("home");
        let sys_root = TestDir::new("sys");

        // Set effective theme drop-in to ryzora-silent
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]\nCurrent=ryzora-silent\n").unwrap();

        let res = uninstall_engine_in(home.path(), Some(sys_root.path()));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("actively using it"));
    }

    #[test]
    fn test_engine_staging_directory_under_local_share_rejected_and_tmp_staging_enforced() {
        // 1. Invariant: Any directory under ~/.local/share/... must fail the staging path validation
        let home = TestDir::new("fake-home");
        let local_share_staging = home.path().join(".local/share/ryzora/lockscreens/silentsddm/staging_engine");
        let path_str = local_share_staging.to_string_lossy();
        
        assert!(
            !helper_staging_path_valid(&path_str),
            "Staging directory under ~/.local/share/ must be rejected by helper staging requirements"
        );

        // 2. Invariant: tempfile::Builder with ryzora-staging- in /tmp creates a valid path that passes helper validation
        let valid_temp = tempfile::Builder::new()
            .prefix("ryzora-staging-")
            .tempdir_in("/tmp")
            .unwrap();
        let valid_str = valid_temp.path().to_string_lossy();
        assert!(
            helper_staging_path_valid(&valid_str),
            "Temp staging directory in /tmp must strictly pass helper staging requirements"
        );
        assert!(valid_str.starts_with("/tmp/ryzora-staging-"));
    }

    #[test]
    fn test_real_helper_install_engine_and_uninstall_lifecycle() {
        use crate::TEST_ENV_MUTEX as ENV_MUTEX;
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let home = TestDir::new("engine-test-home");
        let sys_root = TestDir::new("engine-test-sys");
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());

        // Setup real helper in sys_root
        let helper_dir = sys_root.path().join("usr/lib/ryzora");
        fs::create_dir_all(&helper_dir).unwrap();
        let helper_path = helper_dir.join("ryzora-sddm-helper");
        let real_helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/ryzora-sddm-helper");
        fs::copy(&real_helper, &helper_path).unwrap();
        let mut perms = fs::metadata(&helper_path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&helper_path, perms).unwrap();

        // Create mock engine archive
        let archive_path = home.path().join("engine.tar.gz");
        create_mock_archive(&archive_path, true);

        // 1. Install engine through full install_engine_in pipeline
        let manifest = install_engine_transactional_in(home.path(), Some(sys_root.path()), Some(&archive_path))
            .expect("install_engine_in must succeed with real helper and secure /tmp staging");
        assert_eq!(manifest.engine_id, "silentsddm");
        assert!(manifest.installed);

        let installed_theme = sys_root.path().join("usr/share/sddm/themes/ryzora-silent");
        assert!(installed_theme.exists(), "Installed theme must exist in system directory");
        assert!(installed_theme.join("Main.qml").exists());
        assert!(!installed_theme.join("backgrounds").exists(), "Backgrounds must be pruned!");

        // Manifest must be recorded
        let loaded = load_engine_manifest(home.path()).expect("Manifest must be loaded");
        assert_eq!(loaded.version, ENGINE_VERSION);

        // Ownership status verified owned
        assert_eq!(
            check_engine_ownership_in(home.path(), Some(sys_root.path())),
            EngineOwnershipStatus::VerifiedRyzoraOwned
        );

        // 2. Uninstall engine
        uninstall_engine_in(home.path(), Some(sys_root.path())).unwrap();
        assert!(!installed_theme.exists(), "Installed theme must be removed after uninstall");
        assert!(load_engine_manifest(home.path()).is_none(), "Manifest must be removed after uninstall");
        assert_eq!(
            check_engine_ownership_in(home.path(), Some(sys_root.path())),
            EngineOwnershipStatus::Absent
        );

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    fn helper_staging_path_valid(dir: &str) -> bool {
        // Mirrors validate_staging_dir in ryzora-sddm-helper:
        // ^/tmp/ryzora-staging-[a-zA-Z0-9_-]+(/[a-zA-Z0-9_./-]+)?$
        if !dir.starts_with("/tmp/ryzora-staging-") {
            return false;
        }
        let rest = &dir["/tmp/ryzora-staging-".len()..];
        if rest.is_empty() {
            return false;
        }
        let mut parts = rest.split('/');
        let first_seg = parts.next().unwrap();
        if first_seg.is_empty() || !first_seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            return false;
        }
        for sub in parts {
            if sub.is_empty() || !sub.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
                return false;
            }
        }
        true
    }
}
