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

    if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
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
