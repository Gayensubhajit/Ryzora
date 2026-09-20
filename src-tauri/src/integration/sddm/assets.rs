//! SilentSDDM Assets & Custom Media Management — Phase S3
//!
//! Manages:
//! - Content-Addressable Storage (CAS) for wallpaper blobs at `blobs/{sha256}`
//! - Upstream wallpaper lazy download, SHA-256 integrity verification, and regular file placement
//! - CAS deduplication (e.g. `default.jpg` and `smoky.jpg` share the same SHA-256)
//! - User-facing wallpaper files are regular copies, NOT symlinks
//! - Custom video ingestion: symlink rejection, extension check, 500MB cap, basename sanitization,
//!   atomic copy, SHA-256 calculation, and direct `ffmpeg` poster extraction (no shell)
//! - Ref-counted CAS cleanup on wallpaper uninstallation
//! - Custom video removal and manifest synchronisation
//!
//! Invariant: S3 does NOT touch /etc/sddm.conf.d/, does NOT touch SDDM `Current=`, and does NOT activate themes.

use super::discovery::{
    silentsddm_data_dir, AssetType, CachedAsset, MediaType, REJECTED_EXTS, SUPPORTED_IMAGE_EXTS,
    SUPPORTED_VIDEO_EXTS,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Maximum size for custom media imports: 500 MB.
pub const MAX_CUSTOM_FILE_BYTES: u64 = 500 * 1024 * 1024;

/// Request to install a catalog wallpaper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamWallpaperInstallRequest {
    pub id: String,
    pub filename: String,
    pub media_type: MediaType,
    pub sha256: String,
    pub size_bytes: u64,
    pub download_url: String,
    pub poster_url: Option<String>,
    pub poster_sha256: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Directory Layout Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_blobs_dir(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("blobs")
}

pub fn get_wallpapers_dir(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("wallpapers")
}

pub fn get_custom_dir(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("custom")
}

pub fn get_posters_dir(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("posters")
}

pub fn get_wallpapers_manifest_path(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("wallpapers_manifest.json")
}

pub fn get_custom_manifest_path(home: &Path) -> PathBuf {
    silentsddm_data_dir(home).join("custom_manifest.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest Storage
// ─────────────────────────────────────────────────────────────────────────────

pub fn load_manifest_map(path: &Path) -> HashMap<String, CachedAsset> {
    if !path.exists() {
        return HashMap::new();
    }
    match fs::read_to_string(path) {
        Ok(c) => serde_json::from_str(&c).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub fn save_manifest_map(path: &Path, map: &HashMap<String, CachedAsset>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create manifest parent directory: {}", e))?;
    }
    let tmp_path = path.with_extension("tmp");
    let content = serde_json::to_string_pretty(map)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&tmp_path, content)
        .map_err(|e| format!("Failed to write manifest temp file: {}", e))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Failed to finalize manifest: {}", e))?;
    Ok(())
}

fn iso_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let secs = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}Z", secs)
}

// ─────────────────────────────────────────────────────────────────────────────
// Content-Addressable Storage (CAS)
// ─────────────────────────────────────────────────────────────────────────────

/// Computes SHA-256 hex string of any reader.
pub fn compute_sha256<R: Read>(mut reader: R) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 32768];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Computes SHA-256 hex string of a file on disk.
pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(path)?;
    compute_sha256(file)
}

/// Ensures a blob with `expected_sha256` exists in `blobs_dir`.
/// If already present and valid, returns immediately (CAS deduplication hit).
/// If absent, downloads from `url`, verifies streaming SHA-256 and expected size,
/// and stores atomically in `blobs/{sha256}`.
pub fn ensure_blob_downloaded(
    blobs_dir: &Path,
    url: &str,
    expected_sha256: &str,
    expected_size: Option<u64>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(blobs_dir)
        .map_err(|e| format!("Failed to create blobs directory: {}", e))?;

    let blob_path = blobs_dir.join(expected_sha256);

    // CAS deduplication check: if blob exists and size matches, verify hash
    if blob_path.is_file() {
        if let Ok(meta) = fs::metadata(&blob_path) {
            if expected_size.map(|s| s == meta.len()).unwrap_or(true) {
                // Verify hash of existing blob
                if let Ok(hash) = hash_file(&blob_path) {
                    if hash == expected_sha256 {
                        return Ok(blob_path);
                    }
                }
            }
        }
        // If existing blob was corrupted, remove it
        let _ = fs::remove_file(&blob_path);
    }

    // Download to a unique temporary file
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = blobs_dir.join(format!(".tmp_blob_{}_{}", expected_sha256, nanos));

    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("Failed to download asset from {}: {}", url, e))?;

    let mut reader = resp.into_reader();
    let mut file = File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temporary blob file: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 32768];
    let mut total_bytes: u64 = 0;
    let max_bytes = expected_size.map(|s| s.max(1024 * 1024) * 2).unwrap_or(500 * 1024 * 1024);

    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|e| format!("Error streaming asset download: {}", e))?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])
            .map_err(|e| format!("Error writing asset blob: {}", e))?;
        hasher.update(&buffer[..n]);
        total_bytes += n as u64;

        if total_bytes > max_bytes {
            let _ = fs::remove_file(&tmp_path);
            return Err("Asset download exceeded maximum expected size".to_string());
        }
    }

    file.flush()
        .map_err(|e| format!("Error flushing blob file: {}", e))?;
    drop(file);

    let calculated_sha256 = format!("{:x}", hasher.finalize());
    if calculated_sha256 != expected_sha256 {
        let _ = fs::remove_file(&tmp_path);
        return Err(format!(
            "Asset integrity check failed: expected SHA-256 {}, got {}",
            expected_sha256, calculated_sha256
        ));
    }

    if let Some(expected_len) = expected_size {
        if total_bytes != expected_len {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Asset size check failed: expected {} bytes, got {}",
                expected_len, total_bytes
            ));
        }
    }

    fs::rename(&tmp_path, &blob_path)
        .map_err(|e| format!("Failed to finalize blob file: {}", e))?;

    Ok(blob_path)
}

/// Directly copies a pre-verified data slice into a blob (used in tests or offline seeding).
pub fn store_blob_bytes(
    blobs_dir: &Path,
    data: &[u8],
    expected_sha256: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(blobs_dir)
        .map_err(|e| format!("Failed to create blobs directory: {}", e))?;

    let mut hasher = Sha256::new();
    hasher.update(data);
    let calculated = format!("{:x}", hasher.finalize());

    if calculated != expected_sha256 {
        return Err(format!(
            "Hash mismatch: expected {}, calculated {}",
            expected_sha256, calculated
        ));
    }

    let blob_path = blobs_dir.join(expected_sha256);
    let tmp_path = blobs_dir.join(format!(".tmp_blob_{}", calculated));
    fs::write(&tmp_path, data)
        .map_err(|e| format!("Failed to write blob data: {}", e))?;
    fs::rename(&tmp_path, &blob_path)
        .map_err(|e| format!("Failed to rename blob: {}", e))?;

    Ok(blob_path)
}

// ─────────────────────────────────────────────────────────────────────────────
// Upstream Wallpaper Installation
// ─────────────────────────────────────────────────────────────────────────────

/// Installs an upstream wallpaper:
/// 1. Ensures the main media blob is in CAS (`blobs/{sha256}`)
/// 2. If video poster URL is given, ensures poster blob is in CAS
/// 3. Copies the regular file from `blobs/{sha256}` to `wallpapers/{filename}` (**NO SYMLINKS**)
/// 4. Copies poster to `posters/{filename}.jpg` if video
/// 5. Records entry in `wallpapers_manifest.json`
pub fn install_upstream_wallpaper_in(
    home: &Path,
    req: &UpstreamWallpaperInstallRequest,
) -> Result<CachedAsset, String> {
    let blobs_dir = get_blobs_dir(home);
    let wallpapers_dir = get_wallpapers_dir(home);
    let posters_dir = get_posters_dir(home);
    let manifest_path = get_wallpapers_manifest_path(home);

    fs::create_dir_all(&wallpapers_dir)
        .map_err(|e| format!("Failed to create wallpapers directory: {}", e))?;
    fs::create_dir_all(&posters_dir)
        .map_err(|e| format!("Failed to create posters directory: {}", e))?;

    // 1. Fetch / verify main blob via CAS
    let blob_path = ensure_blob_downloaded(
        &blobs_dir,
        &req.download_url,
        &req.sha256,
        Some(req.size_bytes),
    )?;

    // 2. User-facing file placement: must be regular file copy, NEVER a symlink
    let dest_wallpaper = wallpapers_dir.join(&req.filename);
    if dest_wallpaper.exists() {
        let _ = fs::remove_file(&dest_wallpaper);
    }
    fs::copy(&blob_path, &dest_wallpaper)
        .map_err(|e| format!("Failed to copy wallpaper file to destination: {}", e))?;

    // Invariant verification: target must be a regular file, not a symlink
    let meta = fs::symlink_metadata(&dest_wallpaper)
        .map_err(|e| format!("Failed to verify installed wallpaper file: {}", e))?;
    if meta.is_symlink() {
        let _ = fs::remove_file(&dest_wallpaper);
        return Err("Installed wallpaper must not be a symlink".to_string());
    }

    // 3. Poster download if present
    if let (Some(poster_url), Some(poster_hash)) = (&req.poster_url, &req.poster_sha256) {
        if let Ok(poster_blob) = ensure_blob_downloaded(&blobs_dir, poster_url, poster_hash, None) {
            let poster_filename = format!("{}.jpg", req.filename);
            let dest_poster = posters_dir.join(&poster_filename);
            let _ = fs::copy(&poster_blob, &dest_poster);
        }
    }

    // 4. Update manifest
    let mut manifest = load_manifest_map(&manifest_path);
    let cached = CachedAsset {
        id: req.id.clone(),
        filename: req.filename.clone(),
        asset_type: AssetType::Upstream,
        media_type: req.media_type.clone(),
        sha256: req.sha256.clone(),
        size_bytes: req.size_bytes,
        installed_at: iso_timestamp(),
        upstream_url: Some(req.download_url.clone()),
        original_path: None,
    };
    manifest.insert(req.id.clone(), cached.clone());
    save_manifest_map(&manifest_path, &manifest)?;

    Ok(cached)
}

/// Uninstalls an upstream wallpaper:
/// 1. Removes user-facing file `wallpapers/{filename}`
/// 2. Removes poster if present
/// 3. Updates `wallpapers_manifest.json`
/// 4. Garbage collects unreferenced blob from `blobs/{sha256}`
pub fn uninstall_upstream_wallpaper_in(home: &Path, wallpaper_id: &str) -> Result<(), String> {
    let wallpapers_dir = get_wallpapers_dir(home);
    let posters_dir = get_posters_dir(home);
    let blobs_dir = get_blobs_dir(home);
    let manifest_path = get_wallpapers_manifest_path(home);

    let mut manifest = load_manifest_map(&manifest_path);
    let asset = manifest
        .remove(wallpaper_id)
        .ok_or_else(|| format!("Wallpaper '{}' is not installed", wallpaper_id))?;

    // 1. Remove user-facing file
    let wallpaper_path = wallpapers_dir.join(&asset.filename);
    if wallpaper_path.exists() {
        let _ = fs::remove_file(&wallpaper_path);
    }

    // 2. Remove poster
    let poster_path = posters_dir.join(format!("{}.jpg", asset.filename));
    if poster_path.exists() {
        let _ = fs::remove_file(&poster_path);
    }

    // 3. Save updated manifest
    save_manifest_map(&manifest_path, &manifest)?;

    // 4. CAS Garbage Collection: if no other installed wallpaper shares this blob, delete it
    let blob_referenced = manifest.values().any(|a| a.sha256 == asset.sha256);
    if !blob_referenced {
        let blob_path = blobs_dir.join(&asset.sha256);
        if blob_path.exists() {
            let _ = fs::remove_file(&blob_path);
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Custom Video / Image Import
// ─────────────────────────────────────────────────────────────────────────────

/// Sanitizes a user-supplied filename:
/// - Strips directory components (basename only)
/// - Replaces non-alphanumeric (except `_`, `-`, `.`) with `_`
/// - Prevents hidden files, `..`, empty basenames
pub fn sanitize_custom_basename(raw_name: &str) -> String {
    let path = Path::new(raw_name);
    let base = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("custom_video");

    let mut sanitized = String::new();
    for c in base.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            sanitized.push(c);
        } else {
            sanitized.push('_');
        }
    }

    // Remove leading dots or dashes
    let trimmed = sanitized.trim_start_matches(['.', '-', '_']);
    if trimmed.is_empty() || trimmed == ".." {
        return "custom_video.mp4".to_string();
    }

    trimmed.to_string()
}

/// Generates a video poster image using direct `ffmpeg` process invocation (no shell).
pub fn generate_video_poster(video_path: &Path, poster_path: &Path) -> Result<(), String> {
    if let Some(parent) = poster_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Try first at 1 second offset
    let status_res = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss",
            "00:00:01",
            "-i",
            video_path.to_str().unwrap_or(""),
            "-vframes",
            "1",
            "-q:v",
            "2",
            poster_path.to_str().unwrap_or(""),
        ])
        .output();

    let output = match status_res {
        Ok(out) => out,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("ffmpeg is required for custom video poster generation but was not found on PATH".to_string());
        }
        Err(e) => {
            return Err(format!("Failed to execute ffmpeg: {}", e));
        }
    };

    if output.status.success() && poster_path.exists() && poster_path.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Ok(());
    }

    // If seeking to 1s failed (e.g. short video), retry at offset 00:00:00
    let retry = Command::new("ffmpeg")
        .args([
            "-y",
            "-ss",
            "00:00:00",
            "-i",
            video_path.to_str().unwrap_or(""),
            "-vframes",
            "1",
            "-q:v",
            "2",
            poster_path.to_str().unwrap_or(""),
        ])
        .output();

    match retry {
        Ok(out) if out.status.success() && poster_path.exists() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Err(format!("ffmpeg poster extraction failed: {}", stderr.lines().last().unwrap_or("unknown error")))
        }
        Err(e) => Err(format!("ffmpeg retry execution failed: {}", e)),
    }
}

/// Imports a user-supplied custom video or image into Ryzora's managed storage:
/// - Rejects symlinks (read-only verification of source metadata)
/// - Rejects files > 500 MB
/// - Validates extension (rejects .gif and unsupported types)
/// - Sanitizes basename
/// - Copies atomically to `custom/{sanitized_filename}` (**regular file**)
/// - Generates poster via direct `ffmpeg` invocation for video
/// - Records in `custom_manifest.json`
pub fn import_custom_video_in(home: &Path, source_path: &Path) -> Result<CachedAsset, String> {
    // 1. Validate file existence and metadata
    let meta = fs::symlink_metadata(source_path)
        .map_err(|e| format!("Cannot read source file {:?}: {}", source_path, e))?;

    // Invariant: Reject symlinks unconditionally
    if meta.is_symlink() {
        return Err("Custom media source must be a regular file, symlinks are rejected".to_string());
    }

    if !meta.is_file() {
        return Err("Custom media source must be a regular file".to_string());
    }

    let file_size = meta.len();
    if file_size == 0 {
        return Err("Custom media file is empty (0 bytes)".to_string());
    }
    if file_size > MAX_CUSTOM_FILE_BYTES {
        return Err(format!(
            "File exceeds maximum allowed size of {} MB",
            MAX_CUSTOM_FILE_BYTES / (1024 * 1024)
        ));
    }

    // 2. Validate extension
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    if REJECTED_EXTS.contains(&ext.as_str()) {
        return Err(format!(
            ".{} is not supported: animated GIFs are known to crash SDDM",
            ext
        ));
    }

    let media_type = if SUPPORTED_VIDEO_EXTS.contains(&ext.as_str()) {
        MediaType::Video
    } else if SUPPORTED_IMAGE_EXTS.contains(&ext.as_str()) {
        MediaType::Image
    } else {
        return Err(format!(
            "Unsupported media extension '.{}'. Supported: video ({}), image ({})",
            ext,
            SUPPORTED_VIDEO_EXTS.join(", "),
            SUPPORTED_IMAGE_EXTS.join(", ")
        ));
    };

    // 3. Compute SHA-256 of source file
    let sha256 = hash_file(source_path)
        .map_err(|e| format!("Failed to compute file SHA-256: {}", e))?;

    // 4. Sanitize basename
    let raw_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("custom_video");
    let sanitized_name = sanitize_custom_basename(raw_name);

    let custom_dir = get_custom_dir(home);
    let posters_dir = get_posters_dir(home);
    fs::create_dir_all(&custom_dir)
        .map_err(|e| format!("Failed to create custom media dir: {}", e))?;
    fs::create_dir_all(&posters_dir)
        .map_err(|e| format!("Failed to create posters dir: {}", e))?;

    // Determine target filename (handle collision if different content)
    let dest_filename = {
        let candidate = custom_dir.join(&sanitized_name);
        if candidate.exists() {
            if let Ok(existing_hash) = hash_file(&candidate) {
                if existing_hash == sha256 {
                    sanitized_name.clone()
                } else {
                    let short_sha = &sha256[..8];
                    format!("{}_{}", short_sha, sanitized_name)
                }
            } else {
                sanitized_name.clone()
            }
        } else {
            sanitized_name.clone()
        }
    };

    let dest_path = custom_dir.join(&dest_filename);
    let tmp_dest = custom_dir.join(format!(".tmp_{}", dest_filename));

    // 5. Atomic copy (regular file, not symlink)
    fs::copy(source_path, &tmp_dest)
        .map_err(|e| format!("Failed to copy custom media file: {}", e))?;
    fs::rename(&tmp_dest, &dest_path)
        .map_err(|e| format!("Failed to finalize custom media file: {}", e))?;

    // 6. Poster generation
    let poster_path = posters_dir.join(format!("{}.poster.jpg", dest_filename));
    match media_type {
        MediaType::Video => {
            generate_video_poster(&dest_path, &poster_path)?;
        }
        MediaType::Image => {
            let _ = fs::copy(&dest_path, &poster_path);
        }
    }

    // 7. Update custom manifest
    let manifest_path = get_custom_manifest_path(home);
    let mut manifest = load_manifest_map(&manifest_path);
    let asset_id = format!("custom:{}", dest_filename);

    let cached = CachedAsset {
        id: asset_id.clone(),
        filename: dest_filename,
        asset_type: AssetType::Custom,
        media_type,
        sha256,
        size_bytes: file_size,
        installed_at: iso_timestamp(),
        upstream_url: None,
        original_path: Some(source_path.to_string_lossy().to_string()),
    };

    manifest.insert(asset_id, cached.clone());
    save_manifest_map(&manifest_path, &manifest)?;

    Ok(cached)
}

/// Removes an imported custom video or image.
pub fn remove_custom_video_in(home: &Path, custom_id: &str) -> Result<(), String> {
    let custom_dir = get_custom_dir(home);
    let posters_dir = get_posters_dir(home);
    let manifest_path = get_custom_manifest_path(home);

    let mut manifest = load_manifest_map(&manifest_path);
    let asset = manifest
        .remove(custom_id)
        .ok_or_else(|| format!("Custom media '{}' not found", custom_id))?;

    let media_path = custom_dir.join(&asset.filename);
    if media_path.exists() {
        let _ = fs::remove_file(&media_path);
    }

    let poster_path = posters_dir.join(format!("{}.poster.jpg", asset.filename));
    if poster_path.exists() {
        let _ = fs::remove_file(&poster_path);
    }

    save_manifest_map(&manifest_path, &manifest)?;
    Ok(())
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
            let path = std::env::temp_dir().join(format!("ryzora_test_assets_{}_{}", label, nanos));
            fs::create_dir_all(&path).unwrap();
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

    #[test]
    fn test_cas_deduplication_same_content() {
        let home = TestDir::new("cas_dedup");
        let blobs_dir = get_blobs_dir(home.path());

        // default.jpg and smoky.jpg share SHA256
        let sample_data = b"SHARED_WALLPAPER_CONTENT_27DFE124";
        let mut hasher = Sha256::new();
        hasher.update(sample_data);
        let sample_sha = format!("{:x}", hasher.finalize());

        // 1. Store first blob
        let blob_path1 = store_blob_bytes(&blobs_dir, sample_data, &sample_sha).unwrap();
        assert!(blob_path1.exists());

        // 2. Install upstream wallpaper 1 (e.g. default.jpg)
        let req1 = UpstreamWallpaperInstallRequest {
            id: "silentsddm-default".to_string(),
            filename: "default.jpg".to_string(),
            media_type: MediaType::Image,
            sha256: sample_sha.clone(),
            size_bytes: sample_data.len() as u64,
            download_url: "mock://default.jpg".to_string(),
            poster_url: None,
            poster_sha256: None,
        };
        let asset1 = install_upstream_wallpaper_in(home.path(), &req1).unwrap();
        assert_eq!(asset1.filename, "default.jpg");

        // 3. Install upstream wallpaper 2 (e.g. smoky.jpg) reusing the same SHA-256
        let req2 = UpstreamWallpaperInstallRequest {
            id: "silentsddm-smoky".to_string(),
            filename: "smoky.jpg".to_string(),
            media_type: MediaType::Image,
            sha256: sample_sha.clone(),
            size_bytes: sample_data.len() as u64,
            download_url: "mock://smoky.jpg".to_string(),
            poster_url: None,
            poster_sha256: None,
        };
        let asset2 = install_upstream_wallpaper_in(home.path(), &req2).unwrap();
        assert_eq!(asset2.filename, "smoky.jpg");

        // 4. Verify both exist as regular files in wallpapers/
        let wp1 = get_wallpapers_dir(home.path()).join("default.jpg");
        let wp2 = get_wallpapers_dir(home.path()).join("smoky.jpg");
        assert!(wp1.exists());
        assert!(wp2.exists());
        assert!(!wp1.is_symlink(), "default.jpg must be a regular file");
        assert!(!wp2.is_symlink(), "smoky.jpg must be a regular file");

        // 5. Verify uninstallation ref-counts CAS: removing default.jpg does NOT delete blob
        uninstall_upstream_wallpaper_in(home.path(), "silentsddm-default").unwrap();
        assert!(!wp1.exists());
        assert!(wp2.exists());
        assert!(blobs_dir.join(&sample_sha).exists(), "Blob must survive while smoky is installed");

        // 6. Removing smoky.jpg cleans up blob
        uninstall_upstream_wallpaper_in(home.path(), "silentsddm-smoky").unwrap();
        assert!(!wp2.exists());
        assert!(!blobs_dir.join(&sample_sha).exists(), "Blob must be cleaned up when no refs remain");
    }

    #[test]
    fn test_custom_import_rejects_symlinks() {
        let home = TestDir::new("custom_symlink");
        let source_file = home.path().join("real_video.mp4");
        fs::write(&source_file, b"fake video bytes").unwrap();

        let symlink_source = home.path().join("symlink_video.mp4");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source_file, &symlink_source).unwrap();

        let res = import_custom_video_in(home.path(), &symlink_source);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("symlinks are rejected"));
    }

    #[test]
    fn test_custom_import_rejects_gif() {
        let home = TestDir::new("custom_gif");
        let gif_file = home.path().join("crash.gif");
        fs::write(&gif_file, b"GIF89a...").unwrap();

        let res = import_custom_video_in(home.path(), &gif_file);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("crash SDDM"));
    }

    #[test]
    fn test_custom_basename_sanitization() {
        assert_eq!(sanitize_custom_basename("my wallpaper!.mp4"), "my_wallpaper_.mp4");
        assert_eq!(sanitize_custom_basename("../../../etc/passwd.mp4"), "passwd.mp4");
        assert_eq!(sanitize_custom_basename(".."), "custom_video");
        assert_eq!(sanitize_custom_basename("good-video_123.webm"), "good-video_123.webm");
    }

    #[test]
    fn test_custom_image_import_and_remove() {
        let home = TestDir::new("custom_image");
        let img_file = home.path().join("my_photo.png");
        fs::write(&img_file, b"\x89PNG\r\n\x1a\nfakeimage").unwrap();

        let asset = import_custom_video_in(home.path(), &img_file).unwrap();
        assert_eq!(asset.media_type, MediaType::Image);
        assert_eq!(asset.filename, "my_photo.png");

        let custom_file = get_custom_dir(home.path()).join("my_photo.png");
        assert!(custom_file.exists());
        assert!(!custom_file.is_symlink());

        // Remove
        remove_custom_video_in(home.path(), &asset.id).unwrap();
        assert!(!custom_file.exists());
    }
}
