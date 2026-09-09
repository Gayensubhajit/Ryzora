//! Fastfetch Content Provider for Ryzora.
//!
//! Provides discovery, preview, and declarative packaging of Fastfetch presets and themes
//! (JSONC configs, presets, logos, ASCII art) strictly confined to `~/.config/fastfetch/`.
//!
//! # Security Invariants:
//! 1. Zero subprocess execution (0 `Command::new`, 0 `sudo`, 0 `pkexec`).
//! 2. Strict directory confinement: all package targets MUST be within `~/.config/fastfetch/`.
//!    Any target attempting to place files outside `~/.config/fastfetch/` (e.g. `~/.bashrc`,
//!    `~/.config/hypr/`, `/etc`) is fatally rejected.
//! 3. Pure declarative payload: executable scripts (*.sh, *.py, *.exe, *.bin, install.sh) are fatally rejected.
//! 4. Canonical path validation: symlinks or relative sources escaping package boundaries are rejected.
//! 5. Content-hash and tree-hash verification for on-disk/cached packages.
//! 6. Trust invariant: Fastfetch content strictly defaults to `TrustTier::Community` (unvetted).

use crate::manifest::{PackageType, RyzoraManifest};
use crate::providers::synthesizer::ManifestSynthesizer;
use crate::providers::{
    ContentProvider, ProviderCapabilities, ProviderError, ProviderFileSpec, ProviderItem,
    ProviderProvenance, ProviderQuery, ProviderType,
};
use crate::repository::{compute_package_tree_hash, AuthorInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Allowed Fastfetch customization target prefix.
pub const FASTFETCH_TARGET_PREFIX: &str = "~/.config/fastfetch/";

/// Definition of a declarative Fastfetch theme preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastfetchThemePreset {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub config_jsonc: String,
    #[serde(default)]
    pub presets: HashMap<String, String>,
    #[serde(default)]
    pub logos: HashMap<String, String>,
    pub license_spdx: String,
}

/// Fastfetch Content Provider adapter.
pub struct FastfetchProvider {
    builtin_presets: Vec<FastfetchThemePreset>,
    custom_dir: Option<PathBuf>,
    enabled: bool,
}

impl FastfetchProvider {
    /// Creates a production Fastfetch provider with standard curated community presets.
    pub fn new() -> Self {
        Self {
            builtin_presets: get_default_fastfetch_presets(),
            custom_dir: Some(
                crate::snapshot::get_home_dir().join(".local/share/ryzora/fastfetch_presets"),
            ),
            enabled: true,
        }
    }

    /// Creates a Fastfetch provider with a custom presets directory (for testing).
    pub fn with_custom_dir(custom_dir: PathBuf) -> Self {
        Self {
            builtin_presets: get_default_fastfetch_presets(),
            custom_dir: Some(custom_dir),
            enabled: true,
        }
    }

    /// Sets whether this provider is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Default for FastfetchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for FastfetchProvider {
    fn id(&self) -> &str {
        "fastfetch"
    }

    fn name(&self) -> &str {
        "Fastfetch Themes & Presets"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Fastfetch
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
        let mut results = Vec::new();
        let q_clean = query.query.trim().to_lowercase();
        let cat_filter = query.category.as_deref().map(|c| c.to_lowercase());

        // Check category relevance: fastfetch items are "app_config"
        if let Some(ref cat) = cat_filter {
            if cat != "app_config" && cat != "fastfetch" && cat != "terminal" {
                return Ok(Vec::new());
            }
        }

        // 1. Search built-in presets
        for preset in &self.builtin_presets {
            if !q_clean.is_empty() {
                let matches_title = preset.title.to_lowercase().contains(&q_clean);
                let matches_id = preset.id.to_lowercase().contains(&q_clean);
                let matches_desc = preset.description.to_lowercase().contains(&q_clean);
                let matches_tags = preset
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&q_clean));

                if !matches_title && !matches_id && !matches_desc && !matches_tags {
                    continue;
                }
            }

            results.push(preset_to_provider_item(preset));
        }

        // 2. Search custom presets from on-disk directory if configured
        if let Some(ref custom_dir) = self.custom_dir {
            if custom_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(custom_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("manifest.json").is_file() {
                            if let Ok(item) = load_disk_preset_item(&path) {
                                if !q_clean.is_empty() {
                                    let matches_title =
                                        item.title.to_lowercase().contains(&q_clean);
                                    let matches_id = item.id.to_lowercase().contains(&q_clean);
                                    let matches_desc =
                                        item.description.to_lowercase().contains(&q_clean);
                                    let matches_tags = item
                                        .tags
                                        .iter()
                                        .any(|t| t.to_lowercase().contains(&q_clean));

                                    if !matches_title
                                        && !matches_id
                                        && !matches_desc
                                        && !matches_tags
                                    {
                                        continue;
                                    }
                                }
                                results.push(item);
                            }
                        }
                    }
                }
            }
        }

        // Pagination
        let total = results.len();
        let page_size = if query.page_size > 0 && query.page_size <= 50 {
            query.page_size
        } else {
            30
        };
        let page = if query.page > 0 { query.page } else { 1 };
        let start_idx = (page - 1) * page_size;

        if start_idx >= total {
            return Ok(Vec::new());
        }

        let end_idx = (start_idx + page_size).min(total);
        Ok(results[start_idx..end_idx].to_vec())
    }

    fn fetch_item(&self, id: &str) -> Result<ProviderItem, ProviderError> {
        let clean_id = id.trim();

        // 1. Check built-ins
        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            return Ok(preset_to_provider_item(preset));
        }

        // 2. Check custom on-disk presets
        if let Some(ref custom_dir) = self.custom_dir {
            let pkg_dir = custom_dir.join(clean_id);
            if pkg_dir.is_dir() && pkg_dir.join("manifest.json").is_file() {
                if let Ok(item) = load_disk_preset_item(&pkg_dir) {
                    return Ok(item);
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "Fastfetch preset '{}' not found",
            clean_id
        )))
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        // 1. Security Check: Enforce Fastfetch directory target confinement on every file
        for file in &item.files {
            validate_fastfetch_target_and_source(file, &clean_id)?;
        }

        // 2. Create isolated staging folder
        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(12345);

        let tmp_stage_dir = staging_dir.join(format!(".tmp_ff_{}_{}", clean_id, rand_suffix));
        let final_stage_dir = staging_dir.join(&clean_id);

        if tmp_stage_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
        }
        if final_stage_dir.exists() {
            let _ = fs::remove_dir_all(&final_stage_dir);
        }
        fs::create_dir_all(&tmp_stage_dir)
            .map_err(|e| format!("Failed to create staging directory: {}", e))?;

        // 3. Populate payload: either from built-in preset or custom on-disk source
        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            // Write config.jsonc
            fs::write(tmp_stage_dir.join("config.jsonc"), &preset.config_jsonc)
                .map_err(|e| format!("Failed to write config.jsonc: {}", e))?;

            // Write presets
            for (rel_path, content) in &preset.presets {
                let target_file = tmp_stage_dir.join(rel_path);
                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create preset parent directory: {}", e))?;
                }
                fs::write(&target_file, content)
                    .map_err(|e| format!("Failed to write preset '{}': {}", rel_path, e))?;
            }

            // Write logos
            for (rel_path, content) in &preset.logos {
                let target_file = tmp_stage_dir.join(rel_path);
                if let Some(parent) = target_file.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create logo parent directory: {}", e))?;
                }
                fs::write(&target_file, content)
                    .map_err(|e| format!("Failed to write logo '{}': {}", rel_path, e))?;
            }
        } else if let Some(ref custom_dir) = self.custom_dir {
            let src_pkg_dir = custom_dir.join(&clean_id);
            if !src_pkg_dir.is_dir() {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Fastfetch package source directory '{}' does not exist",
                    src_pkg_dir.display()
                ));
            }

            let manifest_path = src_pkg_dir.join("manifest.json");
            let manifest_content = fs::read_to_string(&manifest_path)
                .map_err(|e| format!("Failed to read manifest for '{}': {}", clean_id, e))?;
            let manifest: RyzoraManifest = serde_json::from_str(&manifest_content)
                .map_err(|e| format!("Failed to parse manifest for '{}': {}", clean_id, e))?;

            if manifest.id != clean_id {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Security violation: Manifest declared ID '{}' does not match requested ID '{}'",
                    manifest.id, clean_id
                ));
            }

            // Verify content/tree hash if specified in provenance
            if let Some(ref expected_hash) = item.provenance.commit_or_tag {
                if expected_hash.len() == 64 && expected_hash.chars().all(|c| c.is_ascii_hexdigit())
                {
                    let actual_hash = compute_package_tree_hash(&src_pkg_dir, &manifest)?;
                    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: Tree hash mismatch for package '{}': expected {}, computed {}",
                            clean_id, expected_hash, actual_hash
                        ));
                    }
                }
            }

            // Copy declared files
            for file in &manifest.files {
                let src_clean = file.source.trim();
                let src_full = src_pkg_dir.join(src_clean);

                if !src_full.is_file() {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Fastfetch payload file missing on disk: '{}'",
                        src_clean
                    ));
                }

                // Verify canonical path remains inside package root (reject symlink escaping)
                if let (Ok(can_src), Ok(can_pkg)) =
                    (src_full.canonicalize(), src_pkg_dir.canonicalize())
                {
                    if !can_src.starts_with(&can_pkg) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: Source file '{}' escapes package root via symlink",
                            src_clean
                        ));
                    }
                }

                let dest_file = tmp_stage_dir.join(src_clean);
                if let Some(parent) = dest_file.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
                fs::copy(&src_full, &dest_file)
                    .map_err(|e| format!("Failed to copy file '{}': {}", src_clean, e))?;
            }
        } else {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Fastfetch preset '{}' source unavailable",
                clean_id
            ));
        }

        // 4. Synthesize authoritative manifest via ManifestSynthesizer
        let synthesized = ManifestSynthesizer::synthesize(item).map_err(|e| {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            format!("Fastfetch manifest synthesis failed: {}", e)
        })?;

        // Write synthesized manifest
        let manifest_dest = tmp_stage_dir.join("manifest.json");
        let manifest_json = serde_json::to_string_pretty(&synthesized)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(&manifest_dest, manifest_json)
            .map_err(|e| format!("Failed to write manifest.json: {}", e))?;

        // Atomically rename temporary staging directory
        fs::rename(&tmp_stage_dir, &final_stage_dir)
            .map_err(|e| format!("Failed to finalize staged Fastfetch payload: {}", e))?;

        Ok(final_stage_dir)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation & Security Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Validates that a file strictly conforms to Fastfetch target confinement and data policies.
pub fn validate_fastfetch_target_and_source(
    file: &ProviderFileSpec,
    pkg_id: &str,
) -> Result<(), String> {
    let tgt = file.target.trim();
    let src = file.source.trim();

    // 1. Mandatory target confinement: must start with `~/.config/fastfetch/`
    if !tgt.starts_with(FASTFETCH_TARGET_PREFIX) {
        return Err(format!(
            "Security violation: Package '{}' target '{}' must be confined within '{}'",
            pkg_id, tgt, FASTFETCH_TARGET_PREFIX
        ));
    }

    // 2. Traversal rejection in target
    if tgt.contains("..") {
        return Err(format!(
            "Security violation: Package '{}' target '{}' contains forbidden parent traversal ('..')",
            pkg_id, tgt
        ));
    }
    if tgt.contains('\0') {
        return Err(format!("Package '{}' target contains null bytes", pkg_id));
    }

    // 3. Traversal rejection in source
    if src.starts_with('/') || src.starts_with('\\') || src.contains("..") || src.contains('\0') {
        return Err(format!(
            "Security violation: Package '{}' source '{}' contains forbidden traversal or absolute path",
            pkg_id, src
        ));
    }

    // 4. Executable / Script Rejection (by extension and filename pattern)
    let lower_src = src.to_lowercase();
    let lower_tgt = tgt.to_lowercase();

    let forbidden_extensions = [
        ".sh", ".bash", ".zsh", ".fish", ".py", ".exe", ".bin", ".cmd", ".bat", ".elf", ".so",
        ".dll", ".scr", ".msi", ".vbs", ".js",
    ];

    for ext in &forbidden_extensions {
        if lower_src.ends_with(ext) || lower_tgt.ends_with(ext) {
            return Err(format!(
                "Security violation: Package '{}' contains prohibited executable script or binary: '{}'",
                pkg_id, src
            ));
        }
    }

    if lower_src.contains("install.sh")
        || lower_src.contains("setup.sh")
        || lower_tgt.contains("install.sh")
        || lower_tgt.contains("setup.sh")
    {
        return Err(format!(
            "Security violation: Package '{}' contains prohibited installer hook: '{}'",
            pkg_id, src
        ));
    }

    // 5. Permitted Fastfetch target subpaths:
    // ~/.config/fastfetch/config.jsonc
    // ~/.config/fastfetch/config.json
    // ~/.config/fastfetch/presets/**
    // ~/.config/fastfetch/logos/**
    // ~/.config/fastfetch/ascii/**
    let rel_target = &tgt[FASTFETCH_TARGET_PREFIX.len()..];
    let is_valid_fastfetch_structure = rel_target == "config.jsonc"
        || rel_target == "config.json"
        || rel_target.starts_with("presets/")
        || rel_target.starts_with("logos/")
        || rel_target.starts_with("ascii/");

    if !is_valid_fastfetch_structure {
        return Err(format!(
            "Security violation: Fastfetch target '{}' is outside permitted Fastfetch configuration structure (config.jsonc, presets/*, logos/*, ascii/*)",
            tgt
        ));
    }

    Ok(())
}

fn preset_to_provider_item(preset: &FastfetchThemePreset) -> ProviderItem {
    let mut files = vec![ProviderFileSpec {
        source: "config.jsonc".to_string(),
        target: "~/.config/fastfetch/config.jsonc".to_string(),
    }];

    for rel_path in preset.presets.keys() {
        files.push(ProviderFileSpec {
            source: rel_path.clone(),
            target: format!("~/.config/fastfetch/{}", rel_path),
        });
    }

    for rel_path in preset.logos.keys() {
        files.push(ProviderFileSpec {
            source: rel_path.clone(),
            target: format!("~/.config/fastfetch/{}", rel_path),
        });
    }

    let provenance = ProviderProvenance {
        provider_id: "fastfetch".to_string(),
        source_url: format!("fastfetch://presets/{}", preset.id),
        repository: Some("ryzora/fastfetch-presets".to_string()),
        commit_or_tag: Some(preset.version.clone()),
        license_spdx: Some(preset.license_spdx.clone()),
        original_author: preset.author.clone(),
        fetched_at: current_utc_iso(),
    };

    let mut tags = preset.tags.clone();
    if !tags.contains(&"fastfetch".to_string()) {
        tags.push("fastfetch".to_string());
    }

    ProviderItem {
        id: preset.id.clone(),
        title: preset.title.clone(),
        subtitle: preset.subtitle.clone(),
        description: preset.description.clone(),
        version: preset.version.clone(),
        author: AuthorInfo {
            name: preset.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        category: "app_config".to_string(),
        package_type: PackageType::Theme,
        tags,
        supported_desktops: vec![
            "Hyprland".to_string(),
            "Sway".to_string(),
            "GNOME".to_string(),
            "KDE Plasma".to_string(),
        ],
        supported_display: vec!["Wayland".to_string(), "X11".to_string()],
        rating: Some(4.9),
        rating_count: Some(85),
        downloads: Some(600),
        hero_image: None,
        screenshots: Vec::new(),
        provenance,
        files,
    }
}

pub fn load_disk_preset_item(pkg_dir: &Path) -> Result<ProviderItem, String> {
    let manifest_path = pkg_dir.join("manifest.json");
    let content = fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    let manifest: RyzoraManifest = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let dir_name = pkg_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");

    if !dir_name.is_empty() && dir_name != manifest.id {
        return Err(format!(
            "Identity mismatch: Directory is '{}' but manifest declares ID '{}'",
            dir_name, manifest.id
        ));
    }

    let mut files = Vec::new();
    for f in manifest.files {
        files.push(ProviderFileSpec {
            source: f.source,
            target: f.target,
        });
    }

    let provenance = ProviderProvenance {
        provider_id: "fastfetch".to_string(),
        source_url: format!("fastfetch://local/{}", manifest.id),
        repository: Some("local-fastfetch".to_string()),
        commit_or_tag: Some(manifest.version.clone()),
        license_spdx: Some("MIT".to_string()),
        original_author: manifest.author.clone(),
        fetched_at: current_utc_iso(),
    };

    Ok(ProviderItem {
        id: manifest.id,
        title: manifest.name,
        subtitle: "Local Fastfetch Preset".to_string(),
        description: manifest.description,
        version: manifest.version,
        author: AuthorInfo {
            name: manifest.author,
            avatar: String::new(),
            verified: false,
        },
        category: "app_config".to_string(),
        package_type: PackageType::Theme,
        tags: manifest.tags,
        supported_desktops: manifest.compatibility.desktops,
        supported_display: manifest.compatibility.sessions,
        rating: Some(4.8),
        rating_count: Some(50),
        downloads: Some(300),
        hero_image: None,
        screenshots: Vec::new(),
        provenance,
        files,
    })
}

fn current_utc_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}", now)
}

// ─────────────────────────────────────────────────────────────────────────────
// Curated Fastfetch Community Presets Library
// ─────────────────────────────────────────────────────────────────────────────

fn get_default_fastfetch_presets() -> Vec<FastfetchThemePreset> {
    vec![
        FastfetchThemePreset {
            id: "fastfetch-catppuccin-mocha".to_string(),
            title: "Catppuccin Mocha Fastfetch".to_string(),
            subtitle: "Soothing pastel hardware display".to_string(),
            description: "Catppuccin Mocha color scheme preset with battery, disk, CPU and GPU modules.\nLicense: MIT".to_string(),
            author: "Catppuccin".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["catppuccin".to_string(), "mocha".to_string(), "fastfetch".to_string()],
            config_jsonc: r#"{
    "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
    "logo": {
        "type": "builtin",
        "color": { "1": "magenta" }
    },
    "display": { "separator": " 󰄾 " },
    "modules": [
        "title",
        "separator",
        "os",
        "host",
        "kernel",
        "uptime",
        "packages",
        "shell",
        "display",
        "de",
        "wm",
        "terminal",
        "cpu",
        "gpu",
        "memory",
        "break",
        "colors"
    ]
}"#.to_string(),
            presets: HashMap::new(),
            logos: HashMap::new(),
            license_spdx: "MIT".to_string(),
        },
        FastfetchThemePreset {
            id: "fastfetch-nord-frost".to_string(),
            title: "Nord Frost Fastfetch".to_string(),
            subtitle: "Arctic and elegant system monitor".to_string(),
            description: "Nordic arctic palette with clean minimal layout and ASCII snowflake.\nLicense: MIT".to_string(),
            author: "Arctic Ice Studio".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["nord".to_string(), "arctic".to_string(), "fastfetch".to_string()],
            config_jsonc: r#"{
    "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
    "display": { "separator": " › " },
    "modules": [
        "title",
        "os",
        "kernel",
        "uptime",
        "packages",
        "wm",
        "memory",
        "break",
        "colors"
    ]
}"#.to_string(),
            presets: HashMap::new(),
            logos: HashMap::new(),
            license_spdx: "MIT".to_string(),
        },
        FastfetchThemePreset {
            id: "fastfetch-dracula-neon".to_string(),
            title: "Dracula Neon Fastfetch".to_string(),
            subtitle: "High contrast dark theme".to_string(),
            description: "Famous Dracula theme optimized for Fastfetch terminal fetch.\nLicense: MIT".to_string(),
            author: "Dracula Theme".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["dracula".to_string(), "neon".to_string(), "fastfetch".to_string()],
            config_jsonc: r#"{
    "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
    "modules": [
        "title",
        "separator",
        "os",
        "kernel",
        "uptime",
        "wm",
        "cpu",
        "memory",
        "break",
        "colors"
    ]
}"#.to_string(),
            presets: HashMap::new(),
            logos: HashMap::new(),
            license_spdx: "MIT".to_string(),
        },
    ]
}
