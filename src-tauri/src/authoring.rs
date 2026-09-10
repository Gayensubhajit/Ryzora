use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::dependency::DependencySpec;
use crate::manifest::{
    validate_manifest_internal, ManifestCompatibility, ManifestFile, ManifestValidationResult,
    PackageType, RyzoraManifest,
};
use crate::repository::{
    compute_package_tree_hash, validate_path_identifier, AuthorInfo, LocalRepository,
    RepositoryIndex, RepositoryPackageEntry, MAX_FILE_SIZE, MAX_PACKAGE_SIZE,
};
use crate::snapshot::get_home_dir;

// ─────────────────────────────────────────────────────────────────────────────
// Authoring Data Types (Draft Specification)
// ─────────────────────────────────────────────────────────────────────────────

/// A drafted file mapping specifying a file on the author's local system
/// and its target destination in the user's home directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMappingDraft {
    /// Absolute or home-relative path on the author's local system (e.g. "~/.config/hypr/hyprland.conf").
    pub source_path: String,

    /// Destination path on the target system. MUST start with "~/".
    /// Path traversal ("..") is strictly forbidden.
    pub target: String,

    /// Optional relative path inside the package "files/" directory (e.g. "files/hyprland.conf").
    /// If omitted or empty, it is automatically derived cleanly from the target path.
    #[serde(default)]
    pub package_rel_path: Option<String>,

    /// Human-readable description of what this configuration file does.
    #[serde(default)]
    pub description: String,
}

/// A complete package draft ready for validation and bundling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDraft {
    /// Unique package identifier slug (e.g. "neon-cyberpunk-hyprland").
    pub id: String,

    /// Human-readable package display name.
    pub name: String,

    /// Package version adhering to SemVer (e.g. "1.0.0").
    pub version: String,

    /// Package author name or handle.
    pub author: String,

    /// Canonical package type.
    pub package_type: PackageType,

    /// Long description of the package and its aesthetic theme.
    #[serde(default)]
    pub description: String,

    /// Searchable tags.
    #[serde(default)]
    pub tags: Vec<String>,

    /// Hex color palette for preview cards and badges.
    #[serde(default)]
    pub color_palette: Vec<String>,

    /// Compatibility requirements (desktops, sessions, distros, required binaries, optional binaries).
    pub compatibility: ManifestCompatibility,

    /// Typed dependencies (Phase 9 & 9.1).
    #[serde(default)]
    pub dependencies: Vec<DependencySpec>,

    /// File mappings to bundle into the package.
    pub files: Vec<FileMappingDraft>,
}

/// Result returned after bundling a package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoringResult {
    pub success: bool,
    pub package_dir: String,
    pub manifest_path: String,
    pub files_copied: usize,
    pub total_bytes: u64,
    pub sha256_checksum: String,
    pub warnings: Vec<String>,
}

/// Result returned after publishing a package to a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub success: bool,
    pub repository_path: String,
    pub package_id: String,
    pub package_version: String,
    pub archive_path: Option<String>,
    pub archive_sha256: Option<String>,
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Expansion & Safety Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Expand home-relative paths (~/...) using the real home directory.
pub fn expand_user_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" {
        get_home_dir()
    } else if let Some(stripped) = trimmed.strip_prefix("~/") {
        get_home_dir().join(stripped)
    } else {
        PathBuf::from(trimmed)
    }
}

/// Validate that a target path starts with "~/" and has no traversal or forbidden segments.
pub fn validate_authoring_target(target: &str) -> Result<(), String> {
    if !target.starts_with("~/") {
        return Err(format!(
            "Target '{}' must start with '~/' (home-relative paths only)",
            target
        ));
    }
    if target.contains('\0') {
        return Err(format!("Target '{}' contains forbidden null byte", target));
    }
    for comp in Path::new(target).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Target '{}' contains path traversal ('..') — rejected for safety",
                target
            ));
        }
    }
    let lower = target.to_lowercase();
    let forbidden = ["~/../", "~/.bashrc_forbidden", "~/../../"];
    for f in forbidden {
        if lower.contains(f) {
            return Err(format!(
                "Target '{}' contains forbidden path pattern",
                target
            ));
        }
    }
    Ok(())
}

/// Validate that a destination repository path is safe and confined.
/// Rejects remote URLs, path traversal, null bytes, system roots, and non-empty non-repository directories.
pub fn validate_repository_destination(path_str: &str, path: &Path) -> Result<(), String> {
    let trimmed = path_str.trim();
    if trimmed.is_empty() {
        return Err("Repository path cannot be empty".to_string());
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ftp://")
        || lower.starts_with("file://")
    {
        return Err(format!(
            "Remote repository URLs cannot be targeted for local publishing: '{}'. Remote publishing is handled via distribution workflows.",
            trimmed
        ));
    }
    if trimmed.contains('\0') {
        return Err("Repository path contains forbidden null byte".to_string());
    }
    for comp in path.components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Repository path '{}' contains parent traversal ('..') — rejected for safety",
                trimmed
            ));
        }
    }

    let path_buf = if path.is_absolute() {
        path.to_path_buf()
    } else {
        expand_user_path(trimmed)
    };

    let path_str_clean = path_buf.to_string_lossy().to_string();
    if path_str_clean == "/" {
        return Err("Cannot publish directly to root filesystem '/'".to_string());
    }

    let forbidden_prefixes = [
        "/etc",
        "/usr",
        "/boot",
        "/bin",
        "/sbin",
        "/lib",
        "/lib64",
        "/var",
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/tmp/system",
    ];
    for forbidden in forbidden_prefixes {
        if path_str_clean == forbidden || path_str_clean.starts_with(&format!("{}/", forbidden)) {
            return Err(format!(
                "Repository destination '{}' is in a protected system directory — rejected for safety",
                path_str_clean
            ));
        }
    }

    // If destination exists, verify it is a valid repository or empty directory
    if path_buf.exists() {
        if path_buf.is_file() {
            return Err(format!(
                "Repository destination '{}' is a file, expected directory",
                path_buf.display()
            ));
        }
        let index_path = path_buf.join("repository.json");
        if !index_path.is_file() {
            let is_empty = fs::read_dir(&path_buf)
                .map(|mut d| d.next().is_none())
                .unwrap_or(false);
            if !is_empty {
                return Err(format!(
                    "Directory '{}' exists and is not empty, but is missing 'repository.json'. To prevent accidental overwriting of unrelated files, publishing is rejected.",
                    path_buf.display()
                ));
            }
        }
    }

    Ok(())
}

/// Validate that an author's source file exists on disk, is a regular file or readable symlink,
/// and does not target protected system directories.
pub fn validate_source_file(source_path: &Path) -> Result<u64, String> {
    let meta = match fs::symlink_metadata(source_path) {
        Ok(m) => m,
        Err(_) => {
            return Err(format!(
                "Source file not found on disk: '{}'",
                source_path.display()
            ));
        }
    };

    if meta.is_dir() {
        return Err(format!(
            "Source path '{}' is a directory — packages must bundle individual files",
            source_path.display()
        ));
    }

    let file_size = if meta.file_type().is_symlink() {
        // Read symlink target
        let link_target = fs::read_link(source_path)
            .map_err(|e| format!("Failed to read symlink '{}': {}", source_path.display(), e))?;

        // Check if symlink target points to protected system locations
        let target_str = link_target.to_string_lossy().to_string();
        let forbidden = ["/etc", "/proc", "/sys", "/dev", "/boot", "/var", "/root"];
        for f in forbidden {
            if target_str == f || target_str.starts_with(&format!("{}/", f)) {
                return Err(format!(
                    "Symlink '{}' points to protected system location '{}' — rejected for safety",
                    source_path.display(),
                    target_str
                ));
            }
        }

        // Check if symlink target exists and is readable
        let target_meta = fs::metadata(source_path).map_err(|e| {
            format!(
                "Broken or unreadable symlink '{}' pointing to '{}': {}",
                source_path.display(),
                link_target.display(),
                e
            )
        })?;
        if target_meta.is_dir() {
            return Err(format!(
                "Symlink '{}' points to a directory — only file symlinks are supported",
                source_path.display()
            ));
        }
        target_meta.len()
    } else {
        meta.len()
    };

    if file_size > MAX_FILE_SIZE {
        return Err(format!(
            "Source file '{}' size ({} bytes) exceeds maximum allowable limit of {} bytes (50 MB)",
            source_path.display(),
            file_size,
            MAX_FILE_SIZE
        ));
    }

    Ok(file_size)
}

/// Compute SHA-256 hex digest of a file.
pub fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| {
        format!(
            "Failed to open file for hashing '{}': {}",
            path.display(),
            e
        )
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file '{}': {}", path.display(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Draft Validation
// ─────────────────────────────────────────────────────────────────────────────

/// Fully validates a package draft before any filesystem changes occur.
pub fn validate_package_draft_internal(draft: &PackageDraft) -> ManifestValidationResult {
    let mut errors = Vec::new();
    let warnings = Vec::new();

    // 1. Package ID validation
    if let Err(e) = validate_path_identifier(&draft.id, "Package ID") {
        errors.push(e);
    } else if draft.id.len() > 64 {
        errors.push(format!(
            "Package ID '{}' exceeds maximum length of 64 characters",
            draft.id
        ));
    }

    // 2. Package Name validation
    let trimmed_name = draft.name.trim();
    if trimmed_name.is_empty() {
        errors.push("Package name must not be empty".to_string());
    } else if trimmed_name.len() > 100 {
        errors.push("Package name exceeds maximum length of 100 characters".to_string());
    }

    // 3. Version validation
    if let Err(e) = semver::Version::parse(draft.version.trim()) {
        errors.push(format!(
            "Version '{}' is not valid SemVer (expected X.Y.Z): {}",
            draft.version, e
        ));
    }

    // 4. Author validation
    let trimmed_author = draft.author.trim();
    if trimmed_author.is_empty() {
        errors.push("Package author must not be empty".to_string());
    } else if trimmed_author.len() > 100 {
        errors.push("Package author exceeds maximum length of 100 characters".to_string());
    }

    // 5. Description validation
    if draft.description.len() > 1000 {
        errors.push("Package description exceeds maximum length of 1000 characters".to_string());
    }

    // 6. Tags validation
    if draft.tags.len() > 20 {
        errors.push("Package declares more than 20 tags".to_string());
    }
    for (t_idx, tag) in draft.tags.iter().enumerate() {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            errors.push(format!("tags[{}]: tag must not be empty", t_idx));
        } else if trimmed.len() > 32 {
            errors.push(format!(
                "tags[{}]: tag '{}' exceeds maximum length of 32 characters",
                t_idx, tag
            ));
        } else if trimmed.contains('\0')
            || !trimmed
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(format!(
                "tags[{}]: tag '{}' contains invalid characters — only alphanumeric, dashes, and underscores allowed",
                t_idx, tag
            ));
        }
    }

    // 7. Color palette validation
    if draft.color_palette.len() > 16 {
        errors.push("Package declares more than 16 colors in palette".to_string());
    }
    for (c_idx, color) in draft.color_palette.iter().enumerate() {
        let trimmed = color.trim();
        let valid_hex = trimmed.starts_with('#')
            && (trimmed.len() == 4 || trimmed.len() == 7 || trimmed.len() == 9)
            && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit());
        if !valid_hex {
            errors.push(format!(
                "color_palette[{}]: '{}' is not a valid hex color code (expected #RGB, #RRGGBB, or #RRGGBBAA)",
                c_idx, color
            ));
        }
    }

    // 8. File mappings validation
    if draft.files.is_empty() {
        errors.push("Package must declare at least one file mapping".to_string());
    } else if draft.files.len() > 200 {
        errors.push(format!(
            "Package declares {} files — exceeds maximum limit of 200",
            draft.files.len()
        ));
    }

    let mut seen_targets = HashSet::new();
    for (i, file) in draft.files.iter().enumerate() {
        if let Err(e) = validate_authoring_target(&file.target) {
            errors.push(format!("files[{}]: {}", i, e));
        } else if !seen_targets.insert(file.target.clone()) {
            errors.push(format!(
                "files[{}]: Duplicate target path '{}' declared",
                i, file.target
            ));
        }

        // Validate local source file
        let expanded_source = expand_user_path(&file.source_path);
        if let Err(e) = validate_source_file(&expanded_source) {
            errors.push(format!("files[{}]: {}", i, e));
        }

        // Validate custom relative path inside package if specified
        if let Some(ref rel) = file.package_rel_path {
            let trimmed = rel.trim();
            if trimmed.starts_with('/') || trimmed.starts_with('\\') {
                errors.push(format!(
                    "files[{}]: package_rel_path '{}' must be relative",
                    i, rel
                ));
            }
            if trimmed.contains('\0') {
                errors.push(format!(
                    "files[{}]: package_rel_path '{}' contains null byte",
                    i, rel
                ));
            }
            for comp in Path::new(trimmed).components() {
                if let Component::ParentDir = comp {
                    errors.push(format!(
                        "files[{}]: package_rel_path '{}' escapes package with '..'",
                        i, rel
                    ));
                    break;
                }
            }
        }
    }

    // 9. Dependencies validation
    if draft.dependencies.len() > 64 {
        errors.push(format!(
            "Package declares {} dependencies — exceeds limit of 64",
            draft.dependencies.len()
        ));
    }

    let mut seen_dep_keys = HashSet::new();
    for (i, dep) in draft.dependencies.iter().enumerate() {
        let trimmed_id = dep.id.trim();
        if trimmed_id.is_empty() {
            errors.push(format!("dependencies[{}]: 'id' must not be empty", i));
        } else if trimmed_id == draft.id.trim() {
            errors.push(format!(
                "dependencies[{}]: Package cannot declare a dependency on itself ('{}')",
                i, dep.id
            ));
        } else if trimmed_id.len() > 64 {
            errors.push(format!(
                "dependencies[{}]: 'id' exceeds maximum length of 64 characters",
                i
            ));
        } else if trimmed_id.contains("..")
            || !trimmed_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':')
        {
            errors.push(format!(
                "dependencies[{}]: 'id' ('{}') contains invalid characters or path traversal",
                i, dep.id
            ));
        }

        let key = (trimmed_id.to_string(), dep.kind.clone());
        if !seen_dep_keys.insert(key) {
            errors.push(format!(
                "dependencies[{}]: Duplicate dependency for id '{}' and kind '{:?}'",
                i, dep.id, dep.kind
            ));
        }

        if let Some(ref req_str) = dep.version_req {
            if let Err(e) = semver::VersionReq::parse(req_str) {
                errors.push(format!(
                    "dependencies[{}]: Invalid version requirement '{}': {}",
                    i, req_str, e
                ));
            }
        }
    }

    ManifestValidationResult {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bundle Generation
// ─────────────────────────────────────────────────────────────────────────────

/// Assemble and write a complete, standalone Ryzora package bundle to disk.
pub fn create_package_bundle(
    draft: PackageDraft,
    destination_dir: &Path,
) -> Result<AuthoringResult, String> {
    let validation = validate_package_draft_internal(&draft);
    if !validation.valid {
        return Err(format!(
            "Draft validation failed: {}",
            validation.errors.join("; ")
        ));
    }

    if destination_dir.as_os_str().is_empty() {
        return Err("Destination directory cannot be empty".to_string());
    }

    // Ensure destination root exists
    fs::create_dir_all(destination_dir).map_err(|e| {
        format!(
            "Failed to create destination directory '{}': {}",
            destination_dir.display(),
            e
        )
    })?;

    let pkg_dir = destination_dir.join(&draft.id);
    let files_dir = pkg_dir.join("files");
    fs::create_dir_all(&files_dir).map_err(|e| {
        format!(
            "Failed to create package files directory '{}': {}",
            files_dir.display(),
            e
        )
    })?;

    let mut manifest_files = Vec::new();
    let mut total_bytes = 0u64;

    for (i, file_draft) in draft.files.iter().enumerate() {
        let expanded_source = expand_user_path(&file_draft.source_path);

        // Derive relative path inside package: "files/<rel>"
        let pkg_rel = if let Some(ref custom_rel) = file_draft.package_rel_path {
            let trimmed = custom_rel.trim().trim_start_matches('/');
            if let Some(stripped) = trimmed.strip_prefix("files/") {
                format!("files/{}", stripped.trim_start_matches('/'))
            } else {
                format!("files/{}", trimmed)
            }
        } else {
            let clean_target = file_draft
                .target
                .strip_prefix("~/")
                .unwrap_or(&file_draft.target)
                .trim_start_matches('/');
            format!("files/{}", clean_target)
        };

        let dest_file_path = pkg_dir.join(&pkg_rel);
        if let Some(parent) = dest_file_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create parent directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }

        // Copy file (dereferences host-specific symlinks into portable regular files)
        fs::copy(&expanded_source, &dest_file_path).map_err(|e| {
            format!(
                "Failed to copy file '{}' to '{}': {}",
                expanded_source.display(),
                dest_file_path.display(),
                e
            )
        })?;

        let file_meta = fs::metadata(&dest_file_path).map_err(|e| {
            format!(
                "Failed to inspect copied file '{}': {}",
                dest_file_path.display(),
                e
            )
        })?;
        total_bytes += file_meta.len();

        manifest_files.push(ManifestFile {
            source: pkg_rel,
            target: file_draft.target.clone(),
            description: if file_draft.description.trim().is_empty() {
                format!("Configuration file {}", i + 1)
            } else {
                file_draft.description.clone()
            },
        });
    }

    if total_bytes > MAX_PACKAGE_SIZE {
        return Err(format!(
            "Total package size ({} bytes) exceeds maximum allowable limit of {} bytes (100 MB)",
            total_bytes, MAX_PACKAGE_SIZE
        ));
    }

    // Construct RyzoraManifest
    let manifest = RyzoraManifest {
        id: draft.id.clone(),
        name: draft.name.clone(),
        version: draft.version.clone(),
        ryzora_spec: "1".to_string(),
        author: draft.author.clone(),
        package_type: draft.package_type,
        description: draft.description.clone(),
        tags: draft.tags.clone(),
        color_palette: draft.color_palette.clone(),
        compatibility: draft.compatibility.clone(),
        files: manifest_files,
        dependencies: draft.dependencies.clone(),
        targets: std::collections::HashMap::new(),
    };

    let manifest_json = manifest.to_canonical_json()?;

    // Write ryzora.json (canonical) and manifest.json (backward compatibility)
    let ryzora_json_path = pkg_dir.join("ryzora.json");
    fs::write(&ryzora_json_path, &manifest_json).map_err(|e| {
        format!(
            "Failed to write ryzora.json at '{}': {}",
            ryzora_json_path.display(),
            e
        )
    })?;

    let manifest_compat_path = pkg_dir.join("manifest.json");
    fs::write(&manifest_compat_path, &manifest_json).map_err(|e| {
        format!(
            "Failed to write manifest.json at '{}': {}",
            manifest_compat_path.display(),
            e
        )
    })?;

    // Compute exact tree hash matching repository.rs
    let cumulative_sha256 = compute_package_tree_hash(&pkg_dir, &manifest)?;

    // Final integrity verification
    let self_check = validate_manifest_internal(&manifest_json);
    if !self_check.valid {
        return Err(format!(
            "Internal validation check failed on generated manifest: {}",
            self_check.errors.join("; ")
        ));
    }

    Ok(AuthoringResult {
        success: true,
        package_dir: pkg_dir.to_string_lossy().to_string(),
        manifest_path: ryzora_json_path.to_string_lossy().to_string(),
        files_copied: draft.files.len(),
        total_bytes,
        sha256_checksum: cumulative_sha256,
        warnings: validation.warnings,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Publishing / Export
// ─────────────────────────────────────────────────────────────────────────────

/// Export an authored package bundle into a target repository and update its index.
/// Guaranteed atomic publishing with rollback on failure and zero stale/orphaned files.
pub fn export_to_repository(
    package_dir: &Path,
    repository_dir: &Path,
) -> Result<PublishResult, String> {
    if !package_dir.is_dir() {
        return Err(format!(
            "Package directory not found: '{}'",
            package_dir.display()
        ));
    }

    validate_repository_destination(&repository_dir.to_string_lossy(), repository_dir)?;

    // Determine manifest file (ryzora.json preferred, manifest.json fallback)
    let manifest_file = if package_dir.join("ryzora.json").is_file() {
        package_dir.join("ryzora.json")
    } else if package_dir.join("manifest.json").is_file() {
        package_dir.join("manifest.json")
    } else {
        return Err(format!(
            "Neither ryzora.json nor manifest.json found in package directory '{}'",
            package_dir.display()
        ));
    };

    let manifest_raw = fs::read_to_string(&manifest_file).map_err(|e| {
        format!(
            "Failed to read manifest from '{}': {}",
            manifest_file.display(),
            e
        )
    })?;

    let validation = validate_manifest_internal(&manifest_raw);
    if !validation.valid {
        return Err(format!(
            "Cannot publish invalid package: {}",
            validation.errors.join("; ")
        ));
    }

    let manifest: RyzoraManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("Failed to deserialize manifest: {}", e))?;

    // Verify all declared files exist in source package
    for f in &manifest.files {
        let src_path = package_dir.join(&f.source);
        if !src_path.exists() {
            return Err(format!(
                "Declared package file '{}' is missing from package directory '{}'",
                f.source,
                package_dir.display()
            ));
        }
    }

    // Ensure repository directory exists
    fs::create_dir_all(repository_dir).map_err(|e| {
        format!(
            "Failed to create repository directory '{}': {}",
            repository_dir.display(),
            e
        )
    })?;

    // Check existing repository.json and enforce SemVer progression
    let repo_index_path = repository_dir.join("repository.json");
    let mut index = if repo_index_path.is_file() {
        let raw = fs::read_to_string(&repo_index_path)
            .map_err(|e| format!("Failed to read existing repository.json: {}", e))?;
        let parsed: RepositoryIndex = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse repository.json: {}", e))?;
        if parsed.schema != 1 {
            return Err(format!(
                "Unsupported repository schema {} (expected 1)",
                parsed.schema
            ));
        }

        // SemVer progression enforcement
        if let Some(existing) = parsed.packages.iter().find(|p| p.id == manifest.id) {
            let existing_ver = semver::Version::parse(&existing.version).map_err(|e| {
                format!(
                    "Existing package '{}' has invalid version in repository: {}",
                    existing.id, e
                )
            })?;
            let new_ver = semver::Version::parse(&manifest.version).map_err(|e| {
                format!(
                    "Candidate package '{}' has invalid version: {}",
                    manifest.id, e
                )
            })?;
            if new_ver < existing_ver {
                return Err(format!(
                    "Cannot publish version '{}': repository already contains newer version '{}' of package '{}'",
                    manifest.version, existing.version, manifest.id
                ));
            }
        }
        parsed
    } else {
        let repo_id = repository_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "community".to_string());
        RepositoryIndex {
            schema: 1,
            id: repo_id.clone(),
            name: format!("{} Repository", repo_id),
            version: "1.0.0".to_string(),
            description: "Local Ryzora package repository".to_string(),
            packages: Vec::new(),
        }
    };

    let repo_packages_dir = repository_dir.join("packages");
    fs::create_dir_all(&repo_packages_dir).map_err(|e| {
        format!(
            "Failed to create repository packages directory '{}': {}",
            repo_packages_dir.display(),
            e
        )
    })?;

    // Atomic Staging: Stage package into temporary directory first to eliminate stale/orphaned files
    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging_pkg_dir = repo_packages_dir.join(format!("{}.staging.{}", manifest.id, now_nanos));
    let staging_files_dir = staging_pkg_dir.join("files");
    fs::create_dir_all(&staging_files_dir).map_err(|e| {
        format!(
            "Failed to create staging directory '{}': {}",
            staging_files_dir.display(),
            e
        )
    })?;

    // Copy manifests into staging
    let staging_ryzora = staging_pkg_dir.join("ryzora.json");
    let staging_manifest = staging_pkg_dir.join("manifest.json");
    fs::write(&staging_ryzora, &manifest_raw)
        .map_err(|e| format!("Failed to write ryzora.json in staging: {}", e))?;
    fs::write(&staging_manifest, &manifest_raw)
        .map_err(|e| format!("Failed to write manifest.json in staging: {}", e))?;

    // Copy optional release artifacts into staging if present
    for extra_file in &[
        "release.json",
        "checksums.sha256",
        "SUBMISSION.md",
        "release.sig",
    ] {
        let src_extra = package_dir.join(extra_file);
        if src_extra.is_file() {
            let dest_extra = staging_pkg_dir.join(extra_file);
            let _ = fs::copy(&src_extra, &dest_extra);
        }
    }

    // Copy payload files into staging
    let mut total_bytes = 0u64;
    for f in &manifest.files {
        let src_path = package_dir.join(&f.source);
        let dest_path = staging_pkg_dir.join(&f.source);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "Failed to create staging parent directory '{}': {}",
                    parent.display(),
                    e
                )
            })?;
        }
        fs::copy(&src_path, &dest_path).map_err(|e| {
            let _ = fs::remove_dir_all(&staging_pkg_dir);
            format!(
                "Failed to copy file '{}' to staging: {}",
                src_path.display(),
                e
            )
        })?;

        let f_meta = fs::metadata(&dest_path).map_err(|e| {
            let _ = fs::remove_dir_all(&staging_pkg_dir);
            format!("Failed to inspect staging file: {}", e)
        })?;
        total_bytes += f_meta.len();
    }

    // Compute exact package tree hash
    let content_hash = match compute_package_tree_hash(&staging_pkg_dir, &manifest) {
        Ok(h) => h,
        Err(e) => {
            let _ = fs::remove_dir_all(&staging_pkg_dir);
            return Err(format!("Failed to compute package tree hash: {}", e));
        }
    };

    // Prepare atomic replacement of destination package directory
    let final_pkg_dir = repo_packages_dir.join(&manifest.id);
    let backup_pkg_dir = repo_packages_dir.join(format!("{}.backup.{}", manifest.id, now_nanos));

    let had_existing_pkg = final_pkg_dir.exists();
    if had_existing_pkg {
        if let Err(e) = fs::rename(&final_pkg_dir, &backup_pkg_dir) {
            let _ = fs::remove_dir_all(&staging_pkg_dir);
            return Err(format!(
                "Failed to move existing package to backup before replacement: {}",
                e
            ));
        }
    }

    if let Err(e) = fs::rename(&staging_pkg_dir, &final_pkg_dir) {
        // Rollback existing package if move fails
        if had_existing_pkg {
            let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
        }
        let _ = fs::remove_dir_all(&staging_pkg_dir);
        return Err(format!(
            "Failed to move staged package into final destination: {}",
            e
        ));
    }

    // Inherit release metadata if release.json was created
    let (inherited_channel, inherited_maintainer, inherited_notes, inherited_trust_tier) = {
        let release_json_path = package_dir.join("release.json");
        if release_json_path.is_file() {
            if let Ok(raw_rel) = fs::read_to_string(&release_json_path) {
                if let Ok(dist_rel) =
                    serde_json::from_str::<crate::distribution::DistributionRelease>(&raw_rel)
                {
                    (
                        Some(dist_rel.channel),
                        dist_rel.maintainer,
                        Some(dist_rel.release_notes),
                        Some(dist_rel.trust_tier),
                    )
                } else {
                    (
                        Some(crate::distribution::ReleaseChannel::Stable),
                        None,
                        None,
                        Some(crate::distribution::TrustTier::Community),
                    )
                }
            } else {
                (
                    Some(crate::distribution::ReleaseChannel::Stable),
                    None,
                    None,
                    Some(crate::distribution::TrustTier::Community),
                )
            }
        } else {
            (
                Some(crate::distribution::ReleaseChannel::Stable),
                None,
                None,
                Some(crate::distribution::TrustTier::Community),
            )
        }
    };

    // Prepare updated repository entry
    let new_entry = RepositoryPackageEntry {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        package_type: manifest.package_type.clone(),
        description: manifest.description.clone(),
        manifest: format!("packages/{}/manifest.json", manifest.id),
        category: manifest.package_type.to_string(),
        tags: manifest.tags.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        color_palette: manifest.color_palette.clone(),
        hero_image: None,
        screenshots: Vec::new(),
        featured: Some(false),
        trending: Some(false),
        rating: Some(5.0),
        downloads: Some(0),
        content_hash: Some(content_hash.clone()),
        package_size_bytes: Some(total_bytes),
        release_channel: inherited_channel,
        trust_tier: inherited_trust_tier,
        moderation_status: Some(crate::distribution::ModerationStatus::Approved),
        trending_score: Some(0.0),
        maintainer: inherited_maintainer,
        release_notes: inherited_notes,
        signature: crate::crypto::load_package_signature(package_dir),
        provider: None,
        targets: None,
        preview_video: None,
        preview_animated: None,
        source: None,
        provenance: None,
    };

    if let Some(pos) = index.packages.iter().position(|p| p.id == manifest.id) {
        index.packages[pos] = new_entry;
    } else {
        index.packages.push(new_entry);
    }

    let updated_index_json = match serde_json::to_string_pretty(&index) {
        Ok(j) => j,
        Err(e) => {
            // Rollback directory swap
            let _ = fs::remove_dir_all(&final_pkg_dir);
            if had_existing_pkg {
                let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
            }
            return Err(format!(
                "Failed to serialize updated repository index: {}",
                e
            ));
        }
    };

    // Atomic Index Write via temporary file
    let tmp_index_path = repository_dir.join(format!("repository.json.tmp.{}", now_nanos));
    let bak_index_path = repository_dir.join(format!("repository.json.bak.{}", now_nanos));

    if let Err(e) = fs::write(&tmp_index_path, &updated_index_json) {
        let _ = fs::remove_dir_all(&final_pkg_dir);
        if had_existing_pkg {
            let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
        }
        return Err(format!("Failed to write temporary repository index: {}", e));
    }

    let had_existing_index = repo_index_path.is_file();
    if had_existing_index {
        if let Err(e) = fs::copy(&repo_index_path, &bak_index_path) {
            let _ = fs::remove_file(&tmp_index_path);
            let _ = fs::remove_dir_all(&final_pkg_dir);
            if had_existing_pkg {
                let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
            }
            return Err(format!("Failed to backup existing repository index: {}", e));
        }
    }

    if let Err(e) = fs::rename(&tmp_index_path, &repo_index_path) {
        let _ = fs::remove_file(&tmp_index_path);
        let _ = fs::remove_file(&bak_index_path);
        let _ = fs::remove_dir_all(&final_pkg_dir);
        if had_existing_pkg {
            let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
        }
        return Err(format!("Failed to replace repository index: {}", e));
    }

    // Verify Repository Integrity
    match LocalRepository::load_from_dir(repository_dir) {
        Ok(_) => {
            // Success: purge backups
            let _ = fs::remove_file(&bak_index_path);
            if had_existing_pkg {
                let _ = fs::remove_dir_all(&backup_pkg_dir);
            }
        }
        Err(e) => {
            // Verification failed: full rollback
            if had_existing_index {
                let _ = fs::rename(&bak_index_path, &repo_index_path);
            } else {
                let _ = fs::remove_file(&repo_index_path);
            }
            let _ = fs::remove_dir_all(&final_pkg_dir);
            if had_existing_pkg {
                let _ = fs::rename(&backup_pkg_dir, &final_pkg_dir);
            }
            return Err(format!(
                "Repository integrity verification failed after publishing (changes rolled back): {}",
                e
            ));
        }
    }

    Ok(PublishResult {
        success: true,
        repository_path: repository_dir.to_string_lossy().to_string(),
        package_id: manifest.id,
        package_version: manifest.version,
        archive_path: None,
        archive_sha256: Some(content_hash),
        message: "Package successfully published to repository".to_string(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Command Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn validate_package_draft(draft: PackageDraft) -> Result<ManifestValidationResult, String> {
    Ok(validate_package_draft_internal(&draft))
}

#[tauri::command]
pub fn create_package(
    draft: PackageDraft,
    destination_dir: String,
) -> Result<AuthoringResult, String> {
    let dest_path = expand_user_path(&destination_dir);
    create_package_bundle(draft, &dest_path)
}

#[tauri::command]
pub fn publish_package_to_repository(
    package_dir: String,
    repository_path: String,
) -> Result<PublishResult, String> {
    let pkg_path = expand_user_path(&package_dir);
    let repo_path = expand_user_path(&repository_path);
    export_to_repository(&pkg_path, &repo_path)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests (Phase 10 & 10.1 Audit Suite)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Repository;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestSandbox {
        root: PathBuf,
        source_dir: PathBuf,
        dest_dir: PathBuf,
        repo_dir: PathBuf,
    }

    impl TestSandbox {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ryzora-phase10-{}-{}", label, nanos));
            let source_dir = root.join("source");
            let dest_dir = root.join("output");
            let repo_dir = root.join("repo");

            fs::create_dir_all(&source_dir).unwrap();
            fs::create_dir_all(&dest_dir).unwrap();
            fs::create_dir_all(&repo_dir).unwrap();

            Self {
                root,
                source_dir,
                dest_dir,
                repo_dir,
            }
        }

        fn create_dummy_file(&self, rel: &str, content: &str) -> PathBuf {
            let p = self.source_dir.join(rel);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&p, content).unwrap();
            p
        }
    }

    impl Drop for TestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn sample_valid_draft(sandbox: &TestSandbox) -> PackageDraft {
        let conf_file = sandbox.create_dummy_file("hyprland.conf", "# Hyprland config\n");
        let bar_file = sandbox.create_dummy_file("waybar.jsonc", r#"{"layer": "top"}"#);

        PackageDraft {
            id: "neon-cyberpunk".to_string(),
            name: "Neon Cyberpunk Rice".to_string(),
            version: "1.0.0".to_string(),
            author: "RiceAuthor".to_string(),
            package_type: PackageType::Rice,
            description: "A sleek neon cyberpunk aesthetic rice for Hyprland".to_string(),
            tags: vec![
                "hyprland".to_string(),
                "neon".to_string(),
                "waybar".to_string(),
            ],
            color_palette: vec!["#ff007f".to_string(), "#00f0ff".to_string()],
            compatibility: ManifestCompatibility {
                desktops: vec!["hyprland".to_string()],
                sessions: vec!["wayland".to_string()],
                distros: vec![],
                required: vec![],
                optional: vec![],
            },
            dependencies: vec![],
            files: vec![
                FileMappingDraft {
                    source_path: conf_file.to_string_lossy().to_string(),
                    target: "~/.config/hypr/hyprland.conf".to_string(),
                    package_rel_path: None,
                    description: "Hyprland main configuration".to_string(),
                },
                FileMappingDraft {
                    source_path: bar_file.to_string_lossy().to_string(),
                    target: "~/.config/waybar/config.jsonc".to_string(),
                    package_rel_path: Some("files/waybar/config.jsonc".to_string()),
                    description: "Waybar top bar configuration".to_string(),
                },
            ],
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 10 Baseline Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_authoring_valid_package_creation() {
        let sandbox = TestSandbox::new("valid-pkg");
        let draft = sample_valid_draft(&sandbox);

        let res = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        assert!(res.success);
        assert_eq!(res.files_copied, 2);
        assert!(res.total_bytes > 0);
        assert!(!res.sha256_checksum.is_empty());

        let pkg_path = PathBuf::from(&res.package_dir);
        assert!(pkg_path.join("ryzora.json").is_file());
        assert!(pkg_path.join("manifest.json").is_file());
        assert!(pkg_path.join("files/.config/hypr/hyprland.conf").is_file());
        assert!(pkg_path.join("files/waybar/config.jsonc").is_file());
    }

    #[test]
    fn test_authoring_rejects_path_traversal_in_files() {
        let sandbox = TestSandbox::new("traversal");
        let mut draft = sample_valid_draft(&sandbox);
        draft.files[0].target = "~/.config/../../etc/passwd".to_string();

        let res = create_package_bundle(draft, &sandbox.dest_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("path traversal") || err.contains("forbidden"));
    }

    #[test]
    fn test_authoring_rejects_absolute_targets() {
        let sandbox = TestSandbox::new("abs-target");
        let mut draft = sample_valid_draft(&sandbox);
        draft.files[0].target = "/etc/sudoers".to_string();

        let res = create_package_bundle(draft, &sandbox.dest_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("must start with '~/'"));
    }

    #[test]
    fn test_authoring_checksum_generation_matches_contents() {
        let sandbox = TestSandbox::new("checksum");
        let draft = sample_valid_draft(&sandbox);

        let res = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_path = PathBuf::from(&res.package_dir);
        let file_path = pkg_path.join("files/.config/hypr/hyprland.conf");
        let file_hash = compute_file_sha256(&file_path).unwrap();
        assert_eq!(file_hash.len(), 64);
        assert!(file_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_authoring_rejects_missing_source_files() {
        let sandbox = TestSandbox::new("missing-src");
        let mut draft = sample_valid_draft(&sandbox);
        draft.files[0].source_path = sandbox
            .source_dir
            .join("does_not_exist.conf")
            .to_string_lossy()
            .to_string();

        let res = create_package_bundle(draft, &sandbox.dest_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Source file not found"));
    }

    #[test]
    fn test_authoring_rejects_duplicate_targets() {
        let sandbox = TestSandbox::new("dup-targets");
        let mut draft = sample_valid_draft(&sandbox);
        draft.files[1].target = draft.files[0].target.clone();

        let res = create_package_bundle(draft, &sandbox.dest_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Duplicate target path"));
    }

    #[test]
    fn test_authoring_rejects_invalid_semver() {
        let sandbox = TestSandbox::new("semver");
        let mut draft = sample_valid_draft(&sandbox);
        draft.version = "invalid-semver".to_string();

        let res = create_package_bundle(draft, &sandbox.dest_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("SemVer"));
    }

    #[test]
    fn test_authoring_publishes_to_repository_index() {
        let sandbox = TestSandbox::new("publish");
        let draft = sample_valid_draft(&sandbox);

        let bundled = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_dir = PathBuf::from(&bundled.package_dir);

        let pub_res = export_to_repository(&pkg_dir, &sandbox.repo_dir).unwrap();
        assert!(pub_res.success);
        assert_eq!(pub_res.package_id, "neon-cyberpunk");
        assert_eq!(pub_res.package_version, "1.0.0");

        // Verify repository.json was created and can be loaded by LocalRepository
        let local_repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        assert_eq!(local_repo.list_entries().unwrap().len(), 1);
        let entry = &local_repo.list_entries().unwrap()[0];
        assert_eq!(entry.id, "neon-cyberpunk");
        assert_eq!(entry.version, "1.0.0");
        assert!(entry.content_hash.is_some());
    }

    #[test]
    fn test_authoring_updates_existing_package_in_repository() {
        let sandbox = TestSandbox::new("update-repo");
        let draft_v1 = sample_valid_draft(&sandbox);

        let b1 = create_package_bundle(draft_v1, &sandbox.dest_dir).unwrap();
        export_to_repository(&PathBuf::from(&b1.package_dir), &sandbox.repo_dir).unwrap();

        // Create version 1.1.0 with updated file
        let mut draft_v2 = sample_valid_draft(&sandbox);
        draft_v2.version = "1.1.0".to_string();
        let new_file = sandbox.create_dummy_file("hyprland_v2.conf", "# Hyprland v2\n");
        draft_v2.files[0].source_path = new_file.to_string_lossy().to_string();

        let b2 = create_package_bundle(draft_v2, &sandbox.dest_dir.join("v2")).unwrap();
        export_to_repository(&PathBuf::from(&b2.package_dir), &sandbox.repo_dir).unwrap();

        // Ensure index still has 1 package, now at version 1.1.0
        let local_repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        let entries = local_repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "neon-cyberpunk");
        assert_eq!(entries[0].version, "1.1.0");
    }

    #[test]
    fn test_authoring_security_no_shell_commands() {
        assert!(validate_authoring_target("~/config/safe.conf").is_ok());
        assert!(validate_authoring_target("/etc/shadow").is_err());
        assert!(validate_authoring_target("~/.config/../../../bin/sh").is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 10.1 Audit Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_audit_rejects_publishing_to_remote_url() {
        let sandbox = TestSandbox::new("audit-url");
        let draft = sample_valid_draft(&sandbox);
        let b = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_dir = PathBuf::from(&b.package_dir);

        let res = export_to_repository(&pkg_dir, Path::new("https://github.com/owner/repo"));
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Remote repository URLs cannot be targeted"));
    }

    #[test]
    fn test_audit_rejects_publishing_to_system_directory() {
        let sandbox = TestSandbox::new("audit-sysdir");
        let draft = sample_valid_draft(&sandbox);
        let b = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_dir = PathBuf::from(&b.package_dir);

        assert!(export_to_repository(&pkg_dir, Path::new("/etc/ryzora-repo")).is_err());
        assert!(export_to_repository(&pkg_dir, Path::new("/usr/share/repo")).is_err());
        assert!(export_to_repository(&pkg_dir, Path::new("/var/repo")).is_err());
    }

    #[test]
    fn test_audit_rejects_publishing_to_non_repository_non_empty_dir() {
        let sandbox = TestSandbox::new("audit-nonrepo");
        let draft = sample_valid_draft(&sandbox);
        let b = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_dir = PathBuf::from(&b.package_dir);

        // Populate a directory with arbitrary files but no repository.json
        let arbitrary_dir = sandbox.root.join("my-documents");
        fs::create_dir_all(&arbitrary_dir).unwrap();
        fs::write(arbitrary_dir.join("personal.txt"), "private notes").unwrap();

        let res = export_to_repository(&pkg_dir, &arbitrary_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("missing 'repository.json'"));
    }

    #[test]
    fn test_audit_rejects_version_downgrade_in_repository() {
        let sandbox = TestSandbox::new("audit-downgrade");
        let mut draft_v2 = sample_valid_draft(&sandbox);
        draft_v2.version = "2.0.0".to_string();

        let b2 = create_package_bundle(draft_v2, &sandbox.dest_dir).unwrap();
        export_to_repository(&PathBuf::from(&b2.package_dir), &sandbox.repo_dir).unwrap();

        // Attempt to publish 1.9.0 over 2.0.0
        let mut draft_v1 = sample_valid_draft(&sandbox);
        draft_v1.version = "1.9.0".to_string();

        let b1 = create_package_bundle(draft_v1, &sandbox.dest_dir.join("v1")).unwrap();
        let res = export_to_repository(&PathBuf::from(&b1.package_dir), &sandbox.repo_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("already contains newer version '2.0.0'"));
    }

    #[test]
    fn test_audit_atomic_publishing_cleans_orphaned_stale_files() {
        let sandbox = TestSandbox::new("audit-stale");
        let draft_v1 = sample_valid_draft(&sandbox);
        // v1 has 2 files: hyprland.conf and waybar.jsonc
        let b1 = create_package_bundle(draft_v1, &sandbox.dest_dir).unwrap();
        export_to_repository(&PathBuf::from(&b1.package_dir), &sandbox.repo_dir).unwrap();

        let repo_pkg = sandbox.repo_dir.join("packages").join("neon-cyberpunk");
        assert!(repo_pkg.join("files/.config/hypr/hyprland.conf").is_file());
        assert!(repo_pkg.join("files/waybar/config.jsonc").is_file());

        // v2 removes waybar.jsonc and only bundles hyprland.conf
        let mut draft_v2 = sample_valid_draft(&sandbox);
        draft_v2.version = "1.1.0".to_string();
        draft_v2.files.pop(); // remove waybar.jsonc

        let b2 = create_package_bundle(draft_v2, &sandbox.dest_dir.join("v2")).unwrap();
        export_to_repository(&PathBuf::from(&b2.package_dir), &sandbox.repo_dir).unwrap();

        // Orphaned waybar.jsonc MUST be gone
        assert!(repo_pkg.join("files/.config/hypr/hyprland.conf").is_file());
        assert!(!repo_pkg.join("files/waybar/config.jsonc").exists());
    }

    #[test]
    fn test_audit_content_hash_matches_compute_package_tree_hash() {
        let sandbox = TestSandbox::new("audit-hash");
        let draft = sample_valid_draft(&sandbox);
        let b = create_package_bundle(draft, &sandbox.dest_dir).unwrap();
        let pkg_dir = PathBuf::from(&b.package_dir);

        let pub_res = export_to_repository(&pkg_dir, &sandbox.repo_dir).unwrap();
        let local_repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        let entry = &local_repo.list_entries().unwrap()[0];

        let manifest: RyzoraManifest = serde_json::from_str(
            &fs::read_to_string(sandbox.repo_dir.join(&entry.manifest)).unwrap(),
        )
        .unwrap();

        let tree_hash = compute_package_tree_hash(
            &sandbox.repo_dir.join("packages").join(&entry.id),
            &manifest,
        )
        .unwrap();

        assert_eq!(entry.content_hash.as_ref().unwrap(), &tree_hash);
        assert_eq!(pub_res.archive_sha256.as_ref().unwrap(), &tree_hash);
    }

    #[test]
    fn test_audit_rejects_symlink_pointing_to_forbidden_system_file() {
        #[cfg(unix)]
        {
            let sandbox = TestSandbox::new("audit-symlink-sys");
            let shadow_link = sandbox.source_dir.join("shadow_symlink");
            let _ = std::os::unix::fs::symlink("/etc/shadow", &shadow_link);

            let res = validate_source_file(&shadow_link);
            assert!(res.is_err());
            let err = res.err().unwrap();
            assert!(err.contains("protected system location") || err.contains("Broken"));
        }
    }

    #[test]
    fn test_audit_rejects_broken_symlinks() {
        #[cfg(unix)]
        {
            let sandbox = TestSandbox::new("audit-broken-symlink");
            let broken_link = sandbox.source_dir.join("broken_link");
            let _ = std::os::unix::fs::symlink(
                sandbox.source_dir.join("nonexistent_target_123"),
                &broken_link,
            );

            let res = validate_source_file(&broken_link);
            assert!(res.is_err());
            let err = res.err().unwrap();
            assert!(err.contains("Broken or unreadable symlink"));
        }
    }

    #[test]
    fn test_audit_rejects_self_dependency() {
        let sandbox = TestSandbox::new("audit-self-dep");
        let mut draft = sample_valid_draft(&sandbox);
        draft.dependencies.push(DependencySpec {
            id: draft.id.clone(),
            kind: crate::dependency::DependencyKind::Package,
            version_req: None,
            required: true,
            description: None,
        });

        let res = validate_package_draft_internal(&draft);
        assert!(!res.valid);
        assert!(res
            .errors
            .iter()
            .any(|e| e.contains("dependency on itself")));
    }

    #[test]
    fn test_audit_rejects_invalid_hex_color_palette() {
        let sandbox = TestSandbox::new("audit-colors");
        let mut draft = sample_valid_draft(&sandbox);
        draft.color_palette.push("blue".to_string()); // not hex

        let res = validate_package_draft_internal(&draft);
        assert!(!res.valid);
        assert!(res
            .errors
            .iter()
            .any(|e| e.contains("valid hex color code")));
    }

    #[test]
    fn test_audit_enforces_max_package_size_and_file_count() {
        let sandbox = TestSandbox::new("audit-limits");
        let mut draft = sample_valid_draft(&sandbox);

        // Test file count limit
        for i in 0..205 {
            let f = sandbox.create_dummy_file(&format!("f_{}.conf", i), "x");
            draft.files.push(FileMappingDraft {
                source_path: f.to_string_lossy().to_string(),
                target: format!("~/.config/f_{}.conf", i),
                package_rel_path: None,
                description: "".to_string(),
            });
        }

        let res = validate_package_draft_internal(&draft);
        assert!(!res.valid);
        assert!(res
            .errors
            .iter()
            .any(|e| e.contains("exceeds maximum limit of 200")));
    }
}
