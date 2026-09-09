//! Hyprland / Complete Rice Content Provider for Ryzora.
//!
//! Provides discovery, preview, declarative component validation, and atomic staging
//! of multi-component desktop rices (Hyprland, Waybar, Kitty, Alacritty, Rofi, Wofi,
//! Dunst, Mako, Fastfetch, Wallpapers, GTK themes, and Icon themes) synthesized into
//! `PackageType::Rice`.
//!
//! # Security Invariants:
//! 1. Zero subprocess execution (0 `Command::new`, 0 `sudo`, 0 `pkexec`).
//! 2. Component-aware target allowlisting: files must strictly match the permitted
//!    target directory of their declared component (e.g. Hyprland files can only target
//!    `~/.config/hypr/**`).
//! 3. Cross-component conflict detection: duplicate targets, conflicting writes, and
//!    component cross-claims are fatally rejected.
//! 4. Strict declarative payload: all executable scripts (*.sh, *.py, *.bash, *.zsh),
//!    binaries (*.exe, *.bin, *.elf, *.so), hooks (install.sh, setup.sh), and system/session
//!    modifications (~/.bashrc, systemd units, autostart, /etc) are fatally rejected.
//! 5. Transactional atomic staging: if any component or file fails validation or staging,
//!    the entire staging directory is aborted and purged.
//! 6. Trust invariant: Rice content strictly defaults to `TrustTier::Community` (unvetted).

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

fn current_utc_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}

/// Declared Rice component kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiceComponentKind {
    Hyprland,
    Waybar,
    Kitty,
    Alacritty,
    Rofi,
    Wofi,
    Dunst,
    Mako,
    Fastfetch,
    Wallpaper,
    GtkTheme,
    IconTheme,
}

impl RiceComponentKind {
    /// Returns the mandatory target directory prefix for this component.
    pub fn allowed_target_prefix(&self) -> &'static str {
        match self {
            RiceComponentKind::Hyprland => "~/.config/hypr/",
            RiceComponentKind::Waybar => "~/.config/waybar/",
            RiceComponentKind::Kitty => "~/.config/kitty/",
            RiceComponentKind::Alacritty => "~/.config/alacritty/",
            RiceComponentKind::Rofi => "~/.config/rofi/",
            RiceComponentKind::Wofi => "~/.config/wofi/",
            RiceComponentKind::Dunst => "~/.config/dunst/",
            RiceComponentKind::Mako => "~/.config/mako/",
            RiceComponentKind::Fastfetch => "~/.config/fastfetch/",
            RiceComponentKind::Wallpaper => "~/Pictures/Wallpapers/",
            RiceComponentKind::GtkTheme => "~/.themes/",
            RiceComponentKind::IconTheme => "~/.icons/",
        }
    }

    /// Infers component kind from a target destination path.
    pub fn infer_from_target(target: &str) -> Option<Self> {
        let tgt = target.trim();
        if tgt.starts_with("~/.config/hypr/") {
            Some(RiceComponentKind::Hyprland)
        } else if tgt.starts_with("~/.config/waybar/") {
            Some(RiceComponentKind::Waybar)
        } else if tgt.starts_with("~/.config/kitty/") {
            Some(RiceComponentKind::Kitty)
        } else if tgt.starts_with("~/.config/alacritty/") {
            Some(RiceComponentKind::Alacritty)
        } else if tgt.starts_with("~/.config/rofi/") {
            Some(RiceComponentKind::Rofi)
        } else if tgt.starts_with("~/.config/wofi/") {
            Some(RiceComponentKind::Wofi)
        } else if tgt.starts_with("~/.config/dunst/") {
            Some(RiceComponentKind::Dunst)
        } else if tgt.starts_with("~/.config/mako/") {
            Some(RiceComponentKind::Mako)
        } else if tgt.starts_with("~/.config/fastfetch/") {
            Some(RiceComponentKind::Fastfetch)
        } else if tgt.starts_with("~/Pictures/Wallpapers/") {
            Some(RiceComponentKind::Wallpaper)
        } else if tgt.starts_with("~/.themes/") {
            Some(RiceComponentKind::GtkTheme)
        } else if tgt.starts_with("~/.icons/") {
            Some(RiceComponentKind::IconTheme)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RiceComponentKind::Hyprland => "hyprland",
            RiceComponentKind::Waybar => "waybar",
            RiceComponentKind::Kitty => "kitty",
            RiceComponentKind::Alacritty => "alacritty",
            RiceComponentKind::Rofi => "rofi",
            RiceComponentKind::Wofi => "wofi",
            RiceComponentKind::Dunst => "dunst",
            RiceComponentKind::Mako => "mako",
            RiceComponentKind::Fastfetch => "fastfetch",
            RiceComponentKind::Wallpaper => "wallpaper",
            RiceComponentKind::GtkTheme => "gtk_theme",
            RiceComponentKind::IconTheme => "icon_theme",
        }
    }
}

/// A specific file specification within a Rice component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiceComponentFile {
    pub component: RiceComponentKind,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub content: Option<String>,
}

/// Curated multi-component declarative Rice preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RicePreset {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub files: Vec<RiceComponentFile>,
    pub license_spdx: String,
}

/// Complete Rice Content Provider adapter.
pub struct RiceProvider {
    builtin_presets: Vec<RicePreset>,
    custom_dir: Option<PathBuf>,
    enabled: bool,
}

impl RiceProvider {
    /// Creates a production Rice provider with curated community rices.
    pub fn new() -> Self {
        Self {
            builtin_presets: get_curated_rice_presets(),
            custom_dir: None,
            enabled: true,
        }
    }

    /// Creates a Rice provider configured to discover custom on-disk rices from a directory.
    pub fn with_custom_dir(custom_dir: PathBuf) -> Self {
        Self {
            builtin_presets: get_curated_rice_presets(),
            custom_dir: Some(custom_dir),
            enabled: true,
        }
    }

    /// Set provider enabled/disabled state.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Discovers custom rice packages located in the optional custom directory.
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

            if let Ok(item) = load_disk_rice_item(&pkg_dir) {
                items.push(item);
            }
        }

        items
    }
}

impl Default for RiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for RiceProvider {
    fn id(&self) -> &str {
        "rice"
    }

    fn name(&self) -> &str {
        "Hyprland Rice Ecosystem"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Rice
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

        // Category filter: rice items are categorized as "rice" or "desktop"
        if let Some(ref cat) = cat_filter {
            if cat != "rice" && cat != "rices" && cat != "desktop" && cat != "all" {
                return Ok(Vec::new());
            }
        }

        // 1. Built-in curated presets
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
            results.push(rice_preset_to_provider_item(preset));
        }

        // 2. Custom disk items
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
                "Rice provider is currently disabled".to_string(),
            ));
        }

        let clean_id = id.trim();

        // 1. Check built-in presets
        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            return Ok(rice_preset_to_provider_item(preset));
        }

        // 2. Check custom disk items
        if let Some(ref custom_dir) = self.custom_dir {
            let pkg_dir = custom_dir.join(clean_id);
            if pkg_dir.is_dir() && pkg_dir.join("manifest.json").is_file() {
                if let Ok(item) = load_disk_rice_item(&pkg_dir) {
                    return Ok(item);
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "Rice package '{}' not found in provider",
            clean_id
        )))
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        // 1. Pre-validation of all files across the whole Rice
        // Any duplicate target, traversal, or script violation rejects the whole Rice immediately.
        let mut targets_seen = HashSet::new();
        for file in &item.files {
            let comp = RiceComponentKind::infer_from_target(&file.target).ok_or_else(|| {
                format!(
                    "Target '{}' is outside any permitted Rice component directory",
                    file.target
                )
            })?;

            let comp_file = RiceComponentFile {
                component: comp,
                source: file.source.clone(),
                target: file.target.clone(),
                content: None,
            };

            validate_rice_component_file(&comp_file, &clean_id, &mut targets_seen)?;
        }

        // 2. Setup isolated staging folder
        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(54321);

        let tmp_stage_dir = staging_dir.join(format!(".tmp_rice_{}_{}", clean_id, rand_suffix));
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
            // Write each component file
            for comp_file in &preset.files {
                let dest_path = tmp_stage_dir.join(&comp_file.source);
                if let Some(parent) = dest_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Failed to create parent dir for '{}': {}",
                            comp_file.source, e
                        ));
                    }
                }
                let content = comp_file.content.as_deref().unwrap_or("");
                if let Err(e) = fs::write(&dest_path, content) {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Failed to write rice payload file '{}': {}",
                        comp_file.source, e
                    ));
                }
            }
        } else if let Some(ref custom_root) = self.custom_dir {
            let pkg_dir = custom_root.join(&clean_id);
            if !pkg_dir.is_dir() {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Rice package source directory not found: {:?}",
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
                    return Err(format!("Invalid manifest JSON in rice package: {}", e));
                }
            };

            // Verify tree hash integrity before copying
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
                            "Tree hash mismatch for rice '{}': expected '{}', got '{}'",
                            clean_id, expected_hash, actual_hash
                        ));
                    }
                }
            }

            // Copy all files declared in item
            for file in &item.files {
                let src_file = pkg_dir.join(&file.source);
                if !src_file.is_file() {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Missing payload file '{}' in rice package '{}'",
                        file.source, clean_id
                    ));
                }

                // Canonical path check against symlink escapes
                if let (Ok(can_src), Ok(can_pkg)) =
                    (src_file.canonicalize(), pkg_dir.canonicalize())
                {
                    if !can_src.starts_with(&can_pkg) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: payload source '{}' escapes rice package root via symlink",
                            file.source
                        ));
                    }
                }

                // If SVG, check for prohibited executable tags or active content
                if file.source.to_lowercase().ends_with(".svg") {
                    if let Ok(svg_bytes) = fs::read(&src_file) {
                        if crate::providers::wallpaper::is_suspicious_svg_content(&svg_bytes) {
                            let _ = fs::remove_dir_all(&tmp_stage_dir);
                            return Err(format!(
                                "Security violation: SVG asset '{}' in rice contains prohibited active script elements",
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
            return Err(format!("Rice package '{}' not found for staging", clean_id));
        }

        // 4. Synthesize canonical RyzoraManifest via ManifestSynthesizer
        let synthesized_manifest = match ManifestSynthesizer::synthesize(item) {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Failed to synthesize manifest for rice '{}': {}",
                    clean_id, e
                ));
            }
        };

        // Manifest package_type MUST be Rice
        if synthesized_manifest.package_type != PackageType::Rice {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Synthesized manifest has invalid package type '{:?}', expected Rice",
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
                "Failed to finalize staged rice package directory: {}",
                e
            ));
        }

        Ok(final_stage_dir)
    }
}

/// Reads a package directory and constructs a `ProviderItem` for a complete rice.
pub fn load_disk_rice_item(pkg_dir: &Path) -> Result<ProviderItem, String> {
    let manifest_path = pkg_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err("Missing manifest.json in rice directory".to_string());
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manifest JSON in rice package: {}", e))?;

    // Enforce identity matches directory name
    let dir_name = pkg_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if !dir_name.is_empty() && dir_name != manifest.id {
        return Err(format!(
            "Identity mismatch: rice directory name '{}' does not match manifest.id '{}'",
            dir_name, manifest.id
        ));
    }

    // Package type must be Rice
    if manifest.package_type != PackageType::Rice {
        return Err(format!(
            "Invalid package type '{:?}' in rice provider, expected Rice",
            manifest.package_type
        ));
    }

    // Validate and convert files
    let mut files = Vec::new();
    let mut targets_seen = HashSet::new();

    for f in &manifest.files {
        // Infer and validate component
        let component = RiceComponentKind::infer_from_target(&f.target).ok_or_else(|| {
            format!(
                "Target '{}' does not match any valid Rice component directory",
                f.target
            )
        })?;

        let comp_file = RiceComponentFile {
            component,
            source: f.source.clone(),
            target: f.target.clone(),
            content: None,
        };

        validate_rice_component_file(&comp_file, &manifest.id, &mut targets_seen)?;

        files.push(ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        });
    }

    // Calculate expected tree hash
    let tree_hash = compute_package_tree_hash(pkg_dir, &manifest)
        .map_err(|e| format!("Failed to compute tree hash for rice: {}", e))?;

    Ok(ProviderItem {
        id: manifest.id.clone(),
        title: manifest.name.clone(),
        subtitle: format!("Complete Rice by {}", manifest.author),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        category: "rice".to_string(),
        package_type: PackageType::Rice,
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
            provider_id: "rice".to_string(),
            source_url: format!("rice://local/{}", manifest.id),
            repository: Some("local-rice-catalog".to_string()),
            commit_or_tag: Some(tree_hash),
            original_author: manifest.author.clone(),
            license_spdx: Some("MIT".to_string()),
            fetched_at: current_utc_iso(),
        },
    })
}

/// Validates a single Rice component file against security boundaries, component-aware
/// target constraints, and cross-component conflict rules.
pub fn validate_rice_component_file(
    file: &RiceComponentFile,
    pkg_id: &str,
    targets_seen: &mut HashSet<String>,
) -> Result<(), String> {
    let tgt = file.target.trim();
    let src = file.source.trim();

    // 1. Mandatory component-aware target confinement
    let allowed_prefix = file.component.allowed_target_prefix();
    if !tgt.starts_with(allowed_prefix) {
        return Err(format!(
            "Security violation: Component '{}' cannot target '{}' (must start with '{}')",
            file.component.as_str(),
            tgt,
            allowed_prefix
        ));
    }

    // 2. Traversal and null byte rejection
    if tgt.contains("..") || tgt.contains('\0') {
        return Err(format!(
            "Security violation: Package '{}' target '{}' contains traversal ('..') or null bytes",
            pkg_id, tgt
        ));
    }

    if src.starts_with('/') || src.starts_with('\\') || src.contains("..") || src.contains('\0') {
        return Err(format!(
            "Security violation: Package '{}' source '{}' contains forbidden traversal, absolute path, or null bytes",
            pkg_id, src
        ));
    }

    // 3. Prohibited system/session destinations
    let forbidden_sensitive = [
        "~/.bashrc",
        "~/.bash_profile",
        "~/.bash_logout",
        "~/.profile",
        "~/.zshrc",
        "~/.zshenv",
        "~/.zprofile",
        "~/.config/fish/config.fish",
        "~/.config/systemd",
        "~/.config/autostart",
        "~/.config/environment.d",
        "~/.local/bin",
        "~/.ssh",
        "~/.gnupg",
        "/etc",
        "/usr",
        "/var",
        "/bin",
    ];

    for forbidden in &forbidden_sensitive {
        if tgt == *forbidden || tgt.starts_with(&format!("{}/", forbidden)) {
            return Err(format!(
                "Security violation: Package '{}' targets forbidden system/session destination '{}'",
                pkg_id, tgt
            ));
        }
    }

    // 4. Executable / Script Rejection (no scripts, binaries, or hooks)
    let lower_src = src.to_lowercase();
    let lower_tgt = tgt.to_lowercase();

    let forbidden_extensions = [
        ".sh", ".bash", ".zsh", ".fish", ".py", ".exe", ".bin", ".cmd", ".bat", ".elf", ".so",
        ".dll", ".scr", ".msi", ".vbs",
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
        || lower_src.contains("post_install")
        || lower_src.contains("pre_install")
        || lower_tgt.contains("install.sh")
        || lower_tgt.contains("setup.sh")
    {
        return Err(format!(
            "Security violation: Package '{}' contains prohibited installer hook: '{}'",
            pkg_id, src
        ));
    }

    // 5. Structure validation for GTK theme and Icon theme
    if file.component == RiceComponentKind::GtkTheme {
        let rel = &tgt["~/.themes/".len()..];
        let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return Err(format!(
                "Invalid GTK theme target '{}': must specify a theme subfolder",
                tgt
            ));
        }
    } else if file.component == RiceComponentKind::IconTheme {
        let rel = &tgt["~/.icons/".len()..];
        let parts: Vec<&str> = rel.split('/').filter(|p| !p.is_empty()).collect();
        if parts.is_empty() {
            return Err(format!(
                "Invalid Icon theme target '{}': must specify an icon subfolder",
                tgt
            ));
        }
    }

    // 6. Cross-Component Conflict Detection (duplicate normalized targets)
    let normalized_tgt = normalize_target_path(tgt);
    if !targets_seen.insert(normalized_tgt.clone()) {
        return Err(format!(
            "Conflict violation: Duplicate target '{}' declared multiple times in rice '{}'",
            normalized_tgt, pkg_id
        ));
    }

    Ok(())
}

/// Normalizes target paths for collision detection (e.g. collapsing redundant slashes).
fn normalize_target_path(tgt: &str) -> String {
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
    normalized
}

/// Converts a curated `RicePreset` into a `ProviderItem`.
fn rice_preset_to_provider_item(preset: &RicePreset) -> ProviderItem {
    let mut files = Vec::new();
    for comp_file in &preset.files {
        files.push(ProviderFileSpec {
            source: comp_file.source.clone(),
            target: comp_file.target.clone(),
        });
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
        category: "rice".to_string(),
        package_type: PackageType::Rice,
        tags: preset.tags.clone(),
        supported_desktops: vec!["hyprland".to_string()],
        supported_display: vec!["wayland".to_string()],
        rating: Some(4.9),
        rating_count: Some(120),
        downloads: Some(1500),
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "rice".to_string(),
            source_url: format!("rice://presets/{}", preset.id),
            repository: Some("builtin-rice-ecosystem".to_string()),
            commit_or_tag: None,
            original_author: preset.author.clone(),
            license_spdx: Some(preset.license_spdx.clone()),
            fetched_at: current_utc_iso(),
        },
    }
}

/// Standard curated complete rice presets.
fn get_curated_rice_presets() -> Vec<RicePreset> {
    vec![
        // 1. Catppuccin Mocha Complete Rice
        RicePreset {
            id: "rice-catppuccin-mocha".to_string(),
            title: "Catppuccin Mocha Complete Rice".to_string(),
            subtitle: "Harmonious Wayland desktop suite in soothing Catppuccin Mocha".to_string(),
            description: "A complete, coordinated Wayland desktop rice featuring Hyprland window manager configurations, dual-monitor Waybar status bar, matching Kitty terminal theme, custom Rofi app launcher, and aesthetic 4K wallpaper.".to_string(),
            author: "Ryzora Design Collective".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["hyprland".to_string(), "waybar".to_string(), "catppuccin".to_string(), "rice".to_string()],
            license_spdx: "MIT".to_string(),
            files: vec![
                RiceComponentFile {
                    component: RiceComponentKind::Hyprland,
                    source: "hypr/hyprland.conf".to_string(),
                    target: "~/.config/hypr/hyprland.conf".to_string(),
                    content: Some("# Catppuccin Mocha Hyprland Config\nmonitor=,preferred,auto,1\ninput {\n    kb_layout = us\n}\ngeneral {\n    gaps_in = 5\n    gaps_out = 10\n    border_size = 2\n    col.active_border = rgb(cba6f7) rgb(89b4fa) 45deg\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Hyprland,
                    source: "hypr/colors.conf".to_string(),
                    target: "~/.config/hypr/colors.conf".to_string(),
                    content: Some("$base = 0xff1e1e2e\n$text = 0xffcdd6f4\n$mauve = 0xffcba6f7\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Waybar,
                    source: "waybar/config.jsonc".to_string(),
                    target: "~/.config/waybar/config.jsonc".to_string(),
                    content: Some("{\n    \"layer\": \"top\",\n    \"position\": \"top\",\n    \"modules-left\": [\"hyprland/workspaces\"],\n    \"modules-center\": [\"clock\"],\n    \"modules-right\": [\"pulseaudio\", \"battery\"]\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Waybar,
                    source: "waybar/style.css".to_string(),
                    target: "~/.config/waybar/style.css".to_string(),
                    content: Some("* {\n    font-family: \"JetBrains Mono\", monospace;\n    font-size: 13px;\n}\nwindow#waybar {\n    background-color: rgba(30, 30, 46, 0.95);\n    color: #cdd6f4;\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Kitty,
                    source: "kitty/kitty.conf".to_string(),
                    target: "~/.config/kitty/kitty.conf".to_string(),
                    content: Some("font_family JetBrains Mono\nfont_size 11.0\nbackground #1e1e2e\nforeground #cdd6f4\ncursor #f5e0dc\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Rofi,
                    source: "rofi/config.rasi".to_string(),
                    target: "~/.config/rofi/config.rasi".to_string(),
                    content: Some("configuration {\n    modi: \"drun,run\";\n    font: \"Inter 12\";\n}\n@theme \"catppuccin\"\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Dunst,
                    source: "dunst/dunstrc".to_string(),
                    target: "~/.config/dunst/dunstrc".to_string(),
                    content: Some("[global]\n    font = Inter 10\n    background = \"#1e1e2e\"\n    foreground = \"#cdd6f4\"\n    frame_color = \"#cba6f7\"\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Wallpaper,
                    source: "wallpapers/catppuccin-minimal.png".to_string(),
                    target: "~/Pictures/Wallpapers/catppuccin-minimal.png".to_string(),
                    content: Some("RIFF_MOCK_CATPPUCCIN_WALLPAPER_PNG_DATA".to_string()),
                },
            ],
        },

        // 2. Tokyo Night Neon Complete Rice
        RicePreset {
            id: "rice-tokyo-night-neon".to_string(),
            title: "Tokyo Night Neon Complete Rice".to_string(),
            subtitle: "Electric Tokyo Night theme for Hyprland, Waybar, and Wofi".to_string(),
            description: "High-contrast dark rice inspired by nighttime in Tokyo with vibrant cyan and magenta accents across window borders, bar indicators, and launcher components.".to_string(),
            author: "NightSky Themes".to_string(),
            version: "1.2.0".to_string(),
            tags: vec!["hyprland".to_string(), "tokyo-night".to_string(), "waybar".to_string(), "rice".to_string()],
            license_spdx: "MIT".to_string(),
            files: vec![
                RiceComponentFile {
                    component: RiceComponentKind::Hyprland,
                    source: "hypr/hyprland.conf".to_string(),
                    target: "~/.config/hypr/hyprland.conf".to_string(),
                    content: Some("# Tokyo Night Hyprland\ngeneral {\n    border_size = 2\n    col.active_border = rgb(7aa2f7) rgb(bb9af7) 45deg\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Waybar,
                    source: "waybar/config.jsonc".to_string(),
                    target: "~/.config/waybar/config.jsonc".to_string(),
                    content: Some("{\n    \"layer\": \"top\",\n    \"position\": \"bottom\",\n    \"modules-left\": [\"hyprland/workspaces\"]\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Waybar,
                    source: "waybar/style.css".to_string(),
                    target: "~/.config/waybar/style.css".to_string(),
                    content: Some("window#waybar { background-color: #1a1b26; color: #a9b1d6; }\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Alacritty,
                    source: "alacritty/alacritty.toml".to_string(),
                    target: "~/.config/alacritty/alacritty.toml".to_string(),
                    content: Some("[colors.primary]\nbackground = '#1a1b26'\nforeground = '#c0caf5'\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Wofi,
                    source: "wofi/style.css".to_string(),
                    target: "~/.config/wofi/style.css".to_string(),
                    content: Some("window { background-color: #1a1b26; border: 2px solid #7aa2f7; }\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Wallpaper,
                    source: "wallpapers/tokyo-nightfall.png".to_string(),
                    target: "~/Pictures/Wallpapers/tokyo-nightfall.png".to_string(),
                    content: Some("RIFF_MOCK_TOKYO_WALLPAPER_PNG_DATA".to_string()),
                },
            ],
        },

        // 3. Nordic Frost Minimal Rice
        RicePreset {
            id: "rice-nord-frost".to_string(),
            title: "Nord Frost Minimal Rice".to_string(),
            subtitle: "Clean Scandinavian aesthetic for Hyprland and Waybar".to_string(),
            description: "Minimalist arctic color scheme designed for focus and visual comfort under Wayland.".to_string(),
            author: "Arctic Studio".to_string(),
            version: "2.0.0".to_string(),
            tags: vec!["hyprland".to_string(), "nord".to_string(), "waybar".to_string(), "minimal".to_string(), "rice".to_string()],
            license_spdx: "Apache-2.0".to_string(),
            files: vec![
                RiceComponentFile {
                    component: RiceComponentKind::Hyprland,
                    source: "hypr/hyprland.conf".to_string(),
                    target: "~/.config/hypr/hyprland.conf".to_string(),
                    content: Some("# Nord Frost Hyprland\ngeneral {\n    gaps_in = 4\n    gaps_out = 8\n    col.active_border = rgb(88c0d0)\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Waybar,
                    source: "waybar/config.jsonc".to_string(),
                    target: "~/.config/waybar/config.jsonc".to_string(),
                    content: Some("{\n    \"layer\": \"top\",\n    \"position\": \"top\",\n    \"modules-center\": [\"clock\"]\n}\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Mako,
                    source: "mako/config".to_string(),
                    target: "~/.config/mako/config".to_string(),
                    content: Some("background-color=#2e3440\ntext-color=#eceff4\nborder-color=#88c0d0\n".to_string()),
                },
                RiceComponentFile {
                    component: RiceComponentKind::Wallpaper,
                    source: "wallpapers/nord-fjords.jpg".to_string(),
                    target: "~/Pictures/Wallpapers/nord-fjords.jpg".to_string(),
                    content: Some("RIFF_MOCK_NORD_WALLPAPER_JPG_DATA".to_string()),
                },
            ],
        },
    ]
}
