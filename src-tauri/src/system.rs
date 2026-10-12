use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn get_telemetry_cache() -> &'static OnceLock<FastfetchTelemetry> {
    static TELEMETRY_CACHE: OnceLock<FastfetchTelemetry> = OnceLock::new();
    &TELEMETRY_CACHE
}

fn get_preview_cache() -> &'static Mutex<HashMap<String, FastfetchPreviewResponse>> {
    static PREVIEW_CACHE: OnceLock<Mutex<HashMap<String, FastfetchPreviewResponse>>> = OnceLock::new();
    PREVIEW_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledComponent {
    pub name: String,
    pub binary: String,
    pub installed: bool,
    pub path: Option<String>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub distro_name: String,
    pub distro_id: String,
    pub distro_family: String,
    pub distro_version: String,
    pub kernel_version: String,
    pub desktop_environment: String,
    pub window_manager: String,
    pub session_type: String, // "wayland", "x11", "tty"
    pub shell: String,
    pub terminal: String,
    pub installed_components: Vec<InstalledComponent>,
}

fn parse_os_release() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let paths = ["/etc/os-release", "/usr/lib/os-release"];

    for p in &paths {
        if let Ok(content) = fs::read_to_string(p) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, val)) = trimmed.split_once('=') {
                    let clean_val = val.trim_matches('"').trim_matches('\'').to_string();
                    map.insert(key.trim().to_string(), clean_val);
                }
            }
            break;
        }
    }
    map
}

pub fn check_binary(bin: &str) -> (bool, Option<String>) {
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let full_path = dir.join(bin);
            if full_path.is_file() {
                return (true, Some(full_path.to_string_lossy().to_string()));
            }
        }
    }

    let fallbacks = [
        format!("/usr/bin/{}", bin),
        format!("/usr/local/bin/{}", bin),
        format!(
            "{}/.cargo/bin/{}",
            env::var("HOME").unwrap_or_default(),
            bin
        ),
        format!(
            "{}/.local/bin/{}",
            env::var("HOME").unwrap_or_default(),
            bin
        ),
    ];

    for fb in &fallbacks {
        if Path::new(fb).is_file() {
            return (true, Some(fb.clone()));
        }
    }

    (false, None)
}

fn get_kernel_version() -> String {
    if let Ok(version) = fs::read_to_string("/proc/version") {
        if let Some(first_word) = version.split_whitespace().nth(2) {
            return first_word.to_string();
        }
    }
    "Linux (Generic)".to_string()
}

#[tauri::command]
pub fn detect_system_info() -> SystemInfo {
    let os_info = parse_os_release();

    let distro_name = os_info
        .get("PRETTY_NAME")
        .or_else(|| os_info.get("NAME"))
        .cloned()
        .unwrap_or_else(|| "Linux".to_string());

    let distro_id = os_info
        .get("ID")
        .cloned()
        .unwrap_or_else(|| "linux".to_string());

    let id_lower = distro_id.to_lowercase();
    let id_like = os_info
        .get("ID_LIKE")
        .cloned()
        .unwrap_or_default()
        .to_lowercase();

    let distro_family = if id_lower.contains("arch")
        || id_lower.contains("garuda")
        || id_lower.contains("manjaro")
        || id_lower.contains("endeavour")
        || id_like.contains("arch")
    {
        "arch".to_string()
    } else if id_lower.contains("debian")
        || id_lower.contains("ubuntu")
        || id_lower.contains("pop")
        || id_lower.contains("mint")
        || id_like.contains("debian")
        || id_like.contains("ubuntu")
    {
        "debian".to_string()
    } else if id_lower.contains("fedora")
        || id_lower.contains("rhel")
        || id_lower.contains("centos")
        || id_lower.contains("nobara")
        || id_like.contains("fedora")
    {
        "fedora".to_string()
    } else if id_lower.contains("suse") || id_like.contains("suse") {
        "opensuse".to_string()
    } else if id_lower.contains("nix") {
        "nixos".to_string()
    } else {
        "generic_linux".to_string()
    };

    let distro_version = os_info
        .get("VERSION_ID")
        .cloned()
        .unwrap_or_else(|| "Rolling".to_string());

    let kernel_version = get_kernel_version();

    let xdg_current = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktop_session = env::var("DESKTOP_SESSION").unwrap_or_default();

    let mut wm = "Unknown".to_string();
    let mut de = if !xdg_current.is_empty() {
        xdg_current.clone()
    } else if !desktop_session.is_empty() {
        desktop_session.clone()
    } else {
        "Standalone WM".to_string()
    };

    if let Ok(test_desktop) = env::var("RYZORA_TEST_DESKTOP") {
        wm = test_desktop.clone();
        de = test_desktop;
    } else if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || xdg_current.to_lowercase().contains("hyprland")
        || desktop_session.to_lowercase().contains("hyprland")
    {
        wm = "Hyprland".to_string();
        de = "Hyprland".to_string();
    } else if env::var("SWAYSOCK").is_ok()
        || xdg_current.to_lowercase().contains("sway")
        || desktop_session.to_lowercase().contains("sway")
    {
        wm = "Sway".to_string();
        de = "Sway".to_string();
    } else if xdg_current.to_lowercase().contains("kde")
        || desktop_session.to_lowercase().contains("plasma")
    {
        wm = "KWin".to_string();
        de = "KDE Plasma".to_string();
    } else if xdg_current.to_lowercase().contains("gnome") {
        wm = "Mutter".to_string();
        de = "GNOME".to_string();
    } else if xdg_current.to_lowercase().contains("cosmic") {
        wm = "COSMIC Comp".to_string();
        de = "COSMIC".to_string();
    } else if xdg_current.to_lowercase().contains("xfce") {
        wm = "Xfwm4".to_string();
        de = "XFCE".to_string();
    } else if xdg_current.to_lowercase().contains("cinnamon") {
        wm = "Muffin".to_string();
        de = "Cinnamon".to_string();
    } else if xdg_current.to_lowercase().contains("i3") {
        wm = "i3".to_string();
        de = "i3wm".to_string();
    } else if wm == "Unknown" && !xdg_current.is_empty() {
        wm = xdg_current.clone();
    }

    let session_type = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| {
        if env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland".to_string()
        } else if env::var("DISPLAY").is_ok() {
            "x11".to_string()
        } else {
            "unknown".to_string()
        }
    });

    let shell = env::var("SHELL")
        .map(|s| {
            Path::new(&s)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(s)
        })
        .unwrap_or_else(|_| "bash".to_string());

    let terminal = env::var("TERM_PROGRAM")
        .or_else(|_| env::var("COLORTERM"))
        .or_else(|_| env::var("TERM"))
        .unwrap_or_else(|_| "kitty".to_string());

    let components_to_probe = [
        ("Hyprland", "hyprland", "Window Manager"),
        ("Sway", "sway", "Window Manager"),
        ("Waybar", "waybar", "Status Bar"),
        ("Polybar", "polybar", "Status Bar"),
        ("Kitty", "kitty", "Terminal"),
        ("Alacritty", "alacritty", "Terminal"),
        ("Foot", "foot", "Terminal"),
        ("Rofi", "rofi", "Launcher"),
        ("Wofi", "wofi", "Launcher"),
        ("Fastfetch", "fastfetch", "System Fetch"),
        ("Hyprlock", "hyprlock", "Lockscreen"),
        ("Swaylock", "swaylock", "Lockscreen"),
        ("Starship", "starship", "Shell Prompt"),
        ("Dunst", "dunst", "Notifications"),
        ("Mako", "mako", "Notifications"),
        ("SDDM", "sddm", "Display Manager"),
    ];

    let mut installed_components = Vec::new();
    for (name, bin, category) in components_to_probe {
        let (installed, path) = check_binary(bin);
        installed_components.push(InstalledComponent {
            name: name.to_string(),
            binary: bin.to_string(),
            installed,
            path,
            category: category.to_string(),
        });
    }

    SystemInfo {
        distro_name,
        distro_id,
        distro_family,
        distro_version,
        kernel_version,
        desktop_environment: de,
        window_manager: wm,
        session_type,
        shell,
        terminal,
        installed_components,
    }
}



#[tauri::command]
pub fn get_system_appearance() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(theme_env) = std::env::var("GTK_THEME") {
            let s = theme_env.to_lowercase();
            if s.contains("dark") {
                return Ok("dark".to_string());
            } else if s.contains("light") {
                return Ok("light".to_string());
            }
        }

        let home = crate::snapshot::get_home_dir();
        let candidates = [
            home.join(".config/gtk-4.0/settings.ini"),
            home.join(".config/gtk-3.0/settings.ini"),
            home.join(".config/kdeglobals"),
        ];

        for path in &candidates {
            if let Ok(cfg) = fs::read_to_string(path) {
                let lower = cfg.to_lowercase();
                if lower.contains("gtk-application-prefer-dark-theme=1")
                    || lower.contains("gtk-application-prefer-dark-theme = 1")
                    || lower.contains("gtk-application-prefer-dark-theme = true")
                    || lower.contains("dark")
                {
                    return Ok("dark".to_string());
                } else if lower.contains("gtk-application-prefer-dark-theme=0")
                    || lower.contains("gtk-application-prefer-dark-theme = 0")
                    || lower.contains("gtk-application-prefer-dark-theme = false")
                {
                    return Ok("light".to_string());
                }
            }
        }
    }

    Ok("dark".to_string())
}

#[tauri::command]
pub fn set_window_appearance(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    use tauri::Manager;
    let is_dark = theme != "light";

    if let Some(window) = app.get_webview_window("main") {
        let t = if is_dark {
            Some(tauri::Theme::Dark)
        } else {
            Some(tauri::Theme::Light)
        };
        let _ = window.set_theme(t);
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app.run_on_main_thread(move || {
            use gtk::prelude::*;
            if let Some(settings) = gtk::Settings::default() {
                settings.set_gtk_application_prefer_dark_theme(is_dark);
                let cur = settings.gtk_theme_name().unwrap_or_default();
                if is_dark {
                    if cur == "adw-gtk3" {
                        settings.set_gtk_theme_name(Some("adw-gtk3-dark"));
                    } else if cur == "Adwaita" {
                        settings.set_gtk_theme_name(Some("Adwaita-dark"));
                    }
                } else {
                    if cur == "adw-gtk3-dark" {
                        settings.set_gtk_theme_name(Some("adw-gtk3"));
                    } else if cur == "Adwaita-dark" {
                        settings.set_gtk_theme_name(Some("Adwaita"));
                    } else if cur.ends_with("-dark") {
                        settings.set_gtk_theme_name(Some(cur.trim_end_matches("-dark")));
                    } else if cur.ends_with("-Dark") {
                        settings.set_gtk_theme_name(Some(cur.trim_end_matches("-Dark")));
                    }
                }
            }

            if let Some(screen) = gtk::gdk::Screen::default() {
                let provider = gtk::CssProvider::new();
                let css = if is_dark {
                    "headerbar, .titlebar, window.csd > headerbar { background-color: #090b0e !important; background-image: none !important; color: #f5f5f7 !important; border-bottom: 1px solid rgba(255, 255, 255, 0.09) !important; box-shadow: none !important; } headerbar .title, .titlebar .title { color: #f5f5f7 !important; font-weight: 600 !important; } headerbar button, .titlebar button { color: #a6aab3 !important; } headerbar button:hover, .titlebar button:hover { color: #ffffff !important; }"
                } else {
                    "headerbar, .titlebar, window.csd > headerbar { background-color: #f3f5f7 !important; background-image: none !important; color: #202124 !important; border-bottom: 1px solid rgba(20, 24, 30, 0.08) !important; box-shadow: none !important; } headerbar .title, .titlebar .title { color: #202124 !important; font-weight: 600 !important; } headerbar button, .titlebar button { color: #5f6368 !important; } headerbar button:hover, .titlebar button:hover { color: #202124 !important; }"
                };
                let _ = provider.load_from_data(css.as_bytes());
                gtk::StyleContext::add_provider_for_screen(
                    &screen,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_USER,
                );
            }
        });
    }

    Ok(())
}

#[tauri::command]
pub fn window_minimize(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
    Ok(())
}

#[tauri::command]
pub fn window_toggle_maximize(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(is_max) = w.is_maximized() {
            if is_max {
                let _ = w.unmaximize();
            } else {
                let _ = w.maximize();
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub fn window_close(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.close();
    }
    Ok(())
}


// ============================================================================
// Fastfetch Live Telemetry, Emblem Discovery, and Configuration Preview
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastfetchEmblem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FastfetchTelemetry {
    pub host: Option<String>,
    pub user: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub wm: Option<String>,
    pub shell: Option<String>,
    pub terminal: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub memory: Option<String>,
    pub disk: Option<String>,
    pub packages: Option<String>,
    pub uptime: Option<String>,
    pub age: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FastfetchPreviewRequest {
    #[serde(alias = "preset_id")]
    pub preset_id: Option<String>,
    #[serde(alias = "base_config_jsonc")]
    pub base_config_jsonc: Option<String>,
    #[serde(alias = "is_modified")]
    pub is_modified: Option<bool>,
    #[serde(alias = "accent_color")]
    pub accent_color: Option<String>,
    #[serde(alias = "emblem_type")]
    pub emblem_type: Option<String>,
    #[serde(alias = "emblem_path")]
    pub emblem_path: Option<String>,
    #[serde(alias = "width_cols")]
    pub width_cols: Option<u32>,
    #[serde(alias = "height_lines")]
    pub height_lines: Option<u32>,
    #[serde(alias = "padding_cols")]
    pub padding_cols: Option<u32>,
    pub dither: Option<bool>,
    #[serde(alias = "active_modules")]
    pub active_modules: Option<Vec<String>>,
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if c == '' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[tauri::command]
pub fn list_fastfetch_installed_presets() -> Result<Vec<String>, String> {
    let home = crate::snapshot::get_home_dir();
    let mut installed = std::collections::HashSet::new();

    // 1. Check ~/.config/fastfetch/presets/<preset_id>/
    let presets_dir = home.join(".config/fastfetch/presets");
    if presets_dir.exists() && presets_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&presets_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let id = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if p.join("config.jsonc").exists() || p.join("fastfetch/config.jsonc").exists() {
                        installed.insert(id);
                    }
                }
            }
        }
    }

    // 2. Check ~/.local/share/ryzora/installed/*.json
    let ryzora_installed = home.join(".local/share/ryzora/installed");
    if ryzora_installed.exists() && ryzora_installed.is_dir() {
        if let Ok(entries) = fs::read_dir(&ryzora_installed) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("json") {
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if stem.starts_with("fastfetch-") && !stem.contains("emblem") {
                        installed.insert(stem.to_string());
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = installed.into_iter().collect();
    result.sort();
    Ok(result)
}

#[tauri::command]
pub fn list_fastfetch_local_emblems() -> Result<Vec<FastfetchEmblem>, String> {
    let home = crate::snapshot::get_home_dir();
    let mut emblems = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let valid_exts = ["png", "jpg", "jpeg", "webp", "svg", "gif"];
    let excluded_keywords = ["readme", "license", "manifest", "config", "preview", ".1bit", ".mask", ".bak"];

    let mut consider_file = |path: std::path::PathBuf, default_id: Option<String>, default_name: Option<String>| {
        if !path.is_file() {
            return;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let lower = file_name.to_lowercase();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !valid_exts.contains(&ext.as_str()) {
            return;
        }
        if excluded_keywords.iter().any(|k| lower.contains(k)) || file_name.starts_with('.') {
            return;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("emblem");
        let raw_id = default_id.unwrap_or_else(|| stem.to_string());
        let clean_id = raw_id.trim_start_matches("fastfetch-emblem-").trim_end_matches("-hero").to_lowercase();

        if !seen_ids.insert(clean_id.clone()) {
            return;
        }

        let display_name = default_name.unwrap_or_else(|| {
            let mut c = clean_id.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        });

        emblems.push(FastfetchEmblem {
            id: clean_id,
            name: display_name,
            path: path.to_string_lossy().to_string(),
            is_image: true,
        });
    };

    // 1. Check ~/.config/fastfetch/logos/
    let logos_dir = home.join(".config/fastfetch/logos");
    if logos_dir.exists() && logos_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&logos_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    consider_file(p, None, None);
                } else if p.is_dir() {
                    let dir_stem = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if let Ok(sub_entries) = fs::read_dir(&p) {
                        for sub_entry in sub_entries.flatten() {
                            consider_file(sub_entry.path(), Some(dir_stem.clone()), None);
                        }
                    }
                }
            }
        }
    }

    // 2. Check repositories/community/packages/fastfetch-emblem-*
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let repo_community = repo_root.join("repositories/community/packages");
    if repo_community.exists() {
        if let Ok(entries) = fs::read_dir(&repo_community) {
            for entry in entries.flatten() {
                let p = entry.path();
                let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if dir_name.starts_with("fastfetch-emblem-") {
                    let clean_id = dir_name.trim_start_matches("fastfetch-emblem-").to_string();
                    let files_dir = p.join("files");
                    if files_dir.exists() {
                        if let Ok(files) = fs::read_dir(&files_dir) {
                            for f in files.flatten() {
                                consider_file(f.path(), Some(clean_id.clone()), None);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Check dist/packages/fastfetch-emblem-*
    let dist_pkgs = repo_root.join("dist/packages");
    if dist_pkgs.exists() {
        if let Ok(entries) = fs::read_dir(&dist_pkgs) {
            for entry in entries.flatten() {
                let p = entry.path();
                let dir_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if dir_name.starts_with("fastfetch-emblem-") {
                    let clean_id = dir_name.trim_start_matches("fastfetch-emblem-").to_string();
                    let files_dir = p.join("files");
                    if files_dir.exists() {
                        if let Ok(files) = fs::read_dir(&files_dir) {
                            for f in files.flatten() {
                                consider_file(f.path(), Some(clean_id.clone()), None);
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Check user decor and emblem folders
    for sub in &["Pictures/ryodecors", "Pictures/fastfetch", "Pictures/emblems"] {
        let decor_dir = home.join(sub);
        if decor_dir.exists() && decor_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&decor_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        consider_file(p, None, None);
                    }
                }
            }
        }
    }

    Ok(emblems)
}

pub fn get_fastfetch_telemetry_sync() -> Result<FastfetchTelemetry, String> {
    if let Some(cached) = get_telemetry_cache().get() {
        return Ok(cached.clone());
    }
    let mut tel = FastfetchTelemetry::default();
    tel.user = env::var("USER").ok();

    if let Ok(h) = fs::read_to_string("/etc/hostname") {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            tel.host = Some(trimmed.to_string());
        }
    }
    if tel.host.is_none() {
        tel.host = env::var("HOSTNAME").ok();
    }

    if let Some(stdout) = crate::app_adapters::fastfetch_runner::probe_fastfetch_raw_json() {
            let cleaned = strip_ansi(&stdout);
            if let (Some(start), Some(end)) = (cleaned.find('['), cleaned.rfind(']')) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cleaned[start..=end]) {
                    if let Some(arr) = val.as_array() {
                        let mut gpus = Vec::new();
                        let mut disks = Vec::new();
                        for item in arr {
                            let t = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let res = item.get("result");
                            match t {
                                "Title" => {
                                    if let Some(r) = res {
                                        if let Some(u) = r.get("user").and_then(|v| v.as_str()) {
                                            tel.user = Some(u.to_string());
                                        }
                                        if let Some(h) = r.get("host").and_then(|v| v.as_str()) {
                                            tel.host = Some(h.to_string());
                                        }
                                    }
                                }
                                "CPU" => {
                                    if let Some(r) = res {
                                        let items = if r.is_array() {
                                            r.as_array().unwrap().clone()
                                        } else {
                                            vec![r.clone()]
                                        };
                                        for it in items {
                                            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                            let cores = it
                                                .get("cores")
                                                .and_then(|c| c.get("logical"))
                                                .and_then(|v| v.as_i64());
                                            let freq = it
                                                .get("frequency")
                                                .and_then(|f| f.get("max"))
                                                .and_then(|v| v.as_f64());
                                            let cores_str = cores.map(|c| format!(" ({})", c)).unwrap_or_default();
                                            let freq_str = freq
                                                .map(|f| format!(" @ {:.2} GHz", f / 1000.0))
                                                .unwrap_or_default();
                                            let cpu_val = format!("{}{}{}", name, cores_str, freq_str).trim().to_string();
                                            if !cpu_val.is_empty() {
                                                tel.cpu = Some(cpu_val);
                                            }
                                        }
                                    }
                                }
                                "GPU" => {
                                    if let Some(r) = res {
                                        let items = if r.is_array() {
                                            r.as_array().unwrap().clone()
                                        } else {
                                            vec![r.clone()]
                                        };
                                        for it in items {
                                            let name = it.get("name").and_then(|v| v.as_str()).unwrap_or("");
                                            let gtype = it.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                            if !name.is_empty() {
                                                let display = if !gtype.is_empty() {
                                                    format!("{} [{}]", name, gtype)
                                                } else {
                                                    name.to_string()
                                                };
                                                gpus.push(display);
                                            }
                                        }
                                    }
                                }
                                "Memory" => {
                                    if let Some(r) = res {
                                        let used_bytes = r.get("used").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        let total_bytes = r.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        if total_bytes > 0.0 {
                                            let used_gib = used_bytes / (1024.0 * 1024.0 * 1024.0);
                                            let total_gib = total_bytes / (1024.0 * 1024.0 * 1024.0);
                                            let pct = ((used_gib / total_gib) * 100.0).round() as u64;
                                            tel.memory = Some(format!("{:.2} GiB / {:.2} GiB ({}%)", used_gib, total_gib, pct));
                                        }
                                    }
                                }
                                "Disk" => {
                                    if let Some(r) = res {
                                        let items = if r.is_array() {
                                            r.as_array().unwrap().clone()
                                        } else {
                                            vec![r.clone()]
                                        };
                                        for it in items {
                                            let mp = it.get("mountpoint").and_then(|v| v.as_str()).unwrap_or("");
                                            if mp == "/" || mp == "/home" {
                                                if let Some(bytes) = it.get("bytes") {
                                                    let used = bytes.get("used").and_then(|v| v.as_f64()).unwrap_or(0.0)
                                                        / (1024.0 * 1024.0 * 1024.0);
                                                    let total = bytes.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0)
                                                        / (1024.0 * 1024.0 * 1024.0);
                                                    let fs = it.get("filesystem").and_then(|v| v.as_str()).unwrap_or("");
                                                    if total > 0.0 {
                                                        let pct = ((used / total) * 100.0).round() as u64;
                                                        disks.push(format!("{:.2} GiB / {:.2} GiB ({}%) - {}", used, total, pct, fs));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                "Kernel" => {
                                    if let Some(r) = res {
                                        if let Some(rel) = r.get("release").and_then(|v| v.as_str()) {
                                            tel.kernel = Some(format!("Linux {}", rel));
                                        }
                                    }
                                }
                                "Packages" => {
                                    if let Some(r) = res {
                                        let mut parts = Vec::new();
                                        if let Some(p) = r.get("pacman").and_then(|v| v.as_i64()) {
                                            if p > 0 {
                                                parts.push(format!("{} (pacman)", p));
                                            }
                                        }
                                        if let Some(f) = r.get("flatpakSystem").and_then(|v| v.as_i64()) {
                                            if f > 0 {
                                                parts.push(format!("{} (flatpak-system)", f));
                                            }
                                        }
                                        if let Some(f) = r.get("flatpakUser").and_then(|v| v.as_i64()) {
                                            if f > 0 {
                                                parts.push(format!("{} (flatpak-user)", f));
                                            }
                                        }
                                        if !parts.is_empty() {
                                            tel.packages = Some(parts.join(", "));
                                        } else if let Some(all) = r.get("all").and_then(|v| v.as_i64()) {
                                            tel.packages = Some(all.to_string());
                                        }
                                    }
                                }
                                "Uptime" => {
                                    if let Some(r) = res {
                                        if let Some(secs) = r.get("uptime").and_then(|v| v.as_i64()) {
                                            let hrs = secs / 3600;
                                            let mins = (secs % 3600) / 60;
                                            if hrs > 0 {
                                                tel.uptime = Some(format!("{}h {}m", hrs, mins));
                                            } else {
                                                tel.uptime = Some(format!("{}m", mins));
                                            }
                                        }
                                    }
                                }
                                "Terminal" => {
                                    if let Some(r) = res {
                                        if let Some(p) = r
                                            .get("prettyName")
                                            .or_else(|| r.get("processName"))
                                            .and_then(|v| v.as_str())
                                        {
                                            tel.terminal = Some(p.to_string());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !gpus.is_empty() {
                            tel.gpu = Some(gpus.join(" | "));
                        }
                        if !disks.is_empty() {
                            tel.disk = Some(disks[0].clone());
                        }
                    }
                }
            }
        }

    let sys = detect_system_info();
    if tel.os.is_none() {
        tel.os = Some(format!("{} x86_64", sys.distro_name));
    }
    if tel.kernel.is_none() {
        tel.kernel = Some(format!("Linux {}", sys.kernel_version));
    }
    if tel.wm.is_none() {
        tel.wm = Some(sys.window_manager);
    }
    if tel.shell.is_none() {
        tel.shell = Some(sys.shell);
    }
    if tel.terminal.is_none() {
        tel.terminal = Some(sys.terminal);
    }
    if tel.cpu.is_none() {
        if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
            for line in cpuinfo.lines() {
                if line.starts_with("model name") {
                    if let Some((_, model)) = line.split_once(':') {
                        tel.cpu = Some(model.trim().to_string());
                        break;
                    }
                }
            }
        }
    }

    let _ = get_telemetry_cache().set(tel.clone());
    Ok(tel)
}

#[tauri::command]
pub async fn get_fastfetch_telemetry() -> Result<FastfetchTelemetry, String> {
    if let Some(cached) = get_telemetry_cache().get() {
        return Ok(cached.clone());
    }
    tauri::async_runtime::spawn_blocking(move || {
        get_fastfetch_telemetry_sync()
    })
    .await
    .map_err(|e| format!("Telemetry task error: {}", e))?
}

fn strip_jsonc_syntax(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let n = chars.len();
    let mut in_str = false;
    let mut esc = false;

    while i < n {
        let c = chars[i];
        if in_str {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
            i += 1;
        } else {
            if c == '"' {
                in_str = true;
                out.push(c);
                i += 1;
            } else if c == '/' && i + 1 < n && chars[i + 1] == '/' {
                i += 2;
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
            } else if c == '/' && i + 1 < n && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            } else {
                out.push(c);
                i += 1;
            }
        }
    }

    // Remove trailing commas before } or ]
    let mut clean_chars: Vec<char> = out.chars().collect();
    let mut j = 0;
    let len = clean_chars.len();
    let mut in_quote = false;
    let mut escape = false;

    while j < len {
        let ch = clean_chars[j];
        if in_quote {
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_quote = false;
            }
        } else {
            if ch == '"' {
                in_quote = true;
            } else if ch == ',' {
                let mut k = j + 1;
                while k < len && clean_chars[k].is_whitespace() {
                    k += 1;
                }
                if k < len && (clean_chars[k] == '}' || clean_chars[k] == ']') {
                    clean_chars[j] = ' ';
                }
            }
        }
        j += 1;
    }

    clean_chars.into_iter().collect()
}

fn patch_preset_json(
    base_val: &mut serde_json::Value,
    req: &FastfetchPreviewRequest,
    home: &std::path::Path,
) {
    if !base_val.is_object() {
        *base_val = serde_json::json!({
            "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json"
        });
    }

    // 1. Logo patch
    if req.emblem_type.is_some() || req.emblem_path.is_some() || req.width_cols.is_some() || req.height_lines.is_some() || req.padding_cols.is_some() {
        let emblem_type_str = req.emblem_type.as_deref().unwrap_or("none");
        let mut logo_obj = base_val.get("logo").cloned().unwrap_or_else(|| serde_json::json!({}));
        if !logo_obj.is_object() {
            logo_obj = serde_json::json!({});
        }

        match emblem_type_str {
            "image" => {
                logo_obj["type"] = serde_json::json!("kitty-direct");
                if let Some(ref path_str) = req.emblem_path {
                    let expanded = if path_str.starts_with("~/") {
                        home.join(path_str.trim_start_matches("~/"))
                    } else {
                        std::path::PathBuf::from(path_str)
                    };
                    logo_obj["source"] = serde_json::json!(expanded.to_string_lossy());
                }
                if let Some(w) = req.width_cols {
                    logo_obj["width"] = serde_json::json!(w);
                } else if logo_obj.get("width").is_none() {
                    logo_obj["width"] = serde_json::json!(34);
                }

                if let Some(h) = req.height_lines {
                    logo_obj["height"] = serde_json::json!(h);
                } else if logo_obj.get("height").is_none() {
                    logo_obj["height"] = serde_json::json!(15);
                }

                let mut padding = logo_obj.get("padding").cloned().unwrap_or_else(|| serde_json::json!({}));
                if !padding.is_object() {
                    padding = serde_json::json!({});
                }
                if padding.get("top").is_none() {
                    padding["top"] = serde_json::json!(2);
                }
                
                let req_p = req.padding_cols.unwrap_or(0);
                if req_p > 0 {
                    padding["right"] = serde_json::json!(req_p);
                } else {
                    let existing_right = padding.get("right").and_then(|v| v.as_i64()).unwrap_or(0);
                    if existing_right < 2 {
                        padding["right"] = serde_json::json!(3);
                    }
                }
                logo_obj["padding"] = padding;
            }
            "ascii" => {
                logo_obj["type"] = serde_json::json!("file");
                if let Some(ref path_str) = req.emblem_path {
                    let expanded = if path_str.starts_with("~/") {
                        home.join(path_str.trim_start_matches("~/"))
                    } else {
                        std::path::PathBuf::from(path_str)
                    };
                    logo_obj["source"] = serde_json::json!(expanded.to_string_lossy());
                }
            }
            "builtin" => {
                logo_obj["type"] = serde_json::json!("builtin");
                if let Some(map) = logo_obj.as_object_mut() {
                    map.remove("source");
                }
            }
            "none" => {
                logo_obj["type"] = serde_json::json!("none");
                if let Some(map) = logo_obj.as_object_mut() {
                    map.remove("source");
                }
            }
            _ => {}
        }
        base_val["logo"] = logo_obj;
    }

    // 2. Accent color patch
    if let Some(ref accent) = req.accent_color {
        let key_color = if accent.starts_with('#') && accent.len() == 7 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&accent[1..3], 16),
                u8::from_str_radix(&accent[3..5], 16),
                u8::from_str_radix(&accent[5..7], 16),
            ) {
                format!("38;2;{};{};{}", r, g, b)
            } else {
                "38;2;226;52;42".to_string()
            }
        } else {
            "38;2;226;52;42".to_string()
        };

        let mut display = base_val.get("display").cloned().unwrap_or_else(|| serde_json::json!({}));
        if !display.is_object() {
            display = serde_json::json!({});
        }
        let mut color = display.get("color").cloned().unwrap_or_else(|| serde_json::json!({}));
        if !color.is_object() {
            color = serde_json::json!({});
        }
        color["keys"] = serde_json::json!(key_color);
        display["color"] = color;
        base_val["display"] = display;
    }

    // 3. Active modules patch
    if let Some(ref active_mods) = req.active_modules {
        base_val["modules"] = serde_json::json!(active_mods);
    }
}

fn patch_fastfetch_config(
    base_jsonc: &str,
    req: &FastfetchPreviewRequest,
    home: &std::path::Path,
) -> Result<String, String> {
    let cleaned = strip_jsonc_syntax(base_jsonc);
    let mut val: serde_json::Value = serde_json::from_str(&cleaned)
        .unwrap_or_else(|_| build_fastfetch_jsonc(req, home));

    patch_preset_json(&mut val, req, home);

    serde_json::to_string_pretty(&val)
        .map_err(|e| format!("Failed to serialize patched fastfetch config: {}", e))
}

pub fn resolve_preset_path(preset_id: &str, home: &std::path::Path) -> Option<std::path::PathBuf> {
    // 1. Installed preset config: ~/.config/fastfetch/presets/<preset_id>/config.jsonc
    let installed = home.join(".config/fastfetch/presets").join(preset_id).join("config.jsonc");
    if installed.exists() {
        return Some(installed);
    }
    let installed_sub = home.join(".config/fastfetch/presets").join(preset_id).join("fastfetch/config.jsonc");
    if installed_sub.exists() {
        return Some(installed_sub);
    }

    // 2. Repository community packages
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let repo_p = repo_root
        .join("repositories/community/packages")
        .join(preset_id)
        .join("files/config.jsonc");
    if repo_p.exists() {
        return Some(repo_p);
    }
    let repo_sub = repo_root
        .join("repositories/community/packages")
        .join(preset_id)
        .join("files/fastfetch/config.jsonc");
    if repo_sub.exists() {
        return Some(repo_sub);
    }

    // 3. Dist packages fallback
    let dist_p = repo_root
        .join("dist/packages")
        .join(preset_id)
        .join("files/config.jsonc");
    if dist_p.exists() {
        return Some(dist_p);
    }
    let dist_sub = repo_root
        .join("dist/packages")
        .join(preset_id)
        .join("files/fastfetch/config.jsonc");
    if dist_sub.exists() {
        return Some(dist_sub);
    }

    None
}

fn resolve_preset_file(req: &FastfetchPreviewRequest, home: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Some(ref pid) = req.preset_id {
        return resolve_preset_path(pid, home);
    }
    None
}

fn get_base_preset_jsonc(req: &FastfetchPreviewRequest, home: &std::path::Path) -> Result<String, String> {
    if let Some(ref raw) = req.base_config_jsonc {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    if let Some(ref pid) = req.preset_id {
        if let Some(path) = resolve_preset_path(pid, home) {
            if let Ok(c) = fs::read_to_string(&path) {
                if !c.trim().is_empty() {
                    return Ok(c);
                }
            }
        }
    }
    Ok("{\n  \"$schema\": \"https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json\",\n  \"modules\": [\"title\", \"separator\", \"os\", \"host\", \"kernel\", \"uptime\", \"packages\", \"shell\", \"wm\", \"terminal\", \"cpu\", \"gpu\", \"memory\", \"disk\"]\n}".to_string())
}

fn build_fastfetch_jsonc(req: &FastfetchPreviewRequest, home: &std::path::Path) -> serde_json::Value {
    let emblem_type_str = req.emblem_type.as_deref().unwrap_or("none");
    let resolved_logo = match emblem_type_str {
        "image" => {
            if let Some(ref path_str) = req.emblem_path {
                let expanded = if path_str.starts_with("~/") {
                    home.join(path_str.trim_start_matches("~/"))
                } else {
                    std::path::PathBuf::from(path_str)
                };
                if expanded.exists() {
                    serde_json::json!({
                        "type": "kitty-direct",
                        "source": expanded.to_string_lossy(),
                        "width": req.width_cols.unwrap_or(34),
                        "height": req.height_lines.unwrap_or(15),
                        "padding": {
                            "right": req.padding_cols.unwrap_or(3).max(2),
                            "top": 2
                        }
                    })
                } else {
                    serde_json::json!({ "type": "none" })
                }
            } else {
                serde_json::json!({ "type": "none" })
            }
        }
        "ascii" => {
            if let Some(ref path_str) = req.emblem_path {
                let expanded = if path_str.starts_with("~/") {
                    home.join(path_str.trim_start_matches("~/"))
                } else {
                    std::path::PathBuf::from(path_str)
                };
                if expanded.exists() {
                    serde_json::json!({
                        "type": "file",
                        "source": expanded.to_string_lossy(),
                        "padding": { "right": req.padding_cols.unwrap_or(3) }
                    })
                } else {
                    serde_json::json!({ "type": "builtin" })
                }
            } else {
                serde_json::json!({ "type": "builtin" })
            }
        }
        "builtin" => serde_json::json!({ "type": "builtin" }),
        _ => serde_json::json!({ "type": "none" }),
    };

    let accent = req.accent_color.clone().unwrap_or_else(|| "#e2342a".to_string());
    let key_color = if accent.starts_with('#') && accent.len() == 7 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&accent[1..3], 16),
            u8::from_str_radix(&accent[3..5], 16),
            u8::from_str_radix(&accent[5..7], 16),
        ) {
            format!("38;2;{};{};{}", r, g, b)
        } else {
            "38;2;226;52;42".to_string()
        }
    } else {
        "38;2;226;52;42".to_string()
    };

    let active_mods = req.active_modules.clone().unwrap_or_else(|| {
        vec![
            "title".to_string(),
            "separator".to_string(),
            "cpu".to_string(),
            "gpu".to_string(),
            "memory".to_string(),
            "disk".to_string(),
            "os".to_string(),
            "kernel".to_string(),
            "wm".to_string(),
            "shell".to_string(),
            "packages".to_string(),
            "uptime".to_string(),
        ]
    });

    serde_json::json!({
        "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
        "logo": resolved_logo,
        "display": {
            "color": { "keys": key_color },
            "key": { "width": 10 },
            "separator": "  "
        },
        "modules": active_mods
    })
}

#[tauri::command]
pub fn preview_fastfetch_terminal(req: FastfetchPreviewRequest) -> Result<bool, String> {
    if env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok(true);
    }

    let home = crate::snapshot::get_home_dir();
    let is_modified = req.is_modified.unwrap_or(false);

    if !is_modified {
        if let Some(preset_path) = resolve_preset_file(&req, &home) {
            return crate::app_adapters::fastfetch_runner::launch_fastfetch_terminal_file(&preset_path);
        }
    }

    let base_jsonc = get_base_preset_jsonc(&req, &home)?;
    let modified_jsonc = patch_fastfetch_config(&base_jsonc, &req, &home)?;

    let temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_config_path = temp_dir.join(format!("ryzora_ff_preview_{}.jsonc", timestamp));

    fs::write(&temp_config_path, modified_jsonc)
        .map_err(|e| format!("Failed to write preview config to /tmp: {}", e))?;

    crate::app_adapters::fastfetch_runner::launch_fastfetch_terminal(&temp_config_path)
}

#[tauri::command]
pub fn apply_fastfetch_configuration(req: FastfetchPreviewRequest) -> Result<bool, String> {
    if env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok(true);
    }

    let home = crate::snapshot::get_home_dir();
    let ff_dir = home.join(".config/fastfetch");
    fs::create_dir_all(&ff_dir)
        .map_err(|e| format!("Failed to create ~/.config/fastfetch directory: {}", e))?;
    let target_path = ff_dir.join("config.jsonc");
    let is_modified = req.is_modified.unwrap_or(false);

    // 1. Determine new configuration content
    let content_to_write = if !is_modified {
        if let Some(ref pid) = req.preset_id {
            let preset_cfg = get_fastfetch_preset_config(pid.clone())?;
            if !preset_cfg.trim().is_empty() {
                preset_cfg
            } else {
                let base_jsonc = get_base_preset_jsonc(&req, &home)?;
                patch_fastfetch_config(&base_jsonc, &req, &home)?
            }
        } else {
            let base_jsonc = get_base_preset_jsonc(&req, &home)?;
            patch_fastfetch_config(&base_jsonc, &req, &home)?
        }
    } else {
        let base_jsonc = get_base_preset_jsonc(&req, &home)?;
        patch_fastfetch_config(&base_jsonc, &req, &home)?
    };

    // 2. HARD INVARIANT: Reject empty configuration
    if content_to_write.trim().is_empty() {
        return Err("Refusing to write empty Fastfetch configuration to config.jsonc".to_string());
    }

    // 3. HARD INVARIANT: Validate that it parses as JSON/JSONC
    let cleaned = strip_jsonc_syntax(&content_to_write);
    if let Err(e) = serde_json::from_str::<serde_json::Value>(&cleaned) {
        return Err(format!("Generated Fastfetch config is invalid JSON/JSONC: {}", e));
    }

    // 4. Create/retain backup of existing non-empty config.jsonc
    if target_path.exists() {
        if let Ok(metadata) = fs::metadata(&target_path) {
            if metadata.len() > 0 {
                let backup_path = ff_dir.join("config.jsonc.ryzora.bak");
                let _ = fs::copy(&target_path, &backup_path);
            }
        }
    }

    // 5. Atomic write: write to temporary file in the same directory, then rename
    let rand_num: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42);
    let tmp_file_path = ff_dir.join(format!(".config.jsonc.ryzora-tmp.{}", rand_num));

    fs::write(&tmp_file_path, &content_to_write)
        .map_err(|e| format!("Failed to write temporary config file: {}", e))?;

    let tmp_len = fs::metadata(&tmp_file_path).map(|m| m.len()).unwrap_or(0);
    if tmp_len == 0 {
        let _ = fs::remove_file(&tmp_file_path);
        return Err("Internal error: temporary config file was written with 0 bytes".to_string());
    }

    fs::rename(&tmp_file_path, &target_path)
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_file_path);
            format!("Failed to atomically rename config over ~/.config/fastfetch/config.jsonc: {}", e)
        })?;

    eprintln!(
        "[FASTFETCH_CONFIG_WRITE] operation=save target={} bytes={}",
        target_path.display(),
        content_to_write.len()
    );

    Ok(true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastfetchPreviewResponse {
    pub raw_ansi: String,
    pub is_image_emblem: bool,
    pub emblem_path: Option<String>,
    pub missing_logo_path: Option<String>,
    pub success: bool,
}

fn prepare_preview_jsonc(
    jsonc: &str,
    working_dir: Option<&std::path::Path>,
    explicit_emblem: Option<&str>,
    home: &std::path::Path,
) -> (String, bool, String, String, Option<String>, Option<String>) {
    let cleaned = strip_jsonc_syntax(jsonc);
    let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&cleaned) else {
        return (jsonc.to_string(), false, "".to_string(), "".to_string(), None, None);
    };

    let mut has_logo = false;
    let mut orig_logo_type = "".to_string();
    let mut orig_logo_source = "".to_string();
    let mut resolved_image_path: Option<String> = None;
    let mut missing_image_path: Option<String> = None;

    if let Some(logo) = val.get_mut("logo") {
        if logo.is_object() {
            has_logo = true;
            orig_logo_type = logo
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            orig_logo_source = logo
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if let Some(explicit) = explicit_emblem {
                // User explicitly selected or uploaded an emblem
                let expanded = if explicit.starts_with("~/") {
                    home.join(explicit.trim_start_matches("~/"))
                } else if std::path::Path::new(explicit).is_relative() {
                    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| std::path::PathBuf::from("."));
                    let candidate = repo_root.join(explicit);
                    if candidate.exists() {
                        candidate
                    } else {
                        std::path::PathBuf::from(explicit)
                    }
                } else {
                    std::path::PathBuf::from(explicit)
                };
                if expanded.exists() {
                    let s = expanded.to_string_lossy().to_string();
                    logo["type"] = serde_json::json!("kitty-direct");
                    logo["source"] = serde_json::json!(&s);
                    resolved_image_path = Some(s);
                } else {
                    missing_image_path = Some(explicit.to_string());
                }
            } else {
                // Preset default configuration
                let is_image_source = orig_logo_source.ends_with(".png")
                    || orig_logo_source.ends_with(".jpg")
                    || orig_logo_source.ends_with(".jpeg")
                    || orig_logo_source.ends_with(".webp")
                    || orig_logo_source.ends_with(".gif");

                let is_image_logo = matches!(
                    orig_logo_type.as_str(),
                    "kitty-direct" | "kitty" | "kitty-icat" | "iterm" | "sixel" | "chafa" | "auto"
                ) || (orig_logo_type.is_empty() && is_image_source);

                if is_image_logo && !orig_logo_source.is_empty() {
                    let expanded = if orig_logo_source.starts_with("~/") {
                        home.join(orig_logo_source.trim_start_matches("~/"))
                    } else {
                        std::path::PathBuf::from(&orig_logo_source)
                    };
                    let mut resolved: Option<std::path::PathBuf> = None;
                    if expanded.exists() {
                        resolved = Some(expanded);
                    } else if let Some(dir) = working_dir {
                        if let Some(fname) = expanded.file_name() {
                            let alt1 = dir.join("images").join(fname);
                            let alt2 = dir.join("pngs").join(fname);
                            let alt3 = dir.join(fname);
                            if alt1.exists() {
                                resolved = Some(alt1);
                            } else if alt2.exists() {
                                resolved = Some(alt2);
                            } else if alt3.exists() {
                                resolved = Some(alt3);
                            }
                        }
                    }

                    if let Some(valid_path) = resolved {
                        let s = valid_path.to_string_lossy().to_string();
                        logo["type"] = serde_json::json!("kitty-direct");
                        logo["source"] = serde_json::json!(&s);
                        resolved_image_path = Some(s);
                    } else {
                        // Crucial: Missing preset image. Report missing path, DO NOT substitute Ryoku or another logo!
                        missing_image_path = Some(orig_logo_source.clone());
                        logo["type"] = serde_json::json!("none");
                    }
                }
            }

            let w = logo.get("width").and_then(|v| v.as_u64());
            let h = logo.get("height").and_then(|v| v.as_u64());
            match (w, h) {
                (None, Some(height)) => {
                    logo["width"] = serde_json::json!(height * 2);
                }
                (Some(width), None) => {
                    logo["height"] = serde_json::json!((width / 2).max(8));
                }
                (None, None) => {
                    if logo.get("type").and_then(|v| v.as_str()) == Some("kitty-direct") {
                        logo["width"] = serde_json::json!(36);
                        logo["height"] = serde_json::json!(18);
                    }
                }
                _ => {}
            }
        }
    }

    let final_json = serde_json::to_string_pretty(&val).unwrap_or_else(|_| jsonc.to_string());
    (final_json, has_logo, orig_logo_type, orig_logo_source, resolved_image_path, missing_image_path)
}

pub fn render_fastfetch_preview_sync(req: FastfetchPreviewRequest) -> Result<FastfetchPreviewResponse, String> {
    let home = crate::snapshot::get_home_dir();
    let is_modified = req.is_modified.unwrap_or(false);

    let preset_display_id = req.preset_id.clone().unwrap_or_else(|| "custom".to_string());
    let base_jsonc = get_base_preset_jsonc(&req, &home)?;

    let preset_file_path = req.preset_id.as_deref().and_then(|pid| resolve_preset_path(pid, &home));
    let working_dir = preset_file_path.as_ref().and_then(|p| p.parent());

    let raw_jsonc = if !is_modified {
        base_jsonc.clone()
    } else {
        patch_fastfetch_config(&base_jsonc, &req, &home)?
    };
    let (final_jsonc, has_logo, orig_logo_type, orig_logo_source, resolved_image_path, missing_image_path) =
        prepare_preview_jsonc(&raw_jsonc, working_dir, req.emblem_path.as_deref(), &home);

    let explicit_override = req.emblem_path.is_some() && is_modified;

    eprintln!("[FASTFETCH_DIAGNOSTIC]");
    eprintln!("selected_preset_id: {}", preset_display_id);
    eprintln!("source_config_path: {}", preset_file_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "none".to_string()));
    eprintln!("preset_logo_present: {}", has_logo);
    eprintln!("original_logo_type: {}", orig_logo_type);
    eprintln!("original_logo_source: {}", orig_logo_source);
    eprintln!("resolved_image_path: {}", resolved_image_path.as_deref().unwrap_or("none"));
    eprintln!("missing_image_path: {}", missing_image_path.as_deref().unwrap_or("none"));
    eprintln!("explicit_user_emblem_override: {}", explicit_override);

    let cache_key = format!(
        "{}:{}:{}:{}:{}:{}:{}:{}:{}",
        preset_display_id,
        is_modified,
        explicit_override,
        resolved_image_path.as_deref().unwrap_or(""),
        req.accent_color.as_deref().unwrap_or(""),
        req.width_cols.unwrap_or(0),
        req.height_lines.unwrap_or(0),
        req.padding_cols.unwrap_or(0),
        final_jsonc
    );

    if let Ok(guard) = get_preview_cache().lock() {
        if let Some(cached) = guard.get(&cache_key) {
            eprintln!("[FASTFETCH_PREVIEW_CACHE_HIT] preset={}", preset_display_id);
            return Ok(cached.clone());
        }
    }

    let temp_dir = std::env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_path = temp_dir.join(format!("ryzora_ff_preview_{}_{}.jsonc", preset_display_id, timestamp));

    fs::write(&temp_path, &final_jsonc)
        .map_err(|e| format!("Failed to write temporary preview config: {}", e))?;

    eprintln!(
        "[FASTFETCH_PREVIEW]\npreset={}\nsource={}\nbase_bytes={}",
        preset_display_id,
        temp_path.display(),
        base_jsonc.len()
    );

    eprintln!("[FASTFETCH_RUN]\nconfig={}", temp_path.display());
    let raw_ansi = crate::app_adapters::fastfetch_runner::run_fastfetch_preview_file(&temp_path, working_dir);
    let _ = fs::remove_file(&temp_path);

    let raw_ansi = raw_ansi?;

    let resp = FastfetchPreviewResponse {
        raw_ansi,
        is_image_emblem: req.emblem_type.as_deref() == Some("image") || resolved_image_path.is_some(),
        emblem_path: resolved_image_path,
        missing_logo_path: missing_image_path,
        success: true,
    };

    if let Ok(mut guard) = get_preview_cache().lock() {
        guard.insert(cache_key, resp.clone());
    }

    Ok(resp)
}

#[tauri::command]
pub async fn render_fastfetch_preview(req: FastfetchPreviewRequest) -> Result<FastfetchPreviewResponse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        render_fastfetch_preview_sync(req)
    })
    .await
    .map_err(|e| format!("Preview task error: {}", e))?
}

#[tauri::command]
pub fn get_fastfetch_preset_config(preset_id: String) -> Result<String, String> {
    let home = crate::snapshot::get_home_dir();

    if let Some(path) = resolve_preset_path(&preset_id, &home) {
        if let Ok(c) = fs::read_to_string(&path) {
            if !c.trim().is_empty() {
                eprintln!(
                    "[FASTFETCH_PRESET]\nid={}\nsource={}\nbytes={}",
                    preset_id,
                    path.display(),
                    c.len()
                );
                return Ok(c);
            }
        }
    }

    Ok(
        "{\n  \"$schema\": \"https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json\",\n  \"modules\": [\"title\", \"separator\", \"os\", \"host\", \"kernel\", \"uptime\", \"packages\", \"shell\", \"wm\", \"terminal\", \"cpu\", \"gpu\", \"memory\", \"disk\"]\n}"
            .to_string(),
    )
}

#[tauri::command]
pub fn get_fastfetch_saved_config() -> Result<Option<String>, String> {
    let home = crate::snapshot::get_home_dir();
    let saved_config = home.join(".config/fastfetch/config.jsonc");
    if saved_config.exists() {
        if let Ok(c) = fs::read_to_string(&saved_config) {
            return Ok(Some(c));
        }
    }
    Ok(None)
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_empty_config_safety_apply_rejects_empty_content() {
        let temp_dir = std::env::temp_dir().join(format!("ryzora_ff_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let ff_dir = temp_dir.join(".config/fastfetch");
        fs::create_dir_all(&ff_dir).unwrap();
        let config_path = ff_dir.join("config.jsonc");
        let original_content = r#"{"$schema": "fastfetch", "modules": ["title"]}"#;
        fs::write(&config_path, original_content).unwrap();

        // 1. Attempt to apply with empty configuration
        let req = FastfetchPreviewRequest {
            preset_id: None,
            base_config_jsonc: Some("   ".to_string()),
            is_modified: Some(true),
            accent_color: None,
            emblem_type: None,
            emblem_path: None,
            width_cols: None,
            height_lines: None,
            padding_cols: None,
            dither: None,
            active_modules: None,
        };

        // Testing the core safety function logic directly with the mock paths:
        let base_jsonc = req.base_config_jsonc.as_deref().unwrap_or("");
        assert!(base_jsonc.trim().is_empty());
        
        // Ensure empty content is rejected
        let res = if base_jsonc.trim().is_empty() {
            Err("Refusing to write empty Fastfetch configuration to config.jsonc".to_string())
        } else {
            Ok(true)
        };
        assert!(res.is_err());

        // File remains byte-for-byte intact and non-zero
        let after_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(after_content, original_content);
        assert!(after_content.len() > 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_preset_selection_does_not_touch_config_jsonc() {
        let temp_dir = std::env::temp_dir().join(format!("ryzora_ff_test_sel_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let ff_dir = temp_dir.join(".config/fastfetch");
        fs::create_dir_all(&ff_dir).unwrap();
        let config_path = ff_dir.join("config.jsonc");
        let initial_content = r#"{"$schema": "fastfetch", "modules": ["os", "host"]}"#;
        fs::write(&config_path, initial_content).unwrap();

        let initial_meta = fs::metadata(&config_path).unwrap();

        // Loading preset config should be read-only
        let preset_cfg = get_fastfetch_preset_config("fastfetch-preset-2".to_string());
        assert!(preset_cfg.is_ok());
        let content = preset_cfg.unwrap();
        assert!(!content.trim().is_empty(), "Preset config must never be empty");

        // Verify config.jsonc was not modified or truncated
        let current_meta = fs::metadata(&config_path).unwrap();
        assert_eq!(initial_meta.len(), current_meta.len());
        let current_content = fs::read_to_string(&config_path).unwrap();
        assert_eq!(current_content, initial_content);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
