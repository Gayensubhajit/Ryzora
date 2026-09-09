use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use url::Url;

use crate::compatibility::CompatibilityRequirements;
use crate::distribution::{ModerationStatus, ReleaseChannel, TrustTier};
use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
use crate::snapshot::get_home_dir;

// ─────────────────────────────────────────────────────────────────────────────
// Size Limits (Enforced on remote/cached content)
// ─────────────────────────────────────────────────────────────────────────────

pub const MAX_INDEX_SIZE: usize = 5 * 1024 * 1024; // 5 MB
pub const MAX_MANIFEST_SIZE: usize = 2 * 1024 * 1024; // 2 MB
pub const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024; // 50 MB
pub const MAX_PACKAGE_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

// ─────────────────────────────────────────────────────────────────────────────
// Repository Data Types (Schema v1)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorInfo {
    pub name: String,
    pub avatar: String,
    #[serde(default)]
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryPackageEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub package_type: PackageType,
    #[serde(default)]
    pub description: String,
    pub manifest: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub author: AuthorInfo,
    #[serde(default)]
    pub color_palette: Vec<String>,
    #[serde(default)]
    pub hero_image: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub featured: Option<bool>,
    #[serde(default)]
    pub trending: Option<bool>,
    #[serde(default)]
    pub rating: Option<f64>,
    #[serde(default)]
    pub downloads: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub package_size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_tier: Option<TrustTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation_status: Option<ModerationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintainer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<crate::crypto::PackageSignatureMetadata>,
}

impl Default for RepositoryPackageEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: String::new(),
            manifest: String::new(),
            category: "rice".to_string(),
            tags: Vec::new(),
            author: AuthorInfo {
                name: String::new(),
                avatar: String::new(),
                verified: false,
            },
            color_palette: Vec::new(),
            hero_image: None,
            screenshots: Vec::new(),
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub schema: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    pub packages: Vec<RepositoryPackageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySummary {
    pub id: String,
    pub name: String,
    pub package_count: usize,
    pub path: String,
    pub repo_type: String, // "local" | "remote"
    pub enabled: bool,
    #[serde(default = "default_repo_status")]
    pub status: String, // "online" | "offline" | "cached" | "refresh_failed" | "disabled"
    #[serde(default)]
    pub last_refreshed: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
}

fn default_repo_status() -> String {
    "online".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySourceConfig {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub repo_type: String, // "local" | "remote"
}

// ─────────────────────────────────────────────────────────────────────────────
// Frontend-Facing Package Item (Compatible with existing UI PackageItem)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendSafetyAudit {
    pub rating: String,
    pub changes_system_files: bool,
    pub requires_root: bool,
    pub sandbox_compatible: bool,
    pub files_modified_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendDependencies {
    pub packages: Vec<String>,
    pub optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendComponentSpec {
    pub name: String,
    pub component_type: String,
    pub target_path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendPackageItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub version: String,
    pub author: AuthorInfo,
    pub category: String,
    pub package_type: PackageType,
    pub tags: Vec<String>,
    pub supported_desktops: Vec<String>,
    pub supported_display: Vec<String>,
    pub rating: f64,
    pub rating_count: u32,
    pub downloads: u64,
    pub hero_image: String,
    pub screenshots: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub featured: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trending: Option<bool>,
    pub color_palette: Vec<String>,
    pub safety_audit: FrontendSafetyAudit,
    pub dependencies: FrontendDependencies,
    pub components: Vec<FrontendComponentSpec>,
    pub compatibility: CompatibilityRequirements,
    pub manifest: Option<RyzoraManifest>,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    #[serde(default)]
    pub package_size_bytes: Option<u64>,
    #[serde(default = "default_integrity_status")]
    pub integrity_status: String,
    #[serde(default)]
    pub is_cached: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_channel: Option<ReleaseChannel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_tier: Option<TrustTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moderation_status: Option<ModerationStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintainer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<crate::crypto::PackageSignatureMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cryptographic_status: Option<crate::crypto::CryptographicStatus>,
}

fn default_integrity_status() -> String {
    "unverified".to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Security Validation, Path Sanitization & Safe URL Resolution
// ─────────────────────────────────────────────────────────────────────────────

pub fn validate_sha256_hash(hash: &str) -> Result<String, String> {
    let trimmed = hash.trim();
    if trimmed.len() != 64 {
        return Err(format!(
            "Invalid SHA-256 hash length: expected 64 hex characters, got {}",
            trimmed.len()
        ));
    }
    if !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!(
            "Invalid SHA-256 hash '{}': contains non-hexadecimal characters",
            trimmed
        ));
    }
    Ok(trimmed.to_ascii_lowercase())
}

pub fn validate_path_identifier(id: &str, field_name: &str) -> Result<(), String> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }
    if id.contains('\0') {
        return Err(format!("{} contains forbidden null byte", field_name));
    }
    if id.contains('/') || id.contains('\\') {
        return Err(format!(
            "{} contains forbidden path separator: '{}'",
            field_name, id
        ));
    }
    if id.contains("..") {
        return Err(format!(
            "{} contains forbidden parent traversal ('..'): '{}'",
            field_name, id
        ));
    }
    if id == "." || id == ".." {
        return Err(format!("{} cannot be '.' or '..'", field_name));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+')
    {
        return Err(format!(
            "{} contains invalid characters: '{}'",
            field_name, id
        ));
    }
    if id.len() > 128 {
        return Err(format!(
            "{} exceeds maximum length of 128 characters",
            field_name
        ));
    }
    Ok(())
}

pub fn validate_image_path_or_url(
    path_or_url: &str,
    field_name: &str,
    pkg_id: &str,
) -> Result<(), String> {
    if path_or_url.contains('\0') {
        return Err(format!(
            "Package '{}' {} contains null byte",
            pkg_id, field_name
        ));
    }
    if path_or_url.starts_with("https://") {
        validate_remote_url(path_or_url)?;
    } else {
        if path_or_url.starts_with('/') || path_or_url.starts_with('\\') {
            return Err(format!(
                "Package '{}' {} relative path must not start with '/': '{}'",
                pkg_id, field_name, path_or_url
            ));
        }
        if path_or_url.contains("://") || path_or_url.starts_with("//") {
            return Err(format!(
                "Package '{}' {} contains invalid scheme or protocol-relative path: '{}'",
                pkg_id, field_name, path_or_url
            ));
        }
        for comp in Path::new(path_or_url).components() {
            if let Component::ParentDir = comp {
                return Err(format!(
                    "Package '{}' {} contains forbidden parent traversal ('..')",
                    pkg_id, field_name
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_repository_package_entry(pkg: &RepositoryPackageEntry) -> Result<(), String> {
    validate_path_identifier(&pkg.id, "package ID")
        .map_err(|e| format!("Invalid package ID '{}' in repository index: {}", pkg.id, e))?;
    validate_path_identifier(&pkg.version, "package version")?;
    semver::Version::parse(&pkg.version).map_err(|e| {
        format!(
            "Invalid semver version '{}' for package '{}': {}",
            pkg.version, pkg.id, e
        )
    })?;

    if pkg.manifest.starts_with('/')
        || pkg.manifest.starts_with('\\')
        || pkg.manifest.contains('\0')
    {
        return Err(format!("Manifest path '{}' must be relative", pkg.manifest));
    }
    for comp in Path::new(&pkg.manifest).components() {
        if let Component::ParentDir = comp {
            return Err(format!(
                "Manifest path '{}' contains forbidden parent traversal ('..')",
                pkg.manifest
            ));
        }
    }

    if let Some(ref hash) = pkg.content_hash {
        validate_sha256_hash(hash)?;
    }

    if let Some(size) = pkg.package_size_bytes {
        if size == 0 {
            return Err(format!(
                "Package '{}' has invalid package_size_bytes of 0",
                pkg.id
            ));
        }
        if size > MAX_PACKAGE_SIZE {
            return Err(format!(
                "Package '{}' declared size {} exceeds maximum allowed ({} bytes)",
                pkg.id, size, MAX_PACKAGE_SIZE
            ));
        }
    }

    let trimmed_name = pkg.name.trim();
    if trimmed_name.is_empty() {
        return Err(format!("Package '{}' name cannot be empty", pkg.id));
    }
    if pkg.name.contains('\0') {
        return Err(format!("Package '{}' name contains null byte", pkg.id));
    }
    if pkg.name.len() > 100 {
        return Err(format!("Package '{}' name exceeds 100 characters", pkg.id));
    }

    if pkg.description.contains('\0') {
        return Err(format!(
            "Package '{}' description contains null byte",
            pkg.id
        ));
    }
    if pkg.description.len() > 10000 {
        return Err(format!(
            "Package '{}' description exceeds 10000 characters",
            pkg.id
        ));
    }

    if pkg.category.contains('\0') {
        return Err(format!("Package '{}' category contains null byte", pkg.id));
    }
    if pkg.category.chars().any(|c| (c as u32) < 0x20 && c != '\t') {
        return Err(format!(
            "Package '{}' category contains control characters",
            pkg.id
        ));
    }
    if pkg.category.len() > 64 {
        return Err(format!(
            "Package '{}' category exceeds 64 characters",
            pkg.id
        ));
    }

    if pkg.tags.len() > 30 {
        return Err(format!("Package '{}' exceeds maximum 30 tags", pkg.id));
    }
    for tag in &pkg.tags {
        if tag.contains('\0') {
            return Err(format!("Package '{}' tag contains null byte", pkg.id));
        }
        if tag.len() > 50 {
            return Err(format!(
                "Package '{}' tag exceeds 50 characters: '{}'",
                pkg.id, tag
            ));
        }
    }

    let trimmed_author = pkg.author.name.trim();
    if trimmed_author.is_empty() {
        return Err(format!("Package '{}' author name cannot be empty", pkg.id));
    }
    if pkg.author.name.contains('\0') {
        return Err(format!(
            "Package '{}' author name contains null byte",
            pkg.id
        ));
    }
    if pkg.author.name.len() > 100 {
        return Err(format!(
            "Package '{}' author name exceeds 100 characters",
            pkg.id
        ));
    }
    if pkg.author.avatar.contains('\0') || pkg.author.avatar.contains("..") {
        return Err(format!(
            "Package '{}' author avatar path is invalid",
            pkg.id
        ));
    }

    if pkg.color_palette.len() > 20 {
        return Err(format!(
            "Package '{}' exceeds maximum 20 colors in palette",
            pkg.id
        ));
    }
    for color in &pkg.color_palette {
        let trimmed_color = color.trim();
        if trimmed_color.is_empty() || trimmed_color.len() > 16 || trimmed_color.contains('\0') {
            return Err(format!(
                "Package '{}' has invalid color '{}'",
                pkg.id, color
            ));
        }
        if !trimmed_color.starts_with('#')
            && !trimmed_color.chars().all(|c| c.is_ascii_alphanumeric())
        {
            return Err(format!(
                "Package '{}' has invalid color format: '{}'",
                pkg.id, color
            ));
        }
    }

    if let Some(ref hero) = pkg.hero_image {
        validate_image_path_or_url(hero, "hero_image", &pkg.id)?;
    }

    if pkg.screenshots.len() > 20 {
        return Err(format!(
            "Package '{}' exceeds maximum 20 screenshots",
            pkg.id
        ));
    }
    for ss in &pkg.screenshots {
        validate_image_path_or_url(ss, "screenshot", &pkg.id)?;
    }

    if let Some(r) = pkg.rating {
        if r.is_nan() || r.is_infinite() || !(0.0..=5.0).contains(&r) {
            return Err(format!(
                "Package '{}' rating must be between 0.0 and 5.0, got {}",
                pkg.id, r
            ));
        }
    }

    Ok(())
}

pub fn get_safe_cache_path(
    cache_base: &Path,
    repo_id: &str,
    package_id: &str,
    version: &str,
) -> Result<PathBuf, String> {
    validate_path_identifier(repo_id, "repository ID")?;
    validate_path_identifier(package_id, "package ID")?;
    validate_path_identifier(version, "package version")?;

    let path = cache_base.join(package_id).join(version);

    let relative = path
        .strip_prefix(cache_base)
        .map_err(|e| format!("Cache path prefix violation: {}", e))?;

    for comp in relative.components() {
        match comp {
            Component::Normal(_) => {}
            _ => {
                return Err(format!(
                    "Suspicious path component detected in cache path: {:?}",
                    comp
                ))
            }
        }
    }

    Ok(path)
}

pub fn validate_remote_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err(format!(
            "Forbidden URL scheme in '{}': remote repositories must use HTTPS only",
            url
        ));
    }
    if url.contains('\0') {
        return Err("URL contains null bytes".to_string());
    }
    if url.contains("..") {
        return Err("URL contains forbidden parent traversal ('..')".to_string());
    }
    let parsed = Url::parse(url).map_err(|e| format!("Malformed HTTPS URL '{}': {}", url, e))?;
    if parsed.scheme() != "https" {
        return Err(format!(
            "Forbidden URL scheme '{}': HTTPS required",
            parsed.scheme()
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(format!(
            "URL contains forbidden user credentials: '{}'",
            url
        ));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| format!("URL missing host: '{}'", url))?;
    if host.trim().is_empty() {
        return Err("URL host cannot be empty".to_string());
    }
    for comp in Path::new(parsed.path()).components() {
        if let Component::ParentDir = comp {
            return Err("URL contains forbidden parent traversal ('..')".to_string());
        }
    }
    Ok(())
}

pub fn resolve_repository_url(base_url: &str, relative_path: &str) -> Result<String, String> {
    validate_remote_url(base_url)?;

    let base = Url::parse(base_url)
        .map_err(|e| format!("Invalid base repository URL '{}': {}", base_url, e))?;

    let base_host = base
        .host_str()
        .ok_or_else(|| format!("Base URL missing host: '{}'", base_url))?;

    if relative_path.contains('\0') {
        return Err("Relative path contains null bytes".to_string());
    }
    if relative_path.chars().any(|c| (c as u32) < 0x20) {
        return Err("Relative path contains control characters".to_string());
    }
    if relative_path.contains('\\') {
        return Err(format!(
            "Relative path contains forbidden backslashes: '{}'",
            relative_path
        ));
    }
    if relative_path.starts_with('/') || relative_path.starts_with("//") {
        return Err(format!(
            "Relative path must not start with '/': '{}'",
            relative_path
        ));
    }
    if relative_path.contains("://") {
        return Err(format!(
            "Relative path must not contain scheme separator '://': '{}'",
            relative_path
        ));
    }
    if relative_path.contains('#') {
        return Err(format!(
            "Relative path must not contain fragment '#': '{}'",
            relative_path
        ));
    }
    let lower_rel = relative_path.to_ascii_lowercase();
    if lower_rel.contains("%2f") || lower_rel.contains("%5c") {
        return Err(format!(
            "Relative path contains encoded separator: '{}'",
            relative_path
        ));
    }

    if let Ok(parsed) = Url::parse(relative_path) {
        if parsed.has_host() || !parsed.scheme().is_empty() {
            return Err(format!(
                "Relative path parsed as absolute or scheme URL: '{}'",
                relative_path
            ));
        }
    }

    // Check raw traversal segments
    for seg in relative_path.split('/') {
        if seg == ".." {
            return Err(format!(
                "Relative path contains forbidden parent traversal ('..'): '{}'",
                relative_path
            ));
        }
    }

    // Multi-level percent-decode (up to 3 levels) to reject %2e%2e, %2f, %5c, double encoded
    let mut current_decoded = relative_path.to_string();
    for _ in 0..3 {
        let next_decoded = percent_encoding::percent_decode_str(&current_decoded)
            .decode_utf8_lossy()
            .to_string();
        if next_decoded == current_decoded {
            break;
        }
        current_decoded = next_decoded;
        if current_decoded.contains('\0') {
            return Err("Percent-encoded path contains null byte".to_string());
        }
        if current_decoded.contains('\\') {
            return Err("Percent-encoded path contains backslash".to_string());
        }
        if current_decoded.contains("://") {
            return Err("Percent-encoded path contains scheme separator".to_string());
        }
        for seg in current_decoded.split(['/', '\\']) {
            if seg == ".." || seg.eq_ignore_ascii_case("%2e%2e") {
                return Err(format!(
                    "Decoded path contains forbidden parent traversal ('..'): '{}'",
                    relative_path
                ));
            }
        }
    }

    // Ensure base ends with '/' so join treats base path as a directory
    let base_with_slash = if base.as_str().ends_with('/') {
        base.clone()
    } else {
        let mut s = base.as_str().to_string();
        s.push('/');
        Url::parse(&s).map_err(|e| format!("Failed to normalize base URL: {}", e))?
    };

    let resolved = base_with_slash.join(relative_path).map_err(|e| {
        format!(
            "Failed to resolve path '{}' against base '{}': {}",
            relative_path, base_url, e
        )
    })?;

    if resolved.scheme() != "https" {
        return Err(format!(
            "Resolved URL must use HTTPS, got '{}'",
            resolved.scheme()
        ));
    }
    if resolved.host_str() != Some(base_host) {
        return Err(format!(
            "Resolved URL host '{:?}' does not match repository base host '{}'",
            resolved.host_str(),
            base_host
        ));
    }
    if resolved.port() != base.port() {
        return Err("Resolved URL port does not match repository base port".to_string());
    }
    if !resolved.username().is_empty() || resolved.password().is_some() {
        return Err("Resolved URL contains forbidden credentials".to_string());
    }
    if resolved.fragment().is_some() {
        return Err("Resolved URL contains unexpected fragment".to_string());
    }

    let base_dir_path = base_with_slash.path();
    let resolved_path = resolved.path();
    if !resolved_path.starts_with(base_dir_path) {
        return Err(format!(
            "Resolved path '{}' escapes repository base hierarchy '{}'",
            resolved_path, base_dir_path
        ));
    }

    Ok(resolved.to_string())
}

pub fn validate_redirect(current_url: &str, target_location: &str) -> Result<String, String> {
    let current = Url::parse(current_url)
        .map_err(|e| format!("Invalid current URL '{}': {}", current_url, e))?;

    if current.scheme() != "https" {
        return Err("Current URL must be HTTPS".to_string());
    }

    let original_host = current
        .host_str()
        .ok_or_else(|| "Current URL missing host".to_string())?;

    if target_location.contains('\0') {
        return Err("Redirect Location contains null byte".to_string());
    }
    if target_location.contains('\\') {
        return Err("Redirect Location contains forbidden backslash".to_string());
    }

    let resolved = current.join(target_location).map_err(|e| {
        format!(
            "Failed to parse redirect location '{}': {}",
            target_location, e
        )
    })?;

    // 1. Strict HTTPS enforcement (no downgrade to HTTP)
    if resolved.scheme() != "https" {
        return Err(format!(
            "Insecure redirect rejected: attempted downgrade from HTTPS to '{}'",
            resolved.scheme()
        ));
    }

    // 2. Strict Origin enforcement (no redirect to external/different host)
    let target_host = resolved
        .host_str()
        .ok_or_else(|| "Target URL missing host".to_string())?;

    if target_host != original_host {
        return Err(format!(
            "Cross-origin redirect rejected: attempted redirect from host '{}' to '{}'",
            original_host, target_host
        ));
    }

    // 3. Port match
    if resolved.port() != current.port() {
        return Err("Redirect to different port rejected".to_string());
    }

    // 4. No credentials
    if !resolved.username().is_empty() || resolved.password().is_some() {
        return Err("Redirect to URL with credentials rejected".to_string());
    }

    Ok(resolved.to_string())
}

pub fn compute_package_tree_hash(
    package_dir: &Path,
    manifest: &RyzoraManifest,
) -> Result<String, String> {
    let mut file_hashes: Vec<(String, String)> = Vec::new();

    for file_decl in &manifest.files {
        let file_path = package_dir.join(&file_decl.source);
        if !file_path.is_file() {
            return Err(format!(
                "Package payload file missing: '{}'",
                file_decl.source
            ));
        }

        let file_bytes = fs::read(&file_path).map_err(|e| {
            format!(
                "Failed to read file '{}' for tree hash: {}",
                file_decl.source, e
            )
        })?;

        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &file_bytes);
        let file_sha256 = format!("{:x}", hasher.finalize());

        file_hashes.push((file_decl.source.clone(), file_sha256));
    }

    file_hashes.sort_by(|a, b| a.0.cmp(&b.0));

    let mut tree_hasher = sha2::Sha256::new();
    for (rel_path, f_hash) in &file_hashes {
        sha2::Digest::update(&mut tree_hasher, rel_path.as_bytes());
        sha2::Digest::update(&mut tree_hasher, f_hash.as_bytes());
    }

    Ok(format!("{:x}", tree_hasher.finalize()))
}

pub fn collect_regular_files_recursive(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }
    for entry in
        fs::read_dir(dir).map_err(|e| format!("Failed to read dir {}: {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            let mut sub = collect_regular_files_recursive(&path)?;
            files.append(&mut sub);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

pub fn verify_cached_package_integrity(
    pkg_cache_dir: &Path,
    manifest: &RyzoraManifest,
    expected_content_hash: Option<&str>,
) -> Result<(), String> {
    let actual_files = collect_regular_files_recursive(pkg_cache_dir)?;
    let mut declared_files_set = HashSet::new();
    declared_files_set.insert(PathBuf::from("manifest.json"));
    for f in &manifest.files {
        declared_files_set.insert(PathBuf::from(&f.source));
    }

    for file_path in actual_files {
        let rel_path = file_path
            .strip_prefix(pkg_cache_dir)
            .map_err(|e| format!("Path prefix error: {}", e))?;
        if !declared_files_set.contains(rel_path) {
            return Err(format!(
                "Unexpected extra file detected in cached package directory: '{}'",
                rel_path.display()
            ));
        }
    }

    if let Some(expected_hash) = expected_content_hash {
        let computed = compute_package_tree_hash(pkg_cache_dir, manifest)?;
        if !computed.eq_ignore_ascii_case(expected_hash) {
            return Err(format!(
                "Tree hash mismatch: expected {}, got {}",
                expected_hash, computed
            ));
        }
    } else {
        for file_decl in &manifest.files {
            let f_path = pkg_cache_dir.join(&file_decl.source);
            if !f_path.is_file() {
                return Err(format!(
                    "Package payload file missing: '{}'",
                    file_decl.source
                ));
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP Fetcher Abstraction & Implementations
// ─────────────────────────────────────────────────────────────────────────────

pub trait HttpFetcher: Send + Sync {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String>;
}

pub struct UreqHttpFetcher;

impl HttpFetcher for UreqHttpFetcher {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        validate_remote_url(url)?;

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .redirects(0)
            .build();

        let mut current_url = url.to_string();
        let mut redirect_count = 0;
        const MAX_REDIRECTS: usize = 5;

        loop {
            let resp_result = agent.get(&current_url).call();

            let resp = match resp_result {
                Ok(r) => r,
                Err(ureq::Error::Status(code, r)) if (300..=399).contains(&code) => r,
                Err(e) => return Err(format!("HTTP request failed for '{}': {}", current_url, e)),
            };

            let status = resp.status();
            if (300..=399).contains(&status) {
                if redirect_count >= MAX_REDIRECTS {
                    return Err(format!(
                        "Exceeded maximum redirects ({}) while fetching '{}'",
                        MAX_REDIRECTS, url
                    ));
                }

                let location = resp.header("Location").ok_or_else(|| {
                    format!(
                        "HTTP {} redirect missing Location header from '{}'",
                        status, current_url
                    )
                })?;

                let next_url = validate_redirect(&current_url, location)?;
                current_url = next_url;
                redirect_count += 1;
                continue;
            }

            if status < 200 || status >= 300 {
                return Err(format!("HTTP error {} fetching '{}'", status, current_url));
            }

            let mut reader = resp.into_reader().take((max_bytes + 1) as u64);
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .map_err(|e| format!("Failed to read response from '{}': {}", current_url, e))?;

            if buf.len() > max_bytes {
                return Err(format!(
                    "Response from '{}' exceeds maximum allowed size ({} bytes)",
                    current_url, max_bytes
                ));
            }

            return Ok(buf);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Trait & Implementations
// ─────────────────────────────────────────────────────────────────────────────

pub trait Repository: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn repo_type(&self) -> &str;
    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String>;
    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String>;
    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String>;
    fn refresh(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn status(&self) -> &str {
        "online"
    }
    fn last_refreshed(&self) -> Option<String> {
        None
    }
    fn last_error(&self) -> Option<String> {
        None
    }
    fn cache_dir(&self) -> Option<&Path> {
        None
    }
}

// ──────── LocalRepository ────────

pub struct LocalRepository {
    root_dir: PathBuf,
    index: RepositoryIndex,
}

impl LocalRepository {
    pub fn load_from_dir(root_dir: &Path) -> Result<Self, String> {
        let index_path = root_dir.join("repository.json");
        if !index_path.is_file() {
            return Err(format!(
                "repository.json not found in '{}'",
                root_dir.display()
            ));
        }

        let raw = fs::read_to_string(&index_path).map_err(|e| {
            format!(
                "Failed to read repository.json in '{}': {}",
                root_dir.display(),
                e
            )
        })?;

        let index: RepositoryIndex = serde_json::from_str(&raw).map_err(|e| {
            format!(
                "Malformed repository index in '{}': {}",
                root_dir.display(),
                e
            )
        })?;

        if index.schema != 1 {
            return Err(format!(
                "Unsupported repository schema {} (expected schema 1)",
                index.schema
            ));
        }

        validate_path_identifier(&index.id, "repository ID")?;

        let mut seen_ids = HashSet::new();
        let canonical_root = root_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize repository root: {}", e))?;

        for pkg in &index.packages {
            validate_repository_package_entry(pkg)?;

            if !seen_ids.insert(pkg.id.clone()) {
                return Err(format!(
                    "Duplicate package ID '{}' in repository index",
                    pkg.id
                ));
            }

            let full_manifest = root_dir.join(&pkg.manifest);
            if !full_manifest.exists() {
                return Err(format!(
                    "Manifest file not found for package '{}': '{}'",
                    pkg.id, pkg.manifest
                ));
            }

            let canonical_manifest = full_manifest
                .canonicalize()
                .map_err(|e| format!("Failed to canonicalize manifest for '{}': {}", pkg.id, e))?;

            if !canonical_manifest.starts_with(&canonical_root) {
                return Err(format!(
                    "Manifest path '{}' escapes repository root via symlink",
                    pkg.manifest
                ));
            }

            let manifest_raw = fs::read_to_string(&canonical_manifest)
                .map_err(|e| format!("Failed to read manifest for '{}': {}", pkg.id, e))?;

            let validation = validate_manifest_internal(&manifest_raw);
            if !validation.valid {
                return Err(format!(
                    "Package '{}' has invalid manifest: {}",
                    pkg.id,
                    validation.errors.join("; ")
                ));
            }

            let manifest: RyzoraManifest = serde_json::from_str(&manifest_raw)
                .map_err(|e| format!("Failed to parse manifest for '{}': {}", pkg.id, e))?;

            if manifest.id != pkg.id {
                return Err(format!(
                    "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository index expects '{}'",
                    pkg.id, manifest.id, pkg.id
                ));
            }

            if manifest.version != pkg.version {
                return Err(format!(
                    "Manifest version mismatch for package '{}': manifest declares version '{}', repository index expects '{}'",
                    pkg.id, manifest.version, pkg.version
                ));
            }

            if manifest.package_type != pkg.package_type {
                return Err(format!(
                    "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository index expects '{:?}'",
                    pkg.id, manifest.package_type, pkg.package_type
                ));
            }
        }

        Ok(Self {
            root_dir: root_dir.to_path_buf(),
            index,
        })
    }
}

impl Repository for LocalRepository {
    fn id(&self) -> &str {
        &self.index.id
    }

    fn name(&self) -> &str {
        &self.index.name
    }

    fn repo_type(&self) -> &str {
        "local"
    }

    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String> {
        Ok(self.index.packages.clone())
    }

    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        let entry = self
            .index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.index.id
                )
            })?;

        let full_manifest = self.root_dir.join(&entry.manifest);
        let canonical_root = self
            .root_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize root: {}", e))?;
        let canonical_manifest = full_manifest.canonicalize().map_err(|e| {
            format!(
                "Failed to canonicalize manifest for '{}': {}",
                package_id, e
            )
        })?;

        if !canonical_manifest.starts_with(&canonical_root) {
            return Err(format!(
                "Manifest path escapes repository root: '{}'",
                entry.manifest
            ));
        }

        let raw = fs::read_to_string(&canonical_manifest)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;

        let manifest: RyzoraManifest =
            serde_json::from_str(&raw).map_err(|e| format!("Failed to parse manifest: {}", e))?;

        if manifest.id != entry.id {
            return Err(format!(
                "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository index expects '{}'",
                entry.id, manifest.id, entry.id
            ));
        }

        if manifest.version != entry.version {
            return Err(format!(
                "Manifest version mismatch for package '{}': manifest declares version '{}', repository index expects '{}'",
                entry.id, manifest.version, entry.version
            ));
        }

        if manifest.package_type != entry.package_type {
            return Err(format!(
                "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository index expects '{:?}'",
                entry.id, manifest.package_type, entry.package_type
            ));
        }

        Ok(manifest)
    }

    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        let entry = self
            .index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.index.id
                )
            })?;

        let manifest_path = self.root_dir.join(&entry.manifest);
        let pkg_dir = manifest_path
            .parent()
            .ok_or_else(|| format!("Invalid manifest path '{}'", entry.manifest))?;

        let canonical_root = self
            .root_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize root: {}", e))?;
        let canonical_pkg = pkg_dir.canonicalize().map_err(|e| {
            format!(
                "Failed to canonicalize package dir for '{}': {}",
                package_id, e
            )
        })?;

        if !canonical_pkg.starts_with(&canonical_root) {
            return Err(format!(
                "Package directory escapes repository root: '{}'",
                entry.manifest
            ));
        }

        Ok(canonical_pkg)
    }
}

// ──────── RemoteRepository ────────

pub struct RemoteRepository {
    id: String,
    name: String,
    url: String,
    cache_dir: PathBuf,
    index: Option<RepositoryIndex>,
    fetcher: Box<dyn HttpFetcher>,
    status: String,
    last_refreshed: Option<String>,
    last_error: Option<String>,
}

impl RemoteRepository {
    pub fn new(id: String, name: String, url: String, cache_dir: PathBuf) -> Result<Self, String> {
        validate_remote_url(&url)?;
        Self::with_fetcher(id, name, url, cache_dir, Box::new(UreqHttpFetcher))
    }

    pub fn with_fetcher(
        id: String,
        name: String,
        url: String,
        cache_dir: PathBuf,
        fetcher: Box<dyn HttpFetcher>,
    ) -> Result<Self, String> {
        validate_remote_url(&url)?;
        validate_path_identifier(&id, "remote repository ID")?;

        let mut repo = Self {
            id,
            name,
            url,
            cache_dir,
            index: None,
            fetcher,
            status: "offline".to_string(),
            last_refreshed: None,
            last_error: None,
        };

        if let Ok(_) = repo.load_cached_index() {
            repo.status = "cached".to_string();
        }
        Ok(repo)
    }

    pub fn load_cached_index(&mut self) -> Result<RepositoryIndex, String> {
        let index_path = self.cache_dir.join("repository.json");
        if !index_path.is_file() {
            return Err("No cached repository index found".to_string());
        }
        let raw = fs::read_to_string(&index_path)
            .map_err(|e| format!("Failed to read cached repository.json: {}", e))?;
        let index = Self::validate_and_parse_index(&raw)?;
        self.index = Some(index.clone());
        Ok(index)
    }

    pub fn refresh_index(&mut self) -> Result<RepositoryIndex, String> {
        let url = resolve_repository_url(&self.url, "repository.json")?;
        let bytes = match self.fetcher.fetch(&url, MAX_INDEX_SIZE) {
            Ok(b) => b,
            Err(e) => {
                self.status = if self.index.is_some() {
                    "offline".to_string()
                } else {
                    "refresh_failed".to_string()
                };
                self.last_error = Some(e.clone());
                // If network fetch fails, keep valid cached index if available
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(format!("Failed to refresh remote repository: {}", e));
            }
        };

        let raw = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => {
                self.status = "refresh_failed".to_string();
                let msg = format!("Invalid UTF-8 in repository.json: {}", e);
                self.last_error = Some(msg.clone());
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(msg);
            }
        };

        let index = match Self::validate_and_parse_index(&raw) {
            Ok(idx) => idx,
            Err(e) => {
                self.status = "refresh_failed".to_string();
                self.last_error = Some(e.clone());
                if let Some(ref cached) = self.index {
                    return Ok(cached.clone());
                }
                return Err(e);
            }
        };

        // Atomically save to cache directory
        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        let tmp_path = self.cache_dir.join("repository.json.tmp");
        fs::write(&tmp_path, &raw)
            .map_err(|e| format!("Failed to write temp repository.json: {}", e))?;
        let final_path = self.cache_dir.join("repository.json");
        fs::rename(&tmp_path, &final_path)
            .map_err(|e| format!("Failed to atomically rename repository.json: {}", e))?;

        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.index = Some(index.clone());
        self.status = "online".to_string();
        self.last_refreshed = Some(format!("{}", now_sec));
        self.last_error = None;
        Ok(index)
    }

    fn validate_and_parse_index(raw: &str) -> Result<RepositoryIndex, String> {
        let index: RepositoryIndex = serde_json::from_str(raw)
            .map_err(|e| format!("Malformed remote repository index: {}", e))?;

        if index.schema != 1 {
            return Err(format!(
                "Unsupported remote repository schema {} (expected schema 1)",
                index.schema
            ));
        }

        validate_path_identifier(&index.id, "repository ID")?;

        let mut seen = HashSet::new();
        for pkg in &index.packages {
            validate_repository_package_entry(pkg)?;

            if !seen.insert(pkg.id.clone()) {
                return Err(format!("Duplicate package ID '{}' in remote index", pkg.id));
            }
        }

        Ok(index)
    }
}

impl Repository for RemoteRepository {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn repo_type(&self) -> &str {
        "remote"
    }

    fn refresh(&mut self) -> Result<(), String> {
        self.refresh_index().map(|_| ())
    }

    fn status(&self) -> &str {
        &self.status
    }

    fn last_refreshed(&self) -> Option<String> {
        self.last_refreshed.clone()
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }

    fn cache_dir(&self) -> Option<&Path> {
        Some(&self.cache_dir)
    }

    fn list_entries(&self) -> Result<Vec<RepositoryPackageEntry>, String> {
        match &self.index {
            Some(idx) => Ok(idx.packages.clone()),
            None => Err(format!(
                "Remote repository '{}' has not been fetched yet",
                self.id
            )),
        }
    }

    fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        let index = match &self.index {
            Some(idx) => idx,
            None => {
                return Err(format!(
                    "Remote repository '{}' has no loaded index",
                    self.id
                ))
            }
        };

        let entry = index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.id
                )
            })?;

        let cached_manifest =
            get_safe_cache_path(&self.cache_dir, &self.id, &entry.id, &entry.version)?
                .join("manifest.json");

        if cached_manifest.is_file() {
            let raw = fs::read_to_string(&cached_manifest)
                .map_err(|e| format!("Failed to read cached manifest: {}", e))?;
            let manifest: RyzoraManifest = serde_json::from_str(&raw)
                .map_err(|e| format!("Failed to parse cached manifest: {}", e))?;

            if manifest.id != entry.id {
                return Err(format!(
                    "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                    entry.id, manifest.id, entry.id
                ));
            }
            if manifest.version != entry.version {
                return Err(format!(
                    "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                    entry.id, manifest.version, entry.version
                ));
            }
            if manifest.package_type != entry.package_type {
                return Err(format!(
                    "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                    entry.id, manifest.package_type, entry.package_type
                ));
            }
            return Ok(manifest);
        }

        // Fetch manifest on demand using secure URL resolution
        let manifest_url = resolve_repository_url(&self.url, &entry.manifest)?;
        let bytes = self.fetcher.fetch(&manifest_url, MAX_MANIFEST_SIZE)?;
        let raw = String::from_utf8(bytes)
            .map_err(|e| format!("Invalid UTF-8 in downloaded manifest: {}", e))?;

        let validation = validate_manifest_internal(&raw);
        if !validation.valid {
            return Err(format!(
                "Downloaded manifest for '{}' failed validation: {}",
                entry.id,
                validation.errors.join("; ")
            ));
        }

        let manifest: RyzoraManifest = serde_json::from_str(&raw)
            .map_err(|e| format!("Failed to parse downloaded manifest: {}", e))?;

        if manifest.id != entry.id {
            return Err(format!(
                "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                entry.id, manifest.id, entry.id
            ));
        }
        if manifest.version != entry.version {
            return Err(format!(
                "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                entry.id, manifest.version, entry.version
            ));
        }
        if manifest.package_type != entry.package_type {
            return Err(format!(
                "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                entry.id, manifest.package_type, entry.package_type
            ));
        }

        Ok(manifest)
    }

    fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        let index = match &self.index {
            Some(idx) => idx,
            None => {
                return Err(format!(
                    "Remote repository '{}' has no loaded index",
                    self.id
                ))
            }
        };

        let entry = index
            .packages
            .iter()
            .find(|p| p.id == package_id)
            .ok_or_else(|| {
                format!(
                    "Package '{}' not found in repository '{}'",
                    package_id, self.id
                )
            })?;

        let pkg_cache_dir =
            get_safe_cache_path(&self.cache_dir, &self.id, &entry.id, &entry.version)?;
        let manifest_file = pkg_cache_dir.join("manifest.json");

        let mut cache_is_valid = false;

        // Cryptographically re-verify existing cache before trusting
        if manifest_file.is_file() {
            if let Ok(manifest_raw) = fs::read_to_string(&manifest_file) {
                if validate_manifest_internal(&manifest_raw).valid {
                    if let Ok(m) = serde_json::from_str::<RyzoraManifest>(&manifest_raw) {
                        // Check manifest identity and package type consistency
                        if m.id == entry.id
                            && m.version == entry.version
                            && m.package_type == entry.package_type
                        {
                            match verify_cached_package_integrity(
                                &pkg_cache_dir,
                                &m,
                                entry.content_hash.as_deref(),
                            ) {
                                Ok(()) => {
                                    cache_is_valid = true;
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[ryzora] Cached package '{}' failed integrity check: {}. Purging cache.",
                                        entry.id, e
                                    );
                                }
                            }
                        } else {
                            eprintln!(
                                "[ryzora] Cached package '{}' manifest identity or type mismatch. Purging cache.",
                                entry.id
                            );
                        }
                    }
                }
            }

            if cache_is_valid {
                return Ok(pkg_cache_dir);
            } else {
                // Purge corrupted/invalid cache immediately
                let _ = fs::remove_dir_all(&pkg_cache_dir);
            }
        }

        // Perform safe on-demand download into temporary staging directory
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let staging_dir = self
            .cache_dir
            .join("staging")
            .join(format!("{}-{}", entry.id, now));
        fs::create_dir_all(&staging_dir)
            .map_err(|e| format!("Failed to create staging dir: {}", e))?;

        let clean_staging = |dir: &Path| {
            let _ = fs::remove_dir_all(dir);
        };

        // Pre-check package size limit
        if let Some(size) = entry.package_size_bytes {
            if size > MAX_PACKAGE_SIZE {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package '{}' declared size {} exceeds maximum allowed ({} bytes)",
                    entry.id, size, MAX_PACKAGE_SIZE
                ));
            }
        }

        // 1. Download manifest.json using secure URL resolution
        let manifest_url = match resolve_repository_url(&self.url, &entry.manifest) {
            Ok(u) => u,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(e);
            }
        };

        let manifest_bytes = match self.fetcher.fetch(&manifest_url, MAX_MANIFEST_SIZE) {
            Ok(b) => b,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to download manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        let manifest_str = match String::from_utf8(manifest_bytes.clone()) {
            Ok(s) => s,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Invalid UTF-8 in manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        let validation = validate_manifest_internal(&manifest_str);
        if !validation.valid {
            clean_staging(&staging_dir);
            return Err(format!(
                "Downloaded manifest for '{}' failed validation: {}",
                entry.id,
                validation.errors.join("; ")
            ));
        }

        let manifest: RyzoraManifest = match serde_json::from_str(&manifest_str) {
            Ok(m) => m,
            Err(e) => {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to parse manifest for '{}': {}",
                    entry.id, e
                ));
            }
        };

        // Check manifest identity and package type consistency
        if manifest.id != entry.id {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest identity mismatch for package '{}': manifest declares ID '{}', repository entry expects '{}'",
                entry.id, manifest.id, entry.id
            ));
        }
        if manifest.version != entry.version {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest version mismatch for package '{}': manifest declares version '{}', repository entry expects '{}'",
                entry.id, manifest.version, entry.version
            ));
        }
        if manifest.package_type != entry.package_type {
            clean_staging(&staging_dir);
            return Err(format!(
                "Manifest package type mismatch for package '{}': manifest declares type '{:?}', repository entry expects '{:?}'",
                entry.id, manifest.package_type, entry.package_type
            ));
        }

        // Write manifest to staging
        if let Err(e) = fs::write(staging_dir.join("manifest.json"), &manifest_bytes) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to write manifest to staging: {}", e));
        }

        // 2. Download each declared payload file
        let mut total_package_bytes: u64 = manifest_bytes.len() as u64;

        let manifest_parent = Path::new(&entry.manifest)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");

        for file_decl in &manifest.files {
            if file_decl.source.starts_with('/')
                || file_decl.source.starts_with('\\')
                || file_decl.source.contains('\0')
            {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Source path '{}' must be relative",
                    file_decl.source
                ));
            }
            for comp in Path::new(&file_decl.source).components() {
                if let Component::ParentDir = comp {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Source path '{}' contains forbidden '..'",
                        file_decl.source
                    ));
                }
            }

            let rel_file_path = if manifest_parent.is_empty() {
                file_decl.source.clone()
            } else {
                format!(
                    "{}/{}",
                    manifest_parent.trim_end_matches('/'),
                    file_decl.source.trim_start_matches('/')
                )
            };

            let file_url = match resolve_repository_url(&self.url, &rel_file_path) {
                Ok(u) => u,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(e);
                }
            };

            let file_bytes = match self.fetcher.fetch(&file_url, MAX_FILE_SIZE as usize) {
                Ok(b) => b,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!(
                        "Failed to download package file '{}': {}",
                        file_decl.source, e
                    ));
                }
            };

            total_package_bytes += file_bytes.len() as u64;
            if total_package_bytes > MAX_PACKAGE_SIZE {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package '{}' exceeds maximum package size limit",
                    entry.id
                ));
            }

            let target_staged_path = staging_dir.join(&file_decl.source);
            if let Some(parent) = target_staged_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    clean_staging(&staging_dir);
                    return Err(format!("Failed to create staging parent dir: {}", e));
                }
            }

            if let Err(e) = fs::write(&target_staged_path, &file_bytes) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Failed to write staged file '{}': {}",
                    file_decl.source, e
                ));
            }
        }

        // 3. Verify content_hash if provided using deterministic tree hash
        if let Some(ref expected_hash) = entry.content_hash {
            let actual_package_hash = match compute_package_tree_hash(&staging_dir, &manifest) {
                Ok(h) => h,
                Err(e) => {
                    clean_staging(&staging_dir);
                    return Err(format!("Failed to compute tree hash for staging: {}", e));
                }
            };

            if !actual_package_hash.eq_ignore_ascii_case(expected_hash) {
                clean_staging(&staging_dir);
                return Err(format!(
                    "Package integrity verification failed for '{}': expected SHA-256 '{}', got '{}'",
                    entry.id, expected_hash, actual_package_hash
                ));
            }
        }

        // 4. Atomically move staging directory to cache
        if let Some(parent) = pkg_cache_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if pkg_cache_dir.exists() {
            let _ = fs::remove_dir_all(&pkg_cache_dir);
        }

        if let Err(e) = fs::rename(&staging_dir, &pkg_cache_dir) {
            clean_staging(&staging_dir);
            return Err(format!("Failed to finalize package cache: {}", e));
        }

        Ok(pkg_cache_dir)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Manager
// ─────────────────────────────────────────────────────────────────────────────

pub struct RepositoryManager {
    repositories: Vec<Box<dyn Repository>>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    pub fn add_repository(&mut self, repo: Box<dyn Repository>) {
        self.repositories.push(repo);
    }

    pub fn discover_local_repositories(&mut self, search_dirs: &[PathBuf]) {
        for search_dir in search_dirs {
            if !search_dir.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(search_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("repository.json").is_file() {
                        if let Ok(local_repo) = LocalRepository::load_from_dir(&path) {
                            self.repositories.push(Box::new(local_repo));
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_all(&mut self) -> Result<(), String> {
        for repo in &mut self.repositories {
            let _ = repo.refresh();
        }
        Ok(())
    }

    pub fn list_repositories(&self) -> Vec<RepositorySummary> {
        self.repositories
            .iter()
            .map(|r| RepositorySummary {
                id: r.id().to_string(),
                name: r.name().to_string(),
                package_count: r.list_entries().map(|e| e.len()).unwrap_or(0),
                path: format!("repository://{}", r.id()),
                repo_type: r.repo_type().to_string(),
                enabled: true,
                status: r.status().to_string(),
                last_refreshed: r.last_refreshed(),
                last_error: r.last_error(),
            })
            .collect()
    }

    pub fn repositories(&self) -> &[Box<dyn Repository>] {
        &self.repositories
    }

    pub fn get_repository_by_id(&self, id: &str) -> Option<&dyn Repository> {
        self.repositories
            .iter()
            .find(|r| r.id() == id)
            .map(|b| b.as_ref())
    }

    pub fn find_repository_for_package(&self, package_id: &str) -> Option<String> {
        for repo in &self.repositories {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    return Some(repo.id().to_string());
                }
            }
        }
        None
    }

    pub fn get_package_dir(&self, package_id: &str) -> Result<PathBuf, String> {
        for repo in &self.repositories {
            if let Ok(dir) = repo.get_package_dir(package_id) {
                return Ok(dir);
            }
        }
        Err(format!(
            "Package '{}' not found in any repository",
            package_id
        ))
    }

    pub fn get_package_manifest(&self, package_id: &str) -> Result<RyzoraManifest, String> {
        for repo in &self.repositories {
            if let Ok(entries) = repo.list_entries() {
                if entries.iter().any(|e| e.id == package_id) {
                    return repo.get_package_manifest(package_id);
                }
            }
        }
        Err(format!(
            "Package '{}' not found in any repository",
            package_id
        ))
    }

    pub fn list_all_packages(&self) -> Result<Vec<FrontendPackageItem>, String> {
        let mut all_packages = Vec::new();
        let mut seen_ids = HashSet::new();

        for repo in &self.repositories {
            let entries = match repo.list_entries() {
                Ok(e) => e,
                Err(_) => continue,
            };

            for entry in entries {
                if !seen_ids.insert(entry.id.clone()) {
                    continue; // Skip duplicates across repos
                }

                let manifest = repo.get_package_manifest(&entry.id).ok();

                let (
                    supported_desktops,
                    supported_display,
                    dependencies,
                    compatibility,
                    components,
                ) = if let Some(ref m) = manifest {
                    let desktops = m.compatibility.desktops.clone();
                    let sessions = m.compatibility.sessions.clone();
                    let deps = FrontendDependencies {
                        packages: m.compatibility.required.clone(),
                        optional: m.compatibility.optional.clone(),
                    };
                    let compat = CompatibilityRequirements {
                        supported_distros: m.compatibility.distros.clone(),
                        supported_desktops: m.compatibility.desktops.clone(),
                        supported_sessions: m.compatibility.sessions.clone(),
                        required_binaries: m.compatibility.required.clone(),
                        optional_binaries: m.compatibility.optional.clone(),
                    };
                    let comps = m
                        .files
                        .iter()
                        .map(|f| FrontendComponentSpec {
                            name: Path::new(&f.target)
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or(&f.target)
                                .to_string(),
                            component_type: format!("{:?}", m.package_type).to_lowercase(),
                            target_path: f.target.clone(),
                            description: f.description.clone(),
                        })
                        .collect();
                    (desktops, sessions, deps, compat, comps)
                } else {
                    (
                        vec![],
                        vec!["wayland".to_string()],
                        FrontendDependencies {
                            packages: vec![],
                            optional: vec![],
                        },
                        CompatibilityRequirements {
                            supported_distros: vec![],
                            supported_desktops: vec![],
                            supported_sessions: vec![],
                            required_binaries: vec![],
                            optional_binaries: vec![],
                        },
                        vec![],
                    )
                };

                let files_count = manifest.as_ref().map(|m| m.files.len()).unwrap_or(0);

                let is_hash_verified = entry.content_hash.is_some();
                let ryzora_cache_root = get_ryzora_cache_dir();
                let repo_cache_root = repo.cache_dir().unwrap_or(&ryzora_cache_root);

                let is_cached = if repo.repo_type() == "local" {
                    true
                } else if let Ok(cache_path) =
                    get_safe_cache_path(repo_cache_root, repo.id(), &entry.id, &entry.version)
                {
                    cache_path.join("manifest.json").is_file()
                } else {
                    false
                };

                let (integrity_status, is_corrupted) = if repo.repo_type() == "local" {
                    if is_hash_verified {
                        ("verified".to_string(), false)
                    } else {
                        ("unverified".to_string(), false)
                    }
                } else if is_cached {
                    if let Some(ref expected_hash) = entry.content_hash {
                        if let Ok(cache_path) = get_safe_cache_path(
                            repo_cache_root,
                            repo.id(),
                            &entry.id,
                            &entry.version,
                        ) {
                            if let Some(ref m) = manifest {
                                match verify_cached_package_integrity(
                                    &cache_path,
                                    m,
                                    Some(expected_hash.as_str()),
                                ) {
                                    Ok(()) => ("verified".to_string(), false),
                                    Err(_) => ("corrupted".to_string(), true),
                                }
                            } else {
                                ("corrupted".to_string(), true)
                            }
                        } else {
                            ("unverified".to_string(), false)
                        }
                    } else {
                        ("unverified".to_string(), false)
                    }
                } else if is_hash_verified {
                    ("pending_download".to_string(), false)
                } else {
                    ("unverified".to_string(), false)
                };

                let audit_rating = if is_corrupted {
                    "corrupted".to_string()
                } else if is_hash_verified {
                    "verified".to_string()
                } else {
                    "unverified".to_string()
                };

                // Phase 12 Cryptographic Trust Chain Evaluation
                let trust_store = crate::crypto::TrustStore::load_default();
                let tree_h = entry.content_hash.as_deref().unwrap_or("");
                let crypto_eval = crate::crypto::evaluate_trust_chain(
                    entry.signature.as_ref(),
                    &entry.id,
                    tree_h,
                    entry.trust_tier,
                    &trust_store,
                );

                let item = FrontendPackageItem {
                    id: entry.id.clone(),
                    title: entry.name.clone(),
                    subtitle: entry.description.clone(),
                    description: entry.description.clone(),
                    version: entry.version.clone(),
                    author: entry.author.clone(),
                    category: entry.category.clone(),
                    package_type: entry.package_type.clone(),
                    tags: entry.tags.clone(),
                    supported_desktops,
                    supported_display,
                    rating: entry.rating.unwrap_or(4.8),
                    rating_count: 120,
                    downloads: entry.downloads.unwrap_or(1000),
                    hero_image: entry.hero_image.clone().unwrap_or_default(),
                    screenshots: entry.screenshots.clone(),
                    featured: entry.featured,
                    trending: entry.trending,
                    color_palette: entry.color_palette.clone(),
                    safety_audit: FrontendSafetyAudit {
                        rating: audit_rating,
                        changes_system_files: false,
                        requires_root: false,
                        sandbox_compatible: true,
                        files_modified_count: files_count,
                    },
                    dependencies,
                    components,
                    compatibility,
                    manifest,
                    repository_id: Some(repo.id().to_string()),
                    content_hash: entry.content_hash.clone(),
                    package_size_bytes: entry.package_size_bytes,
                    integrity_status,
                    is_cached,
                    release_channel: Some(entry.release_channel.unwrap_or_else(|| {
                        let v_lower = entry.version.to_lowercase();
                        if v_lower.contains("nightly") {
                            ReleaseChannel::Nightly
                        } else if v_lower.contains("beta")
                            || v_lower.contains("rc")
                            || v_lower.contains("alpha")
                        {
                            ReleaseChannel::Beta
                        } else {
                            ReleaseChannel::Stable
                        }
                    })),
                    trust_tier: Some(entry.trust_tier.unwrap_or_else(|| {
                        if is_corrupted {
                            TrustTier::Untrusted
                        } else if repo.id() == "ryzora-official" {
                            TrustTier::Official
                        } else if entry.author.verified && is_hash_verified {
                            TrustTier::Verified
                        } else {
                            TrustTier::Community
                        }
                    })),
                    moderation_status: Some(
                        entry
                            .moderation_status
                            .unwrap_or(ModerationStatus::Approved),
                    ),
                    trending_score: Some(entry.trending_score.unwrap_or_else(|| {
                        let dl = entry.downloads.unwrap_or(1000) as f64;
                        let rt = entry.rating.unwrap_or(4.8);
                        let tr = if entry.trending.unwrap_or(false) {
                            50.0
                        } else {
                            0.0
                        };
                        let ft = if entry.featured.unwrap_or(false) {
                            30.0
                        } else {
                            0.0
                        };
                        (dl * 0.05) + (rt * 10.0) + tr + ft
                    })),
                    maintainer: entry.maintainer.clone(),
                    release_notes: entry.release_notes.clone(),
                    signature: entry.signature.clone(),
                    cryptographic_status: Some(crypto_eval.status),
                };

                all_packages.push(item);
            }
        }

        Ok(all_packages)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Configuration Persistence & Discovery
// ─────────────────────────────────────────────────────────────────────────────

pub fn get_ryzora_cache_dir() -> PathBuf {
    get_home_dir().join(".local/share/ryzora/cache")
}

pub fn get_repository_config_file() -> PathBuf {
    get_home_dir().join(".local/share/ryzora/repositories.json")
}

pub fn load_repository_configs() -> Vec<RepositorySourceConfig> {
    let path = get_repository_config_file();
    if path.is_file() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(configs) = serde_json::from_str::<Vec<RepositorySourceConfig>>(&raw) {
                return configs;
            }
        }
    }

    // Default configurations: Official & Community remotes (GitHub-backed) + Local repository
    vec![
        RepositorySourceConfig {
            id: "ryzora-official".to_string(),
            name: "Ryzora Official (GitHub)".to_string(),
            url: "https://raw.githubusercontent.com/Gayensubhajit/Ryzora-official-repo/main"
                .to_string(),
            enabled: false,
            repo_type: "remote".to_string(),
        },
        RepositorySourceConfig {
            id: "ryzora-community".to_string(),
            name: "Ryzora Community (GitHub)".to_string(),
            url:
                "https://raw.githubusercontent.com/Gayensubhajit/Ryzora/main/repositories/community"
                    .to_string(),
            enabled: false,
            repo_type: "remote".to_string(),
        },
        RepositorySourceConfig {
            id: "community-local".to_string(),
            name: "Ryzora Community Repository (Local)".to_string(),
            url: "local://repositories/community".to_string(),
            enabled: true,
            repo_type: "local".to_string(),
        },
    ]
}

pub fn save_repository_configs(configs: &[RepositorySourceConfig]) -> Result<(), String> {
    let path = get_repository_config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(configs)
        .map_err(|e| format!("Failed to serialize repository configs: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to save repository configs: {}", e))
}

pub fn get_default_repository_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("repositories"));
        if cwd.file_name().and_then(|n| n.to_str()) == Some("src-tauri") {
            if let Some(parent) = cwd.parent() {
                dirs.push(parent.join("repositories"));
            }
        }
    }

    dirs.push(get_home_dir().join(".local/share/ryzora/repositories"));
    dirs
}

pub fn create_default_manager() -> RepositoryManager {
    let mut manager = RepositoryManager::new();

    // 1. Discover local repositories from filesystem
    let search_dirs = get_default_repository_search_dirs();
    manager.discover_local_repositories(&search_dirs);

    // 2. Discover configured remote repositories
    let configs = load_repository_configs();
    let cache_base = get_ryzora_cache_dir();

    for cfg in configs {
        if cfg.enabled && cfg.repo_type == "remote" {
            let cache_dir = cache_base.join(&cfg.id);
            if let Ok(remote_repo) =
                RemoteRepository::new(cfg.id.clone(), cfg.name.clone(), cfg.url.clone(), cache_dir)
            {
                manager.add_repository(Box::new(remote_repo));
            }
        }
    }

    manager
}

pub fn find_package_dir(package_id: &str) -> Result<PathBuf, String> {
    let manager = create_default_manager();
    manager.get_package_dir(package_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_catalog_packages() -> Result<Vec<FrontendPackageItem>, String> {
    let manager = create_default_manager();
    manager.list_all_packages()
}

#[tauri::command]
pub fn refresh_catalog() -> Result<Vec<FrontendPackageItem>, String> {
    let mut manager = create_default_manager();
    let _ = manager.refresh_all();
    manager.list_all_packages()
}

#[tauri::command]
pub fn get_repository_info() -> Result<Vec<RepositorySummary>, String> {
    let manager = create_default_manager();
    Ok(manager.list_repositories())
}

#[tauri::command]
pub fn list_repository_sources() -> Result<Vec<RepositorySourceConfig>, String> {
    Ok(load_repository_configs())
}

#[tauri::command]
pub fn add_repository_source(config: RepositorySourceConfig) -> Result<(), String> {
    validate_path_identifier(&config.id, "Repository ID")?;

    if config.repo_type == "remote" {
        validate_remote_url(&config.url)?;
    }

    let mut configs = load_repository_configs();
    if configs.iter().any(|c| c.id == config.id) {
        return Err(format!("Repository ID '{}' already exists", config.id));
    }

    configs.push(config);
    save_repository_configs(&configs)
}

#[tauri::command]
pub fn remove_repository_source(id: String) -> Result<(), String> {
    let mut configs = load_repository_configs();
    configs.retain(|c| c.id != id);
    save_repository_configs(&configs)
}

// ─────────────────────────────────────────────────────────────────────────────
// Cache Management & Confinement (Phase 8.12)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_size_bytes: u64,
    pub cached_packages_count: usize,
    pub cached_repositories_count: usize,
    pub cache_dir: String,
}

pub fn get_directory_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(meta) = p.metadata() {
                    total += meta.len();
                }
            } else if p.is_dir() {
                total += get_directory_size(&p);
            }
        }
    }
    total
}

pub fn validate_cache_root_confinement(cache_root: &Path) -> Result<(), String> {
    if cache_root.as_os_str().is_empty() {
        return Err("Cache root cannot be empty".to_string());
    }

    let home = get_home_dir();
    let config_dir = home.join(".config");
    let installed_dir = home.join(".local/share/ryzora/installed");
    let snapshots_dir = home.join(".local/share/ryzora/snapshots");

    // Must never be root, home, .config, installed, or snapshots
    if cache_root == Path::new("/")
        || cache_root == home
        || cache_root == config_dir
        || cache_root == installed_dir
        || cache_root == snapshots_dir
    {
        return Err("Cache root points to protected system or user directory".to_string());
    }

    if cache_root.starts_with(&config_dir)
        || cache_root.starts_with(&installed_dir)
        || cache_root.starts_with(&snapshots_dir)
    {
        return Err("Cache root escapes into protected directory".to_string());
    }

    Ok(())
}

pub fn inspect_cache(cache_root: &Path) -> Result<CacheStats, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(CacheStats {
            total_size_bytes: 0,
            cached_packages_count: 0,
            cached_repositories_count: 0,
            cache_dir: cache_root.display().to_string(),
        });
    }

    let total_size_bytes = get_directory_size(cache_root);
    let mut cached_packages_count = 0;
    let mut cached_repositories_count = 0;

    if let Ok(entries) = fs::read_dir(cache_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "staging" {
                    continue;
                }
                if p.join("repository.json").is_file() {
                    cached_repositories_count += 1;
                }
                let has_package = if let Ok(subentries) = fs::read_dir(&p) {
                    subentries
                        .flatten()
                        .any(|sub| sub.path().join("manifest.json").is_file())
                } else {
                    false
                };
                if has_package || p.join("manifest.json").is_file() {
                    cached_packages_count += 1;
                }
            }
        }
    }

    Ok(CacheStats {
        total_size_bytes,
        cached_packages_count,
        cached_repositories_count,
        cache_dir: cache_root.display().to_string(),
    })
}

pub fn clean_package_cache(cache_root: &Path, package_id: Option<&str>) -> Result<u64, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(0);
    }

    let mut bytes_freed = 0;

    if let Some(pkg_id) = package_id {
        validate_path_identifier(pkg_id, "package ID")?;
        let pkg_dir = cache_root.join(pkg_id);
        if pkg_dir.exists() {
            bytes_freed += get_directory_size(&pkg_dir);
            fs::remove_dir_all(&pkg_dir)
                .map_err(|e| format!("Failed to remove package cache for '{}': {}", pkg_id, e))?;
        }
    } else {
        if let Ok(entries) = fs::read_dir(cache_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == "staging" {
                        bytes_freed += get_directory_size(&p);
                        let _ = fs::remove_dir_all(&p);
                        continue;
                    }
                    if !p.join("repository.json").is_file() {
                        bytes_freed += get_directory_size(&p);
                        let _ = fs::remove_dir_all(&p);
                    }
                }
            }
        }
    }

    Ok(bytes_freed)
}

pub fn clean_all_cache(cache_root: &Path) -> Result<u64, String> {
    validate_cache_root_confinement(cache_root)?;

    if !cache_root.exists() {
        return Ok(0);
    }

    let bytes_freed = get_directory_size(cache_root);
    if let Ok(entries) = fs::read_dir(cache_root) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let _ = fs::remove_file(&p);
            } else if p.is_dir() {
                let _ = fs::remove_dir_all(&p);
            }
        }
    }

    Ok(bytes_freed)
}

#[tauri::command]
pub fn get_cache_stats() -> Result<CacheStats, String> {
    let cache_dir = get_ryzora_cache_dir();
    inspect_cache(&cache_dir)
}

#[tauri::command]
pub fn clear_package_cache(package_id: Option<String>) -> Result<u64, String> {
    let cache_dir = get_ryzora_cache_dir();
    clean_package_cache(&cache_dir, package_id.as_deref())
}

#[tauri::command]
pub fn clear_all_cache() -> Result<u64, String> {
    let cache_dir = get_ryzora_cache_dir();
    clean_all_cache(&cache_dir)
}

// ─────────────────────────────────────────────────────────────────────────────
// Comprehensive Unit & Security Tests
// Zero touch of real ~/.config or real network. All use sandbox & MockHttpFetcher.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    // ──────── Mock HTTP Fetcher with Redirect Support ────────

    #[derive(Clone)]
    pub enum MockAction {
        Response(Vec<u8>),
        Redirect(String),
        Error(String),
    }

    #[derive(Clone)]
    struct MockHttpFetcher {
        responses: Arc<Mutex<HashMap<String, MockAction>>>,
    }

    impl MockHttpFetcher {
        fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn set_response(&self, url: &str, data: Vec<u8>) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Response(data));
        }

        fn set_redirect(&self, url: &str, target: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Redirect(target.to_string()));
        }

        fn set_error(&self, url: &str, err: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(url.to_string(), MockAction::Error(err.to_string()));
        }
    }

    impl HttpFetcher for MockHttpFetcher {
        fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
            validate_remote_url(url)?;

            let mut current_url = url.to_string();
            let mut redirect_count = 0;
            const MAX_REDIRECTS: usize = 5;

            loop {
                let map = self.responses.lock().unwrap();
                let action = map
                    .get(&current_url)
                    .ok_or_else(|| format!("Mock 404: '{}'", current_url))?
                    .clone();
                drop(map);

                match action {
                    MockAction::Response(res) => {
                        if res.len() > max_bytes {
                            return Err(format!(
                                "Response from '{}' exceeds maximum allowed size ({} bytes)",
                                current_url, max_bytes
                            ));
                        }
                        return Ok(res);
                    }
                    MockAction::Redirect(target) => {
                        if redirect_count >= MAX_REDIRECTS {
                            return Err(format!(
                                "Exceeded maximum redirects ({}) while fetching '{}'",
                                MAX_REDIRECTS, url
                            ));
                        }
                        let next_url = validate_redirect(&current_url, &target)?;
                        current_url = next_url;
                        redirect_count += 1;
                        continue;
                    }
                    MockAction::Error(err) => {
                        return Err(err);
                    }
                }
            }
        }
    }

    // ──────── Test Sandbox ────────

    struct RepoTestSandbox {
        root: PathBuf,
        repo_dir: PathBuf,
        cache_dir: PathBuf,
    }

    impl RepoTestSandbox {
        fn new(name: &str) -> Self {
            let unique_id = format!(
                "ryzora-phase6-test-{}-{}",
                name,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            let root = std::env::temp_dir().join(unique_id);
            let repo_dir = root.join("community");
            let cache_dir = root.join("cache");
            fs::create_dir_all(&repo_dir).unwrap();
            fs::create_dir_all(&cache_dir).unwrap();

            Self {
                root,
                repo_dir,
                cache_dir,
            }
        }

        fn create_valid_package(&self, id: &str) -> String {
            let pkg_dir = self.repo_dir.join("packages").join(id);
            fs::create_dir_all(pkg_dir.join("files")).unwrap();

            let manifest_json = format!(
                r##"{{
  "id": "{}",
  "name": "Test {}",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "A test package",
  "tags": ["test"],
  "color_palette": ["#ffffff"],
  "compatibility": {{
    "desktops": ["hyprland"],
    "sessions": ["wayland"],
    "distros": [],
    "required": [],
    "optional": []
  }},
  "files": [
    {{
      "source": "files/config.conf",
      "target": "~/.config/test/config.conf",
      "description": "test file"
    }}
  ]
}}"##,
                id, id
            );

            fs::write(pkg_dir.join("manifest.json"), manifest_json).unwrap();
            fs::write(pkg_dir.join("files/config.conf"), "test content").unwrap();

            format!("packages/{}/manifest.json", id)
        }

        fn write_repository_json(&self, content: &str) {
            fs::write(self.repo_dir.join("repository.json"), content).unwrap();
        }
    }

    impl Drop for RepoTestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 6 & 6.1 Security Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_remote_repo_https_accepted() {
        assert!(validate_remote_url("https://raw.githubusercontent.com/owner/repo/main").is_ok());
    }

    #[test]
    fn test_remote_repo_rejects_http() {
        let res = validate_remote_url("http://example.com/repo");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("HTTPS only"));
    }

    #[test]
    fn test_remote_repo_rejects_forbidden_schemes() {
        let bad_schemes = vec![
            "file:///etc/passwd",
            "ftp://ftp.example.com/repo",
            "data:text/plain;base64,SGVsbG8=",
            "javascript:alert(1)",
        ];
        for bad_url in bad_schemes {
            let res = validate_remote_url(bad_url);
            assert!(res.is_err());
            assert!(res.err().unwrap().contains("HTTPS only"));
        }
    }

    #[test]
    fn test_remote_repo_rejects_null_bytes_and_traversal() {
        assert!(validate_remote_url("https://example.com/repo\0/bad").is_err());
        assert!(validate_remote_url("https://example.com/repo/../../bad").is_err());
    }

    #[test]
    fn test_remote_repo_malformed_json_rejected() {
        let sandbox = RepoTestSandbox::new("remote-malformed");
        let mock = MockHttpFetcher::new();
        mock.set_response(
            "https://example.com/repo/repository.json",
            b"{ invalid json".to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Malformed remote repository index"));
    }

    #[test]
    fn test_remote_repo_unsupported_schema_rejected() {
        let sandbox = RepoTestSandbox::new("remote-schema");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 99,
  "id": "test-remote",
  "name": "Remote",
  "packages": []
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Unsupported remote repository schema 99"));
    }

    #[test]
    fn test_remote_repo_duplicate_package_ids_rejected() {
        let sandbox = RepoTestSandbox::new("remote-dup");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "dup-pkg", "name": "Dup 1", "version": "1.0.0", "package_type": "rice", "manifest": "packages/dup-pkg/manifest.json", "author": { "name": "A", "avatar": "" } },
    { "id": "dup-pkg", "name": "Dup 2", "version": "1.0.0", "package_type": "rice", "manifest": "packages/dup-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Duplicate package ID 'dup-pkg'"));
    }

    #[test]
    fn test_remote_repo_manifest_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("remote-traversal");
        let mock = MockHttpFetcher::new();
        let json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "evil-pkg", "name": "Evil", "version": "1.0.0", "package_type": "rice", "manifest": "../evil/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response("https://example.com/repo/repository.json", json.to_vec());

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("forbidden parent traversal"));
    }

    #[test]
    fn test_remote_repo_oversized_index_rejected() {
        let sandbox = RepoTestSandbox::new("remote-oversize");
        let mock = MockHttpFetcher::new();
        let huge_data = vec![b'a'; MAX_INDEX_SIZE + 10];
        mock.set_response("https://example.com/repo/repository.json", huge_data);

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        let res = repo.refresh_index();
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("exceeds maximum allowed size"));
    }

    #[test]
    fn test_remote_repo_package_sha256_verification_and_atomic_caching() {
        let sandbox = RepoTestSandbox::new("remote-sha256");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Cyberpunk config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "cyber-remote",
      "name": "Cyber Remote",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cyber-remote/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "cyber-remote",
  "name": "Cyber Remote",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        assert!(pkg_dir.join("manifest.json").is_file());
        assert!(pkg_dir.join("files/test.conf").is_file());

        let staging_dir = sandbox.cache_dir.join("staging");
        if staging_dir.exists() {
            assert_eq!(fs::read_dir(staging_dir).unwrap().count(), 0);
        }

        // Re-verification succeeds on second call
        let pkg_dir_2 = repo.get_package_dir("cyber-remote").unwrap();
        assert_eq!(pkg_dir, pkg_dir_2);
    }

    #[test]
    fn test_remote_repo_package_sha256_mismatch_rejected_and_staging_purged() {
        let sandbox = RepoTestSandbox::new("remote-sha-fail");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "tampered-pkg",
      "name": "Tampered",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/tampered-pkg/manifest.json",
      "content_hash": "deadbeef00000000000000000000000000000000000000000000000000000000",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "tampered-pkg",
  "name": "Tampered",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/bad.conf", "target": "~/.config/bad.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/tampered-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/tampered-pkg/files/bad.conf",
            b"malicious content".to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("tampered-pkg");
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Package integrity verification failed"));

        let final_cache = sandbox.cache_dir.join("tampered-pkg");
        assert!(!final_cache.exists());

        let staging_dir = sandbox.cache_dir.join("staging");
        if staging_dir.exists() {
            assert_eq!(fs::read_dir(staging_dir).unwrap().count(), 0);
        }
    }

    #[test]
    fn test_remote_repo_network_failure_retains_valid_cached_index() {
        let sandbox = RepoTestSandbox::new("remote-fallback");
        let mock = MockHttpFetcher::new();

        let valid_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "cached-pkg", "name": "Cached", "version": "1.0.0", "package_type": "rice", "manifest": "packages/cached-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            valid_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        assert_eq!(repo.list_entries().unwrap().len(), 1);

        mock.set_error(
            "https://example.com/repo/repository.json",
            "Connection timed out",
        );

        let res = repo.refresh_index();
        assert!(res.is_ok());
        assert_eq!(repo.list_entries().unwrap().len(), 1);
        assert_eq!(repo.list_entries().unwrap()[0].id, "cached-pkg");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 6.1 Hardening Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_repository_url_valid_cases() {
        let base = "https://example.com/repo";
        assert_eq!(
            resolve_repository_url(base, "repository.json").unwrap(),
            "https://example.com/repo/repository.json"
        );
        assert_eq!(
            resolve_repository_url(base, "packages/cyber/manifest.json").unwrap(),
            "https://example.com/repo/packages/cyber/manifest.json"
        );
        assert_eq!(
            resolve_repository_url(base, "packages/cyber/files/waybar.conf").unwrap(),
            "https://example.com/repo/packages/cyber/files/waybar.conf"
        );
    }

    #[test]
    fn test_resolve_repository_url_rejects_traversal() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "../escaped.json").is_err());
        assert!(resolve_repository_url(base, "packages/../../escaped.json").is_err());
        assert!(resolve_repository_url(base, "foo/bar/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_encoded_traversal() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "%2e%2e/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%2E%2E/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%2e./escaped.json").is_err());
        assert!(resolve_repository_url(base, ".%2e/escaped.json").is_err());
        assert!(resolve_repository_url(base, "%252e%252e/double_encoded.json").is_err());
        assert!(resolve_repository_url(base, "%2fetc%2fpasswd").is_err());
        assert!(resolve_repository_url(base, "packages%5cescaped").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_schemes_and_absolute() {
        let base = "https://example.com/repo";
        assert!(resolve_repository_url(base, "https://evil.com/payload.json").is_err());
        assert!(resolve_repository_url(base, "http://example.com/repo/payload.json").is_err());
        assert!(resolve_repository_url(base, "file:///etc/shadow").is_err());
        assert!(resolve_repository_url(base, "javascript:alert(1)").is_err());
        assert!(resolve_repository_url(base, "//evil.com/payload.json").is_err());
        assert!(resolve_repository_url(base, "/rooted/path.json").is_err());
    }

    #[test]
    fn test_resolve_repository_url_rejects_credentials_and_null() {
        assert!(resolve_repository_url("https://user:pass@example.com/repo", "test.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "null\0byte.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "back\\slash.json").is_err());
        assert!(resolve_repository_url("https://example.com/repo", "fragment.json#frag").is_err());
    }

    #[test]
    fn test_validate_redirect_security() {
        let curr = "https://example.com/repo/package.json";

        // Same host HTTPS relative -> OK
        assert_eq!(
            validate_redirect(curr, "/repo/v2/package.json").unwrap(),
            "https://example.com/repo/v2/package.json"
        );
        // Same host HTTPS absolute -> OK
        assert_eq!(
            validate_redirect(curr, "https://example.com/repo/v2/package.json").unwrap(),
            "https://example.com/repo/v2/package.json"
        );

        // Downgrade to HTTP -> Rejected
        let res_http = validate_redirect(curr, "http://example.com/repo/package.json");
        assert!(res_http.is_err());
        assert!(res_http.err().unwrap().contains("attempted downgrade"));

        // Cross-origin host switch -> Rejected
        let res_evil = validate_redirect(curr, "https://evil.com/repo/package.json");
        assert!(res_evil.is_err());
        assert!(res_evil
            .err()
            .unwrap()
            .contains("Cross-origin redirect rejected"));

        // Credentials -> Rejected
        let res_cred = validate_redirect(curr, "https://user:pass@example.com/repo/package.json");
        assert!(res_cred.is_err());
        assert!(res_cred.err().unwrap().contains("credentials rejected"));

        // Insecure schemes -> Rejected
        assert!(validate_redirect(curr, "javascript:alert(1)").is_err());
        assert!(validate_redirect(curr, "file:///etc/passwd").is_err());
    }

    #[test]
    fn test_mock_fetcher_follows_valid_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "https://example.com/repo/packages/cyber-v2/manifest.json",
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-v2/manifest.json",
            b"{\"status\":\"ok\"}".to_vec(),
        );

        let data = mock
            .fetch(
                "https://example.com/repo/packages/cyber/manifest.json",
                1024,
            )
            .unwrap();
        assert_eq!(data, b"{\"status\":\"ok\"}");
    }

    #[test]
    fn test_mock_fetcher_rejects_insecure_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "http://example.com/insecure/manifest.json",
        );

        let res = mock.fetch(
            "https://example.com/repo/packages/cyber/manifest.json",
            1024,
        );
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("downgrade"));
    }

    #[test]
    fn test_mock_fetcher_rejects_cross_origin_redirect() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/packages/cyber/manifest.json",
            "https://attacker.com/malicious/manifest.json",
        );

        let res = mock.fetch(
            "https://example.com/repo/packages/cyber/manifest.json",
            1024,
        );
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Cross-origin redirect rejected"));
    }

    #[test]
    fn test_mock_fetcher_rejects_redirect_loop() {
        let mock = MockHttpFetcher::new();
        mock.set_redirect(
            "https://example.com/repo/loop1",
            "https://example.com/repo/loop2",
        );
        mock.set_redirect(
            "https://example.com/repo/loop2",
            "https://example.com/repo/loop1",
        );

        let res = mock.fetch("https://example.com/repo/loop1", 1024);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Exceeded maximum redirects"));
    }

    #[test]
    fn test_cache_trust_reverification_modified_file_purged_and_redownloaded() {
        let sandbox = RepoTestSandbox::new("cache-reverify-mod");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Cyberpunk config v1";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "cyber-remote",
      "name": "Cyber Remote",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cyber-remote/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "cyber-remote",
  "name": "Cyber Remote",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/cyber-remote/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // 1. Initial download & cache
        let pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        let file_path = pkg_dir.join("files/test.conf");
        assert_eq!(fs::read(&file_path).unwrap(), file_content);

        // 2. Adversary or disk corruption tampers with the cached file!
        fs::write(&file_path, b"# Maliciously injected config!").unwrap();

        // 3. Next get_package_dir MUST detect mismatch, purge corrupted cache, and re-download clean file!
        let re_pkg_dir = repo.get_package_dir("cyber-remote").unwrap();
        assert_eq!(re_pkg_dir, pkg_dir);
        let restored_content = fs::read(re_pkg_dir.join("files/test.conf")).unwrap();
        assert_eq!(restored_content, file_content);
    }

    #[test]
    fn test_cache_trust_reverification_corrupt_cache_never_passed_when_offline() {
        let sandbox = RepoTestSandbox::new("cache-offline-corrupt");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Safe config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "safe-pkg",
      "name": "Safe",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/safe-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "safe-pkg",
  "name": "Safe",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/safe-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/safe-pkg/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        let pkg_dir = repo.get_package_dir("safe-pkg").unwrap();

        // Tamper with cached file
        fs::write(pkg_dir.join("files/test.conf"), b"# Corrupted!").unwrap();

        // Network goes down!
        mock.set_error(
            "https://example.com/repo/packages/safe-pkg/files/test.conf",
            "Connection refused",
        );

        // Requesting package dir MUST fail with error and NEVER return corrupted cache!
        let res = repo.get_package_dir("safe-pkg");
        assert!(res.is_err());
        assert!(!pkg_dir.exists(), "Corrupted cache must be deleted");
    }

    #[test]
    fn test_cache_trust_deleted_file_triggers_purge_and_redownload() {
        let sandbox = RepoTestSandbox::new("cache-deleted-file");
        let mock = MockHttpFetcher::new();

        let file_content = b"# File to delete";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/test.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "del-pkg",
      "name": "Del",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/del-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "del-pkg",
  "name": "Del",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/test.conf", "target": "~/.config/test.conf", "description": "t" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/del-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/del-pkg/files/test.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();
        let pkg_dir = repo.get_package_dir("del-pkg").unwrap();

        // Delete the file from cache
        fs::remove_file(pkg_dir.join("files/test.conf")).unwrap();

        // Re-verifying must detect missing file, purge cache, and re-download
        let recovered_dir = repo.get_package_dir("del-pkg").unwrap();
        assert!(recovered_dir.join("files/test.conf").is_file());
    }

    #[test]
    fn test_cache_path_safety_validation() {
        assert!(validate_path_identifier("safe-pkg-123", "pkg").is_ok());
        assert!(validate_path_identifier("v1.0.0+build1", "ver").is_ok());
        assert!(validate_path_identifier("repo_name.ext", "repo").is_ok());

        assert!(validate_path_identifier("", "pkg").is_err());
        assert!(validate_path_identifier("   ", "pkg").is_err());
        assert!(validate_path_identifier("../escape", "pkg").is_err());
        assert!(validate_path_identifier("foo/bar", "pkg").is_err());
        assert!(validate_path_identifier("foo\\bar", "pkg").is_err());
        assert!(validate_path_identifier("pkg\0null", "pkg").is_err());
        assert!(validate_path_identifier(".", "pkg").is_err());
        assert!(validate_path_identifier("..", "pkg").is_err());
        assert!(validate_path_identifier("pkg with spaces", "pkg").is_err());
    }

    #[test]
    fn test_package_metadata_validation() {
        // Valid 64 hex characters
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .is_ok());

        // Invalid length
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85"
        )
        .is_err()); // 63
        assert!(validate_sha256_hash(
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b8555"
        )
        .is_err()); // 65

        // Non-hex
        assert!(validate_sha256_hash(
            "g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )
        .is_err());
        assert!(validate_sha256_hash(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        )
        .is_err());
    }

    #[test]
    fn test_manifest_identity_id_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("id-mismatch");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "declared-id", "name": "Mismatch", "version": "1.0.0", "package_type": "rice", "manifest": "packages/mismatch/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;

        let manifest_json = br#"{
  "id": "different-id",
  "name": "Mismatch",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/mismatch/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("declared-id");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest identity mismatch"));
    }

    #[test]
    fn test_manifest_identity_version_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("ver-mismatch");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "pkg-one", "name": "Version Mismatch", "version": "2.0.0", "package_type": "rice", "manifest": "packages/pkg-one/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-one",
  "name": "Version Mismatch",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-one/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let res = repo.get_package_dir("pkg-one");
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest version mismatch"));
    }

    #[test]
    fn test_safety_audit_distinguishes_verified_vs_unverified() {
        let sandbox = RepoTestSandbox::new("audit-rating");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "verified-pkg", "name": "V", "version": "1.0.0", "package_type": "rice", "manifest": "packages/v/manifest.json", "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", "author": { "name": "A", "avatar": "" } },
    { "id": "unverified-pkg", "name": "U", "version": "1.0.0", "package_type": "rice", "manifest": "packages/u/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(repo));

        let catalog = manager.list_all_packages().unwrap();
        let v_item = catalog.iter().find(|i| i.id == "verified-pkg").unwrap();
        let u_item = catalog.iter().find(|i| i.id == "unverified-pkg").unwrap();

        assert_eq!(v_item.safety_audit.rating, "verified");
        assert_eq!(u_item.safety_audit.rating, "unverified");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 5 LocalRepository Tests (Ensuring zero regressions)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_valid_repository_loading() {
        let sandbox = RepoTestSandbox::new("valid-repo");
        let manifest_path = sandbox.create_valid_package("test-pkg");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "test-pkg",
      "name": "Test Package",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            manifest_path
        );

        sandbox.write_repository_json(&repo_json);

        let repo = LocalRepository::load_from_dir(&sandbox.repo_dir).unwrap();
        assert_eq!(repo.id(), "community");
        assert_eq!(repo.name(), "Community");

        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "test-pkg");

        let manifest = repo.get_package_manifest("test-pkg").unwrap();
        assert_eq!(manifest.id, "test-pkg");

        let pkg_dir = repo.get_package_dir("test-pkg").unwrap();
        assert!(pkg_dir.is_dir());
        assert!(pkg_dir.join("manifest.json").is_file());
    }

    #[test]
    fn test_malformed_repository_json_rejected() {
        let sandbox = RepoTestSandbox::new("malformed-repo");
        sandbox.write_repository_json("{ invalid json syntax");

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Malformed repository index"));
    }

    #[test]
    fn test_unsupported_schema_version_rejected() {
        let sandbox = RepoTestSandbox::new("bad-schema");
        sandbox.write_repository_json(
            r#"{
  "schema": 2,
  "id": "community",
  "name": "Community",
  "packages": []
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .contains("Unsupported repository schema 2"));
    }

    #[test]
    fn test_invalid_package_id_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("traversal-id");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "../escaped-pkg",
      "name": "Escaped",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Invalid package ID"));
    }

    #[test]
    fn test_duplicate_package_ids_rejected() {
        let sandbox = RepoTestSandbox::new("dup-ids");
        let manifest_path = sandbox.create_valid_package("test-pkg");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "test-pkg",
      "name": "First",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }},
    {{
      "id": "test-pkg",
      "name": "Duplicate",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{0}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            manifest_path
        );

        sandbox.write_repository_json(&repo_json);

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Duplicate package ID"));
    }

    #[test]
    fn test_manifest_traversal_rejected() {
        let sandbox = RepoTestSandbox::new("manifest-traversal");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "evil-pkg",
      "name": "Evil",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "../../../etc/passwd",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("forbidden parent traversal"));
    }

    #[test]
    fn test_missing_manifest_rejected() {
        let sandbox = RepoTestSandbox::new("missing-manifest");
        sandbox.write_repository_json(
            r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "missing-pkg",
      "name": "Missing",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/nonexistent/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#,
        );

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Manifest file not found"));
    }

    #[test]
    fn test_malformed_manifest_rejected() {
        let sandbox = RepoTestSandbox::new("malformed-manifest");
        let pkg_dir = sandbox.repo_dir.join("packages").join("bad-pkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), "{ invalid manifest json").unwrap();

        let repo_json = r#"{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {
      "id": "bad-pkg",
      "name": "Bad",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/bad-pkg/manifest.json",
      "author": { "name": "Tester", "avatar": "" }
    }
  ]
}"#;

        sandbox.write_repository_json(repo_json);

        let res = LocalRepository::load_from_dir(&sandbox.repo_dir);
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("invalid manifest"));
    }

    #[test]
    fn test_repository_manager_discovery_and_catalog() {
        let sandbox = RepoTestSandbox::new("manager-test");
        let m1 = sandbox.create_valid_package("pkg-one");
        let m2 = sandbox.create_valid_package("pkg-two");

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "community",
  "name": "Community",
  "packages": [
    {{
      "id": "pkg-one",
      "name": "Package One",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }},
    {{
      "id": "pkg-two",
      "name": "Package Two",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "{}",
      "author": {{ "name": "Tester", "avatar": "" }}
    }}
  ]
}}"#,
            m1, m2
        );

        sandbox.write_repository_json(&repo_json);

        let mut manager = RepositoryManager::new();
        manager.discover_local_repositories(&[sandbox.root.clone()]);

        let repos = manager.list_repositories();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].id, "community");
        assert_eq!(repos[0].package_count, 2);

        let catalog = manager.list_all_packages().unwrap();
        assert_eq!(catalog.len(), 2);
        let ids: Vec<String> = catalog.iter().map(|p| p.id.clone()).collect();
        assert!(ids.contains(&"pkg-one".to_string()));
        assert!(ids.contains(&"pkg-two".to_string()));

        let dir1 = manager.get_package_dir("pkg-one").unwrap();
        assert!(dir1.join("manifest.json").is_file());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 8 Production Store & Workflow Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_phase8_metadata_validation_out_of_bounds_rating() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-rating".to_string(),
            name: "Rating Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec![],
            hero_image: None,
            screenshots: vec![],
            featured: None,
            trending: None,
            rating: Some(5.5),
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
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(-0.1);
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(f64::NAN);
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.rating = Some(4.9);
        assert!(validate_repository_package_entry(&pkg).is_ok());
    }

    #[test]
    fn test_phase8_metadata_validation_invalid_color_palette() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-color".to_string(),
            name: "Color Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec!["not-a-color-at-all-and-extremely-long-string".to_string()],
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
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.color_palette = vec!["#1a1b26".to_string(), "#7aa2f7".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_ok());
    }

    #[test]
    fn test_phase8_metadata_validation_traversal_in_images() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-img".to_string(),
            name: "Image Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec![],
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
            },
            color_palette: vec![],
            hero_image: Some("../secret.png".to_string()),
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
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.hero_image = Some("/etc/shadow.png".to_string());
        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.hero_image = Some("assets/preview.png".to_string());
        assert!(validate_repository_package_entry(&pkg).is_ok());
        pkg.screenshots = vec!["../evil.jpg".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_err());
    }

    #[test]
    fn test_phase8_resolve_repository_url_advanced_attacks() {
        let base = "https://example.com/repo/";

        // Protocol-relative URL
        assert!(resolve_repository_url(base, "//evil.com/payload").is_err());

        // Embedded scheme
        assert!(resolve_repository_url(base, "javascript://alert(1)").is_err());
        assert!(resolve_repository_url(base, "file:///etc/passwd").is_err());

        // Fragment
        assert!(resolve_repository_url(base, "packages/file#section").is_err());

        // Multi-level encoded traversal
        assert!(resolve_repository_url(base, "%252e%252e/escaped").is_err());
        assert!(resolve_repository_url(base, "..%2f..%2fetc/passwd").is_err());

        // Valid nested path
        let resolved = resolve_repository_url(base, "packages/cyber/manifest.json").unwrap();
        assert_eq!(
            resolved,
            "https://example.com/repo/packages/cyber/manifest.json"
        );
    }

    #[test]
    fn test_phase8_cache_trust_extra_unexpected_file_purges_and_redownloads() {
        let sandbox = RepoTestSandbox::new("cache-unexpected-file");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Payload config";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/app.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "pkg-extra",
      "name": "Extra File Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg-extra/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "pkg-extra",
  "name": "Extra File Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.as_bytes().to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-extra/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-extra/files/app.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Download once into cache
        let pkg_dir = repo.get_package_dir("pkg-extra").unwrap();
        assert!(pkg_dir.join("files/app.conf").is_file());

        // Attacker drops an unexpected file into the cached package
        fs::write(pkg_dir.join("unexpected_payload.sh"), b"echo malicious").unwrap();
        assert!(pkg_dir.join("unexpected_payload.sh").is_file());

        // Re-request package dir: unexpected file triggers purge & redownload
        let pkg_dir_new = repo.get_package_dir("pkg-extra").unwrap();
        assert!(
            !pkg_dir_new.join("unexpected_payload.sh").is_file(),
            "Unexpected file should have been purged!"
        );
        assert!(pkg_dir_new.join("files/app.conf").is_file());
    }

    #[test]
    fn test_phase8_cache_trust_modified_manifest_package_type_purges_and_redownloads() {
        let sandbox = RepoTestSandbox::new("cache-mod-type");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Payload";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/app.conf");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "pkg-type",
      "name": "Type Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg-type/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "pkg-type",
  "name": "Type Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.as_bytes().to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-type/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg-type/files/app.conf",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let pkg_dir = repo.get_package_dir("pkg-type").unwrap();
        assert!(pkg_dir.join("files/app.conf").is_file());

        // Modify cached manifest package_type from rice to theme
        let tampered_manifest = br#"{
  "id": "pkg-type",
  "name": "Type Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "theme",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/app.conf", "target": "~/.config/app/app.conf", "description": "App" }
  ]
}"#;
        fs::write(pkg_dir.join("manifest.json"), tampered_manifest).unwrap();

        // Next access should detect inconsistency, purge, and redownload original rice manifest
        let pkg_dir_new = repo.get_package_dir("pkg-type").unwrap();
        let current_m: RyzoraManifest =
            serde_json::from_str(&fs::read_to_string(pkg_dir_new.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(current_m.package_type, PackageType::Rice);
    }

    #[test]
    fn test_phase8_consistency_package_type_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-type");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-mismatch-type",
      "name": "Mismatch Type",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        // Remote manifest declares theme instead of rice
        let manifest_json = br#"{
  "id": "pkg-mismatch-type",
  "name": "Mismatch Type",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "theme",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-mismatch-type").unwrap_err();
        assert!(err.contains("Manifest package type mismatch"));

        let dir_err = repo.get_package_dir("pkg-mismatch-type").unwrap_err();
        assert!(dir_err.contains("Manifest package type mismatch"));
    }

    #[test]
    fn test_phase8_offline_mode_cached_index_browsable_and_repo_marked_offline() {
        let sandbox = RepoTestSandbox::new("offline-browsable");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "cached-pkg",
      "name": "Cached Package",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/cached-pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        // 1. First refresh succeeds
        repo.refresh_index().unwrap();
        assert_eq!(repo.status(), "online");

        // 2. Network goes down
        mock.set_error(
            "https://example.com/repo/repository.json",
            "Network unreachable",
        );

        // Refresh fails, but retains cached index
        let res = repo.refresh_index();
        assert!(res.is_ok(), "Should retain cached index on network failure");
        assert_eq!(repo.status(), "offline", "Status should be marked offline");
        assert!(repo.last_error().is_some());

        // Packages still listed from cache
        let entries = repo.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "cached-pkg");
    }

    #[test]
    fn test_phase8_offline_mode_uncached_package_fails_cleanly() {
        let sandbox = RepoTestSandbox::new("offline-uncached");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "uncached-pkg",
      "name": "Uncached",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/uncached/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Offline network error when trying to fetch uncached package
        mock.set_error(
            "https://example.com/repo/packages/uncached/manifest.json",
            "Connection refused",
        );
        let err = repo.get_package_dir("uncached-pkg").unwrap_err();
        assert!(err.contains("Connection refused") || err.contains("Failed to download manifest"));
    }

    #[test]
    fn test_phase8_cache_management_stats_and_cleanup() {
        let sandbox = RepoTestSandbox::new("cache-mgmt");
        let cache_root = sandbox.root.join("test_cache");
        fs::create_dir_all(&cache_root).unwrap();

        // Create a fake cached repository and a package
        let repo_dir = cache_root.join("remote-one");
        fs::create_dir_all(&repo_dir).unwrap();
        fs::write(repo_dir.join("repository.json"), b"{}").unwrap();

        let pkg_dir = cache_root.join("pkg-one").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("manifest.json"), b"{}").unwrap();
        fs::write(pkg_dir.join("data.bin"), vec![0u8; 1024]).unwrap();

        let stats = inspect_cache(&cache_root).unwrap();
        assert!(stats.total_size_bytes > 1024);
        assert_eq!(stats.cached_repositories_count, 1);
        assert_eq!(stats.cached_packages_count, 1);

        // Clean specific package
        let freed = clean_package_cache(&cache_root, Some("pkg-one")).unwrap();
        assert!(freed >= 1024);
        assert!(!cache_root.join("pkg-one").exists());

        // Clean all
        let all_freed = clean_all_cache(&cache_root).unwrap();
        assert!(all_freed > 0);
        assert!(!repo_dir.join("repository.json").exists());
    }

    #[test]
    fn test_phase8_cache_management_safety_guards_prevent_escape() {
        let home = get_home_dir();
        assert!(validate_cache_root_confinement(Path::new("/")).is_err());
        assert!(validate_cache_root_confinement(&home).is_err());
        assert!(validate_cache_root_confinement(&home.join(".config")).is_err());
        assert!(
            validate_cache_root_confinement(&home.join(".local/share/ryzora/installed")).is_err()
        );
        assert!(
            validate_cache_root_confinement(&home.join(".local/share/ryzora/snapshots")).is_err()
        );
        assert!(validate_cache_root_confinement(&home.join(".config/nested")).is_err());
    }

    #[test]
    fn test_phase8_consistency_id_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-id");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-repo-id",
      "name": "ID Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-manifest-diff-id",
  "name": "ID Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-repo-id").unwrap_err();
        assert!(err.contains("Manifest identity mismatch"));
    }

    #[test]
    fn test_phase8_consistency_version_mismatch_rejected() {
        let sandbox = RepoTestSandbox::new("consistency-ver");
        let mock = MockHttpFetcher::new();

        let repo_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {
      "id": "pkg-ver-test",
      "name": "Ver Pkg",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/pkg/manifest.json",
      "author": { "name": "A", "avatar": "" }
    }
  ]
}"#;

        let manifest_json = br#"{
  "id": "pkg-ver-test",
  "name": "Ver Pkg",
  "version": "2.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": []
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/pkg/manifest.json",
            manifest_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        let err = repo.get_package_manifest("pkg-ver-test").unwrap_err();
        assert!(err.contains("Manifest version mismatch"));
    }

    #[test]
    fn test_phase8_metadata_validation_tags_and_author_length() {
        let mut pkg = RepositoryPackageEntry {
            id: "pkg-limits".to_string(),
            name: "Limits Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Rice,
            description: "Test".to_string(),
            manifest: "manifest.json".to_string(),
            category: "desktop".to_string(),
            tags: vec!["a".repeat(55)], // exceeds 50 chars
            author: AuthorInfo {
                name: "Tester".to_string(),
                avatar: "".to_string(),
                verified: false,
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
        };

        assert!(validate_repository_package_entry(&pkg).is_err());
        pkg.tags = vec!["valid-tag".to_string()];
        assert!(validate_repository_package_entry(&pkg).is_ok());

        pkg.author.name = " ".to_string(); // empty author
        assert!(validate_repository_package_entry(&pkg).is_err());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 8.1 Release-Readiness & UX Audit Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_phase8_1_remote_repo_malformed_json_sets_refresh_failed_and_retains_cached_index() {
        let sandbox = RepoTestSandbox::new("p8-1-malformed-fallback");
        let mock = MockHttpFetcher::new();

        let valid_json = br#"{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    { "id": "cached-pkg", "name": "Cached", "version": "1.0.0", "package_type": "rice", "manifest": "packages/cached-pkg/manifest.json", "author": { "name": "A", "avatar": "" } }
  ]
}"#;
        mock.set_response(
            "https://example.com/repo/repository.json",
            valid_json.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock.clone()),
        )
        .unwrap();

        // 1. Initial successful refresh
        repo.refresh_index().unwrap();
        assert_eq!(repo.status(), "online");
        assert!(repo.last_error().is_none());
        assert_eq!(repo.list_entries().unwrap().len(), 1);

        // 2. Server later returns malformed JSON on subsequent refresh
        mock.set_response(
            "https://example.com/repo/repository.json",
            b"{ malformed json string without close".to_vec(),
        );

        let res = repo.refresh_index();
        // Surviving cached index is returned
        assert!(res.is_ok());
        assert_eq!(repo.status(), "refresh_failed");
        assert!(repo.last_error().is_some());
        assert!(repo
            .last_error()
            .unwrap()
            .contains("Malformed remote repository index"));
        // Cached packages remain intact
        assert_eq!(repo.list_entries().unwrap().len(), 1);
        assert_eq!(repo.list_entries().unwrap()[0].id, "cached-pkg");
    }

    #[test]
    fn test_phase8_1_catalog_marks_corrupt_cache_as_corrupted_and_valid_cache_as_verified() {
        let sandbox = RepoTestSandbox::new("p8-1-catalog-trust");
        let mock = MockHttpFetcher::new();

        let file_content = b"# Safe configuration";
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, file_content);
        let file_sha256 = format!("{:x}", hasher.finalize());

        let mut tree_hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut tree_hasher, b"files/config.toml");
        sha2::Digest::update(&mut tree_hasher, file_sha256.as_bytes());
        let expected_tree_hash = format!("{:x}", tree_hasher.finalize());

        let repo_json = format!(
            r#"{{
  "schema": 1,
  "id": "test-remote",
  "name": "Remote",
  "packages": [
    {{
      "id": "valid-pkg",
      "name": "Valid",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/valid-pkg/manifest.json",
      "content_hash": "{}",
      "author": {{ "name": "A", "avatar": "" }}
    }},
    {{
      "id": "unhashed-pkg",
      "name": "Unhashed",
      "version": "1.0.0",
      "package_type": "rice",
      "manifest": "packages/unhashed-pkg/manifest.json",
      "author": {{ "name": "A", "avatar": "" }}
    }}
  ]
}}"#,
            expected_tree_hash
        );

        let manifest_json = br#"{
  "id": "valid-pkg",
  "name": "Valid",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "A",
  "package_type": "rice",
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [{ "source": "files/config.toml", "target": "~/.config/config.toml", "description": "c" }]
}"#;

        mock.set_response(
            "https://example.com/repo/repository.json",
            repo_json.into_bytes(),
        );
        mock.set_response(
            "https://example.com/repo/packages/valid-pkg/manifest.json",
            manifest_json.to_vec(),
        );
        mock.set_response(
            "https://example.com/repo/packages/valid-pkg/files/config.toml",
            file_content.to_vec(),
        );

        let mut repo = RemoteRepository::with_fetcher(
            "test-remote".to_string(),
            "Remote".to_string(),
            "https://example.com/repo".to_string(),
            sandbox.cache_dir.clone(),
            Box::new(mock),
        )
        .unwrap();

        repo.refresh_index().unwrap();

        // Download valid-pkg so it is in cache
        let pkg_dir = repo.get_package_dir("valid-pkg").unwrap();
        assert!(pkg_dir.exists());

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(repo));

        // Before corruption: valid-pkg is verified, unhashed-pkg is unverified
        let catalog = manager.list_all_packages().unwrap();
        let valid_item = catalog.iter().find(|p| p.id == "valid-pkg").unwrap();
        let unhashed_item = catalog.iter().find(|p| p.id == "unhashed-pkg").unwrap();

        assert_eq!(valid_item.integrity_status, "verified");
        assert_eq!(valid_item.safety_audit.rating, "verified");
        assert_eq!(unhashed_item.integrity_status, "unverified");
        assert_eq!(unhashed_item.safety_audit.rating, "unverified");

        // Now tamper with cached file in pkg_dir
        fs::write(pkg_dir.join("files/config.toml"), b"# Tampered payload!").unwrap();

        // Re-read catalog: valid-pkg must now be detected as corrupted, NEVER verified!
        let catalog_after = manager.list_all_packages().unwrap();
        let tampered_item = catalog_after.iter().find(|p| p.id == "valid-pkg").unwrap();

        assert_eq!(tampered_item.integrity_status, "corrupted");
        assert_eq!(tampered_item.safety_audit.rating, "corrupted");
    }
}
