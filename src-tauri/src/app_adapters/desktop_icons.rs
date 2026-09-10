//! Desktop Icon and XDG Application metadata resolver — Phase 23B
//!
//! Implements the full Freedesktop desktop-entry and icon-theme lookup pipeline.
//! Zero-subprocess guarantee: only std::fs reads. No Command::new, no shell calls.
//!
//! Priority:
//!   1. Match installed .desktop entry by package ID, filename, Name, GenericName, Exec, StartupWMClass
//!   2. Extract Icon= field from desktop entry
//!   3. Resolve icon name through installed icon themes (preferred → hicolor → pixmaps)
//!   4. For uninstalled catalog apps, fall back to icon-theme lookup without desktop entry
//!   5. Return None — let the frontend apply its own fallback

use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Public Data Model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopAppInfo {
    pub desktop_file: String,
    pub name: String,
    pub generic_name: Option<String>,
    pub icon_name: Option<String>,
    pub icon_path: Option<String>,
    pub icon_svg_content: Option<String>,
    pub icon_data_uri: Option<String>,
    pub exec: Option<String>,
    pub startup_wm_class: Option<String>,
    pub categories: Vec<String>,
}

pub type DesktopAppIconInfo = DesktopAppInfo;

// ─────────────────────────────────────────────────────────────────────────────
// Package ID → Desktop/Icon Aliases
// ─────────────────────────────────────────────────────────────────────────────

fn known_aliases(pkg: &str) -> Option<(&'static str, &'static str)> {
    match pkg {
        "visual-studio-code" | "visual-studio-code-bin" | "code-oss" | "vscodium" => Some(("code", "vscode")),
        "obs-studio" | "obs" => Some(("com.obsproject.Studio", "com.obsproject.Studio")),
        "telegram-desktop" | "telegram" => Some(("org.telegram.desktop", "org.telegram.desktop")),
        "thunderbird" | "mozilla-thunderbird" => Some(("org.mozilla.Thunderbird", "org.mozilla.Thunderbird")),
        "spotify" | "spotify-client" => Some(("spotify", "spotify-client")),
        "firefox-esr" => Some(("firefox-esr", "firefox-esr")),
        "firefox-bin" => Some(("firefox", "firefox")),
        "intellij-idea-ultimate-edition" | "intellij-idea-community-edition" => Some(("idea", "idea")),
        "pycharm-professional" | "pycharm-community" => Some(("pycharm", "pycharm")),
        "webstorm" => Some(("webstorm", "webstorm")),
        "neovim" => Some(("nvim", "nvim")),
        "gvim" | "vim" => Some(("gvim", "gvim")),
        "kitty" => Some(("kitty", "kitty")),
        "alacritty" => Some(("Alacritty", "Alacritty")),
        "wezterm" | "wezterm-git" => Some(("org.wezfurlong.wezterm", "org.wezfurlong.wezterm")),
        "discord" => Some(("discord", "discord")),
        "slack-desktop" | "slack" => Some(("slack", "slack")),
        "steam" => Some(("steam", "steam")),
        "lutris" => Some(("lutris", "lutris")),
        "heroic-games-launcher-bin" | "heroic-games-launcher" => Some(("com.heroicgameslauncher.hgl", "com.heroicgameslauncher.hgl")),
        "vlc" => Some(("vlc", "vlc")),
        "mpv" => Some(("mpv", "mpv")),
        "celluloid" => Some(("io.github.celluloid_player.Celluloid", "io.github.celluloid_player.Celluloid")),
        "gimp" | "gimp2" => Some(("gimp", "gimp")),
        "blender" => Some(("blender", "blender")),
        "inkscape" => Some(("inkscape", "inkscape")),
        "krita" => Some(("krita", "krita")),
        "darktable" => Some(("darktable", "darktable")),
        "chromium" => Some(("chromium", "chromium")),
        "firefox" => Some(("firefox", "firefox")),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Path Discovery
// ─────────────────────────────────────────────────────────────────────────────

fn get_xdg_data_dirs() -> Vec<String> {
    std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string())
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn get_desktop_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut add = |p: PathBuf| {
        if seen.insert(p.display().to_string()) {
            dirs.push(p);
        }
    };

    for base in get_xdg_data_dirs() {
        add(PathBuf::from(&base).join("applications"));
    }

    if let Ok(home) = std::env::var("HOME") {
        add(PathBuf::from(&home).join(".local/share/applications"));
    }

    if let Ok(xdg_home) = std::env::var("XDG_DATA_HOME") {
        add(PathBuf::from(xdg_home).join("applications"));
    }

    dirs
}

fn get_icon_theme_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut add = |p: PathBuf| {
        if p.exists() && seen.insert(p.display().to_string()) {
            dirs.push(p);
        }
    };

    if let Ok(home) = std::env::var("HOME") {
        let h = PathBuf::from(&home);
        add(h.join(".icons"));
        add(h.join(".local/share/icons"));
    }

    let data_dirs = get_xdg_data_dirs();

    // Preferred themes first for best icon quality
    for theme in &["BeautyLine", "candy-icons", "breeze", "Papirus", "hicolor"] {
        for base in &data_dirs {
            add(PathBuf::from(base).join("icons").join(theme));
        }
    }

    // All remaining icon themes
    let icons_base = PathBuf::from("/usr/share/icons");
    if icons_base.exists() {
        if let Ok(entries) = fs::read_dir(&icons_base) {
            let mut themes: Vec<PathBuf> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect();
            themes.sort();
            for t in themes {
                add(t);
            }
        }
    }

    // Pixmaps as final resort
    add(PathBuf::from("/usr/share/pixmaps"));

    dirs
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop File Parsing
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct DesktopEntry {
    name: Option<String>,
    generic_name: Option<String>,
    icon: Option<String>,
    exec: Option<String>,
    startup_wm_class: Option<String>,
    categories: Vec<String>,
}

fn parse_desktop_file(content: &str) -> DesktopEntry {
    let mut entry = DesktopEntry::default();
    let mut in_main_section = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_main_section = trimmed == "[Desktop Entry]";
            continue;
        }

        if !in_main_section || trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "Name" if entry.name.is_none() => entry.name = Some(val.to_string()),
                "GenericName" if entry.generic_name.is_none() => {
                    entry.generic_name = Some(val.to_string())
                }
                "Icon" if entry.icon.is_none() => entry.icon = Some(val.to_string()),
                "Exec" if entry.exec.is_none() => {
                    let exec = val
                        .split_whitespace()
                        .filter(|t| !t.starts_with('%'))
                        .collect::<Vec<_>>()
                        .join(" ");
                    entry.exec = Some(exec);
                }
                "StartupWMClass" if entry.startup_wm_class.is_none() => {
                    entry.startup_wm_class = Some(val.to_string())
                }
                "Categories" if entry.categories.is_empty() => {
                    entry.categories = val
                        .split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }

    entry
}

// ─────────────────────────────────────────────────────────────────────────────
// Desktop File Discovery
// ─────────────────────────────────────────────────────────────────────────────

fn find_desktop_entry(package_id: &str) -> Option<(PathBuf, DesktopEntry)> {
    let pkg_lower = package_id.to_lowercase();
    let desktop_dirs = get_desktop_dirs();

    // Build candidate stems ordered by specificity
    let mut candidate_stems: Vec<String> = Vec::new();

    if let Some((desk_stem, _)) = known_aliases(&pkg_lower) {
        if !desk_stem.is_empty() {
            candidate_stems.push(desk_stem.to_string());
        }
    }

    candidate_stems.push(package_id.to_string());
    candidate_stems.push(pkg_lower.clone());

    for prefix in &["org.", "com.", "io.", "net.", "io.github."] {
        candidate_stems.push(format!("{}{}", prefix, pkg_lower));
    }

    // Stripped variants
    let stripped = pkg_lower
        .trim_end_matches("-bin")
        .trim_end_matches("-git")
        .trim_end_matches("-nightly")
        .trim_end_matches("-stable")
        .trim_end_matches("-desktop")
        .trim_end_matches("-app");
    if stripped != pkg_lower {
        candidate_stems.push(stripped.to_string());
    }

    // Deduplicate
    let mut seen_set = std::collections::HashSet::new();
    let candidate_stems: Vec<String> = candidate_stems
        .into_iter()
        .filter(|s| seen_set.insert(s.clone()))
        .collect();

    // Exact filename match first
    for dir in &desktop_dirs {
        if !dir.exists() {
            continue;
        }
        for stem in &candidate_stems {
            let p = dir.join(format!("{}.desktop", stem));
            if p.exists() {
                if let Ok(content) = fs::read_to_string(&p) {
                    return Some((p, parse_desktop_file(&content)));
                }
            }
        }
    }

    // Partial scan with scoring
    for dir in &desktop_dirs {
        if !dir.exists() {
            continue;
        }
        let read = match fs::read_dir(dir) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let mut scored: Vec<(PathBuf, DesktopEntry, u8)> = Vec::new();

        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let fname = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let de = parse_desktop_file(&content);

            let name_lower = de.name.as_deref().unwrap_or("").to_lowercase();
            let icon_lower = de.icon.as_deref().unwrap_or("").to_lowercase();
            let wm_lower = de.startup_wm_class.as_deref().unwrap_or("").to_lowercase();
            let exec_lower = de.exec.as_deref().unwrap_or("").to_lowercase();

            let score: u8 = if fname == pkg_lower {
                10
            } else if fname.contains(&pkg_lower) || pkg_lower.contains(&fname.replace(".desktop", "")) {
                8
            } else if name_lower == pkg_lower || name_lower.replace(' ', "-") == pkg_lower {
                7
            } else if icon_lower == pkg_lower {
                6
            } else if wm_lower == pkg_lower {
                5
            } else if exec_lower.contains(&pkg_lower) {
                4
            } else {
                0
            };

            if score > 0 {
                scored.push((path, de, score));
            }
        }

        if !scored.is_empty() {
            scored.sort_by(|a, b| b.2.cmp(&a.2));
            let (path, entry, _) = scored.remove(0);
            return Some((path, entry));
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Icon Theme Lookup
// ─────────────────────────────────────────────────────────────────────────────

fn resolve_icon_file(icon_name: &str) -> Option<PathBuf> {
    // Absolute path — check directly
    let path = Path::new(icon_name);
    if path.is_absolute() && path.exists() {
        return Some(path.to_path_buf());
    }

    let theme_dirs = get_icon_theme_dirs();
    let extensions = ["svg", "png", "xpm"];
    let size_dirs = [
        "scalable", "512x512", "256x256", "192x192", "128x128",
        "64x64", "48x48", "32x32", "22x22", "16x16",
    ];
    let app_subdirs = ["apps", "categories", "devices", "places", "actions"];

    for theme_dir in &theme_dirs {
        let is_pixmaps = theme_dir
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == "pixmaps")
            .unwrap_or(false);

        if is_pixmaps {
            for ext in &extensions {
                let c = theme_dir.join(format!("{}.{}", icon_name, ext));
                if c.exists() {
                    return Some(c);
                }
            }
            continue;
        }

        if !theme_dir.exists() {
            continue;
        }

        // size/apps/icon.ext  and  apps/scalable/icon.ext patterns
        for size in &size_dirs {
            for subdir in &app_subdirs {
                for ext in &extensions {
                    let c = theme_dir.join(size).join(subdir).join(format!("{}.{}", icon_name, ext));
                    if c.exists() {
                        return Some(c);
                    }
                }
            }
            // apps/size/icon.ext variant
            for ext in &extensions {
                let c = theme_dir.join("apps").join(size).join(format!("{}.{}", icon_name, ext));
                if c.exists() {
                    return Some(c);
                }
            }
        }

        // Flat root (some themes)
        for ext in &extensions {
            let c = theme_dir.join(format!("{}.{}", icon_name, ext));
            if c.exists() {
                return Some(c);
            }
        }
    }

    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Base64 Encoder
// ─────────────────────────────────────────────────────────────────────────────

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        if i + 1 < bytes.len() { out.push(TABLE[((n >> 6) & 63) as usize] as char); } else { out.push('='); }
        if i + 2 < bytes.len() { out.push(TABLE[(n & 63) as usize] as char); } else { out.push('='); }
        i += 3;
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// SVG Processing
// ─────────────────────────────────────────────────────────────────────────────

fn process_svg(content: &str) -> Option<String> {
    let lower = content.to_lowercase();
    if lower.contains("<script")
        || lower.contains("javascript:")
        || lower.contains(" onload=")
        || lower.contains(" onerror=")
        || lower.contains(" onclick=")
        || lower.contains("<!entity")
    {
        return None;
    }

    let svg_start = content.find("<svg")?;
    let tag_end = content[svg_start..].find('>')?;
    let svg_tag = &content[svg_start..svg_start + tag_end + 1];

    if svg_tag.to_lowercase().contains("viewbox") {
        return Some(content.to_string());
    }

    let width_px = extract_svg_dimension(svg_tag, "width");
    let height_px = extract_svg_dimension(svg_tag, "height");

    if let (Some(w), Some(h)) = (width_px, height_px) {
        let inject_pos = svg_start + tag_end;
        let viewbox_attr = format!(" viewBox=\"0 0 {} {}\"", w as u32, h as u32);
        let mut normalized = content.to_string();
        normalized.insert_str(inject_pos, &viewbox_attr);
        return Some(normalized);
    }

    Some(content.to_string())
}

fn extract_svg_dimension(tag: &str, attr: &str) -> Option<f64> {
    let search = format!("{}=", attr);
    let tag_lower = tag.to_lowercase();
    let pos = tag_lower.find(&search)?;
    let after = &tag[pos + search.len()..];
    let (quote, rest) = if after.starts_with('"') {
        ('"', &after[1..])
    } else if after.starts_with('\'') {
        ('\'', &after[1..])
    } else {
        return None;
    };
    let end = rest.find(quote)?;
    let val = rest[..end].trim();
    let numeric: String = val.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    numeric.parse::<f64>().ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Icon Loading
// ─────────────────────────────────────────────────────────────────────────────

fn load_icon_file(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let icon_path_str = Some(path.display().to_string());
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    match ext.as_str() {
        "svg" => {
            if let Ok(raw) = fs::read_to_string(path) {
                if let Some(normalized) = process_svg(&raw) {
                    return (icon_path_str, Some(normalized), None);
                }
            }
            (icon_path_str, None, None)
        }
        "png" => {
            if fs::metadata(path).map(|m| m.len() < 2 * 1024 * 1024).unwrap_or(false) {
                if let Ok(bytes) = fs::read(path) {
                    return (icon_path_str, None, Some(format!("data:image/png;base64,{}", encode_base64(&bytes))));
                }
            }
            (icon_path_str, None, None)
        }
        "xpm" => {
            if fs::metadata(path).map(|m| m.len() < 512 * 1024).unwrap_or(false) {
                if let Ok(bytes) = fs::read(path) {
                    return (icon_path_str, None, Some(format!("data:image/x-xpixmap;base64,{}", encode_base64(&bytes))));
                }
            }
            (icon_path_str, None, None)
        }
        _ => (icon_path_str, None, None),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public Entry Point
// ─────────────────────────────────────────────────────────────────────────────

pub fn resolve_desktop_icon_for_app(package_id: &str) -> Option<DesktopAppInfo> {
    let pkg_lower = package_id.to_lowercase().trim().to_string();
    let alias = known_aliases(&pkg_lower);
    let icon_hint = alias.map(|(_, h)| h);

    // Step 1: Locate .desktop entry
    let desktop_result = find_desktop_entry(&pkg_lower);
    let (desktop_file, entry) = match desktop_result {
        Some((path, e)) => (path.display().to_string(), Some(e)),
        None => (String::new(), None),
    };

    let name = entry.as_ref().and_then(|e| e.name.clone()).unwrap_or_else(|| package_id.to_string());
    let generic_name = entry.as_ref().and_then(|e| e.generic_name.clone());
    let icon_field = entry.as_ref().and_then(|e| e.icon.clone());
    let exec = entry.as_ref().and_then(|e| e.exec.clone());
    let startup_wm_class = entry.as_ref().and_then(|e| e.startup_wm_class.clone());
    let categories = entry.as_ref().map(|e| e.categories.clone()).unwrap_or_default();

    // Step 2: Determine icon name (alias hint > desktop Icon= > package ID)
    let icon_name = icon_hint
        .filter(|h| !h.is_empty())
        .map(|h| h.to_string())
        .or_else(|| icon_field.clone())
        .or_else(|| Some(pkg_lower.clone()));

    // Step 3: Resolve icon file via theme lookup
    let icon_file = icon_name.as_deref().and_then(resolve_icon_file).or_else(|| {
        if icon_name.as_deref() != Some(pkg_lower.as_str()) {
            resolve_icon_file(&pkg_lower)
        } else {
            None
        }
    });

    // Step 4: Load the icon
    let (icon_path, icon_svg_content, icon_data_uri) = match icon_file.as_ref() {
        Some(f) => load_icon_file(f),
        None => (None, None, None),
    };

    if desktop_file.is_empty() && icon_path.is_none() {
        return None;
    }

    Some(DesktopAppInfo {
        desktop_file,
        name,
        generic_name,
        icon_name: icon_name.or(icon_field),
        icon_path,
        icon_svg_content,
        icon_data_uri,
        exec,
        startup_wm_class,
        categories,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_desktop_file_basic() {
        let content = "[Desktop Entry]\nName=Firefox\nGenericName=Web Browser\nIcon=firefox\nCategories=Network;WebBrowser;\nExec=/usr/lib/firefox/firefox %u\nStartupWMClass=firefox\nType=Application\n";
        let entry = parse_desktop_file(content);
        assert_eq!(entry.name.as_deref(), Some("Firefox"));
        assert_eq!(entry.generic_name.as_deref(), Some("Web Browser"));
        assert_eq!(entry.icon.as_deref(), Some("firefox"));
        assert!(entry.categories.contains(&"Network".to_string()));
        assert_eq!(entry.startup_wm_class.as_deref(), Some("firefox"));
    }

    #[test]
    fn test_parse_desktop_ignores_action_sections() {
        let content = "[Desktop Entry]\nName=Firefox\nIcon=firefox\n\n[Desktop Action NewWindow]\nName=New Window\nExec=firefox --new-window\nIcon=other-icon\n";
        let entry = parse_desktop_file(content);
        assert_eq!(entry.icon.as_deref(), Some("firefox"));
        assert_eq!(entry.name.as_deref(), Some("Firefox"));
    }

    #[test]
    fn test_encode_base64() {
        assert_eq!(encode_base64(b"hello world"), "aGVsbG8gd29ybGQ=");
        assert_eq!(encode_base64(b"test"), "dGVzdA==");
        assert_eq!(encode_base64(b""), "");
    }

    #[test]
    fn test_svg_viewbox_injection() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512"><circle cx="256" cy="256" r="200"/></svg>"#;
        let result = process_svg(svg).unwrap();
        assert!(result.contains("viewBox=\"0 0 512 512\""), "viewBox not injected: {}", result);
    }

    #[test]
    fn test_svg_viewbox_preserved() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40"/></svg>"#;
        let result = process_svg(svg).unwrap();
        assert_eq!(result.matches("viewBox").count(), 1);
    }

    #[test]
    fn test_svg_security_reject_script() {
        assert!(process_svg(r#"<svg><script>alert(1)</script></svg>"#).is_none());
    }

    #[test]
    fn test_svg_security_reject_onerror() {
        assert!(process_svg(r#"<svg onerror="alert(1)"><circle/></svg>"#).is_none());
    }

    #[test]
    fn test_extract_svg_dimension() {
        assert_eq!(extract_svg_dimension(r#"<svg width="512" height="256">"#, "width"), Some(512.0));
        assert_eq!(extract_svg_dimension(r#"<svg width="512px" height="512px">"#, "width"), Some(512.0));
        assert_eq!(extract_svg_dimension(r#"<svg width="512" height="256">"#, "height"), Some(256.0));
    }

    #[test]
    fn test_known_alias_vscode() {
        assert_eq!(known_aliases("visual-studio-code-bin"), Some(("code", "vscode")));
    }

    #[test]
    fn test_known_alias_obs() {
        assert_eq!(known_aliases("obs-studio"), Some(("com.obsproject.Studio", "com.obsproject.Studio")));
    }

    #[test]
    fn test_desktop_dirs_not_empty() {
        assert!(!get_desktop_dirs().is_empty());
    }

    #[test]
    fn test_icon_theme_dirs_not_empty() {
        assert!(!get_icon_theme_dirs().is_empty());
    }
}
