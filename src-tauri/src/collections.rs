/// collections.rs — .ryzlist package collections (Phase 15.4)
///
/// A collection is a named, versioned, exportable list of Ryzora package IDs
/// with optional pinned versions. Collections are advisory manifests only —
/// they contain no executable installation instructions.
///
/// Installation of a collection routes through the existing installer/snapshot/
/// rollback pipeline — no second installer is introduced.
///
/// Collections are stored in ~/.local/share/ryzora/collections/ as JSON files.
/// Export format is .ryzlist JSON (human-readable, git-friendly).
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// .ryzlist Format
// ─────────────────────────────────────────────────────────────────────────────

/// Schema version for the .ryzlist format.
const RYZLIST_VERSION: &str = "1";

/// A package entry within a collection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CollectionPackage {
    /// Ryzora package ID. Validated against manifest_id_safe().
    pub id: String,
    /// Optional version requirement (e.g. ">=1.2.0"). null = any version.
    pub version_req: Option<String>,
    /// Optional preferred repository ID. null = any available repository.
    pub repository: Option<String>,
}

/// Full collection document (stored on disk and exported as .ryzlist).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// .ryzlist schema version. Currently "1".
    pub ryzora_collection: String,
    /// Stable slug ID for this collection (auto-generated from name).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Author / owner display name.
    pub author: Option<String>,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
    /// Ordered list of packages in this collection.
    pub packages: Vec<CollectionPackage>,
}

/// Lightweight summary for listing collections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub package_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Result of previewing a collection install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInstallPreview {
    pub collection_id: String,
    pub collection_name: String,
    pub total_packages: usize,
    pub already_installed: Vec<String>,
    pub to_install: Vec<String>,
    pub unavailable: Vec<String>,
    pub warnings: Vec<String>,
}

/// Result of installing a collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInstallResult {
    pub collection_id: String,
    pub total_packages: usize,
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
    pub rolled_back: bool,
    pub errors: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Paths
// ─────────────────────────────────────────────────────────────────────────────

pub fn collections_dir() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora/collections")
}

fn collection_path(id: &str) -> PathBuf {
    collections_dir().join(format!("{}.json", sanitize_id(id)))
}

// ─────────────────────────────────────────────────────────────────────────────
// ID / timestamp helpers
// ─────────────────────────────────────────────────────────────────────────────

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let diy = if is_leap(year) { 366 } else { 365 };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Produce a URL-safe slug from a collection name.
fn slugify(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse consecutive dashes and trim
    let mut out = String::new();
    let mut last_dash = false;
    for c in s.chars() {
        if c == '-' {
            if !last_dash && !out.is_empty() {
                out.push('-');
            }
            last_dash = true;
        } else {
            out.push(c);
            last_dash = false;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "collection".to_string()
    } else {
        out
    }
}

/// Make a collection ID unique within the collections directory.
fn unique_id(base: &str) -> String {
    let dir = collections_dir();
    let mut candidate = base.to_string();
    let mut n = 1u32;
    loop {
        let path = dir.join(format!("{}.json", sanitize_id(&candidate)));
        if !path.exists() {
            return candidate;
        }
        candidate = format!("{}-{}", base, n);
        n += 1;
    }
}

/// Sanitize an ID so it is safe to use as a file name component.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>()
        .to_lowercase()
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Validate a package ID using the same rules as installer::manifest_id_safe.
///
/// IDs must be non-empty, ≤64 chars, and contain only [a-z0-9._-].
/// This prevents path traversal, null bytes, and shell metacharacters.
fn validate_package_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err("Package ID must not be empty.".to_string());
    }
    if id.len() > 64 {
        return Err(format!("Package ID '{}' exceeds 64 characters.", id));
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(format!(
            "Package ID '{}' contains path traversal sequences.",
            id
        ));
    }
    let valid: bool = id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if !valid {
        return Err(format!(
            "Package ID '{}' contains invalid characters. Only [a-zA-Z0-9._-] are permitted.",
            id
        ));
    }
    Ok(())
}

/// Validate a collection document for schema compliance and ID safety.
/// Rejects executable content, path traversal, and schema violations.
pub fn validate_collection(col: &Collection) -> Result<(), String> {
    if col.ryzora_collection != RYZLIST_VERSION {
        return Err(format!(
            "Unsupported .ryzlist schema version '{}'. Expected '{}'.",
            col.ryzora_collection, RYZLIST_VERSION
        ));
    }
    if col.name.trim().is_empty() {
        return Err("Collection name must not be empty.".to_string());
    }
    if col.name.len() > 128 {
        return Err("Collection name must not exceed 128 characters.".to_string());
    }
    if col.packages.len() > 500 {
        return Err("Collection must not contain more than 500 packages.".to_string());
    }

    // Validate all package IDs
    let mut seen = HashSet::new();
    for pkg in &col.packages {
        validate_package_id(&pkg.id)?;
        if !seen.insert(pkg.id.clone()) {
            return Err(format!("Duplicate package ID '{}' in collection.", pkg.id));
        }
        // Validate version_req if present — must not contain shell metacharacters
        if let Some(ref vr) = pkg.version_req {
            if vr.len() > 32 {
                return Err(format!(
                    "version_req for '{}' exceeds 32 characters.",
                    pkg.id
                ));
            }
            let safe: bool = vr
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ">=<~^.*-+".contains(c));
            if !safe {
                return Err(format!(
                    "version_req '{}' for package '{}' contains invalid characters.",
                    vr, pkg.id
                ));
            }
        }
        // Validate repository ID if present
        if let Some(ref repo) = pkg.repository {
            if repo.contains("..") || repo.contains('/') {
                return Err(format!(
                    "repository ID '{}' for package '{}' contains path traversal.",
                    repo, pkg.id
                ));
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// CRUD operations
// ─────────────────────────────────────────────────────────────────────────────

/// Load a collection by ID.
pub fn load_collection(id: &str) -> Result<Collection, String> {
    let path = collection_path(id);
    if !path.is_file() {
        return Err(format!("Collection '{}' not found.", id));
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read collection '{}': {}", id, e))?;
    let col = serde_json::from_str::<Collection>(&raw)
        .map_err(|e| format!("Failed to parse collection '{}': {}", id, e))?;
    validate_collection(&col)?;
    Ok(col)
}

/// Save a collection to disk (atomic write: temp + rename).
pub fn save_collection_to_path(col: &Collection, path: &Path) -> Result<(), String> {
    validate_collection(col)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create collections directory: {}", e))?;
        let _ = crate::crypto::set_secure_permissions(parent, 0o700);
    }

    let json = serde_json::to_string_pretty(col)
        .map_err(|e| format!("Failed to serialize collection: {}", e))?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tmp = path
        .parent()
        .unwrap_or_else(|| Path::new("/tmp"))
        .join(format!(".col-{}.tmp.{}", sanitize_id(&col.id), nanos));

    fs::write(&tmp, &json).map_err(|e| format!("Failed to write collection temp file: {}", e))?;
    let _ = crate::crypto::set_secure_permissions(&tmp, 0o600);

    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to atomically rename collection file: {}", e)
    })?;
    let _ = crate::crypto::set_secure_permissions(path, 0o600);

    Ok(())
}

/// Save a collection to disk (atomic write: temp + rename) with hardened permissions.
pub fn save_collection(col: &Collection) -> Result<(), String> {
    save_collection_to_path(col, &collection_path(&col.id))
}

/// List all collections stored locally.
pub fn list_collections_impl() -> Result<Vec<CollectionSummary>, String> {
    let dir = collections_dir();
    if !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    let entries =
        fs::read_dir(&dir).map_err(|e| format!("Failed to read collections directory: {}", e))?;

    for entry in entries.filter_map(|e| e.ok()) {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&p) {
            if let Ok(col) = serde_json::from_str::<Collection>(&raw) {
                summaries.push(CollectionSummary {
                    id: col.id.clone(),
                    name: col.name.clone(),
                    description: col.description.clone(),
                    package_count: col.packages.len(),
                    created_at: col.created_at.clone(),
                    updated_at: col.updated_at.clone(),
                });
            }
        }
    }

    // Sort by name
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// List all saved collections.
#[tauri::command]
pub fn list_collections() -> Result<Vec<CollectionSummary>, String> {
    list_collections_impl()
}

/// Load a specific collection by ID.
#[tauri::command]
pub fn get_collection(id: String) -> Result<Collection, String> {
    load_collection(&id)
}

/// Create a new empty collection.
#[tauri::command]
pub fn create_collection(name: String, description: Option<String>) -> Result<Collection, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Collection name must not be empty.".to_string());
    }
    if name.len() > 128 {
        return Err("Collection name must not exceed 128 characters.".to_string());
    }

    let base_id = slugify(&name);
    let id = unique_id(&base_id);
    let now = now_iso();

    let col = Collection {
        ryzora_collection: RYZLIST_VERSION.to_string(),
        id,
        name,
        description,
        author: None,
        created_at: now.clone(),
        updated_at: now,
        packages: Vec::new(),
    };

    save_collection(&col)?;
    Ok(col)
}

/// Rename an existing collection.
#[tauri::command]
pub fn rename_collection(id: String, new_name: String) -> Result<Collection, String> {
    let mut col = load_collection(&id)?;
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err("Collection name must not be empty.".to_string());
    }
    col.name = new_name;
    col.updated_at = now_iso();
    save_collection(&col)?;
    Ok(col)
}

/// Delete a collection.
#[tauri::command]
pub fn delete_collection(id: String) -> Result<(), String> {
    let path = collection_path(&id);
    if !path.is_file() {
        return Err(format!("Collection '{}' not found.", id));
    }
    fs::remove_file(&path).map_err(|e| format!("Failed to delete collection '{}': {}", id, e))?;
    Ok(())
}

/// Add a package to a collection.
#[tauri::command]
pub fn add_package_to_collection(
    collection_id: String,
    package_id: String,
    version_req: Option<String>,
    repository: Option<String>,
) -> Result<Collection, String> {
    validate_package_id(&package_id)?;
    let mut col = load_collection(&collection_id)?;

    // Reject duplicates
    if col.packages.iter().any(|p| p.id == package_id) {
        return Err(format!(
            "Package '{}' is already in collection '{}'.",
            package_id, collection_id
        ));
    }
    if col.packages.len() >= 500 {
        return Err("Collection already contains the maximum of 500 packages.".to_string());
    }

    col.packages.push(CollectionPackage {
        id: package_id,
        version_req,
        repository,
    });
    col.updated_at = now_iso();
    // Re-validate the whole collection before saving
    validate_collection(&col)?;
    save_collection(&col)?;
    Ok(col)
}

/// Remove a package from a collection.
#[tauri::command]
pub fn remove_package_from_collection(
    collection_id: String,
    package_id: String,
) -> Result<Collection, String> {
    let mut col = load_collection(&collection_id)?;
    let original_len = col.packages.len();
    col.packages.retain(|p| p.id != package_id);
    if col.packages.len() == original_len {
        return Err(format!(
            "Package '{}' not found in collection '{}'.",
            package_id, collection_id
        ));
    }
    col.updated_at = now_iso();
    save_collection(&col)?;
    Ok(col)
}

/// Export a collection as a .ryzlist JSON string.
#[tauri::command]
pub fn export_collection(id: String) -> Result<String, String> {
    let col = load_collection(&id)?;
    serde_json::to_string_pretty(&col)
        .map_err(|e| format!("Failed to serialize collection for export: {}", e))
}

/// Import a collection from a .ryzlist JSON string.
/// Validates schema version, package IDs, and rejects executable content.
#[tauri::command]
pub fn import_collection(json: String) -> Result<Collection, String> {
    let mut col: Collection =
        serde_json::from_str(&json).map_err(|e| format!("Failed to parse .ryzlist JSON: {}", e))?;

    validate_collection(&col)?;

    // Assign a local unique ID (imported collections may conflict with existing ones)
    let base_id = slugify(&col.name);
    col.id = unique_id(&base_id);
    col.updated_at = now_iso();

    save_collection(&col)?;

    crate::notifications::notify(
        crate::notifications::NotificationKind::CollectionImported,
        crate::notifications::Severity::Info,
        &format!("Collection '{}' imported", col.name),
        &format!("{} packages imported", col.packages.len()),
        None,
        None,
    );

    Ok(col)
}

// ─────────────────────────────────────────────────────────────────────────────
// Resolution & Transactional Installation Pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Resolved package ready for validation/staging.
#[derive(Debug, Clone)]
pub struct ResolvedPackageEntry {
    pub package_id: String,
    pub package_dir: PathBuf,
    pub manifest: crate::manifest::RyzoraManifest,
    pub repository_id: Option<String>,
}

/// Package provider bridge connecting collections resolution with DependencyResolver.
pub struct CollectionPackageProvider<'a> {
    pub manager: &'a crate::repository::RepositoryManager,
    pub custom_root: Option<&'a Path>,
}

impl<'a> crate::dependency::PackageProvider for CollectionPackageProvider<'a> {
    fn get_manifest(&self, package_id: &str) -> Result<crate::manifest::RyzoraManifest, String> {
        if let Some(root) = self.custom_root {
            let p = root.join(package_id).join("manifest.json");
            if p.is_file() {
                return crate::installer::load_package_manifest(&root.join(package_id));
            }
        }
        self.manager.get_package_manifest(package_id)
    }

    fn get_repository_id(&self, package_id: &str) -> Option<String> {
        self.manager.find_repository_for_package(package_id)
    }
}

/// Resolve a package in a collection enforcing repository pinning if specified.
pub fn resolve_collection_package(
    pkg: &CollectionPackage,
    manager: &crate::repository::RepositoryManager,
    custom_packages_root: Option<&Path>,
) -> Result<ResolvedPackageEntry, String> {
    validate_package_id(&pkg.id)?;

    if let Some(ref repo_id) = pkg.repository {
        let repo = manager.get_repository_by_id(repo_id).ok_or_else(|| {
            format!(
                "Pinned repository '{}' not found for package '{}'",
                repo_id, pkg.id
            )
        })?;
        let manifest = repo.get_package_manifest(&pkg.id).map_err(|_| {
            format!(
                "Package '{}' not found in pinned repository '{}'",
                pkg.id, repo_id
            )
        })?;
        let dir = repo.get_package_dir(&pkg.id).map_err(|_| {
            format!(
                "Package directory for '{}' not available in pinned repository '{}'",
                pkg.id, repo_id
            )
        })?;
        Ok(ResolvedPackageEntry {
            package_id: pkg.id.clone(),
            package_dir: dir,
            manifest,
            repository_id: Some(repo_id.clone()),
        })
    } else {
        let dir = crate::installer::find_package_dir(&pkg.id, custom_packages_root)
            .map_err(|e| format!("Package '{}' not found: {}", pkg.id, e))?;
        let manifest = crate::installer::load_package_manifest(&dir)
            .map_err(|e| format!("Failed to load manifest for '{}': {}", pkg.id, e))?;
        let repo_id = manager.find_repository_for_package(&pkg.id);
        Ok(ResolvedPackageEntry {
            package_id: pkg.id.clone(),
            package_dir: dir,
            manifest,
            repository_id: repo_id,
        })
    }
}

/// Validate that a package's SemVer version satisfies the .ryzlist version requirement.
pub fn check_package_version_requirement(
    pkg_id: &str,
    actual_version: &str,
    version_req: Option<&str>,
) -> Result<(), String> {
    if let Some(req_str) = version_req {
        let req = semver::VersionReq::parse(req_str).map_err(|e| {
            format!(
                "Invalid version requirement '{}' for package '{}': {}",
                req_str, pkg_id, e
            )
        })?;
        let ver = semver::Version::parse(actual_version).map_err(|e| {
            format!(
                "Package '{}' has invalid semver version '{}': {}",
                pkg_id, actual_version, e
            )
        })?;
        if !req.matches(&ver) {
            return Err(format!(
                "Package '{}' version '{}' does not satisfy requirement '{}'",
                pkg_id, actual_version, req_str
            ));
        }
    }
    Ok(())
}

/// Preview installing a collection against specified system paths.
pub fn preview_collection_install_in(
    col: &Collection,
    installed_root: &Path,
    system_info: &crate::system::SystemInfo,
    manager: &crate::repository::RepositoryManager,
    custom_packages_root: Option<&Path>,
) -> Result<CollectionInstallPreview, String> {
    validate_collection(col)?;

    let installed =
        crate::installer::list_installed_packages_in(installed_root).unwrap_or_default();
    let installed_map: HashMap<String, String> = installed
        .into_iter()
        .map(|r| (r.package_id, r.version))
        .collect();

    let mut already_installed = Vec::new();
    let mut to_install = Vec::new();
    let mut unavailable = Vec::new();
    let mut warnings = Vec::new();

    let provider = CollectionPackageProvider {
        manager,
        custom_root: custom_packages_root,
    };
    let resolver = crate::dependency::DependencyResolver::new(&provider, system_info);

    for pkg in &col.packages {
        if let Some(installed_ver) = installed_map.get(&pkg.id) {
            if let Err(e) = check_package_version_requirement(
                &pkg.id,
                installed_ver,
                pkg.version_req.as_deref(),
            ) {
                warnings.push(format!("Installed {}", e));
            }
            already_installed.push(pkg.id.clone());
            continue;
        }

        match resolve_collection_package(pkg, manager, custom_packages_root) {
            Ok(resolved) => {
                if let Err(e) = check_package_version_requirement(
                    &pkg.id,
                    &resolved.manifest.version,
                    pkg.version_req.as_deref(),
                ) {
                    unavailable.push(pkg.id.clone());
                    warnings.push(e);
                    continue;
                }

                let report = resolver.resolve_manifest(&resolved.manifest);
                if !report.missing_required.is_empty() {
                    unavailable.push(pkg.id.clone());
                    warnings.push(format!(
                        "Package '{}' has missing required dependencies: {}",
                        pkg.id,
                        report.missing_required.join(", ")
                    ));
                    continue;
                }
                if !report.conflicts.is_empty() {
                    unavailable.push(pkg.id.clone());
                    warnings.push(format!(
                        "Package '{}' has dependency conflicts: {}",
                        pkg.id,
                        report.conflicts.join("; ")
                    ));
                    continue;
                }

                to_install.push(pkg.id.clone());
            }
            Err(e) => {
                unavailable.push(pkg.id.clone());
                warnings.push(e);
            }
        }
    }

    Ok(CollectionInstallPreview {
        collection_id: col.id.clone(),
        collection_name: col.name.clone(),
        total_packages: col.packages.len(),
        already_installed,
        to_install,
        unavailable,
        warnings,
    })
}

/// Preview installing a collection — reports what would be installed,
/// what is already installed, and what is unavailable.
#[tauri::command]
pub fn preview_collection_install(
    collection_id: String,
) -> Result<CollectionInstallPreview, String> {
    let col = load_collection(&collection_id)?;
    let installed_root = crate::installer::get_ryzora_installed_dir();
    let sys = crate::system::detect_system_info();
    let manager = crate::repository::create_default_manager();
    preview_collection_install_in(&col, &installed_root, &sys, &manager, None)
}

/// Transactional batch installation of a collection with a single snapshot boundary
/// and automatic rollback if ANY package installation fails.
pub fn install_collection_in(
    col: &Collection,
    snapshots_root: &Path,
    installed_root: &Path,
    staging_root: &Path,
    home_dir: &Path,
    system_info: &crate::system::SystemInfo,
    manager: &crate::repository::RepositoryManager,
    custom_packages_root: Option<&Path>,
) -> Result<CollectionInstallResult, String> {
    validate_collection(col)?;

    let installed =
        crate::installer::list_installed_packages_in(installed_root).unwrap_or_default();
    let installed_ids: HashSet<String> = installed.iter().map(|r| r.package_id.clone()).collect();
    let installed_map: HashMap<String, String> = installed
        .into_iter()
        .map(|r| (r.package_id, r.version))
        .collect();

    let mut skipped = Vec::new();
    let mut packages_to_install: Vec<ResolvedPackageEntry> = Vec::new();
    let mut pre_errors = Vec::new();

    let provider = CollectionPackageProvider {
        manager,
        custom_root: custom_packages_root,
    };
    let resolver = crate::dependency::DependencyResolver::new(&provider, system_info);

    // 1. Pre-flight resolve all packages in collection
    for pkg in &col.packages {
        if let Some(installed_ver) = installed_map.get(&pkg.id) {
            if let Err(e) = check_package_version_requirement(
                &pkg.id,
                installed_ver,
                pkg.version_req.as_deref(),
            ) {
                pre_errors.push(format!("Installed {}", e));
            } else {
                skipped.push(pkg.id.clone());
            }
            continue;
        }

        match resolve_collection_package(pkg, manager, custom_packages_root) {
            Ok(resolved) => {
                if let Err(e) = check_package_version_requirement(
                    &pkg.id,
                    &resolved.manifest.version,
                    pkg.version_req.as_deref(),
                ) {
                    pre_errors.push(e);
                    continue;
                }

                let report = resolver.resolve_manifest(&resolved.manifest);
                if !report.missing_required.is_empty() {
                    pre_errors.push(format!(
                        "Package '{}' has missing required dependencies: {}",
                        pkg.id,
                        report.missing_required.join(", ")
                    ));
                    continue;
                }
                if !report.conflicts.is_empty() {
                    pre_errors.push(format!(
                        "Package '{}' has dependency conflicts: {}",
                        pkg.id,
                        report.conflicts.join("; ")
                    ));
                    continue;
                }

                // If package declares uninstalled dependencies, include them first in install order
                for dep_id in &report.install_order {
                    if dep_id != &pkg.id
                        && !installed_ids.contains(dep_id)
                        && !packages_to_install.iter().any(|p| &p.package_id == dep_id)
                    {
                        let dep_pkg = CollectionPackage {
                            id: dep_id.clone(),
                            version_req: None,
                            repository: None,
                        };
                        match resolve_collection_package(&dep_pkg, manager, custom_packages_root) {
                            Ok(dep_resolved) => packages_to_install.push(dep_resolved),
                            Err(e) => pre_errors.push(format!(
                                "Required dependency '{}' cannot be resolved: {}",
                                dep_id, e
                            )),
                        }
                    }
                }

                if !packages_to_install.iter().any(|p| p.package_id == pkg.id) {
                    packages_to_install.push(resolved);
                }
            }
            Err(e) => {
                pre_errors.push(e);
            }
        }
    }

    // Fail safely if any package failed pre-flight
    if !pre_errors.is_empty() {
        let failed_ids: Vec<String> = col
            .packages
            .iter()
            .filter(|p| !skipped.contains(&p.id))
            .map(|p| p.id.clone())
            .collect();

        return Ok(CollectionInstallResult {
            collection_id: col.id.clone(),
            total_packages: col.packages.len(),
            installed: Vec::new(),
            skipped,
            failed: failed_ids,
            rolled_back: false,
            errors: pre_errors,
        });
    }

    if packages_to_install.is_empty() {
        return Ok(CollectionInstallResult {
            collection_id: col.id.clone(),
            total_packages: col.packages.len(),
            installed: Vec::new(),
            skipped,
            failed: Vec::new(),
            rolled_back: false,
            errors: Vec::new(),
        });
    }

    // 2. Collect ALL declared target paths across all packages to be installed
    let mut all_target_paths = Vec::new();
    for pkg in &packages_to_install {
        for f in &pkg.manifest.files {
            all_target_paths.push(f.target.clone());
        }
    }
    all_target_paths.sort();
    all_target_paths.dedup();

    // 3. Create SINGLE collection snapshot boundary before installing anything
    let snap_label = format!("collection-pre-install-{}", col.id);
    let snap_meta = crate::snapshot::create_snapshot_in(
        &snap_label,
        &all_target_paths,
        home_dir,
        snapshots_root,
    )
    .map_err(|e| format!("Failed to create collection snapshot: {}", e))?;

    let snap_valid = crate::snapshot::verify_snapshot_in(&snap_meta.id, snapshots_root)
        .map_err(|e| format!("Collection snapshot verification error: {}", e))?;
    if !snap_valid {
        return Err(
            "Collection pre-install snapshot failed integrity verification — installation aborted before modification"
                .to_string(),
        );
    }

    // 4. Sequentially install each package
    let mut batch_installed: Vec<String> = Vec::new();
    let mut failed_pkg_id: Option<String> = None;
    let mut install_errors: Vec<String> = Vec::new();

    for pkg in &packages_to_install {
        match crate::installer::install_package_in(
            &pkg.package_dir,
            snapshots_root,
            installed_root,
            staging_root,
            home_dir,
            system_info,
        ) {
            Ok(install_res) => {
                if install_res.success {
                    batch_installed.push(pkg.package_id.clone());
                    crate::notifications::notify(
                        crate::notifications::NotificationKind::PackageInstalled,
                        crate::notifications::Severity::Info,
                        &format!("Installed '{}'", pkg.package_id),
                        &format!("Installed via collection '{}'", col.name),
                        Some(&pkg.package_id),
                        None,
                    );
                } else {
                    failed_pkg_id = Some(pkg.package_id.clone());
                    install_errors.extend(install_res.errors);
                    break;
                }
            }
            Err(e) => {
                failed_pkg_id = Some(pkg.package_id.clone());
                install_errors.push(format!("{}: {}", pkg.package_id, e));
                break;
            }
        }
    }

    // 5. If ANY package failed: ATOMIC TRANSACTIONAL ROLLBACK
    if let Some(failed_id) = failed_pkg_id {
        // Remove all installed metadata records for packages installed in this batch
        for id in &batch_installed {
            let record_file = installed_root.join(format!("{}.json", id));
            let _ = fs::remove_file(&record_file);
        }

        // Restore the single collection snapshot to revert all filesystem changes
        let rollback_res =
            crate::snapshot::restore_snapshot_in(&snap_meta.id, home_dir, snapshots_root);
        let restored_ok = rollback_res.map(|r| r.success).unwrap_or(false);

        crate::notifications::notify(
            crate::notifications::NotificationKind::RollbackOccurred,
            crate::notifications::Severity::Warning,
            &format!(
                "Collection '{}' installation failed — rolled back",
                col.name
            ),
            &format!(
                "Failed on package '{}'. Entire collection was atomically rolled back.",
                failed_id
            ),
            Some(&failed_id),
            None,
        );

        let mut errors = vec![format!(
            "Collection installation failed on package '{}': {}. Complete collection rollback was {}.",
            failed_id,
            install_errors.join("; "),
            if restored_ok {
                "successful"
            } else {
                "attempted with errors"
            }
        )];
        errors.extend(install_errors);

        return Ok(CollectionInstallResult {
            collection_id: col.id.clone(),
            total_packages: col.packages.len(),
            installed: Vec::new(),
            skipped,
            failed: vec![failed_id],
            rolled_back: true,
            errors,
        });
    }

    // 6. Complete success
    Ok(CollectionInstallResult {
        collection_id: col.id.clone(),
        total_packages: col.packages.len(),
        installed: batch_installed,
        skipped,
        failed: Vec::new(),
        rolled_back: false,
        errors: Vec::new(),
    })
}

/// Install a collection by routing each package through the existing installer
/// pipeline with a single snapshot boundary and automatic rollback.
#[tauri::command]
pub fn install_collection(collection_id: String) -> Result<CollectionInstallResult, String> {
    let col = load_collection(&collection_id)?;

    let home = crate::snapshot::get_home_dir();
    let snapshots_root = crate::snapshot::get_ryzora_snapshots_dir();
    let installed_root = crate::installer::get_ryzora_installed_dir();
    let staging_root = crate::installer::get_ryzora_staging_dir();
    let sys = crate::system::detect_system_info();
    let manager = crate::repository::create_default_manager();

    install_collection_in(
        &col,
        &snapshots_root,
        &installed_root,
        &staging_root,
        &home,
        &sys,
        &manager,
        None,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::SystemInfo;
    use std::env;

    fn temp_test_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let p = env::temp_dir().join(format!("ryzora-col-test-{}-{}", label, nanos));
        let _ = fs::create_dir_all(&p);
        p
    }

    fn make_collection(id: &str, name: &str, packages: Vec<&str>) -> Collection {
        Collection {
            ryzora_collection: RYZLIST_VERSION.to_string(),
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            author: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            packages: packages
                .into_iter()
                .map(|p| CollectionPackage {
                    id: p.to_string(),
                    version_req: None,
                    repository: None,
                })
                .collect(),
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
            installed_components: vec![],
        }
    }

    fn create_test_pkg(
        packages_dir: &Path,
        id: &str,
        version: &str,
        target_file: &str,
        file_content: &str,
    ) -> PathBuf {
        let pkg_dir = packages_dir.join(id);
        fs::create_dir_all(pkg_dir.join("files")).unwrap();
        let manifest_json = format!(
            r#"{{
  "id": "{}",
  "name": "Test {}",
  "version": "{}",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": ["arch"],
    "required": [],
    "optional": []
  }},
  "files": [
    {{
      "source": "files/config.conf",
      "target": "{}",
      "description": "test config"
    }}
  ]
}}"#,
            id, id, version, target_file
        );
        fs::write(pkg_dir.join("manifest.json"), manifest_json).unwrap();
        fs::write(pkg_dir.join("files/config.conf"), file_content).unwrap();
        pkg_dir
    }

    #[test]
    fn test_collection_create_and_validate() {
        let col = make_collection(
            "my-rice",
            "My Rice",
            vec!["archcraft-hyprland", "catppuccin-waybar"],
        );
        assert!(validate_collection(&col).is_ok());
        assert_eq!(col.packages.len(), 2);
        assert_eq!(col.ryzora_collection, "1");
    }

    #[test]
    fn test_collection_validate_rejects_wrong_schema_version() {
        let mut col = make_collection("test", "Test", vec![]);
        col.ryzora_collection = "2".to_string();
        assert!(validate_collection(&col).is_err());
        assert!(validate_collection(&col)
            .unwrap_err()
            .contains("schema version"));
    }

    #[test]
    fn test_collection_validate_rejects_empty_name() {
        let col = make_collection("test", "", vec![]);
        assert!(validate_collection(&col).is_err());
    }

    #[test]
    fn test_collection_validate_rejects_path_traversal_in_package_id() {
        let col = make_collection("test", "Test", vec!["../../../etc/passwd"]);
        let result = validate_collection(&col);
        assert!(
            result.is_err(),
            "Path traversal in package ID must be rejected"
        );
        assert!(result.unwrap_err().contains("path traversal"));
    }

    #[test]
    fn test_collection_validate_rejects_duplicate_package_ids() {
        let col = make_collection("test", "Test", vec!["pkg-a", "pkg-b", "pkg-a"]);
        let result = validate_collection(&col);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate"));
    }

    #[test]
    fn test_collection_validate_rejects_shell_metacharacters_in_version_req() {
        let mut col = make_collection("test", "Test", vec![]);
        col.packages.push(CollectionPackage {
            id: "pkg-a".to_string(),
            version_req: Some(">=1.0.0; rm -rf /".to_string()),
            repository: None,
        });
        let result = validate_collection(&col);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid characters"));
    }

    #[test]
    fn test_collection_validate_rejects_too_many_packages() {
        let packages: Vec<String> = (0..501).map(|i| format!("pkg-{:03}", i)).collect();
        let col = Collection {
            ryzora_collection: RYZLIST_VERSION.to_string(),
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            author: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            packages: packages
                .iter()
                .map(|p| CollectionPackage {
                    id: p.clone(),
                    version_req: None,
                    repository: None,
                })
                .collect(),
        };
        assert!(validate_collection(&col).is_err());
        assert!(validate_collection(&col).unwrap_err().contains("500"));
    }

    #[test]
    fn test_collection_export_import_round_trip() {
        let col = make_collection(
            "round-trip-test",
            "Round Trip Test",
            vec!["archcraft-hyprland", "catppuccin-waybar", "rofi-config"],
        );
        let json = serde_json::to_string_pretty(&col).unwrap();
        let parsed: Collection = serde_json::from_str(&json).unwrap();
        assert!(validate_collection(&parsed).is_ok());
        assert_eq!(parsed.name, col.name);
        assert_eq!(parsed.packages.len(), col.packages.len());
        assert_eq!(parsed.packages[0].id, "archcraft-hyprland");
        assert_eq!(parsed.packages[2].id, "rofi-config");
    }

    #[test]
    fn test_collection_import_rejects_invalid_schema_version() {
        let json = r#"{
            "ryzora_collection": "99",
            "id": "test",
            "name": "Test",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "packages": []
        }"#;
        let parsed: Result<Collection, _> = serde_json::from_str(json);
        if let Ok(col) = parsed {
            assert!(validate_collection(&col).is_err());
        }
    }

    #[test]
    fn test_collection_slugify() {
        assert_eq!(slugify("My Hyprland Rice"), "my-hyprland-rice");
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  Test  "), "test");
        assert_eq!(slugify("A---B"), "a-b");
        assert_eq!(slugify(""), "collection");
    }

    #[test]
    fn test_collection_sanitize_id_no_path_traversal() {
        let id = "../../../etc";
        let safe = sanitize_id(id);
        assert!(!safe.contains(".."));
        assert!(!safe.contains('/'));
    }

    #[test]
    fn test_collection_validate_package_id_safety() {
        assert!(validate_package_id("archcraft-hyprland").is_ok());
        assert!(validate_package_id("pkg.v2").is_ok());
        assert!(validate_package_id("../etc/passwd").is_err());
        assert!(validate_package_id("").is_err());
        assert!(validate_package_id("pkg with spaces").is_err());
        assert!(validate_package_id("pkg;rm").is_err());
        let long_id = "a".repeat(65);
        assert!(validate_package_id(&long_id).is_err());
    }

    #[test]
    fn test_collection_permission_hardening() {
        let dir = temp_test_dir("perms");
        let path = dir.join("test-col.json");
        let col = make_collection("test-col", "Test Collection", vec!["pkg-a"]);

        save_collection_to_path(&col, &path).unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir_mode = fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
            let file_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(dir_mode, 0o700, "Collections dir must be 0700");
            assert_eq!(file_mode, 0o600, "Collection file must be 0600");
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collection_version_req_enforced() {
        let dir = temp_test_dir("ver_req");
        let packages_dir = dir.join("packages");
        let installed_dir = dir.join("installed");
        fs::create_dir_all(&packages_dir).unwrap();
        fs::create_dir_all(&installed_dir).unwrap();

        // Create package with version 1.0.0
        create_test_pkg(
            &packages_dir,
            "test-pkg",
            "1.0.0",
            "~/.config/test/config.conf",
            "content",
        );

        // Collection requires >=2.0.0
        let mut col = make_collection("ver-col", "Version Test", vec![]);
        col.packages.push(CollectionPackage {
            id: "test-pkg".to_string(),
            version_req: Some(">=2.0.0".to_string()),
            repository: None,
        });

        let sys = mock_system();
        let manager = crate::repository::create_default_manager();

        // Preview should flag package as unavailable with version mismatch warning
        let preview = preview_collection_install_in(
            &col,
            &installed_dir,
            &sys,
            &manager,
            Some(&packages_dir),
        )
        .unwrap();

        assert_eq!(preview.unavailable.len(), 1);
        assert_eq!(preview.unavailable[0], "test-pkg");
        assert!(preview
            .warnings
            .iter()
            .any(|w| w.contains("does not satisfy requirement")));

        // Install must fail safely without creating snapshots or modifying files
        let snapshots_dir = dir.join("snapshots");
        let staging_dir = dir.join("staging");
        let home_dir = dir.join("home");
        fs::create_dir_all(&snapshots_dir).unwrap();
        fs::create_dir_all(&staging_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        let res = install_collection_in(
            &col,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &manager,
            Some(&packages_dir),
        )
        .unwrap();

        assert_eq!(res.installed.len(), 0);
        assert_eq!(res.failed.len(), 1);
        assert!(!res.rolled_back); // Failed safely before install started
        assert!(res
            .errors
            .iter()
            .any(|e| e.contains("does not satisfy requirement")));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collection_repository_pinning_enforced() {
        let dir = temp_test_dir("repo_pin");
        let packages_dir = dir.join("packages");
        let installed_dir = dir.join("installed");
        fs::create_dir_all(&packages_dir).unwrap();
        fs::create_dir_all(&installed_dir).unwrap();

        create_test_pkg(
            &packages_dir,
            "test-pkg",
            "1.0.0",
            "~/.config/test/config.conf",
            "content",
        );

        // Package pinned to a nonexistent repository
        let mut col = make_collection("repo-col", "Repo Test", vec![]);
        col.packages.push(CollectionPackage {
            id: "test-pkg".to_string(),
            version_req: None,
            repository: Some("nonexistent-repo-123".to_string()),
        });

        let sys = mock_system();
        let manager = crate::repository::create_default_manager();

        let preview = preview_collection_install_in(
            &col,
            &installed_dir,
            &sys,
            &manager,
            Some(&packages_dir),
        )
        .unwrap();

        assert_eq!(preview.unavailable.len(), 1);
        assert!(preview
            .warnings
            .iter()
            .any(|w| w.contains("Pinned repository 'nonexistent-repo-123' not found")));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collection_single_snapshot_boundary_and_atomic_rollback() {
        let dir = temp_test_dir("rollback");
        let packages_dir = dir.join("packages");
        let snapshots_dir = dir.join("snapshots");
        let installed_dir = dir.join("installed");
        let staging_dir = dir.join("staging");
        let home_dir = dir.join("home");

        fs::create_dir_all(&packages_dir).unwrap();
        fs::create_dir_all(&snapshots_dir).unwrap();
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&staging_dir).unwrap();
        fs::create_dir_all(&home_dir.join(".config/app_a")).unwrap();

        // Pre-existing file in home directory
        let pre_existing_file = home_dir.join(".config/app_a/pre.txt");
        fs::write(&pre_existing_file, "initial pre-collection state").unwrap();

        // Pkg A: valid package targeting .config/app_a/file_a.conf
        create_test_pkg(
            &packages_dir,
            "pkg-a",
            "1.0.0",
            "~/.config/app_a/file_a.conf",
            "content_a",
        );

        // Pkg B: corrupted package whose source file is deleted so install fails
        let pkg_b_dir = create_test_pkg(
            &packages_dir,
            "pkg-b",
            "1.0.0",
            "~/.config/app_b/file_b.conf",
            "content_b",
        );
        // Deliberately delete payload file of pkg-b to force installation failure
        fs::remove_file(pkg_b_dir.join("files/config.conf")).unwrap();

        let col = make_collection("atomic-col", "Atomic Test", vec!["pkg-a", "pkg-b"]);
        let sys = mock_system();
        let manager = crate::repository::create_default_manager();

        // Run installation: pkg-a succeeds, pkg-b fails, entire collection rolls back
        let result = install_collection_in(
            &col,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &manager,
            Some(&packages_dir),
        )
        .unwrap();

        assert!(
            result.rolled_back,
            "Collection must report rolled_back = true"
        );
        assert_eq!(
            result.installed.len(),
            0,
            "Installed list must be empty after rollback"
        );
        assert_eq!(result.failed, vec!["pkg-b"], "Failed package must be pkg-b");

        // 1. Verify that pkg-a's file was completely rolled back and removed
        assert!(
            !home_dir.join("~/.config/app_a/file_a.conf").exists(),
            "Pkg A's installed file must be removed by the collection rollback"
        );

        // 2. Verify that pre-existing file was preserved intact
        let pre_content = fs::read_to_string(&pre_existing_file).unwrap();
        assert_eq!(pre_content, "initial pre-collection state");

        // 3. Verify that NO partial installed metadata records remain in installed_dir
        let installed_packages =
            crate::installer::list_installed_packages_in(&installed_dir).unwrap();
        assert!(
            installed_packages.is_empty(),
            "No partial installed metadata records must remain after collection rollback"
        );
        assert!(
            !installed_dir.join("pkg-a.json").exists(),
            "pkg-a.json record must be purged by rollback"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collection_unavailable_packages_fail_safely() {
        let dir = temp_test_dir("unavailable");
        let packages_dir = dir.join("packages");
        let snapshots_dir = dir.join("snapshots");
        let installed_dir = dir.join("installed");
        let staging_dir = dir.join("staging");
        let home_dir = dir.join("home");

        fs::create_dir_all(&packages_dir).unwrap();
        fs::create_dir_all(&snapshots_dir).unwrap();
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&staging_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        // Collection referencing a package that does not exist anywhere
        let col = make_collection("missing-col", "Missing Test", vec!["does-not-exist-xyz"]);
        let sys = mock_system();
        let manager = crate::repository::create_default_manager();

        let result = install_collection_in(
            &col,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &manager,
            Some(&packages_dir),
        )
        .unwrap();

        assert_eq!(result.installed.len(), 0);
        assert_eq!(result.failed, vec!["does-not-exist-xyz"]);
        assert!(!result.rolled_back); // Fails safely before modifying disk
        assert!(result.errors.iter().any(|e| e.contains("not found")));

        // Snapshots directory must have zero snapshots created
        let snapshots = crate::snapshot::list_snapshots_in(&snapshots_dir).unwrap();
        assert_eq!(
            snapshots.len(),
            0,
            "No snapshot must be created for unavailable packages"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_collection_zero_command_execution() {
        let col = make_collection("test", "Test", vec!["pkg-a"]);
        assert!(validate_collection(&col).is_ok());
    }
}
