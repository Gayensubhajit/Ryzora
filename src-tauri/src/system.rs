use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

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
pub struct FastfetchPreviewRequest {
    pub preset_id: Option<String>,
    pub accent_color: Option<String>,
    pub emblem_type: Option<String>,
    pub emblem_path: Option<String>,
    pub width_cols: Option<u32>,
    pub height_lines: Option<u32>,
    pub padding_cols: Option<u32>,
    pub dither: Option<bool>,
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
pub fn list_fastfetch_local_emblems() -> Result<Vec<FastfetchEmblem>, String> {
    let home = crate::snapshot::get_home_dir();
    let mut emblems = Vec::new();

    // 1. Check ~/.config/fastfetch/logos/
    let logos_dir = home.join(".config/fastfetch/logos");
    if logos_dir.exists() && logos_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&logos_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "svg" | "webp" | "txt") {
                        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("emblem");
                        emblems.push(FastfetchEmblem {
                            id: stem.to_string(),
                            name: stem.to_uppercase(),
                            path: p.to_string_lossy().to_string(),
                            is_image: ext != "txt",
                        });
                    }
                }
            }
        }
    }

    // 2. Check ~/.config/fastfetch/ for specific emblem files
    let ff_dir = home.join(".config/fastfetch");
    if ff_dir.exists() && ff_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&ff_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if (name.contains("emblem") || name.contains("logo"))
                        && !emblems.iter().any(|e| e.path == p.to_string_lossy())
                    {
                        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "svg" | "webp" | "txt") {
                            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("emblem");
                            emblems.push(FastfetchEmblem {
                                id: stem.to_string(),
                                name: stem.to_uppercase(),
                                path: p.to_string_lossy().to_string(),
                                is_image: ext != "txt",
                            });
                        }
                    }
                }
            }
        }
    }

    // 3. Check installed fastfetch presets under ~/.config/fastfetch/presets/
    let presets_dir = home.join(".config/fastfetch/presets");
    if presets_dir.exists() && presets_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&presets_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    if let Ok(sub_entries) = fs::read_dir(&p) {
                        for sub in sub_entries.flatten() {
                            let sub_p = sub.path();
                            if sub_p.is_file() {
                                let ext = sub_p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "svg" | "webp" | "txt") {
                                    let stem = sub_p.file_stem().and_then(|s| s.to_str()).unwrap_or("emblem");
                                    if !emblems.iter().any(|e| e.path == sub_p.to_string_lossy()) {
                                        emblems.push(FastfetchEmblem {
                                            id: stem.to_string(),
                                            name: stem.to_uppercase(),
                                            path: sub_p.to_string_lossy().to_string(),
                                            is_image: ext != "txt",
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(emblems)
}

#[tauri::command]
pub fn get_fastfetch_telemetry() -> Result<FastfetchTelemetry, String> {
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

    Ok(tel)
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
                        "width": req.width_cols.unwrap_or(28),
                        "height": req.height_lines.unwrap_or(14),
                        "padding": {
                            "left": req.padding_cols.unwrap_or(3),
                            "right": req.padding_cols.unwrap_or(3),
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
    let temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_config_path = temp_dir.join(format!("ryzora_ff_preview_{}.jsonc", timestamp));

    let config_json = build_fastfetch_jsonc(&req, &home);
    let serialized = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Failed to serialize preview config: {}", e))?;
    fs::write(&temp_config_path, serialized)
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
    let _ = fs::create_dir_all(&ff_dir);
    let target_path = ff_dir.join("config.jsonc");

    let config_json = build_fastfetch_jsonc(&req, &home);
    let serialized = serde_json::to_string_pretty(&config_json)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&target_path, serialized)
        .map_err(|e| format!("Failed to write ~/.config/fastfetch/config.jsonc: {}", e))?;

    Ok(true)
}
