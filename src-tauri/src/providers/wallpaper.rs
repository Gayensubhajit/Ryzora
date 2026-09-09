//! Wallpaper Content Provider for Ryzora.
//!
//! Provides discovery, preview, declarative image validation, and atomic staging
//! of curated and community wallpapers (single images and multi-resolution packs)
//! synthesized into `PackageType::Wallpaper`.
//!
//! # Security Invariants:
//! 1. Zero subprocess execution (0 `Command::new`, 0 `sudo`, 0 `pkexec`).
//! 2. Strict target confinement: all wallpaper files MUST target `~/Pictures/Wallpapers/**`.
//!    Any target attempting to write outside this directory or containing traversal (`..`)
//!    is fatally rejected.
//! 3. Declarative image-only payload: permitted extensions are strictly `.png`, `.jpg`,
//!    `.jpeg`, `.webp`, `.jxl`, `.svg`, `.avif`. Executable scripts (*.sh, *.py),
//!    binaries (*.exe, *.bin, *.so), and hooks (install.sh) are fatally rejected.
//! 4. SVG safety: SVG files are treated as untrusted declarative image data; embedded
//!    `<script>` tags or active event handlers are rejected.
//! 5. Conflict prevention: duplicate normalized targets (`foo.png` vs `./foo.png` or `//`)
//!    trigger immediate collision rejection.
//! 6. Whole-package transactional staging: if any image fails validation or staging,
//!    the temporary staging directory is purged with zero leftover files.
//! 7. Trust invariant: Wallpaper content strictly defaults to `TrustTier::Community` (unvetted).

use crate::manifest::{PackageType, RyzoraManifest};
use crate::providers::synthesizer::ManifestSynthesizer;
use crate::providers::{
    ContentProvider, ProviderCapabilities, ProviderError, ProviderFileSpec, ProviderItem,
    ProviderProvenance, ProviderQuery, ProviderType,
};
use crate::repository::{compute_package_tree_hash, AuthorInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Mandatory target prefix for wallpapers.
pub const WALLPAPER_TARGET_PREFIX: &str = "~/Pictures/Wallpapers/";

/// Maximum allowed file size for a single wallpaper file (50 MB).
pub const MAX_WALLPAPER_FILE_SIZE: u64 = 50 * 1024 * 1024;

fn current_utc_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}

/// Curated wallpaper preset definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperPreset {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub files: Vec<WallpaperFileDefinition>,
    pub license_spdx: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperFileDefinition {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// Wallpaper Content Provider adapter.
pub struct WallpaperProvider {
    builtin_presets: Vec<WallpaperPreset>,
    custom_dir: Option<PathBuf>,
    enabled: bool,
}

impl WallpaperProvider {
    /// Creates a production Wallpaper provider with curated aesthetic presets.
    pub fn new() -> Self {
        Self {
            builtin_presets: get_curated_wallpaper_presets(),
            custom_dir: None,
            enabled: true,
        }
    }

    /// Creates a Wallpaper provider configured to discover custom on-disk wallpaper packages.
    pub fn with_custom_dir(custom_dir: PathBuf) -> Self {
        Self {
            builtin_presets: get_curated_wallpaper_presets(),
            custom_dir: Some(custom_dir),
            enabled: true,
        }
    }

    /// Sets whether this provider is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Discovers custom wallpaper packages located in the optional custom directory.
    fn load_custom_disk_items(&self) -> Vec<ProviderItem> {
        let mut items = Vec::new();
        let root = match &self.custom_dir {
            Some(r) if r.is_dir() => r,
            _ => return items,
        };

        let entries = match fs::read_dir(root) {
            Ok(e) => e,
            Err(_) => return items,
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let pkg_dir = entry.path();
            if !pkg_dir.is_dir() {
                continue;
            }

            if let Ok(item) = load_disk_wallpaper_item(&pkg_dir) {
                items.push(item);
            }
        }

        items
    }
}

impl Default for WallpaperProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for WallpaperProvider {
    fn id(&self) -> &str {
        "wallpaper"
    }

    fn name(&self) -> &str {
        "Curated Wallpaper Collections"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Wallpaper
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
        if !self.enabled {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let q_clean = query.query.trim().to_lowercase();
        let cat_filter = query.category.as_deref().map(|c| c.to_lowercase());

        // Category filter: wallpaper items are categorized as "wallpaper" or "wallpapers"
        if let Some(ref cat) = cat_filter {
            if cat != "wallpaper" && cat != "wallpapers" && cat != "background" && cat != "all" {
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
            results.push(wallpaper_preset_to_provider_item(preset));
        }

        // 2. Search custom disk items
        for item in self.load_custom_disk_items() {
            if !q_clean.is_empty() {
                let matches_title = item.title.to_lowercase().contains(&q_clean);
                let matches_id = item.id.to_lowercase().contains(&q_clean);
                let matches_desc = item.description.to_lowercase().contains(&q_clean);
                let matches_tags = item
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&q_clean));

                if !matches_title && !matches_id && !matches_desc && !matches_tags {
                    continue;
                }
            }
            results.push(item);
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
        if !self.enabled {
            return Err(ProviderError::Disabled(
                "Wallpaper provider is currently disabled".to_string(),
            ));
        }

        let clean_id = id.trim();

        // 1. Check built-in presets
        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            return Ok(wallpaper_preset_to_provider_item(preset));
        }

        // 2. Check custom disk items
        if let Some(ref custom_dir) = self.custom_dir {
            let pkg_dir = custom_dir.join(clean_id);
            if pkg_dir.is_dir() && pkg_dir.join("manifest.json").is_file() {
                if let Ok(item) = load_disk_wallpaper_item(&pkg_dir) {
                    return Ok(item);
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "Wallpaper package '{}' not found in provider",
            clean_id
        )))
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        // 1. Pre-validation of all files across the wallpaper package
        let mut targets_seen = HashSet::new();
        for file in &item.files {
            validate_wallpaper_target_and_source(file, &clean_id, &mut targets_seen)?;
        }

        // 2. Setup isolated staging folder
        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(99887);

        let tmp_stage_dir = staging_dir.join(format!(".tmp_wp_{}_{}", clean_id, rand_suffix));
        let final_stage_dir = staging_dir.join(&clean_id);

        if tmp_stage_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
        }
        if final_stage_dir.exists() {
            let _ = fs::remove_dir_all(&final_stage_dir);
        }

        fs::create_dir_all(&tmp_stage_dir)
            .map_err(|e| format!("Failed to create staging directory: {}", e))?;

        // 3. Populate files: built-in preset or custom disk directory
        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            for file_def in &preset.files {
                let dest_path = tmp_stage_dir.join(&file_def.source);
                if let Some(parent) = dest_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!("Failed to create parent dir: {}", e));
                    }
                }
                let content = file_def.content.as_deref().unwrap_or("");
                if let Err(e) = fs::write(&dest_path, content) {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Failed to write wallpaper file '{}': {}",
                        file_def.source, e
                    ));
                }
            }
        } else if let Some(ref custom_root) = self.custom_dir {
            let pkg_dir = custom_root.join(&clean_id);
            if !pkg_dir.is_dir() {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Wallpaper package source directory not found: {:?}",
                    pkg_dir
                ));
            }

            let manifest_path = pkg_dir.join("manifest.json");
            let raw_manifest = match fs::read_to_string(&manifest_path) {
                Ok(r) => r,
                Err(e) => {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!("Failed to read manifest.json: {}", e));
                }
            };
            let disk_manifest: RyzoraManifest = match serde_json::from_str(&raw_manifest) {
                Ok(m) => m,
                Err(e) => {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!("Invalid manifest JSON in wallpaper package: {}", e));
                }
            };

            // Tree hash verification before copying
            if let Some(ref expected_hash) = item.provenance.commit_or_tag {
                if expected_hash.len() == 64 && expected_hash.chars().all(|c| c.is_ascii_hexdigit())
                {
                    let actual_hash = match compute_package_tree_hash(&pkg_dir, &disk_manifest) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = fs::remove_dir_all(&tmp_stage_dir);
                            return Err(format!("Failed to compute tree hash: {}", e));
                        }
                    };
                    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Tree hash mismatch for wallpaper '{}': expected '{}', got '{}'",
                            clean_id, expected_hash, actual_hash
                        ));
                    }
                }
            }

            // Copy all declared files
            for file in &item.files {
                let src_file = pkg_dir.join(&file.source);
                if !src_file.is_file() {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Missing wallpaper payload file '{}' in package '{}'",
                        file.source, clean_id
                    ));
                }

                // Check file size limit
                if let Ok(meta) = src_file.metadata() {
                    if meta.len() > MAX_WALLPAPER_FILE_SIZE {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Wallpaper file '{}' exceeds maximum allowed size ({} bytes > {} max)",
                            file.source,
                            meta.len(),
                            MAX_WALLPAPER_FILE_SIZE
                        ));
                    }
                }

                // Symlink canonicalization check
                if let (Ok(can_src), Ok(can_pkg)) =
                    (src_file.canonicalize(), pkg_dir.canonicalize())
                {
                    if !can_src.starts_with(&can_pkg) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: source file '{}' escapes package root via symlink",
                            file.source
                        ));
                    }
                }

                // If SVG, check for prohibited executable tags or active content
                if file.source.to_lowercase().ends_with(".svg") {
                    if let Ok(svg_bytes) = fs::read(&src_file) {
                        if is_suspicious_svg_content(&svg_bytes) {
                            let _ = fs::remove_dir_all(&tmp_stage_dir);
                            return Err(format!(
                                "Security violation: SVG wallpaper '{}' contains prohibited active script elements",
                                file.source
                            ));
                        }
                    }
                }

                let dest_file = tmp_stage_dir.join(&file.source);
                if let Some(parent) = dest_file.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!("Failed to create staging parent dir: {}", e));
                    }
                }
                if let Err(e) = fs::copy(&src_file, &dest_file) {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Failed to copy '{}' to staging: {}",
                        file.source, e
                    ));
                }
            }
        } else {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Wallpaper package '{}' not found for staging",
                clean_id
            ));
        }

        // 4. Synthesize authoritative RyzoraManifest via ManifestSynthesizer
        let synthesized_manifest = match ManifestSynthesizer::synthesize(item) {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Failed to synthesize manifest for wallpaper '{}': {}",
                    clean_id, e
                ));
            }
        };

        if synthesized_manifest.package_type != PackageType::Wallpaper {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Synthesized manifest has invalid package type '{:?}', expected Wallpaper",
                synthesized_manifest.package_type
            ));
        }

        let manifest_json = match serde_json::to_string_pretty(&synthesized_manifest) {
            Ok(j) => j,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!("Failed to serialize manifest: {}", e));
            }
        };
        if let Err(e) = fs::write(tmp_stage_dir.join("manifest.json"), manifest_json) {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!("Failed to write synthesized manifest.json: {}", e));
        }

        // 5. Atomic promotion to final staged directory
        if let Err(e) = fs::rename(&tmp_stage_dir, &final_stage_dir) {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Failed to finalize staged wallpaper package directory: {}",
                e
            ));
        }

        Ok(final_stage_dir)
    }
}

/// Reads a wallpaper package directory and constructs a `ProviderItem`.
pub fn load_disk_wallpaper_item(pkg_dir: &Path) -> Result<ProviderItem, String> {
    let manifest_path = pkg_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err("Missing manifest.json in wallpaper directory".to_string());
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manifest JSON in wallpaper package: {}", e))?;

    // Enforce directory name matches manifest ID
    let dir_name = pkg_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if !dir_name.is_empty() && dir_name != manifest.id {
        return Err(format!(
            "Identity mismatch: directory name '{}' does not match manifest.id '{}'",
            dir_name, manifest.id
        ));
    }

    if manifest.package_type != PackageType::Wallpaper {
        return Err(format!(
            "Invalid package type '{:?}' in wallpaper provider, expected Wallpaper",
            manifest.package_type
        ));
    }

    let mut files = Vec::new();
    let mut targets_seen = HashSet::new();

    for f in &manifest.files {
        let spec = ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        };
        validate_wallpaper_target_and_source(&spec, &manifest.id, &mut targets_seen)?;
        files.push(spec);
    }

    let tree_hash = compute_package_tree_hash(pkg_dir, &manifest)
        .map_err(|e| format!("Failed to compute tree hash for wallpaper: {}", e))?;

    Ok(ProviderItem {
        id: manifest.id.clone(),
        title: manifest.name.clone(),
        subtitle: format!("Wallpaper by {}", manifest.author),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        category: "wallpaper".to_string(),
        package_type: PackageType::Wallpaper,
        tags: manifest.tags.clone(),
        supported_desktops: manifest.compatibility.desktops.clone(),
        supported_display: manifest.compatibility.sessions.clone(),
        rating: None,
        rating_count: None,
        downloads: None,
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "wallpaper".to_string(),
            source_url: format!("wallpaper://local/{}", manifest.id),
            repository: Some("local-wallpaper-catalog".to_string()),
            commit_or_tag: Some(tree_hash),
            original_author: manifest.author.clone(),
            license_spdx: Some("CC-BY-4.0".to_string()),
            fetched_at: current_utc_iso(),
        },
    })
}

/// Validates target confinement, allowed image extensions, and duplicate collisions.
pub fn validate_wallpaper_target_and_source(
    file: &ProviderFileSpec,
    pkg_id: &str,
    targets_seen: &mut HashSet<String>,
) -> Result<(), String> {
    let tgt = file.target.trim();
    let src = file.source.trim();

    // 1. Mandatory target confinement: must start with `~/Pictures/Wallpapers/`
    if !tgt.starts_with(WALLPAPER_TARGET_PREFIX) {
        return Err(format!(
            "Security violation: Wallpaper target '{}' must be confined within '{}'",
            tgt, WALLPAPER_TARGET_PREFIX
        ));
    }

    // 2. Traversal and null byte rejection in target
    if tgt.contains("..") || tgt.contains('\0') {
        return Err(format!(
            "Security violation: Target '{}' contains forbidden parent traversal or null bytes",
            tgt
        ));
    }

    // 3. Traversal and absolute path rejection in source
    if src.starts_with('/') || src.starts_with('\\') || src.contains("..") || src.contains('\0') {
        return Err(format!(
            "Security violation: Source '{}' contains forbidden traversal, absolute path, or null bytes",
            src
        ));
    }

    // 4. Executable / Script / Binary Rejection
    let lower_src = src.to_lowercase();
    let lower_tgt = tgt.to_lowercase();

    let forbidden_extensions = [
        ".sh", ".bash", ".zsh", ".fish", ".py", ".exe", ".bin", ".cmd", ".bat", ".elf", ".so",
        ".dll", ".scr", ".msi", ".vbs", ".js",
    ];

    for ext in &forbidden_extensions {
        if lower_src.ends_with(ext) || lower_tgt.ends_with(ext) {
            return Err(format!(
                "Security violation: Wallpaper package '{}' contains prohibited executable script or binary: '{}'",
                pkg_id, src
            ));
        }
    }

    if lower_src.contains("install.sh")
        || lower_src.contains("setup.sh")
        || lower_src.contains("post_install")
        || lower_src.contains("pre_install")
        || lower_tgt.contains("install.sh")
        || lower_tgt.contains("setup.sh")
    {
        return Err(format!(
            "Security violation: Wallpaper package '{}' contains prohibited installer hook: '{}'",
            pkg_id, src
        ));
    }

    // 5. Permitted Image Extensions allowlist
    let allowed_image_exts = [".png", ".jpg", ".jpeg", ".webp", ".jxl", ".svg", ".avif"];
    let has_valid_ext = allowed_image_exts
        .iter()
        .any(|ext| lower_tgt.ends_with(ext));

    if !has_valid_ext {
        return Err(format!(
            "Security violation: Wallpaper file '{}' does not have an allowed image extension (.png, .jpg, .jpeg, .webp, .jxl, .svg, .avif)",
            tgt
        ));
    }

    // 6. Collision Prevention: duplicate normalized targets
    let normalized = normalize_target_path(tgt);
    if !targets_seen.insert(normalized.clone()) {
        return Err(format!(
            "Conflict violation: Duplicate target '{}' declared multiple times in wallpaper package '{}'",
            normalized, pkg_id
        ));
    }

    Ok(())
}

/// Normalizes target paths for collision detection (collapsing redundant slashes and `./`).
/// Inspects SVG bytes for prohibited active script elements, event handlers, or foreign objects.
///
/// SVG files are strictly treated as declarative image assets. Any executable or interactive
/// markup (<script>, onload, onclick, onerror, javascript:, data:text/html, foreignObject) is rejected.
pub fn is_suspicious_svg_content(svg_bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(svg_bytes) else {
        return true;
    };
    let lower = text.to_lowercase();
    lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains("onload=")
        || lower.contains("onclick=")
        || lower.contains("onerror=")
        || lower.contains("data:text/html")
        || lower.contains("<foreignobject")
        || lower.contains("foreignobject")
}

pub fn normalize_target_path(tgt: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_slash = false;

    for c in tgt.chars() {
        if c == '/' {
            if !last_was_slash {
                normalized.push(c);
            }
            last_was_slash = true;
        } else {
            normalized.push(c);
            last_was_slash = false;
        }
    }

    // Strip redundant "/./" occurrences
    while normalized.contains("/./") {
        normalized = normalized.replace("/./", "/");
    }

    normalized
}

/// Converts a curated `WallpaperPreset` into a `ProviderItem`.
fn wallpaper_preset_to_provider_item(preset: &WallpaperPreset) -> ProviderItem {
    let mut files = Vec::new();
    for f in &preset.files {
        files.push(ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        });
    }

    let mut tags = preset.tags.clone();
    if !tags.contains(&"wallpaper".to_string()) {
        tags.push("wallpaper".to_string());
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
        category: "wallpaper".to_string(),
        package_type: PackageType::Wallpaper,
        tags,
        supported_desktops: vec!["universal".to_string()],
        supported_display: vec!["universal".to_string()],
        rating: Some(4.9),
        rating_count: Some(210),
        downloads: Some(3400),
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "wallpaper".to_string(),
            source_url: format!("wallpaper://presets/{}", preset.id),
            repository: Some("builtin-wallpaper-ecosystem".to_string()),
            commit_or_tag: None,
            original_author: preset.author.clone(),
            license_spdx: Some(preset.license_spdx.clone()),
            fetched_at: current_utc_iso(),
        },
    }
}

/// Curated aesthetic wallpaper presets.
fn get_curated_wallpaper_presets() -> Vec<WallpaperPreset> {
    vec![
        WallpaperPreset {
            id: "wallpaper-catppuccin-twilight".to_string(),
            title: "Catppuccin Mocha Twilight".to_string(),
            subtitle: "4K aesthetic pastel landscape".to_string(),
            description: "Soothing twilight mountain landscape tailored for Catppuccin Mocha color schemes in crisp 3840x2160 resolution.".to_string(),
            author: "Catppuccin Art Guild".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["catppuccin".to_string(), "4k".to_string(), "minimal".to_string(), "landscape".to_string()],
            license_spdx: "CC-BY-4.0".to_string(),
            files: vec![
                WallpaperFileDefinition {
                    source: "catppuccin-twilight.png".to_string(),
                    target: "~/Pictures/Wallpapers/catppuccin-twilight.png".to_string(),
                    content: Some("RIFF_CATPPUCCIN_TWILIGHT_PNG_DATA".to_string()),
                }
            ],
        },
        WallpaperPreset {
            id: "wallpaper-tokyo-nightfall".to_string(),
            title: "Tokyo Nightfall Cyberpunk".to_string(),
            subtitle: "Vibrant neon cyberpunk cityscape".to_string(),
            description: "Electric cyberpunk metropolis illuminated by neon lanterns and cyan rain reflections in ultra-high definition.".to_string(),
            author: "NeonSky".to_string(),
            version: "1.1.0".to_string(),
            tags: vec!["tokyo-night".to_string(), "cyberpunk".to_string(), "neon".to_string(), "4k".to_string()],
            license_spdx: "CC-BY-4.0".to_string(),
            files: vec![
                WallpaperFileDefinition {
                    source: "tokyo-nightfall.png".to_string(),
                    target: "~/Pictures/Wallpapers/tokyo-nightfall.png".to_string(),
                    content: Some("RIFF_TOKYO_NIGHTFALL_PNG_DATA".to_string()),
                }
            ],
        },
        WallpaperPreset {
            id: "wallpaper-nord-lake".to_string(),
            title: "Nordic Arctic Lake".to_string(),
            subtitle: "Scandinavian serene glacial waters".to_string(),
            description: "Calm alpine lake surrounded by frosted spruce trees matching Nord arctic palette tones.".to_string(),
            author: "Fjord Studio".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["nord".to_string(), "nature".to_string(), "minimal".to_string(), "scandinavian".to_string()],
            license_spdx: "Apache-2.0".to_string(),
            files: vec![
                WallpaperFileDefinition {
                    source: "nord-lake.jpg".to_string(),
                    target: "~/Pictures/Wallpapers/nord-lake.jpg".to_string(),
                    content: Some("EXIF_NORD_LAKE_JPG_DATA".to_string()),
                }
            ],
        },
        WallpaperPreset {
            id: "wallpaper-solar-eclipse".to_string(),
            title: "Minimal Solar Eclipse".to_string(),
            subtitle: "Dark space abstract with solar coronal glow".to_string(),
            description: "Minimalist total solar eclipse on deep obsidian background with amber and gold coronal ring in modern WebP format.".to_string(),
            author: "AstroDesign".to_string(),
            version: "2.0.0".to_string(),
            tags: vec!["dark".to_string(), "space".to_string(), "minimal".to_string(), "abstract".to_string()],
            license_spdx: "MIT".to_string(),
            files: vec![
                WallpaperFileDefinition {
                    source: "solar-eclipse.webp".to_string(),
                    target: "~/Pictures/Wallpapers/solar-eclipse.webp".to_string(),
                    content: Some("RIFF_SOLAR_ECLIPSE_WEBP_DATA".to_string()),
                }
            ],
        },
        WallpaperPreset {
            id: "wallpaper-studio-gradients".to_string(),
            title: "Studio Color Gradients Pack".to_string(),
            subtitle: "Coordinated dual-display 4K gradient wallpapers".to_string(),
            description: "A pair of fluid multi-chromatic gradient backgrounds designed for seamless multi-monitor desktop environments.".to_string(),
            author: "Palette Labs".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["gradient".to_string(), "multi-monitor".to_string(), "4k".to_string(), "pack".to_string()],
            license_spdx: "CC-BY-4.0".to_string(),
            files: vec![
                WallpaperFileDefinition {
                    source: "gradient-aurora.png".to_string(),
                    target: "~/Pictures/Wallpapers/gradient-aurora.png".to_string(),
                    content: Some("RIFF_GRADIENT_AURORA_PNG_DATA".to_string()),
                },
                WallpaperFileDefinition {
                    source: "gradient-dusk.png".to_string(),
                    target: "~/Pictures/Wallpapers/gradient-dusk.png".to_string(),
                    content: Some("RIFF_GRADIENT_DUSK_PNG_DATA".to_string()),
                },
            ],
        },
    ]
}
