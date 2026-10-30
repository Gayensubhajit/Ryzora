//! Ryzora repository data models, schema validation, local repository discovery,
//! and catalog querying.

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use url::Url;

use crate::compatibility::CompatibilityRequirements;
use crate::crypto::{CryptographicStatus, PackageSignatureMetadata};
use crate::distribution::{ModerationStatus, ReleaseChannel, TrustTier};
use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};

pub const MAX_INDEX_SIZE: usize = 5 * 1024 * 1024; // 5 MB
pub const MAX_MANIFEST_SIZE: usize = 2 * 1024 * 1024; // 2 MB
pub const MAX_FILE_SIZE: u64 = 500 * 1024 * 1024; // 500 MB (supports HD video assets)
pub const MAX_PACKAGE_SIZE: u64 = 1024 * 1024 * 1024; // 1 GB (supports rich multimedia bundles)

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSourceSpec {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageProvenanceSpec {
    pub upstream: String,
    pub revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
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
    pub signature: Option<PackageSignatureMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_video: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_animated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PackageSourceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<PackageProvenanceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subfilter: Option<String>,
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
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,
            media_type: None,
            source: None,
            provenance: None,
            subfilter: None,
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
    pub signature: Option<PackageSignatureMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cryptographic_status: Option<CryptographicStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_video: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_animated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_video_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_poster_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PackageSourceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<PackageProvenanceSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subfilter: Option<String>,
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

pub trait HttpFetcher: Send + Sync {
    fn fetch(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String>;
}

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

#[derive(Debug)]
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

// ─────────────────────────────────────────────────────────────────────────────
// Repository Discovery & Catalog Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve default search directories for local repositories.
/// Robust against launching from outside the repository root.
pub fn get_default_repository_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Explicit developer override RYZORA_REPOSITORY
    if let Ok(override_repo) = std::env::var("RYZORA_REPOSITORY") {
        let trimmed = override_repo.trim();
        if !trimmed.is_empty() {
            if let Some(file_path) = trimmed.strip_prefix("file://") {
                let p = PathBuf::from(file_path);
                if p.is_dir() {
                    dirs.push(p);
                }
            } else if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
                let p = PathBuf::from(trimmed);
                if p.is_dir() {
                    dirs.push(p);
                }
            }
        }
    }

    // 2. Current working directory checks
    if let Ok(cwd) = std::env::current_dir() {
        let repo_subdir = cwd.join("repositories");
        if repo_subdir.is_dir() {
            dirs.push(repo_subdir);
        }
        if cwd.file_name().and_then(|n| n.to_str()) == Some("qt") || cwd.file_name().and_then(|n| n.to_str()) == Some("build") {
            if let Some(parent) = cwd.parent() {
                let parent_repos = parent.join("repositories");
                if parent_repos.is_dir() {
                    dirs.push(parent_repos);
                }
            }
        }
    }

    // 3. Current executable directory checks (when run from outside CWD)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let exe_repos = exe_dir.join("repositories");
            if exe_repos.is_dir() && !dirs.contains(&exe_repos) {
                dirs.push(exe_repos);
            }
            let mut cur = exe_dir.parent();
            while let Some(parent) = cur {
                let candidate = parent.join("repositories");
                if candidate.is_dir() && !dirs.contains(&candidate) {
                    dirs.push(candidate);
                    break;
                }
                cur = parent.parent();
            }
        }
    }

    // 4. User data directory ~/.local/share/ryzora/repositories
    if let Ok(home) = std::env::var("HOME") {
        let user_share = PathBuf::from(home).join(".local/share/ryzora/repositories");
        if user_share.is_dir() && !dirs.contains(&user_share) {
            dirs.push(user_share);
        }
    }

    // 5. System wide /usr/share/ryzora/repositories
    let sys_repos = PathBuf::from("/usr/share/ryzora/repositories");
    if sys_repos.is_dir() && !dirs.contains(&sys_repos) {
        dirs.push(sys_repos);
    }

    dirs
}

/// Discover all local repositories within provided search paths.
pub fn discover_local_repositories(search_dirs: &[PathBuf]) -> Vec<LocalRepository> {
    let mut repos = Vec::new();
    let mut seen_ids = HashSet::new();

    for search_dir in search_dirs {
        if !search_dir.is_dir() {
            continue;
        }

        // Direct repository root check
        if search_dir.join("repository.json").is_file() {
            if let Ok(local_repo) = LocalRepository::load_from_dir(search_dir) {
                if seen_ids.insert(local_repo.id().to_string()) {
                    repos.push(local_repo);
                }
                continue;
            }
        }

        // Subdirectories check
        if let Ok(entries) = fs::read_dir(search_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("repository.json").is_file() {
                    if let Ok(local_repo) = LocalRepository::load_from_dir(&path) {
                        if seen_ids.insert(local_repo.id().to_string()) {
                            repos.push(local_repo);
                        }
                    }
                }
            }
        }
    }

    repos
}

/// Pure catalog filtering helper by category, search query, and desktop environment.
pub fn filter_catalog_items(
    items: &[FrontendPackageItem],
    category: Option<&str>,
    query: Option<&str>,
    desktop: Option<&str>,
) -> Vec<FrontendPackageItem> {
    let cat_filter = category
        .map(|c| c.trim().to_lowercase())
        .filter(|c| !c.is_empty() && c != "all");
    let q_filter = query
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let desk_filter = desktop
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty() && d != "all");

    items
        .iter()
        .filter(|item| {
            if let Some(ref cat) = cat_filter {
                if item.category.to_lowercase() != *cat {
                    return false;
                }
            }

            if let Some(ref desk) = desk_filter {
                if !item
                    .supported_desktops
                    .iter()
                    .any(|d| d.to_lowercase() == *desk)
                {
                    return false;
                }
            }

            if let Some(ref q) = q_filter {
                let title_match = item.title.to_lowercase().contains(q);
                let id_match = item.id.to_lowercase().contains(q);
                let desc_match = item.description.to_lowercase().contains(q);
                let tag_match = item.tags.iter().any(|t| t.to_lowercase().contains(q));
                let author_match = item.author.name.to_lowercase().contains(q);

                if !title_match && !id_match && !desc_match && !tag_match && !author_match {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_path_identifier_valid() {
        assert!(validate_path_identifier("community", "repo ID").is_ok());
        assert!(validate_path_identifier("fastfetch-cyber-spec", "pkg ID").is_ok());
        assert!(validate_path_identifier("v1.0.0", "version").is_ok());
    }

    #[test]
    fn test_validate_path_identifier_rejects_traversal_and_separators() {
        assert!(validate_path_identifier("..", "id").is_err());
        assert!(validate_path_identifier("../escape", "id").is_err());
        assert!(validate_path_identifier("foo/bar", "id").is_err());
        assert!(validate_path_identifier(r"foo\bar", "id").is_err());
        assert!(validate_path_identifier("foo\0bar", "id").is_err());
        assert!(validate_path_identifier("", "id").is_err());
    }

    #[test]
    fn test_validate_remote_url_requires_https() {
        assert!(validate_remote_url("https://example.com/repo.json").is_ok());
        assert!(validate_remote_url("http://example.com/repo.json").is_err());
        assert!(validate_remote_url("file:///etc/passwd").is_err());
        assert!(validate_remote_url("https://user:pass@example.com/repo").is_err());
    }

    #[test]
    fn test_filter_catalog_items_by_category_and_query() {
        let author = AuthorInfo {
            name: "Tester".to_string(),
            avatar: "avatar.png".to_string(),
            verified: true,
        };
        let items = vec![FrontendPackageItem {
            id: "nord-theme".to_string(),
            title: "Nord Theme".to_string(),
            subtitle: "A cold blue theme".to_string(),
            description: "Clean Arctic palette".to_string(),
            version: "1.0.0".to_string(),
            author: author.clone(),
            category: "themes".to_string(),
            package_type: PackageType::Theme,
            tags: vec!["nord".to_string(), "arctic".to_string()],
            supported_desktops: vec!["hyprland".to_string()],
            supported_display: vec!["wayland".to_string()],
            rating: 4.9,
            rating_count: 10,
            downloads: 150,
            hero_image: "".to_string(),
            screenshots: vec![],
            featured: Some(true),
            trending: Some(false),
            color_palette: vec![],
            safety_audit: FrontendSafetyAudit {
                rating: "verified".to_string(),
                changes_system_files: false,
                requires_root: false,
                sandbox_compatible: true,
                files_modified_count: 2,
            },
            dependencies: FrontendDependencies {
                packages: vec![],
                optional: vec![],
            },
            components: vec![],
            compatibility: CompatibilityRequirements::default(),
            manifest: None,
            repository_id: Some("test-repo".to_string()),
            content_hash: None,
            package_size_bytes: Some(1024),
            integrity_status: "verified".to_string(),
            is_cached: true,
            release_channel: Some(ReleaseChannel::Stable),
            trust_tier: Some(TrustTier::Community),
            moderation_status: Some(ModerationStatus::Approved),
            trending_score: None,
            maintainer: None,
            release_notes: None,
            signature: None,
            cryptographic_status: None,
            provider: None,
            targets: None,
            preview_video: None,
            preview_animated: None,
            media_type: None,
            preview_video_url: None,
            preview_poster_url: None,
            source: None,
            provenance: None,
            subfilter: None,
        }];

        let themes = filter_catalog_items(&items, Some("themes"), None, None);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].id, "nord-theme");

        let empty = filter_catalog_items(&items, Some("wallpapers"), None, None);
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_unsupported_schema_rejected() {
        let temp_dir =
            std::env::temp_dir().join(format!("ryzora_test_schema_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let repo_json =
            r#"{"schema": 99, "id": "test-repo", "name": "Future Repo", "packages": []}"#;
        fs::write(temp_dir.join("repository.json"), repo_json).unwrap();

        let res = LocalRepository::load_from_dir(&temp_dir);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Unsupported repository schema 99"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_malformed_index_rejected() {
        let temp_dir =
            std::env::temp_dir().join(format!("ryzora_test_malformed_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        fs::write(temp_dir.join("repository.json"), "NOT_JSON").unwrap();

        let res = LocalRepository::load_from_dir(&temp_dir);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Malformed repository index"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_missing_manifest_rejected() {
        let temp_dir = std::env::temp_dir().join(format!(
            "ryzora_test_missing_manifest_{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&temp_dir);
        let repo_json = r#"{
            "schema": 1,
            "id": "test-repo",
            "name": "Test Repo",
            "packages": [{
                "id": "missing-pkg",
                "name": "Missing Pkg",
                "version": "1.0.0",
                "package_type": "rice",
                "manifest": "packages/missing/manifest.json",
                "category": "rices",
                "author": {"name": "Test", "avatar": "test.png"}
            }]
        }"#;
        fs::write(temp_dir.join("repository.json"), repo_json).unwrap();

        let res = LocalRepository::load_from_dir(&temp_dir);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .contains("Manifest file not found for package 'missing-pkg'"));
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_repository_discovery_and_loading_live_community() {
        let dirs = get_default_repository_search_dirs();
        assert!(!dirs.is_empty(), "Search dirs must not be empty");
        let repos = discover_local_repositories(&dirs);
        assert!(
            !repos.is_empty(),
            "Must discover at least one local repository"
        );
        let community = repos.iter().find(|r| r.id() == "community");
        assert!(community.is_some(), "Must discover 'community' repository");
        let entries = community.unwrap().list_entries().expect("list_entries");
        assert!(
            !entries.is_empty(),
            "Community repository must have package entries"
        );
    }
}
