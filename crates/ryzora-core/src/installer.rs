//! Ryzora Core Package Installation & Uninstallation Engine
//!
//! Authoritative, UI-independent transactional package installer, materializer,
//! and uninstaller with snapshot-backed rollback and fail-closed state safety.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
use crate::snapshot::{create_snapshot_in, restore_snapshot_in, verify_snapshot_in};

// ─────────────────────────────────────────────────────────────────────────────
// Installer Data Types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationPlan {
    pub package_id: String,
    pub package_name: String,
    pub package_version: String,
    pub files_to_create: Vec<String>,
    pub files_to_replace: Vec<String>,
    pub files_unchanged: Vec<String>,
    pub directories_to_create: Vec<String>,
    pub conflicts: Vec<String>,
    pub compatibility_status: String,
    pub required_dependencies: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub selected_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub package_id: String,
    pub version: String,
    pub snapshot_id: String,
    pub installed_files: Vec<String>,
    pub errors: Vec<String>,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledFileEntry {
    pub target: String,
    pub sha256: String,
    #[serde(default)]
    pub is_symlink: bool,
    #[serde(default)]
    pub symlink_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledHistoryEntry {
    pub version: String,
    pub installed_at: u64,
    pub snapshot_id: String,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default)]
    pub tree_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackageRecord {
    pub package_id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub package_type: Option<PackageType>,
    #[serde(default)]
    pub repository_id: Option<String>,
    pub installed_at: u64,
    pub snapshot_id: String,
    pub installed_files: Vec<String>,
    #[serde(default)]
    pub files: Vec<InstalledFileEntry>,
    pub package_source_path: String,
    #[serde(default)]
    pub history: Vec<InstalledHistoryEntry>,
}

impl Default for InstalledPackageRecord {
    fn default() -> Self {
        Self {
            package_id: String::new(),
            name: String::new(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 0,
            snapshot_id: String::new(),
            installed_files: Vec::new(),
            files: Vec::new(),
            package_source_path: String::new(),
            history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UninstallResult {
    pub success: bool,
    pub package_id: String,
    pub snapshot_id: String,
    pub removed_files: Vec<String>,
    pub errors: Vec<String>,
    pub rolled_back: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ActiveLockscreenState {
    #[serde(default)]
    pub sddm_login: Option<String>,
    #[serde(default)]
    pub sddm_lock: Option<String>,
    pub quickshell: Option<String>,
    pub sddm: Option<String>,
    pub session_lock: Option<String>,
    pub hyprlock: Option<String>,
    pub quickshell_theme_path: Option<String>,
    pub sddm_theme_path: Option<String>,
    pub last_applied_at: Option<u64>,
    pub hypridle_override: bool,
    pub hypridle_source_hash: Option<String>,
    pub hypridle_source_path: Option<String>,
    pub hypridle_ryzora_config: Option<String>,
    pub lock_wrapper_path: Option<String>,
    pub sddm_previous_theme: Option<String>,
    #[serde(default)]
    pub active_config: Option<std::collections::HashMap<String, serde_json::Value>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Safety & Validation Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn compute_sha256_file(path: &Path) -> Result<String, String> {
    let content = fs::read(path)
        .map_err(|e| format!("Failed to read '{}' for SHA-256: {}", path.display(), e))?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn validate_package_source_file(package_dir: &Path, source: &str) -> Result<PathBuf, String> {
    if source.contains('\0') {
        return Err("Source path contains null byte".to_string());
    }
    let p = Path::new(source);
    if p.is_absolute() {
        return Err(format!("Source path '{}' must not be absolute", source));
    }
    for comp in p.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(format!(
                "Source path '{}' must not contain '..' components",
                source
            ));
        }
    }
    let resolved = package_dir.join(p);
    if !resolved.exists() {
        return Err(format!(
            "Source file '{}' does not exist in package directory",
            source
        ));
    }
    Ok(resolved)
}

pub fn validate_target_safety(target_str: &str, home_dir: &Path) -> Result<PathBuf, String> {
    if target_str.contains('\0') {
        return Err("Target path contains null byte".to_string());
    }
    let p = Path::new(target_str);
    for comp in p.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(format!(
                "Target path '{}' must not contain '..' components",
                target_str
            ));
        }
    }

    if let Some(stripped) = target_str.strip_prefix("~/") {
        Ok(home_dir.join(stripped))
    } else if target_str.starts_with('/') {
        let pb = PathBuf::from(target_str);
        if pb.starts_with(home_dir) {
            Ok(pb)
        } else {
            // Allow system paths only when running inside mock system root
            if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
                Ok(pb)
            } else {
                Err(format!(
                    "Target path '{}' must resolve inside home directory",
                    target_str
                ))
            }
        }
    } else {
        Ok(home_dir.join(target_str))
    }
}

pub fn cleanup_empty_parents(start_dir: &Path, stop_at: &Path) {
    let mut current = start_dir.to_path_buf();
    while current != stop_at && current.starts_with(stop_at) {
        if let Ok(mut entries) = fs::read_dir(&current) {
            if entries.next().is_none() {
                let parent = current.parent().map(|p| p.to_path_buf());
                let _ = fs::remove_dir(&current);
                if let Some(p) = parent {
                    current = p;
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Transactional Package Installation
// ─────────────────────────────────────────────────────────────────────────────

pub fn install_package_in(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    target: Option<&str>,
    create_snapshot: bool,
) -> Result<InstallResult, String> {
    // 1. Load and validate manifest
    let manifest_path = package_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(format!(
            "manifest.json not found in {}",
            package_dir.display()
        ));
    }
    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    let val = validate_manifest_internal(&manifest_raw);
    if !val.valid {
        return Err(format!(
            "Manifest validation failed: {}",
            val.errors.join("; ")
        ));
    }

    let target_files = manifest.target_files(target)?;
    if target_files.is_empty() {
        return Err(format!(
            "Package '{}' contains no files for target {:?}",
            manifest.id, target
        ));
    }

    // 2. Pre-flight path validation
    for f in &target_files {
        validate_package_source_file(package_dir, &f.source)?;
        validate_target_safety(&f.target, home_dir)?;
    }

    // 3. Optional pre-install snapshot
    let snapshot_id: String = if create_snapshot {
        fs::create_dir_all(snapshots_root)
            .map_err(|e| format!("Failed to create snapshots directory: {}", e))?;
        let target_paths: Vec<String> = target_files.iter().map(|f| f.target.clone()).collect();
        let label = format!("pre-install-{}", manifest.id);
        let meta = create_snapshot_in(&label, &target_paths, home_dir, snapshots_root)
            .map_err(|e| format!("Failed to create pre-install snapshot: {}", e))?;
        if !verify_snapshot_in(&meta.id, snapshots_root).unwrap_or(false) {
            return Err(
                "Pre-install snapshot failed integrity check. Aborting before modifications."
                    .to_string(),
            );
        }
        meta.id
    } else {
        String::new()
    };

    // 4. Create staging directory
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let staging_id = format!("{}-{}", manifest.id, ts);
    let staging_dir = staging_root.join(&staging_id);
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    let clean_staging = |dir: &Path| {
        let _ = fs::remove_dir_all(dir);
    };

    // 5. Stage files and calculate hashes
    let mut staged_items = Vec::new(); // (staged_path, target_path, sha256, target_str)
    for (idx, f) in target_files.iter().enumerate() {
        let src_path = match validate_package_source_file(package_dir, &f.source) {
            Ok(p) => p,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(e);
            }
        };
        let target_path = match validate_target_safety(&f.target, home_dir) {
            Ok(p) => p,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(e);
            }
        };

        let staged_file = staging_dir.join(format!("payload_{}", idx));
        if let Err(e) = fs::copy(&src_path, &staged_file) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to stage file '{}': {}", f.source, e));
        }

        let hash = match compute_sha256_file(&staged_file) {
            Ok(h) => h,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(e);
            }
        };

        staged_items.push((staged_file, target_path, hash, f.target.clone()));
    }

    // 6. Materialize files atomically
    let mut materialized_targets = Vec::new();
    let mut installed_file_entries = Vec::new();

    for (staged_file, target_path, hash, target_str) in staged_items {
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                // Rollback if possible
                if !snapshot_id.is_empty() {
                    let _ = restore_snapshot_in(&snapshot_id, home_dir, snapshots_root);
                }
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to create target directory '{}': {}",
                    parent.display(),
                    e
                ));
            }
        }

        // Write atomically via temporary sibling
        let temp_dest = target_path.with_extension(format!("tmp_{}", ts));
        if let Err(e) = fs::copy(&staged_file, &temp_dest) {
            let _ = fs::remove_file(&temp_dest);
            if !snapshot_id.is_empty() {
                let _ = restore_snapshot_in(&snapshot_id, home_dir, snapshots_root);
            }
            clean_staging(&staging_dir);
            return Err(format!(
                "Failed to write target file '{}': {}",
                target_path.display(),
                e
            ));
        }

        if let Err(e) = fs::rename(&temp_dest, &target_path) {
            let _ = fs::remove_file(&temp_dest);
            if !snapshot_id.is_empty() {
                let _ = restore_snapshot_in(&snapshot_id, home_dir, snapshots_root);
            }
            clean_staging(&staging_dir);
            return Err(format!(
                "Failed to materialize file '{}': {}",
                target_path.display(),
                e
            ));
        }

        materialized_targets.push(target_str.clone());
        installed_file_entries.push(InstalledFileEntry {
            target: target_str,
            sha256: hash,
            is_symlink: false,
            symlink_target: None,
        });
    }

    clean_staging(&staging_dir);

    // 7. Write authoritative installed package record
    fs::create_dir_all(installed_root)
        .map_err(|e| format!("Failed to create installed directory: {}", e))?;
    let record_file = installed_root.join(format!("{}.json", manifest.id));
    let record = InstalledPackageRecord {
        package_id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        package_type: Some(manifest.package_type),
        repository_id: Some("community".to_string()),
        installed_at: ts,
        snapshot_id: snapshot_id.clone(),
        installed_files: materialized_targets.clone(),
        files: installed_file_entries,
        package_source_path: package_dir.display().to_string(),
        history: vec![InstalledHistoryEntry {
            version: manifest.version.clone(),
            installed_at: ts,
            snapshot_id: snapshot_id.clone(),
            repository_id: Some("community".to_string()),
            tree_hash: None,
        }],
    };

    let record_json = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("Failed to serialize installed record: {}", e))?;
    fs::write(&record_file, record_json)
        .map_err(|e| format!("Failed to write installed package record: {}", e))?;

    // Invariant: DOES NOT touch active_lockscreen.json or system drop-ins. Installed != Active!

    Ok(InstallResult {
        success: true,
        package_id: manifest.id,
        version: manifest.version,
        snapshot_id,
        installed_files: materialized_targets,
        errors: vec![],
        rolled_back: false,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Transactional Package Uninstallation
// ─────────────────────────────────────────────────────────────────────────────

pub fn uninstall_package_in(
    package_id: &str,
    home_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    state_dir: &Path,
) -> Result<UninstallResult, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    // 1. Fail-closed active target verification
    let active_state = get_active_lockscreen_state_in(state_dir);
    let slug = package_id
        .strip_prefix("lockscreen-qylock-")
        .or_else(|| package_id.strip_prefix("lockscreen-"))
        .unwrap_or(package_id);

    let is_active = active_state.quickshell.as_deref() == Some(package_id)
        || active_state.quickshell.as_deref() == Some(slug)
        || active_state.session_lock.as_deref() == Some(package_id)
        || active_state.session_lock.as_deref() == Some(slug)
        || active_state.hyprlock.as_deref() == Some(package_id)
        || active_state.hyprlock.as_deref() == Some(slug)
        || active_state.sddm.as_deref() == Some(package_id)
        || active_state.sddm.as_deref() == Some(slug)
        || active_state.sddm_login.as_deref() == Some(package_id)
        || active_state.sddm_login.as_deref() == Some(slug)
        || active_state.sddm_lock.as_deref() == Some(package_id)
        || active_state.sddm_lock.as_deref() == Some(slug);

    if is_active {
        return Err(format!(
            "Cannot uninstall active package '{}'. Please deactivate it first to restore system defaults safely.",
            package_id
        ));
    }

    // 2. Read installed record
    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let record_raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read installed package record: {}", e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&record_raw)
        .map_err(|e| format!("Failed to parse installed package record: {}", e))?;

    // 3. Pre-uninstall snapshot
    let snapshot_label = format!("pre-uninstall-{}", package_id);
    let snapshot_paths: Vec<String> = record.installed_files.clone();
    let snapshot_meta =
        create_snapshot_in(&snapshot_label, &snapshot_paths, home_dir, snapshots_root)
            .map_err(|e| format!("Failed to create pre-uninstall snapshot: {}", e))?;

    // 4. Remove files safely
    let mut removed_files = Vec::new();
    let mut removal_errors = Vec::new();

    for target_str in &record.installed_files {
        match validate_target_safety(target_str, home_dir) {
            Ok(p) => {
                if p.is_file() || p.is_symlink() {
                    if let Err(e) = fs::remove_file(&p) {
                        removal_errors.push(format!("Failed to remove '{}': {}", p.display(), e));
                    } else {
                        removed_files.push(target_str.clone());
                        if let Some(parent) = p.parent() {
                            cleanup_empty_parents(parent, home_dir);
                        }
                    }
                }
            }
            Err(e) => {
                removal_errors.push(e);
            }
        }
    }

    // 5. Remove record file
    let _ = fs::remove_file(&record_file);

    Ok(UninstallResult {
        success: removal_errors.is_empty(),
        package_id: package_id.to_string(),
        snapshot_id: snapshot_meta.id,
        removed_files,
        errors: removal_errors,
        rolled_back: false,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Record & State Accessors
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_installed_package_in(
    package_id: &str,
    installed_root: &Path,
) -> Result<Option<InstalledPackageRecord>, String> {
    let rec_file = installed_root.join(format!("{}.json", package_id));
    if !rec_file.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&rec_file).map_err(|e| format!("Failed to read record: {}", e))?;
    let rec: InstalledPackageRecord =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse record: {}", e))?;
    Ok(Some(rec))
}

pub fn list_installed_packages_in(
    installed_root: &Path,
) -> Result<Vec<InstalledPackageRecord>, String> {
    let mut result = Vec::new();
    if !installed_root.is_dir() {
        return Ok(result);
    }
    for entry in fs::read_dir(installed_root)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(raw) = fs::read_to_string(&p) {
                if let Ok(rec) = serde_json::from_str::<InstalledPackageRecord>(&raw) {
                    result.push(rec);
                }
            }
        }
    }
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

pub fn get_active_lockscreen_state_in(state_dir: &Path) -> ActiveLockscreenState {
    let state_file = state_dir.join("active_lockscreen.json");
    if state_file.is_file() {
        if let Ok(raw) = fs::read_to_string(&state_file) {
            if let Ok(s) = serde_json::from_str::<ActiveLockscreenState>(&raw) {
                return s;
            }
        }
    }
    ActiveLockscreenState::default()
}

pub fn save_active_lockscreen_state_in(
    state_dir: &Path,
    state: &ActiveLockscreenState,
) -> Result<(), String> {
    fs::create_dir_all(state_dir).map_err(|e| format!("Failed to create state dir: {}", e))?;
    let state_file = state_dir.join("active_lockscreen.json");
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize active state: {}", e))?;
    fs::write(&state_file, json)
        .map_err(|e| format!("Failed to write active state file: {}", e))?;
    Ok(())
}

pub fn find_package_dir(package_id: &str, repo_root: Option<&Path>) -> Result<PathBuf, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    if let Some(root) = repo_root {
        let candidates = [
            root.join("repositories/community/packages")
                .join(package_id),
            root.join("packages").join(package_id),
            root.join("repositories/packages").join(package_id),
        ];
        for c in &candidates {
            if c.join("manifest.json").is_file() {
                return Ok(c.clone());
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let mut candidates = vec![
            cwd.join("repositories/community/packages").join(package_id),
            cwd.join("packages").join(package_id),
        ];
        if let Some(parent) = cwd.parent() {
            candidates.push(
                parent
                    .join("repositories/community/packages")
                    .join(package_id),
            );
            candidates.push(parent.join("packages").join(package_id));
            if let Some(grandparent) = parent.parent() {
                candidates.push(
                    grandparent
                        .join("repositories/community/packages")
                        .join(package_id),
                );
                candidates.push(grandparent.join("packages").join(package_id));
            }
        }
        for c in &candidates {
            if c.join("manifest.json").is_file() {
                return Ok(c.clone());
            }
        }
    }

    Err(format!(
        "Package '{}' directory not found in repository or workspace",
        package_id
    ))
}

pub fn apply_lockscreen_target_in(
    package_id: &str,
    target: &str,
    home: &Path,
    installed_root: &Path,
    state_dir: &Path,
    sys_root: Option<&Path>,
) -> Result<ActiveLockscreenState, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let norm_target = target.trim().to_lowercase();
    let is_session = norm_target == "session_lock" || norm_target == "quickshell";
    let is_sddm =
        norm_target == "sddm_login" || norm_target == "sddm" || norm_target == "login_screen";
    let is_sddm_lock = norm_target == "sddm_lock";

    if is_sddm_lock {
        return Err("Target 'sddm_lock' is not supported on Linux (SDDM provides login screen greeter integration only; Wayland session locking is unsupported). State preserved.".to_string());
    }

    if !is_session && !is_sddm {
        return Err(format!(
            "Unknown or unsupported activation target: '{}'",
            target
        ));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!(
            "Package '{}' is not installed. Materialize/install it before applying.",
            package_id
        ));
    }

    let mut state = get_active_lockscreen_state_in(state_dir);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if is_session {
        return Err("Desktop session lock is not managed by Ryzora. Dusky/Hyprlock controls your desktop lock independently.".to_string());
    }

    if is_sddm {
        let is_test_root = sys_root.is_some() || std::env::var("RYZORA_SYSTEM_ROOT").is_ok();
        let helper_installed = is_test_root
            || Path::new("/usr/lib/ryzora/ryzora-sddm-helper").exists()
            || Path::new("/usr/local/bin/ryzora-sddm-helper").exists()
            || Path::new("/usr/libexec/ryzora-sddm-helper").exists();

        if !helper_installed {
            return Err("SDDM activation requires privileged helper '/usr/lib/ryzora/ryzora-sddm-helper'. System configuration cannot be modified safely without helper elevation.".to_string());
        }

        let slug = package_id
            .strip_prefix("lockscreen-qylock-")
            .or_else(|| package_id.strip_prefix("lockscreen-"))
            .unwrap_or(package_id);

        if package_id.starts_with("silentsddm-") {
            let asset_name = package_id.strip_prefix("silentsddm-").unwrap_or(package_id);
            let sys_silent_dir = if let Some(root) = sys_root {
                root.join("usr/share/sddm/themes/ryzora-silent")
            } else {
                PathBuf::from("/usr/share/sddm/themes/ryzora-silent")
            };

            // 1. Resolve wallpaper source file and its filename
            let mut wp_filename = format!("{}.mp4", asset_name);
            let candidate_wp_sources = [
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}.mp4", asset_name)),
                sys_silent_dir.join(format!("backgrounds/{}.mp4", asset_name)),
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}.webm", asset_name)),
                sys_silent_dir.join(format!("backgrounds/{}.webm", asset_name)),
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}.jpg", asset_name)),
                sys_silent_dir.join(format!("backgrounds/{}.jpg", asset_name)),
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}.png", asset_name)),
                sys_silent_dir.join(format!("backgrounds/{}.png", asset_name)),
                PathBuf::from(format!("repositories/community/packages/{}/files/backgrounds/{}.mp4", package_id, asset_name)),
                home.join(".local/share/ryzora/lockscreens/silentsddm/wallpapers/default.jpg"),
                sys_silent_dir.join("backgrounds/default.jpg"),
            ];

            let mut found_wp_source: Option<PathBuf> = None;
            for c in &candidate_wp_sources {
                if c.is_file() {
                    found_wp_source = Some(c.clone());
                    if let Some(fn_os) = c.file_name() {
                        wp_filename = fn_os.to_string_lossy().to_string();
                    }
                    break;
                }
            }

            // 2. Resolve placeholder image source file
            let placeholder_filename = format!("{}.png", asset_name);
            let candidate_ph_sources = [
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}", placeholder_filename)),
                sys_silent_dir.join(format!("backgrounds/{}", placeholder_filename)),
                PathBuf::from(format!("repositories/community/packages/{}/files/backgrounds/{}", package_id, placeholder_filename)),
                home.join(format!(".local/share/ryzora/lockscreens/silentsddm/wallpapers/{}.jpg", asset_name)),
                sys_silent_dir.join(format!("backgrounds/{}.jpg", asset_name)),
            ];
            let mut found_ph_source: Option<PathBuf> = None;
            for p in &candidate_ph_sources {
                if p.is_file() {
                    found_ph_source = Some(p.clone());
                    break;
                }
            }

            // 3. Read authentic base config for the theme (e.g. silvia.conf, ken.conf, default.conf)
            let candidate_base_confs = [
                sys_silent_dir.join(format!("configs/{}.conf", asset_name)),
                PathBuf::from(format!("repositories/community/packages/{}/files/configs/{}.conf", package_id, asset_name)),
                PathBuf::from(format!("repositories/community/packages/{}/files/{}.conf", package_id, asset_name)),
                sys_silent_dir.join("configs/default.conf"),
                PathBuf::from("repositories/community/packages/silentsddm-ken/files/configs/ken.conf"),
            ];

            let mut base_conf_str = String::new();
            for c in &candidate_base_confs {
                if c.is_file() {
                    if let Ok(content) = fs::read_to_string(c) {
                        if content.contains("[General]") && !content.contains("[%General]") {
                            base_conf_str = content;
                            break;
                        }
                    }
                }
            }
            if base_conf_str.is_empty() {
                base_conf_str = format!(
r#"[General]
scale = 1.0
enable-animations = true
animated-background-placeholder = {ph}
background-fill-mode = "fill"

[LockScreen]
display = true
background = "{wp}"
blur = 0
brightness = 0.0
saturation = 0.0

[LockScreen.Clock]
display = true
position = "center-right"
format = "hh:mm"

[LockScreen.Date]
display = true
format = "dddd, MMMM dd, yyyy"

[LoginScreen]
background = "{wp}"
blur = 0
brightness = 0.0
saturation = 0.0

[LoginScreen.LoginArea]
position = "right"

[LoginScreen.LoginArea.Avatar]
shape = "circle"
"#, wp = wp_filename, ph = placeholder_filename);
            }

            // 4. Read user overrides from configuration.json
            let mut user_overrides: Option<serde_json::Value> = None;
            let config_json_path = home.join(".local/share/ryzora/lockscreens/silentsddm/configuration.json");
            if config_json_path.is_file() {
                if let Ok(raw_json) = fs::read_to_string(&config_json_path) {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&raw_json) {
                        if let Some(themes) = json_val.get("themes").and_then(|t| t.as_object()) {
                            if let Some(pkg_cfg) = themes.get(package_id) {
                                user_overrides = Some(pkg_cfg.clone());
                            }
                        }
                        if user_overrides.is_none() && !json_val.get("themes").is_some() {
                            user_overrides = Some(json_val);
                        }
                    }
                }
            }

            // 5. Merge overrides and wallpaper into ryzora-active.conf
            let mut active_conf_lines = Vec::new();
            let mut cur_sec = String::new();
            let ovr = user_overrides.unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            let mut has_lock_bg = false;
            let mut has_login_bg = false;
            let mut has_placeholder = false;

            for line in base_conf_str.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    if cur_sec == "[General]" && !has_placeholder {
                        active_conf_lines.push(format!("animated-background-placeholder = {}", placeholder_filename));
                        has_placeholder = true;
                    } else if cur_sec == "[LockScreen]" && !has_lock_bg {
                        active_conf_lines.push(format!("background = \"{}\"", wp_filename));
                        has_lock_bg = true;
                    } else if cur_sec == "[LoginScreen]" && !has_login_bg {
                        active_conf_lines.push(format!("background = \"{}\"", wp_filename));
                        has_login_bg = true;
                    }

                    cur_sec = trimmed.to_string();
                    active_conf_lines.push(line.to_string());
                    continue;
                }

                if cur_sec == "[General]" {
                    if trimmed.starts_with("animated-background-placeholder") {
                        active_conf_lines.push(format!("animated-background-placeholder = {}", placeholder_filename));
                        has_placeholder = true;
                        continue;
                    }
                    if trimmed.starts_with("background-fill-mode") {
                        if let Some(fm) = ovr.get("fillMode").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("background-fill-mode = \"{}\"", fm));
                            continue;
                        }
                    }
                    if trimmed.starts_with("enable-animations") {
                        if let Some(ea) = ovr.get("enableAnimations") {
                            let b = ea.as_bool().unwrap_or_else(|| ea.as_str().map(|s| s == "true" || s == "1").unwrap_or(true));
                            active_conf_lines.push(format!("enable-animations = {}", if b { "true" } else { "false" }));
                            continue;
                        }
                    }
                } else if cur_sec == "[LockScreen]" {
                    if trimmed.starts_with("background =") {
                        active_conf_lines.push(format!("background = \"{}\"", wp_filename));
                        has_lock_bg = true;
                        continue;
                    }
                    if trimmed.starts_with("blur =") {
                        if let Some(bl) = ovr.get("blur").and_then(|v| v.as_i64()) {
                            active_conf_lines.push(format!("blur = {}", bl));
                            continue;
                        }
                    }
                    if trimmed.starts_with("brightness =") {
                        if let Some(br) = ovr.get("brightness").and_then(|v| v.as_str()) {
                            let bval = match br { "dark" => "-0.4", "dim" => "-0.2", "bright" => "0.2", _ => "0.0" };
                            active_conf_lines.push(format!("brightness = {}", bval));
                            continue;
                        }
                    }
                    if trimmed.starts_with("saturation =") {
                        if let Some(st) = ovr.get("saturation").and_then(|v| v.as_str()) {
                            let sval = match st { "grayscale" => "-1.0", "muted" => "-0.5", "vibrant" => "0.2", _ => "0.0" };
                            active_conf_lines.push(format!("saturation = {}", sval));
                            continue;
                        }
                    }
                } else if cur_sec == "[LockScreen.Clock]" {
                    if trimmed.starts_with("display =") {
                        if let Some(cd) = ovr.get("clockDisplay") {
                            let b = cd.as_bool().unwrap_or_else(|| cd.as_str().map(|s| s == "true" || s == "1").unwrap_or(true));
                            active_conf_lines.push(format!("display = {}", if b { "true" } else { "false" }));
                            continue;
                        }
                    }
                    if trimmed.starts_with("position =") {
                        if let Some(cp) = ovr.get("clockPosition").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("position = \"{}\"", cp));
                            continue;
                        }
                    }
                    if trimmed.starts_with("format =") {
                        if let Some(cf) = ovr.get("clockFormat").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("format = \"{}\"", cf));
                            continue;
                        }
                    }
                } else if cur_sec == "[LockScreen.Date]" {
                    if trimmed.starts_with("display =") {
                        if let Some(dd) = ovr.get("dateDisplay") {
                            let b = dd.as_bool().unwrap_or_else(|| dd.as_str().map(|s| s == "true" || s == "1").unwrap_or(true));
                            active_conf_lines.push(format!("display = {}", if b { "true" } else { "false" }));
                            continue;
                        }
                    }
                    if trimmed.starts_with("format =") {
                        if let Some(df) = ovr.get("dateFormat").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("format = \"{}\"", df));
                            continue;
                        }
                    }
                } else if cur_sec == "[LoginScreen]" {
                    if trimmed.starts_with("background =") {
                        active_conf_lines.push(format!("background = \"{}\"", wp_filename));
                        has_login_bg = true;
                        continue;
                    }
                    if trimmed.starts_with("blur =") {
                        if let Some(bl) = ovr.get("blur").and_then(|v| v.as_i64()) {
                            active_conf_lines.push(format!("blur = {}", bl));
                            continue;
                        }
                    }
                    if trimmed.starts_with("brightness =") {
                        if let Some(br) = ovr.get("brightness").and_then(|v| v.as_str()) {
                            let bval = match br { "dark" => "-0.4", "dim" => "-0.2", "bright" => "0.2", _ => "0.0" };
                            active_conf_lines.push(format!("brightness = {}", bval));
                            continue;
                        }
                    }
                    if trimmed.starts_with("saturation =") {
                        if let Some(st) = ovr.get("saturation").and_then(|v| v.as_str()) {
                            let sval = match st { "grayscale" => "-1.0", "muted" => "-0.5", "vibrant" => "0.2", _ => "0.0" };
                            active_conf_lines.push(format!("saturation = {}", sval));
                            continue;
                        }
                    }
                } else if cur_sec == "[LoginScreen.LoginArea]" {
                    if trimmed.starts_with("position =") {
                        if let Some(lp) = ovr.get("loginAreaPosition").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("position = \"{}\"", lp));
                            continue;
                        }
                    }
                } else if cur_sec == "[LoginScreen.LoginArea.Avatar]" {
                    if trimmed.starts_with("shape =") {
                        if let Some(as_) = ovr.get("avatarShape").and_then(|v| v.as_str()) {
                            active_conf_lines.push(format!("shape = \"{}\"", as_));
                            continue;
                        }
                    }
                }

                active_conf_lines.push(line.to_string());
            }

            if cur_sec == "[General]" && !has_placeholder {
                active_conf_lines.push(format!("animated-background-placeholder = {}", placeholder_filename));
            } else if cur_sec == "[LockScreen]" && !has_lock_bg {
                active_conf_lines.push(format!("background = \"{}\"", wp_filename));
            } else if cur_sec == "[LoginScreen]" && !has_login_bg {
                active_conf_lines.push(format!("background = \"{}\"", wp_filename));
            }

            let ryzora_active_conf_content = active_conf_lines.join("\n") + "\n";

            // 6. Apply configuration and wallpaper
            if let Some(root) = sys_root {
                let dropin_dir = root.join("etc/sddm.conf.d");
                fs::create_dir_all(&dropin_dir)
                    .map_err(|e| format!("Failed to create SDDM test drop-in directory: {}", e))?;
                let dropin_file = dropin_dir.join("zz-ryzora-theme.conf");
                fs::write(&dropin_file, "[Theme]\nCurrent=ryzora-silent\n")
                    .map_err(|e| format!("Failed to write SDDM test drop-in: {}", e))?;
                let sddm_silent_dir = root.join("usr/share/sddm/themes/ryzora-silent");
                let configs_dir = sddm_silent_dir.join("configs");
                fs::create_dir_all(&configs_dir)
                    .map_err(|e| format!("Failed to create configs dir: {}", e))?;
                fs::write(configs_dir.join("ryzora-active.conf"), &ryzora_active_conf_content)
                    .map_err(|e| format!("Failed to write ryzora-active.conf: {}", e))?;
                let backgrounds_dir = sddm_silent_dir.join("backgrounds");
                fs::create_dir_all(&backgrounds_dir)
                    .map_err(|e| format!("Failed to create backgrounds dir: {}", e))?;
                if let Some(ref src) = found_wp_source {
                    let _ = fs::copy(src, backgrounds_dir.join(&wp_filename));
                } else {
                    let _ = fs::write(backgrounds_dir.join(&wp_filename), b"video_bytes");
                }
                if let Some(ref ph) = found_ph_source {
                    let _ = fs::copy(ph, backgrounds_dir.join(&placeholder_filename));
                }
                let meta_path = sddm_silent_dir.join("metadata.desktop");
                if meta_path.is_file() {
                    if let Ok(meta_raw) = fs::read_to_string(&meta_path) {
                        let mut m_lines = Vec::new();
                        let mut has_cfg = false;
                        for l in meta_raw.lines() {
                            if l.trim_start().starts_with("ConfigFile=") {
                                m_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
                                has_cfg = true;
                            } else {
                                m_lines.push(l.to_string());
                            }
                        }
                        if !has_cfg {
                            m_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
                        }
                        let _ = fs::write(&meta_path, m_lines.join("\n") + "\n");
                    }
                }
            } else {
                let sddm_silent_dir = Path::new("/usr/share/sddm/themes/ryzora-silent");
                if !sddm_silent_dir.join("Main.qml").exists() {
                    return Err("SilentSDDM base theme engine is not installed at /usr/share/sddm/themes/ryzora-silent. Please install it first.".to_string());
                }

                // Stage ryzora-active.conf and wallpaper in staging dir for privileged helper
                let staging_dir = PathBuf::from(format!("/tmp/ryzora-staging-silent-{}-{}", asset_name, now));
                let staging_bg = staging_dir.join("backgrounds");
                let staging_cfg = staging_dir.join("configs");
                fs::create_dir_all(&staging_bg)
                    .map_err(|e| format!("Failed to create staging backgrounds dir: {}", e))?;
                fs::create_dir_all(&staging_cfg)
                    .map_err(|e| format!("Failed to create staging configs dir: {}", e))?;

                fs::write(staging_cfg.join("ryzora-active.conf"), &ryzora_active_conf_content)
                    .map_err(|e| format!("Failed to write staged ryzora-active.conf: {}", e))?;

                if let Some(ref src) = found_wp_source {
                    let _ = fs::copy(src, staging_bg.join(&wp_filename));
                } else {
                    let installed_bg = sddm_silent_dir.join("backgrounds").join(&wp_filename);
                    if installed_bg.is_file() {
                        let _ = fs::copy(&installed_bg, staging_bg.join(&wp_filename));
                    } else {
                        let _ = fs::write(staging_bg.join(&wp_filename), b"dummy_content");
                    }
                }

                if let Some(ref ph) = found_ph_source {
                    let _ = fs::copy(ph, staging_bg.join(&placeholder_filename));
                } else {
                    let installed_ph = sddm_silent_dir.join("backgrounds").join(&placeholder_filename);
                    if installed_ph.is_file() {
                        let _ = fs::copy(&installed_ph, staging_bg.join(&placeholder_filename));
                    }
                }

                let meta_src = sddm_silent_dir.join("metadata.desktop");
                if meta_src.is_file() {
                    if let Ok(meta_raw) = fs::read_to_string(&meta_src) {
                        let mut m_lines = Vec::new();
                        let mut has_cfg = false;
                        for l in meta_raw.lines() {
                            if l.trim_start().starts_with("ConfigFile=") {
                                m_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
                                has_cfg = true;
                            } else {
                                m_lines.push(l.to_string());
                            }
                        }
                        if !has_cfg {
                            m_lines.push("ConfigFile=configs/ryzora-active.conf".to_string());
                        }
                        let _ = fs::write(staging_dir.join("metadata.desktop"), m_lines.join("\n") + "\n");
                    }
                }

                let helper_res = crate::sddm_helper::set_sddm_theme_background(&staging_dir, "silent");
                let _ = fs::remove_dir_all(&staging_dir);
                helper_res.map_err(|e| format!("Failed to update SDDM silent background/config: {}", e))?;

                crate::sddm_helper::activate_sddm_theme("silent")
                    .map_err(|e| format!("Failed to activate SDDM theme: {}", e))?;
            }
            if state.sddm_previous_theme.is_none() {
                state.sddm_previous_theme = Some("garuda-white".to_string());
            }
            state.sddm = Some(package_id.to_string());
            state.sddm_login = Some(package_id.to_string());
            state.last_applied_at = Some(now);

            save_active_lockscreen_state_in(state_dir, &state)?;
            return Ok(state);
        }

        if let Some(root) = sys_root {
            let dropin_dir = root.join("etc/sddm.conf.d");
            fs::create_dir_all(&dropin_dir)
                .map_err(|e| format!("Failed to create SDDM test drop-in directory: {}", e))?;
            let dropin_file = dropin_dir.join("zz-ryzora-theme.conf");
            fs::write(&dropin_file, format!("[Theme]\nCurrent=ryzora-{}\n", slug))
                .map_err(|e| format!("Failed to write SDDM test drop-in: {}", e))?;
        } else {
            // Stage and install to /usr/share/sddm/themes/ryzora-{slug}
            let staging_dir = PathBuf::from(format!("/tmp/ryzora-staging-sddm-{}-{}", slug, now));
            let sddm_theme_staging = staging_dir.join(format!("ryzora-{}", slug));
            fs::create_dir_all(&sddm_theme_staging)
                .map_err(|e| format!("Failed to create staging directory for SDDM theme: {}", e))?;

            let candidate_dirs = [
                home.join(format!(".local/share/ryzora/lockscreens/qylock/{}", slug)),
                home.join(format!(".local/share/ryzora/installed/{}/files", package_id)),
                PathBuf::from(format!("repositories/community/packages/{}/files", package_id)),
                PathBuf::from(format!("/mnt/windows/Users/subha/Documents/Code Playground/Ryzora/repositories/community/packages/{}/files", package_id)),
            ];

            let mut found_dir: Option<PathBuf> = None;
            for c in &candidate_dirs {
                if c.join("Main.qml").exists() {
                    found_dir = Some(c.clone());
                    break;
                }
            }

            if let Some(ref dir) = found_dir {
                fn copy_all_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
                    if src.is_dir() {
                        fs::create_dir_all(dst)?;
                        for entry in fs::read_dir(src)? {
                            let entry = entry?;
                            let path = entry.path();
                            let target = dst.join(entry.file_name());
                            if path.is_dir() {
                                copy_all_recursive(&path, &target)?;
                            } else {
                                fs::copy(&path, &target)?;
                            }
                        }
                    }
                    Ok(())
                }

                if let Err(e) = copy_all_recursive(dir, &sddm_theme_staging) {
                    let _ = fs::remove_dir_all(&staging_dir);
                    return Err(format!("Failed to stage SDDM theme files: {}", e));
                }
            } else {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(format!("Could not locate Main.qml for theme '{}'", package_id));
            }

            // Install theme via privileged helper
            let install_res = crate::sddm_helper::install_sddm_theme(&sddm_theme_staging, slug);
            let _ = fs::remove_dir_all(&staging_dir);
            install_res.map_err(|e| format!("Failed to install SDDM theme: {}", e))?;

            // Snapshot previous SDDM theme if not recorded
            if state.sddm_previous_theme.is_none() {
                let (curr_theme, _) = crate::sddm_helper::read_current_sddm_theme_in(sys_root);
                state.sddm_previous_theme = curr_theme;
            }

            // Activate SDDM theme
            crate::sddm_helper::activate_sddm_theme(slug)
                .map_err(|e| format!("Failed to activate SDDM theme: {}", e))?;
        }

        if state.sddm_previous_theme.is_none() {
            state.sddm_previous_theme = Some("garuda-white".to_string());
        }
        state.sddm = Some(package_id.to_string());
        state.sddm_login = Some(package_id.to_string());
        state.last_applied_at = Some(now);
    }

    save_active_lockscreen_state_in(state_dir, &state)?;
    Ok(state)
}

/// Reconciles state in `active_lockscreen.json` with the real system and host files.
pub fn reconcile_active_state_in(
    home: &Path,
    state_dir: &Path,
    sys_root: Option<&Path>,
) -> Result<ActiveLockscreenState, String> {
    let mut state = get_active_lockscreen_state_in(state_dir);
    let mut modified = false;

    // 1. Reconcile session_lock: only active if wrappers/shims and valid symlink exist
    let active_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
    let wrapper_exists = home.join(".local/bin/ryzora-lock").exists();
    let shim_exists = home.join(".local/bin/hyprlock").exists();
    let symlink_valid = active_symlink.is_dir() && active_symlink.join("Main.qml").exists();

    if state.session_lock.is_some() {
        if !symlink_valid || (!wrapper_exists && !shim_exists) {
            state.session_lock = None;
            state.quickshell = None;
            state.quickshell_theme_path = None;
            state.hyprlock = None;
            modified = true;
        }
    } else if !wrapper_exists
        && !shim_exists
        && (active_symlink.exists() || fs::symlink_metadata(&active_symlink).is_ok())
    {
        let _ = fs::remove_file(&active_symlink);
        let _ = fs::remove_dir_all(&active_symlink);
    }

    // 2. Reconcile sddm_login against /etc/sddm.conf.d/zz-ryzora-theme.conf
    let is_real_user_home = std::env::var("HOME").ok().map(PathBuf::from).as_deref() == Some(home);
    let sddm_dropin = if let Some(root) = sys_root {
        Some(root.join("etc/sddm.conf.d/zz-ryzora-theme.conf"))
    } else if is_real_user_home {
        Some(PathBuf::from("/etc/sddm.conf.d/zz-ryzora-theme.conf"))
    } else {
        None
    };

    if let Some(dropin) = sddm_dropin {
        let sddm_dropin = dropin;

        if sddm_dropin.is_file() {
            if let Ok(file_content) = fs::read_to_string(&sddm_dropin) {
                for line in file_content.lines() {
                    let trimmed = line.trim();
                    if let Some(val) = trimmed.strip_prefix("Current=") {
                        let theme = val.trim();
                        if let Some(slug) = theme.strip_prefix("ryzora-") {
                            let expected_id = format!("lockscreen-qylock-{}", slug);
                            if state.sddm_login.as_deref() != Some(&expected_id)
                                && state.sddm.as_deref() != Some(&expected_id)
                            {
                                state.sddm_login = Some(expected_id.clone());
                                state.sddm = Some(expected_id);
                                modified = true;
                            }
                        }
                    }
                }
            }
        }
    }

    if modified {
        let _ = save_active_lockscreen_state_in(state_dir, &state);
    }

    Ok(state)
}

pub fn deactivate_lockscreen_target_in(
    target: &str,
    home: &Path,
    state_dir: &Path,
    sys_root: Option<&Path>,
) -> Result<ActiveLockscreenState, String> {
    let norm_target = target.trim().to_lowercase();
    let is_session =
        norm_target == "session_lock" || norm_target == "quickshell" || norm_target == "hyprlock";
    let is_sddm =
        norm_target == "sddm_login" || norm_target == "sddm" || norm_target == "login_screen";
    let is_sddm_lock = norm_target == "sddm_lock";
    let is_all = norm_target == "all";

    if is_sddm_lock {
        return Err("Deactivation for target 'sddm_lock' is not supported on Linux (SDDM provides login screen greeter integration only; Wayland session locking is unsupported). State preserved.".to_string());
    }

    if !is_session && !is_sddm && !is_all {
        return Err(format!("Unknown deactivation target: '{}'", target));
    }

    let mut state = get_active_lockscreen_state_in(state_dir);

    if is_session || is_all {
        let active_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
        if active_symlink.exists() || fs::symlink_metadata(&active_symlink).is_ok() {
            let _ = fs::remove_file(&active_symlink);
        }
        let _ = crate::hypridle::remove_hyprlock_shim(home);
        let _ = crate::hypridle::unhook_user_lock_script(home);
        let _ = crate::hypridle::unhook_waybar_lock_trigger(home);
        let _ = crate::hypridle::deactivate_hypridle_integration(home);

        let dropin_file = home.join(".config/hypr/hypridle.conf.d/ryzora.conf");
        if dropin_file.is_file() {
            let _ = fs::remove_file(&dropin_file);
        }
        state.session_lock = None;
        state.quickshell = None;
        state.quickshell_theme_path = None;
        state.hyprlock = None;
    }

    if is_sddm || is_all {
        if let Some(root) = sys_root {
            let dropin_file = root.join("etc/sddm.conf.d/zz-ryzora-theme.conf");
            if dropin_file.is_file() {
                fs::remove_file(&dropin_file)
                    .map_err(|e| format!("Failed to remove SDDM test drop-in: {}", e))?;
            }
        } else {
            let _ = crate::sddm_helper::deactivate_sddm_theme();
        }
        state.sddm = None;
        state.sddm_login = None;
        state.sddm_theme_path = None;
        state.sddm_previous_theme = None;
    }

    save_active_lockscreen_state_in(state_dir, &state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_dummy_package(dir: &Path, id: &str) {
        fs::create_dir_all(dir.join("files/config")).unwrap();
        fs::write(
            dir.join("files/config/dummy.conf"),
            "dummy payload content\n",
        )
        .unwrap();
        let manifest = serde_json::json!({
            "id": id,
            "name": "Dummy Package",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Tester",
            "package_type": "lockscreen",
            "description": "A dummy test package",
            "compatibility": {
                "desktops": [],
                "sessions": [],
                "distros": [],
                "required": [],
                "optional": []
            },
            "files": [
                {
                    "source": "files/config/dummy.conf",
                    "target": "~/.config/dummy/dummy.conf",
                    "description": "Dummy config file"
                }
            ]
        });
        fs::write(
            dir.join("manifest.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_install_and_uninstall_lifecycle() {
        let temp_home = tempdir().unwrap();
        let temp_pkg = tempdir().unwrap();
        let temp_snapshots = tempdir().unwrap();
        let temp_installed = tempdir().unwrap();
        let temp_staging = tempdir().unwrap();
        let temp_state = tempdir().unwrap();

        let pkg_id = "test-pkg-lifecycle";
        create_dummy_package(temp_pkg.path(), pkg_id);

        // 1. Install package
        let res = install_package_in(
            temp_pkg.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_staging.path(),
            temp_home.path(),
            None,
            true,
        )
        .expect("Installation should succeed");

        assert!(res.success);
        assert_eq!(res.package_id, pkg_id);
        assert!(!res.snapshot_id.is_empty());
        assert_eq!(res.installed_files.len(), 1);

        // Verify target file materialized
        let target_file = temp_home.path().join(".config/dummy/dummy.conf");
        assert!(target_file.is_file());
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            "dummy payload content\n"
        );

        // Verify installed record written
        let rec = get_installed_package_in(pkg_id, temp_installed.path())
            .unwrap()
            .expect("Record must exist");
        assert_eq!(rec.name, "Dummy Package");
        assert_eq!(rec.version, "1.0.0");
        assert_eq!(rec.files.len(), 1);

        // Invariant: installing does NOT activate package
        let active = get_active_lockscreen_state_in(temp_state.path());
        assert_ne!(active.quickshell.as_deref(), Some(pkg_id));
        assert_ne!(active.sddm_login.as_deref(), Some(pkg_id));

        // 2. Refuse uninstall if active
        let mut active_mod = active.clone();
        active_mod.quickshell = Some(pkg_id.to_string());
        save_active_lockscreen_state_in(temp_state.path(), &active_mod).unwrap();

        let uninst_err = uninstall_package_in(
            pkg_id,
            temp_home.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_state.path(),
        );
        assert!(uninst_err.is_err());
        assert!(uninst_err
            .unwrap_err()
            .contains("Cannot uninstall active package"));

        // Still installed because uninstallation was refused
        assert!(target_file.is_file());

        // 3. Deactivate then uninstall
        save_active_lockscreen_state_in(temp_state.path(), &ActiveLockscreenState::default())
            .unwrap();
        let uninst_res = uninstall_package_in(
            pkg_id,
            temp_home.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_state.path(),
        )
        .expect("Uninstall should succeed after deactivation");

        assert!(uninst_res.success);
        assert!(!target_file.exists(), "Target file must be removed");
        assert!(get_installed_package_in(pkg_id, temp_installed.path())
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_path_traversal_rejection() {
        let temp_home = tempdir().unwrap();
        assert!(validate_target_safety("~/../../../etc/shadow", temp_home.path()).is_err());
        assert!(validate_target_safety("~/\0bad", temp_home.path()).is_err());
    }

    #[test]
    fn test_target_activation_and_deactivation_lifecycle() {
        let temp_home = tempdir().unwrap();
        let temp_pkg = tempdir().unwrap();
        let temp_snapshots = tempdir().unwrap();
        let temp_installed = tempdir().unwrap();
        let temp_staging = tempdir().unwrap();
        let temp_state = tempdir().unwrap();

        let pkg_id = "test-pkg-activation";
        create_dummy_package(temp_pkg.path(), pkg_id);

        // 1. Install package
        let res = install_package_in(
            temp_pkg.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_staging.path(),
            temp_home.path(),
            None,
            false,
        )
        .expect("Installation should succeed");
        assert!(res.success);

        // Verify package is installed but NOT active
        let state_initial = get_active_lockscreen_state_in(temp_state.path());
        assert!(state_initial.session_lock.is_none());
        assert!(state_initial.sddm.is_none());

        // 2. session_lock activation MUST fail closed
        let act_session_err = apply_lockscreen_target_in(
            pkg_id,
            "session_lock",
            temp_home.path(),
            temp_installed.path(),
            temp_state.path(),
            None,
        );
        assert!(act_session_err.is_err(), "session_lock activation must fail closed");
        assert!(act_session_err.unwrap_err().contains("Desktop session lock is not managed"));

        // 3. Activate sddm_login target (with test sys_root)
        let temp_sysroot = tempfile::tempdir().unwrap();
        let act_sddm_res = apply_lockscreen_target_in(
            pkg_id,
            "sddm_login",
            temp_home.path(),
            temp_installed.path(),
            temp_state.path(),
            Some(temp_sysroot.path()),
        )
        .expect("SDDM login activation must succeed");
        assert_eq!(act_sddm_res.sddm.as_deref(), Some(pkg_id));
        let sddm_dropin = temp_sysroot.path().join("etc/sddm.conf.d/zz-ryzora-theme.conf");
        assert!(sddm_dropin.is_file(), "SDDM drop-in file must be written");

        // 4. Attempting uninstall while active in SDDM MUST fail closed
        let uninst_active = uninstall_package_in(
            pkg_id,
            temp_home.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_state.path(),
        );
        assert!(
            uninst_active.is_err(),
            "Must refuse uninstallation while package is active in SDDM"
        );
        let err_msg = uninst_active.unwrap_err();
        assert!(err_msg.contains("Cannot uninstall active package"));

        // 5. Deactivate sddm_login target
        let deact_res = deactivate_lockscreen_target_in(
            "sddm_login",
            temp_home.path(),
            temp_state.path(),
            Some(temp_sysroot.path()),
        )
        .expect("SDDM deactivation must succeed");

        assert!(deact_res.sddm.is_none());
        assert!(
            !sddm_dropin.exists(),
            "SDDM drop-in must be cleanly removed to restore default greeter"
        );
        // 5. Now uninstallation succeeds cleanly
        let uninst_ok = uninstall_package_in(
            pkg_id,
            temp_home.path(),
            temp_snapshots.path(),
            temp_installed.path(),
            temp_state.path(),
        )
        .expect("Uninstall should succeed after deactivation");
        assert!(uninst_ok.success);
    }

    #[test]
    fn test_sddm_lock_target_rejected() {
        let temp_home = tempdir().unwrap();
        let temp_installed = tempdir().unwrap();
        let temp_state = tempdir().unwrap();

        // Activation of unsupported sddm_lock target
        let err_act = apply_lockscreen_target_in(
            "any-pkg",
            "sddm_lock",
            temp_home.path(),
            temp_installed.path(),
            temp_state.path(),
            None,
        )
        .unwrap_err();
        assert!(err_act.contains("not supported on Linux"));

        // Deactivation of unsupported sddm_lock target
        let err_deact =
            deactivate_lockscreen_target_in("sddm_lock", temp_home.path(), temp_state.path(), None)
                .unwrap_err();
        assert!(err_deact.contains("not supported on Linux"));
    }

    #[test]
    fn test_activation_requires_installed_package() {
        let temp_home = tempdir().unwrap();
        let temp_installed = tempdir().unwrap();
        let temp_state = tempdir().unwrap();

        let err = apply_lockscreen_target_in(
            "non-installed-pkg",
            "session_lock",
            temp_home.path(),
            temp_installed.path(),
            temp_state.path(),
            None,
        )
        .unwrap_err();
        assert!(err.contains("is not installed. Materialize/install it before applying."));
    }
}
