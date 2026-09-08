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

fn check_binary(bin: &str) -> (bool, Option<String>) {
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
