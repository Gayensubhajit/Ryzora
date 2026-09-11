//! Ryzora Dynamic Arch Linux Repository Catalog Engine — Phase 24
//!
//! Provides zero-subprocess, dynamic discovery, AppStream metadata correlation,
//! package classification, and indexed caching across configured official Arch
//! Linux Pacman repositories (`/etc/pacman.conf`).
//!
//! # Core Invariants
//! - Respects `/etc/pacman.conf` enabled repositories (no assumed hardcoded list).
//! - Zero subprocesses: parses ALPM `.db` archives via `flate2` + `tar`, and AppStream XML via streaming readers.
//! - Clear separation: Desktop Applications vs All Packages.
//! - Authoritative ALPM installed state query from `/var/lib/pacman/local/`.
//! - Asynchronous, cached, non-blocking: instant startup from disk cache, non-blocking background refresh.
//! - Strict security: No AUR, yay, paru, Flatpak, or fake package actions.

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_PACMAN_CONF: &str = "/etc/pacman.conf";
pub const DEFAULT_PACMAN_SYNC_DIR: &str = "/var/lib/pacman/sync";
pub const DEFAULT_PACMAN_LOCAL_DIR: &str = "/var/lib/pacman/local";
pub const DEFAULT_APPDATA_XML_DIR: &str = "/usr/share/swcatalog/xml";
pub const DEFAULT_APPDATA_ICONS_DIR: &str = "/usr/share/swcatalog/icons";
pub const DEFAULT_DESKTOP_DIR: &str = "/usr/share/applications";

// ─────────────────────────────────────────────────────────────────────────────
// Data Models
// ─────────────────────────────────────────────────────────────────────────────

/// Full normalized catalog item for the store and package browser.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogItem {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub description: String,
    pub repository: String,
    pub version: String,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub is_application: bool,
    pub category: String,
    pub subcategories: Vec<String>,
    pub icon_name: Option<String>,
    pub icon_path: Option<String>,
    pub launchable: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub download_size: Option<u64>,
    pub installed_size: Option<u64>,
    pub dependencies: Vec<String>,
    pub screenshots: Vec<String>,
    pub developer: Option<String>,
    pub metadata_source: String, // "appstream" | "desktop_entry" | "pacman_sync"
}

/// Status summary of the catalog engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CatalogStatus {
    pub total_packages: usize,
    pub total_applications: usize,
    #[serde(default)]
    pub total_installed_applications: usize,
    #[serde(default)]
    pub total_installed_packages: usize,
    pub enabled_repositories: Vec<String>,
    pub last_refreshed: u64,
    pub is_refreshing: bool,
    pub error: Option<String>,
}

/// Request parameters for querying catalog items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogQueryFilter {
    pub is_application_only: Option<bool>,
    pub category: Option<String>,
    pub search_query: Option<String>,
    pub repository: Option<String>,
    pub is_installed_only: Option<bool>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

/// Paginated response for catalog items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogPageResponse {
    pub items: Vec<CatalogItem>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
}

/// Intermediate raw AppStream metadata entry.
#[derive(Debug, Clone, Default)]
pub struct RawAppStreamComponent {
    pub id: String,
    pub pkgname: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub icon_cached: Option<String>,
    pub icon_stock: Option<String>,
    pub launchable: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub categories: Vec<String>,
    pub screenshots: Vec<String>,
    pub developer: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Resolvers (Sandbox-Aware)
// ─────────────────────────────────────────────────────────────────────────────

pub fn resolve_pacman_conf_path(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("etc/pacman.conf");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_PACMAN_CONF)
}

pub fn resolve_sync_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("var/lib/pacman/sync");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_PACMAN_SYNC_DIR)
}

pub fn resolve_local_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("var/lib/pacman/local");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_PACMAN_LOCAL_DIR)
}

pub fn resolve_appstream_xml_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("usr/share/swcatalog/xml");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_APPDATA_XML_DIR)
}

pub fn resolve_appstream_icons_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("usr/share/swcatalog/icons");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_APPDATA_ICONS_DIR)
}

pub fn resolve_desktop_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("usr/share/applications");
        if candidate.exists() {
            return candidate;
        }
    }
    PathBuf::from(DEFAULT_DESKTOP_DIR)
}

pub fn resolve_cache_file_path() -> PathBuf {
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let candidate = PathBuf::from(&sys_root).join("cache/ryzora/arch_catalog_v1.json");
        if candidate.is_file() {
            return candidate;
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("ryzora/arch_catalog_v1.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache/ryzora/arch_catalog_v1.json");
    }
    PathBuf::from("/tmp/ryzora_arch_catalog_v1.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository Configuration Discovery
// ─────────────────────────────────────────────────────────────────────────────

/// Discovers active repository names from `/etc/pacman.conf`.
pub fn parse_pacman_conf_repositories(conf_path: &Path) -> Vec<String> {
    if !conf_path.exists() {
        return vec![
            "core".to_string(),
            "extra".to_string(),
            "multilib".to_string(),
        ];
    }

    let mut repos = Vec::new();
    if let Ok(file) = File::open(conf_path) {
        let reader = BufReader::new(file);
        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let section = trimmed.trim_matches(|c| c == '[' || c == ']').trim();
                if !section.is_empty() && section != "options" && !repos.contains(&section.to_string()) {
                    repos.push(section.to_string());
                }
            }
        }
    }

    if repos.is_empty() {
        vec![
            "core".to_string(),
            "extra".to_string(),
            "multilib".to_string(),
        ]
    } else {
        repos
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AppStream XML Metadata Parser (Streaming & Non-Blocking)
// ─────────────────────────────────────────────────────────────────────────────

/// Parses AppStream catalog XML (.xml.gz) files from the swcatalog directory.
/// Returns a map of `pkgname` -> `RawAppStreamComponent`.
pub fn parse_appstream_catalogs(xml_dir: &Path) -> HashMap<String, RawAppStreamComponent> {
    let mut components = HashMap::new();
    if !xml_dir.exists() {
        return components;
    }

    let entries = match fs::read_dir(xml_dir) {
        Ok(e) => e,
        Err(_) => return components,
    };

    for entry_res in entries.flatten() {
        let path = entry_res.path();
        if path.is_file() {
            let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            if filename.ends_with(".xml.gz") {
                if let Ok(file) = File::open(&path) {
                    let gz = GzDecoder::new(file);
                    parse_appstream_xml_stream(gz, &mut components);
                }
            } else if filename.ends_with(".xml") {
                if let Ok(file) = File::open(&path) {
                    parse_appstream_xml_stream(file, &mut components);
                }
            }
        }
    }

    components
}

/// Lightweight streaming parser for AppStream `<component type="...">` entries.
fn parse_appstream_xml_stream<R: Read>(reader: R, output: &mut HashMap<String, RawAppStreamComponent>) {
    let buf_reader = BufReader::new(reader);
    let mut current_comp: Option<RawAppStreamComponent> = None;
    let mut is_desktop_app = false;
    let mut inside_description = false;
    let mut inside_developer = false;
    let mut description_accum = String::new();

    for line_res in buf_reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
        };
        let trimmed = line.trim();

        if trimmed.starts_with("<component") {
            is_desktop_app = trimmed.contains("type=\"desktop-application\"");
            current_comp = Some(RawAppStreamComponent::default());
            inside_description = false;
            inside_developer = false;
            description_accum.clear();
            continue;
        }

        if trimmed.starts_with("</component>") {
            if let Some(comp) = current_comp.take() {
                if is_desktop_app && !comp.pkgname.is_empty() {
                    output.insert(comp.pkgname.clone(), comp);
                }
            }
            inside_developer = false;
            continue;
        }

        let comp = match current_comp.as_mut() {
            Some(c) => c,
            None => continue,
        };

        // Track <developer> block
        if trimmed.starts_with("<developer") {
            inside_developer = true;
        }
        if trimmed.starts_with("</developer>") {
            inside_developer = false;
        }

        // Track <description> block
        if trimmed.starts_with("<description>") {
            inside_description = true;
            description_accum.clear();
            continue;
        }
        if trimmed.starts_with("</description>") {
            inside_description = false;
            comp.description = clean_xml_tags(&description_accum);
            continue;
        }
        if inside_description {
            description_accum.push_str(trimmed);
            description_accum.push(' ');
            continue;
        }

        // Tags parsing
        if trimmed.starts_with("<pkgname>") && trimmed.ends_with("</pkgname>") {
            comp.pkgname = extract_tag_content(trimmed, "pkgname");
        } else if !inside_developer && trimmed.starts_with("<name>") && trimmed.ends_with("</name>") && comp.name.is_empty() {
            comp.name = extract_tag_content(trimmed, "name");
        } else if inside_developer && trimmed.starts_with("<name>") && trimmed.ends_with("</name>") {
            let dev = extract_tag_content(trimmed, "name");
            if !dev.is_empty() && comp.developer.is_none() {
                comp.developer = Some(dev);
            }
        } else if trimmed.starts_with("<summary>") && trimmed.ends_with("</summary>") {
            comp.summary = extract_tag_content(trimmed, "summary");
        } else if trimmed.starts_with("<id>") && trimmed.ends_with("</id>") {
            comp.id = extract_tag_content(trimmed, "id");
        } else if trimmed.starts_with("<project_license>") && trimmed.ends_with("</project_license>") {
            comp.license = Some(extract_tag_content(trimmed, "project_license"));
        } else if trimmed.starts_with("<url type=\"homepage\">") && trimmed.ends_with("</url>") {
            comp.homepage = Some(extract_tag_content(trimmed, "url"));
        } else if trimmed.starts_with("<launchable type=\"desktop-id\">") && trimmed.ends_with("</launchable>") {
            comp.launchable = Some(extract_tag_content(trimmed, "launchable"));
        } else if trimmed.starts_with("<category>") && trimmed.ends_with("</category>") {
            let cat = extract_tag_content(trimmed, "category");
            if !cat.is_empty() {
                comp.categories.push(cat);
            }
        } else if trimmed.starts_with("<icon type=\"cached\"") && trimmed.ends_with("</icon>") {
            let icon = extract_tag_content(trimmed, "icon");
            if !icon.is_empty() && comp.icon_cached.is_none() {
                comp.icon_cached = Some(icon);
            }
        } else if trimmed.starts_with("<icon type=\"stock\">") && trimmed.ends_with("</icon>") {
            let icon = extract_tag_content(trimmed, "icon");
            if !icon.is_empty() && comp.icon_stock.is_none() {
                comp.icon_stock = Some(icon);
            }
        } else if trimmed.contains("<image type=\"source\">") {
            let url = extract_tag_content(trimmed, "image");
            if !url.is_empty() && comp.screenshots.len() < 5 {
                comp.screenshots.push(url);
            }
        } else if (trimmed.starts_with("<developer_name>") || trimmed.starts_with("<developer><name>"))
            && (trimmed.ends_with("</developer_name>") || trimmed.ends_with("</name></developer>"))
        {
            let dev = clean_xml_tags(trimmed);
            if !dev.is_empty() {
                comp.developer = Some(dev);
            }
        }
    }
}

fn extract_tag_content(line: &str, tag: &str) -> String {
    let start_pat = format!("<{}", tag);
    let end_pat = format!("</{}>", tag);

    if let Some(start_idx) = line.find(&start_pat) {
        if let Some(close_angle) = line[start_idx..].find('>') {
            let content_start = start_idx + close_angle + 1;
            if let Some(end_idx) = line[content_start..].find(&end_pat) {
                return line[content_start..content_start + end_idx].trim().to_string();
            }
        }
    }
    String::new()
}

fn clean_xml_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut inside_tag = false;
    for ch in input.chars() {
        if ch == '<' {
            inside_tag = true;
        } else if ch == '>' {
            inside_tag = false;
        } else if !inside_tag {
            out.push(ch);
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop Entry File Discovery (.desktop files)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct RawDesktopEntry {
    pub name: String,
    pub generic_name: Option<String>,
    pub comment: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub categories: Vec<String>,
    pub no_display: bool,
}

pub fn parse_system_desktop_entries(desktop_dir: &Path) -> HashMap<String, RawDesktopEntry> {
    let mut entries_map = HashMap::new();
    if !desktop_dir.exists() {
        return entries_map;
    }

    if let Ok(entries) = fs::read_dir(desktop_dir) {
        for entry_res in entries.flatten() {
            let path = entry_res.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "desktop") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(parsed) = parse_single_desktop_file(&content) {
                        if !parsed.no_display {
                            entries_map.insert(stem.to_lowercase(), parsed);
                        }
                    }
                }
            }
        }
    }

    entries_map
}

fn parse_single_desktop_file(content: &str) -> Option<RawDesktopEntry> {
    let mut in_desktop_entry = false;
    let mut is_application = false;
    let mut name = String::new();
    let mut generic_name = None;
    let mut comment = None;
    let mut exec = None;
    let mut icon = None;
    let mut categories = Vec::new();
    let mut no_display = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, val)) = trimmed.split_once('=') {
            let k = key.trim();
            let v = val.trim();
            match k {
                "Type" => {
                    if v == "Application" {
                        is_application = true;
                    }
                }
                "Name" if name.is_empty() => name = v.to_string(),
                "GenericName" if generic_name.is_none() => generic_name = Some(v.to_string()),
                "Comment" if comment.is_none() => comment = Some(v.to_string()),
                "Exec" if exec.is_none() => exec = Some(v.to_string()),
                "Icon" if icon.is_none() => icon = Some(v.to_string()),
                "NoDisplay" => no_display = v.eq_ignore_ascii_case("true"),
                "Categories" => {
                    categories = v
                        .split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    if is_application && !name.is_empty() {
        Some(RawDesktopEntry {
            name,
            generic_name,
            comment,
            exec,
            icon,
            categories,
            no_display,
        })
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Category Normalization & Generic Package Classification
// ─────────────────────────────────────────────────────────────────────────────

/// Maps FreeDesktop / AppStream categories into Ryzora store categories.
pub fn normalize_app_category(categories: &[String]) -> (String, Vec<String>) {
    let mut primary = "Utilities".to_string();
    let mut subcats = Vec::new();

    for cat in categories {
        let c_lower = cat.to_lowercase();
        subcats.push(cat.clone());

        if c_lower.contains("graphics") || c_lower.contains("2dgraphics") || c_lower.contains("3dgraphics") || c_lower.contains("rastergraphics") || c_lower.contains("vectorgraphics") {
            primary = "Graphics".to_string();
        } else if c_lower.contains("audiovideo") || c_lower.contains("audio") || c_lower.contains("video") || c_lower.contains("music") || c_lower.contains("player") || c_lower.contains("recorder") {
            primary = "Multimedia".to_string();
        } else if c_lower.contains("development") || c_lower.contains("ide") || c_lower.contains("debugger") || c_lower.contains("texteditor") {
            primary = "Development".to_string();
        } else if c_lower.contains("network") || c_lower.contains("webbrowser") || c_lower.contains("email") || c_lower.contains("chat") || c_lower.contains("ircclient") || c_lower.contains("feed") {
            primary = "Internet".to_string();
        } else if c_lower.contains("game") || c_lower.contains("arcade") || c_lower.contains("actiongame") || c_lower.contains("adventuregame") || c_lower.contains("simulation") {
            primary = "Games".to_string();
        } else if c_lower.contains("office") || c_lower.contains("wordprocessor") || c_lower.contains("spreadsheet") || c_lower.contains("presentation") {
            primary = "Office".to_string();
        } else if c_lower.contains("system") || c_lower.contains("settings") || c_lower.contains("hardware") || c_lower.contains("monitor") {
            primary = "System".to_string();
        } else if c_lower.contains("education") || c_lower.contains("science") {
            primary = "Education".to_string();
        }
    }

    (primary, subcats)
}

/// Classifies non-desktop packages into clear categories (CLI, Libraries, Fonts, Plugins, System, etc.).
pub fn classify_generic_package(name: &str, desc: &str, _deps: &[String]) -> String {
    let n = name.to_lowercase();
    let d = desc.to_lowercase();

    if n.starts_with("lib")
        || n.starts_with("python-")
        || n.starts_with("perl-")
        || n.starts_with("ruby-")
        || n.starts_with("haskell-")
        || n.starts_with("rust-")
        || n.starts_with("go-")
        || n.starts_with("php-")
        || n.starts_with("lua-")
        || n.starts_with("qt5-")
        || n.starts_with("qt6-")
        || n.starts_with("kf5-")
        || n.starts_with("kf6-")
        || n.ends_with("-libs")
    {
        "Libraries".to_string()
    } else if n.starts_with("ttf-") || n.starts_with("otf-") || n.contains("font") {
        "Fonts".to_string()
    } else if n.contains("-plugin") || n.contains("-addon") || n.contains("-extension") || n.contains("theme") {
        "Plugins".to_string()
    } else if n.starts_with("linux")
        || n.contains("kernel")
        || n.contains("firmware")
        || n.starts_with("systemd")
        || n.starts_with("xf86-")
        || n.starts_with("vulkan-")
        || n.starts_with("mesa")
        || n.starts_with("nvidia")
        || n.contains("bootloader")
        || n.contains("dracut")
        || n.contains("grub")
    {
        "System Components".to_string()
    } else if n.ends_with("-devel")
        || n.contains("compiler")
        || n.contains("toolchain")
        || n.contains("debugger")
        || n == "git"
        || n == "cmake"
        || n == "meson"
    {
        "Development Tools".to_string()
    } else if d.contains("command line")
        || d.contains("cli")
        || d.contains("terminal")
        || d.contains("utility")
        || d.contains("console")
    {
        "CLI Tools".to_string()
    } else {
        "Other Packages".to_string()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Sync Database Parsing
// ─────────────────────────────────────────────────────────────────────────────

pub struct RawSyncPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub url: Option<String>,
    pub license: Option<String>,
    pub download_size: Option<u64>,
    pub installed_size: Option<u64>,
    pub dependencies: Vec<String>,
}

/// Parses all `.db` tar.gz archives in `sync_dir` for configured repositories.
pub fn parse_pacman_sync_databases(
    sync_dir: &Path,
    enabled_repos: &[String],
) -> Result<Vec<RawSyncPackage>, String> {
    if !sync_dir.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();
    let mut seen_names = HashSet::new();

    for repo_name in enabled_repos {
        let db_path = sync_dir.join(format!("{}.db", repo_name));
        if !db_path.is_file() {
            continue;
        }

        let file = match File::open(&db_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let gz = GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);

        let entries = match archive.entries() {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry_res in entries {
            let mut entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path_buf = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            };

            if !path_buf.to_string_lossy().ends_with("/desc") {
                continue;
            }

            let mut content = String::new();
            if entry.read_to_string(&mut content).is_err() {
                continue;
            }

            if let Some(raw) = parse_alpm_desc_to_sync_pkg(&content, repo_name) {
                if !seen_names.contains(&raw.name) {
                    seen_names.insert(raw.name.clone());
                    packages.push(raw);
                }
            }
        }
    }

    Ok(packages)
}

fn parse_alpm_desc_to_sync_pkg(content: &str, repo_name: &str) -> Option<RawSyncPackage> {
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut url = None;
    let mut license = None;
    let mut download_size = None;
    let mut installed_size = None;
    let mut dependencies = Vec::new();

    let mut current_key = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') && trimmed.ends_with('%') {
            current_key = trimmed.trim_matches('%');
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }

        match current_key {
            "NAME" if name.is_empty() => name = trimmed.to_string(),
            "VERSION" if version.is_empty() => version = trimmed.to_string(),
            "DESC" if description.is_empty() => description = trimmed.to_string(),
            "URL" if url.is_none() => url = Some(trimmed.to_string()),
            "LICENSE" if license.is_none() => license = Some(trimmed.to_string()),
            "CSIZE" if download_size.is_none() => {
                if let Ok(sz) = trimmed.parse::<u64>() {
                    download_size = Some(sz);
                }
            }
            "ISIZE" if installed_size.is_none() => {
                if let Ok(sz) = trimmed.parse::<u64>() {
                    installed_size = Some(sz);
                }
            }
            "DEPENDS" => dependencies.push(trimmed.to_string()),
            _ => {}
        }
    }

    if name.is_empty() || version.is_empty() {
        return None;
    }

    Some(RawSyncPackage {
        name,
        version,
        description,
        repository: repo_name.to_string(),
        url,
        license,
        download_size,
        installed_size,
        dependencies,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Local Installed State
// ─────────────────────────────────────────────────────────────────────────────

/// Scans `/var/lib/pacman/local/` and builds a map of installed package names -> version.
pub fn scan_alpm_installed_packages(local_dir: &Path) -> HashMap<String, String> {
    let mut installed = HashMap::new();
    if !local_dir.exists() {
        return installed;
    }

    if let Ok(entries) = fs::read_dir(local_dir) {
        for entry_res in entries.flatten() {
            let path = entry_res.path();
            if path.is_dir() {
                let dir_name = entry_res.file_name().to_string_lossy().to_string();
                let desc_path = path.join("desc");
                if desc_path.is_file() {
                    if let Ok(content) = fs::read_to_string(&desc_path) {
                        let mut name = String::new();
                        let mut version = String::new();
                        let mut current_key = "";
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with('%') && trimmed.ends_with('%') {
                                current_key = trimmed.trim_matches('%');
                                continue;
                            }
                            if trimmed.is_empty() {
                                continue;
                            }
                            match current_key {
                                "NAME" if name.is_empty() => name = trimmed.to_string(),
                                "VERSION" if version.is_empty() => version = trimmed.to_string(),
                                _ => {}
                            }
                            if !name.is_empty() && !version.is_empty() {
                                break;
                            }
                        }
                        if !name.is_empty() && !version.is_empty() {
                            installed.insert(name.to_lowercase(), version);
                            continue;
                        }
                    }
                }

                if let Some(last_dash) = dir_name.rfind('-') {
                    if let Some(second_last_dash) = dir_name[..last_dash].rfind('-') {
                        let pkg = &dir_name[..second_last_dash];
                        let ver = &dir_name[second_last_dash + 1..];
                        if !pkg.is_empty() {
                            installed.insert(pkg.to_lowercase(), ver.to_string());
                        }
                    }
                }
            }
        }
    }

    installed
}

// ─────────────────────────────────────────────────────────────────────────────
// Icon Resolution from AppStream Cached Theme Dirs
// ─────────────────────────────────────────────────────────────────────────────

fn find_appstream_cached_icon(icon_filename: &str, icons_dir: &Path) -> Option<String> {
    if !icons_dir.exists() {
        return None;
    }

    let subdirs = ["archlinux-arch-extra", "archlinux-arch-core", "archlinux-arch-multilib"];
    let sizes = ["128x128", "64x64", "48x48"];

    for sub in subdirs {
        for sz in sizes {
            let candidate = icons_dir.join(sub).join(sz).join(icon_filename);
            if candidate.is_file() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Full Catalog Construction Pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Builds the complete dynamic catalog from pacman.conf, sync DBs, AppStream, and local ALPM.
pub fn build_dynamic_catalog(
    conf_path: &Path,
    sync_dir: &Path,
    local_dir: &Path,
    appstream_dir: &Path,
    appstream_icons_dir: &Path,
    desktop_dir: &Path,
) -> Result<(Vec<CatalogItem>, CatalogStatus), String> {
    let enabled_repos = parse_pacman_conf_repositories(conf_path);
    let appstream_map = parse_appstream_catalogs(appstream_dir);
    let desktop_map = parse_system_desktop_entries(desktop_dir);
    let installed_map = scan_alpm_installed_packages(local_dir);
    let sync_pkgs = parse_pacman_sync_databases(sync_dir, &enabled_repos)?;

    let mut catalog_items = Vec::with_capacity(sync_pkgs.len() + 200);
    let mut seen_ids = HashSet::new();
    let mut total_apps = 0;

    for raw in sync_pkgs {
        let pkg_id_lower = raw.name.to_lowercase();
        seen_ids.insert(pkg_id_lower.clone());
        let is_installed = installed_map.contains_key(&pkg_id_lower);
        let installed_version = installed_map.get(&pkg_id_lower).cloned();

        // 1. Check if AppStream metadata exists
        if let Some(appstream) = appstream_map.get(&raw.name) {
            total_apps += 1;
            let (primary_cat, subcats) = normalize_app_category(&appstream.categories);

            let icon_path = appstream
                .icon_cached
                .as_deref()
                .and_then(|f| find_appstream_cached_icon(f, appstream_icons_dir));

            catalog_items.push(CatalogItem {
                id: raw.name.clone(),
                display_name: if !appstream.name.is_empty() {
                    appstream.name.clone()
                } else {
                    capitalize_pkg_name(&raw.name)
                },
                summary: if !appstream.summary.is_empty() {
                    appstream.summary.clone()
                } else {
                    raw.description.clone()
                },
                description: if !appstream.description.is_empty() {
                    appstream.description.clone()
                } else {
                    raw.description.clone()
                },
                repository: raw.repository,
                version: raw.version,
                is_installed,
                installed_version,
                is_application: true,
                category: primary_cat,
                subcategories: subcats,
                icon_name: appstream.icon_stock.clone().or_else(|| appstream.icon_cached.clone()),
                icon_path,
                launchable: appstream.launchable.clone(),
                homepage: appstream.homepage.clone().or(raw.url),
                license: appstream.license.clone().or(raw.license),
                download_size: raw.download_size,
                installed_size: raw.installed_size,
                dependencies: raw.dependencies,
                screenshots: appstream.screenshots.clone(),
                developer: appstream.developer.clone(),
                metadata_source: "appstream".to_string(),
            });
            continue;
        }

        // 2. Check if a valid desktop entry matches
        if let Some(desktop) = desktop_map.get(&pkg_id_lower) {
            total_apps += 1;
            let (primary_cat, subcats) = normalize_app_category(&desktop.categories);

            catalog_items.push(CatalogItem {
                id: raw.name.clone(),
                display_name: if !desktop.name.is_empty() {
                    desktop.name.clone()
                } else {
                    capitalize_pkg_name(&raw.name)
                },
                summary: desktop.comment.clone().unwrap_or_else(|| raw.description.clone()),
                description: raw.description.clone(),
                repository: raw.repository,
                version: raw.version,
                is_installed,
                installed_version,
                is_application: true,
                category: primary_cat,
                subcategories: subcats,
                icon_name: desktop.icon.clone(),
                icon_path: None,
                launchable: Some(format!("{}.desktop", pkg_id_lower)),
                homepage: raw.url,
                license: raw.license,
                download_size: raw.download_size,
                installed_size: raw.installed_size,
                dependencies: raw.dependencies,
                screenshots: Vec::new(),
                developer: None,
                metadata_source: "desktop_entry".to_string(),
            });
            continue;
        }

        // 3. Generic Pacman package
        let generic_cat = classify_generic_package(&raw.name, &raw.description, &raw.dependencies);
        catalog_items.push(CatalogItem {
            id: raw.name.clone(),
            display_name: raw.name.clone(),
            summary: raw.description.clone(),
            description: raw.description.clone(),
            repository: raw.repository,
            version: raw.version,
            is_installed,
            installed_version,
            is_application: false,
            category: generic_cat,
            subcategories: Vec::new(),
            icon_name: None,
            icon_path: None,
            launchable: None,
            homepage: raw.url,
            license: raw.license,
            download_size: raw.download_size,
            installed_size: raw.installed_size,
            dependencies: raw.dependencies,
            screenshots: Vec::new(),
            developer: None,
            metadata_source: "pacman_sync".to_string(),
        });
    }

    // Include locally installed packages not in sync_pkgs (e.g. local / AUR packages with desktop entries)
    for (inst_pkg, inst_ver) in &installed_map {
        let pkg_lower = inst_pkg.to_lowercase();
        if seen_ids.contains(&pkg_lower) {
            continue;
        }
        seen_ids.insert(pkg_lower.clone());

        // Read local desc file
        let desc_path = local_dir.join(format!("{}-{}/desc", inst_pkg, inst_ver));
        let mut desc_text = String::new();
        let mut url = None;
        let mut license = None;
        let mut installed_size = None;
        if let Ok(desc_c) = fs::read_to_string(&desc_path) {
            let mut current_key = "";
            for line in desc_c.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('%') && trimmed.ends_with('%') {
                    current_key = trimmed.trim_matches('%');
                    continue;
                }
                if trimmed.is_empty() {
                    continue;
                }
                match current_key {
                    "DESC" if desc_text.is_empty() => desc_text = trimmed.to_string(),
                    "URL" if url.is_none() => url = Some(trimmed.to_string()),
                    "LICENSE" if license.is_none() => license = Some(trimmed.to_string()),
                    "ISIZE" if installed_size.is_none() => {
                        if let Ok(sz) = trimmed.parse::<u64>() {
                            installed_size = Some(sz);
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(desktop) = desktop_map.get(&pkg_lower) {
            total_apps += 1;
            let (primary_cat, subcats) = normalize_app_category(&desktop.categories);
            catalog_items.push(CatalogItem {
                id: inst_pkg.clone(),
                display_name: if !desktop.name.is_empty() { desktop.name.clone() } else { capitalize_pkg_name(inst_pkg) },
                summary: desktop.comment.clone().unwrap_or_else(|| desc_text.clone()),
                description: desc_text,
                repository: "local".to_string(),
                version: inst_ver.clone(),
                is_installed: true,
                installed_version: Some(inst_ver.clone()),
                is_application: true,
                category: primary_cat,
                subcategories: subcats,
                icon_name: desktop.icon.clone(),
                icon_path: None,
                launchable: Some(format!("{}.desktop", pkg_lower)),
                homepage: url,
                license,
                download_size: None,
                installed_size,
                dependencies: Vec::new(),
                screenshots: Vec::new(),
                developer: None,
                metadata_source: "desktop_entry".to_string(),
            });
        } else if let Some(appstream) = appstream_map.get(inst_pkg) {
            total_apps += 1;
            let (primary_cat, subcats) = normalize_app_category(&appstream.categories);
            let icon_path = appstream.icon_cached.as_deref().and_then(|f| find_appstream_cached_icon(f, appstream_icons_dir));
            catalog_items.push(CatalogItem {
                id: inst_pkg.clone(),
                display_name: if !appstream.name.is_empty() { appstream.name.clone() } else { capitalize_pkg_name(inst_pkg) },
                summary: if !appstream.summary.is_empty() { appstream.summary.clone() } else { desc_text.clone() },
                description: if !appstream.description.is_empty() { appstream.description.clone() } else { desc_text },
                repository: "local".to_string(),
                version: inst_ver.clone(),
                is_installed: true,
                installed_version: Some(inst_ver.clone()),
                is_application: true,
                category: primary_cat,
                subcategories: subcats,
                icon_name: appstream.icon_stock.clone().or_else(|| appstream.icon_cached.clone()),
                icon_path,
                launchable: appstream.launchable.clone(),
                homepage: appstream.homepage.clone().or(url),
                license: appstream.license.clone().or(license),
                download_size: None,
                installed_size,
                dependencies: Vec::new(),
                screenshots: appstream.screenshots.clone(),
                developer: appstream.developer.clone(),
                metadata_source: "appstream".to_string(),
            });
        } else {
            let generic_cat = classify_generic_package(inst_pkg, &desc_text, &[]);
            catalog_items.push(CatalogItem {
                id: inst_pkg.clone(),
                display_name: inst_pkg.clone(),
                summary: desc_text.clone(),
                description: desc_text,
                repository: "local".to_string(),
                version: inst_ver.clone(),
                is_installed: true,
                installed_version: Some(inst_ver.clone()),
                is_application: false,
                category: generic_cat,
                subcategories: Vec::new(),
                icon_name: None,
                icon_path: None,
                launchable: None,
                homepage: url,
                license,
                download_size: None,
                installed_size,
                dependencies: Vec::new(),
                screenshots: Vec::new(),
                developer: None,
                metadata_source: "pacman_local".to_string(),
            });
        }
    }

    catalog_items.sort_by(|a, b| {
        match (a.is_application, b.is_application) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()),
        }
    });

    let now_sec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let total_installed_apps = catalog_items.iter().filter(|i| i.is_application && i.is_installed).count();
    let total_installed_pkgs = catalog_items.iter().filter(|i| i.is_installed).count();

    let status = CatalogStatus {
        total_packages: catalog_items.len(),
        total_applications: total_apps,
        total_installed_applications: total_installed_apps,
        total_installed_packages: total_installed_pkgs,
        enabled_repositories: enabled_repos,
        last_refreshed: now_sec,
        is_refreshing: false,
        error: None,
    };

    Ok((catalog_items, status))
}

fn capitalize_pkg_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Catalog Store & In-Memory Thread-Safe State
// ─────────────────────────────────────────────────────────────────────────────

pub struct CatalogStore {
    pub items: Vec<CatalogItem>,
    pub by_id: HashMap<String, usize>,
    pub status: CatalogStatus,
}

impl Default for CatalogStore {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            by_id: HashMap::new(),
            status: CatalogStatus {
                total_packages: 0,
                total_applications: 0,
                total_installed_applications: 0,
                total_installed_packages: 0,
                enabled_repositories: Vec::new(),
                last_refreshed: 0,
                is_refreshing: false,
                error: None,
            },
        }
    }
}

static GLOBAL_CATALOG: OnceLock<Arc<RwLock<CatalogStore>>> = OnceLock::new();

pub fn get_catalog_store() -> Arc<RwLock<CatalogStore>> {
    GLOBAL_CATALOG
        .get_or_init(|| Arc::new(RwLock::new(CatalogStore::default())))
        .clone()
}

/// Fast and non-blocking refresh of ALPM installed state from /var/lib/pacman/local/.
/// Updates is_installed, installed_version, total_installed_applications, and total_installed_packages.
pub fn refresh_installed_state_internal() -> Result<CatalogStatus, String> {
    let store_arc = get_catalog_store();
    let local_dir = resolve_local_dir(None);
    let installed_map = scan_alpm_installed_packages(&local_dir);

    let mut store = store_arc.write().map_err(|e| e.to_string())?;
    let mut total_installed_apps = 0;
    let mut total_installed_pkgs = 0;

    for item in store.items.iter_mut() {
        let pkg_lower = item.id.to_lowercase();
        if let Some(ver) = installed_map.get(&pkg_lower) {
            item.is_installed = true;
            item.installed_version = Some(ver.clone());
            if item.is_application {
                total_installed_apps += 1;
            }
            total_installed_pkgs += 1;
        } else {
            item.is_installed = false;
            item.installed_version = None;
        }
    }

    // Also include any locally installed desktop packages not in store.items
    let desktop_dir = resolve_desktop_dir(None);
    let desktop_map = parse_system_desktop_entries(&desktop_dir);

    for (inst_pkg, inst_ver) in &installed_map {
        let pkg_lower = inst_pkg.to_lowercase();
        if !store.by_id.contains_key(&pkg_lower) {
            let (is_app, display_name, category, subcats, icon_name) = if let Some(d) = desktop_map.get(&pkg_lower) {
                let (cat, subs) = normalize_app_category(&d.categories);
                (true, if !d.name.is_empty() { d.name.clone() } else { capitalize_pkg_name(inst_pkg) }, cat, subs, d.icon.clone())
            } else {
                (false, inst_pkg.clone(), "Other Packages".to_string(), Vec::new(), None)
            };

            let new_item = CatalogItem {
                id: inst_pkg.clone(),
                display_name,
                summary: format!("Installed package {}", inst_pkg),
                description: String::new(),
                repository: "local".to_string(),
                version: inst_ver.clone(),
                is_installed: true,
                installed_version: Some(inst_ver.clone()),
                is_application: is_app,
                category,
                subcategories: subcats,
                icon_name,
                icon_path: None,
                launchable: if is_app { Some(format!("{}.desktop", pkg_lower)) } else { None },
                homepage: None,
                license: None,
                download_size: None,
                installed_size: None,
                dependencies: Vec::new(),
                screenshots: Vec::new(),
                developer: None,
                metadata_source: "pacman_local".to_string(),
            };

            let new_idx = store.items.len();
            store.by_id.insert(pkg_lower, new_idx);
            if is_app {
                total_installed_apps += 1;
            }
            total_installed_pkgs += 1;
            store.items.push(new_item);
        }
    }

    store.status.total_packages = store.items.len();
    store.status.total_applications = store.items.iter().filter(|i| i.is_application).count();
    store.status.total_installed_applications = total_installed_apps;
    store.status.total_installed_packages = total_installed_pkgs;
    Ok(store.status.clone())
}

/// Ensures the catalog is loaded in memory.
/// If empty, loads from disk cache (<15ms) and updates ALPM installed state (<10ms).
/// If cache is missing, builds dynamic catalog synchronously on first call.
pub fn ensure_catalog_initialized() {
    let store_arc = get_catalog_store();
    let is_empty = {
        let r = store_arc.read().ok();
        r.map(|s| s.items.is_empty()).unwrap_or(true)
    };

    if is_empty {
        let cache_file = resolve_cache_file_path();
        if cache_file.is_file() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok((items, status)) = serde_json::from_str::<(Vec<CatalogItem>, CatalogStatus)>(&content) {
                    if let Ok(mut store) = store_arc.write() {
                        store.by_id.clear();
                        for (idx, item) in items.iter().enumerate() {
                            store.by_id.insert(item.id.clone(), idx);
                        }
                        store.items = items;
                        store.status = status;
                    }
                    let _ = refresh_installed_state_internal();
                    return;
                }
            }
        }

        // Cache missing or unreadable — build dynamic catalog synchronously
        let _ = refresh_catalog_internal();
    }
}

pub fn initialize_catalog() {
    ensure_catalog_initialized();
}

/// Performs a full catalog refresh and saves to disk cache.
pub fn refresh_catalog_internal() -> Result<CatalogStatus, String> {
    let store_arc = get_catalog_store();

    if let Ok(mut store) = store_arc.write() {
        store.status.is_refreshing = true;
    }

    let conf = resolve_pacman_conf_path(None);
    let sync = resolve_sync_dir(None);
    let local = resolve_local_dir(None);
    let appstream = resolve_appstream_xml_dir(None);
    let icons = resolve_appstream_icons_dir(None);
    let desktop = resolve_desktop_dir(None);

    match build_dynamic_catalog(&conf, &sync, &local, &appstream, &icons, &desktop) {
        Ok((items, status)) => {
            let cache_file = resolve_cache_file_path();
            if let Some(parent) = cache_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(&(&items, &status)) {
                let _ = fs::write(&cache_file, json);
            }

            if let Ok(mut store) = store_arc.write() {
                store.by_id.clear();
                for (idx, item) in items.iter().enumerate() {
                    store.by_id.insert(item.id.clone(), idx);
                }
                store.items = items;
                store.status = status.clone();
            }

            Ok(status)
        }
        Err(err) => {
            if let Ok(mut store) = store_arc.write() {
                store.status.is_refreshing = false;
                store.status.error = Some(err.clone());
            }
            Err(err)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fast Filtered & Indexed Queries
// ─────────────────────────────────────────────────────────────────────────────

pub fn query_catalog(filter: CatalogQueryFilter) -> CatalogPageResponse {
    let store_arc = get_catalog_store();
    let store = match store_arc.read() {
        Ok(s) => s,
        Err(_) => {
            return CatalogPageResponse {
                items: Vec::new(),
                total_count: 0,
                page: 0,
                page_size: 50,
            }
        }
    };

    let search_q = filter.search_query.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty());
    let category_q = filter.category.map(|c| c.trim().to_lowercase()).filter(|c| !c.is_empty() && c != "all");
    let repo_q = filter.repository.map(|r| r.trim().to_lowercase()).filter(|r| !r.is_empty());
    let app_only = filter.is_application_only.unwrap_or(true);
    let installed_only = filter.is_installed_only.unwrap_or(false);

    let page = filter.page.unwrap_or(0);
    let page_size = filter.page_size.unwrap_or(60).min(500);

    let mut matched: Vec<&CatalogItem> = store
        .items
        .iter()
        .filter(|item| {
            if app_only && !item.is_application {
                return false;
            }
            if installed_only && !item.is_installed {
                return false;
            }
            if let Some(ref r) = repo_q {
                if !item.repository.eq_ignore_ascii_case(r) {
                    return false;
                }
            }
            if let Some(ref cat) = category_q {
                if cat == "installed" {
                    if !item.is_installed {
                        return false;
                    }
                } else if cat == "popular" {
                    let is_pop = item.id == "firefox"
                        || item.id == "chromium"
                        || item.id == "code"
                        || item.id == "discord"
                        || item.id == "steam"
                        || item.id == "vlc"
                        || item.id == "gimp"
                        || item.id == "obs-studio"
                        || item.id == "blender"
                        || item.id == "inkscape";
                    if !is_pop {
                        return false;
                    }
                } else {
                    let match_cat = item.category.to_lowercase() == *cat
                        || item.subcategories.iter().any(|s| s.to_lowercase() == *cat);
                    if !match_cat {
                        return false;
                    }
                }
            }
            if let Some(ref q) = search_q {
                let id_lower = item.id.to_lowercase();
                let name_lower = item.display_name.to_lowercase();
                let sum_lower = item.summary.to_lowercase();
                let cat_lower = item.category.to_lowercase();

                let matches = id_lower.contains(q)
                    || name_lower.contains(q)
                    || sum_lower.contains(q)
                    || cat_lower.contains(q);
                if !matches {
                    return false;
                }
            }
            true
        })
        .collect();

    if let Some(ref q) = search_q {
        matched.sort_by_cached_key(|item| {
            let id_lower = item.id.to_lowercase();
            let name_lower = item.display_name.to_lowercase();

            if id_lower == *q || name_lower == *q {
                0
            } else if id_lower.starts_with(q) || name_lower.starts_with(q) {
                1
            } else if id_lower.contains(q) {
                2
            } else {
                3
            }
        });
    }

    let total_count = matched.len();
    let start_idx = page * page_size;
    let items = if start_idx < total_count {
        matched
            .into_iter()
            .skip(start_idx)
            .take(page_size)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    CatalogPageResponse {
        items,
        total_count,
        page,
        page_size,
    }
}

pub fn get_catalog_item_by_id(id: &str) -> Option<CatalogItem> {
    let store_arc = get_catalog_store();
    let store = store_arc.read().ok()?;
    let idx = store.by_id.get(id)?;
    store.items.get(*idx).cloned()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pacman_conf_repositories() {
        let conf_content = r#"
[options]
HoldPkg     = pacman glibc
Architecture = auto

[garuda]
Include = /etc/pacman.d/chaotic-mirrorlist

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist

[multilib]
Include = /etc/pacman.d/mirrorlist
"#;
        let tmp_dir = std::env::temp_dir().join("ryzora_test_pacman_conf");
        let _ = fs::create_dir_all(&tmp_dir);
        let conf_file = tmp_dir.join("pacman.conf");
        fs::write(&conf_file, conf_content).unwrap();

        let repos = parse_pacman_conf_repositories(&conf_file);
        assert_eq!(repos, vec!["garuda", "core", "extra", "multilib"]);
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_normalize_app_category() {
        let (cat, sub) = normalize_app_category(&["2DGraphics".into(), "RasterGraphics".into()]);
        assert_eq!(cat, "Graphics");
        assert_eq!(sub.len(), 2);

        let (cat, _) = normalize_app_category(&["AudioVideo".into(), "Player".into()]);
        assert_eq!(cat, "Multimedia");

        let (cat, _) = normalize_app_category(&["Network".into(), "WebBrowser".into()]);
        assert_eq!(cat, "Internet");

        let (cat, _) = normalize_app_category(&["Development".into(), "IDE".into()]);
        assert_eq!(cat, "Development");
    }

    #[test]
    fn test_classify_generic_package() {
        assert_eq!(classify_generic_package("libpng", "PNG library", &[]), "Libraries");
        assert_eq!(classify_generic_package("python-requests", "HTTP client", &[]), "Libraries");
        assert_eq!(classify_generic_package("ttf-dejavu", "DejaVu fonts", &[]), "Fonts");
        assert_eq!(classify_generic_package("linux-firmware", "Kernel firmware", &[]), "System Components");
        assert_eq!(classify_generic_package("ripgrep", "Fast regex search CLI tool", &[]), "CLI Tools");
    }

    #[test]
    fn test_extract_tag_content() {
        assert_eq!(extract_tag_content("<pkgname>blender</pkgname>", "pkgname"), "blender");
        assert_eq!(extract_tag_content("<url type=\"homepage\">https://blender.org</url>", "url"), "https://blender.org");
        assert_eq!(extract_tag_content("  <name>GIMP Photo Editor</name> ", "name"), "GIMP Photo Editor");
    }

    static TEST_CATALOG_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_installed_state_refresh_and_query() {
        let _guard = TEST_CATALOG_MUTEX.lock().unwrap();
        let store_arc = get_catalog_store();
        {
            let mut store = store_arc.write().unwrap();
            store.items = vec![
                CatalogItem {
                    id: "firefox".to_string(),
                    display_name: "Firefox".to_string(),
                    summary: "Web browser".to_string(),
                    description: "Web browser".to_string(),
                    repository: "extra".to_string(),
                    version: "130.0".to_string(),
                    is_installed: false,
                    installed_version: None,
                    is_application: true,
                    category: "Internet".to_string(),
                    subcategories: vec!["WebBrowser".to_string()],
                    icon_name: Some("firefox".to_string()),
                    icon_path: None,
                    launchable: Some("firefox.desktop".to_string()),
                    homepage: None,
                    license: None,
                    download_size: None,
                    installed_size: None,
                    dependencies: Vec::new(),
                    screenshots: Vec::new(),
                    developer: None,
                    metadata_source: "appstream".to_string(),
                },
                CatalogItem {
                    id: "blender".to_string(),
                    display_name: "Blender".to_string(),
                    summary: "3D suite".to_string(),
                    description: "3D suite".to_string(),
                    repository: "extra".to_string(),
                    version: "4.2.0".to_string(),
                    is_installed: false,
                    installed_version: None,
                    is_application: true,
                    category: "Graphics".to_string(),
                    subcategories: vec!["3DGraphics".to_string()],
                    icon_name: Some("blender".to_string()),
                    icon_path: None,
                    launchable: Some("blender.desktop".to_string()),
                    homepage: None,
                    license: None,
                    download_size: None,
                    installed_size: None,
                    dependencies: Vec::new(),
                    screenshots: Vec::new(),
                    developer: None,
                    metadata_source: "appstream".to_string(),
                },
            ];
            store.by_id.clear();
            store.by_id.insert("firefox".to_string(), 0);
            store.by_id.insert("blender".to_string(), 1);
        }

        let status = refresh_installed_state_internal().unwrap();
        assert!(status.total_packages >= 2);

        let res = query_catalog(CatalogQueryFilter {
            is_application_only: Some(true),
            category: None,
            search_query: None,
            repository: None,
            is_installed_only: Some(true),
            page: Some(0),
            page_size: Some(10),
        });

        let has_firefox = res.items.iter().any(|i| i.id == "firefox" && i.is_installed);
        assert!(has_firefox, "installed query must return installed Firefox");

        let has_blender = res.items.iter().any(|i| i.id == "blender");
        assert!(!has_blender, "installed query must not return uninstalled Blender");

        // Restore catalog store from cache
        let cache_file = resolve_cache_file_path();
        if cache_file.is_file() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok((items, status)) = serde_json::from_str::<(Vec<CatalogItem>, CatalogStatus)>(&content) {
                    if let Ok(mut store) = store_arc.write() {
                        store.by_id.clear();
                        for (idx, item) in items.iter().enumerate() {
                            store.by_id.insert(item.id.clone(), idx);
                        }
                        store.items = items;
                        store.status = status;
                    }
                    let _ = refresh_installed_state_internal();
                }
            }
        }
    }

    #[test]
    fn test_real_machine_catalog_full_validation() {
        let _guard = TEST_CATALOG_MUTEX.lock().unwrap();
        let store_arc = get_catalog_store();
        // Force reload full catalog from cache into store to ensure clean state
        let cache_file = resolve_cache_file_path();
        let mut loaded = false;
        if cache_file.is_file() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok((items, status)) = serde_json::from_str::<(Vec<CatalogItem>, CatalogStatus)>(&content) {
                    if let Ok(mut store) = store_arc.write() {
                        store.by_id.clear();
                        for (idx, item) in items.iter().enumerate() {
                            store.by_id.insert(item.id.clone(), idx);
                        }
                        store.items = items;
                        store.status = status;
                        loaded = true;
                    }
                }
            }
        }
        if !loaded {
            let _ = refresh_catalog_internal();
        } else {
            let _ = refresh_installed_state_internal();
        }
        let store = store_arc.read().unwrap();

        println!("=== REAL MACHINE ARCH CATALOG SUMMARY ===");
        println!("Total Packages: {}", store.status.total_packages);
        println!("Total Applications: {}", store.status.total_applications);
        println!("Total Installed Applications: {}", store.status.total_installed_applications);
        println!("Total Installed Packages: {}", store.status.total_installed_packages);
        println!("Configured Repositories: {:?}", store.status.enabled_repositories);

        assert!(store.status.total_packages > 5000, "Must discover thousands of pacman packages");
        assert!(store.status.total_applications > 500, "Must discover hundreds of desktop applications");
        assert!(store.status.total_installed_applications > 20, "Must discover real installed applications");

        let res = query_catalog(CatalogQueryFilter {
            is_application_only: Some(true),
            category: None,
            search_query: None,
            repository: None,
            is_installed_only: Some(true),
            page: Some(0),
            page_size: Some(60),
        });

        println!("=== INSTALLED DESKTOP APPLICATIONS (Page 0, 60 items) ===");
        println!("Query returned {} items (total count {})", res.items.len(), res.total_count);
        for item in &res.items {
            assert!(item.is_installed, "All items in installed query must be is_installed = true");
            assert!(item.is_application, "All items in installed applications query must be is_application = true");
            println!("  [✓] {} (pkg: {}, ver: {}) - {}", item.display_name, item.id, item.version, item.category);
        }

        // Verify known installed applications exist in installed query results
        let ids: Vec<String> = res.items.iter().map(|i| i.id.to_lowercase()).collect();
        println!("Sample installed IDs: {:?}", &ids[..ids.len().min(15)]);

        // Check search across full catalog
        let firefox_search = query_catalog(CatalogQueryFilter {
            is_application_only: Some(true),
            category: None,
            search_query: Some("firefox".to_string()),
            repository: None,
            is_installed_only: None,
            page: Some(0),
            page_size: Some(10),
        });
        assert!(!firefox_search.items.is_empty(), "Must find firefox in full catalog search");
        assert!(firefox_search.items[0].is_installed, "Firefox must be marked as installed");

        let blender_search = query_catalog(CatalogQueryFilter {
            is_application_only: Some(true),
            category: None,
            search_query: Some("blender".to_string()),
            repository: None,
            is_installed_only: None,
            page: Some(0),
            page_size: Some(10),
        });
        assert!(!blender_search.items.is_empty(), "Must find blender in full catalog search");
    }
}
