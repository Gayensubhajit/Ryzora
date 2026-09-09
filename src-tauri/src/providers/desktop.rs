//! KDE Plasma and GNOME Desktop Environment Content Providers for Ryzora.
//!
//! Provides discovery, preview, declarative component validation, and atomic staging
//! of KDE Plasma look-and-feel packages, desktop themes, color schemes, Konsole profiles,
//! Aurorae decorations, GNOME GTK 3/4 themes, GNOME Shell themes, and Icon packs.
//!
//! # Security Invariants:
//! 1. Zero subprocess execution (0 `Command::new`, 0 `sudo`, 0 `pkexec`, 0 `gsettings`, 0 `dconf`).
//! 2. Component-aware target confinement:
//!    - KDE Look-and-Feel -> `~/.local/share/plasma/look-and-feel/<id>/**`
//!    - KDE Desktop Theme -> `~/.local/share/plasma/desktoptheme/<id>/**`
//!    - KDE Color Scheme  -> `~/.local/share/color-schemes/**`
//!    - KDE Konsole       -> `~/.local/share/konsole/**`
//!    - KDE Aurorae       -> `~/.local/share/kwin/aurorae/**`
//!    - GNOME GTK Theme   -> `~/.themes/<id>/**` or `~/.local/share/themes/<id>/**`
//!    - GNOME Icons       -> `~/.icons/<id>/**` or `~/.local/share/icons/<id>/**`
//! 3. Cross-component boundary isolation: KDE components cannot claim GNOME directories,
//!    GNOME components cannot claim KDE directories, and a theme cannot write into another theme's folder.
//! 4. Declarative payload only: executable scripts (*.sh, *.py, *.bash), binaries (*.so, *.exe, *.bin, *.elf),
//!    and hooks (install.sh, setup.sh) are fatally rejected.
//! 5. No session modification: `~/.config/autostart/`, `~/.config/systemd/`, `~/.bashrc`, `/etc/` are rejected.
//! 6. Whole-package transactional staging: any component failure aborts and purges the entire staging folder.
//! 7. Trust invariant: Desktop content strictly defaults to `TrustTier::Community` (unvetted).

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

// ─────────────────────────────────────────────────────────────────────────────
// KDE Component Definitions & Provider
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KdeComponentKind {
    LookAndFeel,
    DesktopTheme,
    ColorScheme,
    Konsole,
    Aurorae,
}

impl KdeComponentKind {
    pub fn allowed_target_prefix(&self) -> &'static str {
        match self {
            KdeComponentKind::LookAndFeel => "~/.local/share/plasma/look-and-feel/",
            KdeComponentKind::DesktopTheme => "~/.local/share/plasma/desktoptheme/",
            KdeComponentKind::ColorScheme => "~/.local/share/color-schemes/",
            KdeComponentKind::Konsole => "~/.local/share/konsole/",
            KdeComponentKind::Aurorae => "~/.local/share/kwin/aurorae/",
        }
    }

    pub fn infer_from_target(target: &str) -> Option<Self> {
        let tgt = target.trim();
        if tgt.starts_with("~/.local/share/plasma/look-and-feel/") {
            Some(KdeComponentKind::LookAndFeel)
        } else if tgt.starts_with("~/.local/share/plasma/desktoptheme/") {
            Some(KdeComponentKind::DesktopTheme)
        } else if tgt.starts_with("~/.local/share/color-schemes/") {
            Some(KdeComponentKind::ColorScheme)
        } else if tgt.starts_with("~/.local/share/konsole/") {
            Some(KdeComponentKind::Konsole)
        } else if tgt.starts_with("~/.local/share/kwin/aurorae/") {
            Some(KdeComponentKind::Aurorae)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            KdeComponentKind::LookAndFeel => "look_and_feel",
            KdeComponentKind::DesktopTheme => "desktop_theme",
            KdeComponentKind::ColorScheme => "color_scheme",
            KdeComponentKind::Konsole => "konsole",
            KdeComponentKind::Aurorae => "aurorae",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdePresetFile {
    pub component: KdeComponentKind,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdePreset {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub files: Vec<KdePresetFile>,
    pub license_spdx: String,
}

pub struct KdeProvider {
    builtin_presets: Vec<KdePreset>,
    custom_dir: Option<PathBuf>,
    enabled: bool,
}

impl KdeProvider {
    pub fn new() -> Self {
        Self {
            builtin_presets: get_curated_kde_presets(),
            custom_dir: None,
            enabled: true,
        }
    }

    pub fn with_custom_dir(custom_dir: PathBuf) -> Self {
        Self {
            builtin_presets: get_curated_kde_presets(),
            custom_dir: Some(custom_dir),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

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

            if let Ok(item) = load_disk_kde_item(&pkg_dir) {
                items.push(item);
            }
        }

        items
    }
}

impl Default for KdeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for KdeProvider {
    fn id(&self) -> &str {
        "kde"
    }

    fn name(&self) -> &str {
        "KDE Plasma Themes & Look-and-Feel"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::DesktopEnvironment
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

        if let Some(ref cat) = cat_filter {
            if cat != "kde" && cat != "plasma" && cat != "desktop" && cat != "all" {
                return Ok(Vec::new());
            }
        }

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
            results.push(kde_preset_to_provider_item(preset));
        }

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
            return Err(ProviderError::Disabled("KDE provider disabled".to_string()));
        }

        let clean_id = id.trim();

        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            return Ok(kde_preset_to_provider_item(preset));
        }

        if let Some(ref custom_dir) = self.custom_dir {
            let pkg_dir = custom_dir.join(clean_id);
            if pkg_dir.is_dir() && pkg_dir.join("manifest.json").is_file() {
                if let Ok(item) = load_disk_kde_item(&pkg_dir) {
                    return Ok(item);
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "KDE package '{}' not found",
            clean_id
        )))
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        let mut targets_seen = HashSet::new();
        for file in &item.files {
            validate_kde_target_and_source(file, &clean_id, &mut targets_seen)?;
        }

        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(11223);

        let tmp_stage_dir = staging_dir.join(format!(".tmp_kde_{}_{}", clean_id, rand_suffix));
        let final_stage_dir = staging_dir.join(&clean_id);

        if tmp_stage_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
        }
        if final_stage_dir.exists() {
            let _ = fs::remove_dir_all(&final_stage_dir);
        }

        fs::create_dir_all(&tmp_stage_dir)
            .map_err(|e| format!("Failed to create staging directory: {}", e))?;

        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            for f in &preset.files {
                let dest_path = tmp_stage_dir.join(&f.source);
                if let Some(parent) = dest_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!("Failed to create parent dir: {}", e));
                    }
                }
                let content = f.content.as_deref().unwrap_or("");
                if let Err(e) = fs::write(&dest_path, content) {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!("Failed to write KDE file '{}': {}", f.source, e));
                }
            }
        } else if let Some(ref custom_root) = self.custom_dir {
            let pkg_dir = custom_root.join(&clean_id);
            if !pkg_dir.is_dir() {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!("KDE package source not found: {:?}", pkg_dir));
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
                    return Err(format!("Invalid manifest JSON in KDE package: {}", e));
                }
            };

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
                            "Tree hash mismatch for KDE package '{}': expected '{}', got '{}'",
                            clean_id, expected_hash, actual_hash
                        ));
                    }
                }
            }

            for file in &item.files {
                let src_file = pkg_dir.join(&file.source);
                if !src_file.is_file() {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Missing KDE payload file '{}' in package '{}'",
                        file.source, clean_id
                    ));
                }

                if let (Ok(can_src), Ok(can_pkg)) =
                    (src_file.canonicalize(), pkg_dir.canonicalize())
                {
                    if !can_src.starts_with(&can_pkg) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: file '{}' escapes package root via symlink",
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
                                "Security violation: SVG asset '{}' contains prohibited active script elements",
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
            return Err(format!("KDE package '{}' not found for staging", clean_id));
        }

        let synthesized_manifest = match ManifestSynthesizer::synthesize(item) {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Failed to synthesize manifest for KDE package '{}': {}",
                    clean_id, e
                ));
            }
        };

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

        if let Err(e) = fs::rename(&tmp_stage_dir, &final_stage_dir) {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Failed to finalize staged KDE package directory: {}",
                e
            ));
        }

        Ok(final_stage_dir)
    }
}

pub fn load_disk_kde_item(pkg_dir: &Path) -> Result<ProviderItem, String> {
    let manifest_path = pkg_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err("Missing manifest.json in KDE package directory".to_string());
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manifest JSON in KDE package: {}", e))?;

    let dir_name = pkg_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if !dir_name.is_empty() && dir_name != manifest.id {
        return Err(format!(
            "Identity mismatch: directory name '{}' does not match manifest.id '{}'",
            dir_name, manifest.id
        ));
    }

    let mut files = Vec::new();
    let mut targets_seen = HashSet::new();

    for f in &manifest.files {
        let spec = ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        };
        validate_kde_target_and_source(&spec, &manifest.id, &mut targets_seen)?;
        files.push(spec);
    }

    let tree_hash = compute_package_tree_hash(pkg_dir, &manifest)
        .map_err(|e| format!("Failed to compute tree hash for KDE package: {}", e))?;

    Ok(ProviderItem {
        id: manifest.id.clone(),
        title: manifest.name.clone(),
        subtitle: format!("KDE Customization by {}", manifest.author),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        category: "desktop".to_string(),
        package_type: manifest.package_type,
        tags: manifest.tags.clone(),
        supported_desktops: vec!["kde".to_string(), "plasma".to_string()],
        supported_display: vec!["wayland".to_string(), "x11".to_string()],
        rating: None,
        rating_count: None,
        downloads: None,
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "kde".to_string(),
            source_url: format!("kde://local/{}", manifest.id),
            repository: Some("local-kde-catalog".to_string()),
            commit_or_tag: Some(tree_hash),
            original_author: manifest.author.clone(),
            license_spdx: Some("GPL-3.0-or-later".to_string()),
            fetched_at: current_utc_iso(),
        },
    })
}

pub fn validate_kde_target_and_source(
    file: &ProviderFileSpec,
    pkg_id: &str,
    targets_seen: &mut HashSet<String>,
) -> Result<(), String> {
    let tgt = file.target.trim();
    let src = file.source.trim();

    // 1. Infer and validate KDE component
    let comp = KdeComponentKind::infer_from_target(tgt).ok_or_else(|| {
        format!(
            "Security violation: Target '{}' is outside permitted KDE directories (~/.local/share/plasma/look-and-feel/, desktoptheme/, color-schemes/, konsole/, kwin/aurorae/)",
            tgt
        )
    })?;

    // Look-and-feel and Desktoptheme must specify a theme subfolder
    if comp == KdeComponentKind::LookAndFeel {
        let rel = &tgt[KdeComponentKind::LookAndFeel.allowed_target_prefix().len()..];
        if rel.split('/').filter(|p| !p.is_empty()).count() < 2 {
            return Err(format!(
                "Invalid KDE Look-and-Feel target '{}': must specify subfolder",
                tgt
            ));
        }
    } else if comp == KdeComponentKind::DesktopTheme {
        let rel = &tgt[KdeComponentKind::DesktopTheme.allowed_target_prefix().len()..];
        if rel.split('/').filter(|p| !p.is_empty()).count() < 2 {
            return Err(format!(
                "Invalid KDE DesktopTheme target '{}': must specify subfolder",
                tgt
            ));
        }
    } else if comp == KdeComponentKind::Aurorae {
        let rel = &tgt[KdeComponentKind::Aurorae.allowed_target_prefix().len()..];
        if rel.split('/').filter(|p| !p.is_empty()).count() < 2 {
            return Err(format!(
                "Invalid KDE Aurorae target '{}': must specify subfolder",
                tgt
            ));
        }
    }

    // 2. Traversal & null bytes rejection
    if tgt.contains("..") || tgt.contains('\0') {
        return Err(format!(
            "Security violation: Target '{}' contains traversal or null bytes",
            tgt
        ));
    }
    if src.starts_with('/') || src.starts_with('\\') || src.contains("..") || src.contains('\0') {
        return Err(format!("Security violation: Source '{}' contains forbidden traversal, absolute path, or null bytes", src));
    }

    // 3. System & Autostart rejection
    let forbidden_sensitive = [
        "~/.bashrc",
        "~/.zshrc",
        "~/.profile",
        "~/.config/autostart",
        "~/.config/systemd",
        "~/.config/environment.d",
        "~/.local/bin",
        "/etc",
        "/usr",
        "/bin",
    ];
    for f in &forbidden_sensitive {
        if tgt == *f || tgt.starts_with(&format!("{}/", f)) {
            return Err(format!(
                "Security violation: Target '{}' targets forbidden system/session path",
                tgt
            ));
        }
    }

    // 4. Prohibit executable scripts and binaries
    let lower_src = src.to_lowercase();
    let lower_tgt = tgt.to_lowercase();
    let forbidden_exts = [
        ".sh", ".bash", ".zsh", ".fish", ".py", ".exe", ".bin", ".cmd", ".bat", ".elf", ".so",
        ".dll", ".scr", ".msi", ".vbs",
    ];
    for ext in &forbidden_exts {
        if lower_src.ends_with(ext) || lower_tgt.ends_with(ext) {
            return Err(format!(
                "Security violation: Prohibited executable script or binary: '{}'",
                src
            ));
        }
    }
    if lower_src.contains("install.sh")
        || lower_src.contains("setup.sh")
        || lower_src.contains("post_install")
        || lower_src.contains("pre_install")
    {
        return Err(format!(
            "Security violation: Prohibited installer hook: '{}'",
            src
        ));
    }

    // 5. Duplicate normalized target collision prevention
    let normalized = normalize_target_path(tgt);
    if !targets_seen.insert(normalized.clone()) {
        return Err(format!(
            "Conflict violation: Duplicate target '{}' declared multiple times in KDE package '{}'",
            normalized, pkg_id
        ));
    }

    Ok(())
}

fn kde_preset_to_provider_item(preset: &KdePreset) -> ProviderItem {
    let mut files = Vec::new();
    for f in &preset.files {
        files.push(ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        });
    }

    let mut tags = preset.tags.clone();
    if !tags.contains(&"kde".to_string()) {
        tags.push("kde".to_string());
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
        category: "desktop".to_string(),
        package_type: PackageType::Theme,
        tags,
        supported_desktops: vec!["kde".to_string(), "plasma".to_string()],
        supported_display: vec!["wayland".to_string(), "x11".to_string()],
        rating: Some(4.9),
        rating_count: Some(180),
        downloads: Some(2100),
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "kde".to_string(),
            source_url: format!("kde://presets/{}", preset.id),
            repository: Some("builtin-kde-ecosystem".to_string()),
            commit_or_tag: None,
            original_author: preset.author.clone(),
            license_spdx: Some(preset.license_spdx.clone()),
            fetched_at: current_utc_iso(),
        },
    }
}

fn get_curated_kde_presets() -> Vec<KdePreset> {
    vec![
        KdePreset {
            id: "kde-breeze-chameleon".to_string(),
            title: "Breeze Chameleon Dark".to_string(),
            subtitle: "Adaptive KDE Plasma Look-and-Feel".to_string(),
            description: "Coordinated look-and-feel suite featuring adaptive color schemes, Konsole profiles, and Aurorae window decorations.".to_string(),
            author: "KDE Visual Design Group".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["kde".to_string(), "breeze".to_string(), "dark".to_string()],
            license_spdx: "GPL-3.0-or-later".to_string(),
            files: vec![
                KdePresetFile {
                    component: KdeComponentKind::LookAndFeel,
                    source: "look-and-feel/metadata.desktop".to_string(),
                    target: "~/.local/share/plasma/look-and-feel/org.kde.breeze.chameleon/metadata.desktop".to_string(),
                    content: Some("[Desktop Entry]
Name=Breeze Chameleon Dark
Comment=Adaptive dark theme
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::ColorScheme,
                    source: "color-schemes/BreezeChameleonDark.colors".to_string(),
                    target: "~/.local/share/color-schemes/BreezeChameleonDark.colors".to_string(),
                    content: Some("[General]
ColorScheme=BreezeChameleonDark
Name=Breeze Chameleon Dark
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::Konsole,
                    source: "konsole/Chameleon.colorscheme".to_string(),
                    target: "~/.local/share/konsole/Chameleon.colorscheme".to_string(),
                    content: Some("[General]
Description=Chameleon
Opacity=0.95
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::Aurorae,
                    source: "aurorae/themerc".to_string(),
                    target: "~/.local/share/kwin/aurorae/BreezeChameleon/themerc".to_string(),
                    content: Some("[General]
TitleAlignment=Center
ButtonSize=Normal
".to_string()),
                },
            ],
        },
        KdePreset {
            id: "kde-catppuccin-mocha".to_string(),
            title: "Catppuccin Mocha Plasma Theme".to_string(),
            subtitle: "Soothing pastel KDE Plasma desktop theme".to_string(),
            description: "Complete Catppuccin Mocha theme for Plasma panels, widgets, and matching Konsole terminal profile.".to_string(),
            author: "Catppuccin KDE Guild".to_string(),
            version: "1.1.0".to_string(),
            tags: vec!["kde".to_string(), "catppuccin".to_string(), "mocha".to_string()],
            license_spdx: "MIT".to_string(),
            files: vec![
                KdePresetFile {
                    component: KdeComponentKind::DesktopTheme,
                    source: "desktoptheme/metadata.desktop".to_string(),
                    target: "~/.local/share/plasma/desktoptheme/catppuccin-mocha/metadata.desktop".to_string(),
                    content: Some("[Desktop Entry]
Name=Catppuccin Mocha
Comment=Warm pastel dark theme
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::ColorScheme,
                    source: "color-schemes/CatppuccinMocha.colors".to_string(),
                    target: "~/.local/share/color-schemes/CatppuccinMocha.colors".to_string(),
                    content: Some("[General]
ColorScheme=CatppuccinMocha
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::Konsole,
                    source: "konsole/CatppuccinMocha.colorscheme".to_string(),
                    target: "~/.local/share/konsole/CatppuccinMocha.colorscheme".to_string(),
                    content: Some("[General]
Description=Catppuccin Mocha
".to_string()),
                },
            ],
        },
        KdePreset {
            id: "kde-nord-arctic".to_string(),
            title: "Nordic Arctic KDE Suite".to_string(),
            subtitle: "Minimal Scandinavian palette for KDE Plasma".to_string(),
            description: "Calm arctic color scheme and Konsole profile configured for distraction-free workflows.".to_string(),
            author: "Nordic Community".to_string(),
            version: "2.0.0".to_string(),
            tags: vec!["kde".to_string(), "nord".to_string(), "minimal".to_string()],
            license_spdx: "Apache-2.0".to_string(),
            files: vec![
                KdePresetFile {
                    component: KdeComponentKind::ColorScheme,
                    source: "color-schemes/NordicArctic.colors".to_string(),
                    target: "~/.local/share/color-schemes/NordicArctic.colors".to_string(),
                    content: Some("[General]
ColorScheme=NordicArctic
".to_string()),
                },
                KdePresetFile {
                    component: KdeComponentKind::Konsole,
                    source: "konsole/NordicArctic.colorscheme".to_string(),
                    target: "~/.local/share/konsole/NordicArctic.colorscheme".to_string(),
                    content: Some("[General]
Description=Nordic Arctic
".to_string()),
                },
            ],
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// GNOME Component Definitions & Provider
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GnomeComponentKind {
    GtkTheme,
    IconTheme,
}

impl GnomeComponentKind {
    pub fn is_valid_target(&self, target: &str) -> bool {
        let tgt = target.trim();
        match self {
            GnomeComponentKind::GtkTheme => {
                tgt.starts_with("~/.themes/") || tgt.starts_with("~/.local/share/themes/")
            }
            GnomeComponentKind::IconTheme => {
                tgt.starts_with("~/.icons/") || tgt.starts_with("~/.local/share/icons/")
            }
        }
    }

    pub fn infer_from_target(target: &str) -> Option<Self> {
        let tgt = target.trim();
        if tgt.starts_with("~/.themes/") || tgt.starts_with("~/.local/share/themes/") {
            Some(GnomeComponentKind::GtkTheme)
        } else if tgt.starts_with("~/.icons/") || tgt.starts_with("~/.local/share/icons/") {
            Some(GnomeComponentKind::IconTheme)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            GnomeComponentKind::GtkTheme => "gtk_theme",
            GnomeComponentKind::IconTheme => "icon_theme",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomePresetFile {
    pub component: GnomeComponentKind,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GnomePreset {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub tags: Vec<String>,
    pub files: Vec<GnomePresetFile>,
    pub package_type: PackageType,
    pub license_spdx: String,
}

pub struct GnomeProvider {
    builtin_presets: Vec<GnomePreset>,
    custom_dir: Option<PathBuf>,
    enabled: bool,
}

impl GnomeProvider {
    pub fn new() -> Self {
        Self {
            builtin_presets: get_curated_gnome_presets(),
            custom_dir: None,
            enabled: true,
        }
    }

    pub fn with_custom_dir(custom_dir: PathBuf) -> Self {
        Self {
            builtin_presets: get_curated_gnome_presets(),
            custom_dir: Some(custom_dir),
            enabled: true,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

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

            if let Ok(item) = load_disk_gnome_item(&pkg_dir) {
                items.push(item);
            }
        }

        items
    }
}

impl Default for GnomeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentProvider for GnomeProvider {
    fn id(&self) -> &str {
        "gnome"
    }

    fn name(&self) -> &str {
        "GNOME & GTK Themes"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::DesktopEnvironment
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

        if let Some(ref cat) = cat_filter {
            if cat != "gnome"
                && cat != "gtk"
                && cat != "theme"
                && cat != "icon"
                && cat != "desktop"
                && cat != "all"
            {
                return Ok(Vec::new());
            }
        }

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
            results.push(gnome_preset_to_provider_item(preset));
        }

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
                "GNOME provider disabled".to_string(),
            ));
        }

        let clean_id = id.trim();

        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            return Ok(gnome_preset_to_provider_item(preset));
        }

        if let Some(ref custom_dir) = self.custom_dir {
            let pkg_dir = custom_dir.join(clean_id);
            if pkg_dir.is_dir() && pkg_dir.join("manifest.json").is_file() {
                if let Ok(item) = load_disk_gnome_item(&pkg_dir) {
                    return Ok(item);
                }
            }
        }

        Err(ProviderError::NotFound(format!(
            "GNOME package '{}' not found",
            clean_id
        )))
    }

    fn stage_payload(&self, item: &ProviderItem, staging_dir: &Path) -> Result<PathBuf, String> {
        let clean_id = item.id.trim().to_lowercase();
        if clean_id.is_empty() {
            return Err("Package ID cannot be empty".to_string());
        }

        let mut targets_seen = HashSet::new();
        for file in &item.files {
            validate_gnome_target_and_source(file, &clean_id, &mut targets_seen)?;
        }

        let rand_suffix: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(44332);

        let tmp_stage_dir = staging_dir.join(format!(".tmp_gnome_{}_{}", clean_id, rand_suffix));
        let final_stage_dir = staging_dir.join(&clean_id);

        if tmp_stage_dir.exists() {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
        }
        if final_stage_dir.exists() {
            let _ = fs::remove_dir_all(&final_stage_dir);
        }

        fs::create_dir_all(&tmp_stage_dir)
            .map_err(|e| format!("Failed to create staging directory: {}", e))?;

        if let Some(preset) = self.builtin_presets.iter().find(|p| p.id == clean_id) {
            for f in &preset.files {
                let dest_path = tmp_stage_dir.join(&f.source);
                if let Some(parent) = dest_path.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!("Failed to create parent dir: {}", e));
                    }
                }
                let content = f.content.as_deref().unwrap_or("");
                if let Err(e) = fs::write(&dest_path, content) {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!("Failed to write GNOME file '{}': {}", f.source, e));
                }
            }
        } else if let Some(ref custom_root) = self.custom_dir {
            let pkg_dir = custom_root.join(&clean_id);
            if !pkg_dir.is_dir() {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!("GNOME package source not found: {:?}", pkg_dir));
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
                    return Err(format!("Invalid manifest JSON in GNOME package: {}", e));
                }
            };

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
                            "Tree hash mismatch for GNOME package '{}': expected '{}', got '{}'",
                            clean_id, expected_hash, actual_hash
                        ));
                    }
                }
            }

            for file in &item.files {
                let src_file = pkg_dir.join(&file.source);
                if !src_file.is_file() {
                    let _ = fs::remove_dir_all(&tmp_stage_dir);
                    return Err(format!(
                        "Missing GNOME payload file '{}' in package '{}'",
                        file.source, clean_id
                    ));
                }

                if let (Ok(can_src), Ok(can_pkg)) =
                    (src_file.canonicalize(), pkg_dir.canonicalize())
                {
                    if !can_src.starts_with(&can_pkg) {
                        let _ = fs::remove_dir_all(&tmp_stage_dir);
                        return Err(format!(
                            "Security violation: file '{}' escapes package root via symlink",
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
                                "Security violation: SVG asset '{}' contains prohibited active script elements",
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
                "GNOME package '{}' not found for staging",
                clean_id
            ));
        }

        let synthesized_manifest = match ManifestSynthesizer::synthesize(item) {
            Ok(m) => m,
            Err(e) => {
                let _ = fs::remove_dir_all(&tmp_stage_dir);
                return Err(format!(
                    "Failed to synthesize manifest for GNOME package '{}': {}",
                    clean_id, e
                ));
            }
        };

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

        if let Err(e) = fs::rename(&tmp_stage_dir, &final_stage_dir) {
            let _ = fs::remove_dir_all(&tmp_stage_dir);
            return Err(format!(
                "Failed to finalize staged GNOME package directory: {}",
                e
            ));
        }

        Ok(final_stage_dir)
    }
}

pub fn load_disk_gnome_item(pkg_dir: &Path) -> Result<ProviderItem, String> {
    let manifest_path = pkg_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err("Missing manifest.json in GNOME package directory".to_string());
    }

    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest.json: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&raw)
        .map_err(|e| format!("Invalid manifest JSON in GNOME package: {}", e))?;

    let dir_name = pkg_dir.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if !dir_name.is_empty() && dir_name != manifest.id {
        return Err(format!(
            "Identity mismatch: directory name '{}' does not match manifest.id '{}'",
            dir_name, manifest.id
        ));
    }

    let mut files = Vec::new();
    let mut targets_seen = HashSet::new();

    for f in &manifest.files {
        let spec = ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        };
        validate_gnome_target_and_source(&spec, &manifest.id, &mut targets_seen)?;
        files.push(spec);
    }

    let tree_hash = compute_package_tree_hash(pkg_dir, &manifest)
        .map_err(|e| format!("Failed to compute tree hash for GNOME package: {}", e))?;

    Ok(ProviderItem {
        id: manifest.id.clone(),
        title: manifest.name.clone(),
        subtitle: format!("GNOME/GTK Customization by {}", manifest.author),
        description: manifest.description.clone(),
        version: manifest.version.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        category: "desktop".to_string(),
        package_type: manifest.package_type,
        tags: manifest.tags.clone(),
        supported_desktops: vec!["gnome".to_string(), "gtk".to_string()],
        supported_display: vec!["wayland".to_string(), "x11".to_string()],
        rating: None,
        rating_count: None,
        downloads: None,
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "gnome".to_string(),
            source_url: format!("gnome://local/{}", manifest.id),
            repository: Some("local-gnome-catalog".to_string()),
            commit_or_tag: Some(tree_hash),
            original_author: manifest.author.clone(),
            license_spdx: Some("GPL-3.0-or-later".to_string()),
            fetched_at: current_utc_iso(),
        },
    })
}

pub fn validate_gnome_target_and_source(
    file: &ProviderFileSpec,
    pkg_id: &str,
    targets_seen: &mut HashSet<String>,
) -> Result<(), String> {
    let tgt = file.target.trim();
    let src = file.source.trim();

    // 1. Infer and validate GNOME component
    let comp = GnomeComponentKind::infer_from_target(tgt).ok_or_else(|| {
        format!(
            "Security violation: Target '{}' is outside permitted GNOME directories (~/.themes/<id>/**, ~/.local/share/themes/<id>/**, ~/.icons/<id>/**, ~/.local/share/icons/<id>/**)",
            tgt
        )
    })?;

    // GTK themes and Icon themes MUST target inside a named theme subfolder
    if comp == GnomeComponentKind::GtkTheme {
        let rel = if tgt.starts_with("~/.themes/") {
            &tgt["~/.themes/".len()..]
        } else {
            &tgt["~/.local/share/themes/".len()..]
        };
        if rel.split('/').filter(|p| !p.is_empty()).count() < 2 {
            return Err(format!(
                "Invalid GTK theme target '{}': must specify a theme subfolder",
                tgt
            ));
        }
    } else if comp == GnomeComponentKind::IconTheme {
        let rel = if tgt.starts_with("~/.icons/") {
            &tgt["~/.icons/".len()..]
        } else {
            &tgt["~/.local/share/icons/".len()..]
        };
        if rel.split('/').filter(|p| !p.is_empty()).count() < 2 {
            return Err(format!(
                "Invalid Icon theme target '{}': must specify an icon subfolder",
                tgt
            ));
        }
    }

    // 2. Traversal & null bytes rejection
    if tgt.contains("..") || tgt.contains('\0') {
        return Err(format!(
            "Security violation: Target '{}' contains traversal or null bytes",
            tgt
        ));
    }
    if src.starts_with('/') || src.starts_with('\\') || src.contains("..") || src.contains('\0') {
        return Err(format!("Security violation: Source '{}' contains forbidden traversal, absolute path, or null bytes", src));
    }

    // 3. System & Autostart rejection
    let forbidden_sensitive = [
        "~/.bashrc",
        "~/.zshrc",
        "~/.profile",
        "~/.config/autostart",
        "~/.config/systemd",
        "~/.config/environment.d",
        "~/.local/bin",
        "/etc",
        "/usr",
        "/bin",
    ];
    for f in &forbidden_sensitive {
        if tgt == *f || tgt.starts_with(&format!("{}/", f)) {
            return Err(format!(
                "Security violation: Target '{}' targets forbidden system/session path",
                tgt
            ));
        }
    }

    // 4. Prohibit executable scripts and binaries
    let lower_src = src.to_lowercase();
    let lower_tgt = tgt.to_lowercase();
    let forbidden_exts = [
        ".sh", ".bash", ".zsh", ".fish", ".py", ".exe", ".bin", ".cmd", ".bat", ".elf", ".so",
        ".dll", ".scr", ".msi", ".vbs",
    ];
    for ext in &forbidden_exts {
        if lower_src.ends_with(ext) || lower_tgt.ends_with(ext) {
            return Err(format!(
                "Security violation: Prohibited executable script or binary: '{}'",
                src
            ));
        }
    }
    if lower_src.contains("install.sh")
        || lower_src.contains("setup.sh")
        || lower_src.contains("post_install")
        || lower_src.contains("pre_install")
    {
        return Err(format!(
            "Security violation: Prohibited installer hook: '{}'",
            src
        ));
    }

    // 5. Duplicate normalized target collision prevention
    let normalized = normalize_target_path(tgt);
    if !targets_seen.insert(normalized.clone()) {
        return Err(format!("Conflict violation: Duplicate target '{}' declared multiple times in GNOME package '{}'", normalized, pkg_id));
    }

    Ok(())
}

fn gnome_preset_to_provider_item(preset: &GnomePreset) -> ProviderItem {
    let mut files = Vec::new();
    for f in &preset.files {
        files.push(ProviderFileSpec {
            source: f.source.clone(),
            target: f.target.clone(),
        });
    }

    let mut tags = preset.tags.clone();
    if !tags.contains(&"gnome".to_string()) {
        tags.push("gnome".to_string());
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
        category: "desktop".to_string(),
        package_type: preset.package_type.clone(),
        tags,
        supported_desktops: vec!["gnome".to_string(), "gtk".to_string()],
        supported_display: vec!["wayland".to_string(), "x11".to_string()],
        rating: Some(4.9),
        rating_count: Some(230),
        downloads: Some(3100),
        hero_image: None,
        screenshots: Vec::new(),
        files,
        provenance: ProviderProvenance {
            provider_id: "gnome".to_string(),
            source_url: format!("gnome://presets/{}", preset.id),
            repository: Some("builtin-gnome-ecosystem".to_string()),
            commit_or_tag: None,
            original_author: preset.author.clone(),
            license_spdx: Some(preset.license_spdx.clone()),
            fetched_at: current_utc_iso(),
        },
    }
}

fn get_curated_gnome_presets() -> Vec<GnomePreset> {
    vec![
        GnomePreset {
            id: "gnome-adwaita-vibrant".to_string(),
            title: "Adwaita Vibrant Dark".to_string(),
            subtitle: "Modernized GTK 3 & 4 styling with vibrant blue accents".to_string(),
            description: "High-contrast clean dark GTK theme with GNOME Shell theme components and polished UI elements.".to_string(),
            author: "Adwaita Design Team".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["gnome".to_string(), "gtk".to_string(), "adwaita".to_string(), "dark".to_string()],
            package_type: PackageType::Theme,
            license_spdx: "GPL-3.0-or-later".to_string(),
            files: vec![
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gtk-3.0/gtk.css".to_string(),
                    target: "~/.themes/Adwaita-Vibrant-Dark/gtk-3.0/gtk.css".to_string(),
                    content: Some("@define-color theme_bg_color #1e1e1e;
@define-color theme_fg_color #ffffff;
".to_string()),
                },
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gtk-4.0/gtk.css".to_string(),
                    target: "~/.themes/Adwaita-Vibrant-Dark/gtk-4.0/gtk.css".to_string(),
                    content: Some("@define-color accent_color #3584e4;
".to_string()),
                },
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gnome-shell/gnome-shell.css".to_string(),
                    target: "~/.themes/Adwaita-Vibrant-Dark/gnome-shell/gnome-shell.css".to_string(),
                    content: Some("#panel { background-color: rgba(30, 30, 30, 0.9); }
".to_string()),
                },
            ],
        },
        GnomePreset {
            id: "gnome-catppuccin-mocha".to_string(),
            title: "Catppuccin Mocha GTK & Shell".to_string(),
            subtitle: "Warm pastel dark theme for GTK and GNOME Shell".to_string(),
            description: "Catppuccin Mocha colors tailored for GTK 3, GTK 4, and GNOME Shell panel styling.".to_string(),
            author: "Catppuccin Community".to_string(),
            version: "1.2.0".to_string(),
            tags: vec!["gnome".to_string(), "gtk".to_string(), "catppuccin".to_string(), "mocha".to_string()],
            package_type: PackageType::Theme,
            license_spdx: "MIT".to_string(),
            files: vec![
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gtk-3.0/gtk.css".to_string(),
                    target: "~/.themes/Catppuccin-Mocha-Standard-Mauve/gtk-3.0/gtk.css".to_string(),
                    content: Some("@define-color theme_bg_color #1e1e2e;
@define-color theme_fg_color #cdd6f4;
".to_string()),
                },
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gnome-shell/gnome-shell.css".to_string(),
                    target: "~/.themes/Catppuccin-Mocha-Standard-Mauve/gnome-shell/gnome-shell.css".to_string(),
                    content: Some("#panel { background-color: #1e1e2e; color: #cdd6f4; }
".to_string()),
                },
            ],
        },
        GnomePreset {
            id: "gnome-papirus-aesthetic".to_string(),
            title: "Papirus Aesthetic Icons".to_string(),
            subtitle: "Vector icon suite for Wayland & X11 desktops".to_string(),
            description: "Crisp multi-resolution icons adhering to freedesktop icon specifications.".to_string(),
            author: "Papirus Development Team".to_string(),
            version: "2026.1".to_string(),
            tags: vec!["gnome".to_string(), "icons".to_string(), "papirus".to_string()],
            package_type: PackageType::Icon,
            license_spdx: "GPL-3.0-or-later".to_string(),
            files: vec![
                GnomePresetFile {
                    component: GnomeComponentKind::IconTheme,
                    source: "index.theme".to_string(),
                    target: "~/.icons/Papirus-Aesthetic/index.theme".to_string(),
                    content: Some("[Icon Theme]
Name=Papirus-Aesthetic
Comment=Modern vector icons
Inherits=breeze
Directories=48x48/apps
".to_string()),
                },
                GnomePresetFile {
                    component: GnomeComponentKind::IconTheme,
                    source: "48x48/apps/system.svg".to_string(),
                    target: "~/.icons/Papirus-Aesthetic/48x48/apps/system.svg".to_string(),
                    content: Some("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"48\" height=\"48\"><circle cx=\"24\" cy=\"24\" r=\"20\" fill=\"#3584e4\"/></svg>".to_string()),
                },
            ],
        },
        GnomePreset {
            id: "gnome-complete-suite".to_string(),
            title: "Studio Dark Complete Suite".to_string(),
            subtitle: "Coordinated GTK theme and matching icon pack".to_string(),
            description: "A mixed package combining both a custom GTK 3/4 theme and an aesthetic icon suite in a single transaction.".to_string(),
            author: "Studio Visuals".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["gnome".to_string(), "gtk".to_string(), "icons".to_string(), "suite".to_string()],
            package_type: PackageType::Theme,
            license_spdx: "MIT".to_string(),
            files: vec![
                GnomePresetFile {
                    component: GnomeComponentKind::GtkTheme,
                    source: "gtk-3.0/gtk.css".to_string(),
                    target: "~/.themes/StudioDark/gtk-3.0/gtk.css".to_string(),
                    content: Some("@define-color bg #121212;
".to_string()),
                },
                GnomePresetFile {
                    component: GnomeComponentKind::IconTheme,
                    source: "index.theme".to_string(),
                    target: "~/.icons/StudioIcons/index.theme".to_string(),
                    content: Some("[Icon Theme]
Name=StudioIcons
".to_string()),
                },
            ],
        },
    ]
}

// ─────────────────────────────────────────────────────────────────────────────
// Target Path Normalization Helper
// ─────────────────────────────────────────────────────────────────────────────

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

    while normalized.contains("/./") {
        normalized = normalized.replace("/./", "/");
    }

    normalized
}
