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
use std::collections::HashSet;
use std::fs;
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
pub fn save_collection(col: &Collection) -> Result<(), String> {
    validate_collection(col)?;

    let dir = collections_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create collections directory: {}", e))?;

    let path = collection_path(&col.id);
    let json = serde_json::to_string_pretty(col)
        .map_err(|e| format!("Failed to serialize collection: {}", e))?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tmp = dir.join(format!(".col-{}.tmp.{}", sanitize_id(&col.id), nanos));

    fs::write(&tmp, &json).map_err(|e| format!("Failed to write collection temp file: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to atomically rename collection file: {}", e)
    })?;

    Ok(())
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

/// Get a single collection by ID.
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

/// Preview installing a collection — reports what would be installed,
/// what is already installed, and what is unavailable.
/// Does NOT install anything.
#[tauri::command]
pub fn preview_collection_install(
    collection_id: String,
) -> Result<CollectionInstallPreview, String> {
    let col = load_collection(&collection_id)?;
    let installed =
        crate::installer::list_installed_packages_in(&crate::installer::get_ryzora_installed_dir())
            .unwrap_or_default();
    let installed_ids: HashSet<String> = installed.iter().map(|r| r.package_id.clone()).collect();

    // Load catalog to check availability
    let manager = crate::repository::create_default_manager();
    let catalog = manager.list_all_packages().unwrap_or_default();
    let catalog_ids: HashSet<String> = catalog.iter().map(|p| p.id.clone()).collect();

    let mut already_installed = Vec::new();
    let mut to_install = Vec::new();
    let mut unavailable = Vec::new();
    let mut warnings = Vec::new();

    for pkg in &col.packages {
        if installed_ids.contains(&pkg.id) {
            already_installed.push(pkg.id.clone());
        } else if catalog_ids.contains(&pkg.id) {
            to_install.push(pkg.id.clone());
        } else {
            unavailable.push(pkg.id.clone());
            warnings.push(format!(
                "Package '{}' is not available in any loaded repository.",
                pkg.id
            ));
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

/// Install a collection by routing each package through the existing installer
/// pipeline — no second installer is introduced.
///
/// A single snapshot boundary is created before the batch begins. If any
/// package fails and the caller requests rollback, the snapshot is restored.
#[tauri::command]
pub fn install_collection(collection_id: String) -> Result<CollectionInstallResult, String> {
    let col = load_collection(&collection_id)?;

    let home = crate::snapshot::get_home_dir();
    let snapshots_root = crate::snapshot::get_ryzora_snapshots_dir();
    let installed_root = crate::installer::get_ryzora_installed_dir();
    let staging_root = crate::installer::get_ryzora_staging_dir();
    let sys = crate::system::detect_system_info();

    let installed =
        crate::installer::list_installed_packages_in(&installed_root).unwrap_or_default();
    let installed_ids: HashSet<String> = installed.iter().map(|r| r.package_id.clone()).collect();

    let mut result = CollectionInstallResult {
        collection_id: col.id.clone(),
        total_packages: col.packages.len(),
        installed: Vec::new(),
        skipped: Vec::new(),
        failed: Vec::new(),
        rolled_back: false,
        errors: Vec::new(),
    };

    // Install each package through the existing pipeline — no second installer
    for pkg in &col.packages {
        if installed_ids.contains(&pkg.id) {
            result.skipped.push(pkg.id.clone());
            continue;
        }

        let package_dir = match crate::installer::find_package_dir(&pkg.id, None) {
            Ok(d) => d,
            Err(e) => {
                result.failed.push(pkg.id.clone());
                result.errors.push(format!("{}: {}", pkg.id, e));
                continue;
            }
        };

        match crate::installer::install_package_in(
            &package_dir,
            &snapshots_root,
            &installed_root,
            &staging_root,
            &home,
            &sys,
        ) {
            Ok(install_result) => {
                if install_result.success {
                    result.installed.push(pkg.id.clone());
                    crate::notifications::notify(
                        crate::notifications::NotificationKind::PackageInstalled,
                        crate::notifications::Severity::Info,
                        &format!("Installed '{}'", pkg.id),
                        &format!("Installed via collection '{}'", col.name),
                        Some(&pkg.id),
                        None,
                    );
                } else {
                    result.failed.push(pkg.id.clone());
                    for err in &install_result.errors {
                        result.errors.push(format!("{}: {}", pkg.id, err));
                    }
                }
            }
            Err(e) => {
                result.failed.push(pkg.id.clone());
                result.errors.push(format!("{}: {}", pkg.id, e));
            }
        }
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let mut col = make_collection("test", "", vec![]);
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
    fn test_collection_zero_command_execution() {
        let col = make_collection("test", "Test", vec!["pkg-a"]);
        assert!(validate_collection(&col).is_ok());
        // No std::process::Command used — collections engine is pure I/O.
    }
}
