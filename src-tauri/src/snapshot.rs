use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileEntryType {
    Regular,
    Directory,
    Symlink,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapshotStatus {
    Pending,
    Complete,
    Verified,
    Corrupted,
    Restored,
}

/// Metadata for one backed-up filesystem entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// The original input path as given (e.g. "~/.config/hypr/hyprland.conf").
    pub original_path: String,
    /// Resolved absolute path on the real filesystem.
    pub absolute_original: String,
    /// Path of the backed-up copy relative to the snapshot's files/ dir.
    pub backup_relative: String,
    pub file_type: FileEntryType,
    /// For symlinks: the recorded link target (not followed).
    pub symlink_target: Option<String>,
    pub size_bytes: u64,
    /// SHA-256 hex digest for regular files.
    pub sha256: Option<String>,
    /// Whether the path existed at snapshot time.
    pub existed: bool,
}

/// The full snapshot record written to snapshot.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub id: String,
    pub label: String,
    pub created_at: u64,
    pub formatted_date: String,
    pub ryzora_version: String,
    pub status: SnapshotStatus,
    pub entries: Vec<FileEntry>,
    pub total_size_bytes: u64,
    pub verified: bool,
}

/// Result returned to the frontend after a restore operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub restored_count: usize,
    pub skipped_count: usize,
    pub removed_absent_count: usize,
    pub errors: Vec<String>,
    pub success: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Directory Resolution
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

pub fn get_ryzora_snapshots_dir() -> PathBuf {
    let xdg_data = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| get_home_dir().join(".local").join("share"));
    xdg_data.join("ryzora").join("snapshots")
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Lexically strip `.` and `..` components without any syscall.
fn normalize_path_lexical(path: &Path) -> PathBuf {
    let mut parts: Vec<Component> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.iter().collect()
}

/// Validate an input path string and expand it relative to `home_dir`.
///
/// Accepts:
///   - `~/...`   → `home_dir/...`
///   - bare relative (no leading `/`) → `home_dir/<raw>`
///
/// Rejects:
///   - absolute paths (`/etc/...`)
///   - any `..` segment
///   - null bytes
///   - empty strings
///   - paths that resolve outside `home_dir` after normalization
pub fn validate_and_expand_path(raw: &str, home_dir: &Path) -> Result<PathBuf, String> {
    if raw.contains('\0') {
        return Err(format!("Path contains null byte: '{}'", raw));
    }
    if raw.trim().is_empty() {
        return Err("Empty path rejected".to_string());
    }
    if raw.starts_with('/') {
        return Err(format!(
            "Absolute path '{}' rejected — Ryzora only accepts ~/... home-relative paths",
            raw
        ));
    }

    let relative_part: &str = if raw.starts_with("~/") {
        &raw[2..]
    } else if raw == "~" {
        ""
    } else {
        raw
    };

    // Reject .. in any segment of the relative part
    for segment in relative_part.split('/') {
        if segment == ".." {
            return Err(format!(
                "Path '{}' contains '..' traversal — rejected for safety",
                raw
            ));
        }
    }

    let expanded = if relative_part.is_empty() {
        home_dir.to_path_buf()
    } else {
        home_dir.join(relative_part)
    };

    // Lexical safety check after joining
    let normalized = normalize_path_lexical(&expanded);
    if !normalized.starts_with(home_dir) {
        return Err(format!(
            "Path '{}' resolves outside home directory — rejected",
            raw
        ));
    }

    Ok(expanded)
}

/// Validate a snapshot ID — must not contain path separators or traversal.
fn validate_snapshot_id(id: &str) -> Result<(), String> {
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('\0') || id.trim().is_empty() {
        return Err(format!("Invalid snapshot ID '{}'", id));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// SHA-256
// ─────────────────────────────────────────────────────────────────────────────

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Cannot open {:?}: {}", path, e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("Read error on {:?}: {}", path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Timestamp Utilities (no chrono)
// ─────────────────────────────────────────────────────────────────────────────

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn format_ts_id(secs: u64) -> String {
    let rem = secs % 86400;
    let (y, mo, d) = days_to_ymd((secs / 86400) as i64);
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        y,
        mo,
        d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn format_ts_human(secs: u64) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let rem = secs % 86400;
    let (y, mo, d) = days_to_ymd((secs / 86400) as i64);
    let month_name = if mo >= 1 && mo <= 12 {
        MONTHS[(mo - 1) as usize]
    } else {
        "???"
    };
    format!(
        "{} {} {} {:02}:{:02} UTC",
        d,
        month_name,
        y,
        rem / 3600,
        (rem % 3600) / 60
    )
}

fn make_snapshot_id(label: &str, timestamp: u64) -> String {
    let sanitized: String = label
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .take(32)
        .collect();
    let sanitized = sanitized.trim_matches('-').to_string();
    let sanitized = if sanitized.is_empty() { "snapshot".to_string() } else { sanitized };
    format!("{}-{}", format_ts_id(timestamp), sanitized)
}

// ─────────────────────────────────────────────────────────────────────────────
// Backup — Individual Entry
// ─────────────────────────────────────────────────────────────────────────────

/// Strip home_dir prefix from an absolute path to get the relative path.
fn relative_from_home(abs: &Path, home_dir: &Path) -> Result<PathBuf, String> {
    abs.strip_prefix(home_dir)
        .map(|p| p.to_path_buf())
        .map_err(|_| format!("{:?} is not under home dir {:?}", abs, home_dir))
}

/// Back up a single regular file into the snapshot files dir.
fn backup_regular_file(
    original_input: &str,
    absolute: &Path,
    rel_from_home: &Path,
    snap_files_dir: &Path,
) -> Result<Vec<FileEntry>, String> {
    let dest = snap_files_dir.join(rel_from_home);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Cannot create backup dir {:?}: {}", parent, e))?;
    }
    fs::copy(absolute, &dest)
        .map_err(|e| format!("Cannot copy {:?} → {:?}: {}", absolute, dest, e))?;

    let size = dest.metadata().map(|m| m.len()).unwrap_or(0);
    let hash = sha256_file(absolute)?;

    Ok(vec![FileEntry {
        original_path: original_input.to_string(),
        absolute_original: absolute.to_string_lossy().to_string(),
        backup_relative: rel_from_home.to_string_lossy().to_string(),
        file_type: FileEntryType::Regular,
        symlink_target: None,
        size_bytes: size,
        sha256: Some(hash),
        existed: true,
    }])
}

/// Back up a directory recursively. Returns FileEntry for each item.
fn backup_directory(
    original_input: &str,
    absolute: &Path,
    rel_from_home: &Path,
    home_dir: &Path,
    snap_files_dir: &Path,
) -> Result<Vec<FileEntry>, String> {
    let dest_dir = snap_files_dir.join(rel_from_home);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Cannot create backup dir {:?}: {}", dest_dir, e))?;

    let mut entries = vec![FileEntry {
        original_path: original_input.to_string(),
        absolute_original: absolute.to_string_lossy().to_string(),
        backup_relative: rel_from_home.to_string_lossy().to_string(),
        file_type: FileEntryType::Directory,
        symlink_target: None,
        size_bytes: 0,
        sha256: None,
        existed: true,
    }];

    let read_dir = fs::read_dir(absolute)
        .map_err(|e| format!("Cannot read dir {:?}: {}", absolute, e))?;

    for item in read_dir {
        let item = item.map_err(|e| e.to_string())?;
        let child_abs = item.path();
        let child_rel = child_abs
            .strip_prefix(home_dir)
            .map_err(|_| "child not under home_dir".to_string())?
            .to_path_buf();
        let child_input = format!("~/{}", child_rel.to_string_lossy());

        // Use symlink_metadata to NOT follow symlinks
        let child_meta = child_abs
            .symlink_metadata()
            .map_err(|e| format!("Cannot stat {:?}: {}", child_abs, e))?;
        let ft = child_meta.file_type();

        if ft.is_symlink() {
            let target = fs::read_link(&child_abs)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            entries.push(FileEntry {
                original_path: child_input,
                absolute_original: child_abs.to_string_lossy().to_string(),
                backup_relative: child_rel.to_string_lossy().to_string(),
                file_type: FileEntryType::Symlink,
                symlink_target: Some(target),
                size_bytes: 0,
                sha256: None,
                existed: true,
            });
        } else if ft.is_dir() {
            let sub = backup_directory(
                &child_input,
                &child_abs,
                &child_rel,
                home_dir,
                snap_files_dir,
            )?;
            entries.extend(sub);
        } else {
            let sub = backup_regular_file(&child_input, &child_abs, &child_rel, snap_files_dir)?;
            entries.extend(sub);
        }
    }

    Ok(entries)
}

/// Back up one path (file, directory, symlink, or absent).
fn backup_path(
    input: &str,
    home_dir: &Path,
    snap_files_dir: &Path,
) -> Result<Vec<FileEntry>, String> {
    let absolute = validate_and_expand_path(input, home_dir)?;
    let rel_from_home = relative_from_home(&absolute, home_dir)?;
    let backup_relative = rel_from_home.to_string_lossy().to_string();

    // Use symlink_metadata so we never silently follow a symlink
    match absolute.symlink_metadata() {
        Err(_) => {
            // Path does not exist — record as Absent
            Ok(vec![FileEntry {
                original_path: input.to_string(),
                absolute_original: absolute.to_string_lossy().to_string(),
                backup_relative,
                file_type: FileEntryType::Absent,
                symlink_target: None,
                size_bytes: 0,
                sha256: None,
                existed: false,
            }])
        }
        Ok(meta) => {
            let ft = meta.file_type();
            if ft.is_symlink() {
                let target = fs::read_link(&absolute)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                Ok(vec![FileEntry {
                    original_path: input.to_string(),
                    absolute_original: absolute.to_string_lossy().to_string(),
                    backup_relative,
                    file_type: FileEntryType::Symlink,
                    symlink_target: Some(target),
                    size_bytes: 0,
                    sha256: None,
                    existed: true,
                }])
            } else if ft.is_dir() {
                backup_directory(input, &absolute, &rel_from_home, home_dir, snap_files_dir)
            } else {
                backup_regular_file(input, &absolute, &rel_from_home, snap_files_dir)
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Core Snapshot Operations (testable — accept explicit paths)
// ─────────────────────────────────────────────────────────────────────────────

pub fn create_snapshot_in(
    label: &str,
    paths: &[String],
    home_dir: &Path,
    snapshots_root: &Path,
) -> Result<SnapshotMetadata, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let id = make_snapshot_id(label, now);
    let snap_dir = snapshots_root.join(&id);
    let snap_files_dir = snap_dir.join("files");

    fs::create_dir_all(&snap_files_dir)
        .map_err(|e| format!("Cannot create snapshot dir: {}", e))?;

    let mut all_entries: Vec<FileEntry> = Vec::new();

    for raw_path in paths {
        match backup_path(raw_path, home_dir, &snap_files_dir) {
            Ok(entries) => all_entries.extend(entries),
            Err(e) => {
                let _ = fs::remove_dir_all(&snap_dir);
                return Err(format!("Failed to back up '{}': {}", raw_path, e));
            }
        }
    }

    let total_size: u64 = all_entries.iter().map(|e| e.size_bytes).sum();

    let mut meta = SnapshotMetadata {
        id: id.clone(),
        label: label.to_string(),
        created_at: now,
        formatted_date: format_ts_human(now),
        ryzora_version: env!("CARGO_PKG_VERSION").to_string(),
        status: SnapshotStatus::Complete,
        entries: all_entries,
        total_size_bytes: total_size,
        verified: false,
    };

    // Write snapshot.json
    let meta_path = snap_dir.join("snapshot.json");
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Serialization error: {}", e))?;
    fs::write(&meta_path, &meta_json)
        .map_err(|e| format!("Cannot write snapshot.json: {}", e))?;

    // Verify immediately after creation
    let verified = verify_snapshot_in(&id, snapshots_root).unwrap_or(false);
    meta.verified = verified;
    meta.status = if verified {
        SnapshotStatus::Verified
    } else {
        SnapshotStatus::Corrupted
    };

    // Re-write with updated status
    let meta_json = serde_json::to_string_pretty(&meta)
        .map_err(|e| format!("Serialization error: {}", e))?;
    fs::write(&meta_path, &meta_json)
        .map_err(|e| format!("Cannot update snapshot.json: {}", e))?;

    Ok(meta)
}

pub fn verify_snapshot_in(id: &str, snapshots_root: &Path) -> Result<bool, String> {
    validate_snapshot_id(id)?;
    let snap_dir = snapshots_root.join(id);
    if !snap_dir.exists() {
        return Err(format!("Snapshot '{}' not found", id));
    }

    let meta_path = snap_dir.join("snapshot.json");
    let json = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Cannot read snapshot.json: {}", e))?;
    let meta: SnapshotMetadata =
        serde_json::from_str(&json).map_err(|e| format!("Cannot parse snapshot.json: {}", e))?;

    let snap_files_dir = snap_dir.join("files");

    for entry in &meta.entries {
        match entry.file_type {
            FileEntryType::Regular => {
                let backed_up = snap_files_dir.join(&entry.backup_relative);
                if !backed_up.is_file() {
                    return Ok(false);
                }
                if let Some(expected) = &entry.sha256 {
                    match sha256_file(&backed_up) {
                        Ok(actual) if &actual == expected => {}
                        _ => return Ok(false),
                    }
                }
            }
            FileEntryType::Directory => {
                let backed_up = snap_files_dir.join(&entry.backup_relative);
                if !backed_up.is_dir() {
                    return Ok(false);
                }
            }
            FileEntryType::Symlink | FileEntryType::Absent => {}
        }
    }

    Ok(true)
}

pub fn list_snapshots_in(snapshots_root: &Path) -> Result<Vec<SnapshotMetadata>, String> {
    if !snapshots_root.exists() {
        return Ok(Vec::new());
    }

    let read_dir = fs::read_dir(snapshots_root)
        .map_err(|e| format!("Cannot read snapshots dir: {}", e))?;

    let mut list = Vec::new();
    for item in read_dir {
        let item = item.map_err(|e| e.to_string())?;
        let snap_dir = item.path();
        if !snap_dir.is_dir() {
            continue;
        }
        let meta_path = snap_dir.join("snapshot.json");
        if !meta_path.exists() {
            continue;
        }
        if let Ok(json) = fs::read_to_string(&meta_path) {
            if let Ok(meta) = serde_json::from_str::<SnapshotMetadata>(&json) {
                list.push(meta);
            }
        }
    }

    list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(list)
}

pub fn get_snapshot_in(id: &str, snapshots_root: &Path) -> Result<SnapshotMetadata, String> {
    validate_snapshot_id(id)?;
    let meta_path = snapshots_root.join(id).join("snapshot.json");
    if !meta_path.exists() {
        return Err(format!("Snapshot '{}' not found", id));
    }
    let json = fs::read_to_string(&meta_path)
        .map_err(|e| format!("Cannot read snapshot.json: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Cannot parse snapshot.json: {}", e))
}

pub fn delete_snapshot_in(id: &str, snapshots_root: &Path) -> Result<(), String> {
    validate_snapshot_id(id)?;
    let snap_dir = snapshots_root.join(id);

    // Safety: ensure snap_dir is strictly inside snapshots_root
    let norm_snap = normalize_path_lexical(&snap_dir);
    let norm_root = normalize_path_lexical(snapshots_root);
    if !norm_snap.starts_with(&norm_root) || norm_snap == norm_root {
        return Err("Snapshot directory is not inside Ryzora snapshots root".to_string());
    }

    if !snap_dir.exists() {
        return Err(format!("Snapshot '{}' not found", id));
    }

    fs::remove_dir_all(&snap_dir)
        .map_err(|e| format!("Cannot delete snapshot '{}': {}", id, e))?;
    Ok(())
}

pub fn restore_snapshot_in(
    id: &str,
    home_dir: &Path,
    snapshots_root: &Path,
) -> Result<RestoreResult, String> {
    validate_snapshot_id(id)?;
    let snap_dir = snapshots_root.join(id);
    let snap_files_dir = snap_dir.join("files");

    // 1. Load metadata
    let meta = get_snapshot_in(id, snapshots_root)?;

    // 2. Verify integrity before touching anything
    let is_valid = verify_snapshot_in(id, snapshots_root)?;
    if !is_valid {
        return Err(format!(
            "Snapshot '{}' failed integrity verification — restore aborted",
            id
        ));
    }

    let mut restored_count: usize = 0;
    let mut skipped_count: usize = 0;
    let mut removed_absent_count: usize = 0;
    let mut errors: Vec<String> = Vec::new();
    // (staging path, final destination)
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();

    // 3. Stage phase — copy each backup to a temp file beside the destination
    for entry in &meta.entries {
        match entry.file_type {
            FileEntryType::Regular => {
                let backup_file = snap_files_dir.join(&entry.backup_relative);
                let dest = home_dir.join(&entry.backup_relative);

                // Validate destination is inside home
                let norm_dest = normalize_path_lexical(&dest);
                if !norm_dest.starts_with(home_dir) {
                    errors.push(format!(
                        "Restore target {:?} is outside home directory — skipped",
                        dest
                    ));
                    skipped_count += 1;
                    continue;
                }

                if let Some(parent) = dest.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        errors.push(format!("Cannot create dir {:?}: {}", parent, e));
                        skipped_count += 1;
                        continue;
                    }
                }

                // Stage to a sibling temp file (atomic rename works within same fs)
                let staging = dest.with_extension("ryzora-restoring");
                match fs::copy(&backup_file, &staging) {
                    Ok(_) => staged.push((staging, dest)),
                    Err(e) => {
                        errors.push(format!("Stage failed for {:?}: {}", dest, e));
                        skipped_count += 1;
                    }
                }
            }
            FileEntryType::Directory => {
                let dest_dir = home_dir.join(&entry.backup_relative);
                if let Err(e) = fs::create_dir_all(&dest_dir) {
                    errors.push(format!("Cannot create directory {:?}: {}", dest_dir, e));
                }
            }
            FileEntryType::Symlink => {
                // Symlinks are recorded but not restored in this phase for safety
                skipped_count += 1;
            }
            FileEntryType::Absent => {
                // This path was absent before the package was installed.
                // If it now exists, remove it to restore the pre-install state.
                let dest = home_dir.join(&entry.backup_relative);
                let dest_exists = dest.symlink_metadata().is_ok();
                if dest_exists {
                    match fs::remove_file(&dest) {
                        Ok(_) => removed_absent_count += 1,
                        Err(e) => errors.push(format!(
                            "Cannot remove {:?} (was absent before install): {}",
                            dest, e
                        )),
                    }
                }
            }
        }
    }

    // 4. Swap phase — atomic rename from staging to final destination
    for (staging, final_dest) in &staged {
        match fs::rename(staging, final_dest) {
            Ok(_) => restored_count += 1,
            Err(e) => {
                errors.push(format!(
                    "Swap failed {:?} → {:?}: {}",
                    staging, final_dest, e
                ));
                // Try to clean up staging file
                let _ = fs::remove_file(staging);
                skipped_count += 1;
            }
        }
    }

    // 5. Mark snapshot as Restored
    if let Ok(mut updated_meta) = get_snapshot_in(id, snapshots_root) {
        updated_meta.status = SnapshotStatus::Restored;
        if let Ok(json) = serde_json::to_string_pretty(&updated_meta) {
            let _ = fs::write(snap_dir.join("snapshot.json"), json);
        }
    }

    let success = errors.is_empty();
    Ok(RestoreResult {
        restored_count,
        skipped_count,
        removed_absent_count,
        errors,
        success,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_snapshot(label: String, paths: Vec<String>) -> Result<SnapshotMetadata, String> {
    let home = get_home_dir();
    let root = get_ryzora_snapshots_dir();
    fs::create_dir_all(&root)
        .map_err(|e| format!("Cannot create snapshots directory: {}", e))?;
    create_snapshot_in(&label, &paths, &home, &root)
}

#[tauri::command]
pub fn list_snapshots() -> Result<Vec<SnapshotMetadata>, String> {
    list_snapshots_in(&get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn get_snapshot(id: String) -> Result<SnapshotMetadata, String> {
    get_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn verify_snapshot(id: String) -> Result<bool, String> {
    verify_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn delete_snapshot(id: String) -> Result<(), String> {
    delete_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn restore_snapshot(id: String) -> Result<RestoreResult, String> {
    let home = get_home_dir();
    let root = get_ryzora_snapshots_dir();
    restore_snapshot_in(&id, &home, &root)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    // ── Test Environment ────────────────────────────────────────────────────

    struct TestEnv {
        root: PathBuf,
        pub home: PathBuf,
        pub snapshots: PathBuf,
    }

    impl TestEnv {
        fn new(tag: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            let root = std::env::temp_dir()
                .join(format!("ryzora-test-{}-{}", tag, nanos));
            let home = root.join("home");
            let snapshots = root.join("snapshots");
            fs::create_dir_all(&home).expect("create home");
            fs::create_dir_all(&snapshots).expect("create snapshots");
            TestEnv { root, home, snapshots }
        }

        /// Write a file relative to home dir, creating parent dirs.
        fn write(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.home.join(rel);
            if let Some(par) = p.parent() {
                fs::create_dir_all(par).unwrap();
            }
            fs::write(&p, content).unwrap();
            p
        }

        /// Return a `~/...` input string for a home-relative path.
        fn inp(&self, rel: &str) -> String {
            format!("~/{}", rel)
        }

        fn create_snap(&self, label: &str, paths: &[&str]) -> SnapshotMetadata {
            let owned: Vec<String> = paths.iter().map(|p| self.inp(p)).collect();
            create_snapshot_in(label, &owned, &self.home, &self.snapshots).unwrap()
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ── Path Validation ─────────────────────────────────────────────────────

    #[test]
    fn test_reject_absolute_path() {
        let env = TestEnv::new("abs");
        let result = validate_and_expand_path("/etc/shadow", &env.home);
        assert!(result.is_err(), "Absolute path must be rejected");
        assert!(result.unwrap_err().contains("Absolute path"));
    }

    #[test]
    fn test_reject_path_traversal() {
        let env = TestEnv::new("trav");
        let result = validate_and_expand_path("~/.config/../../etc/passwd", &env.home);
        assert!(result.is_err(), "Path traversal must be rejected");
        assert!(result.unwrap_err().contains(".."));
    }

    #[test]
    fn test_reject_null_byte() {
        let env = TestEnv::new("null");
        let result = validate_and_expand_path("~/.config/foo\0bar", &env.home);
        assert!(result.is_err(), "Null byte must be rejected");
    }

    #[test]
    fn test_valid_tilde_path() {
        let env = TestEnv::new("valid");
        let result = validate_and_expand_path("~/.config/hypr/hyprland.conf", &env.home);
        assert!(result.is_ok(), "Valid ~/... path must be accepted");
        let expanded = result.unwrap();
        assert!(expanded.starts_with(&env.home));
    }

    // ── SHA-256 ──────────────────────────────────────────────────────────────

    #[test]
    fn test_sha256_checksum_known_value() {
        let env = TestEnv::new("sha256");
        // SHA-256 of empty string is e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        env.write(".config/empty.txt", "");
        let p = env.home.join(".config/empty.txt");
        let hash = sha256_file(&p).unwrap();
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    // ── Backup a Regular File ────────────────────────────────────────────────

    #[test]
    fn test_backup_regular_file() {
        let env = TestEnv::new("regfile");
        env.write(".config/hypr/hyprland.conf", "monitor=,1920x1080@60,0x0,1");

        let snap = env.create_snap("test-regular", &[".config/hypr/hyprland.conf"]);

        assert_eq!(snap.entries.len(), 1);
        let entry = &snap.entries[0];
        assert_eq!(entry.file_type, FileEntryType::Regular);
        assert!(entry.existed);
        assert!(entry.sha256.is_some());
        assert!(entry.size_bytes > 0);

        // Backed-up file must exist in snapshot
        let backed_up = env.snapshots.join(&snap.id).join("files").join(&entry.backup_relative);
        assert!(backed_up.exists(), "Backed-up file must exist: {:?}", backed_up);

        // Content must match
        let orig = fs::read_to_string(&env.home.join(".config/hypr/hyprland.conf")).unwrap();
        let bak = fs::read_to_string(&backed_up).unwrap();
        assert_eq!(orig, bak);
    }

    // ── Backup a Directory ───────────────────────────────────────────────────

    #[test]
    fn test_backup_directory() {
        let env = TestEnv::new("dir");
        env.write(".config/waybar/config.jsonc", r#"{"modules-left":[]}"#);
        env.write(".config/waybar/style.css", "* { margin: 0; }");

        let snap = env.create_snap("test-dir", &[".config/waybar"]);

        let dir_entry = snap.entries.iter().find(|e| e.file_type == FileEntryType::Directory);
        assert!(dir_entry.is_some(), "Must have a Directory entry");

        let file_entries: Vec<_> = snap.entries.iter()
            .filter(|e| e.file_type == FileEntryType::Regular)
            .collect();
        assert_eq!(file_entries.len(), 2, "Must have 2 regular file entries");
    }

    // ── Backup a Missing Path ────────────────────────────────────────────────

    #[test]
    fn test_backup_absent_path() {
        let env = TestEnv::new("absent");
        // Do not create the file
        let snap = env.create_snap("test-absent", &[".config/nonexistent/file.conf"]);

        assert_eq!(snap.entries.len(), 1);
        let entry = &snap.entries[0];
        assert_eq!(entry.file_type, FileEntryType::Absent);
        assert!(!entry.existed);
        assert!(entry.sha256.is_none());
    }

    // ── Backup Nested Paths ──────────────────────────────────────────────────

    #[test]
    fn test_backup_nested_paths() {
        let env = TestEnv::new("nested");
        env.write(".config/hypr/hyprland.conf", "monitor=,auto");
        env.write(".config/kitty/kitty.conf", "font_size 13.0");
        env.write(".config/starship.toml", "[character]\n");

        let snap = env.create_snap("test-nested", &[
            ".config/hypr/hyprland.conf",
            ".config/kitty/kitty.conf",
            ".config/starship.toml",
        ]);

        let regular: Vec<_> = snap.entries.iter()
            .filter(|e| e.file_type == FileEntryType::Regular)
            .collect();
        assert_eq!(regular.len(), 3);
    }

    // ── Snapshot Verification ────────────────────────────────────────────────

    #[test]
    fn test_snapshot_verify_passes() {
        let env = TestEnv::new("verify-ok");
        env.write(".config/hypr/hyprland.conf", "monitor=eDP-1,1920x1080,0x0,1");
        let snap = env.create_snap("verify-test", &[".config/hypr/hyprland.conf"]);

        assert!(snap.verified, "Snapshot should be verified immediately after creation");

        let result = verify_snapshot_in(&snap.id, &env.snapshots).unwrap();
        assert!(result, "Re-verification should pass");
    }

    #[test]
    fn test_snapshot_verify_fails_on_corruption() {
        let env = TestEnv::new("verify-fail");
        env.write(".config/hypr/hyprland.conf", "original content");
        let snap = env.create_snap("corrupt-test", &[".config/hypr/hyprland.conf"]);

        // Tamper with the backed-up file
        let backed_up = env.snapshots
            .join(&snap.id)
            .join("files")
            .join(&snap.entries[0].backup_relative);
        fs::write(&backed_up, "TAMPERED CONTENT").unwrap();

        let result = verify_snapshot_in(&snap.id, &env.snapshots).unwrap();
        assert!(!result, "Corrupted snapshot must fail verification");
    }

    // ── Snapshot Discovery ───────────────────────────────────────────────────

    #[test]
    fn test_snapshot_discovery_after_restart() {
        let env = TestEnv::new("discover");
        env.write(".config/hypr/hyprland.conf", "monitor=eDP-1");
        env.write(".config/kitty/kitty.conf", "font_size 12.0");

        let s1 = env.create_snap("first", &[".config/hypr/hyprland.conf"]);
        let s2 = env.create_snap("second", &[".config/kitty/kitty.conf"]);

        // Simulate app restart — list from disk only
        let found = list_snapshots_in(&env.snapshots).unwrap();
        assert_eq!(found.len(), 2, "Both snapshots must be discoverable from disk");

        let ids: Vec<&str> = found.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&s1.id.as_str()));
        assert!(ids.contains(&s2.id.as_str()));

        // Newest first
        assert_eq!(found[0].created_at >= found[1].created_at, true);
    }

    // ── Snapshot Deletion ────────────────────────────────────────────────────

    #[test]
    fn test_snapshot_deletion() {
        let env = TestEnv::new("delete");
        env.write(".config/hypr/hyprland.conf", "monitor=eDP-1");
        let snap = env.create_snap("delete-me", &[".config/hypr/hyprland.conf"]);

        let snap_dir = env.snapshots.join(&snap.id);
        assert!(snap_dir.exists(), "Snapshot directory must exist before deletion");

        delete_snapshot_in(&snap.id, &env.snapshots).unwrap();
        assert!(!snap_dir.exists(), "Snapshot directory must be gone after deletion");

        // List must return empty
        let list = list_snapshots_in(&env.snapshots).unwrap();
        assert!(list.is_empty(), "Snapshot list must be empty after deletion");
    }

    #[test]
    fn test_delete_rejects_traversal_id() {
        let env = TestEnv::new("del-trav");
        let result = delete_snapshot_in("../evil", &env.snapshots);
        assert!(result.is_err(), "Traversal ID must be rejected");
    }

    // ── Restore ──────────────────────────────────────────────────────────────

    #[test]
    fn test_restore_regular_file() {
        let env = TestEnv::new("restore-file");
        env.write(".config/hypr/hyprland.conf", "original content");
        let snap = env.create_snap("restore-test", &[".config/hypr/hyprland.conf"]);

        // Simulate modification after package install
        fs::write(env.home.join(".config/hypr/hyprland.conf"), "modified by package").unwrap();

        let result = restore_snapshot_in(&snap.id, &env.home, &env.snapshots).unwrap();
        assert!(result.success, "Restore must succeed; errors: {:?}", result.errors);
        assert_eq!(result.restored_count, 1);

        let restored = fs::read_to_string(env.home.join(".config/hypr/hyprland.conf")).unwrap();
        assert_eq!(restored, "original content", "Restored content must match original");
    }

    #[test]
    fn test_restore_absent_file_removes_it() {
        let env = TestEnv::new("restore-absent");
        // Originally absent — record snapshot
        let snap = env.create_snap("absent-restore", &[".config/newfile.conf"]);

        // Simulate: package installed a file that didn't exist before
        env.write(".config/newfile.conf", "installed by package");

        // Restore must remove the file (it was absent before)
        let result = restore_snapshot_in(&snap.id, &env.home, &env.snapshots).unwrap();
        assert!(result.success, "Restore must succeed; errors: {:?}", result.errors);
        assert_eq!(result.removed_absent_count, 1);

        assert!(
            !env.home.join(".config/newfile.conf").exists(),
            "File that was absent before must be removed on restore"
        );
    }

    // ── Symlink Handling ─────────────────────────────────────────────────────

    #[test]
    fn test_symlink_recorded_not_followed() {
        let env = TestEnv::new("symlink");
        // Create a real file and a symlink pointing to it
        env.write(".config/real.conf", "real file content");
        let link_path = env.home.join(".config/link.conf");
        symlink(env.home.join(".config/real.conf"), &link_path).unwrap();

        let snap = env.create_snap("symlink-test", &[".config/link.conf"]);

        assert_eq!(snap.entries.len(), 1);
        let entry = &snap.entries[0];
        assert_eq!(entry.file_type, FileEntryType::Symlink, "Symlink must be recorded as Symlink");
        assert!(entry.symlink_target.is_some(), "Symlink target must be recorded");
        // No content was copied
        assert_eq!(entry.sha256, None);
    }
}
