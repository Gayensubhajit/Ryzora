use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::compatibility::{
    evaluate_compatibility, CompatibilityLevel, CompatibilityRequirements,
};
use crate::manifest::{validate_manifest_internal, RyzoraManifest};
use crate::snapshot::{
    create_snapshot_in, get_home_dir, get_ryzora_snapshots_dir, restore_snapshot_in,
    sha256_file, validate_and_expand_path, verify_snapshot_in,
};
use crate::system::{detect_system_info, SystemInfo};

// ─────────────────────────────────────────────────────────────────────────────
// Installer Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// High-level plan generated for preview or pre-flight check.
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
    pub compatibility_status: String, // "compatible", "missing_dependencies", "incompatible"
    pub required_dependencies: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub warnings: Vec<String>,
}

/// Result returned after attempting an installation.
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

/// Authoritative record stored in ~/.local/share/ryzora/installed/<pkg_id>.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackageRecord {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub installed_at: u64,
    pub snapshot_id: String,
    pub installed_files: Vec<String>,
    pub package_source_path: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Resolvers
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_ryzora_base_dir() -> PathBuf {
    get_home_dir().join(".local/share/ryzora")
}

pub fn get_ryzora_installed_dir() -> PathBuf {
    get_ryzora_base_dir().join("installed")
}

pub fn get_ryzora_staging_dir() -> PathBuf {
    get_ryzora_base_dir().join("staging")
}

/// Locate a package directory by ID.
pub fn find_package_dir(
    package_id: &str,
    custom_packages_root: Option<&Path>,
) -> Result<PathBuf, String> {
    if package_id.is_empty() || package_id.contains("..") || package_id.contains('/') || package_id.contains('\\') {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    if let Some(custom_root) = custom_packages_root {
        let candidate = custom_root.join(package_id);
        if candidate.join("manifest.json").is_file() {
            return Ok(candidate);
        }
    }

    // Check Repository Manager (Phase 5)
    if let Ok(repo_dir) = crate::repository::find_package_dir(package_id) {
        return Ok(repo_dir);
    }

    // Check project workspace / current directory `packages/<id>`
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("packages").join(package_id);
        if candidate.join("manifest.json").is_file() {
            return Ok(candidate);
        }
    }

    // Check application data dir `~/.local/share/ryzora/packages/<id>`
    let local_share = get_ryzora_base_dir().join("packages").join(package_id);
    if local_share.join("manifest.json").is_file() {
        return Ok(local_share);
    }

    Err(format!(
        "Package '{}' not found in package repositories",
        package_id
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Payload & Security Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Maximum single file size allowed (50 MB).
const MAX_PAYLOAD_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// Validate a single source path inside a package.
/// Must be a relative path, must not contain '..', must not escape via symlink,
/// must be a regular file, and must not exceed size limits.
pub fn validate_package_source_file(
    package_root: &Path,
    source_rel: &str,
) -> Result<PathBuf, String> {
    if source_rel.starts_with('/') || source_rel.starts_with('\\') {
        return Err(format!(
            "Source path '{}' must be relative to package root",
            source_rel
        ));
    }
    if source_rel.contains('\0') {
        return Err(format!("Source path '{}' contains null bytes", source_rel));
    }
    for comp in Path::new(source_rel).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Source path '{}' contains forbidden parent traversal ('..')",
                source_rel
            ));
        }
    }

    let full_path = package_root.join(source_rel);
    if !full_path.exists() {
        return Err(format!(
            "Package payload file missing: '{}'",
            source_rel
        ));
    }

    // Check symlink safety: resolved path must stay within package_root
    let canonical_root = package_root
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize package root: {}", e))?;
    let canonical_file = full_path
        .canonicalize()
        .map_err(|e| format!("Failed to canonicalize source file '{}': {}", source_rel, e))?;

    if !canonical_file.starts_with(&canonical_root) {
        return Err(format!(
            "Source path '{}' escapes package root via symlink",
            source_rel
        ));
    }

    // Verify it's a regular file (reject sockets, FIFOs, device nodes, dirs)
    let meta = fs::symlink_metadata(&full_path)
        .map_err(|e| format!("Failed to read metadata for '{}': {}", source_rel, e))?;
    if meta.file_type().is_symlink() {
        // If it's a symlink within the package, ensure the target is also a regular file
        let target_meta = fs::metadata(&full_path)
            .map_err(|e| format!("Symlink '{}' points to unreadable target: {}", source_rel, e))?;
        if !target_meta.is_file() {
            return Err(format!(
                "Source symlink '{}' must point to a regular file",
                source_rel
            ));
        }
    } else if !meta.is_file() {
        return Err(format!(
            "Source '{}' is not a regular file (special files, FIFOs, and directories are rejected)",
            source_rel
        ));
    }

    if meta.len() > MAX_PAYLOAD_FILE_SIZE {
        return Err(format!(
            "Source file '{}' size ({} bytes) exceeds maximum limit ({} bytes)",
            source_rel,
            meta.len(),
            MAX_PAYLOAD_FILE_SIZE
        ));
    }

    Ok(canonical_file)
}

/// Validate that a target path is allowed and safe.
/// Must start with "~/", must be within user home, and must be under ~/.config/
/// or other standard user config scope.
pub fn validate_target_safety(target_str: &str, home_dir: &Path) -> Result<PathBuf, String> {
    if !target_str.starts_with("~/") {
        return Err(format!(
            "Target path '{}' must start with '~/'",
            target_str
        ));
    }
    if target_str.contains('\0') {
        return Err(format!("Target path '{}' contains null bytes", target_str));
    }
    for comp in Path::new(target_str).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Target path '{}' contains forbidden parent traversal ('..')",
                target_str
            ));
        }
    }

    let expanded = validate_and_expand_path(target_str, home_dir)?;

    // Bounded configuration scope check:
    // Only allow targets within ~/.config/ or ~/.local/
    let rel_to_home = expanded
        .strip_prefix(home_dir)
        .map_err(|_| format!("Target '{}' is outside user home directory", target_str))?;

    let allowed_prefixes = [".config", ".local/share", ".local/state"];
    let is_allowed = allowed_prefixes
        .iter()
        .any(|p| rel_to_home.starts_with(Path::new(p)));

    // Also allow direct home config files like ~/.config/... or dotfiles
    // if under .config
    if !is_allowed {
        return Err(format!(
            "Target '{}' is outside allowed configuration scope (must be inside ~/.config/ or ~/.local/)",
            target_str
        ));
    }

    Ok(expanded)
}

// ─────────────────────────────────────────────────────────────────────────────
// Planning & Dry Run
// ─────────────────────────────────────────────────────────────────────────────

/// Load and parse manifest from package directory.
pub fn load_package_manifest(package_dir: &Path) -> Result<RyzoraManifest, String> {
    let manifest_path = package_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(format!(
            "manifest.json not found in package directory '{}'",
            package_dir.display()
        ));
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;

    let validation = validate_manifest_internal(&raw);
    if !validation.valid {
        return Err(format!(
            "Manifest validation failed: {}",
            validation.errors.join("; ")
        ));
    }

    let manifest: RyzoraManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse manifest: {}", e))?;

    Ok(manifest)
}

/// Generate an InstallationPlan without modifying any user files.
pub fn generate_installation_plan_in(
    package_dir: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
) -> Result<InstallationPlan, String> {
    let manifest = load_package_manifest(package_dir)?;

    // Compatibility check
    let reqs = CompatibilityRequirements {
        supported_distros: manifest.compatibility.distros.clone(),
        supported_desktops: manifest.compatibility.desktops.clone(),
        supported_sessions: manifest.compatibility.sessions.clone(),
        required_binaries: manifest.compatibility.required.clone(),
        optional_binaries: manifest.compatibility.optional.clone(),
    };
    let compat_report = evaluate_compatibility(system_info, &reqs);

    let (compat_status, missing_deps) = match compat_report.level {
        CompatibilityLevel::Compatible => ("compatible".to_string(), vec![]),
        CompatibilityLevel::MissingDependencies => (
            "missing_dependencies".to_string(),
            compat_report.missing_required_apps.clone(),
        ),
        _ => (
            "incompatible".to_string(),
            compat_report.missing_required_apps.clone(),
        ),
    };

    let mut files_to_create = Vec::new();
    let mut files_to_replace = Vec::new();
    let mut files_unchanged = Vec::new();
    let mut directories_to_create = Vec::new();
    let mut conflicts = Vec::new();
    let mut warnings = Vec::new();

    for file_decl in &manifest.files {
        // Validate source file
        let src_path = match validate_package_source_file(package_dir, &file_decl.source) {
            Ok(p) => p,
            Err(e) => {
                conflicts.push(format!("Source error for '{}': {}", file_decl.source, e));
                continue;
            }
        };

        // Validate target path
        let target_path = match validate_target_safety(&file_decl.target, home_dir) {
            Ok(p) => p,
            Err(e) => {
                conflicts.push(format!("Target error for '{}': {}", file_decl.target, e));
                continue;
            }
        };

        // Check parent directory
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                let parent_str = parent.display().to_string();
                if !directories_to_create.contains(&parent_str) {
                    directories_to_create.push(parent_str);
                }
            }
        }

        // Check if target already exists
        if target_path.exists() || fs::symlink_metadata(&target_path).is_ok() {
            // Check for symlinks
            let sym_meta = fs::symlink_metadata(&target_path).ok();
            if let Some(meta) = sym_meta {
                if meta.file_type().is_symlink() {
                    warnings.push(format!(
                        "Target '{}' is currently a symlink; it will be replaced safely without writing through it",
                        file_decl.target
                    ));
                }
            }

            // Check if contents are identical
            let src_hash = sha256_file(&src_path).unwrap_or_default();
            let dest_hash = sha256_file(&target_path).unwrap_or_default();

            if !src_hash.is_empty() && src_hash == dest_hash {
                files_unchanged.push(file_decl.target.clone());
            } else {
                files_to_replace.push(file_decl.target.clone());
            }
        } else {
            files_to_create.push(file_decl.target.clone());
        }
    }

    Ok(InstallationPlan {
        package_id: manifest.id,
        package_name: manifest.name,
        package_version: manifest.version,
        files_to_create,
        files_to_replace,
        files_unchanged,
        directories_to_create,
        conflicts,
        compatibility_status: compat_status,
        required_dependencies: manifest.compatibility.required,
        missing_dependencies: missing_deps,
        warnings,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Real Staged Installation & Verification
// ─────────────────────────────────────────────────────────────────────────────

/// Perform a real declarative installation.
///
/// Steps:
/// 1. Validate manifest and all payload files.
/// 2. Check compatibility; fail if required dependencies are missing or desktop is incompatible.
/// 3. Create snapshot of ALL target paths via Phase 3 snapshot engine.
/// 4. Verify snapshot.
/// 5. Stage payload files in a temporary Ryzora staging directory.
/// 6. Validate staged files (checksum matches source).
/// 7. Apply staged files to destination target paths.
/// 8. Verify installed files using checksums.
/// 9. If any failure occurs during apply or verification, trigger automatic rollback.
/// 10. Persist metadata to ~/.local/share/ryzora/installed/<pkg_id>.json.
pub fn install_package_in(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
) -> Result<InstallResult, String> {
    let manifest = load_package_manifest(package_dir)?;

    // 1. Pre-flight check & plan
    let plan = generate_installation_plan_in(package_dir, home_dir, system_info)?;

    if !plan.conflicts.is_empty() {
        return Err(format!(
            "Installation blocked due to conflicts: {}",
            plan.conflicts.join("; ")
        ));
    }

    if plan.compatibility_status == "missing_dependencies" {
        return Err(format!(
            "Cannot install package '{}': missing required dependencies: {}",
            manifest.id,
            plan.missing_dependencies.join(", ")
        ));
    }

    if plan.compatibility_status == "incompatible" {
        return Err(format!(
            "Cannot install package '{}': incompatible with detected system environment",
            manifest.id
        ));
    }

    // 2. Collect all declared target paths for snapshot
    let target_paths: Vec<String> = manifest
        .files
        .iter()
        .map(|f| f.target.clone())
        .collect();

    // 3. Create snapshot
    let snapshot_label = format!("pre-install-{}", manifest.id);
    let snapshot_meta = create_snapshot_in(
        &snapshot_label,
        &target_paths,
        home_dir,
        snapshots_root,
    ).map_err(|e| format!("Failed to create pre-install snapshot: {}", e))?;

    // 4. Verify snapshot immediately
    let snapshot_valid = verify_snapshot_in(&snapshot_meta.id, snapshots_root)
        .map_err(|e| format!("Snapshot verification error: {}", e))?;

    if !snapshot_valid {
        return Err("Pre-install snapshot failed integrity verification — installation aborted before modification".to_string());
    }

    // 5. Create staging directory
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let staging_id = format!("{}-{}", manifest.id, now);
    let staging_dir = staging_root.join(&staging_id);
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    // Staging cleanup closure
    let clean_staging = |dir: &Path| {
        let _ = fs::remove_dir_all(dir);
    };

    // 6. Stage payload files and calculate expected checksums
    let mut staged_items = Vec::new(); // (staged_path, target_path, expected_sha256, target_str)

    for (idx, file_decl) in manifest.files.iter().enumerate() {
        let src_path = match validate_package_source_file(package_dir, &file_decl.source) {
            Ok(p) => p,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!("Payload validation failed: {}", e));
            }
        };

        let target_path = match validate_target_safety(&file_decl.target, home_dir) {
            Ok(p) => p,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!("Target validation failed: {}", e));
            }
        };

        let src_hash = match sha256_file(&src_path) {
            Ok(h) => h,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!("Failed to checksum source file '{}': {}", file_decl.source, e));
            }
        };

        let staged_file_path = staging_dir.join(format!("file_{}", idx));
        if let Err(e) = fs::copy(&src_path, &staged_file_path) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to stage file '{}': {}", file_decl.source, e));
        }

        // Validate staged file matches expected hash
        let staged_hash = match sha256_file(&staged_file_path) {
            Ok(h) => h,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!("Failed to verify staged file '{}': {}", file_decl.source, e));
            }
        };

        if staged_hash != src_hash {
            clean_staging(&staging_dir);
            return Err(format!(
                "Staged file checksum mismatch for '{}'",
                file_decl.source
            ));
        }

        staged_items.push((staged_file_path, target_path, src_hash, file_decl.target.clone()));
    }

    // 7. Apply staged files
    let mut applied_targets = Vec::new();
    let mut apply_error: Option<String> = None;

    for (staged_path, target_path, expected_hash, target_str) in &staged_items {
        // Ensure parent dir exists
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                apply_error = Some(format!("Failed to create parent directory for '{}': {}", target_str, e));
                break;
            }
        }

        // If target is an existing symlink, remove the symlink itself so we don't write through it
        if let Ok(meta) = fs::symlink_metadata(target_path) {
            if meta.file_type().is_symlink() {
                if let Err(e) = fs::remove_file(target_path) {
                    apply_error = Some(format!("Failed to remove target symlink '{}': {}", target_str, e));
                    break;
                }
            }
        }

        // Copy staged file to destination
        if let Err(e) = fs::copy(staged_path, target_path) {
            apply_error = Some(format!("Failed to copy staged file to '{}': {}", target_str, e));
            break;
        }

        // 8. Verify installed file immediately
        let installed_hash = match sha256_file(target_path) {
            Ok(h) => h,
            Err(e) => {
                apply_error = Some(format!("Failed to verify installed file '{}': {}", target_str, e));
                break;
            }
        };

        if &installed_hash != expected_hash {
            apply_error = Some(format!(
                "Installed file verification failed for '{}': checksum mismatch",
                target_str
            ));
            break;
        }

        applied_targets.push(target_str.clone());
    }

    // 9. Handle failure and automatic rollback
    if let Some(err) = apply_error {
        clean_staging(&staging_dir);

        // Perform rollback using snapshot
        let rollback_res = restore_snapshot_in(&snapshot_meta.id, home_dir, snapshots_root);
        let rolled_back_ok = rollback_res.map(|r| r.success).unwrap_or(false);

        return Ok(InstallResult {
            success: false,
            package_id: manifest.id,
            version: manifest.version,
            snapshot_id: snapshot_meta.id,
            installed_files: vec![],
            errors: vec![format!(
                "Installation failed ({}); automatic rollback was {}",
                err,
                if rolled_back_ok { "successful" } else { "attempted with warnings" }
            )],
            rolled_back: true,
        });
    }

    // Clean up staging directory on success
    clean_staging(&staging_dir);

    // 10. Persist authoritative installation record
    if let Err(e) = fs::create_dir_all(installed_root) {
        return Err(format!("Failed to create installed packages directory: {}", e));
    }

    let record = InstalledPackageRecord {
        package_id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        installed_at: now,
        snapshot_id: snapshot_meta.id.clone(),
        installed_files: applied_targets.clone(),
        package_source_path: package_dir.display().to_string(),
    };

    let record_json = serde_json::to_string_pretty(&record)
        .map_err(|e| format!("Failed to serialize install record: {}", e))?;
    let record_file = installed_root.join(format!("{}.json", manifest.id));
    if let Err(e) = fs::write(&record_file, record_json) {
        return Err(format!("Failed to save installation record: {}", e));
    }

    Ok(InstallResult {
        success: true,
        package_id: manifest.id,
        version: manifest.version,
        snapshot_id: snapshot_meta.id,
        installed_files: applied_targets,
        errors: vec![],
        rolled_back: false,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Installed Packages Querying
// ─────────────────────────────────────────────────────────────────────────────

/// Discover all installed packages from authoritative disk metadata.
pub fn list_installed_packages_in(
    installed_root: &Path,
) -> Result<Vec<InstalledPackageRecord>, String> {
    if !installed_root.exists() {
        return Ok(vec![]);
    }

    let mut records = Vec::new();
    let entries = fs::read_dir(installed_root)
        .map_err(|e| format!("Failed to read installed directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            if let Ok(raw) = fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<InstalledPackageRecord>(&raw) {
                    records.push(record);
                }
            }
        }
    }

    // Sort newest installed first
    records.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
    Ok(records)
}

/// Retrieve a specific installed package record.
pub fn get_installed_package_in(
    package_id: &str,
    installed_root: &Path,
) -> Result<Option<InstalledPackageRecord>, String> {
    let path = installed_root.join(format!("{}.json", package_id));
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;
    Ok(Some(record))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Exposed Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn preview_installation(package_id: String) -> Result<InstallationPlan, String> {
    let home = get_home_dir();
    let package_dir = find_package_dir(&package_id, None)?;
    let sys = detect_system_info();
    generate_installation_plan_in(&package_dir, &home, &sys)
}

#[tauri::command]
pub fn install_package(package_id: String) -> Result<InstallResult, String> {
    let home = get_home_dir();
    let package_dir = find_package_dir(&package_id, None)?;
    let snapshots_root = get_ryzora_snapshots_dir();
    let installed_root = get_ryzora_installed_dir();
    let staging_root = get_ryzora_staging_dir();
    let sys = detect_system_info();

    install_package_in(
        &package_dir,
        &snapshots_root,
        &installed_root,
        &staging_root,
        &home,
        &sys,
    )
}

#[tauri::command]
pub fn list_installed_packages() -> Result<Vec<InstalledPackageRecord>, String> {
    let installed_root = get_ryzora_installed_dir();
    list_installed_packages_in(&installed_root)
}

#[tauri::command]
pub fn get_installed_package(package_id: String) -> Result<Option<InstalledPackageRecord>, String> {
    let installed_root = get_ryzora_installed_dir();
    get_installed_package_in(&package_id, &installed_root)
}

// ─────────────────────────────────────────────────────────────────────────────
// Comprehensive Unit & Integration Tests
// Zero tests touch ~/.config or real user files. Everything uses /tmp sandbox.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TestSandbox {
        root: PathBuf,
        packages_dir: PathBuf,
        snapshots_dir: PathBuf,
        installed_dir: PathBuf,
        staging_dir: PathBuf,
        home_dir: PathBuf,
    }

    impl TestSandbox {
        fn new(name: &str) -> Self {
            let unique_id = format!(
                "ryzora-installer-test-{}-{}",
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique_id);
            let packages_dir = root.join("packages");
            let snapshots_dir = root.join("snapshots");
            let installed_dir = root.join("installed");
            let staging_dir = root.join("staging");
            let home_dir = root.join("mock_home");

            fs::create_dir_all(&packages_dir).unwrap();
            fs::create_dir_all(&snapshots_dir).unwrap();
            fs::create_dir_all(&installed_dir).unwrap();
            fs::create_dir_all(&staging_dir).unwrap();
            fs::create_dir_all(&home_dir.join(".config")).unwrap();

            Self {
                root,
                packages_dir,
                snapshots_dir,
                installed_dir,
                staging_dir,
                home_dir,
            }
        }

        fn mock_system() -> SystemInfo {
            SystemInfo {
                distro_name: "Arch Linux".to_string(),
                distro_id: "arch".to_string(),
                distro_family: "arch".to_string(),
                distro_version: "Rolling".to_string(),
                kernel_version: "6.12.0".to_string(),
                desktop_environment: "hyprland".to_string(),
                window_manager: "hyprland".to_string(),
                session_type: "wayland".to_string(),
                shell: "zsh".to_string(),
                terminal: "kitty".to_string(),
                installed_components: vec![
                    crate::system::InstalledComponent {
                        name: "Hyprland".to_string(),
                        binary: "hyprland".to_string(),
                        installed: true,
                        path: Some("/usr/bin/hyprland".to_string()),
                        category: "WM".to_string(),
                    },
                    crate::system::InstalledComponent {
                        name: "Waybar".to_string(),
                        binary: "waybar".to_string(),
                        installed: true,
                        path: Some("/usr/bin/waybar".to_string()),
                        category: "Bar".to_string(),
                    },
                    crate::system::InstalledComponent {
                        name: "Kitty".to_string(),
                        binary: "kitty".to_string(),
                        installed: true,
                        path: Some("/usr/bin/kitty".to_string()),
                        category: "Terminal".to_string(),
                    },
                ],
            }
        }

        fn create_sample_package(
            &self,
            id: &str,
            files: &[(&str, &str, &str)], // (source, target, content)
            required_binaries: &[&str],
        ) -> PathBuf {
            let pkg_dir = self.packages_dir.join(id);
            fs::create_dir_all(&pkg_dir).unwrap();

            let manifest_files_json: Vec<String> = files
                .iter()
                .map(|(s, t, _)| {
                    format!(
                        r#"{{"source": "{}", "target": "{}", "description": "test file"}}"#,
                        s, t
                    )
                })
                .collect();

            let req_json: Vec<String> = required_binaries
                .iter()
                .map(|b| format!(r#""{}""#, b))
                .collect();

            let manifest_json = format!(
                r##"{{
  "id": "{}",
  "name": "Test Package {}",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test package",
  "tags": ["test"],
  "color_palette": ["#000000"],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch"],
    "required": [{}],
    "optional": []
  }},
  "files": [{}]
}}"##,
                id,
                id,
                req_json.join(", "),
                manifest_files_json.join(", ")
            );

            fs::write(pkg_dir.join("manifest.json"), manifest_json).unwrap();

            for (src_rel, _, content) in files {
                let full_src = pkg_dir.join(src_rel);
                if let Some(parent) = full_src.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(full_src, content).unwrap();
            }

            pkg_dir
        }
    }

    impl Drop for TestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn test_plan_generation_new_and_replace() {
        let sandbox = TestSandbox::new("plan-gen");
        let sys = TestSandbox::mock_system();

        // Create an existing file in mock home
        let existing_dest = sandbox.home_dir.join(".config/waybar/style.css");
        fs::create_dir_all(existing_dest.parent().unwrap()).unwrap();
        fs::write(&existing_dest, "old waybar css").unwrap();

        let pkg_dir = sandbox.create_sample_package(
            "test-pkg-1",
            &[
                ("files/hypr/hyprland.conf", "~/.config/hypr/hyprland.conf", "new hypr config"),
                ("files/waybar/style.css", "~/.config/waybar/style.css", "new waybar css"),
            ],
            &["hyprland", "waybar"],
        );

        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();

        assert_eq!(plan.package_id, "test-pkg-1");
        assert_eq!(plan.compatibility_status, "compatible");
        assert_eq!(plan.files_to_create, vec!["~/.config/hypr/hyprland.conf"]);
        assert_eq!(plan.files_to_replace, vec!["~/.config/waybar/style.css"]);
        assert_eq!(plan.files_unchanged.len(), 0);
        assert_eq!(plan.conflicts.len(), 0);
    }

    #[test]
    fn test_dry_run_leaves_filesystem_untouched() {
        let sandbox = TestSandbox::new("dry-run");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "dry-run-pkg",
            &[("files/app.conf", "~/.config/test/app.conf", "payload content")],
            &["hyprland"],
        );

        let target_path = sandbox.home_dir.join(".config/test/app.conf");
        assert!(!target_path.exists());

        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();
        assert_eq!(plan.files_to_create.len(), 1);

        // Ensure nothing was written to mock home or snapshot dirs
        assert!(!target_path.exists());
        assert_eq!(fs::read_dir(&sandbox.snapshots_dir).unwrap().count(), 0);
    }

    #[test]
    fn test_successful_installation_flow() {
        let sandbox = TestSandbox::new("success-install");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "real-install-pkg",
            &[
                ("files/hypr/hyprland.conf", "~/.config/hypr/hyprland.conf", "hyprland config 1"),
                ("files/waybar/config.jsonc", "~/.config/waybar/config.jsonc", "waybar config 1"),
            ],
            &["hyprland", "waybar"],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();

        assert!(res.success);
        assert!(!res.rolled_back);
        assert_eq!(res.installed_files.len(), 2);

        // Verify files exist in mock home with exact content
        let hypr_dest = sandbox.home_dir.join(".config/hypr/hyprland.conf");
        let waybar_dest = sandbox.home_dir.join(".config/waybar/config.jsonc");
        assert!(hypr_dest.is_file());
        assert!(waybar_dest.is_file());
        assert_eq!(fs::read_to_string(hypr_dest).unwrap(), "hyprland config 1");
        assert_eq!(fs::read_to_string(waybar_dest).unwrap(), "waybar config 1");

        // Verify snapshot was created and verified
        let snap_valid = verify_snapshot_in(&res.snapshot_id, &sandbox.snapshots_dir).unwrap();
        assert!(snap_valid);

        // Verify metadata was persisted
        let installed = list_installed_packages_in(&sandbox.installed_dir).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].package_id, "real-install-pkg");
        assert_eq!(installed[0].installed_files.len(), 2);
    }

    #[test]
    fn test_snapshot_created_and_verified_before_install() {
        let sandbox = TestSandbox::new("snapshot-before");
        let sys = TestSandbox::mock_system();

        // Target already exists with original content
        let dest = sandbox.home_dir.join(".config/existing.conf");
        fs::write(&dest, "original user content").unwrap();

        let pkg_dir = sandbox.create_sample_package(
            "replace-pkg",
            &[("files/existing.conf", "~/.config/existing.conf", "new package content")],
            &["hyprland"],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();

        assert!(res.success);

        // Dest has new content
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new package content");

        // Snapshot contains original content
        let snap = crate::snapshot::get_snapshot_in(&res.snapshot_id, &sandbox.snapshots_dir).unwrap();
        assert_eq!(snap.entries.len(), 1);
        let entry = &snap.entries[0];
        let backup_file = sandbox.snapshots_dir.join(&snap.id).join("files").join(&entry.backup_relative);
        assert_eq!(fs::read_to_string(backup_file).unwrap(), "original user content");
    }

    #[test]
    fn test_source_traversal_rejection() {
        let sandbox = TestSandbox::new("source-traversal");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "traversal-pkg",
            &[("../escape.txt", "~/.config/escape.txt", "evil")],
            &["hyprland"],
        );

        let res = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("parent traversal") || err.contains("escapes the package root"));
    }

    #[test]
    fn test_target_traversal_rejection() {
        let sandbox = TestSandbox::new("target-traversal");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "target-traversal-pkg",
            &[("files/test.conf", "~/.config/../../etc/bad.conf", "evil")],
            &["hyprland"],
        );

        let res = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("parent traversal") || err.contains("path traversal"));
    }

    #[test]
    fn test_absolute_target_rejection() {
        let sandbox = TestSandbox::new("abs-target");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "abs-target-pkg",
            &[("files/test.conf", "/etc/bad.conf", "evil")],
            &["hyprland"],
        );

        let res = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("must start with '~/'"));
    }

    #[test]
    fn test_missing_payload_file_rejection() {
        let sandbox = TestSandbox::new("missing-payload");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "missing-file-pkg",
            &[("files/exists.conf", "~/.config/exists.conf", "present")],
            &["hyprland"],
        );

        // Delete the file from payload
        fs::remove_file(pkg_dir.join("files/exists.conf")).unwrap();

        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();
        assert!(plan.conflicts.iter().any(|c| c.contains("missing")));
    }

    #[test]
    fn test_missing_required_dependency_blocks_install() {
        let sandbox = TestSandbox::new("missing-dep");
        let sys = TestSandbox::mock_system(); // has hyprland, waybar, kitty

        let pkg_dir = sandbox.create_sample_package(
            "missing-dep-pkg",
            &[("files/test.conf", "~/.config/test.conf", "test")],
            &["hyprland", "nonexistent_binary_xyz"],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        );

        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("missing required dependencies"));
    }

    #[test]
    fn test_incompatible_desktop_blocks_install() {
        let sandbox = TestSandbox::new("incompat-desktop");
        let mut sys = TestSandbox::mock_system();
        sys.desktop_environment = "gnome".to_string();
        sys.window_manager = "gnome-shell".to_string();

        let pkg_dir = sandbox.create_sample_package(
            "gnome-pkg",
            &[("files/test.conf", "~/.config/test.conf", "test")],
            &["hyprland"],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        );

        assert!(res.is_err());
        assert!(res.err().unwrap().contains("incompatible with detected system environment"));
    }

    #[test]
    fn test_install_failure_triggers_automatic_rollback() {
        let sandbox = TestSandbox::new("rollback-on-failure");
        let sys = TestSandbox::mock_system();

        // Existing file with initial content
        let dest = sandbox.home_dir.join(".config/original.conf");
        fs::write(&dest, "untouched original configuration").unwrap();

        let _pkg_dir = sandbox.create_sample_package(
            "fail-pkg",
            &[
                ("files/original.conf", "~/.config/original.conf", "new replaced configuration"),
            ],
            &["hyprland"],
        );

        // Corrupt the package file AFTER staging planning by making dest read-only directory
        // Or create an unwritable parent directory for a second target
        let unwriteable_pkg_dir = sandbox.create_sample_package(
            "fail-multi-pkg",
            &[
                ("files/original.conf", "~/.config/original.conf", "new replaced configuration"),
                ("files/impossible.conf", "~/.config/impossible_dir/sub/file.conf", "will fail"),
            ],
            &["hyprland"],
        );

        // Pre-create ~/.config/impossible_dir as a read-only file so directory creation fails!
        let blocking_file = sandbox.home_dir.join(".config/impossible_dir");
        fs::write(&blocking_file, "blocking normal directory creation").unwrap();

        let res = install_package_in(
            &unwriteable_pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();

        // Installation failed, but handled safely
        assert!(!res.success);
        assert!(res.rolled_back);

        // Original file MUST be preserved with original content
        assert_eq!(
            fs::read_to_string(&dest).unwrap(),
            "untouched original configuration"
        );
    }

    #[test]
    fn test_installed_packages_persistence_and_discovery() {
        let sandbox = TestSandbox::new("persistence");
        let sys = TestSandbox::mock_system();

        let pkg1 = sandbox.create_sample_package(
            "app-one",
            &[("files/one.conf", "~/.config/one.conf", "1")],
            &["hyprland"],
        );
        let pkg2 = sandbox.create_sample_package(
            "app-two",
            &[("files/two.conf", "~/.config/two.conf", "2")],
            &["hyprland"],
        );

        install_package_in(
            &pkg1,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();

        install_package_in(
            &pkg2,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();

        let list = list_installed_packages_in(&sandbox.installed_dir).unwrap();
        assert_eq!(list.len(), 2);
        let ids: Vec<String> = list.iter().map(|p| p.package_id.clone()).collect();
        assert!(ids.contains(&"app-one".to_string()));
        assert!(ids.contains(&"app-two".to_string()));

        let fetched = get_installed_package_in("app-one", &sandbox.installed_dir).unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Test Package app-one");
    }

    #[test]
    fn test_symlink_escape_rejection() {
        let sandbox = TestSandbox::new("symlink-escape");

        // Create a file outside the package root
        let outside_file = sandbox.root.join("secret.txt");
        fs::write(&outside_file, "secret outside package").unwrap();

        let pkg_dir = sandbox.packages_dir.join("symlink-escape-pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        // Create a symlink inside package pointing outside
        let evil_link = pkg_dir.join("files/evil.conf");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside_file, &evil_link).unwrap();

        let res = validate_package_source_file(&pkg_dir, "files/evil.conf");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("escapes package root via symlink"));
    }

    #[test]
    fn test_unsupported_special_file_rejection() {
        let sandbox = TestSandbox::new("special-file");
        let pkg_dir = sandbox.packages_dir.join("dir-as-file-pkg");
        let sub_dir = pkg_dir.join("files/not_a_file");
        fs::create_dir_all(&sub_dir).unwrap();

        // Pass directory path as source
        let res = validate_package_source_file(&pkg_dir, "files/not_a_file");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("not a regular file"));
    }

    #[test]
    fn test_corrupted_package_rejection() {
        let sandbox = TestSandbox::new("corrupted-pkg");
        let sys = TestSandbox::mock_system();
        let pkg_dir = sandbox.packages_dir.join("corrupted-pkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), "{ invalid json: true ").unwrap();

        let res = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Invalid JSON"));
    }

    #[test]
    fn test_repeated_installation_behavior() {
        let sandbox = TestSandbox::new("repeated-install");
        let sys = TestSandbox::mock_system();

        let pkg = sandbox.create_sample_package(
            "repeat-pkg",
            &[("files/app.conf", "~/.config/repeat/app.conf", "v1 content")],
            &["hyprland"],
        );

        // First install
        let res1 = install_package_in(
            &pkg,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();
        assert!(res1.success);

        // Check plan now reports it as unchanged
        let plan_same = generate_installation_plan_in(&pkg, &sandbox.home_dir, &sys).unwrap();
        assert_eq!(plan_same.files_unchanged, vec!["~/.config/repeat/app.conf"]);
        assert_eq!(plan_same.files_to_create.len(), 0);
        assert_eq!(plan_same.files_to_replace.len(), 0);

        // Update package to v2
        fs::write(pkg.join("files/app.conf"), "v2 content").unwrap();

        // Check plan now reports it as to_replace
        let plan_diff = generate_installation_plan_in(&pkg, &sandbox.home_dir, &sys).unwrap();
        assert_eq!(plan_diff.files_to_replace, vec!["~/.config/repeat/app.conf"]);

        // Second install
        let res2 = install_package_in(
            &pkg,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        ).unwrap();
        assert!(res2.success);

        let dest = sandbox.home_dir.join(".config/repeat/app.conf");
        assert_eq!(fs::read_to_string(&dest).unwrap(), "v2 content");
    }

    #[test]
    fn test_installer_finds_package_in_repository() {
        // Verify that find_package_dir locates packages configured in the repository
        let found = find_package_dir("rice-cyberpunk-neon", None);
        assert!(found.is_ok());
        let pkg_dir = found.unwrap();
        assert!(pkg_dir.join("manifest.json").is_file());
    }
}
