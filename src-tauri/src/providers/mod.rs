pub mod community;
pub mod desktop;
pub mod fastfetch;
pub mod github;
pub mod normalizer;
pub mod rice;
pub mod synthesizer;
pub mod wallpaper;

use crate::manifest::PackageType;
use crate::repository::{AuthorInfo, FrontendPackageItem};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Core Provider Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Classification of content providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Community,
    GitHub,
    Fastfetch,
    Rice,
    Wallpaper,
    DesktopEnvironment,
    Custom(String),
}

/// Declared operational capabilities of a content provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub can_search: bool,
    pub can_fetch_item: bool,
    pub can_stage_payload: bool,
    pub supports_categories: bool,
    pub supports_pagination: bool,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            can_search: true,
            can_fetch_item: true,
            can_stage_payload: true,
            supports_categories: true,
            supports_pagination: false,
        }
    }
}

/// Canonical query structure for searching across providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderQuery {
    pub query: String,
    pub category: Option<String>,
    pub desktop: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

/// Mandatory provenance metadata tracing content back to its source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProvenance {
    pub provider_id: String,
    pub source_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_or_tag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_spdx: Option<String>,
    pub original_author: String,
    pub fetched_at: String,
}

/// A declarative file specification supplied by a provider item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderFileSpec {
    pub source: String,
    pub target: String,
}

/// Intermediate item supplied by an external or internal content provider.
/// Note: Provider items NEVER bypass security; they must be normalized and
/// synthesized into an authoritative RyzoraManifest before installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderItem {
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
    pub rating: Option<f64>,
    pub rating_count: Option<u64>,
    pub downloads: Option<u64>,
    pub hero_image: Option<String>,
    pub screenshots: Vec<String>,
    pub provenance: ProviderProvenance,
    pub files: Vec<ProviderFileSpec>,
}

/// Provider operational status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Online,
    Offline,
    RateLimited { reset_seconds: u64 },
    Disabled,
    Error(String),
}

/// Summary metadata of a registered content provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub enabled: bool,
    pub status: ProviderStatus,
    pub capabilities: ProviderCapabilities,
}

/// Typed provider error hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderError {
    NetworkError(String),
    RateLimitExceeded { reset_seconds: u64, message: String },
    InvalidData(String),
    SecurityRejected(String),
    NotFound(String),
    Disabled(String),
    Other(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::RateLimitExceeded {
                reset_seconds,
                message,
            } => {
                write!(
                    f,
                    "Rate limit exceeded (resets in {}s): {}",
                    reset_seconds, message
                )
            }
            Self::InvalidData(msg) => write!(f, "Invalid provider data: {}", msg),
            Self::SecurityRejected(msg) => write!(f, "Security rejection: {}", msg),
            Self::NotFound(msg) => write!(f, "Item not found: {}", msg),
            Self::Disabled(msg) => write!(f, "Provider disabled: {}", msg),
            Self::Other(msg) => write!(f, "Provider error: {}", msg),
        }
    }
}

impl std::error::Error for ProviderError {}

// ─────────────────────────────────────────────────────────────────────────────
// ContentProvider Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Authoritative contract for external and internal content ecosystem adapters.
/// Providers only supply raw items and metadata; they do NOT evaluate trust or
/// execute installations directly.
pub trait ContentProvider: Send + Sync {
    /// Unique provider identifier (e.g. "community", "github", "fastfetch")
    fn id(&self) -> &str;

    /// User-facing display name
    fn name(&self) -> &str;

    /// Specific category / type of provider
    fn provider_type(&self) -> ProviderType;

    /// Capabilities exposed by this provider
    fn capabilities(&self) -> ProviderCapabilities;

    /// Whether this provider is currently active
    fn is_enabled(&self) -> bool;

    /// Search the provider with normalized query parameters
    fn search(&self, query: &ProviderQuery) -> Result<Vec<ProviderItem>, ProviderError>;

    /// Fetch a single specific item with complete metadata
    fn fetch_item(&self, id: &str) -> Result<ProviderItem, ProviderError>;

    /// Download and stage the package payload into an isolated directory
    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Search Aggregator Response
// ─────────────────────────────────────────────────────────────────────────────

/// Result of an aggregated search across multiple content providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSearchResponse {
    pub items: Vec<FrontendPackageItem>,
    pub total_results: usize,
    pub provider_statuses: HashMap<String, ProviderStatus>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Aggregator Engine (ProviderManager)
// ─────────────────────────────────────────────────────────────────────────────

/// Central manager orchestrating queries and deduplication across content providers.
pub struct ProviderManager {
    providers: Vec<Arc<dyn ContentProvider>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers a content provider.
    pub fn register_provider(&mut self, provider: Arc<dyn ContentProvider>) {
        // Replace existing provider with matching ID if present
        let id = provider.id().to_string();
        self.providers.retain(|p| p.id() != id);
        self.providers.push(provider);
    }

    /// Lists summaries of all registered providers.
    pub fn list_providers(&self) -> Vec<ProviderSummary> {
        self.providers
            .iter()
            .map(|p| ProviderSummary {
                id: p.id().to_string(),
                name: p.name().to_string(),
                provider_type: p.provider_type(),
                enabled: p.is_enabled(),
                status: if p.is_enabled() {
                    ProviderStatus::Online
                } else {
                    ProviderStatus::Disabled
                },
                capabilities: p.capabilities(),
            })
            .collect()
    }

    /// Queries all enabled providers, captures partial failures without failing the search,
    /// normalizes items into FrontendPackageItem, and deduplicates by canonical source identity.

    /// Finds and retrieves a raw ProviderItem along with its handling provider.
    pub fn fetch_raw_item(
        &self,
        package_id: &str,
    ) -> Result<(Arc<dyn ContentProvider>, ProviderItem), String> {
        for provider in &self.providers {
            if !provider.is_enabled() {
                continue;
            }
            if let Ok(item) = provider.fetch_item(package_id) {
                return Ok((provider.clone(), item));
            }
        }
        Err(format!(
            "Provider package '{}' not found in any registered provider",
            package_id
        ))
    }

    /// Stages a provider package into the specified directory, synthesizes its canonical
    /// RyzoraManifest, validates it, and writes `manifest.json`.
    pub fn stage_provider_package(
        &self,
        package_id: &str,
        dest_parent_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let (provider, item) = self.fetch_raw_item(package_id)?;

        std::fs::create_dir_all(dest_parent_dir)
            .map_err(|e| format!("Failed to create destination dir: {}", e))?;

        // Stage payload: returns the created package directory `dest_parent_dir.join(clean_id)`
        let staged_dir = provider.stage_payload(&item, dest_parent_dir)?;

        // Synthesize authoritative manifest
        let manifest = synthesizer::ManifestSynthesizer::synthesize(&item)?;

        // Serialize and write manifest.json into the staged package directory
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Failed to serialize synthesized manifest: {}", e))?;
        std::fs::write(staged_dir.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Failed to write synthesized manifest.json: {}", e))?;

        Ok(staged_dir)
    }

    /// Prepares a provider package in a target packages root directory (e.g. `~/.local/share/ryzora/packages/<pkg_id>`).
    pub fn prepare_provider_package_in(
        &self,
        package_id: &str,
        base_packages_dir: &std::path::Path,
    ) -> Result<std::path::PathBuf, String> {
        let clean_id = package_id.trim().to_lowercase();
        let target_dir = base_packages_dir.join(&clean_id);
        if target_dir.join("manifest.json").is_file() {
            return Ok(target_dir);
        }
        self.stage_provider_package(&clean_id, base_packages_dir)
    }

    pub fn search_all(&self, query: &ProviderQuery) -> ProviderSearchResponse {
        let mut raw_items: Vec<ProviderItem> = Vec::new();
        let mut statuses: HashMap<String, ProviderStatus> = HashMap::new();

        for provider in &self.providers {
            let pid = provider.id().to_string();
            if !provider.is_enabled() {
                statuses.insert(pid, ProviderStatus::Disabled);
                continue;
            }

            match provider.search(query) {
                Ok(items) => {
                    statuses.insert(pid, ProviderStatus::Online);
                    raw_items.extend(items);
                }
                Err(ProviderError::RateLimitExceeded {
                    reset_seconds,
                    message: _,
                }) => {
                    statuses.insert(pid, ProviderStatus::RateLimited { reset_seconds });
                }
                Err(ProviderError::Disabled(_msg)) => {
                    statuses.insert(pid, ProviderStatus::Disabled);
                }
                Err(e) => {
                    statuses.insert(pid, ProviderStatus::Error(e.to_string()));
                }
            }
        }

        // Deduplicate items across providers:
        // Key is normalized_id (with priority given to earlier providers)
        let mut seen_ids = std::collections::HashSet::new();
        let mut deduplicated_items = Vec::new();

        for item in raw_items {
            let norm_id = item.id.trim().to_lowercase();
            if seen_ids.insert(norm_id) {
                deduplicated_items.push(item);
            }
        }

        // Apply desktop environment filtering if specified
        if let Some(ref dt) = query.desktop {
            let dt_lower = dt.trim().to_lowercase();
            if !dt_lower.is_empty() && dt_lower != "all" {
                deduplicated_items.retain(|item| {
                    item.supported_desktops.iter().any(|d| {
                        let dl = d.trim().to_lowercase();
                        dl == "universal" || dl == "all" || dl == dt_lower
                    })
                });
            }
        }

        // Normalize provider items to FrontendPackageItem for UI consumption
        let normalized: Vec<FrontendPackageItem> = deduplicated_items
            .into_iter()
            .map(normalizer::normalize_provider_item)
            .collect();

        let total = normalized.len();
        ProviderSearchResponse {
            items: normalized,
            total_results: total,
            provider_statuses: statuses,
        }
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Global Provider State & Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

use std::sync::{Mutex, OnceLock};

static GLOBAL_PROVIDER_MANAGER: OnceLock<Mutex<ProviderManager>> = OnceLock::new();

pub fn get_global_provider_manager() -> &'static Mutex<ProviderManager> {
    GLOBAL_PROVIDER_MANAGER.get_or_init(|| Mutex::new(create_default_provider_manager()))
}

pub fn create_default_provider_manager() -> ProviderManager {
    let mut mgr = ProviderManager::new();
    mgr.register_provider(Arc::new(community::CommunityProvider::new()));
    mgr.register_provider(Arc::new(github::GitHubProvider::new()));
    mgr.register_provider(Arc::new(fastfetch::FastfetchProvider::new()));
    mgr.register_provider(Arc::new(rice::RiceProvider::new()));
    mgr.register_provider(Arc::new(wallpaper::WallpaperProvider::new()));
    mgr.register_provider(Arc::new(desktop::KdeProvider::new()));
    mgr.register_provider(Arc::new(desktop::GnomeProvider::new()));
    mgr
}

#[tauri::command]
pub fn list_content_providers() -> Result<Vec<ProviderSummary>, String> {
    let mgr = get_global_provider_manager()
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(mgr.list_providers())
}

#[tauri::command]
pub fn search_content_providers(query: ProviderQuery) -> Result<ProviderSearchResponse, String> {
    let mgr = get_global_provider_manager()
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(mgr.search_all(&query))
}

#[tauri::command]
pub fn synthesize_provider_manifest(
    item: ProviderItem,
) -> Result<crate::manifest::RyzoraManifest, String> {
    synthesizer::ManifestSynthesizer::synthesize(&item)
}

/// Explicitly stages and prepares a provider package into the application data directory
/// `~/.local/share/ryzora/packages/<package_id>` so it can be resolved by the standard installer.
pub fn prepare_provider_package_dir(package_id: &str) -> Result<std::path::PathBuf, String> {
    let packages_dir = crate::installer::get_ryzora_base_dir().join("packages");
    let mgr = get_global_provider_manager()
        .lock()
        .map_err(|e| e.to_string())?;
    mgr.prepare_provider_package_in(package_id, &packages_dir)
}

#[tauri::command]
pub fn prepare_provider_package(package_id: String) -> Result<String, String> {
    let path = prepare_provider_package_dir(&package_id)?;
    Ok(path.to_string_lossy().to_string())
}
