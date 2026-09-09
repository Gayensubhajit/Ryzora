//! GitHub Content Provider for Ryzora.
//!
//! Provides unauthenticated search, discovery, and declarative packaging of Linux desktop
//! configurations (Hyprland, Waybar, Fastfetch, Kitty, Rofi, Hyprlock, and Wallpapers)
//! hosted on public GitHub repositories.
//!
//! # Security Invariants:
//! 1. Zero subprocess execution (0 `Command::new`, 0 `sudo`, 0 `pkexec`).
//! 2. Strictly unauthenticated public access (no GitHub token required or stored).
//! 3. Pure Rust decompression (`flate2` + `tar`) with strict size limits (50MB compressed, 200MB uncompressed).
//! 4. Archive entries containing symlinks, hardlinks, or parent path traversal (`..`) are strictly rejected.
//! 5. Executable scripts (`install.sh`, `*.sh`, `*.py`, etc.) are detected and stripped.
//! 6. Providers NEVER evaluate trust: all GitHub content is strictly assigned `TrustTier::Community`.
//! 7. Full provenance tracking: repository URL, commit/tag, author, and SPDX license preserved.

use crate::manifest::PackageType;
use crate::providers::synthesizer::ManifestSynthesizer;
use crate::providers::{
    ContentProvider, ProviderCapabilities, ProviderError, ProviderFileSpec, ProviderItem,
    ProviderProvenance, ProviderQuery, ProviderType,
};
use crate::repository::AuthorInfo;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

/// Maximum allowable compressed tarball size (50 MB).
pub const MAX_ARCHIVE_COMPRESSED_BYTES: usize = 50 * 1024 * 1024;

/// Maximum allowable uncompressed archive payload size (200 MB).
pub const MAX_ARCHIVE_UNCOMPRESSED_BYTES: usize = 200 * 1024 * 1024;

/// Maximum allowable API JSON response size (5 MB).
pub const MAX_API_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

/// Maximum entries allowed in an archive (zip-bomb defense).
pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;

/// Default search cache TTL in seconds (15 minutes).
pub const SEARCH_CACHE_TTL_SECS: u64 = 900;

// ─────────────────────────────────────────────────────────────────────────────
// HTTP Transport Abstraction (for Production & Deterministic Testing)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub trait HttpTransport: Send + Sync {
    fn get(
        &self,
        url: &str,
        headers: &[(&str, &str)],
        max_bytes: usize,
    ) -> Result<HttpResponse, ProviderError>;
}

/// Production HTTP transport using pure Rust `ureq`.
pub struct UreqHttpTransport;

impl HttpTransport for UreqHttpTransport {
    fn get(
        &self,
        url: &str,
        custom_headers: &[(&str, &str)],
        max_bytes: usize,
    ) -> Result<HttpResponse, ProviderError> {
        validate_github_url(url)?;

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(12))
            .redirects(0)
            .build();

        let mut current_url = url.to_string();
        let mut redirect_count = 0;
        const MAX_REDIRECTS: usize = 5;

        loop {
            let mut req = agent.get(&current_url);
            req = req.set("User-Agent", "Ryzora-Desktop/0.1.0");
            req = req.set("Accept", "application/vnd.github.v3+json");

            for (k, v) in custom_headers {
                req = req.set(k, v);
            }

            let resp = match req.call() {
                Ok(r) => r,
                Err(ureq::Error::Status(code, r)) => {
                    // Extract rate limit headers on status errors
                    let mut headers_map = HashMap::new();
                    for name in ["x-ratelimit-remaining", "x-ratelimit-reset", "location"] {
                        if let Some(val) = r.header(name) {
                            headers_map.insert(name.to_lowercase(), val.to_string());
                        }
                    }

                    if code == 403 || code == 429 {
                        let remaining = headers_map
                            .get("x-ratelimit-remaining")
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(1);

                        if remaining == 0 || code == 429 {
                            let reset_epoch = headers_map
                                .get("x-ratelimit-reset")
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(0);

                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);

                            let reset_seconds = reset_epoch.saturating_sub(now).max(1);
                            return Err(ProviderError::RateLimitExceeded {
                                reset_seconds,
                                message: format!(
                                    "GitHub unauthenticated API rate limit reached. Resets in {} seconds.",
                                    reset_seconds
                                ),
                            });
                        }
                    }

                    if code == 404 {
                        return Err(ProviderError::NotFound(format!(
                            "GitHub resource not found at '{}'",
                            current_url
                        )));
                    }

                    return Err(ProviderError::NetworkError(format!(
                        "GitHub API returned HTTP status {}: {}",
                        code, current_url
                    )));
                }
                Err(e) => {
                    return Err(ProviderError::NetworkError(format!(
                        "HTTP request failed for '{}': {}",
                        current_url, e
                    )));
                }
            };

            let status = resp.status();
            if (300..=399).contains(&status) {
                if redirect_count >= MAX_REDIRECTS {
                    return Err(ProviderError::NetworkError(format!(
                        "Exceeded maximum redirects ({}) while fetching '{}'",
                        MAX_REDIRECTS, url
                    )));
                }

                let location = resp.header("Location").ok_or_else(|| {
                    ProviderError::NetworkError(format!(
                        "HTTP {} redirect missing Location header from '{}'",
                        status, current_url
                    ))
                })?;

                let next_url = validate_redirect_url(&current_url, location)?;
                current_url = next_url;
                redirect_count += 1;
                continue;
            }

            // Extract response headers
            let mut headers_map = HashMap::new();
            for name in ["x-ratelimit-remaining", "x-ratelimit-reset", "content-type"] {
                if let Some(val) = resp.header(name) {
                    headers_map.insert(name.to_lowercase(), val.to_string());
                }
            }

            // Read response body with strict upper bound
            let mut reader = resp.into_reader().take((max_bytes + 1) as u64);
            let mut body = Vec::new();
            reader.read_to_end(&mut body).map_err(|e| {
                ProviderError::NetworkError(format!("Failed to read response body: {}", e))
            })?;

            if body.len() > max_bytes {
                return Err(ProviderError::SecurityRejected(format!(
                    "Response payload size exceeded maximum allowable limit ({} bytes)",
                    max_bytes
                )));
            }

            return Ok(HttpResponse {
                status,
                headers: headers_map,
                body,
            });
        }
    }
}

/// Validates that a target URL is strictly an approved HTTPS GitHub endpoint.
pub fn validate_github_url(url_str: &str) -> Result<(), ProviderError> {
    if !url_str.starts_with("https://") {
        return Err(ProviderError::SecurityRejected(format!(
            "Forbidden URL scheme in '{}': only HTTPS is permitted",
            url_str
        )));
    }
    if url_str.contains(' ') {
        return Err(ProviderError::SecurityRejected(
            "URL contains null bytes".to_string(),
        ));
    }
    if url_str.contains("..") {
        return Err(ProviderError::SecurityRejected(
            "URL contains parent traversal '..'".to_string(),
        ));
    }

    let parsed = Url::parse(url_str).map_err(|e| {
        ProviderError::SecurityRejected(format!("Malformed HTTPS URL '{}': {}", url_str, e))
    })?;

    if parsed.scheme() != "https" {
        return Err(ProviderError::SecurityRejected(
            "HTTPS scheme required".to_string(),
        ));
    }

    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(ProviderError::SecurityRejected(
            "URL contains forbidden credentials".to_string(),
        ));
    }

    let host = parsed.host_str().unwrap_or("").to_lowercase();
    let allowed_hosts = [
        "api.github.com",
        "github.com",
        "codeload.github.com",
        "raw.githubusercontent.com",
    ];

    if !allowed_hosts.contains(&host.as_str()) {
        return Err(ProviderError::SecurityRejected(format!(
            "Host '{}' is not an authorized GitHub domain",
            host
        )));
    }

    Ok(())
}

/// Validates that an HTTP redirect stays within authorized GitHub domains.
fn validate_redirect_url(current_url: &str, location: &str) -> Result<String, ProviderError> {
    let base = Url::parse(current_url).map_err(|e| {
        ProviderError::SecurityRejected(format!("Invalid base URL '{}': {}", current_url, e))
    })?;
    let target = base.join(location).map_err(|e| {
        ProviderError::SecurityRejected(format!("Invalid redirect Location '{}': {}", location, e))
    })?;

    let target_str = target.to_string();
    validate_github_url(&target_str)?;
    Ok(target_str)
}

// ─────────────────────────────────────────────────────────────────────────────
// GitHub API Models
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSearchResponse {
    pub total_count: u64,
    #[serde(default)]
    pub items: Vec<GitHubRepoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepoItem {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: GitHubOwner,
    pub html_url: String,
    pub description: Option<String>,
    pub stargazers_count: u64,
    pub forks_count: u64,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    pub license: Option<GitHubLicense>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubOwner {
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLicense {
    pub key: Option<String>,
    pub name: Option<String>,
    pub spdx_id: Option<String>,
    pub url: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe Tarball Decompressor
// ─────────────────────────────────────────────────────────────────────────────

/// Safely extracts an in-memory or streamed gzipped tarball archive into `destination`.
///
/// Enforces:
/// - Maximum uncompressed total size cap (200MB) to prevent decompression bombs.
/// - Maximum entry count cap (10,000 entries).
/// - Unconditional rejection of symlinks and hardlinks.
/// - Unconditional rejection of parent traversal (`..`) or absolute paths.
/// - Stripping of GitHub tarball root container directory (`owner-repo-commit/`).
pub fn extract_tarball_safe<R: Read>(
    reader: R,
    destination: &Path,
    max_decompressed_bytes: usize,
) -> Result<(), ProviderError> {
    fs::create_dir_all(destination).map_err(|e| {
        ProviderError::Other(format!(
            "Failed to create target extraction directory: {}",
            e
        ))
    })?;

    let gz = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);

    let mut total_uncompressed: usize = 0;
    let mut entry_count: usize = 0;

    let entries = archive.entries().map_err(|e| {
        ProviderError::InvalidData(format!("Malformed tarball archive stream: {}", e))
    })?;

    for entry_res in entries {
        entry_count += 1;
        if entry_count > MAX_ARCHIVE_ENTRIES {
            return Err(ProviderError::SecurityRejected(format!(
                "Archive exceeds maximum allowed entry count ({})",
                MAX_ARCHIVE_ENTRIES
            )));
        }

        let mut entry = entry_res.map_err(|e| {
            ProviderError::InvalidData(format!("Failed to read tarball entry: {}", e))
        })?;

        let entry_type = entry.header().entry_type();

        // 1. Unconditional rejection of symlinks and hard links
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ProviderError::SecurityRejected(format!(
                "Security violation: Archive contains forbidden symlink or hardlink entry: '{:?}'",
                entry.path().unwrap_or_default()
            )));
        }

        // 2. Reject non-regular, non-directory entries (FIFOs, device files, etc.)
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(ProviderError::SecurityRejected(format!(
                "Security violation: Archive contains forbidden entry type '{:?}'",
                entry_type
            )));
        }

        // 3. Inspect path components for traversal attacks
        let raw_path = entry.path().map_err(|e| {
            ProviderError::InvalidData(format!("Invalid entry path in archive: {}", e))
        })?;

        for comp in raw_path.components() {
            match comp {
                Component::ParentDir => {
                    return Err(ProviderError::SecurityRejected(format!(
                        "Security violation: Archive path contains forbidden parent traversal ('..'): '{:?}'",
                        raw_path
                    )));
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(ProviderError::SecurityRejected(format!(
                        "Security violation: Archive path must be relative: '{:?}'",
                        raw_path
                    )));
                }
                _ => {}
            }
        }

        // 4. GitHub tarballs wrap all contents inside a root folder: `{owner}-{repo}-{ref}/`
        // We strip this single top-level directory component so paths are relative to repo root.
        let mut components = raw_path.components();
        let _root_folder = components.next();
        let rel_path: PathBuf = components.collect();

        if rel_path.as_os_str().is_empty() {
            continue; // Skip root folder itself
        }

        let target_file_path = destination.join(&rel_path);

        // Verification: target path must strictly remain inside destination directory
        if !target_file_path.starts_with(destination) {
            return Err(ProviderError::SecurityRejected(format!(
                "Security violation: Destination path '{:?}' escapes extraction root",
                target_file_path
            )));
        }

        if entry_type.is_dir() {
            fs::create_dir_all(&target_file_path).map_err(|e| {
                ProviderError::Other(format!(
                    "Failed to create directory '{:?}': {}",
                    target_file_path, e
                ))
            })?;
        } else if entry_type.is_file() {
            if let Some(parent) = target_file_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    ProviderError::Other(format!(
                        "Failed to create parent directory '{:?}': {}",
                        parent, e
                    ))
                })?;
            }

            let mut out_file = fs::File::create(&target_file_path).map_err(|e| {
                ProviderError::Other(format!(
                    "Failed to create file '{:?}': {}",
                    target_file_path, e
                ))
            })?;

            let copied = io::copy(&mut entry, &mut out_file).map_err(|e| {
                ProviderError::Other(format!(
                    "Failed to write file '{:?}': {}",
                    target_file_path, e
                ))
            })?;

            total_uncompressed += copied as usize;
            if total_uncompressed > max_decompressed_bytes {
                return Err(ProviderError::SecurityRejected(format!(
                    "Decompressed payload exceeded maximum limit of {} bytes",
                    max_decompressed_bytes
                )));
            }
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Convention-Based Discovery Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Standard tool directories and their target placements in user-space.
const CONVENTION_MAPPINGS: &[(&str, &str)] = &[
    ("hypr/", "~/.config/hypr/"),
    (".config/hypr/", "~/.config/hypr/"),
    ("waybar/", "~/.config/waybar/"),
    (".config/waybar/", "~/.config/waybar/"),
    ("kitty/", "~/.config/kitty/"),
    (".config/kitty/", "~/.config/kitty/"),
    ("fastfetch/", "~/.config/fastfetch/"),
    (".config/fastfetch/", "~/.config/fastfetch/"),
    ("rofi/", "~/.config/rofi/"),
    (".config/rofi/", "~/.config/rofi/"),
    ("hyprlock/", "~/.config/hyprlock/"),
    (".config/hyprlock/", "~/.config/hyprlock/"),
    ("alacritty/", "~/.config/alacritty/"),
    (".config/alacritty/", "~/.config/alacritty/"),
    ("wofi/", "~/.config/wofi/"),
    (".config/wofi/", "~/.config/wofi/"),
    ("mako/", "~/.config/mako/"),
    (".config/mako/", "~/.config/mako/"),
    ("dunst/", "~/.config/dunst/"),
    (".config/dunst/", "~/.config/dunst/"),
    ("wallpapers/", "~/Pictures/Wallpapers/"),
    ("wallpaper/", "~/Pictures/Wallpapers/"),
    ("backgrounds/", "~/Pictures/Wallpapers/"),
];

/// Scans an extracted repository root for convention-based customization files or native manifest.
pub fn discover_conventions(repo_root: &Path) -> Result<Vec<ProviderFileSpec>, String> {
    let mut specs = Vec::new();

    // Priority 1: Check if native manifest.json or ryzora.json exists at root
    for manifest_name in ["ryzora.json", "manifest.json"] {
        let manifest_path = repo_root.join(manifest_name);
        if manifest_path.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) =
                    serde_json::from_str::<crate::manifest::RyzoraManifest>(&content)
                {
                    for f in manifest.files {
                        specs.push(ProviderFileSpec {
                            source: f.source,
                            target: f.target,
                        });
                    }
                    if !specs.is_empty() {
                        return Ok(specs);
                    }
                }
            }
        }
    }

    // Priority 2: Walk directory tree and match against convention directories
    let mut all_files = Vec::new();
    collect_regular_files(repo_root, repo_root, &mut all_files)?;

    for rel_path_str in all_files {
        // Strip out scripts, hidden VCS files, build files, and READMEs
        if is_script_or_metadata(&rel_path_str) {
            continue;
        }

        let mut matched = false;

        // Check folder convention mappings
        for (conv_prefix, target_prefix) in CONVENTION_MAPPINGS {
            if rel_path_str.starts_with(conv_prefix) {
                let subpath = &rel_path_str[conv_prefix.len()..];
                if !subpath.is_empty() {
                    specs.push(ProviderFileSpec {
                        source: rel_path_str.clone(),
                        target: format!("{}{}", target_prefix, subpath),
                    });
                    matched = true;
                    break;
                }
            }
        }

        // Check root wallpaper files
        if !matched {
            let lower = rel_path_str.to_lowercase();
            if is_image_extension(&lower)
                && (lower.starts_with("wallpaper.") || lower.starts_with("background."))
            {
                specs.push(ProviderFileSpec {
                    source: rel_path_str.clone(),
                    target: format!("~/Pictures/Wallpapers/{}", rel_path_str),
                });
            }
        }
    }

    Ok(specs)
}

fn collect_regular_files(current: &Path, root: &Path, acc: &mut Vec<String>) -> Result<(), String> {
    if !current.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" || name == ".github" || name == "node_modules" {
                continue;
            }
            collect_regular_files(&path, root, acc)?;
        } else if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                acc.push(rel_str);
            }
        }
    }

    Ok(())
}

fn is_script_or_metadata(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();

    // Scripts and executables to drop
    if lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".zsh")
        || lower.ends_with(".fish")
        || lower.ends_with(".py")
        || lower.ends_with(".exe")
        || lower.ends_with(".bin")
        || lower.contains("install.sh")
        || lower.contains("setup.sh")
    {
        return true;
    }

    // Common repo documentation/meta files to drop
    if lower == "readme.md"
        || lower == "license"
        || lower == "license.md"
        || lower == "license.txt"
        || lower == "makefile"
        || lower == "cmakelists.txt"
        || lower.starts_with(".github/")
    {
        return true;
    }

    false
}

fn is_image_extension(path_str: &str) -> bool {
    path_str.ends_with(".png")
        || path_str.ends_with(".jpg")
        || path_str.ends_with(".jpeg")
        || path_str.ends_with(".webp")
}

// ─────────────────────────────────────────────────────────────────────────────
// Caching Engine
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct CacheEntry<T> {
    pub cached_at_secs: u64,
    pub ttl_secs: u64,
    pub payload: T,
}

pub struct GitHubCache {
    cache_dir: PathBuf,
}

impl GitHubCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    fn ensure_cache_dir(&self) -> io::Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    pub fn get_search(&self, query_key: &str) -> Option<Vec<ProviderItem>> {
        self.get_entry::<Vec<ProviderItem>>(&format!("search_{}.json", query_key))
    }

    pub fn set_search(&self, query_key: &str, items: &[ProviderItem]) {
        self.set_entry(
            &format!("search_{}.json", query_key),
            items,
            SEARCH_CACHE_TTL_SECS,
        );
    }

    pub fn get_repo(&self, repo_key: &str) -> Option<ProviderItem> {
        self.get_entry::<ProviderItem>(&format!("repo_{}.json", repo_key))
    }

    pub fn set_repo(&self, repo_key: &str, item: &ProviderItem) {
        self.set_entry(
            &format!("repo_{}.json", repo_key),
            item,
            SEARCH_CACHE_TTL_SECS,
        );
    }

    fn get_entry<T: for<'de> Deserialize<'de>>(&self, filename: &str) -> Option<T> {
        let path = self.cache_dir.join(filename);
        if !path.is_file() {
            return None;
        }

        let content = fs::read_to_string(&path).ok()?;
        let entry = serde_json::from_str::<CacheEntry<T>>(&content).ok()?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now.saturating_sub(entry.cached_at_secs) < entry.ttl_secs {
            Some(entry.payload)
        } else {
            let _ = fs::remove_file(&path);
            None
        }
    }

    fn set_entry<T: Serialize + ?Sized>(&self, filename: &str, payload: &T, ttl_secs: u64) {
        if self.ensure_cache_dir().is_err() {
            return;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = CacheEntry {
            cached_at_secs: now,
            ttl_secs,
            payload,
        };

        if let Ok(serialized) = serde_json::to_string_pretty(&entry) {
            let final_path = self.cache_dir.join(filename);
            let tmp_path = self.cache_dir.join(format!("{}.tmp", filename));

            if fs::write(&tmp_path, serialized).is_ok() {
                let _ = fs::rename(&tmp_path, &final_path);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GitHub Provider Implementation
// ─────────────────────────────────────────────────────────────────────────────

/// Unauthenticated GitHub Provider adapter for Ryzora.
pub struct GitHubProvider {
    http: Arc<dyn HttpTransport>,
    cache: GitHubCache,
    enabled: bool,
}

impl GitHubProvider {
    /// Creates a production GitHub provider with the default pure-Rust HTTP transport
    /// and standard Ryzora cache directory.
    pub fn new() -> Self {
        let cache_dir = crate::snapshot::get_home_dir().join(".local/share/ryzora/github_cache");
        Self::with_transport(Arc::new(UreqHttpTransport), cache_dir)
    }

    /// Creates a GitHub provider with custom HTTP transport and cache directory (for testing).
    pub fn with_transport(http: Arc<dyn HttpTransport>, cache_dir: PathBuf) -> Self {
        Self {
            http,
            cache: GitHubCache::new(cache_dir),
            enabled: true,
        }
    }

    /// Sets whether this provider is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for GitHubProvider {
    fn id(&self) -> &str {
        "github"
    }

    fn name(&self) -> &str {
        "GitHub (Public Repositories)"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::GitHub
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            can_search: true,
            can_fetch_item: true,
            can_stage_payload: true,
            supports_categories: true,
            supports_pagination: true,
        }
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn search(&self, query: &ProviderQuery) -> Result<Vec<ProviderItem>, ProviderError> {
        let query_key = compute_query_hash(query);

        // 1. Check cache first to avoid consuming unauthenticated rate limit
        if let Some(cached_items) = self.cache.get_search(&query_key) {
            return Ok(cached_items);
        }

        // 2. Build GitHub Search API query
        let search_url = build_github_search_url(query);

        // 3. Execute HTTP request
        let resp = self.http.get(&search_url, &[], MAX_API_RESPONSE_BYTES)?;

        // 4. Parse search response
        let search_data: GitHubSearchResponse =
            serde_json::from_slice(&resp.body).map_err(|e| {
                ProviderError::InvalidData(format!("Failed to parse GitHub search JSON: {}", e))
            })?;

        // 5. Convert repositories into ProviderItems
        let items: Vec<ProviderItem> = search_data
            .items
            .iter()
            .map(repo_to_provider_item)
            .collect();

        // 6. Cache valid items
        self.cache.set_search(&query_key, &items);

        Ok(items)
    }

    fn fetch_item(&self, id: &str) -> Result<ProviderItem, ProviderError> {
        let clean_id = id.trim().to_lowercase();

        // Check cache first
        if let Some(cached) = self.cache.get_repo(&clean_id) {
            return Ok(cached);
        }

        // Parse owner and repo from ID (e.g. "gh-alice-hyprland-dots")
        let (owner, repo_name) = parse_github_id(&clean_id)?;

        let api_url = format!("https://api.github.com/repos/{}/{}", owner, repo_name);
        let resp = self.http.get(&api_url, &[], MAX_API_RESPONSE_BYTES)?;

        let repo_data: GitHubRepoItem = serde_json::from_slice(&resp.body).map_err(|e| {
            ProviderError::InvalidData(format!("Failed to parse GitHub repo JSON: {}", e))
        })?;

        let item = repo_to_provider_item(&repo_data);
        self.cache.set_repo(&clean_id, &item);

        Ok(item)
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        let repo_full = item
            .provenance
            .repository
            .as_deref()
            .ok_or_else(|| "GitHub item missing repository provenance".to_string())?;

        let target_ref = item.provenance.commit_or_tag.as_deref().unwrap_or("main");

        let tarball_url = format!(
            "https://api.github.com/repos/{}/tarball/{}",
            repo_full, target_ref
        );

        // Fetch tarball archive
        let resp = self
            .http
            .get(&tarball_url, &[], MAX_ARCHIVE_COMPRESSED_BYTES)
            .map_err(|e| format!("Failed to download GitHub repository tarball: {}", e))?;

        // Create isolated temporary extraction folder
        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(12345);

        let tmp_extract_dir = staging_dir.join(format!(".tmp_gh_{}_{}", clean_id, rand_suffix));
        let final_staged_dir = staging_dir.join(&clean_id);

        if tmp_extract_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_extract_dir);
        }
        if final_staged_dir.exists() {
            let _ = fs::remove_dir_all(&final_staged_dir);
        }

        // Safely extract tarball
        extract_tarball_safe(
            io::Cursor::new(&resp.body),
            &tmp_extract_dir,
            MAX_ARCHIVE_UNCOMPRESSED_BYTES,
        )
        .map_err(|e| format!("Safe tarball extraction failed: {}", e))?;

        // Discover convention files or read native manifest
        let discovered_files = discover_conventions(&tmp_extract_dir)
            .map_err(|e| format!("Convention discovery failed: {}", e))?;

        if discovered_files.is_empty() {
            let _ = fs::remove_dir_all(&tmp_extract_dir);
            return Err(format!(
                "Repository '{}' contains no recognized Linux desktop customizations (hypr, waybar, kitty, fastfetch, rofi, hyprlock, wallpapers)",
                repo_full
            ));
        }

        // Build updated item with discovered files
        let mut updated_item = item.clone();
        updated_item.files = discovered_files;

        // Synthesize authoritative RyzoraManifest
        let manifest = ManifestSynthesizer::synthesize(&updated_item)
            .map_err(|e| format!("Manifest synthesis failed: {}", e))?;

        // Write synthesized manifest to staged directory
        let manifest_path = tmp_extract_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize synthesized manifest: {}", e))?;
        fs::write(&manifest_path, manifest_json)
            .map_err(|e| format!("Failed to write manifest.json to staging: {}", e))?;

        // Atomically rename temporary staging dir to final staged dir
        fs::rename(&tmp_extract_dir, &final_staged_dir)
            .map_err(|e| format!("Failed to finalize staged payload: {}", e))?;

        Ok(final_staged_dir)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

pub fn generate_github_item_id(owner: &str, repo: &str) -> String {
    let raw = format!("gh-{}-{}", owner, repo);
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn parse_github_id(id: &str) -> Result<(String, String), ProviderError> {
    let clean = id.trim_start_matches("gh-");
    let parts: Vec<&str> = clean.splitn(2, '-').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(ProviderError::InvalidData(format!(
            "Invalid GitHub provider item ID '{}': expected 'gh-<owner>-<repo>'",
            id
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn compute_query_hash(query: &ProviderQuery) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.query.trim().to_lowercase().as_bytes());
    if let Some(ref cat) = query.category {
        hasher.update(cat.as_bytes());
    }
    if let Some(ref dt) = query.desktop {
        hasher.update(dt.as_bytes());
    }
    hasher.update(query.page.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

fn build_github_search_url(query: &ProviderQuery) -> String {
    let mut q_parts = Vec::new();

    let user_q = query.query.trim();
    if !user_q.is_empty() {
        q_parts.push(user_q.to_string());
    } else {
        q_parts.push("hyprland OR dotfiles OR waybar".to_string());
    }

    if let Some(ref cat) = query.category {
        match cat.as_str() {
            "wallpaper" => q_parts.push("topic:wallpaper OR topic:wallpapers".to_string()),
            "panel_bar" => q_parts.push("topic:waybar OR waybar".to_string()),
            "terminal" => q_parts.push("topic:kitty OR topic:alacritty".to_string()),
            "app_config" => q_parts.push("topic:fastfetch OR fastfetch".to_string()),
            "launcher" => q_parts.push("topic:rofi OR topic:wofi".to_string()),
            "wm_rice" => q_parts.push("topic:hyprland-rice OR topic:dotfiles".to_string()),
            _ => {}
        }
    }

    let encoded_q = percent_encoding::utf8_percent_encode(
        &q_parts.join(" "),
        percent_encoding::NON_ALPHANUMERIC,
    )
    .to_string();

    let per_page = if query.page_size > 0 && query.page_size <= 50 {
        query.page_size
    } else {
        30
    };

    let page = if query.page > 0 { query.page } else { 1 };

    format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc&per_page={}&page={}",
        encoded_q, per_page, page
    )
}

pub fn repo_to_provider_item(repo: &GitHubRepoItem) -> ProviderItem {
    let id = generate_github_item_id(&repo.owner.login, &repo.name);
    let title = format_repo_title(&repo.name);
    let subtitle = format!("By @{} on GitHub", repo.owner.login);
    let description = repo.description.clone().unwrap_or_else(|| {
        format!(
            "Linux desktop customization repository by {}",
            repo.owner.login
        )
    });
    let version = repo
        .default_branch
        .clone()
        .unwrap_or_else(|| "main".to_string());

    let category = detect_repo_category(&repo.name, &repo.description, &repo.topics);
    let package_type = detect_package_type(&category);
    let supported_desktops = detect_desktops(&repo.name, &repo.description, &repo.topics);

    let mut tags = repo.topics.clone();
    if !tags.contains(&"github".to_string()) {
        tags.push("github".to_string());
    }
    if !tags.contains(&"dotfiles".to_string()) {
        tags.push("dotfiles".to_string());
    }

    let provenance = ProviderProvenance {
        provider_id: "github".to_string(),
        source_url: repo.html_url.clone(),
        repository: Some(repo.full_name.clone()),
        commit_or_tag: Some(version.clone()),
        license_spdx: repo.license.as_ref().and_then(|l| l.spdx_id.clone()),
        original_author: repo.owner.login.clone(),
        fetched_at: current_utc_iso(),
    };

    ProviderItem {
        id,
        title,
        subtitle,
        description,
        version,
        author: AuthorInfo {
            name: repo.owner.login.clone(),
            avatar: repo.owner.avatar_url.clone().unwrap_or_default(),
            verified: false, // Strictly unverified
        },
        category,
        package_type,
        tags,
        supported_desktops,
        supported_display: vec!["Wayland".to_string(), "X11".to_string()],
        rating: None, // Never infer official rating from GitHub stars
        rating_count: Some(repo.stargazers_count),
        downloads: Some(repo.forks_count),
        hero_image: None,
        screenshots: Vec::new(),
        provenance,
        files: Vec::new(), // Populated during staging
    }
}

fn format_repo_title(repo_name: &str) -> String {
    let clean = repo_name.replace(['-', '_'], " ").trim().to_string();

    clean
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

fn detect_repo_category(name: &str, description: &Option<String>, topics: &[String]) -> String {
    let lower_name = name.to_lowercase();
    let lower_desc = description.as_deref().unwrap_or("").to_lowercase();

    let all_text = format!("{} {} {}", lower_name, lower_desc, topics.join(" "));

    if all_text.contains("wallpaper") || all_text.contains("background") {
        "wallpaper".to_string()
    } else if all_text.contains("rice")
        || all_text.contains("dotfiles")
        || all_text.contains("hyprland")
    {
        "wm_rice".to_string()
    } else if all_text.contains("waybar") {
        "panel_bar".to_string()
    } else if all_text.contains("fastfetch") {
        "app_config".to_string()
    } else if all_text.contains("kitty") || all_text.contains("alacritty") {
        "terminal".to_string()
    } else if all_text.contains("rofi") || all_text.contains("wofi") {
        "launcher".to_string()
    } else {
        "wm_rice".to_string()
    }
}

fn detect_package_type(category: &str) -> PackageType {
    match category {
        "wallpaper" => PackageType::Wallpaper,
        "wm_rice" => PackageType::Rice,
        "panel_bar" => PackageType::Theme,
        _ => PackageType::Theme,
    }
}

fn detect_desktops(name: &str, description: &Option<String>, topics: &[String]) -> Vec<String> {
    let mut desktops = Vec::new();
    let lower = format!(
        "{} {} {}",
        name.to_lowercase(),
        description.as_deref().unwrap_or("").to_lowercase(),
        topics.join(" ")
    );

    if lower.contains("hyprland") || lower.contains("hypr") {
        desktops.push("Hyprland".to_string());
    }
    if lower.contains("sway") {
        desktops.push("Sway".to_string());
    }
    if lower.contains("kde") || lower.contains("plasma") {
        desktops.push("KDE Plasma".to_string());
    }
    if lower.contains("gnome") {
        desktops.push("GNOME".to_string());
    }

    if desktops.is_empty() {
        desktops.push("Hyprland".to_string());
    }

    desktops
}

fn current_utc_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", now)
}
