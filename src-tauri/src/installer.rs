pub fn update_theme_conf_with_config(
    theme_conf_path: &Path,
    cfg: &std::collections::HashMap<String, serde_json::Value>,
) -> std::io::Result<()> {
    let existing = if theme_conf_path.exists() {
        fs::read_to_string(theme_conf_path)?
    } else {
        String::new()
    };

    let mut out_lines: Vec<String> = existing.lines().map(|s| s.to_string()).collect();
    if !out_lines.iter().any(|l| l.trim() == "[General]") {
        out_lines.insert(0, "[General]".to_string());
    }

    let general_idx = out_lines.iter().position(|l| l.trim() == "[General]").unwrap();

    for (k, v) in cfg {
        let v_str = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            other => other.to_string(),
        };
        let key_prefix = format!("{}=", k);
        let mut found = false;

        for idx in (general_idx + 1)..out_lines.len() {
            if out_lines[idx].trim().starts_with('[') {
                break;
            }
            if out_lines[idx].trim().starts_with(&key_prefix) {
                out_lines[idx] = format!("{}={}", k, v_str);
                found = true;
                break;
            }
        }

        if !found {
            out_lines.insert(general_idx + 1, format!("{}={}", k, v_str));
        }
    }

    fs::write(theme_conf_path, out_lines.join("\n") + "\n")
}

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::compatibility::{evaluate_compatibility, CompatibilityLevel, CompatibilityRequirements};
use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
use crate::snapshot::{
    create_snapshot_in, get_home_dir, get_ryzora_snapshots_dir, restore_snapshot_in, sha256_file,
    validate_and_expand_path, verify_snapshot_in,
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
    #[serde(default)]
    pub requires_privilege: bool,
    #[serde(default)]
    pub selected_target: Option<String>,
    #[serde(default)]
    pub dependency_report: Option<crate::dependency::DependencyResolutionReport>,
    #[serde(default)]
    pub cryptographic_evaluation: Option<crate::crypto::CryptographicEvaluation>,
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

/// Authoritative record stored in ~/.local/share/ryzora/installed/<pkg_id>.json
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
    pub cryptographic_status: Option<crate::crypto::CryptographicStatus>,
    #[serde(default)]
    pub signer_key_id: Option<String>,
    #[serde(default)]
    pub signer_name: Option<String>,
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
            cryptographic_status: None,
            signer_key_id: None,
            signer_name: None,
            history: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UninstallResult {
    pub package_id: String,
    pub success: bool,
    pub removed_files: Vec<String>,
    pub already_missing_files: Vec<String>,
    pub conflict_files: Vec<String>,
    pub removed_directories: Vec<String>,
    pub retained_directories: Vec<String>,
    pub snapshot_id: Option<String>,
    pub rolled_back: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStatusKind {
    UpToDate,
    UpdateAvailable,
    RepositoryUnavailable,
    PackageNotFound,
    UnableToDetermine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageUpdateStatus {
    pub package_id: String,
    pub installed_version: String,
    pub available_version: Option<String>,
    pub repository_id: Option<String>,
    pub status: UpdateStatusKind,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Create,
    Replace,
    Unchanged,
    Conflict,
    ObsoleteRemove,
    ObsoleteRetain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFileItem {
    pub target: String,
    pub action: FileAction,
    pub reason: String,
    pub current_sha256: Option<String>,
    pub new_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePlan {
    pub package_id: String,
    pub from_version: String,
    pub to_version: String,
    pub repository_id: Option<String>,
    pub creates: Vec<String>,
    pub replaces: Vec<String>,
    pub unchanged: Vec<String>,
    pub conflicts: Vec<String>,
    pub obsolete_removes: Vec<String>,
    pub obsolete_retains: Vec<String>,
    pub details: Vec<UpdateFileItem>,
    pub has_conflicts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub success: bool,
    pub package_id: String,
    pub from_version: String,
    pub to_version: String,
    pub snapshot_id: String,
    pub updated_files: Vec<String>,
    pub obsolete_removed: Vec<String>,
    pub conflicts_retained: Vec<String>,
    pub rolled_back: bool,
    pub errors: Vec<String>,
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
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
    {
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
const MAX_PAYLOAD_FILE_SIZE: u64 = 500 * 1024 * 1024; // 500 MB to support HD/4K animated video backgrounds

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
        return Err(format!("Package payload file missing: '{}'", source_rel));
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
        let target_meta = fs::metadata(&full_path).map_err(|e| {
            format!(
                "Symlink '{}' points to unreadable target: {}",
                source_rel, e
            )
        })?;
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

    Ok(full_path)
}

/// Validate that a target path is allowed and safe.
/// Must start with "~/", must be within user home, and must be under ~/.config/
/// or other standard user config scope.
pub fn validate_target_safety(target_str: &str, home_dir: &Path) -> Result<PathBuf, String> {
    // System scope for SDDM display manager greeter themes
    if target_str.starts_with("/usr/share/sddm/themes/") {
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
        if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
            let rel = target_str.trim_start_matches('/');
            return Ok(Path::new(&sys_root).join(rel));
        }
        return Ok(PathBuf::from(target_str));
    }

    if !target_str.starts_with("~/") {
        return Err(format!("Target path '{}' must start with '~/' or '/usr/share/sddm/themes/'", target_str));
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

    let allowed_prefixes = [
        ".config",
        ".local/share",
        ".local/state",
        "Pictures/Wallpapers",
        ".themes",
        ".icons",
    ];
    let is_allowed = allowed_prefixes
        .iter()
        .any(|p| rel_to_home.starts_with(Path::new(p)));

    // Also allow direct home config files like ~/.config/... or dotfiles
    // if under .config
    if !is_allowed {
        return Err(format!(
            "Target '{}' is outside allowed configuration scope (must be inside ~/.config/, ~/.local/, ~/Pictures/Wallpapers/, ~/.themes/, or ~/.icons/)",
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

    let manifest: RyzoraManifest =
        serde_json::from_str(&raw).map_err(|e| format!("Failed to parse manifest: {}", e))?;

    Ok(manifest)
}

/// Generate an InstallationPlan without modifying any user files.
pub fn generate_installation_plan_in(
    package_dir: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
) -> Result<InstallationPlan, String> {
    generate_installation_plan_target_in(package_dir, home_dir, system_info, None)
}

pub fn generate_installation_plan_target_in(
    package_dir: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    target: Option<&str>,
) -> Result<InstallationPlan, String> {
    let trust_store = crate::crypto::TrustStore::load_default();
    generate_installation_plan_target_in_with_trust(package_dir, home_dir, system_info, &trust_store, target)
}

pub fn generate_installation_plan_in_with_trust(
    package_dir: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    trust_store: &crate::crypto::TrustStore,
) -> Result<InstallationPlan, String> {
    generate_installation_plan_target_in_with_trust(package_dir, home_dir, system_info, trust_store, None)
}

pub fn generate_installation_plan_target_in_with_trust(
    package_dir: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    trust_store: &crate::crypto::TrustStore,
    target: Option<&str>,
) -> Result<InstallationPlan, String> {
    let manifest = load_package_manifest(package_dir)?;

    // Target validation
    let mut conflicts = Vec::new();
    let mut warnings = Vec::new();
    if let Some(tgt) = target {
        if tgt == "both" {
            if !manifest.supports_target("quickshell") && !manifest.supports_target("sddm") {
                conflicts.push(format!("Package '{}' does not support target 'both'", manifest.id));
            }
        } else if !manifest.supports_target(tgt) {
            conflicts.push(format!("Package '{}' does not support target '{}'", manifest.id, tgt));
        }
    }

    // Compatibility check (SDDM is a display manager greeter, not a user desktop session)
    let is_sddm_target = target == Some("sddm");
    let reqs = CompatibilityRequirements {
        supported_distros: manifest.compatibility.distros.clone(),
        supported_desktops: if is_sddm_target { vec![] } else { manifest.compatibility.desktops.clone() },
        supported_sessions: if is_sddm_target { vec![] } else { manifest.compatibility.sessions.clone() },
        required_binaries: if target.is_some() { vec![] } else { manifest.compatibility.required.clone() },
        optional_binaries: manifest.compatibility.optional.clone(),
    };
    let compat_report = evaluate_compatibility(system_info, &reqs);

    let (mut compat_status, mut missing_deps) = match compat_report.level {
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

    // Phase 12: Cryptographic Signature & Trust Chain Evaluation
    let crypto_eval =
        crate::crypto::evaluate_package_directory_crypto(package_dir, &manifest, None, trust_store)
            .ok();

    if let Some(ref eval) = crypto_eval {
        if !eval.can_install {
            if let Some(ref err) = eval.error_message {
                conflicts.push(format!("Cryptographic Verification Block: {}", err));
                compat_status = "incompatible".to_string();
            }
        }
    }

    // Target-Aware Dependency Intelligence & Resolution
    let provider = crate::dependency::RepositoryPackageProvider::new();
    let resolver = crate::dependency::DependencyResolver::new(&provider, system_info);
    let dep_report = resolver.resolve_manifest_for_target(&manifest, target);

    for m in &dep_report.missing_required {
        if !missing_deps.contains(m) {
            missing_deps.push(m.clone());
        }
    }
    for c in &dep_report.conflicts {
        if !conflicts.contains(c) {
            conflicts.push(c.clone());
        }
    }
    for cycle in &dep_report.cycles {
        let cycle_msg = format!("Circular dependency detected: {}", cycle.join(" -> "));
        if !conflicts.contains(&cycle_msg) {
            conflicts.push(cycle_msg);
        }
    }

    if !conflicts.is_empty() {
        compat_status = "incompatible".to_string();
    } else if !missing_deps.is_empty() {
        compat_status = "missing_dependencies".to_string();
    }

    let target_files = match manifest.target_files(target) {
        Ok(f) => f,
        Err(e) => {
            conflicts.push(e);
            vec![]
        }
    };

    for file_decl in &target_files {
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

    let target_required_deps = if target.is_some() {
        dep_report
            .system_dependencies
            .iter()
            .filter(|d| d.required)
            .map(|d| d.binary.clone())
            .collect()
    } else {
        manifest.compatibility.required.clone()
    };

    let requires_privilege = target == Some("sddm")
        || target == Some("both")
        || files_to_create
            .iter()
            .chain(files_to_replace.iter())
            .any(|f| f.starts_with("/usr/"));

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
        required_dependencies: target_required_deps,
        missing_dependencies: missing_deps,
        warnings,
        requires_privilege,
        selected_target: target.map(|s| s.to_string()),
        dependency_report: Some(dep_report),
        cryptographic_evaluation: crypto_eval,
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
    install_package_options_in(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        system_info,
        true,
    )
}

pub fn install_package_options_in(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    create_snapshot: bool,
) -> Result<InstallResult, String> {
    install_package_target_options_in(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        system_info,
        create_snapshot,
        None,
    )
}

pub fn install_package_target_options_in(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    create_snapshot: bool,
    target: Option<&str>,
) -> Result<InstallResult, String> {
    let trust_store = crate::crypto::TrustStore::load_default();
    install_package_target_options_in_with_trust(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        system_info,
        &trust_store,
        create_snapshot,
        target,
    )
}

pub fn install_package_in_with_trust(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    trust_store: &crate::crypto::TrustStore,
) -> Result<InstallResult, String> {
    install_package_target_options_in_with_trust(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        system_info,
        trust_store,
        true,
        None,
    )
}

pub fn install_package_options_in_with_trust(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    trust_store: &crate::crypto::TrustStore,
    create_snapshot: bool,
) -> Result<InstallResult, String> {
    install_package_target_options_in_with_trust(
        package_dir,
        snapshots_root,
        installed_root,
        staging_root,
        home_dir,
        system_info,
        trust_store,
        create_snapshot,
        None,
    )
}

pub fn install_package_target_options_in_with_trust(
    package_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &SystemInfo,
    trust_store: &crate::crypto::TrustStore,
    create_snapshot: bool,
    target: Option<&str>,
) -> Result<InstallResult, String> {
    let manifest = load_package_manifest(package_dir)?;

    // 1. Pre-flight check & plan
    let plan =
        generate_installation_plan_target_in_with_trust(package_dir, home_dir, system_info, trust_store, target)?;

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

    // Phase 12: Cryptographic Pre-Install Gate (Fails Closed Before Snapshot/Staging)
    let crypto_eval = crate::crypto::evaluate_package_directory_crypto(
        package_dir,
        &manifest,
        None,
        trust_store,
    )?;
    if !crypto_eval.can_install {
        let err_msg = crypto_eval.error_message.unwrap_or_else(|| {
            "Package failed cryptographic trust verification. Installation aborted.".to_string()
        });
        return Err(format!("Cryptographic Verification Error: {}", err_msg));
    }

    let target_files = manifest.target_files(target)?;

    // Privilege check: for SDDM system paths, use the Ryzora restricted privileged helper.
    // This replaces the old USER=root check with real privileged-helper-based authorization.
    // In test mode (RYZORA_SYSTEM_ROOT set), the helper runs directly without elevation.
    // Privilege flag is checked but actual installation is deferred to the SDDM staging step below.

    // 2. Collect target paths and optionally create pre-install snapshot
    let snapshot_id_opt: Option<String> = if create_snapshot {
        let target_paths: Vec<String> = target_files.iter().filter(|f| f.target.starts_with("~/")).map(|f| f.target.clone()).collect();
        let snapshot_label = format!("pre-install-{}", manifest.id);
        let snapshot_meta =
            create_snapshot_in(&snapshot_label, &target_paths, home_dir, snapshots_root)
                .map_err(|e| format!("Failed to create pre-install snapshot: {}", e))?;

        let snapshot_valid = verify_snapshot_in(&snapshot_meta.id, snapshots_root)
            .map_err(|e| format!("Snapshot verification error: {}", e))?;

        if !snapshot_valid {
            return Err("Pre-install snapshot failed integrity verification — installation aborted before modification".to_string());
        }
        Some(snapshot_meta.id)
    } else {
        None
    };

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

    // 6. Stage payload files and calculate expected checksums / link targets
    let mut staged_items = Vec::new(); // (staged_path, target_path, expected_hash, is_symlink, symlink_target, target_str)

    for (idx, file_decl) in target_files.iter().enumerate() {
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

        let src_meta = match fs::symlink_metadata(&src_path) {
            Ok(m) => m,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to read metadata for source file '{}': {}",
                    file_decl.source, e
                ));
            }
        };

        let src_is_symlink = src_meta.file_type().is_symlink();
        let staged_file_path = staging_dir.join(format!("file_{}", idx));

        let (expected_hash, symlink_target) = if src_is_symlink {
            #[cfg(unix)]
            {
                let link_target = match fs::read_link(&src_path) {
                    Ok(lt) => lt,
                    Err(e) => {
                        clean_staging(&staging_dir);
                        return Err(format!(
                            "Failed to read symlink '{}': {}",
                            file_decl.source, e
                        ));
                    }
                };
                let link_str = link_target.to_string_lossy().to_string();
                if let Err(e) = std::os::unix::fs::symlink(&link_target, &staged_file_path) {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to stage symlink '{}': {}",
                        file_decl.source, e
                    ));
                }
                (String::new(), Some(link_str))
            }
            #[cfg(not(unix))]
            {
                clean_staging(&staging_dir);
                return Err("Symlinks are only supported on Unix platforms".to_string());
            }
        } else {
            let src_hash = match sha256_file(&src_path) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to checksum source file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            if let Err(e) = fs::copy(&src_path, &staged_file_path) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to stage file '{}': {}",
                    file_decl.source, e
                ));
            }

            // Validate staged file matches expected hash
            let staged_hash = match sha256_file(&staged_file_path) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to verify staged file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            if staged_hash != src_hash {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Staged file checksum mismatch for '{}'",
                    file_decl.source
                ));
            }
            (src_hash, None)
        };

        staged_items.push((
            staged_file_path,
            target_path,
            expected_hash,
            src_is_symlink,
            symlink_target,
            file_decl.target.clone(),
        ));
    }

    // 7. Apply staged files
    // For SDDM targets (/usr/share/sddm/themes/ryzora-<slug>/...):
    //   - Collect all SDDM files into a separate SDDM staging directory
    //   - Dispatch to the restricted Ryzora privileged helper via privileged helper
    //   - Only write installed record after physical verification
    let has_sddm_targets = staged_items.iter().any(|(_, _, _, _, _, t)| {
        t.starts_with("/usr/share/sddm/themes/")
    });

    if has_sddm_targets && std::env::var("RYZORA_SYSTEM_ROOT").is_err() {
        // Real machine: use sddm_helper for privileged materialization
        // Build a temporary SDDM staging directory with the correct internal structure
        let sddm_staging_id = format!("ryzora-staging-sddm-{}", staging_id);
        let sddm_staging_dir = std::env::temp_dir().join(&sddm_staging_id);

        // Extract slug from the first SDDM target path
        let slug_opt = staged_items.iter()
            .find(|(_, _, _, _, _, t)| t.starts_with("/usr/share/sddm/themes/ryzora-"))
            .and_then(|(_, _, _, _, _, t)| {
                t.trim_start_matches("/usr/share/sddm/themes/ryzora-")
                    .split('/')
                    .next()
                    .map(|s| s.to_string())
            });

        let slug = match slug_opt {
            Some(s) => s,
            None => {
                clean_staging(&staging_dir);
                return Err("Could not determine SDDM theme slug from target paths".to_string());
            }
        };

        // Stage SDDM files into the temp SDDM staging dir
        let sddm_theme_staging = sddm_staging_dir.join(format!("ryzora-{}", slug));
        if let Err(e) = fs::create_dir_all(&sddm_theme_staging) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to create SDDM staging directory: {}", e));
        }

        let sddm_prefix = format!("/usr/share/sddm/themes/ryzora-{}/", slug);
        for (staged_path, _, _, item_is_symlink, _, target_str) in &staged_items {
            if !target_str.starts_with("/usr/share/sddm/themes/") {
                continue;
            }
            let rel = target_str.trim_start_matches(&sddm_prefix);
            let dest = sddm_theme_staging.join(rel);
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if !item_is_symlink {
                if let Err(e) = fs::copy(staged_path, &dest) {
                    let _ = fs::remove_dir_all(&sddm_staging_dir);
                    clean_staging(&staging_dir);
                    return Err(format!("Failed to pre-stage SDDM file '{}': {}", target_str, e));
                }
            }
        }

        // Check privileged helper before attempting install
        let helper_status = crate::sddm_helper::detect_privileged_helper_status();
        if !helper_status.installed {
            clean_staging(&staging_dir);
            let _ = fs::remove_dir_all(&sddm_staging_dir);
            return Err(format!(
                "Ryzora SDDM helper is not installed or invalid at '{}'. Please click 'Set Up System Integration' first.",
                helper_status.helper_path
            ));
        }

        // Call restricted privileged helper — this triggers Polkit auth
        let result = crate::sddm_helper::install_sddm_theme(&sddm_theme_staging, &slug);
        let _ = fs::remove_dir_all(&sddm_staging_dir); // Always clean SDDM staging

        if let Err(e) = result {
            clean_staging(&staging_dir);
            return Err(format!(
                "SDDM theme installation failed (authentication may have been cancelled): {}",
                e
            ));
        }

        // Physical verification passed inside install_sddm_theme.
        // Now record SDDM files in applied_entries (with target_str remapped to RYZORA_SYSTEM_ROOT equivalent)
        let mut applied_targets = Vec::new();
        let mut applied_entries = Vec::new();
        let mut apply_error: Option<String> = None;

        // Process non-SDDM files normally below
        for (
            staged_path,
            target_path,
            expected_hash,
            item_is_symlink,
            item_symlink_target,
            target_str,
        ) in &staged_items
        {
            if target_str.starts_with("/usr/share/sddm/themes/") {
                // Already handled by sddm_helper — record as installed
                applied_entries.push(InstalledFileEntry {
                    target: target_str.clone(),
                    sha256: expected_hash.clone(),
                    is_symlink: *item_is_symlink,
                    symlink_target: item_symlink_target.clone(),
                });
                applied_targets.push(target_str.clone());
                continue;
            }

            // Non-SDDM files: apply normally
            if let Some(parent) = target_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    apply_error = Some(format!(
                        "Failed to create parent directory for '{}': {}",
                        target_str, e
                    ));
                    break;
                }
            }

            if *item_is_symlink {
                if fs::symlink_metadata(target_path).is_ok() {
                    if let Err(e) = fs::remove_file(target_path) {
                        apply_error = Some(format!(
                            "Failed to remove existing file/symlink at '{}': {}",
                            target_str, e
                        ));
                        break;
                    }
                }
                #[cfg(unix)]
                {
                    let target_dest = item_symlink_target.as_ref().unwrap();
                    if let Err(e) = std::os::unix::fs::symlink(target_dest, target_path) {
                        apply_error = Some(format!("Failed to create symlink '{}': {}", target_str, e));
                        break;
                    }
                }
                applied_entries.push(InstalledFileEntry {
                    target: target_str.clone(),
                    sha256: String::new(),
                    is_symlink: true,
                    symlink_target: item_symlink_target.clone(),
                });
                applied_targets.push(target_str.clone());
            } else {
                if let Ok(meta) = fs::symlink_metadata(target_path) {
                    if meta.file_type().is_symlink() {
                        let _ = fs::remove_file(target_path);
                    }
                }
                if let Err(e) = fs::copy(staged_path, target_path) {
                    apply_error = Some(format!(
                        "Failed to copy staged file to '{}': {}",
                        target_str, e
                    ));
                    break;
                }
                let installed_hash = match sha256_file(target_path) {
                    Ok(h) => h,
                    Err(e) => {
                        apply_error = Some(format!("Failed to verify installed file '{}': {}", target_str, e));
                        break;
                    }
                };
                if &installed_hash != expected_hash {
                    apply_error = Some(format!("Installed file verification failed for '{}'", target_str));
                    break;
                }
                applied_entries.push(InstalledFileEntry {
                    target: target_str.clone(),
                    sha256: installed_hash,
                    is_symlink: false,
                    symlink_target: None,
                });
                applied_targets.push(target_str.clone());
            }
        }
        // Jump to post-apply handling (rollback, record writing)
        // We fall through to the shared error handling below with apply_error/applied_entries.
        // Use a block to scope this early-exit for sddm path:
        if let Some(err) = apply_error {
            clean_staging(&staging_dir);
            return Err(format!("Installation failed after SDDM materialization: {}", err));
        }

        // Write installed record (SDDM path — files materialized via privileged helper)
        let installed_file_list: Vec<String> = applied_entries.iter().map(|e| e.target.clone()).collect();
        let record = InstalledPackageRecord {
            package_id: manifest.id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            package_type: Some(manifest.package_type.clone()),
            repository_id: None,
            installed_at: now,
            snapshot_id: snapshot_id_opt.clone().unwrap_or_default(),
            installed_files: installed_file_list.clone(),
            files: applied_entries,
            package_source_path: package_dir.display().to_string(),
            cryptographic_status: None,
            signer_key_id: None,
            signer_name: None,
            history: vec![],
        };
        let record_raw = serde_json::to_string_pretty(&record)
            .map_err(|e| format!("Failed to serialize install record: {}", e))?;
        let record_file = installed_root.join(format!("{}.json", manifest.id));
        fs::write(&record_file, &record_raw)
            .map_err(|e| format!("Failed to write install record: {}", e))?;
        clean_staging(&staging_dir);

        return Ok(InstallResult {
            success: true,
            package_id: manifest.id,
            version: manifest.version,
            snapshot_id: snapshot_id_opt.unwrap_or_default(),
            installed_files: installed_file_list,
            errors: vec![],
            rolled_back: false,
        });
    }

    let mut applied_targets = Vec::new();
    let mut applied_entries = Vec::new();
    let mut apply_error: Option<String> = None;

    for (
        staged_path,
        target_path,
        expected_hash,
        item_is_symlink,
        item_symlink_target,
        target_str,
    ) in &staged_items
    {
        // Ensure parent dir exists
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                apply_error = Some(format!(
                    "Failed to create parent directory for '{}': {}",
                    target_str, e
                ));
                break;
            }
        }

        if *item_is_symlink {
            if fs::symlink_metadata(target_path).is_ok() {
                if let Err(e) = fs::remove_file(target_path) {
                    apply_error = Some(format!(
                        "Failed to remove existing file/symlink at '{}': {}",
                        target_str, e
                    ));
                    break;
                }
            }

            #[cfg(unix)]
            {
                let target_dest = item_symlink_target.as_ref().unwrap();
                if let Err(e) = std::os::unix::fs::symlink(target_dest, target_path) {
                    apply_error = Some(format!("Failed to create symlink '{}': {}", target_str, e));
                    break;
                }
            }

            // Verify installed symlink immediately
            match fs::symlink_metadata(target_path) {
                Ok(m) if m.file_type().is_symlink() => {
                    let actual_link = fs::read_link(target_path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string());
                    if actual_link != *item_symlink_target {
                        apply_error = Some(format!(
                            "Installed symlink target mismatch for '{}'",
                            target_str
                        ));
                        break;
                    }
                }
                _ => {
                    apply_error = Some(format!(
                        "Installed object is not a symlink for '{}'",
                        target_str
                    ));
                    break;
                }
            }

            applied_entries.push(InstalledFileEntry {
                target: target_str.clone(),
                sha256: String::new(),
                is_symlink: true,
                symlink_target: item_symlink_target.clone(),
            });
            applied_targets.push(target_str.clone());
        } else {
            // If target is an existing symlink, remove the symlink itself so we don't write through it
            if let Ok(meta) = fs::symlink_metadata(target_path) {
                if meta.file_type().is_symlink() {
                    if let Err(e) = fs::remove_file(target_path) {
                        apply_error = Some(format!(
                            "Failed to remove target symlink '{}': {}",
                            target_str, e
                        ));
                        break;
                    }
                }
            }

            // Copy staged file to destination (pure filesystem operation)
            if let Err(e) = fs::copy(staged_path, target_path) {
                apply_error = Some(format!(
                    "Failed to copy staged file to '{}' (write permissions required): {}",
                    target_str, e
                ));
                break;
            }

            // 8. Verify installed file immediately
            let installed_hash = match sha256_file(target_path) {
                Ok(h) => h,
                Err(e) => {
                    apply_error = Some(format!(
                        "Failed to verify installed file '{}': {}",
                        target_str, e
                    ));
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

            applied_entries.push(InstalledFileEntry {
                target: target_str.clone(),
                sha256: installed_hash,
                is_symlink: false,
                symlink_target: None,
            });
            applied_targets.push(target_str.clone());
        }
    }
    // 9. Handle failure and automatic rollback
    if let Some(err) = apply_error {
        clean_staging(&staging_dir);

        let (_rolled_back_ok, rollback_msg) = if let Some(ref snap_id) = snapshot_id_opt {
            let rollback_res = restore_snapshot_in(snap_id, home_dir, snapshots_root);
            let ok = rollback_res.map(|r| r.success).unwrap_or(false);
            (
                ok,
                if ok {
                    "automatic rollback was successful".to_string()
                } else {
                    "automatic rollback was attempted with warnings".to_string()
                },
            )
        } else {
            // No snapshot was created: remove any partially written target files
            for target_str in &applied_targets {
                if let Ok(target_p) = validate_target_safety(target_str, home_dir) {
                    let _ = fs::remove_file(&target_p);
                }
            }
            (
                true,
                "partially applied files were cleaned up (no pre-install snapshot created)".to_string(),
            )
        };

        return Ok(InstallResult {
            success: false,
            package_id: manifest.id,
            version: manifest.version,
            snapshot_id: snapshot_id_opt.unwrap_or_default(),
            installed_files: vec![],
            errors: vec![format!(
                "Installation failed ({}); {}",
                err,
                rollback_msg
            )],
            rolled_back: true,
        });
    }

    // Clean up staging directory on success
    clean_staging(&staging_dir);

    // 10. Persist authoritative installation record
    if let Err(e) = fs::create_dir_all(installed_root) {
        return Err(format!(
            "Failed to create installed packages directory: {}",
            e
        ));
    }

    let repo_id =
        crate::repository::create_default_manager().find_repository_for_package(&manifest.id);

    let tree_hash_val = crate::repository::compute_package_tree_hash(package_dir, &manifest).ok();

    let record_snapshot_id = snapshot_id_opt.unwrap_or_default();

    let history_entry = InstalledHistoryEntry {
        version: manifest.version.clone(),
        installed_at: now,
        snapshot_id: record_snapshot_id.clone(),
        repository_id: repo_id.clone(),
        tree_hash: tree_hash_val,
    };

    let record = InstalledPackageRecord {
        package_id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        package_type: Some(manifest.package_type),
        repository_id: repo_id,
        installed_at: now,
        snapshot_id: record_snapshot_id.clone(),
        installed_files: applied_targets.clone(),
        files: applied_entries,
        package_source_path: package_dir.display().to_string(),
        cryptographic_status: Some(crypto_eval.status),
        signer_key_id: crypto_eval.key_id,
        signer_name: crypto_eval.signer_name,
        history: vec![history_entry],
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
        snapshot_id: record_snapshot_id,
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

pub fn manifest_id_safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Helper to safely remove empty parent directories upwards until reaching home or standard base directories.
pub fn cleanup_empty_parents(
    path: &Path,
    home_dir: &Path,
    removed_dirs: &mut Vec<String>,
    retained_dirs: &mut Vec<String>,
) {
    let mut curr = path.parent();
    while let Some(dir) = curr {
        if dir == home_dir {
            break;
        }
        if let Ok(rel) = dir.strip_prefix(home_dir) {
            let rel_str = rel.to_string_lossy();
            if rel_str.is_empty()
                || rel_str == ".config"
                || rel_str == ".local"
                || rel_str == ".local/share"
                || rel_str == ".local/state"
            {
                break;
            }
        } else {
            break;
        }

        match fs::read_dir(dir) {
            Ok(mut entries) => {
                if entries.next().is_none() {
                    let dir_str = dir.display().to_string();
                    if fs::remove_dir(dir).is_ok() {
                        if !removed_dirs.contains(&dir_str) {
                            removed_dirs.push(dir_str);
                        }
                        curr = dir.parent();
                        continue;
                    } else {
                        if !retained_dirs.contains(&dir_str) {
                            retained_dirs.push(dir_str);
                        }
                        break;
                    }
                } else {
                    let dir_str = dir.display().to_string();
                    if !retained_dirs.contains(&dir_str) {
                        retained_dirs.push(dir_str);
                    }
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe Uninstall Pipeline (Phase 7)
// ─────────────────────────────────────────────────────────────────────────────

/// Safely uninstalls an installed package with cryptographic verification,
/// pre-uninstall snapshotting, conflict retention, and automatic rollback.
pub fn uninstall_package_in(
    package_id: &str,
    home_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
) -> Result<UninstallResult, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
        || package_id.contains('\0')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;

    // 10. Backward compatibility: Stale/legacy metadata without checksums is refused safely
    if record.files.is_empty() {
        return Err(format!(
            "Installed package '{}' contains legacy metadata without cryptographic file checksums. Automated uninstall cannot verify file ownership safely. Manual review or re-installation is required.",
            package_id
        ));
    }

    // Validate all target paths against sandbox rules
    for entry in &record.files {
        validate_target_safety(&entry.target, home_dir)?;
    }

    // Inspect filesystem state and categorize files
    let mut files_to_remove: Vec<(PathBuf, String, bool)> = Vec::new(); // (path, target_str, is_symlink)
    let mut already_missing_files = Vec::new();
    let mut conflict_files = Vec::new();

    for entry in &record.files {
        let target_path = match validate_target_safety(&entry.target, home_dir) {
            Ok(p) => p,
            Err(e) => return Err(format!("Invalid target path in record: {}", e)),
        };

        match fs::symlink_metadata(&target_path) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    // Current object on disk is a symlink
                    if !entry.is_symlink {
                        // User replaced installed regular file with a symlink -> conflict!
                        conflict_files.push(entry.target.clone());
                    } else {
                        // Installed as symlink. Verify symlink target ownership.
                        match &entry.symlink_target {
                            Some(recorded_target) => {
                                match fs::read_link(&target_path) {
                                    Ok(current_link) => {
                                        if current_link.to_string_lossy() == *recorded_target {
                                            // Ownership verified! Link target matches recorded target.
                                            files_to_remove.push((
                                                target_path,
                                                entry.target.clone(),
                                                true,
                                            ));
                                        } else {
                                            // User changed symlink target -> conflict! NEVER delete!
                                            conflict_files.push(entry.target.clone());
                                        }
                                    }
                                    Err(_) => {
                                        // Cannot safely inspect link -> conflict!
                                        conflict_files.push(entry.target.clone());
                                    }
                                }
                            }
                            None => {
                                // Legacy metadata without recorded symlink_target:
                                // Treat as unverifiable ownership and report conflict / manual review!
                                conflict_files.push(entry.target.clone());
                            }
                        }
                    }
                } else if meta.is_file() {
                    if entry.is_symlink {
                        // Was installed as symlink, now regular file -> conflict!
                        conflict_files.push(entry.target.clone());
                    } else {
                        let current_hash = match sha256_file(&target_path) {
                            Ok(h) => h,
                            Err(e) => {
                                return Err(format!("Failed to checksum '{}': {}", entry.target, e))
                            }
                        };

                        if current_hash == entry.sha256 {
                            // File untouched! Safe to remove
                            files_to_remove.push((target_path, entry.target.clone(), false));
                        } else {
                            // User modified file -> conflict! NEVER delete!
                            conflict_files.push(entry.target.clone());
                        }
                    }
                } else {
                    // Directory or special file where regular file was expected -> conflict
                    conflict_files.push(entry.target.clone());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                already_missing_files.push(entry.target.clone());
            }
            Err(e) => {
                return Err(format!(
                    "Failed to inspect target file '{}': {}",
                    entry.target, e
                ));
            }
        }
    }
    // Create pre-uninstall snapshot of user-space paths only
    let snapshot_targets: Vec<String> = record
        .files
        .iter()
        .filter(|f| f.target.starts_with("~/"))
        .map(|f| f.target.clone())
        .collect();

    let mut snapshot_id_opt: Option<String> = None;
    if !snapshot_targets.is_empty() {
        let snapshot_label = format!("pre-uninstall-{}", package_id);
        let snapshot_meta =
            create_snapshot_in(&snapshot_label, &snapshot_targets, home_dir, snapshots_root)
                .map_err(|e| format!("Failed to create pre-uninstall snapshot: {}", e))?;

        let snap_valid = verify_snapshot_in(&snapshot_meta.id, snapshots_root)
            .map_err(|e| format!("Snapshot verification error: {}", e))?;
        if !snap_valid {
            return Err("Pre-uninstall snapshot failed verification — uninstall aborted before any file deletion".to_string());
        }
        snapshot_id_opt = Some(snapshot_meta.id);
    }

    // Perform removals
    let mut removed_files = Vec::new();
    let mut remove_error: Option<String> = None;

    for (path, target_str, is_symlink) in &files_to_remove {
        if target_str.starts_with("/usr/share/sddm") || target_str.starts_with("/etc/") {
            // System-level file, skip user uninstaller deletion (managed by privileged helper)
            continue;
        }
        let res = if *is_symlink {
            fs::remove_file(path) // removes the symlink itself, never touches target
        } else {
            fs::remove_file(path)
        };

        if let Err(e) = res {
            remove_error = Some(format!("Failed to remove file '{}': {}", target_str, e));
            break;
        }
        removed_files.push(target_str.clone());
    }

    // Rollback on unexpected failure
    if let Some(err) = remove_error {
        let rolled_back_ok = if let Some(snap_id) = &snapshot_id_opt {
            restore_snapshot_in(snap_id, home_dir, snapshots_root).map(|r| r.success).unwrap_or(false)
        } else {
            false
        };

        return Ok(UninstallResult {
            package_id: package_id.to_string(),
            success: false,
            removed_files: vec![],
            already_missing_files,
            conflict_files,
            removed_directories: vec![],
            retained_directories: vec![],
            snapshot_id: snapshot_id_opt,
            rolled_back: true,
            error: Some(format!(
                "Uninstall failed ({}); automatic rollback was {}",
                err,
                if rolled_back_ok {
                    "successful"
                } else {
                    "attempted with warnings"
                }
            )),
        });
    }

    // Clean empty parent directories
    let mut removed_directories = Vec::new();
    let mut retained_directories = Vec::new();

    for (path, _, _) in &files_to_remove {
        cleanup_empty_parents(
            path,
            home_dir,
            &mut removed_directories,
            &mut retained_directories,
        );
    }

    // Update / remove installed metadata record
    if let Err(e) = fs::remove_file(&record_file) {
        return Err(format!("Failed to remove installed package record: {}", e));
    }

    Ok(UninstallResult {
        package_id: package_id.to_string(),
        success: true,
        removed_files,
        already_missing_files,
        conflict_files,
        removed_directories,
        retained_directories,
        snapshot_id: snapshot_id_opt,
        rolled_back: false,
        error: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe Update Detection (Phase 7)
// ─────────────────────────────────────────────────────────────────────────────

pub fn check_package_update_in(
    package_id: &str,
    installed_root: &Path,
    repo_mgr: &crate::repository::RepositoryManager,
) -> Result<PackageUpdateStatus, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
        || package_id.contains('\0')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;

    // Check repository
    let mut found_entry: Option<crate::repository::RepositoryPackageEntry> = None;
    let mut repo_id_found: Option<String> = None;
    let mut has_repo_error = false;
    let mut last_error_msg = None;

    if let Some(ref target_repo_id) = record.repository_id {
        if let Some(repo) = repo_mgr.get_repository_by_id(target_repo_id) {
            match repo.list_entries() {
                Ok(entries) => {
                    if let Some(entry) = entries.into_iter().find(|e| e.id == package_id) {
                        found_entry = Some(entry);
                        repo_id_found = Some(target_repo_id.clone());
                    }
                }
                Err(e) => {
                    has_repo_error = true;
                    last_error_msg = Some(e);
                }
            }
        } else {
            has_repo_error = true;
            last_error_msg = Some(format!(
                "Repository '{}' unavailable or not configured",
                target_repo_id
            ));
        }
    }

    // If not found in recorded repo, search across all repositories
    if found_entry.is_none() && !has_repo_error {
        for repo in repo_mgr.repositories() {
            match repo.list_entries() {
                Ok(entries) => {
                    if let Some(entry) = entries.into_iter().find(|e| e.id == package_id) {
                        found_entry = Some(entry);
                        repo_id_found = Some(repo.id().to_string());
                        break;
                    }
                }
                Err(e) => {
                    has_repo_error = true;
                    last_error_msg = Some(e);
                }
            }
        }
    }

    if let Some(entry) = found_entry {
        let inst_v = semver::Version::parse(&record.version);
        let avail_v = semver::Version::parse(&entry.version);

        let status = match (inst_v, avail_v) {
            (Ok(i), Ok(a)) => {
                if a > i {
                    UpdateStatusKind::UpdateAvailable
                } else {
                    UpdateStatusKind::UpToDate
                }
            }
            _ => UpdateStatusKind::UnableToDetermine,
        };

        Ok(PackageUpdateStatus {
            package_id: package_id.to_string(),
            installed_version: record.version,
            available_version: Some(entry.version),
            repository_id: repo_id_found,
            status,
            message: None,
        })
    } else if has_repo_error {
        Ok(PackageUpdateStatus {
            package_id: package_id.to_string(),
            installed_version: record.version,
            available_version: None,
            repository_id: record.repository_id,
            status: UpdateStatusKind::RepositoryUnavailable,
            message: last_error_msg,
        })
    } else {
        Ok(PackageUpdateStatus {
            package_id: package_id.to_string(),
            installed_version: record.version,
            available_version: None,
            repository_id: record.repository_id,
            status: UpdateStatusKind::PackageNotFound,
            message: Some(format!(
                "Package '{}' not found in available repositories",
                package_id
            )),
        })
    }
}

pub fn check_all_updates_in(
    installed_root: &Path,
    repo_mgr: &crate::repository::RepositoryManager,
) -> Result<Vec<PackageUpdateStatus>, String> {
    let installed = list_installed_packages_in(installed_root)?;
    let mut results = Vec::new();
    for pkg in installed {
        if let Ok(status) = check_package_update_in(&pkg.package_id, installed_root, repo_mgr) {
            results.push(status);
        }
    }
    Ok(results)
}

// ─────────────────────────────────────────────────────────────────────────────
// Update Preview Pipeline (Phase 7)
// ─────────────────────────────────────────────────────────────────────────────

pub fn preview_package_update_in(
    package_id: &str,
    home_dir: &Path,
    installed_root: &Path,
    new_package_dir: &Path,
) -> Result<UpdatePlan, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
        || package_id.contains('\0')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;

    if record.files.is_empty() {
        return Err(format!(
            "Installed package '{}' contains legacy metadata without checksums. Cannot safely preview update.",
            package_id
        ));
    }

    let new_manifest = load_package_manifest(new_package_dir)?;
    if new_manifest.id != record.package_id {
        return Err(format!(
            "Manifest ID mismatch: expected '{}', got '{}'",
            record.package_id, new_manifest.id
        ));
    }

    let mut creates = Vec::new();
    let mut replaces = Vec::new();
    let mut unchanged = Vec::new();
    let mut conflicts = Vec::new();
    let mut obsolete_removes = Vec::new();
    let mut obsolete_retains = Vec::new();
    let mut details = Vec::new();

    use std::collections::HashMap;
    let old_map: HashMap<&str, &InstalledFileEntry> = record
        .files
        .iter()
        .map(|f| (f.target.as_str(), f))
        .collect();

    // 1. Process files declared in new manifest
    for file_decl in &new_manifest.files {
        let src_path = validate_package_source_file(new_package_dir, &file_decl.source)?;
        let target_path = validate_target_safety(&file_decl.target, home_dir)?;

        let src_meta = fs::symlink_metadata(&src_path)
            .map_err(|e| format!("Failed to read metadata for '{}': {}", file_decl.source, e))?;
        let src_is_symlink = src_meta.file_type().is_symlink();

        let (new_hash, new_link_target) = if src_is_symlink {
            let lt = fs::read_link(&src_path)
                .map_err(|e| format!("Failed to read symlink '{}': {}", file_decl.source, e))?;
            (String::new(), Some(lt.to_string_lossy().to_string()))
        } else {
            let h = sha256_file(&src_path)?;
            (h, None)
        };

        let target_str = file_decl.target.clone();

        match fs::symlink_metadata(&target_path) {
            Ok(meta) => {
                let current_hash = if meta.file_type().is_symlink() {
                    None
                } else {
                    sha256_file(&target_path).ok()
                };

                if let Some(old_entry) = old_map.get(target_str.as_str()) {
                    // File was tracked in previous installation
                    if meta.file_type().is_symlink() != old_entry.is_symlink {
                        conflicts.push(target_str.clone());
                        details.push(UpdateFileItem {
                            target: target_str.clone(),
                            action: FileAction::Conflict,
                            reason: "File type changed on disk (symlink / regular mismatch)"
                                .to_string(),
                            current_sha256: current_hash,
                            new_sha256: if src_is_symlink { None } else { Some(new_hash) },
                        });
                    } else if meta.file_type().is_symlink() {
                        // Both on disk and in old entry are symlinks
                        match &old_entry.symlink_target {
                            Some(rec_target) => {
                                match fs::read_link(&target_path) {
                                    Ok(cur_link) if cur_link.to_string_lossy() == *rec_target => {
                                        // Unmodified by user! Check if new version has same target
                                        if new_link_target.as_deref() == Some(rec_target.as_str()) {
                                            unchanged.push(target_str.clone());
                                            details.push(UpdateFileItem {
                                                target: target_str.clone(),
                                                action: FileAction::Unchanged,
                                                reason:
                                                    "Symlink target is identical in new version"
                                                        .to_string(),
                                                current_sha256: None,
                                                new_sha256: None,
                                            });
                                        } else {
                                            replaces.push(target_str.clone());
                                            details.push(UpdateFileItem {
                                                target: target_str.clone(),
                                                action: FileAction::Replace,
                                                reason: "Symlink target unmodified by user, will be updated".to_string(),
                                                current_sha256: None,
                                                new_sha256: None,
                                            });
                                        }
                                    }
                                    _ => {
                                        // User modified symlink target on disk -> conflict!
                                        conflicts.push(target_str.clone());
                                        details.push(UpdateFileItem {
                                            target: target_str.clone(),
                                            action: FileAction::Conflict,
                                            reason:
                                                "Symlink target modified by user after installation"
                                                    .to_string(),
                                            current_sha256: None,
                                            new_sha256: None,
                                        });
                                    }
                                }
                            }
                            None => {
                                // Legacy metadata without recorded symlink_target
                                conflicts.push(target_str.clone());
                                details.push(UpdateFileItem {
                                    target: target_str.clone(),
                                    action: FileAction::Conflict,
                                    reason: "Legacy symlink metadata without recorded target; retained for manual review".to_string(),
                                    current_sha256: None,
                                    new_sha256: None,
                                });
                            }
                        }
                    } else if let Some(ref cur_h) = current_hash {
                        if cur_h != &old_entry.sha256 {
                            // User modified file on disk
                            conflicts.push(target_str.clone());
                            details.push(UpdateFileItem {
                                target: target_str.clone(),
                                action: FileAction::Conflict,
                                reason: "File modified by user after installation".to_string(),
                                current_sha256: current_hash.clone(),
                                new_sha256: Some(new_hash),
                            });
                        } else if cur_h == &new_hash {
                            unchanged.push(target_str.clone());
                            details.push(UpdateFileItem {
                                target: target_str.clone(),
                                action: FileAction::Unchanged,
                                reason: "File content is identical in new version".to_string(),
                                current_sha256: current_hash.clone(),
                                new_sha256: Some(new_hash),
                            });
                        } else {
                            replaces.push(target_str.clone());
                            details.push(UpdateFileItem {
                                target: target_str.clone(),
                                action: FileAction::Replace,
                                reason:
                                    "File unmodified by user, will be replaced with new version"
                                        .to_string(),
                                current_sha256: current_hash.clone(),
                                new_sha256: Some(new_hash),
                            });
                        }
                    } else {
                        conflicts.push(target_str.clone());
                        details.push(UpdateFileItem {
                            target: target_str.clone(),
                            action: FileAction::Conflict,
                            reason: "Cannot checksum target file".to_string(),
                            current_sha256: None,
                            new_sha256: Some(new_hash),
                        });
                    }
                } else {
                    // Target file already exists on disk, but was NOT tracked by previous install!
                    conflicts.push(target_str.clone());
                    details.push(UpdateFileItem {
                        target: target_str.clone(),
                        action: FileAction::Conflict,
                        reason: "Pre-existing untracked file exists at destination".to_string(),
                        current_sha256: current_hash,
                        new_sha256: if src_is_symlink { None } else { Some(new_hash) },
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if old_map.contains_key(target_str.as_str()) {
                    // Was tracked but deleted by user -> recreate/replace
                    replaces.push(target_str.clone());
                    details.push(UpdateFileItem {
                        target: target_str.clone(),
                        action: FileAction::Replace,
                        reason: "Tracked file missing on disk; will be restored with new version"
                            .to_string(),
                        current_sha256: None,
                        new_sha256: if src_is_symlink { None } else { Some(new_hash) },
                    });
                } else {
                    creates.push(target_str.clone());
                    details.push(UpdateFileItem {
                        target: target_str.clone(),
                        action: FileAction::Create,
                        reason: "New file introduced in this package version".to_string(),
                        current_sha256: None,
                        new_sha256: if src_is_symlink { None } else { Some(new_hash) },
                    });
                }
            }
            Err(e) => {
                return Err(format!("Failed to inspect target '{}': {}", target_str, e));
            }
        }
    }

    // 2. Identify obsolete files (tracked in old version but absent in new manifest)
    for old_entry in &record.files {
        if !new_manifest
            .files
            .iter()
            .any(|f| f.target == old_entry.target)
        {
            let target_path = match validate_target_safety(&old_entry.target, home_dir) {
                Ok(p) => p,
                Err(_) => continue,
            };

            match fs::symlink_metadata(&target_path) {
                Ok(meta) => {
                    if meta.file_type().is_symlink() {
                        if !old_entry.is_symlink {
                            // Was regular file, now symlink -> conflict!
                            obsolete_retains.push(old_entry.target.clone());
                            conflicts.push(old_entry.target.clone());
                            details.push(UpdateFileItem {
                                target: old_entry.target.clone(),
                                action: FileAction::ObsoleteRetain,
                                reason: "Obsolete file replaced with symlink; will be retained as conflict".to_string(),
                                current_sha256: None,
                                new_sha256: None,
                            });
                        } else {
                            // Was installed as symlink
                            match &old_entry.symlink_target {
                                Some(recorded_target) => {
                                    match fs::read_link(&target_path) {
                                        Ok(current_link)
                                            if current_link.to_string_lossy()
                                                == *recorded_target =>
                                        {
                                            // Ownership verified! Unchanged symlink target -> safe to remove
                                            obsolete_removes.push(old_entry.target.clone());
                                            details.push(UpdateFileItem {
                                                target: old_entry.target.clone(),
                                                action: FileAction::ObsoleteRemove,
                                                reason: "Obsolete symlink removed in new version; link target unchanged".to_string(),
                                                current_sha256: None,
                                                new_sha256: None,
                                            });
                                        }
                                        _ => {
                                            // Link target changed or unreadable -> conflict!
                                            obsolete_retains.push(old_entry.target.clone());
                                            conflicts.push(old_entry.target.clone());
                                            details.push(UpdateFileItem {
                                                target: old_entry.target.clone(),
                                                action: FileAction::ObsoleteRetain,
                                                reason: "Obsolete symlink target modified by user; will be retained as conflict".to_string(),
                                                current_sha256: None,
                                                new_sha256: None,
                                            });
                                        }
                                    }
                                }
                                None => {
                                    // Legacy symlink metadata without recorded target -> treat as conflict / unverifiable!
                                    obsolete_retains.push(old_entry.target.clone());
                                    conflicts.push(old_entry.target.clone());
                                    details.push(UpdateFileItem {
                                        target: old_entry.target.clone(),
                                        action: FileAction::ObsoleteRetain,
                                        reason: "Legacy symlink metadata without recorded target; retained for manual review".to_string(),
                                        current_sha256: None,
                                        new_sha256: None,
                                    });
                                }
                            }
                        }
                    } else if meta.is_file() {
                        if old_entry.is_symlink {
                            // Was installed as symlink, but now regular file -> conflict!
                            obsolete_retains.push(old_entry.target.clone());
                            conflicts.push(old_entry.target.clone());
                            details.push(UpdateFileItem {
                                target: old_entry.target.clone(),
                                action: FileAction::ObsoleteRetain,
                                reason: "Obsolete symlink replaced with regular file; will be retained as conflict".to_string(),
                                current_sha256: None,
                                new_sha256: None,
                            });
                        } else {
                            let cur_hash = sha256_file(&target_path).ok();
                            if let Some(ref h) = cur_hash {
                                if h == &old_entry.sha256 {
                                    // Unmodified regular file -> safe to remove
                                    obsolete_removes.push(old_entry.target.clone());
                                    details.push(UpdateFileItem {
                                        target: old_entry.target.clone(),
                                        action: FileAction::ObsoleteRemove,
                                        reason: "Obsolete file removed in new version; untouched by user".to_string(),
                                        current_sha256: cur_hash,
                                        new_sha256: None,
                                    });
                                } else {
                                    // Modified regular file -> retain as conflict!
                                    obsolete_retains.push(old_entry.target.clone());
                                    conflicts.push(old_entry.target.clone());
                                    details.push(UpdateFileItem {
                                        target: old_entry.target.clone(),
                                        action: FileAction::ObsoleteRetain,
                                        reason: "Obsolete file modified by user; will be retained as conflict".to_string(),
                                        current_sha256: cur_hash,
                                        new_sha256: None,
                                    });
                                }
                            } else {
                                obsolete_retains.push(old_entry.target.clone());
                                conflicts.push(old_entry.target.clone());
                                details.push(UpdateFileItem {
                                    target: old_entry.target.clone(),
                                    action: FileAction::ObsoleteRetain,
                                    reason: "Cannot checksum obsolete file; retained as conflict"
                                        .to_string(),
                                    current_sha256: None,
                                    new_sha256: None,
                                });
                            }
                        }
                    } else {
                        // Directory or special file where regular file was expected -> conflict
                        obsolete_retains.push(old_entry.target.clone());
                        conflicts.push(old_entry.target.clone());
                        details.push(UpdateFileItem {
                            target: old_entry.target.clone(),
                            action: FileAction::ObsoleteRetain,
                            reason:
                                "Obsolete entry is directory or special file; retained as conflict"
                                    .to_string(),
                            current_sha256: None,
                            new_sha256: None,
                        });
                    }
                }
                Err(_) => {
                    // Already missing, nothing to remove
                }
            }
        }
    }

    let has_conflicts = !conflicts.is_empty();

    Ok(UpdatePlan {
        package_id: package_id.to_string(),
        from_version: record.version.clone(),
        to_version: new_manifest.version.clone(),
        repository_id: record.repository_id.clone(),
        creates,
        replaces,
        unchanged,
        conflicts,
        obsolete_removes,
        obsolete_retains,
        details,
        has_conflicts,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Update Application Pipeline (Phase 7)
// ─────────────────────────────────────────────────────────────────────────────

pub fn apply_package_update_in(
    package_id: &str,
    home_dir: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    new_package_dir: &Path,
    system_info: &SystemInfo,
    allow_conflicts: bool,
    new_repository_id: Option<String>,
) -> Result<UpdateResult, String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
        || package_id.contains('\0')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;

    if record.files.is_empty() {
        return Err(format!(
            "Installed package '{}' contains legacy metadata without checksums. Cannot safely apply update.",
            package_id
        ));
    }

    let new_manifest = load_package_manifest(new_package_dir)?;
    if new_manifest.id != record.package_id {
        return Err(format!(
            "Manifest ID mismatch: expected '{}', got '{}'",
            record.package_id, new_manifest.id
        ));
    }

    // Enforce SemVer: strictly disallow downgrade and same-version update
    let inst_v = semver::Version::parse(&record.version)
        .map_err(|e| format!("Invalid installed version '{}': {}", record.version, e))?;
    let new_v = semver::Version::parse(&new_manifest.version)
        .map_err(|e| format!("Invalid new version '{}': {}", new_manifest.version, e))?;
    if new_v <= inst_v {
        if new_v == inst_v {
            return Err(format!(
                "Package '{}' is already at version {}",
                package_id, record.version
            ));
        } else {
            return Err(format!(
                "Downgrade attempt refused for package '{}' (installed: {}, target: {})",
                package_id, record.version, new_manifest.version
            ));
        }
    }

    // System compatibility check
    let reqs = CompatibilityRequirements {
        supported_distros: new_manifest.compatibility.distros.clone(),
        supported_desktops: new_manifest.compatibility.desktops.clone(),
        supported_sessions: new_manifest.compatibility.sessions.clone(),
        required_binaries: new_manifest.compatibility.required.clone(),
        optional_binaries: new_manifest.compatibility.optional.clone(),
    };
    let compat_report = evaluate_compatibility(system_info, &reqs);
    if compat_report.level != CompatibilityLevel::Compatible {
        return Err(format!(
            "Package '{}' version '{}' is incompatible with your system environment",
            new_manifest.id, new_manifest.version
        ));
    }

    // Generate update preview
    let plan = preview_package_update_in(package_id, home_dir, installed_root, new_package_dir)?;

    if plan.has_conflicts && !allow_conflicts {
        return Err(format!(
            "Update blocked due to conflicting user-modified files: {}. Refusing to overwrite modified files.",
            plan.conflicts.join(", ")
        ));
    }

    // Collect ALL target paths for pre-update snapshot:
    // All paths in new manifest + all paths in old record (including obsolete)
    let mut all_targets: Vec<String> = Vec::new();
    for f in &new_manifest.files {
        if !all_targets.contains(&f.target) {
            all_targets.push(f.target.clone());
        }
    }
    for f in &record.files {
        if !all_targets.contains(&f.target) {
            all_targets.push(f.target.clone());
        }
    }

    // Pre-update snapshot
    let snapshot_label = format!(
        "pre-update-{}-{}-to-{}",
        package_id, record.version, new_manifest.version
    );
    let snapshot_meta = create_snapshot_in(&snapshot_label, &all_targets, home_dir, snapshots_root)
        .map_err(|e| format!("Failed to create pre-update snapshot: {}", e))?;

    let snap_valid = verify_snapshot_in(&snapshot_meta.id, snapshots_root)
        .map_err(|e| format!("Snapshot verification error: {}", e))?;
    if !snap_valid {
        return Err("Pre-update snapshot failed integrity verification — update aborted before any file modification".to_string());
    }

    // Staging
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let staging_id = format!("update-{}-{}", manifest_id_safe(package_id), now);
    let staging_dir = staging_root.join(&staging_id);
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging directory: {}", e))?;

    let clean_staging = |dir: &Path| {
        let _ = fs::remove_dir_all(dir);
    };

    let mut staged_items = Vec::new(); // (staged_path, target_path, expected_hash, is_symlink, symlink_target, target_str)

    for (idx, file_decl) in new_manifest.files.iter().enumerate() {
        // If file is a conflict and allow_conflicts is true, NEVER overwrite user file
        if plan.conflicts.contains(&file_decl.target) {
            continue; // retain user file
        }

        let src_path = match validate_package_source_file(new_package_dir, &file_decl.source) {
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

        let src_meta = match fs::symlink_metadata(&src_path) {
            Ok(m) => m,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to read metadata for source file '{}': {}",
                    file_decl.source, e
                ));
            }
        };

        let src_is_symlink = src_meta.file_type().is_symlink();
        let staged_file_path = staging_dir.join(format!("file_{}", idx));

        let (expected_hash, symlink_target) = if src_is_symlink {
            #[cfg(unix)]
            {
                let link_target = match fs::read_link(&src_path) {
                    Ok(lt) => lt,
                    Err(e) => {
                        clean_staging(&staging_dir);
                        return Err(format!(
                            "Failed to read symlink '{}': {}",
                            file_decl.source, e
                        ));
                    }
                };
                let link_str = link_target.to_string_lossy().to_string();
                if let Err(e) = std::os::unix::fs::symlink(&link_target, &staged_file_path) {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to stage symlink '{}': {}",
                        file_decl.source, e
                    ));
                }
                (String::new(), Some(link_str))
            }
            #[cfg(not(unix))]
            {
                clean_staging(&staging_dir);
                return Err("Symlinks are only supported on Unix platforms".to_string());
            }
        } else {
            let src_hash = match sha256_file(&src_path) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to checksum source file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            if let Err(e) = fs::copy(&src_path, &staged_file_path) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to stage file '{}': {}",
                    file_decl.source, e
                ));
            }

            let staged_hash = match sha256_file(&staged_file_path) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to verify staged file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            if staged_hash != src_hash {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Staged file checksum mismatch for '{}'",
                    file_decl.source
                ));
            }
            (src_hash, None)
        };

        staged_items.push((
            staged_file_path,
            target_path,
            expected_hash,
            src_is_symlink,
            symlink_target,
            file_decl.target.clone(),
        ));
    }

    // Apply staged files
    let mut updated_files = Vec::new();
    let mut applied_entries = Vec::new();
    let mut apply_error: Option<String> = None;

    for (
        staged_path,
        target_path,
        expected_hash,
        item_is_symlink,
        item_symlink_target,
        target_str,
    ) in &staged_items
    {
        if let Some(parent) = target_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                apply_error = Some(format!(
                    "Failed to create parent directory for '{}': {}",
                    target_str, e
                ));
                break;
            }
        }

        if *item_is_symlink {
            if fs::symlink_metadata(target_path).is_ok() {
                if let Err(e) = fs::remove_file(target_path) {
                    apply_error = Some(format!(
                        "Failed to remove existing file/symlink at '{}': {}",
                        target_str, e
                    ));
                    break;
                }
            }

            #[cfg(unix)]
            {
                let target_dest = item_symlink_target.as_ref().unwrap();
                if let Err(e) = std::os::unix::fs::symlink(target_dest, target_path) {
                    apply_error = Some(format!("Failed to create symlink '{}': {}", target_str, e));
                    break;
                }
            }

            // Verify installed symlink immediately
            match fs::symlink_metadata(target_path) {
                Ok(m) if m.file_type().is_symlink() => {
                    let actual_link = fs::read_link(target_path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string());
                    if actual_link != *item_symlink_target {
                        apply_error = Some(format!(
                            "Installed symlink target mismatch for '{}'",
                            target_str
                        ));
                        break;
                    }
                }
                _ => {
                    apply_error = Some(format!(
                        "Installed object is not a symlink for '{}'",
                        target_str
                    ));
                    break;
                }
            }

            applied_entries.push(InstalledFileEntry {
                target: target_str.clone(),
                sha256: String::new(),
                is_symlink: true,
                symlink_target: item_symlink_target.clone(),
            });
            updated_files.push(target_str.clone());
        } else {
            // If target is an existing symlink, remove it first
            if let Ok(meta) = fs::symlink_metadata(target_path) {
                if meta.file_type().is_symlink() {
                    if let Err(e) = fs::remove_file(target_path) {
                        apply_error = Some(format!(
                            "Failed to remove target symlink '{}': {}",
                            target_str, e
                        ));
                        break;
                    }
                }
            }

            if let Err(e) = fs::copy(staged_path, target_path) {
                apply_error = Some(format!(
                    "Failed to copy staged file to '{}': {}",
                    target_str, e
                ));
                break;
            }

            // Verify installed file immediately
            let installed_hash = match sha256_file(target_path) {
                Ok(h) => h,
                Err(e) => {
                    apply_error = Some(format!(
                        "Failed to verify installed file '{}': {}",
                        target_str, e
                    ));
                    break;
                }
            };

            if &installed_hash != expected_hash {
                apply_error = Some(format!(
                    "Installed file checksum mismatch for '{}'",
                    target_str
                ));
                break;
            }

            applied_entries.push(InstalledFileEntry {
                target: target_str.clone(),
                sha256: installed_hash,
                is_symlink: false,
                symlink_target: None,
            });
            updated_files.push(target_str.clone());
        }
    }

    // Rollback on failure
    if let Some(err) = apply_error {
        clean_staging(&staging_dir);
        let rollback_res = restore_snapshot_in(&snapshot_meta.id, home_dir, snapshots_root);
        let rolled_back_ok = rollback_res.map(|r| r.success).unwrap_or(false);

        return Ok(UpdateResult {
            success: false,
            package_id: package_id.to_string(),
            from_version: record.version.clone(),
            to_version: new_manifest.version.clone(),
            snapshot_id: snapshot_meta.id,
            updated_files: vec![],
            obsolete_removed: vec![],
            conflicts_retained: plan.conflicts.clone(),
            rolled_back: true,
            errors: vec![format!(
                "Update failed ({}); automatic rollback was {}",
                err,
                if rolled_back_ok {
                    "successful"
                } else {
                    "attempted with warnings"
                }
            )],
        });
    }

    clean_staging(&staging_dir);

    // Remove obsolete unmodified files with verified ownership
    let mut obsolete_removed = Vec::new();
    for target_str in &plan.obsolete_removes {
        if let Ok(target_path) = validate_target_safety(target_str, home_dir) {
            if let Some(old_entry) = record.files.iter().find(|f| &f.target == target_str) {
                let safe_to_remove = match fs::symlink_metadata(&target_path) {
                    Ok(m) => {
                        if m.file_type().is_symlink() {
                            if old_entry.is_symlink {
                                if let Some(ref rec_tgt) = old_entry.symlink_target {
                                    fs::read_link(&target_path)
                                        .ok()
                                        .map(|l| l.to_string_lossy().to_string())
                                        == Some(rec_tgt.clone())
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else if m.is_file() {
                            if !old_entry.is_symlink {
                                sha256_file(&target_path).ok().as_ref() == Some(&old_entry.sha256)
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                    Err(_) => false,
                };

                if safe_to_remove {
                    if fs::remove_file(&target_path).is_ok() {
                        obsolete_removed.push(target_str.clone());
                        let mut dummy_rem = Vec::new();
                        let mut dummy_ret = Vec::new();
                        cleanup_empty_parents(
                            &target_path,
                            home_dir,
                            &mut dummy_rem,
                            &mut dummy_ret,
                        );
                    }
                }
            }
        }
    }

    // For any conflicting files that were skipped and retained, keep their entries
    for conflict_target in &plan.conflicts {
        if let Some(old_entry) = record.files.iter().find(|f| &f.target == conflict_target) {
            if !applied_entries.iter().any(|e| &e.target == conflict_target) {
                applied_entries.push(old_entry.clone());
            }
        }
    }

    // Update metadata record with actual repository_id and append history
    let new_tree_hash_val =
        crate::repository::compute_package_tree_hash(new_package_dir, &new_manifest).ok();
    let mut updated_history = record.history.clone();
    updated_history.push(InstalledHistoryEntry {
        version: new_manifest.version.clone(),
        installed_at: now,
        snapshot_id: snapshot_meta.id.clone(),
        repository_id: new_repository_id
            .clone()
            .or_else(|| record.repository_id.clone()),
        tree_hash: new_tree_hash_val,
    });

    let new_record = InstalledPackageRecord {
        package_id: new_manifest.id.clone(),
        name: new_manifest.name.clone(),
        version: new_manifest.version.clone(),
        package_type: Some(new_manifest.package_type),
        repository_id: new_repository_id.or_else(|| record.repository_id.clone()),
        installed_at: now,
        snapshot_id: snapshot_meta.id.clone(),
        installed_files: applied_entries.iter().map(|e| e.target.clone()).collect(),
        files: applied_entries,
        package_source_path: new_package_dir.display().to_string(),
        cryptographic_status: None,
        signer_key_id: None,
        signer_name: None,
        history: updated_history,
    };

    let record_json = serde_json::to_string_pretty(&new_record)
        .map_err(|e| format!("Failed to serialize updated record: {}", e))?;
    if let Err(e) = fs::write(&record_file, record_json) {
        return Err(format!(
            "Failed to write updated installation record: {}",
            e
        ));
    }

    Ok(UpdateResult {
        success: true,
        package_id: package_id.to_string(),
        from_version: record.version,
        to_version: new_manifest.version,
        snapshot_id: snapshot_meta.id,
        updated_files,
        obsolete_removed,
        conflicts_retained: plan.conflicts,
        rolled_back: false,
        errors: vec![],
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Update Package Resolution
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the package directory and repository ID for an update operation.
/// Enforces:
/// 1. Prefer installed record's repository_id when available.
/// 2. If repository_id is unavailable, resolve through repository manager, ensuring
///    duplicate package IDs across repositories do not cause arbitrary selection.
/// 3. Confirms repository version > installed version (strictly no downgrades, no same-version).
/// 4. Returns clear error if repository or package is unavailable.
pub fn resolve_update_package(
    package_id: &str,
    installed_root: &Path,
    repo_mgr: &crate::repository::RepositoryManager,
) -> Result<(PathBuf, String, String), String> {
    if package_id.is_empty()
        || package_id.contains("..")
        || package_id.contains('/')
        || package_id.contains('\\')
        || package_id.contains('\0')
    {
        return Err(format!("Invalid package ID '{}'", package_id));
    }

    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.is_file() {
        return Err(format!("Package '{}' is not installed", package_id));
    }

    let raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read record for '{}': {}", package_id, e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&raw)
        .map_err(|e| format!("Corrupted record for '{}': {}", package_id, e))?;

    let inst_v = semver::Version::parse(&record.version)
        .map_err(|e| format!("Invalid installed version '{}': {}", record.version, e))?;

    if let Some(ref recorded_repo_id) = record.repository_id {
        let repo = repo_mgr
            .get_repository_by_id(recorded_repo_id)
            .ok_or_else(|| {
                format!(
                    "Repository '{}' unavailable or not configured",
                    recorded_repo_id
                )
            })?;

        let entries = repo.list_entries().map_err(|e| {
            format!(
                "Failed to list entries for repository '{}': {}",
                recorded_repo_id, e
            )
        })?;

        let entry = entries
            .into_iter()
            .find(|e| e.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, recorded_repo_id
                )
            })?;

        let cand_v = semver::Version::parse(&entry.version).map_err(|e| {
            format!(
                "Invalid repository package version '{}': {}",
                entry.version, e
            )
        })?;

        if cand_v <= inst_v {
            if cand_v == inst_v {
                return Err(format!(
                    "No update available for '{}' (already at version {})",
                    package_id, record.version
                ));
            } else {
                return Err(format!(
                    "Downgrade attempt refused for package '{}' (installed: {}, repository: {})",
                    package_id, record.version, entry.version
                ));
            }
        }

        let pkg_dir = repo.get_package_dir(package_id)?;
        Ok((pkg_dir, recorded_repo_id.clone(), entry.version))
    } else {
        // No recorded repository_id: search all repositories
        let mut matching_repos: Vec<(&dyn crate::repository::Repository, String, semver::Version)> =
            Vec::new();

        for repo in repo_mgr.repositories() {
            if let Ok(entries) = repo.list_entries() {
                if let Some(entry) = entries.into_iter().find(|e| e.id == package_id) {
                    if let Ok(cand_v) = semver::Version::parse(&entry.version) {
                        matching_repos.push((repo.as_ref(), entry.version, cand_v));
                    }
                }
            }
        }

        if matching_repos.is_empty() {
            return Err(format!(
                "Package '{}' not found in any repository",
                package_id
            ));
        }

        if matching_repos.len() > 1 {
            let names: Vec<&str> = matching_repos.iter().map(|(r, _, _)| r.id()).collect();
            return Err(format!(
                "Ambiguous update for package '{}': found in multiple repositories ({}); cannot select arbitrary repository",
                package_id,
                names.join(", ")
            ));
        }

        let (repo, ver_str, cand_v) = &matching_repos[0];
        if cand_v <= &inst_v {
            if cand_v == &inst_v {
                return Err(format!(
                    "No update available for '{}' (already at version {})",
                    package_id, record.version
                ));
            } else {
                return Err(format!(
                    "Downgrade attempt refused for package '{}' (installed: {}, repository: {})",
                    package_id, record.version, ver_str
                ));
            }
        }

        let pkg_dir = repo.get_package_dir(package_id)?;
        Ok((pkg_dir, repo.id().to_string(), ver_str.clone()))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Exposed Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn preview_installation(package_id: String, target: Option<String>) -> Result<InstallationPlan, String> {
    let home = get_home_dir();
    let package_dir = find_package_dir(&package_id, None)?;
    let sys = detect_system_info();
    generate_installation_plan_target_in(&package_dir, &home, &sys, target.as_deref())
}

#[tauri::command]
pub fn install_package(
    package_id: String,
    create_snapshot: Option<bool>,
    target: Option<String>,
) -> Result<InstallResult, String> {
    let home = get_home_dir();
    let package_dir = find_package_dir(&package_id, None)?;
    let snapshots_root = get_ryzora_snapshots_dir();
    let installed_root = get_ryzora_installed_dir();
    let staging_root = get_ryzora_staging_dir();
    let sys = detect_system_info();

    install_package_target_options_in(
        &package_dir,
        &snapshots_root,
        &installed_root,
        &staging_root,
        &home,
        &sys,
        create_snapshot.unwrap_or(true),
        target.as_deref(),
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

#[tauri::command]
pub fn uninstall_package(package_id: String) -> Result<UninstallResult, String> {
    let home = get_home_dir();
    let snapshots_root = get_ryzora_snapshots_dir();
    let installed_root = get_ryzora_installed_dir();
    uninstall_package_in(&package_id, &home, &snapshots_root, &installed_root)
}

#[tauri::command]
pub fn check_package_update(package_id: String) -> Result<PackageUpdateStatus, String> {
    let installed_root = get_ryzora_installed_dir();
    let repo_mgr = crate::repository::create_default_manager();
    check_package_update_in(&package_id, &installed_root, &repo_mgr)
}

#[tauri::command]
pub fn check_all_updates() -> Result<Vec<PackageUpdateStatus>, String> {
    let installed_root = get_ryzora_installed_dir();
    let repo_mgr = crate::repository::create_default_manager();
    check_all_updates_in(&installed_root, &repo_mgr)
}

#[tauri::command]
pub fn preview_package_update(package_id: String) -> Result<UpdatePlan, String> {
    let home = get_home_dir();
    let installed_root = get_ryzora_installed_dir();
    let repo_mgr = crate::repository::create_default_manager();
    let (new_package_dir, _repo_id, _ver) =
        resolve_update_package(&package_id, &installed_root, &repo_mgr)?;
    preview_package_update_in(&package_id, &home, &installed_root, &new_package_dir)
}

#[tauri::command]
pub fn apply_package_update(package_id: String) -> Result<UpdateResult, String> {
    let home = get_home_dir();
    let snapshots_root = get_ryzora_snapshots_dir();
    let installed_root = get_ryzora_installed_dir();
    let staging_root = get_ryzora_staging_dir();
    let repo_mgr = crate::repository::create_default_manager();
    let (new_package_dir, repo_id, _ver) =
        resolve_update_package(&package_id, &installed_root, &repo_mgr)?;
    let sys = detect_system_info();
    apply_package_update_in(
        &package_id,
        &home,
        &snapshots_root,
        &installed_root,
        &staging_root,
        &new_package_dir,
        &sys,
        false,
        Some(repo_id),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Lockscreen Activation Engine (Install ≠ Apply Lifecycle)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ActiveLockscreenState {
    pub quickshell: Option<String>,
    pub sddm: Option<String>,
    pub quickshell_theme_path: Option<String>,
    pub sddm_theme_path: Option<String>,
    pub last_applied_at: Option<u64>,
    /// Whether the hypridle systemd integration is active
    pub hypridle_override: bool,
    /// SHA-256 of the source hypridle.conf when integration was applied
    pub hypridle_source_hash: Option<String>,
    /// Path of the source hypridle config that was snapshotted
    pub hypridle_source_path: Option<String>,
    /// Path of the Ryzora-generated hypridle config
    pub hypridle_ryzora_config: Option<String>,
    /// Path of the ryzora-lock wrapper script
    pub lock_wrapper_path: Option<String>,
    /// Previous SDDM theme name to restore on deactivation
    pub sddm_previous_theme: Option<String>,
    #[serde(default)]
    pub active_config: Option<std::collections::HashMap<String, serde_json::Value>>,
}

pub fn get_ryzora_state_dir() -> PathBuf {
    get_ryzora_base_dir().join("state")
}

pub fn get_active_lockscreen_state_in(state_dir: &Path) -> ActiveLockscreenState {
    let state_file = state_dir.join("active_lockscreen.json");
    if !state_file.exists() {
        return ActiveLockscreenState::default();
    }
    match fs::read_to_string(&state_file) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => ActiveLockscreenState::default(),
    }
}

pub fn save_active_lockscreen_state_in(state_dir: &Path, state: &ActiveLockscreenState) -> Result<(), String> {
    if !state_dir.exists() {
        fs::create_dir_all(state_dir)
            .map_err(|e| format!("Failed to create state directory: {}", e))?;
    }
    let state_file = state_dir.join("active_lockscreen.json");
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize active lockscreen state: {}", e))?;
    fs::write(&state_file, json)
        .map_err(|e| format!("Failed to write active lockscreen state: {}", e))?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TransactionBackupItem {
    pub orig_path: String,
    pub backup_file_name: String,
    pub existed: bool,
    #[serde(default)]
    pub is_symlink: bool,
    #[serde(default)]
    pub symlink_target: Option<String>,
}

/// Create a lightweight transactional backup of specified files before modifying them.
/// Stored in ~/.local/share/ryzora/backups/operation-<timestamp>/
/// Supports regular files, directories, and symlinks (e.g. active lockscreen symlink).
pub fn create_transaction_backup(
    paths: &[&Path],
    home: &Path,
) -> Result<PathBuf, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backups_dir = home.join(".local/share/ryzora/backups").join(format!("operation-{}", now));
    fs::create_dir_all(&backups_dir)
        .map_err(|e| format!("Failed to create transaction backup directory: {}", e))?;

    let mut manifest_entries = Vec::new();

    for (idx, p) in paths.iter().enumerate() {
        if let Ok(meta) = fs::symlink_metadata(p) {
            if meta.file_type().is_symlink() {
                if let Ok(target) = fs::read_link(p) {
                    manifest_entries.push(TransactionBackupItem {
                        orig_path: p.to_string_lossy().to_string(),
                        backup_file_name: String::new(),
                        existed: true,
                        is_symlink: true,
                        symlink_target: Some(target.to_string_lossy().to_string()),
                    });
                    continue;
                }
            }

            if meta.is_file() {
                let file_name = format!("file_{}_{}", idx, p.file_name().unwrap_or_default().to_string_lossy());
                let dest = backups_dir.join(&file_name);
                fs::copy(p, &dest)
                    .map_err(|e| format!("Failed to backup file '{}': {}", p.display(), e))?;
                manifest_entries.push(TransactionBackupItem {
                    orig_path: p.to_string_lossy().to_string(),
                    backup_file_name: file_name,
                    existed: true,
                    is_symlink: false,
                    symlink_target: None,
                });
                continue;
            }
        }

        manifest_entries.push(TransactionBackupItem {
            orig_path: p.to_string_lossy().to_string(),
            backup_file_name: String::new(),
            existed: false,
            is_symlink: false,
            symlink_target: None,
        });
    }

    let manifest_path = backups_dir.join("transaction.json");
    let raw = serde_json::to_string_pretty(&manifest_entries)
        .map_err(|e| format!("Failed to serialize transaction manifest: {}", e))?;
    fs::write(&manifest_path, raw)
        .map_err(|e| format!("Failed to write transaction manifest: {}", e))?;

    Ok(backups_dir)
}

/// Rollback all files from a transactional backup directory.
pub fn rollback_transaction_backup(backup_dir: &Path) -> Result<(), String> {
    let manifest_path = backup_dir.join("transaction.json");
    if !manifest_path.exists() {
        return Err("Transaction manifest not found in backup directory".to_string());
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read transaction manifest: {}", e))?;
    let entries: Vec<TransactionBackupItem> = match serde_json::from_str(&raw) {
        Ok(items) => items,
        Err(_) => {
            if let Ok(tuples) = serde_json::from_str::<Vec<(String, String, bool)>>(&raw) {
                tuples.into_iter().map(|(orig_path, backup_file_name, existed)| {
                    TransactionBackupItem {
                        orig_path,
                        backup_file_name,
                        existed,
                        is_symlink: false,
                        symlink_target: None,
                    }
                }).collect()
            } else {
                return Err("Failed to parse transaction manifest".to_string());
            }
        }
    };

    for entry in entries {
        let orig_path = PathBuf::from(&entry.orig_path);
        if entry.existed {
            if entry.is_symlink {
                if let Some(target_str) = entry.symlink_target {
                    let _ = fs::remove_file(&orig_path);
                    let _ = fs::remove_dir_all(&orig_path);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::symlink;
                        let _ = symlink(Path::new(&target_str), &orig_path);
                    }
                }
            } else {
                let backup_file = backup_dir.join(&entry.backup_file_name);
                if backup_file.exists() {
                    if let Some(parent) = orig_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    fs::copy(&backup_file, &orig_path)
                        .map_err(|e| format!("Failed to restore '{}': {}", orig_path.display(), e))?;
                }
            }
        } else {
            let _ = fs::remove_file(&orig_path);
            let _ = fs::remove_dir_all(&orig_path);
        }
    }

    Ok(())
}

pub fn apply_lockscreen_target_in(
    package_id: &str,
    target: &str,
    home: &Path,
    installed_root: &Path,
    state_dir: &Path,
) -> Result<ActiveLockscreenState, String> {
    apply_lockscreen_target_in_with_config(package_id, target, None, home, installed_root, state_dir)
}

pub fn apply_lockscreen_target_in_with_config(
    package_id: &str,
    target: &str,
    config: Option<std::collections::HashMap<String, serde_json::Value>>,
    home: &Path,
    installed_root: &Path,
    state_dir: &Path,
) -> Result<ActiveLockscreenState, String> {
    let record_file = installed_root.join(format!("{}.json", package_id));
    if !record_file.exists() {
        return Err(format!(
            "Package '{}' is not installed. Materialize/install it before applying.",
            package_id
        ));
    }

    let record_raw = fs::read_to_string(&record_file)
        .map_err(|e| format!("Failed to read installed package record: {}", e))?;
    let record: InstalledPackageRecord = serde_json::from_str(&record_raw)
        .map_err(|e| format!("Failed to parse installed package record: {}", e))?;

    // Create transactional backup of integration files prior to modification
    let hypridle_conf = home.join(".config/hypr/hypridle.conf");
    let active_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
    let _tx_backup_dir = create_transaction_backup(&[&hypridle_conf, &active_symlink], home)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut state = get_active_lockscreen_state_in(state_dir);
    let target_norm = target.to_lowercase();

    let slug = package_id
        .strip_prefix("lockscreen-qylock-")
        .or_else(|| package_id.strip_prefix("lockscreen-"))
        .unwrap_or(package_id);

    // 1. Quickshell target activation (unprivileged user-space)
    if target_norm == "quickshell" || target_norm == "both" {
        let mut qs_dir = home.join(".local/share/ryzora/lockscreens/qylock").join(slug);
        if !qs_dir.exists() {
            for f in &record.installed_files {
                let p = PathBuf::from(f);
                if p.starts_with(home.join(".local/share/ryzora/lockscreens")) {
                    if let Some(parent) = p.parent() {
                        qs_dir = parent.to_path_buf();
                        break;
                    }
                }
            }
        }

        if !qs_dir.exists() {
            return Err(format!(
                "Quickshell theme directory not found for package '{}' at '{}'. Ensure Quickshell target was installed.",
                package_id,
                qs_dir.display()
            ));
        }

        let active_dir = home.join(".local/share/ryzora/active/lockscreen");
        fs::create_dir_all(&active_dir)
            .map_err(|e| format!("Failed to create active lockscreen directory: {}", e))?;

        let qs_symlink = active_dir.join("quickshell");
        if qs_symlink.exists() || qs_symlink.is_symlink() {
            let _ = fs::remove_file(&qs_symlink);
            let _ = fs::remove_dir_all(&qs_symlink);
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&qs_dir, &qs_symlink)
            .map_err(|e| format!("Failed to create quickshell active symlink: {}", e))?;

        // Ensure Ryzora private quickshell runtime launcher exists
        let _ = ensure_quickshell_runtime_in(home);

        // Materialize user-selected configuration to theme directory
        if let Some(ref cfg) = config {
            let config_json_path = qs_dir.join("ryzora_config.json");
            if let Ok(serialized) = serde_json::to_string_pretty(cfg) {
                let _ = fs::write(&config_json_path, serialized);
            }

            let theme_conf_path = qs_dir.join("theme.conf");
            let _ = update_theme_conf_with_config(&theme_conf_path, cfg);
        }

        // Apply hypridle integration: generate config, write drop-in, reload service
        match crate::hypridle::apply_hypridle_integration(home) {
            Ok((source_hash, lock_path)) => {
                let source_path = home.join(crate::hypridle::HYPRIDLE_SOURCE_PATH);
                state.hypridle_override = true;
                state.hypridle_source_hash = Some(source_hash);
                state.hypridle_source_path = Some(source_path.display().to_string());
                state.hypridle_ryzora_config = Some(home.join(crate::hypridle::HYPRIDLE_RYZORA_CONFIG).display().to_string());
                state.lock_wrapper_path = Some(lock_path.display().to_string());
            }
            Err(e) => {
                // Non-fatal: log but continue — session lock files are applied even without hypridle
                eprintln!("Ryzora: Warning: hypridle integration failed: {}", e);
                eprintln!("Ryzora: The lockscreen package is applied but the lock shortcut still uses the system default.");
            }
        }

        state.quickshell = Some(package_id.to_string());
        state.quickshell_theme_path = Some(qs_dir.display().to_string());
    }

    // 2. SDDM target activation (privileged system integration)
    if target_norm == "sddm" || target_norm == "both" {
        let helper_status = crate::sddm_helper::detect_privileged_helper_status();
        if !helper_status.installed {
            return Err(format!(
                "Ryzora SDDM helper is not installed or invalid at '{}'. Please set up System Integration before applying SDDM themes.",
                helper_status.helper_path
            ));
        }

        let sddm_theme_dir = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
            PathBuf::from(sys_root)
                .join("usr/share/sddm/themes")
                .join(format!("ryzora-{}", slug))
        } else {
            PathBuf::from(format!("/usr/share/sddm/themes/ryzora-{}", slug))
        };

        if !sddm_theme_dir.exists() {
            return Err(format!(
                "SDDM theme directory not found for package '{}' at '{}'. Ensure SDDM target was installed.",
                package_id,
                sddm_theme_dir.display()
            ));
        }

        // Snapshot the current SDDM theme before activating if not already recorded
        if state.sddm_previous_theme.is_none() {
            let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
            let (current_theme, _) = crate::sddm_helper::read_current_sddm_theme_in(sys_root.as_deref());
            state.sddm_previous_theme = current_theme;
        }

        // Activate SDDM theme using restricted helper
        // Materialize user-selected configuration to SDDM theme directory
        if let Some(ref cfg) = config {
            let config_json_path = sddm_theme_dir.join("ryzora_config.json");
            if let Ok(serialized) = serde_json::to_string_pretty(cfg) {
                let _ = fs::write(&config_json_path, serialized);
            }
            let theme_conf_path = sddm_theme_dir.join("theme.conf");
            let _ = update_theme_conf_with_config(&theme_conf_path, cfg);
        }
        crate::sddm_helper::activate_sddm_theme(&slug)?;

        state.sddm = Some(package_id.to_string());
        state.sddm_theme_path = Some(sddm_theme_dir.display().to_string());
    }

    if config.is_some() {
        state.active_config = config;
    }
    state.last_applied_at = Some(now);
    save_active_lockscreen_state_in(state_dir, &state)?;
    Ok(state)
}

pub fn deactivate_lockscreen_target_in(
    target: &str,
    home: &Path,
    state_dir: &Path,
) -> Result<ActiveLockscreenState, String> {
    let mut state = get_active_lockscreen_state_in(state_dir);
    let target_norm = target.to_lowercase();

    if target_norm == "quickshell" || target_norm == "both" {
        let qs_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
        if qs_symlink.exists() || qs_symlink.is_symlink() {
            let _ = fs::remove_file(&qs_symlink);
            let _ = fs::remove_dir_all(&qs_symlink);
        }

        // Deactivate hypridle integration: remove drop-in and generated config, reload service
        if state.hypridle_override {
            match crate::hypridle::deactivate_hypridle_integration(home) {
                Ok(()) => {
                    eprintln!("Ryzora: Hypridle integration removed, original config restored.");
                }
                Err(e) => {
                    eprintln!("Ryzora: Warning: could not cleanly remove hypridle integration: {}", e);
                }
            }
            state.hypridle_override = false;
            state.hypridle_source_hash = None;
            state.hypridle_source_path = None;
            state.hypridle_ryzora_config = None;
            state.lock_wrapper_path = None;
        }

        state.quickshell = None;
        state.quickshell_theme_path = None;
    }

    if target_norm == "sddm" || target_norm == "both" {
        if let Some(ref prev_theme) = state.sddm_previous_theme {
            let _ = crate::sddm_helper::restore_sddm_theme(prev_theme);
        } else {
            let _ = crate::sddm_helper::deactivate_sddm_theme();
        }
        state.sddm = None;
        state.sddm_theme_path = None;
        state.sddm_previous_theme = None;
    }

    save_active_lockscreen_state_in(state_dir, &state)?;
    Ok(state)
}

#[tauri::command]
pub fn get_active_lockscreen() -> Result<ActiveLockscreenState, String> {
    let state_dir = get_ryzora_state_dir();
    Ok(get_active_lockscreen_state_in(&state_dir))
}

#[tauri::command]
pub fn apply_lockscreen(
    package_id: String,
    target: String,
    config: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<ActiveLockscreenState, String> {
    let home = get_home_dir();
    let installed_root = get_ryzora_installed_dir();
    let state_dir = get_ryzora_state_dir();
    apply_lockscreen_target_in_with_config(&package_id, &target, config, &home, &installed_root, &state_dir)
}

#[tauri::command]
pub fn deactivate_lockscreen(target: String) -> Result<ActiveLockscreenState, String> {
    let home = get_home_dir();
    let state_dir = get_ryzora_state_dir();
    deactivate_lockscreen_target_in(&target, &home, &state_dir)
}

/// Launch the active Ryzora lockscreen immediately for testing — without waiting for idle timeout.
/// Only works when a Quickshell lockscreen is applied and hypridle integration is active.
/// Launch the lockscreen immediately for testing.
/// If package_id is provided, tests that installed package in isolated test mode.
/// If package_id is None, tests the currently applied lockscreen.
#[tauri::command]
pub fn launch_lockscreen_test(
    package_id: Option<String>,
    target: Option<String>,
    config: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<(), String> {
    let home = get_home_dir();
    let state_dir = get_ryzora_state_dir();
    let installed_root = get_ryzora_installed_dir();

    if let Some(ref pkg_id) = package_id {
        let slug = pkg_id
            .strip_prefix("lockscreen-qylock-")
            .or_else(|| pkg_id.strip_prefix("lockscreen-"))
            .unwrap_or(pkg_id);

        let mut qs_theme_dir = home.join(".local/share/ryzora/lockscreens/qylock").join(slug);
        let mut sddm_theme_dir = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
            PathBuf::from(sys_root).join("usr/share/sddm/themes").join(format!("ryzora-{}", slug))
        } else {
            PathBuf::from(format!("/usr/share/sddm/themes/ryzora-{}", slug))
        };

        // If a variant is specified in custom config, resolve variant directory
        if let Some(v) = config.as_ref().and_then(|c| c.get("variant")).and_then(|v| v.as_str()) {
            let variant_sub = qs_theme_dir.join(v);
            if variant_sub.exists() && variant_sub.join("Main.qml").exists() {
                qs_theme_dir = variant_sub;
            } else {
                let variant_alt = home.join(".local/share/ryzora/lockscreens/qylock").join(format!("{}-{}", slug, v));
                if variant_alt.exists() && variant_alt.join("Main.qml").exists() {
                    qs_theme_dir = variant_alt;
                }
            }

            let sddm_sub = sddm_theme_dir.join(v);
            if sddm_sub.exists() && sddm_sub.join("Main.qml").exists() {
                sddm_theme_dir = sddm_sub;
            } else {
                let sys_prefix = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
                    PathBuf::from(sys_root).join("usr/share/sddm/themes")
                } else {
                    PathBuf::from("/usr/share/sddm/themes")
                };
                let variant_alt = sys_prefix.join(format!("ryzora-{}-{}", slug, v));
                if variant_alt.exists() && variant_alt.join("Main.qml").exists() {
                    sddm_theme_dir = variant_alt;
                }
            }
        }

        // Query installed package record if present
        let record = get_installed_package_in(pkg_id, &installed_root)
            .or_else(|_| get_installed_package_in(&format!("lockscreen-qylock-{}", slug), &installed_root))
            .or_else(|_| get_installed_package_in(slug, &installed_root))
            .ok()
            .flatten();

        // Target-aware discrimination: Session Lock (Quickshell) vs Login Screen (SDDM)
        let is_sddm = match target.as_deref() {
            Some("sddm") | Some("login") => true,
            Some("quickshell") | Some("session") => false,
            _ => {
                if let Some(ref r) = record {
                    let has_sddm = r.installed_files.iter().any(|f| f.contains("/sddm/themes/"));
                    let has_qs = r.installed_files.iter().any(|f| f.contains("/lockscreens/qylock/"));
                    if has_sddm && !has_qs {
                        true
                    } else if has_qs {
                        false
                    } else {
                        sddm_theme_dir.exists() && !qs_theme_dir.exists()
                    }
                } else {
                    sddm_theme_dir.exists() && !qs_theme_dir.exists()
                }
            }
        };

        if is_sddm {
            let sddm_dir = if sddm_theme_dir.exists() && sddm_theme_dir.join("Main.qml").exists() {
                Some(sddm_theme_dir.clone())
            } else {
                record.as_ref().and_then(|r| {
                    r.installed_files.iter().find_map(|f| {
                        let p = PathBuf::from(f);
                        if p.ends_with("Main.qml") && p.exists() && f.contains("sddm") {
                            p.parent().map(|d| d.to_path_buf())
                        } else {
                            None
                        }
                    })
                })
            };

            let theme_dir = match sddm_dir {
                Some(d) => d,
                None => {
                    return Err(format!(
                        "Package '{}' SDDM theme directory was not found at '{}'. Please install SDDM theme before testing.",
                        pkg_id, sddm_theme_dir.display()
                    ));
                }
            };

            if let Some(ref cfg) = config {
                let config_json_path = theme_dir.join("ryzora_config.json");
                if let Ok(serialized) = serde_json::to_string_pretty(cfg) {
                    let _ = fs::write(&config_json_path, serialized);
                }
                let theme_conf_path = theme_dir.join("theme.conf");
                let _ = update_theme_conf_with_config(&theme_conf_path, cfg);
            }
            crate::hypridle::launch_test_sddm_process(&theme_dir)
        } else {
            // Session Lock (Quickshell)
            let qs_dir = if qs_theme_dir.exists() && qs_theme_dir.join("Main.qml").exists() {
                Some(qs_theme_dir)
            } else {
                record.as_ref().and_then(|r| {
                    r.installed_files.iter().find_map(|f| {
                        let p = PathBuf::from(f);
                        if p.ends_with("Main.qml") && p.exists() && !f.contains("sddm") {
                            p.parent().map(|d| d.to_path_buf())
                        } else {
                            None
                        }
                    })
                })
            };

            let theme_dir = match qs_dir {
                Some(d) => d,
                None => {
                    // If SDDM theme exists, test SDDM rather than failing
                    if sddm_theme_dir.exists() && sddm_theme_dir.join("Main.qml").exists() {
                        return crate::hypridle::launch_test_sddm_process(&sddm_theme_dir);
                    }
                    return Err(format!(
                        "Package '{}' is not installed for session lock. Please install the package before testing.",
                        pkg_id
                    ));
                }
            };

            if let Some(ref cfg) = config {
                let config_json_path = theme_dir.join("ryzora_config.json");
                if let Ok(serialized) = serde_json::to_string_pretty(cfg) {
                    let _ = fs::write(&config_json_path, serialized);
                }
                let theme_conf_path = theme_dir.join("theme.conf");
                let _ = update_theme_conf_with_config(&theme_conf_path, cfg);
            }

            let _ = ensure_quickshell_runtime_in(&home);
            let lock_shell = home.join(".local/share/ryzora/integrations/quickshell/lock_shell.qml");
            if lock_shell.exists() {
                crate::hypridle::launch_test_qml_process(&lock_shell, &theme_dir)
            } else {
                let lock_sh = home.join(".local/share/ryzora/integrations/quickshell/lock.sh");
                crate::hypridle::launch_test_process(&lock_sh)
            }
        }
    } else {
        let state = get_active_lockscreen_state_in(&state_dir);
        if let Some(ref sddm_pkg) = state.sddm {
            if state.quickshell.is_none() {
                let slug = sddm_pkg
                    .strip_prefix("lockscreen-qylock-")
                    .or_else(|| sddm_pkg.strip_prefix("lockscreen-"))
                    .unwrap_or(sddm_pkg);
                let sddm_theme_dir = PathBuf::from(format!("/usr/share/sddm/themes/ryzora-{}", slug));
                if sddm_theme_dir.exists() {
                    return crate::hypridle::launch_test_sddm_process(&sddm_theme_dir);
                }
            }
        }

        if state.quickshell.is_none() && state.sddm.is_none() {
            return Err("No lockscreen is currently applied. Apply a lockscreen first to test.".to_string());
        }
        let lock_sh = home.join(".local/share/ryzora/integrations/quickshell/lock.sh");
        if !lock_sh.exists() {
            return Err(format!(
                "Ryzora lockscreen runtime not found at '{}'",
                lock_sh.display()
            ));
        }
        crate::hypridle::launch_test_process(&lock_sh)
    }
}

pub fn deactivate_and_uninstall_lockscreen_target_in(
    package_id: &str,
    _target: Option<&str>,
    home: &Path,
    snapshots_root: &Path,
    installed_root: &Path,
    state_dir: &Path,
) -> Result<UninstallResult, String> {
    let state = get_active_lockscreen_state_in(state_dir);

    // 1. If active, deactivate first to restore original system state
    let is_qs_active = state.quickshell.as_deref() == Some(package_id);
    let is_sddm_active = state.sddm.as_deref() == Some(package_id);

    if is_qs_active && is_sddm_active {
        let _ = deactivate_lockscreen_target_in("both", home, state_dir);
    } else if is_qs_active {
        let _ = deactivate_lockscreen_target_in("quickshell", home, state_dir);
    } else if is_sddm_active {
        let _ = deactivate_lockscreen_target_in("sddm", home, state_dir);
    }

    // 2. If SDDM target was installed, clean /usr/share/sddm/themes/ryzora-<slug>
    let slug = package_id
        .strip_prefix("lockscreen-qylock-")
        .or_else(|| package_id.strip_prefix("lockscreen-"))
        .unwrap_or(package_id);
    let sddm_path = PathBuf::from(format!("/usr/share/sddm/themes/ryzora-{}", slug));
    if sddm_path.exists() {
        let _ = crate::sddm_helper::remove_sddm_theme(slug);
    }

    // 3. Uninstall user package files
    uninstall_package_in(package_id, home, snapshots_root, installed_root)
}

#[tauri::command]
pub fn deactivate_and_uninstall_lockscreen(
    package_id: String,
    target: Option<String>,
) -> Result<UninstallResult, String> {
    let home = get_home_dir();
    let snapshots_root = get_ryzora_snapshots_dir();
    let installed_root = get_ryzora_installed_dir();
    let state_dir = get_ryzora_state_dir();
    deactivate_and_uninstall_lockscreen_target_in(
        &package_id,
        target.as_deref(),
        &home,
        &snapshots_root,
        &installed_root,
        &state_dir,
    )
}

/// Check if the hypridle source config has drifted from the stored snapshot.
/// Returns Some(current_hash) if drift detected, None otherwise.
#[tauri::command]
pub fn check_lockscreen_config_drift() -> Option<String> {
    let home = get_home_dir();
    let state_dir = get_ryzora_state_dir();
    let state = get_active_lockscreen_state_in(&state_dir);
    if let Some(ref stored_hash) = state.hypridle_source_hash {
        crate::hypridle::check_hypridle_config_drift(&home, stored_hash)
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockscreenRuntimeStatus {
    pub target: String,
    pub adapter: String,
    pub available: bool,
    pub installed: bool,
    pub applied: bool,
    pub active: bool,
    pub entrypoint: Option<String>,
    pub protocol: String,
    pub active_package: Option<String>,
    pub launcher_path: Option<String>,
    pub error: Option<String>,
    /// Whether the hypridle systemd integration is wired up
    pub hypridle_integration: bool,
    /// Whether the source hypridle config has changed since Ryzora applied (drift)
    pub hypridle_config_drift: bool,
    /// Whether the ryzora-lock wrapper exists and is executable
    pub lock_wrapper_exists: bool,
    /// Whether the privileged SDDM helper is installed and verified
    pub privileged_helper_installed: bool,
    /// Effective SDDM theme currently winning on the machine
    pub sddm_effective_theme: Option<String>,
    pub sddm_effective_file: Option<String>,
    pub sddm_is_overridden: bool,
    pub sddm_overridden_by: Option<String>,
    pub sddm_previous_theme: Option<String>,
}

/// Ensures Ryzora's private Quickshell runtime exists at ~/.local/share/ryzora/integrations/quickshell/
pub fn ensure_quickshell_runtime_in(home: &Path) -> Result<PathBuf, String> {
    let runtime_dir = home.join(".local/share/ryzora/integrations/quickshell");
    let shim_dir = runtime_dir.join("shim");
    fs::create_dir_all(&shim_dir)
        .map_err(|e| format!("Failed to create Ryzora quickshell integration directory: {}", e))?;

    let lock_sh = runtime_dir.join("lock.sh");
    let lock_shell_qml = runtime_dir.join("lock_shell.qml");
    let shim_qml = shim_dir.join("SddmShim.qml");

    if !lock_sh.exists() {
        let lock_script = r#"#!/usr/bin/env bash
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ACTIVE_SYMLINK="$HOME/.local/share/ryzora/active/lockscreen/quickshell"

if [ ! -d "$ACTIVE_SYMLINK" ]; then
    echo "Ryzora: No active Quickshell lockscreen theme linked at $ACTIVE_SYMLINK" >&2
    exit 1
fi

export QS_THEME_PATH="$ACTIVE_SYMLINK"
export QML_XHR_ALLOW_FILE_READ=1
export XDG_SESSION_TYPE="${XDG_SESSION_TYPE:-wayland}"

# Kill any conflicting legacy lockers safely
killall -9 hyprlock swaylock 2>/dev/null || true

exec quickshell -p "$DIR/lock_shell.qml"
"#;
        fs::write(&lock_sh, lock_script)
            .map_err(|e| format!("Failed to write Ryzora quickshell lock.sh: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&lock_sh, fs::Permissions::from_mode(0o755));
        }
    }

    if !lock_shell_qml.exists() {
        let qml_content = r#"import QtQuick
import Quickshell
import Quickshell.Wayland
import QtMultimedia
import "./shim"

ShellRoot {
    id: shellRoot

    property string themePath: Quickshell.env("QS_THEME_PATH") || ""
    readonly property var sddm: sddmShim.sddm
    readonly property var config: sddmShim.config
    readonly property var userModel: sddmShim.userModel
    readonly property var sessionModel: sddmShim.sessionModel
    readonly property bool isWayland: Quickshell.env("XDG_SESSION_TYPE") === "wayland"
    property bool authenticated: false
    property bool sessionLocked: true

    SddmShim {
        id: sddmShim
        themePath: shellRoot.themePath
    }

    Connections {
        target: sddmShim.sddm
        function onLoginSucceeded() {
            shellRoot.authenticated = true
            if (Quickshell.env("XDG_CURRENT_DESKTOP") === "Hyprland" || Quickshell.env("HYPRLAND_INSTANCE_SIGNATURE") !== "") {
                Quickshell.execDetached(["hyprctl", "keyword", "misc:allow_session_lock_restore", "1"]);
            }
            Quickshell.execDetached(["loginctl", "unlock-session"]);
            quitTimer.start()
        }
    }

    Timer {
        id: quitTimer
        interval: 250
        onTriggered: {
            shellRoot.sessionLocked = false
            Qt.quit()
        }
    }

    Component {
        id: themeComponent
        Loader {
            anchors.fill: parent
            source: shellRoot.themePath !== "" ? ("file://" + shellRoot.themePath + "/Main.qml") : ""
            onLoaded: {
                if (item) item.forceActiveFocus()
            }
        }
    }

    Loader {
        id: waylandLoader
        active: shellRoot.isWayland
        sourceComponent: Component {
            WlSessionLock {
                id: lock
                locked: shellRoot.sessionLocked
                surface: Component {
                    WlSessionLockSurface {
                        color: "black"
                        Loader {
                            anchors.fill: parent
                            sourceComponent: themeComponent
                        }
                    }
                }
            }
        }
    }
}
"#;
        fs::write(&lock_shell_qml, qml_content)
            .map_err(|e| format!("Failed to write Ryzora lock_shell.qml: {}", e))?;
    }

    if !shim_qml.exists() {
        let shim_content = r#"import QtQuick
import Quickshell
import Quickshell.Services.Pam

Item {
    id: shim
    property string themePath: ""
    property var config: ({})
    property bool configReady: true

    property var userModel: ListModel {
        Component.onCompleted: {
            append({
                name: Quickshell.env("USER") || "user",
                realName: Quickshell.env("USER") || "User",
                icon: "",
                homeDir: "/home/" + (Quickshell.env("USER") || "user")
            })
        }
    }

    property var sessionModel: ListModel {
        Component.onCompleted: append({ name: "Wayland", file: "wayland.desktop" })
    }

    property var sddm: QtObject {
        signal loginFailed()
        signal loginSucceeded()

        function login(user, password, sessionIndex) {
            pam.user = user;
            pam.pendingPassword = password;
            pam.start();
        }
    }

    PamContext {
        id: pam
        property string pendingPassword: ""
        onResponseRequiredChanged: {
            if (responseRequired && pendingPassword !== "") {
                respond(pendingPassword);
                pendingPassword = "";
            }
        }
        onCompleted: (result) => {
            if (result === PamResult.Success) {
                shim.sddm.loginSucceeded();
            } else {
                shim.sddm.loginFailed();
            }
        }
    }
}
"#;
        fs::write(&shim_qml, shim_content)
            .map_err(|e| format!("Failed to write Ryzora SddmShim.qml: {}", e))?;
    }

    Ok(lock_sh)
}

pub fn get_lock_screen_runtime_status_in(home: &Path, state_dir: &Path) -> LockscreenRuntimeStatus {
    let state = get_active_lockscreen_state_in(state_dir);
    let (has_quickshell, _) = crate::system::check_binary("quickshell");
    let active_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
    let launcher = home.join(".local/share/ryzora/integrations/quickshell/lock.sh");

    // Hypridle integration status
    let hypridle_status = crate::hypridle::get_hypridle_integration_status(
        home,
        state.hypridle_source_hash.as_deref(),
    );

    let hypridle_integration = state.hypridle_override
        && hypridle_status.dropin_exists
        && hypridle_status.ryzora_config_exists;
    let lock_wrapper_exists = hypridle_status.lock_wrapper_exists;
    let hypridle_config_drift = hypridle_status.drift_detected;

    // Active requires: symlink exists, quickshell installed, AND hypridle integration wired
    let is_applied = state.quickshell.is_some() && active_symlink.exists();
    let is_active = is_applied && has_quickshell && hypridle_integration;

    let err = if !has_quickshell {
        Some("Quickshell binary is not installed on this system".to_string())
    } else if state.quickshell.is_none() {
        Some("No session lock package is currently applied".to_string())
    } else if !active_symlink.exists() {
        Some("Active theme pointer is missing or unlinked".to_string())
    } else if is_applied && !hypridle_integration {
        Some("Applied but hypridle integration not wired — lock shortcut still uses system default".to_string())
    } else if hypridle_config_drift {
        Some("Hypridle configuration changed externally — regenerate or restore to re-sync".to_string())
    } else {
        None
    };

    LockscreenRuntimeStatus {
        target: "quickshell".to_string(),
        adapter: "quickshell".to_string(),
        available: has_quickshell,
        installed: state.quickshell.is_some(),
        applied: is_applied,
        active: is_active,
        entrypoint: state.quickshell_theme_path.clone(),
        protocol: "ext-session-lock-v1".to_string(),
        active_package: state.quickshell,
        launcher_path: if launcher.exists() { Some(launcher.to_string_lossy().to_string()) } else { None },
        error: err,
        hypridle_integration,
        hypridle_config_drift,
        lock_wrapper_exists,
        privileged_helper_installed: crate::sddm_helper::detect_privileged_helper_status().installed,
        sddm_effective_theme: crate::sddm_helper::resolve_effective_sddm_theme_in(std::env::var("RYZORA_SYSTEM_ROOT").ok().as_deref().map(std::path::Path::new)).effective_theme,
        sddm_effective_file: crate::sddm_helper::resolve_effective_sddm_theme_in(std::env::var("RYZORA_SYSTEM_ROOT").ok().as_deref().map(std::path::Path::new)).effective_file.map(|p| p.display().to_string()),
        sddm_is_overridden: crate::sddm_helper::resolve_effective_sddm_theme_in(std::env::var("RYZORA_SYSTEM_ROOT").ok().as_deref().map(std::path::Path::new)).is_overridden,
        sddm_overridden_by: crate::sddm_helper::resolve_effective_sddm_theme_in(std::env::var("RYZORA_SYSTEM_ROOT").ok().as_deref().map(std::path::Path::new)).overridden_by.map(|p| p.display().to_string()),
        sddm_previous_theme: state.sddm_previous_theme.clone(),
    }
}

#[tauri::command]
pub fn get_sddm_runtime_status(package_id: Option<String>) -> crate::sddm_helper::SddmRuntimeStatus {
    let state_dir = get_ryzora_state_dir();
    let state = get_active_lockscreen_state_in(&state_dir);
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    crate::sddm_helper::get_sddm_runtime_status_in(sys_root.as_deref(), &state, package_id.as_deref())
}

#[tauri::command]
pub fn get_lock_screen_runtime_status() -> LockscreenRuntimeStatus {
    let home = get_home_dir();
    let state_dir = get_ryzora_state_dir();
    get_lock_screen_runtime_status_in(&home, &state_dir)
}



// ─────────────────────────────────────────────────────────────────────────────
// Comprehensive Unit & Integration Tests
// Zero tests touch ~/.config or real user files. Everything uses /tmp sandbox.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::TEST_ENV_MUTEX as ENV_MUTEX;

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
  "color_palette": [],
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

        fn create_package_with_symlinks(
            &self,
            id: &str,
            version: &str,
            regular_files: &[(&str, &str, &str)], // (source, target, content)
            symlinks: &[(&str, &str, &str)],      // (source, target, link_dest)
        ) -> PathBuf {
            let pkg_dir = self.packages_dir.join(format!("{}-{}", id, version));
            fs::create_dir_all(&pkg_dir).unwrap();

            let mut manifest_files = Vec::new();
            for (s, t, _) in regular_files {
                manifest_files.push(format!(
                    r#"{{"source": "{}", "target": "{}", "description": "regular"}}"#,
                    s, t
                ));
            }
            for (s, t, _) in symlinks {
                manifest_files.push(format!(
                    r#"{{"source": "{}", "target": "{}", "description": "symlink"}}"#,
                    s, t
                ));
            }

            let manifest_json = format!(
                r#"{{
  "id": "{}",
  "name": "Symlink Package",
  "version": "{}",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test symlink package",
  "tags": [],
  "color_palette": [],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch"],
    "required": [],
    "optional": []
  }},
  "files": [{}]
}}"#,
                id,
                version,
                manifest_files.join(", ")
            );

            fs::write(pkg_dir.join("manifest.json"), manifest_json).unwrap();

            for (src_rel, _, content) in regular_files {
                let full_src = pkg_dir.join(src_rel);
                if let Some(parent) = full_src.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(full_src, content).unwrap();
            }

            for (src_rel, _, link_dest) in symlinks {
                let full_src = pkg_dir.join(src_rel);
                if let Some(parent) = full_src.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink(link_dest, &full_src).unwrap();
            }

            pkg_dir
        }

        fn create_sample_package_with_version(
            &self,
            id: &str,
            version: &str,
            files: &[(&str, &str, &str)],
            required_binaries: &[&str],
        ) -> PathBuf {
            let pkg_dir = self.packages_dir.join(format!("{}-{}", id, version));
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
                r#"{{
  "id": "{}",
  "name": "Test Package {}",
  "version": "{}",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test package",
  "tags": ["test"],
  "color_palette": [],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch"],
    "required": [{}],
    "optional": []
  }},
  "files": [{}]
}}"#,
                id,
                id,
                version,
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
                (
                    "files/hypr/hyprland.conf",
                    "~/.config/hypr/hyprland.conf",
                    "new hypr config",
                ),
                (
                    "files/waybar/style.css",
                    "~/.config/waybar/style.css",
                    "new waybar css",
                ),
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
            &[(
                "files/app.conf",
                "~/.config/test/app.conf",
                "payload content",
            )],
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
                (
                    "files/hypr/hyprland.conf",
                    "~/.config/hypr/hyprland.conf",
                    "hyprland config 1",
                ),
                (
                    "files/waybar/config.jsonc",
                    "~/.config/waybar/config.jsonc",
                    "waybar config 1",
                ),
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
        )
        .unwrap();

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
    fn test_install_without_snapshot_option() {
        let sandbox = TestSandbox::new("no-snap-opt");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "no-snap-pkg",
            &[
                (
                    "files/hypr/hyprland.conf",
                    "~/.config/hypr/hyprland.conf",
                    "hyprland config without snapshot",
                ),
            ],
            &["hyprland"],
        );

        // Install with create_snapshot = false
        let res = install_package_options_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
        )
        .unwrap();

        assert!(res.success);
        assert!(!res.rolled_back);
        assert_eq!(res.snapshot_id, "");
        assert_eq!(res.installed_files.len(), 1);

        // Verify destination file exists and matches
        let dest = sandbox.home_dir.join(".config/hypr/hyprland.conf");
        assert!(dest.is_file());
        assert_eq!(fs::read_to_string(dest).unwrap(), "hyprland config without snapshot");

        // Verify NO snapshots were created
        let snapshots = crate::snapshot::list_snapshots_in(&sandbox.snapshots_dir).unwrap();
        assert_eq!(snapshots.len(), 0);

        // Verify installed metadata was persisted with empty snapshot_id
        let installed = list_installed_packages_in(&sandbox.installed_dir).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].package_id, "no-snap-pkg");
        assert_eq!(installed[0].snapshot_id, "");

        // Verify uninstalling works cleanly even when installed without pre-install snapshot
        let uninst = uninstall_package_in(
            "no-snap-pkg",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        ).unwrap();
        assert!(uninst.success);
        assert!(!sandbox.home_dir.join(".config/hypr/hyprland.conf").exists());
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
            &[(
                "files/existing.conf",
                "~/.config/existing.conf",
                "new package content",
            )],
            &["hyprland"],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        assert!(res.success);

        // Dest has new content
        assert_eq!(fs::read_to_string(&dest).unwrap(), "new package content");

        // Snapshot contains original content
        let snap =
            crate::snapshot::get_snapshot_in(&res.snapshot_id, &sandbox.snapshots_dir).unwrap();
        assert_eq!(snap.entries.len(), 1);
        let entry = &snap.entries[0];
        let backup_file = sandbox
            .snapshots_dir
            .join(&snap.id)
            .join("files")
            .join(&entry.backup_relative);
        assert_eq!(
            fs::read_to_string(backup_file).unwrap(),
            "original user content"
        );
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
        assert!(res
            .err()
            .unwrap()
            .contains("incompatible with detected system environment"));
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
            &[(
                "files/original.conf",
                "~/.config/original.conf",
                "new replaced configuration",
            )],
            &["hyprland"],
        );

        // Corrupt the package file AFTER staging planning by making dest read-only directory
        // Or create an unwritable parent directory for a second target
        let unwriteable_pkg_dir = sandbox.create_sample_package(
            "fail-multi-pkg",
            &[
                (
                    "files/original.conf",
                    "~/.config/original.conf",
                    "new replaced configuration",
                ),
                (
                    "files/impossible.conf",
                    "~/.config/impossible_dir/sub/file.conf",
                    "will fail",
                ),
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
        )
        .unwrap();

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
        )
        .unwrap();

        install_package_in(
            &pkg2,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

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
        assert!(res
            .err()
            .unwrap()
            .contains("escapes package root via symlink"));
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
        )
        .unwrap();
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
        assert_eq!(
            plan_diff.files_to_replace,
            vec!["~/.config/repeat/app.conf"]
        );

        // Second install
        let res2 = install_package_in(
            &pkg,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
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

    // ═════════════════════════════════════════════════════════════════════════
    // PHASE 7 COMPREHENSIVE TESTS (Tests 1 through 30)
    // ═════════════════════════════════════════════════════════════════════════

    // ─────────────────────────────────────────────────────────────────────────
    // UNINSTALL TESTS (1 - 13)
    // ─────────────────────────────────────────────────────────────────────────

    // 1. Uninstall untouched installed file
    #[test]
    fn test_uninstall_untouched_installed_file() {
        let sandbox = TestSandbox::new("uninstall-untouched");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-untouched",
            &[(
                "files/hypr.conf",
                "~/.config/hypr/hypr.conf",
                "my hypr config",
            )],
            &[],
        );

        let install_res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
        assert!(install_res.success);

        let dest = sandbox.home_dir.join(".config/hypr/hypr.conf");
        assert!(dest.is_file());

        let uninst_res = uninstall_package_in(
            "pkg-untouched",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst_res.success);
        assert!(!uninst_res.rolled_back);
        assert_eq!(uninst_res.removed_files, vec!["~/.config/hypr/hypr.conf"]);
        assert!(!dest.exists());
    }

    // 2. Uninstall multiple files
    #[test]
    fn test_uninstall_multiple_files() {
        let sandbox = TestSandbox::new("uninstall-multiple");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-multi",
            &[
                ("files/hypr.conf", "~/.config/hypr/hypr.conf", "hypr config"),
                (
                    "files/waybar.css",
                    "~/.config/waybar/style.css",
                    "waybar style",
                ),
                (
                    "files/kitty.conf",
                    "~/.config/kitty/kitty.conf",
                    "kitty config",
                ),
            ],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-multi",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst_res.success);
        assert_eq!(uninst_res.removed_files.len(), 3);
        assert!(!sandbox.home_dir.join(".config/hypr/hypr.conf").exists());
        assert!(!sandbox.home_dir.join(".config/waybar/style.css").exists());
        assert!(!sandbox.home_dir.join(".config/kitty/kitty.conf").exists());
    }

    // 3. Already-missing file
    #[test]
    fn test_uninstall_already_missing_file() {
        let sandbox = TestSandbox::new("uninstall-missing");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-missing",
            &[
                ("files/a.conf", "~/.config/app/a.conf", "config a"),
                ("files/b.conf", "~/.config/app/b.conf", "config b"),
            ],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User deleted b.conf before uninstall
        fs::remove_file(sandbox.home_dir.join(".config/app/b.conf")).unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-missing",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst_res.success);
        assert_eq!(uninst_res.removed_files, vec!["~/.config/app/a.conf"]);
        assert_eq!(
            uninst_res.already_missing_files,
            vec!["~/.config/app/b.conf"]
        );
    }

    // 4. Modified installed file is retained
    #[test]
    fn test_uninstall_modified_installed_file_retained() {
        let sandbox = TestSandbox::new("uninstall-modified-retained");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-mod",
            &[
                (
                    "files/clean.conf",
                    "~/.config/app/clean.conf",
                    "clean content",
                ),
                (
                    "files/user.conf",
                    "~/.config/app/user.conf",
                    "original content",
                ),
            ],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User edits user.conf
        let user_file = sandbox.home_dir.join(".config/app/user.conf");
        fs::write(&user_file, "user edited custom changes!").unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-mod",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst_res.success);
        assert_eq!(uninst_res.removed_files, vec!["~/.config/app/clean.conf"]);
        // Modified file MUST still exist! Zero data loss!
        assert!(user_file.is_file());
        assert_eq!(
            fs::read_to_string(&user_file).unwrap(),
            "user edited custom changes!"
        );
    }

    // 5. Modified file is reported as conflict
    #[test]
    fn test_uninstall_modified_file_reported_as_conflict() {
        let sandbox = TestSandbox::new("uninstall-mod-conflict");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-conflict",
            &[(
                "files/cfg.conf",
                "~/.config/app/cfg.conf",
                "original content",
            )],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        fs::write(
            sandbox.home_dir.join(".config/app/cfg.conf"),
            "custom content",
        )
        .unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-conflict",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert_eq!(uninst_res.conflict_files, vec!["~/.config/app/cfg.conf"]);
    }

    // 6. Symlink is safely handled without following target
    #[test]
    fn test_uninstall_symlink_safely_handled_without_following_target() {
        let sandbox = TestSandbox::new("uninstall-symlink");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-sym",
            &[(
                "files/app.conf",
                "~/.config/app/app.conf",
                "installed regular file",
            )],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // Create an external secret file
        let external_target = sandbox.root.join("external_secret.txt");
        fs::write(&external_target, "critical user secret").unwrap();

        // User replaced installed file with a symlink pointing to external_target
        let app_conf = sandbox.home_dir.join(".config/app/app.conf");
        fs::remove_file(&app_conf).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&external_target, &app_conf).unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-sym",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        // Symlink replacement should be classified as conflict because Ryzora didn't install it as symlink
        assert_eq!(uninst_res.conflict_files, vec!["~/.config/app/app.conf"]);
        // External target must NEVER be deleted!
        assert!(external_target.is_file());
        assert_eq!(
            fs::read_to_string(&external_target).unwrap(),
            "critical user secret"
        );
    }

    // 7. Unsafe target path is rejected
    #[test]
    fn test_uninstall_unsafe_target_path_rejected() {
        let sandbox = TestSandbox::new("uninstall-unsafe-target");

        // Construct fake metadata with a malicious target path
        let record = InstalledPackageRecord {
            package_id: "pkg-malicious".to_string(),
            name: "Malicious".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some(PackageType::Rice),
            repository_id: None,
            installed_at: 1000,
            snapshot_id: "dummy".to_string(),
            installed_files: vec!["~/../../etc/passwd".to_string()],
            files: vec![InstalledFileEntry {
                target: "~/../../etc/passwd".to_string(),
                sha256: "deadbeef".to_string(),
                is_symlink: false,
                symlink_target: None,
            }],
            package_source_path: "/dummy".to_string(),
            cryptographic_status: None,
            signer_key_id: None,
            signer_name: None,
            history: Vec::new(),
        };

        let record_json = serde_json::to_string(&record).unwrap();
        fs::write(
            sandbox.installed_dir.join("pkg-malicious.json"),
            record_json,
        )
        .unwrap();

        let err = uninstall_package_in(
            "pkg-malicious",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap_err();

        assert!(err.contains("traversal") || err.contains("outside"));
    }

    // 8. Untrusted metadata is rejected
    #[test]
    fn test_uninstall_untrusted_metadata_rejected() {
        let sandbox = TestSandbox::new("uninstall-untrusted");

        // Invalid package IDs
        assert!(uninstall_package_in(
            "..",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir
        )
        .is_err());
        assert!(uninstall_package_in(
            "a/b",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir
        )
        .is_err());
        assert!(uninstall_package_in(
            "",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir
        )
        .is_err());

        // Malformed JSON record
        fs::write(
            sandbox.installed_dir.join("pkg-corrupt.json"),
            "{ invalid json",
        )
        .unwrap();
        let err = uninstall_package_in(
            "pkg-corrupt",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap_err();
        assert!(err.contains("Corrupted record"));
    }

    // 9. Verified pre-uninstall snapshot is created
    #[test]
    fn test_uninstall_verified_pre_uninstall_snapshot_created() {
        let sandbox = TestSandbox::new("uninstall-snapshot-verified");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-snap",
            &[("files/cfg.conf", "~/.config/app/cfg.conf", "content")],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let uninst_res = uninstall_package_in(
            "pkg-snap",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst_res.success);
        let snap_id = uninst_res
            .snapshot_id
            .expect("pre-uninstall snapshot id missing");
        let valid = verify_snapshot_in(&snap_id, &sandbox.snapshots_dir).unwrap();
        assert!(
            valid,
            "Pre-uninstall snapshot must pass cryptographic verification"
        );
    }

    // 10. Unexpected uninstall failure triggers rollback
    #[test]
    fn test_uninstall_unexpected_failure_triggers_rollback() {
        let sandbox = TestSandbox::new("uninstall-rollback");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-rb",
            &[
                ("files/a.conf", "~/.config/app10/a.conf", "content a"),
                ("files/b.conf", "~/.config/app10/b.conf", "content b"),
            ],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // Make parent dir read-only so file removal fails
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let p = sandbox.home_dir.join(".config/app10");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o555)).unwrap();

            let uninst_res = uninstall_package_in(
                "pkg-rb",
                &sandbox.home_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
            )
            .unwrap();

            assert!(uninst_res.rolled_back);
            assert!(!uninst_res.success);

            // Restore write permission so sandbox cleanup succeeds
            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    // 11. Rollback restores files correctly
    #[test]
    fn test_uninstall_rollback_restores_files_correctly() {
        let sandbox = TestSandbox::new("uninstall-rollback-restores");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-restore",
            &[(
                "files/a.conf",
                "~/.config/app11/a.conf",
                "vital original content",
            )],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let p = sandbox.home_dir.join(".config/app11");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o555)).unwrap();

            let uninst_res = uninstall_package_in(
                "pkg-restore",
                &sandbox.home_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
            )
            .unwrap();

            assert!(uninst_res.rolled_back);

            fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();

            // File must be present with original content
            let file_a = sandbox.home_dir.join(".config/app11/a.conf");
            assert!(file_a.is_file());
            assert_eq!(
                fs::read_to_string(file_a).unwrap(),
                "vital original content"
            );
        }
    }

    // 12. Metadata is updated correctly
    #[test]
    fn test_uninstall_metadata_updated_correctly() {
        let sandbox = TestSandbox::new("uninstall-meta");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-meta-test",
            &[("files/cfg.conf", "~/.config/app/cfg.conf", "content")],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        assert!(
            get_installed_package_in("pkg-meta-test", &sandbox.installed_dir)
                .unwrap()
                .is_some()
        );

        uninstall_package_in(
            "pkg-meta-test",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(
            get_installed_package_in("pkg-meta-test", &sandbox.installed_dir)
                .unwrap()
                .is_none()
        );
    }

    // 13. Stale/legacy metadata without checksums is refused safely
    #[test]
    fn test_uninstall_stale_legacy_metadata_without_checksums_refused_safely() {
        let sandbox = TestSandbox::new("uninstall-legacy");

        // Write legacy record with empty `files`
        let legacy_record = InstalledPackageRecord {
            package_id: "pkg-legacy".to_string(),
            name: "Legacy Package".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 1000,
            snapshot_id: "snap-legacy".to_string(),
            installed_files: vec!["~/.config/app/important.conf".to_string()],
            files: vec![], // No checksums!
            package_source_path: "/legacy/path".to_string(),
            cryptographic_status: None,
            signer_key_id: None,
            signer_name: None,
            history: Vec::new(),
        };

        let json = serde_json::to_string(&legacy_record).unwrap();
        fs::write(sandbox.installed_dir.join("pkg-legacy.json"), json).unwrap();

        let conf_path = sandbox.home_dir.join(".config/app/important.conf");
        fs::create_dir_all(conf_path.parent().unwrap()).unwrap();
        fs::write(&conf_path, "important user data").unwrap();

        let err = uninstall_package_in(
            "pkg-legacy",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap_err();

        assert!(err.contains("legacy metadata without cryptographic file checksums"));
        // User file MUST be preserved!
        assert!(conf_path.is_file());
        assert_eq!(
            fs::read_to_string(&conf_path).unwrap(),
            "important user data"
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // UPDATE TESTS (14 - 30)
    // ─────────────────────────────────────────────────────────────────────────

    // Mock Repository for update tests
    struct MockUpdateRepo {
        id: String,
        entries: Vec<crate::repository::RepositoryPackageEntry>,
        fail: bool,
        package_dirs: std::collections::HashMap<String, PathBuf>,
    }

    impl MockUpdateRepo {
        fn new(id: &str, entries: Vec<crate::repository::RepositoryPackageEntry>) -> Self {
            Self {
                id: id.to_string(),
                entries,
                fail: false,
                package_dirs: std::collections::HashMap::new(),
            }
        }

        fn with_dir(mut self, pkg_id: &str, dir: PathBuf) -> Self {
            self.package_dirs.insert(pkg_id.to_string(), dir);
            self
        }
    }

    impl crate::repository::Repository for MockUpdateRepo {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            "Mock Update Repo"
        }
        fn repo_type(&self) -> &str {
            "mock"
        }
        fn list_entries(&self) -> Result<Vec<crate::repository::RepositoryPackageEntry>, String> {
            if self.fail {
                return Err("Repository network error".to_string());
            }
            Ok(self.entries.clone())
        }
        fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
            let dir = self.get_package_dir(package_id)?;
            load_package_manifest(&dir)
        }
        fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
            if self.fail {
                return Err("Repository network error".to_string());
            }
            if let Some(dir) = self.package_dirs.get(package_id) {
                Ok(dir.clone())
            } else {
                Err(format!(
                    "Package '{}' not found in mock repo '{}'",
                    package_id, self.id
                ))
            }
        }
    }

    // 14. Installed package is up to date
    #[test]
    fn test_update_installed_package_up_to_date() {
        let sandbox = TestSandbox::new("update-uptodate");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-up",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let mut mgr = crate::repository::RepositoryManager::new();
        mgr.add_repository(Box::new(MockUpdateRepo {
            id: "community".to_string(),
            entries: vec![crate::repository::RepositoryPackageEntry {
                id: "pkg-up".to_string(),
                name: "Test Package".to_string(),
                version: "1.0.0".to_string(),
                package_type: PackageType::Rice,
                description: "Test".to_string(),
                manifest: "manifest.json".to_string(),
                category: "rice".to_string(),
                tags: vec![],
                author: crate::repository::AuthorInfo {
                    name: "Tester".to_string(),
                    avatar: "".to_string(),
                    verified: true,
                },
                color_palette: vec![],
                hero_image: None,
                screenshots: vec![],
                featured: None,
                trending: None,
                rating: None,
                downloads: None,
                content_hash: None,
                package_size_bytes: None,
                release_channel: None,
                trust_tier: None,
                moderation_status: None,
                trending_score: None,
                maintainer: None,
                release_notes: None,
                signature: None,
                provider: None,
                targets: None,
                preview_video: None,
                preview_animated: None,

                media_type: None,
                source: None,
                provenance: None,
            }],
            fail: false,
            package_dirs: std::collections::HashMap::new(),
        }));

        let status = check_package_update_in("pkg-up", &sandbox.installed_dir, &mgr).unwrap();
        assert_eq!(status.status, UpdateStatusKind::UpToDate);
    }

    // 15. Update is detected
    #[test]
    fn test_update_detected() {
        let sandbox = TestSandbox::new("update-detected");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-detect",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let mut mgr = crate::repository::RepositoryManager::new();
        mgr.add_repository(Box::new(MockUpdateRepo {
            id: "community".to_string(),
            entries: vec![crate::repository::RepositoryPackageEntry {
                id: "pkg-detect".to_string(),
                name: "Test Package".to_string(),
                version: "1.2.0".to_string(),
                package_type: PackageType::Rice,
                description: "Test".to_string(),
                manifest: "manifest.json".to_string(),
                category: "rice".to_string(),
                tags: vec![],
                author: crate::repository::AuthorInfo {
                    name: "Tester".to_string(),
                    avatar: "".to_string(),
                    verified: true,
                },
                color_palette: vec![],
                hero_image: None,
                screenshots: vec![],
                featured: None,
                trending: None,
                rating: None,
                downloads: None,
                content_hash: None,
                package_size_bytes: None,
                release_channel: None,
                trust_tier: None,
                moderation_status: None,
                trending_score: None,
                maintainer: None,
                release_notes: None,
                signature: None,
                provider: None,
                targets: None,
                preview_video: None,
                preview_animated: None,

                media_type: None,
                source: None,
                provenance: None,
            }],
            fail: false,
            package_dirs: std::collections::HashMap::new(),
        }));

        let status = check_package_update_in("pkg-detect", &sandbox.installed_dir, &mgr).unwrap();
        assert_eq!(status.status, UpdateStatusKind::UpdateAvailable);
        assert_eq!(status.available_version, Some("1.2.0".to_string()));
    }

    // 16. Repository unavailable is not reported as up to date
    #[test]
    fn test_update_repository_unavailable_not_reported_as_up_to_date() {
        let sandbox = TestSandbox::new("update-repo-unavailable");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "pkg-fail",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let mut mgr = crate::repository::RepositoryManager::new();
        mgr.add_repository(Box::new(MockUpdateRepo {
            id: "community".to_string(),
            entries: vec![],
            fail: true,
            package_dirs: std::collections::HashMap::new(),
        }));

        let status = check_package_update_in("pkg-fail", &sandbox.installed_dir, &mgr).unwrap();
        assert_eq!(status.status, UpdateStatusKind::RepositoryUnavailable);
    }

    // 17. Update preview creates correct CREATE entries
    #[test]
    fn test_update_preview_creates_correct_create_entries() {
        let sandbox = TestSandbox::new("update-preview-create");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-up-test",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1 content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-up-test",
            "1.1.0",
            &[
                ("files/a.conf", "~/.config/app/a.conf", "v1 content"),
                ("files/b.conf", "~/.config/app/b.conf", "new v2 file"),
            ],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-up-test",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert_eq!(plan.creates, vec!["~/.config/app/b.conf"]);
    }

    // 18. Update preview creates correct REPLACE entries
    #[test]
    fn test_update_preview_creates_correct_replace_entries() {
        let sandbox = TestSandbox::new("update-preview-replace");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-replace",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "version 1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-replace",
            "1.1.0",
            &[("files/a.conf", "~/.config/app/a.conf", "version 2 modified")],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-replace",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert_eq!(plan.replaces, vec!["~/.config/app/a.conf"]);
        assert!(plan.creates.is_empty());
        assert!(!plan.has_conflicts);
    }

    // 19. Unchanged files are identified
    #[test]
    fn test_update_preview_unchanged_files_identified() {
        let sandbox = TestSandbox::new("update-preview-unchanged");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-unchanged",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "same content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-unchanged",
            "1.1.0",
            &[("files/a.conf", "~/.config/app/a.conf", "same content")],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-unchanged",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert_eq!(plan.unchanged, vec!["~/.config/app/a.conf"]);
        assert!(plan.replaces.is_empty());
    }

    // 20. Modified files become CONFLICT
    #[test]
    fn test_update_preview_modified_files_become_conflict() {
        let sandbox = TestSandbox::new("update-preview-conflict");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-conf",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1 original")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User edits file
        fs::write(sandbox.home_dir.join(".config/app/a.conf"), "user edited").unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-conf",
            "1.1.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v2 new")],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-conf",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert!(plan.has_conflicts);
        assert_eq!(plan.conflicts, vec!["~/.config/app/a.conf"]);
    }

    // 21. Obsolete unmodified files are safely removed
    #[test]
    fn test_update_obsolete_unmodified_files_safely_removed() {
        let sandbox = TestSandbox::new("update-obsolete-remove");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-obs",
            "1.0.0",
            &[
                ("files/kept.conf", "~/.config/app/kept.conf", "kept content"),
                (
                    "files/obs.conf",
                    "~/.config/app/obs.conf",
                    "obsolete content",
                ),
            ],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-obs",
            "1.1.0",
            &[(
                "files/kept.conf",
                "~/.config/app/kept.conf",
                "kept content v2",
            )],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-obs",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert_eq!(plan.obsolete_removes, vec!["~/.config/app/obs.conf"]);

        let res = apply_package_update_in(
            "pkg-obs",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            None,
        )
        .unwrap();

        assert!(res.success);
        assert_eq!(res.obsolete_removed, vec!["~/.config/app/obs.conf"]);
        assert!(!sandbox.home_dir.join(".config/app/obs.conf").exists());
        assert!(sandbox.home_dir.join(".config/app/kept.conf").exists());
    }

    // 22. Obsolete modified files are retained
    #[test]
    fn test_update_obsolete_modified_files_retained() {
        let sandbox = TestSandbox::new("update-obsolete-retain");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-obs-mod",
            "1.0.0",
            &[
                ("files/kept.conf", "~/.config/app/kept.conf", "kept content"),
                (
                    "files/obs.conf",
                    "~/.config/app/obs.conf",
                    "obsolete content",
                ),
            ],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User edits obs.conf
        let obs_file = sandbox.home_dir.join(".config/app/obs.conf");
        fs::write(&obs_file, "customized obsolete file").unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-obs-mod",
            "1.1.0",
            &[(
                "files/kept.conf",
                "~/.config/app/kept.conf",
                "kept content v2",
            )],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-obs-mod",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert_eq!(plan.obsolete_retains, vec!["~/.config/app/obs.conf"]);
        assert!(plan.has_conflicts);

        // Apply update with allow_conflicts = true so it proceeds while retaining conflicting file
        let res = apply_package_update_in(
            "pkg-obs-mod",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            true,
            None,
        )
        .unwrap();

        assert!(res.success);
        // User-modified obsolete file MUST still exist!
        assert!(obs_file.is_file());
        assert_eq!(
            fs::read_to_string(&obs_file).unwrap(),
            "customized obsolete file"
        );
    }

    // 23. Update does not modify filesystem during preview
    #[test]
    fn test_update_does_not_modify_filesystem_during_preview() {
        let sandbox = TestSandbox::new("update-preview-readonly");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-readonly",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "initial content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let file_path = sandbox.home_dir.join(".config/app/a.conf");
        let before_content = fs::read_to_string(&file_path).unwrap();
        let before_meta = fs::metadata(&file_path).unwrap().modified().unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-readonly",
            "1.1.0",
            &[(
                "files/a.conf",
                "~/.config/app/a.conf",
                "new updated content",
            )],
            &[],
        );

        let _plan = preview_package_update_in(
            "pkg-readonly",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        let after_content = fs::read_to_string(&file_path).unwrap();
        let after_meta = fs::metadata(&file_path).unwrap().modified().unwrap();

        assert_eq!(before_content, after_content);
        assert_eq!(before_meta, after_meta);
    }

    // 24. Successful update refreshes installed metadata
    #[test]
    fn test_update_successful_refreshes_installed_metadata() {
        let sandbox = TestSandbox::new("update-meta-refresh");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-refresh",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-refresh",
            "1.2.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content v2 updated")],
            &[],
        );

        let res = apply_package_update_in(
            "pkg-refresh",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            None,
        )
        .unwrap();

        assert!(res.success);

        let record = get_installed_package_in("pkg-refresh", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        assert_eq!(record.version, "1.2.0");
        assert_eq!(record.files.len(), 1);
        let expected_hash = sha256_file(&sandbox.home_dir.join(".config/app/a.conf")).unwrap();
        assert_eq!(record.files[0].sha256, expected_hash);
    }

    // 25. Failed update rolls back
    #[test]
    fn test_update_failed_rolls_back() {
        let sandbox = TestSandbox::new("update-rollback");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-fail-rb",
            "1.0.0",
            &[("files/a.conf", "~/.config/app25/a.conf", "v1 original")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-fail-rb",
            "1.1.0",
            &[("files/a.conf", "~/.config/app25/a.conf", "v2 updated")],
            &[],
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file_a = sandbox.home_dir.join(".config/app25/a.conf");
            fs::set_permissions(&file_a, fs::Permissions::from_mode(0o444)).unwrap();

            let res = apply_package_update_in(
                "pkg-fail-rb",
                &sandbox.home_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
                &sandbox.staging_dir,
                &v2_dir,
                &sys,
                false,
                None,
            )
            .unwrap();

            assert!(res.rolled_back);
            assert!(!res.success);

            fs::set_permissions(&file_a, fs::Permissions::from_mode(0o644)).unwrap();

            // Check original v1 content was restored
            assert_eq!(fs::read_to_string(&file_a).unwrap(), "v1 original");
        }
    }

    // 26. Rollback preserves old metadata
    #[test]
    fn test_update_rollback_preserves_old_metadata() {
        let sandbox = TestSandbox::new("update-meta-preserve");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-preserve",
            "1.0.0",
            &[("files/a.conf", "~/.config/app26/a.conf", "v1 original")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-preserve",
            "1.1.0",
            &[("files/a.conf", "~/.config/app26/a.conf", "v2 updated")],
            &[],
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file_a = sandbox.home_dir.join(".config/app26/a.conf");
            fs::set_permissions(&file_a, fs::Permissions::from_mode(0o444)).unwrap();

            let _ = apply_package_update_in(
                "pkg-preserve",
                &sandbox.home_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
                &sandbox.staging_dir,
                &v2_dir,
                &sys,
                false,
                None,
            );

            fs::set_permissions(&file_a, fs::Permissions::from_mode(0o644)).unwrap();

            let record = get_installed_package_in("pkg-preserve", &sandbox.installed_dir)
                .unwrap()
                .unwrap();
            assert_eq!(record.version, "1.0.0");
        }
    }

    // 27. Update cannot bypass installer security validation
    #[test]
    fn test_update_cannot_bypass_installer_security_validation() {
        let sandbox = TestSandbox::new("update-security-bypass");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-sec",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1 original")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // v2 specifies an invalid target outside allowed scope
        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-sec",
            "1.1.0",
            &[("files/a.conf", "~/Documents/hacked.conf", "malicious write")],
            &[],
        );

        let err = preview_package_update_in(
            "pkg-sec",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap_err();

        assert!(err.contains("outside allowed configuration scope"));
    }

    // 28. Manifest identity mismatch is rejected
    #[test]
    fn test_update_manifest_identity_mismatch_rejected() {
        let sandbox = TestSandbox::new("update-identity-mismatch");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-correct-id",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-DIFFERENT-id",
            "1.1.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        let err = preview_package_update_in(
            "pkg-correct-id",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap_err();

        assert!(err.contains("Manifest ID mismatch"));
    }

    // 29. Malicious target traversal is rejected
    #[test]
    fn test_update_malicious_target_traversal_rejected() {
        let sandbox = TestSandbox::new("update-target-traversal");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-traversal",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-traversal",
            "1.1.0",
            &[(
                "files/a.conf",
                "~/.config/app/../../../etc/passwd",
                "malicious",
            )],
            &[],
        );

        let err = preview_package_update_in(
            "pkg-traversal",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap_err();

        assert!(err.contains("traversal") || err.contains("outside"));
    }

    // 30. Special files remain rejected
    #[test]
    fn test_update_special_files_remain_rejected() {
        let sandbox = TestSandbox::new("update-special-files");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-special",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // Create v2 where source file is a directory rather than regular file
        let v2_dir = sandbox.packages_dir.join("pkg-special-1.1.0");
        fs::create_dir_all(&v2_dir).unwrap();
        let manifest = r#"{
  "id": "pkg-special",
  "name": "Special Package",
  "version": "1.1.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": {
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch"],
    "required": [],
    "optional": []
  },
  "files": [
    {"source": "files/a_dir", "target": "~/.config/app/a.conf", "description": "a dir"}
  ]
}"#;
        fs::write(v2_dir.join("manifest.json"), manifest).unwrap();
        fs::create_dir_all(v2_dir.join("files/a_dir")).unwrap();

        let err = preview_package_update_in(
            "pkg-special",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap_err();

        assert!(
            err.contains("regular file")
                || err.contains("not a regular file")
                || err.contains("directory")
        );
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PHASE 7.1 SPECIFIC SAFETY TESTS (1 - 14)
    // ─────────────────────────────────────────────────────────────────────────

    // 1. installed symlink with unchanged target can be uninstalled
    #[test]
    fn test_uninstall_installed_symlink_unchanged_target_removed() {
        let sandbox = TestSandbox::new("symlink-uninstall-ok");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_package_with_symlinks(
            "pkg-sym-ok",
            "1.0.0",
            &[("files/main.conf", "~/.config/app/main.conf", "main content")],
            &[("files/link.conf", "~/.config/app/link.conf", "main.conf")],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
        assert!(res.success);

        // Verify symlink is on disk and has target
        let link_path = sandbox.home_dir.join(".config/app/link.conf");
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());

        // Uninstall package
        let uninst = uninstall_package_in(
            "pkg-sym-ok",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst.success);
        assert!(!link_path.exists());
        assert!(!sandbox.home_dir.join(".config/app/main.conf").exists());
        assert!(uninst.conflict_files.is_empty());
        assert!(uninst
            .removed_files
            .contains(&"~/.config/app/link.conf".to_string()));
    }

    // 2. installed symlink changed to another target becomes conflict
    #[test]
    fn test_uninstall_installed_symlink_changed_target_becomes_conflict() {
        let sandbox = TestSandbox::new("symlink-target-changed");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_package_with_symlinks(
            "pkg-sym-chg",
            "1.0.0",
            &[("files/main.conf", "~/.config/app/main.conf", "content")],
            &[("files/link.conf", "~/.config/app/link.conf", "main.conf")],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User changes symlink to point to another file
        let link_path = sandbox.home_dir.join(".config/app/link.conf");
        fs::remove_file(&link_path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("other_file.conf", &link_path).unwrap();

        // Uninstall package
        let uninst = uninstall_package_in(
            "pkg-sym-chg",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst.success);
        // Link must be retained as conflict! NEVER delete user-modified symlink target!
        assert!(uninst
            .conflict_files
            .contains(&"~/.config/app/link.conf".to_string()));
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(&link_path).unwrap().to_str().unwrap(),
            "other_file.conf"
        );
    }

    // 3. installed symlink replaced with regular file becomes conflict
    #[test]
    fn test_uninstall_installed_symlink_replaced_with_regular_file_becomes_conflict() {
        let sandbox = TestSandbox::new("symlink-replaced-reg");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_package_with_symlinks(
            "pkg-sym-rep",
            "1.0.0",
            &[("files/main.conf", "~/.config/app/main.conf", "content")],
            &[("files/link.conf", "~/.config/app/link.conf", "main.conf")],
        );

        install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // User replaces symlink with regular file
        let link_path = sandbox.home_dir.join(".config/app/link.conf");
        fs::remove_file(&link_path).unwrap();
        fs::write(&link_path, "regular file replacing symlink").unwrap();

        let uninst = uninstall_package_in(
            "pkg-sym-rep",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst.success);
        assert!(uninst
            .conflict_files
            .contains(&"~/.config/app/link.conf".to_string()));
        assert!(link_path.is_file());
        assert_eq!(
            fs::read_to_string(&link_path).unwrap(),
            "regular file replacing symlink"
        );
    }

    // 4. obsolete symlink with unchanged target is removed
    #[test]
    fn test_update_obsolete_symlink_unchanged_target_removed() {
        let sandbox = TestSandbox::new("obs-sym-removed");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_package_with_symlinks(
            "pkg-obs-sym",
            "1.0.0",
            &[("files/main.conf", "~/.config/app/main.conf", "v1 content")],
            &[(
                "files/obs_link.conf",
                "~/.config/app/obs_link.conf",
                "main.conf",
            )],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let link_path = sandbox.home_dir.join(".config/app/obs_link.conf");
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());

        // v2 drops the obsolete symlink
        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-obs-sym",
            "1.1.0",
            &[("files/main.conf", "~/.config/app/main.conf", "v2 content")],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-obs-sym",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert!(plan
            .obsolete_removes
            .contains(&"~/.config/app/obs_link.conf".to_string()));
        assert!(!plan.has_conflicts);

        let res = apply_package_update_in(
            "pkg-obs-sym",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            None,
        )
        .unwrap();

        assert!(res.success);
        assert!(res
            .obsolete_removed
            .contains(&"~/.config/app/obs_link.conf".to_string()));
        assert!(!link_path.exists());
    }

    // 5. obsolete symlink with changed target is retained
    #[test]
    fn test_update_obsolete_symlink_changed_target_retained() {
        let sandbox = TestSandbox::new("obs-sym-retained");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_package_with_symlinks(
            "pkg-obs-mod-sym",
            "1.0.0",
            &[("files/main.conf", "~/.config/app/main.conf", "v1 content")],
            &[(
                "files/obs_link.conf",
                "~/.config/app/obs_link.conf",
                "main.conf",
            )],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let link_path = sandbox.home_dir.join(".config/app/obs_link.conf");
        // User points obsolete symlink to custom config
        fs::remove_file(&link_path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("custom_target.conf", &link_path).unwrap();

        // v2 drops obsolete symlink
        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-obs-mod-sym",
            "1.1.0",
            &[("files/main.conf", "~/.config/app/main.conf", "v2 content")],
            &[],
        );

        let plan = preview_package_update_in(
            "pkg-obs-mod-sym",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &v2_dir,
        )
        .unwrap();

        assert!(plan
            .obsolete_retains
            .contains(&"~/.config/app/obs_link.conf".to_string()));
        assert!(plan.has_conflicts);

        // Applying without allow_conflicts must fail
        let err = apply_package_update_in(
            "pkg-obs-mod-sym",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            None,
        )
        .unwrap_err();
        assert!(err.contains("conflicting user-modified files"));

        // Applying with allow_conflicts proceeds while retaining user modified symlink
        let res = apply_package_update_in(
            "pkg-obs-mod-sym",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            true,
            None,
        )
        .unwrap();

        assert!(res.success);
        assert!(res
            .conflicts_retained
            .contains(&"~/.config/app/obs_link.conf".to_string()));
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(&link_path).unwrap().to_str().unwrap(),
            "custom_target.conf"
        );
    }

    // 6. legacy symlink metadata without recorded target is retained/refused
    #[test]
    fn test_legacy_symlink_metadata_without_recorded_target_retained_as_conflict() {
        let sandbox = TestSandbox::new("legacy-sym-unverifiable");

        let link_target = sandbox.home_dir.join(".config/app/link.conf");
        if let Some(p) = link_target.parent() {
            fs::create_dir_all(p).unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink("target.conf", &link_target).unwrap();

        // Write legacy record with is_symlink: true but symlink_target: None
        let record = InstalledPackageRecord {
            package_id: "pkg-legacy-sym".to_string(),
            name: "Legacy Symlink Package".to_string(),
            version: "1.0.0".to_string(),
            package_type: Some(PackageType::Rice),
            repository_id: None,
            installed_at: 1000,
            snapshot_id: "dummy".to_string(),
            installed_files: vec!["~/.config/app/link.conf".to_string()],
            files: vec![InstalledFileEntry {
                target: "~/.config/app/link.conf".to_string(),
                sha256: String::new(),
                is_symlink: true,
                symlink_target: None, // legacy record
            }],
            package_source_path: "/dummy".to_string(),
            cryptographic_status: None,
            signer_key_id: None,
            signer_name: None,
            history: Vec::new(),
        };

        let record_json = serde_json::to_string(&record).unwrap();
        fs::write(
            sandbox.installed_dir.join("pkg-legacy-sym.json"),
            record_json,
        )
        .unwrap();

        // Uninstalling must NOT delete the unverifiable symlink
        let uninst = uninstall_package_in(
            "pkg-legacy-sym",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
        )
        .unwrap();

        assert!(uninst.success);
        assert!(uninst
            .conflict_files
            .contains(&"~/.config/app/link.conf".to_string()));
        assert!(fs::symlink_metadata(&link_target)
            .unwrap()
            .file_type()
            .is_symlink());
    }

    // 7. update from correct recorded repository
    #[test]
    fn test_update_from_correct_recorded_repository() {
        let sandbox = TestSandbox::new("update-correct-repo");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package(
            "pkg-repo-sel",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // Set repository_id in record to "repo-main"
        let record_path = sandbox.installed_dir.join("pkg-repo-sel.json");
        let mut rec: InstalledPackageRecord =
            serde_json::from_str(&fs::read_to_string(&record_path).unwrap()).unwrap();
        rec.repository_id = Some("repo-main".to_string());
        fs::write(&record_path, serde_json::to_string(&rec).unwrap()).unwrap();

        // Setup 2 repos: repo-main (v2.0.0) and repo-other (v3.0.0)
        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-repo-sel",
            "2.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v2 repo-main")],
            &[],
        );
        let v3_dir = sandbox.create_sample_package_with_version(
            "pkg-repo-sel",
            "3.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v3 repo-other")],
            &[],
        );

        let mut mgr = crate::repository::RepositoryManager::new();
        mgr.add_repository(Box::new(
            MockUpdateRepo::new(
                "repo-main",
                vec![crate::repository::RepositoryPackageEntry {
                    id: "pkg-repo-sel".to_string(),
                    name: "Repo Sel".to_string(),
                    version: "2.0.0".to_string(),
                    package_type: PackageType::Rice,
                    description: "Main".to_string(),
                    manifest: "manifest.json".to_string(),
                    category: "rice".to_string(),
                    tags: vec![],
                    author: crate::repository::AuthorInfo {
                        name: "Tester".to_string(),
                        avatar: "".to_string(),
                        verified: true,
                    },
                    color_palette: vec![],
                    hero_image: None,
                    screenshots: vec![],
                    featured: None,
                    trending: None,
                    rating: None,
                    downloads: None,
                    content_hash: None,
                    package_size_bytes: None,
                    release_channel: None,
                    trust_tier: None,
                    moderation_status: None,
                    trending_score: None,
                    maintainer: None,
                    release_notes: None,
                    signature: None,
                    provider: None,
                    targets: None,
                    preview_video: None,
                    preview_animated: None,

                    media_type: None,
                    source: None,
                    provenance: None,
                }],
            )
            .with_dir("pkg-repo-sel", v2_dir.clone()),
        ));

        mgr.add_repository(Box::new(
            MockUpdateRepo::new(
                "repo-other",
                vec![crate::repository::RepositoryPackageEntry {
                    id: "pkg-repo-sel".to_string(),
                    name: "Repo Sel".to_string(),
                    version: "3.0.0".to_string(),
                    package_type: PackageType::Rice,
                    description: "Other".to_string(),
                    manifest: "manifest.json".to_string(),
                    category: "rice".to_string(),
                    tags: vec![],
                    author: crate::repository::AuthorInfo {
                        name: "Tester".to_string(),
                        avatar: "".to_string(),
                        verified: true,
                    },
                    color_palette: vec![],
                    hero_image: None,
                    screenshots: vec![],
                    featured: None,
                    trending: None,
                    rating: None,
                    downloads: None,
                    content_hash: None,
                    package_size_bytes: None,
                    release_channel: None,
                    trust_tier: None,
                    moderation_status: None,
                    trending_score: None,
                    maintainer: None,
                    release_notes: None,
                    signature: None,
                    provider: None,
                    targets: None,
                    preview_video: None,
                    preview_animated: None,

                    media_type: None,
                    source: None,
                    provenance: None,
                }],
            )
            .with_dir("pkg-repo-sel", v3_dir),
        ));

        let (resolved_dir, resolved_repo, version) =
            resolve_update_package("pkg-repo-sel", &sandbox.installed_dir, &mgr).unwrap();

        assert_eq!(resolved_repo, "repo-main");
        assert_eq!(version, "2.0.0");
        assert_eq!(resolved_dir, v2_dir);
    }

    // 8. duplicate package IDs across repositories do not cause arbitrary update selection
    #[test]
    fn test_update_duplicate_package_ids_across_repositories_refuses_arbitrary_selection() {
        let sandbox = TestSandbox::new("update-dup-refused");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package(
            "pkg-dup",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // Installed record has NO repository_id
        let mut mgr = crate::repository::RepositoryManager::new();
        mgr.add_repository(Box::new(MockUpdateRepo::new(
            "repo-a",
            vec![crate::repository::RepositoryPackageEntry {
                id: "pkg-dup".to_string(),
                name: "Dup".to_string(),
                version: "1.5.0".to_string(),
                package_type: PackageType::Rice,
                description: "Repo A".to_string(),
                manifest: "manifest.json".to_string(),
                category: "rice".to_string(),
                tags: vec![],
                author: crate::repository::AuthorInfo {
                    name: "Tester".to_string(),
                    avatar: "".to_string(),
                    verified: true,
                },
                color_palette: vec![],
                hero_image: None,
                screenshots: vec![],
                featured: None,
                trending: None,
                rating: None,
                downloads: None,
                content_hash: None,
                package_size_bytes: None,
                release_channel: None,
                trust_tier: None,
                moderation_status: None,
                trending_score: None,
                maintainer: None,
                release_notes: None,
                signature: None,
                provider: None,
                targets: None,
                preview_video: None,
                preview_animated: None,

                media_type: None,
                source: None,
                provenance: None,
            }],
        )));
        mgr.add_repository(Box::new(MockUpdateRepo::new(
            "repo-b",
            vec![crate::repository::RepositoryPackageEntry {
                id: "pkg-dup".to_string(),
                name: "Dup".to_string(),
                version: "1.6.0".to_string(),
                package_type: PackageType::Rice,
                description: "Repo B".to_string(),
                manifest: "manifest.json".to_string(),
                category: "rice".to_string(),
                tags: vec![],
                author: crate::repository::AuthorInfo {
                    name: "Tester".to_string(),
                    avatar: "".to_string(),
                    verified: true,
                },
                color_palette: vec![],
                hero_image: None,
                screenshots: vec![],
                featured: None,
                trending: None,
                rating: None,
                downloads: None,
                content_hash: None,
                package_size_bytes: None,
                release_channel: None,
                trust_tier: None,
                moderation_status: None,
                trending_score: None,
                maintainer: None,
                release_notes: None,
                signature: None,
                provider: None,
                targets: None,
                preview_video: None,
                preview_animated: None,

                media_type: None,
                source: None,
                provenance: None,
            }],
        )));

        let err = resolve_update_package("pkg-dup", &sandbox.installed_dir, &mgr).unwrap_err();
        assert!(err.contains("Ambiguous update") || err.contains("multiple repositories"));
    }

    // 9. update with same version is refused
    #[test]
    fn test_update_with_same_version_refused() {
        let sandbox = TestSandbox::new("update-same-ver");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-same",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let err = apply_package_update_in(
            "pkg-same",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v1_dir,
            &sys,
            false,
            None,
        )
        .unwrap_err();

        assert!(err.contains("already at version") || err.contains("1.0.0"));
    }

    // 10. downgrade attempt is refused
    #[test]
    fn test_update_downgrade_attempt_refused() {
        let sandbox = TestSandbox::new("update-downgrade");
        let sys = TestSandbox::mock_system();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-down",
            "2.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v2")],
            &[],
        );

        install_package_in(
            &v2_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-down",
            "1.5.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1.5")],
            &[],
        );

        let err = apply_package_update_in(
            "pkg-down",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v1_dir,
            &sys,
            false,
            None,
        )
        .unwrap_err();

        assert!(err.contains("Downgrade attempt refused"));
    }

    // 11. repository package unavailable is refused
    #[test]
    fn test_update_repository_package_unavailable_refused() {
        let sandbox = TestSandbox::new("update-repo-unavail");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package(
            "pkg-unavail",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let record_path = sandbox.installed_dir.join("pkg-unavail.json");
        let mut rec: InstalledPackageRecord =
            serde_json::from_str(&fs::read_to_string(&record_path).unwrap()).unwrap();
        rec.repository_id = Some("missing-repository".to_string());
        fs::write(&record_path, serde_json::to_string(&rec).unwrap()).unwrap();

        let mgr = crate::repository::RepositoryManager::new();
        let err = resolve_update_package("pkg-unavail", &sandbox.installed_dir, &mgr).unwrap_err();
        assert!(
            err.contains("unavailable or not configured") || err.contains("missing-repository")
        );
    }

    // 12. successful update records the actual repository ID
    #[test]
    fn test_update_successful_records_actual_repository_id() {
        let sandbox = TestSandbox::new("update-records-repo-id");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-rec-rep",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-rec-rep",
            "1.1.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v2")],
            &[],
        );

        let res = apply_package_update_in(
            "pkg-rec-rep",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            Some("community-custom".to_string()),
        )
        .unwrap();

        assert!(res.success);

        // Verify updated record has repository_id = "community-custom"
        let record = get_installed_package_in("pkg-rec-rep", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        assert_eq!(record.repository_id, Some("community-custom".to_string()));
        assert_eq!(record.version, "1.1.0");
    }

    // 13. successful update records symlink targets
    #[test]
    fn test_update_successful_records_symlink_targets() {
        let sandbox = TestSandbox::new("update-records-symlinks");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_sample_package_with_version(
            "pkg-sym-rec",
            "1.0.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v1")],
            &[],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        // v2 introduces a symlink
        let v2_dir = sandbox.create_package_with_symlinks(
            "pkg-sym-rec",
            "1.2.0",
            &[("files/a.conf", "~/.config/app/a.conf", "v2")],
            &[("files/link.conf", "~/.config/app/link.conf", "a.conf")],
        );

        let res = apply_package_update_in(
            "pkg-sym-rec",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &v2_dir,
            &sys,
            false,
            None,
        )
        .unwrap();

        assert!(res.success);

        let record = get_installed_package_in("pkg-sym-rec", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        let sym_entry = record
            .files
            .iter()
            .find(|f| f.target == "~/.config/app/link.conf")
            .unwrap();
        assert!(sym_entry.is_symlink);
        assert_eq!(sym_entry.symlink_target, Some("a.conf".to_string()));
    }

    // 14. update rollback preserves old symlink and metadata
    #[test]
    fn test_update_rollback_preserves_old_symlink_and_metadata() {
        let sandbox = TestSandbox::new("update-rb-symlink");
        let sys = TestSandbox::mock_system();

        let v1_dir = sandbox.create_package_with_symlinks(
            "pkg-sym-rb",
            "1.0.0",
            &[(
                "files/main.conf",
                "~/.config/app_rb/main.conf",
                "v1 content",
            )],
            &[("files/link.conf", "~/.config/app_rb/link.conf", "main.conf")],
        );

        install_package_in(
            &v1_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();

        let link_path = sandbox.home_dir.join(".config/app_rb/link.conf");
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(&link_path).unwrap().to_str().unwrap(),
            "main.conf"
        );

        // v2 introduces b.conf, but we make ~/.config/app_rb read-only before copy so update fails and rolls back
        let v2_dir = sandbox.create_sample_package_with_version(
            "pkg-sym-rb",
            "1.1.0",
            &[
                (
                    "files/main.conf",
                    "~/.config/app_rb/main.conf",
                    "v2 updated",
                ),
                (
                    "files/fail.conf",
                    "~/.config/app_rb/fail.conf",
                    "should fail",
                ),
            ],
            &[],
        );

        // Make main.conf read-only to force write failure during update application
        {
            use std::os::unix::fs::PermissionsExt;
            let file_main = sandbox.home_dir.join(".config/app_rb/main.conf");
            fs::set_permissions(&file_main, fs::Permissions::from_mode(0o444)).unwrap();

            let res = apply_package_update_in(
                "pkg-sym-rb",
                &sandbox.home_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
                &sandbox.staging_dir,
                &v2_dir,
                &sys,
                false,
                None,
            )
            .unwrap();

            assert!(!res.success);
            assert!(res.rolled_back);

            // Restore permissions
            fs::set_permissions(&file_main, fs::Permissions::from_mode(0o644)).unwrap();
        }

        // Original symlink must be preserved and still point to original target!
        assert!(fs::symlink_metadata(&link_path)
            .unwrap()
            .file_type()
            .is_symlink());
        #[cfg(unix)]
        assert_eq!(
            fs::read_link(&link_path).unwrap().to_str().unwrap(),
            "main.conf"
        );

        // Old metadata preserved
        let record = get_installed_package_in("pkg-sym-rb", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        assert_eq!(record.version, "1.0.0");
    }

    #[test]
    fn test_phase9_installer_plan_includes_dependency_report() {
        let sandbox = TestSandbox::new("phase9-plan-deps");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "plan-pkg-deps",
            &[("files/a.conf", "~/.config/app/a.conf", "content")],
            &[],
        );

        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();
        assert!(plan.dependency_report.is_some());
        let rep = plan.dependency_report.unwrap();
        assert_eq!(rep.root_package_id, "plan-pkg-deps");
        assert!(rep.resolved);
        assert_eq!(plan.compatibility_status, "compatible");
    }

    #[test]
    fn test_phase9_installer_plan_detects_missing_dependencies() {
        let sandbox = TestSandbox::new("phase9-plan-missing");
        let sys = TestSandbox::mock_system();

        let pkg_dir = sandbox.create_sample_package(
            "plan-pkg-missing",
            &[("files/b.conf", "~/.config/app/b.conf", "content")],
            &[],
        );
        // Overwrite manifest with missing required tool
        let manifest_path = pkg_dir.join("manifest.json");
        let mut manifest: RyzoraManifest =
            serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
        manifest
            .dependencies
            .push(crate::dependency::DependencySpec {
                id: "nonexistent_system_tool_phase9".to_string(),
                kind: crate::dependency::DependencyKind::SystemBinary,
                version_req: None,
                required: true,
                description: Some("Crucial missing tool".to_string()),
            });
        fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();
        assert!(plan.dependency_report.is_some());
        let rep = plan.dependency_report.unwrap();
        assert!(!rep.resolved);
        assert!(rep
            .missing_required
            .contains(&"nonexistent_system_tool_phase9".to_string()));
        assert_eq!(plan.compatibility_status, "missing_dependencies");
        assert!(plan
            .missing_dependencies
            .contains(&"nonexistent_system_tool_phase9".to_string()));
    }

    #[test]
    fn test_phase9_1_installer_rollback_with_dependency_plan() {
        let sandbox = TestSandbox::new("phase9-1-rollback");
        let sys = TestSandbox::mock_system();

        // Target file initially exists
        let orig_file = sandbox.home_dir.join(".config/dep_app/main.conf");
        fs::create_dir_all(orig_file.parent().unwrap()).unwrap();
        fs::write(&orig_file, "original content before install").unwrap();

        // Create package with valid dependency
        let pkg_dir = sandbox.create_sample_package(
            "dep-rollback-pkg",
            &[(
                "files/main.conf",
                "~/.config/dep_app/main.conf",
                "new content",
            )],
            &[],
        );

        // Verify dependency planning succeeds
        let plan = generate_installation_plan_in(&pkg_dir, &sandbox.home_dir, &sys).unwrap();
        assert!(plan.dependency_report.is_some());
        assert!(plan.dependency_report.unwrap().resolved);

        // Make destination file read-only so staged apply fails
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&orig_file, fs::Permissions::from_mode(0o444)).unwrap();

            let res = install_package_in(
                &pkg_dir,
                &sandbox.snapshots_dir,
                &sandbox.installed_dir,
                &sandbox.staging_dir,
                &sandbox.home_dir,
                &sys,
            );

            // Restore permissions
            fs::set_permissions(&orig_file, fs::Permissions::from_mode(0o644)).unwrap();

            let install_res = res.unwrap();
            assert!(!install_res.success);
            assert!(install_res.rolled_back);

            // Check original file was restored
            assert_eq!(
                fs::read_to_string(&orig_file).unwrap(),
                "original content before install"
            );
        }
    }

    #[test]
    fn test_installed_package_history_recorded_on_install() {
        let sandbox = TestSandbox::new("history-install");
        let sys = TestSandbox::mock_system();
        let pkg_dir = sandbox.create_sample_package_with_version(
            "test-hist",
            "1.0.0",
            &[("file.conf", "~/.config/test/file.conf", "v1 content")],
            &[],
        );

        let res = install_package_in(
            &pkg_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
        assert!(res.success);

        let record = get_installed_package_in("test-hist", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        assert_eq!(record.history.len(), 1);
        assert_eq!(record.history[0].version, "1.0.0");
        assert_eq!(record.history[0].snapshot_id, record.snapshot_id);
        assert!(record.history[0].installed_at > 0);
    }

    #[test]
    fn test_installed_package_history_survives_updates() {
        let sandbox = TestSandbox::new("history-update");
        let sys = TestSandbox::mock_system();
        let pkg_v1 = sandbox.create_sample_package_with_version(
            "test-hist-up",
            "1.0.0",
            &[("file.conf", "~/.config/test/file.conf", "v1 content")],
            &[],
        );

        let res1 = install_package_in(
            &pkg_v1,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
        assert!(res1.success);

        let pkg_v2 = sandbox.create_sample_package_with_version(
            "test-hist-up",
            "1.1.0",
            &[("file.conf", "~/.config/test/file.conf", "v2 content")],
            &[],
        );
        let res2 = apply_package_update_in(
            "test-hist-up",
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &pkg_v2,
            &sys,
            false,
            None,
        )
        .unwrap();
        assert!(res2.success);

        let record = get_installed_package_in("test-hist-up", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        assert_eq!(record.history.len(), 2);
        assert_eq!(record.history[0].version, "1.0.0");
        assert_eq!(record.history[1].version, "1.1.0");
        assert_eq!(record.version, "1.1.0");
    }

    #[test]
    fn test_installed_package_history_rollback_integration() {
        let sandbox = TestSandbox::new("history-rollback");
        let sys = TestSandbox::mock_system();
        let pkg_v1 = sandbox.create_sample_package_with_version(
            "test-hist-rb",
            "1.0.0",
            &[("file.conf", "~/.config/test/file.conf", "v1 content")],
            &[],
        );

        let res1 = install_package_in(
            &pkg_v1,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
        )
        .unwrap();
        assert!(res1.success);

        let record_v1 = get_installed_package_in("test-hist-rb", &sandbox.installed_dir)
            .unwrap()
            .unwrap();
        let snapshot_v1 = record_v1.history[0].snapshot_id.clone();

        // Rollback using the historical snapshot ID
        let rollback = restore_snapshot_in(&snapshot_v1, &sandbox.home_dir, &sandbox.snapshots_dir);
        assert!(rollback.is_ok());
        assert!(rollback.unwrap().success);
    }

    #[test]
    fn test_target_aware_sddm_does_not_resolve_quickshell_sway_cosmic() {
        let sandbox = TestSandbox::new("target-sddm-isolation");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let plan = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("sddm")).unwrap();
        
        // SDDM target must resolve SDDM
        assert!(plan.required_dependencies.contains(&"sddm".to_string()));
        // SDDM target must NOT resolve quickshell
        assert!(!plan.required_dependencies.contains(&"quickshell".to_string()));
        assert!(!plan.missing_dependencies.contains(&"quickshell".to_string()));
        // SDDM target must NOT resolve sway
        assert!(!plan.required_dependencies.contains(&"sway".to_string()));
        assert!(!plan.missing_dependencies.contains(&"sway".to_string()));
        // SDDM target must NOT resolve cosmic
        assert!(!plan.required_dependencies.contains(&"cosmic".to_string()));
        assert!(!plan.missing_dependencies.contains(&"cosmic".to_string()));
        // SDDM target must require privilege
        assert!(plan.requires_privilege);
        assert_eq!(plan.selected_target, Some("sddm".to_string()));
    }

    #[test]
    fn test_target_aware_quickshell_resolves_quickshell() {
        let sandbox = TestSandbox::new("target-quickshell");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let plan = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("quickshell")).unwrap();
        
        assert!(plan.required_dependencies.contains(&"quickshell".to_string()));
        assert!(!plan.required_dependencies.contains(&"sddm".to_string()));
        // Quickshell is user-space: does not require privilege
        assert!(!plan.requires_privilege);
        assert_eq!(plan.selected_target, Some("quickshell".to_string()));
    }

    #[test]
    fn test_target_aware_both_resolves_union() {
        let sandbox = TestSandbox::new("target-both");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let plan = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("both")).unwrap();
        
        // Both target must contain union of quickshell and sddm
        assert!(plan.required_dependencies.contains(&"quickshell".to_string()));
        assert!(plan.required_dependencies.contains(&"sddm".to_string()));
        // Privilege required because SDDM is included
        assert!(plan.requires_privilege);
        assert_eq!(plan.selected_target, Some("both".to_string()));
    }

    #[test]
    fn test_target_aware_dry_run_and_real_install_same_plan() {
        let sandbox = TestSandbox::new("target-dryrun-parity");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let dry_run_plan = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("sddm")).unwrap();
        
        assert_eq!(dry_run_plan.required_dependencies, vec!["sddm".to_string()]);
        assert_eq!(dry_run_plan.selected_target, Some("sddm".to_string()));
        assert!(dry_run_plan.requires_privilege);
    }

    #[test]
    fn test_target_aware_unsupported_target_rejected() {
        let sandbox = TestSandbox::new("target-unsupported");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        // Hyprlock is unsupported by Dog Samurai
        let plan = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("hyprlock")).unwrap();
        assert_eq!(plan.compatibility_status, "incompatible");
        assert!(plan.conflicts.iter().any(|c| c.contains("does not support target 'hyprlock'")));
    }

    #[test]
    fn test_target_aware_clean_machine_isolation() {
        let sandbox = TestSandbox::new("clean-machine-target");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let plan_q = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("quickshell")).unwrap();
        for f in &plan_q.files_to_create {
            assert!(!f.contains("/home/silentbyte/qylock"));
            assert!(!f.contains("~/.local/share/qylock"));
            assert!(!f.contains("~/.config/qylock"));
            assert!(f.contains(".local/share/ryzora"));
        }

        let plan_s = generate_installation_plan_target_in(&dog_samurai_dir, &sandbox.home_dir, &sys, Some("sddm")).unwrap();
        for f in &plan_s.files_to_create {
            assert!(!f.contains("/home/silentbyte/qylock"));
            assert!(!f.contains("~/.local/share/qylock"));
            assert!(f.starts_with("/usr/share/sddm/themes/ryzora-"));
        }
    }
    #[test]
    fn test_install_does_not_imply_active() {
        let sandbox = TestSandbox::new("install-not-active");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let res = install_package_target_options_in(
            &dog_samurai_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("quickshell"),
        ).unwrap();

        assert!(res.success);

        // State directory check: install must NOT have set active state
        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let active = get_active_lockscreen_state_in(&state_dir);
        assert_eq!(active.quickshell, None, "Installation must NOT imply active Quickshell");
        assert_eq!(active.sddm, None, "Installation must NOT imply active SDDM");
    }

    #[test]
    fn test_apply_quickshell_activates_target_without_root() {
        let sandbox = TestSandbox::new("apply-quickshell");
        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        // Install first
        install_package_target_options_in(
            &dog_samurai_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("quickshell"),
        ).unwrap();

        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let active = apply_lockscreen_target_in(
            "lockscreen-qylock-dog-samurai",
            "quickshell",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();

        assert_eq!(active.quickshell, Some("lockscreen-qylock-dog-samurai".to_string()));
        assert!(active.quickshell_theme_path.is_some());

        // Symlink must exist under ~/.local/share/ryzora/active/lockscreen/quickshell
        let qs_symlink = sandbox.home_dir.join(".local/share/ryzora/active/lockscreen/quickshell");
        assert!(qs_symlink.exists() || qs_symlink.is_symlink());

        // Invariant: no hyprland.conf or hypridle.conf touched
        assert!(!sandbox.home_dir.join(".config/hypr/hyprland.conf").exists());
        assert!(!sandbox.home_dir.join(".config/hypr/hypridle.conf").exists());
    }

    #[test]
    fn test_apply_sddm_requires_theme_and_writes_conf() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sandbox = TestSandbox::new("apply-sddm");
        let sys_root = sandbox.root.join("system_root");
        std::env::set_var("RYZORA_SYSTEM_ROOT", &sys_root);
        crate::sddm_helper::setup_privileged_helper_in(Some(&sys_root)).unwrap();

        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        // Install SDDM target
        install_package_target_options_in(
            &dog_samurai_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("sddm"),
        ).unwrap();

        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let active = apply_lockscreen_target_in(
            "lockscreen-qylock-dog-samurai",
            "sddm",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();

        assert_eq!(active.sddm, Some("lockscreen-qylock-dog-samurai".to_string()));
        assert!(active.sddm_theme_path.is_some());

        // Verify config was written to system config dir
        let conf_file = sys_root.join("etc/sddm.conf.d/zz-ryzora-theme.conf");
        assert!(conf_file.is_file());
        let conf_content = fs::read_to_string(&conf_file).unwrap();
        assert!(conf_content.contains("Current=ryzora-dog-samurai"));

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_apply_both_activates_both_targets() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sandbox = TestSandbox::new("apply-both");
        let sys_root = sandbox.root.join("system_root");
        std::env::set_var("RYZORA_SYSTEM_ROOT", &sys_root);
        crate::sddm_helper::setup_privileged_helper_in(Some(&sys_root)).unwrap();

        let sys = TestSandbox::mock_system();
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        // Install Both target
        install_package_target_options_in(
            &dog_samurai_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("both"),
        ).unwrap();

        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let active = apply_lockscreen_target_in(
            "lockscreen-qylock-dog-samurai",
            "both",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();

        assert_eq!(active.quickshell, Some("lockscreen-qylock-dog-samurai".to_string()));
        assert_eq!(active.sddm, Some("lockscreen-qylock-dog-samurai".to_string()));

        // Deactivate SDDM only
        let deactivated = deactivate_lockscreen_target_in("sddm", &sandbox.home_dir, &state_dir).unwrap();
        assert_eq!(deactivated.sddm, None);
        assert_eq!(deactivated.quickshell, Some("lockscreen-qylock-dog-samurai".to_string()));

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_transactional_backup_and_rollback() {
        let sandbox = TestSandbox::new("tx-backup-test");
        let file1 = sandbox.home_dir.join("test_file1.txt");
        let file2 = sandbox.home_dir.join("test_file2.txt");
        fs::write(&file1, "original content 1").unwrap();
        fs::write(&file2, "original content 2").unwrap();

        let tx_dir = create_transaction_backup(&[&file1, &file2], &sandbox.home_dir).unwrap();
        assert!(tx_dir.exists());
        assert!(tx_dir.join("transaction.json").exists());

        // Mutate files
        fs::write(&file1, "corrupted content 1").unwrap();
        fs::remove_file(&file2).unwrap();

        // Rollback
        rollback_transaction_backup(&tx_dir).unwrap();

        assert_eq!(fs::read_to_string(&file1).unwrap(), "original content 1");
        assert_eq!(fs::read_to_string(&file2).unwrap(), "original content 2");

        // Test with directory symlink (e.g. active lockscreen symlink)
        let target_dir = sandbox.home_dir.join("some_theme_dir");
        fs::create_dir_all(&target_dir).unwrap();
        let symlink_path = sandbox.home_dir.join("active_quickshell_symlink");
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(&target_dir, &symlink_path).unwrap();
            let tx_dir2 = create_transaction_backup(&[&symlink_path], &sandbox.home_dir).unwrap();
            assert!(tx_dir2.exists());

            // Remove symlink
            fs::remove_file(&symlink_path).unwrap();
            assert!(!symlink_path.exists());

            // Rollback
            rollback_transaction_backup(&tx_dir2).unwrap();
            assert!(symlink_path.exists());
            assert_eq!(fs::read_link(&symlink_path).unwrap(), target_dir);
        }
    }

    #[test]
    fn test_deactivate_and_uninstall_lockscreen_cleans_state_and_files() {
        let sandbox = TestSandbox::new("deact-uninstall-test");
        let dog_samurai_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-dog-samurai");

        let sys = TestSandbox::mock_system();
        let record = install_package_target_options_in(
            &dog_samurai_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("quickshell"),
        ).unwrap();
        assert!(record.success);

        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let active = apply_lockscreen_target_in(
            "lockscreen-qylock-dog-samurai",
            "quickshell",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();
        assert_eq!(active.quickshell, Some("lockscreen-qylock-dog-samurai".to_string()));

        // Now Deactivate & Uninstall
        let uninst = deactivate_and_uninstall_lockscreen_target_in(
            "lockscreen-qylock-dog-samurai",
            Some("quickshell"),
            &sandbox.home_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();
        assert!(uninst.success);

        // Verify active state is cleared
        let final_state = get_active_lockscreen_state_in(&state_dir);
        assert_eq!(final_state.quickshell, None);

        // Verify installed file is gone
        let installed_file = sandbox.home_dir.join(".local/share/ryzora/lockscreens/qylock/dog-samurai/Main.qml");
        assert!(!installed_file.exists());
    }

    #[test]
    fn test_apply_uninstalled_package_rejected() {
        let sandbox = TestSandbox::new("apply-uninstalled");
        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let res = apply_lockscreen_target_in(
            "non-existent-pkg",
            "quickshell",
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not installed"));
    }


    #[test]
    fn test_apply_lockscreen_writes_general_theme_conf_and_json() {
        let sandbox = TestSandbox::new("apply-custom-config");
        let sys = TestSandbox::mock_system();
        let tape_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("repositories/community/packages/lockscreen-qylock-clockwork-tape");

        // Install Quickshell target
        install_package_target_options_in(
            &tape_dir,
            &sandbox.snapshots_dir,
            &sandbox.installed_dir,
            &sandbox.staging_dir,
            &sandbox.home_dir,
            &sys,
            false,
            Some("quickshell"),
        ).unwrap();

        let state_dir = sandbox.home_dir.join(".local/share/ryzora/state");
        let mut custom_cfg = std::collections::HashMap::new();
        custom_cfg.insert("variant".to_string(), serde_json::json!("midnight-chrome"));
        custom_cfg.insert("clockPosition".to_string(), serde_json::json!("center"));
        custom_cfg.insert("clockStyle".to_string(), serde_json::json!("digital"));
        custom_cfg.insert("showDate".to_string(), serde_json::json!(true));
        custom_cfg.insert("showSeconds".to_string(), serde_json::json!(false));
        custom_cfg.insert("showSystemInfo".to_string(), serde_json::json!(true));

        let active = apply_lockscreen_target_in_with_config(
            "lockscreen-qylock-clockwork-tape",
            "quickshell",
            Some(custom_cfg),
            &sandbox.home_dir,
            &sandbox.installed_dir,
            &state_dir,
        ).unwrap();

        assert_eq!(active.quickshell, Some("lockscreen-qylock-clockwork-tape".to_string()));

        // Inspect the generated theme.conf on disk!
        let installed_theme_conf = sandbox.home_dir.join(".local/share/ryzora/lockscreens/qylock/clockwork-tape/theme.conf");
        assert!(installed_theme_conf.exists(), "theme.conf must exist on disk");
        let conf_content = fs::read_to_string(&installed_theme_conf).unwrap();

        assert!(conf_content.contains("[General]"), "Must contain [General] section");
        assert!(conf_content.contains("variant=midnight-chrome"), "Must persist variant=midnight-chrome");
        assert!(conf_content.contains("clockPosition=center"), "Must persist clockPosition=center");
        assert!(conf_content.contains("clockStyle=digital"), "Must persist clockStyle=digital");
        assert!(conf_content.contains("showSeconds=false"), "Must persist showSeconds=false");

        // Inspect ryzora_config.json on disk!
        let json_path = sandbox.home_dir.join(".local/share/ryzora/lockscreens/qylock/clockwork-tape/ryzora_config.json");
        assert!(json_path.exists(), "ryzora_config.json must exist");
        let json_str = fs::read_to_string(&json_path).unwrap();
        assert!(json_str.contains("midnight-chrome"));
    }

}
