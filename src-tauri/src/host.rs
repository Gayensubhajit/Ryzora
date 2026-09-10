use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use crate::system::check_binary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockscreenTargetCapability {
    pub adapter: String,
    pub name: String,
    pub category: String, // "session_lock" | "login_screen"
    pub supported: bool,
    pub reason: String,
    pub required_privilege: String, // "user" | "administrator"
    pub runtime_binary: String,
    pub binary_installed: bool,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveLockscreenInfo {
    pub session_lock_type: String, // e.g. "hyprlock", "quickshell", "swaylock", "none"
    pub session_lock_name: Option<String>,
    pub session_lock_config: Option<String>,
    pub login_screen_type: String, // e.g. "sddm", "gdm", "lightdm", "greetd", "none"
    pub login_screen_theme: Option<String>,
    pub login_screen_config: Option<String>,
    pub managed_by: Option<String>, // e.g. "Dusky", "Ryzora", "System Default"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostCapabilities {
    pub os: String,
    pub distro_id: String,
    pub distro_name: String,
    pub desktop_environment: String,
    pub compositor: String,
    pub compositor_version: Option<String>,
    pub session_type: String, // "wayland" | "x11" | "tty"
    pub session_lock_protocol: String, // "ext-session-lock-v1" | "compositor-native" | "unavailable"
    pub display_manager: String, // "sddm" | "gdm" | "lightdm" | "greetd" | "none"
    pub display_manager_service: Option<String>,
    pub display_manager_theme: Option<String>,
    pub active_lockscreen: ActiveLockscreenInfo,
    pub installed_commands: HashMap<String, bool>,
    pub supported_adapters: Vec<LockscreenTargetCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SystemIntegrationReport {
    pub desktop: String,
    pub display_server: String,
    pub session_lock_provider: String,
    pub session_lock_entrypoint: Option<String>,
    pub idle_provider: String,
    pub idle_config: Option<String>,
    pub login_manager: String,
    pub login_theme: Option<String>,
    pub login_config: Option<String>,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub warnings: Vec<String>,
}

fn parse_os_release_field(key: &str) -> Option<String> {
    let paths = ["/etc/os-release", "/usr/lib/os-release"];
    for p in &paths {
        if let Ok(content) = fs::read_to_string(p) {
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some((k, val)) = trimmed.split_once('=') {
                    if k.trim() == key {
                        return Some(val.trim_matches('"').trim_matches('\'').to_string());
                    }
                }
            }
        }
    }
    None
}

/// Detects the active display manager by inspecting systemd links and service states
pub fn detect_display_manager() -> (String, Option<String>, Option<String>) {
    // 1. Check systemd default display-manager link
    let dm_link = Path::new("/etc/systemd/system/display-manager.service");
    let target_service = if dm_link.is_symlink() || dm_link.exists() {
        fs::read_link(dm_link)
            .ok()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
    } else {
        None
    };

    let mut dm_name = "none".to_string();
    let mut service_name = target_service.clone();

    if let Some(ref svc) = target_service {
        let svc_lower = svc.to_lowercase();
        if svc_lower.contains("sddm") {
            dm_name = "sddm".to_string();
        } else if svc_lower.contains("gdm") {
            dm_name = "gdm".to_string();
        } else if svc_lower.contains("lightdm") {
            dm_name = "lightdm".to_string();
        } else if svc_lower.contains("greetd") {
            dm_name = "greetd".to_string();
        }
    }

    // 2. If not found via symlink, check running service status via check_binary & service files
    if dm_name == "none" {
        for candidate in &["sddm", "gdm", "lightdm", "greetd"] {
            let svc_path = format!("/usr/lib/systemd/system/{}.service", candidate);
            if Path::new(&svc_path).exists() {
                let (has_bin, _) = check_binary(candidate);
                if has_bin {
                    dm_name = candidate.to_string();
                    service_name = Some(format!("{}.service", candidate));
                    break;
                }
            }
        }
    }

    // 3. Inspect active theme if SDDM
    let mut active_theme = None;
    if dm_name == "sddm" {
        let conf_paths = [
            "/etc/sddm.conf.d/ryzora-theme.conf",
            "/etc/sddm.conf.d/theme.conf",
            "/etc/sddm.conf.d/kde_settings.conf",
            "/etc/sddm.conf",
        ];
        for cp in &conf_paths {
            if let Ok(content) = fs::read_to_string(cp) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("Current=") {
                        let val = trimmed.trim_start_matches("Current=").trim();
                        if !val.is_empty() {
                            active_theme = Some(val.to_string());
                            break;
                        }
                    }
                }
                if active_theme.is_some() {
                    break;
                }
            }
        }
    }

    (dm_name, service_name, active_theme)
}

/// Detects the current desktop session lock setup (e.g. hyprlock in hypridle.conf)
pub fn detect_active_session_lock(home: &Path) -> (String, Option<String>, Option<String>, Option<String>) {
    // Check hypridle.conf
    let hypridle_path = home.join(".config/hypr/hypridle.conf");
    let hyprlock_path = home.join(".config/hypr/hyprlock.conf");

    if hypridle_path.exists() {
        if let Ok(content) = fs::read_to_string(&hypridle_path) {
            if content.contains("hyprlock") {
                let theme_name = if hyprlock_path.exists() {
                    fs::read_to_string(&hyprlock_path).ok().and_then(|c| {
                        for l in c.lines() {
                            let t = l.trim();
                            if t.starts_with("source =") || t.starts_with("source=") {
                                let parts: Vec<&str> = t.split('=').collect();
                                if parts.len() >= 2 {
                                    let path_str = parts[1].trim();
                                    return Some(path_str.to_string());
                                }
                            }
                        }
                        None
                    })
                } else {
                    None
                };

                let managed_by = if content.contains("Dusky") || content.contains("edit_here") {
                    "Dusky".to_string()
                } else {
                    "User Configuration".to_string()
                };

                return (
                    "hyprlock".to_string(),
                    theme_name,
                    Some(hypridle_path.to_string_lossy().to_string()),
                    Some(managed_by),
                );
            }
        }
    }

    // Check swayidle / swaylock
    let sway_config = home.join(".config/sway/config");
    if sway_config.exists() {
        if let Ok(content) = fs::read_to_string(&sway_config) {
            if content.contains("swaylock") {
                return (
                    "swaylock".to_string(),
                    Some("Swaylock Default".to_string()),
                    Some(sway_config.to_string_lossy().to_string()),
                    Some("Sway Configuration".to_string()),
                );
            }
        }
    }

    ("none".to_string(), None, None, None)
}

/// Inspect the host system and resolve complete HostCapabilities
pub fn detect_host_capabilities_in(home: &Path) -> HostCapabilities {
    let os = parse_os_release_field("NAME").unwrap_or_else(|| "Linux".to_string());
    let distro_id = parse_os_release_field("ID").unwrap_or_else(|| "linux".to_string());
    let distro_name = parse_os_release_field("PRETTY_NAME").unwrap_or_else(|| os.clone());

    let xdg_session = env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| {
        if env::var("WAYLAND_DISPLAY").is_ok() {
            "wayland".to_string()
        } else if env::var("DISPLAY").is_ok() {
            "x11".to_string()
        } else {
            "tty".to_string()
        }
    });

    let xdg_current = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktop_session = env::var("DESKTOP_SESSION").unwrap_or_default();

    let (de, compositor, comp_ver) = if env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
        || xdg_current.to_lowercase().contains("hyprland")
        || desktop_session.to_lowercase().contains("hyprland")
    {
        ("Hyprland".to_string(), "Hyprland".to_string(), Some("0.55.x".to_string()))
    } else if env::var("SWAYSOCK").is_ok()
        || xdg_current.to_lowercase().contains("sway")
        || desktop_session.to_lowercase().contains("sway")
    {
        ("Sway".to_string(), "Sway".to_string(), None)
    } else if xdg_current.to_lowercase().contains("kde") || desktop_session.to_lowercase().contains("plasma") {
        ("KDE Plasma".to_string(), "KWin".to_string(), None)
    } else if xdg_current.to_lowercase().contains("gnome") {
        ("GNOME".to_string(), "Mutter".to_string(), None)
    } else if xdg_current.to_lowercase().contains("cosmic") {
        ("COSMIC".to_string(), "COSMIC Comp".to_string(), None)
    } else {
        (
            if !xdg_current.is_empty() { xdg_current.clone() } else { "Standalone WM".to_string() },
            if !desktop_session.is_empty() { desktop_session.clone() } else { "Unknown".to_string() },
            None,
        )
    };

    let session_lock_protocol = if xdg_session == "wayland" {
        if compositor == "Hyprland" || compositor == "Sway" || compositor == "COSMIC Comp" {
            "ext-session-lock-v1".to_string()
        } else if de == "KDE Plasma" || de == "GNOME" {
            "compositor-native".to_string()
        } else {
            "ext-session-lock-v1".to_string()
        }
    } else {
        "unavailable".to_string()
    };

    let (dm, dm_service, dm_theme) = detect_display_manager();
    let (lock_type, lock_name, lock_conf, managed_by) = detect_active_session_lock(home);

    let active_lockscreen = ActiveLockscreenInfo {
        session_lock_type: lock_type,
        session_lock_name: lock_name,
        session_lock_config: lock_conf,
        login_screen_type: dm.clone(),
        login_screen_theme: dm_theme.clone(),
        login_screen_config: dm_service.clone(),
        managed_by,
    };

    let commands_to_probe = [
        "quickshell",
        "hyprlock",
        "swaylock",
        "sddm",
        "gdm",
        "lightdm",
        "greetd",
        "pkexec", // probe
        "hyprctl",
        "loginctl",
    ];

    let mut installed_commands = HashMap::new();
    for cmd in commands_to_probe {
        let (installed, _) = check_binary(cmd);
        installed_commands.insert(cmd.to_string(), installed);
    }

    let has_quickshell = *installed_commands.get("quickshell").unwrap_or(&false);
    let has_hyprlock = *installed_commands.get("hyprlock").unwrap_or(&false);
    let has_swaylock = *installed_commands.get("swaylock").unwrap_or(&false);
    let has_sddm = *installed_commands.get("sddm").unwrap_or(&false);

    let mut supported_adapters = Vec::new();

    // 1. Quickshell Adapter
    let qs_supported = xdg_session == "wayland" && has_quickshell && (compositor == "Hyprland" || compositor == "Sway" || compositor == "COSMIC Comp");
    let qs_reason = if !has_quickshell {
        "Quickshell binary is not installed on this host system (pacman -S quickshell or build from source)".to_string()
    } else if xdg_session != "wayland" {
        "Quickshell lockscreen requires a Wayland compositor with ext-session-lock-v1 support".to_string()
    } else if compositor != "Hyprland" && compositor != "Sway" && compositor != "COSMIC Comp" {
        format!("Desktop compositor '{}' does not use ext-session-lock-v1 with Quickshell", compositor)
    } else {
        "Fully compatible with your Wayland compositor and ext-session-lock-v1".to_string()
    };
    supported_adapters.push(LockscreenTargetCapability {
        adapter: "quickshell".to_string(),
        name: "Quickshell Session Lock".to_string(),
        category: "session_lock".to_string(),
        supported: qs_supported,
        reason: qs_reason,
        required_privilege: "user".to_string(),
        runtime_binary: "quickshell".to_string(),
        binary_installed: has_quickshell,
        protocol: "ext-session-lock-v1".to_string(),
    });

    // 2. Hyprlock Adapter
    let hyprlock_supported = compositor == "Hyprland" && has_hyprlock;
    let hyprlock_reason = if !has_hyprlock {
        "hyprlock binary is not installed on this system".to_string()
    } else if compositor != "Hyprland" {
        "hyprlock requires the Hyprland compositor".to_string()
    } else {
        "Native Hyprland lockscreen runtime available".to_string()
    };
    supported_adapters.push(LockscreenTargetCapability {
        adapter: "hyprlock".to_string(),
        name: "Hyprlock Native Lock".to_string(),
        category: "session_lock".to_string(),
        supported: hyprlock_supported,
        reason: hyprlock_reason,
        required_privilege: "user".to_string(),
        runtime_binary: "hyprlock".to_string(),
        binary_installed: has_hyprlock,
        protocol: "compositor-native".to_string(),
    });

    // 3. Swaylock Adapter
    let swaylock_supported = (compositor == "Sway" || compositor == "Hyprland") && has_swaylock;
    let swaylock_reason = if !has_swaylock {
        "swaylock binary is not installed on this system".to_string()
    } else {
        "Wayland swaylock session lock runtime available".to_string()
    };
    supported_adapters.push(LockscreenTargetCapability {
        adapter: "swaylock".to_string(),
        name: "Swaylock Session Lock".to_string(),
        category: "session_lock".to_string(),
        supported: swaylock_supported,
        reason: swaylock_reason,
        required_privilege: "user".to_string(),
        runtime_binary: "swaylock".to_string(),
        binary_installed: has_swaylock,
        protocol: "ext-session-lock-v1".to_string(),
    });

    // 4. SDDM Login Screen Adapter
    let sddm_supported = dm == "sddm" || has_sddm;
    let sddm_reason = if dm != "sddm" && !has_sddm {
        "SDDM is not installed or active as your display manager".to_string()
    } else if dm != "sddm" && has_sddm {
        "SDDM binary is installed but not active as the primary display-manager service".to_string()
    } else {
        "SDDM is your active system display manager (requires administrator elevation to install system theme)".to_string()
    };
    supported_adapters.push(LockscreenTargetCapability {
        adapter: "sddm".to_string(),
        name: "SDDM Login Screen".to_string(),
        category: "login_screen".to_string(),
        supported: sddm_supported,
        reason: sddm_reason,
        required_privilege: "administrator".to_string(),
        runtime_binary: "sddm".to_string(),
        binary_installed: has_sddm,
        protocol: "sddm-greeter".to_string(),
    });

    HostCapabilities {
        os,
        distro_id,
        distro_name,
        desktop_environment: de,
        compositor,
        compositor_version: comp_ver,
        session_type: xdg_session,
        session_lock_protocol,
        display_manager: dm,
        display_manager_service: dm_service,
        display_manager_theme: dm_theme,
        active_lockscreen,
        installed_commands,
        supported_adapters,
    }
}

#[tauri::command]
pub fn get_host_capabilities() -> HostCapabilities {
    let home = crate::snapshot::get_home_dir();
    detect_host_capabilities_in(&home)
}

pub fn detect_system_integration_report_in(home: &Path) -> SystemIntegrationReport {
    let mut evidence = Vec::new();
    let mut warnings = Vec::new();

    // 1. Desktop & Compositor Detection
    let xdg_current = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktop_session = env::var("DESKTOP_SESSION").unwrap_or_default();
    let hypr_sig = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok();
    let sway_sock = env::var("SWAYSOCK").ok();

    let desktop = if let Some(sig) = hypr_sig {
        evidence.push(format!("env:HYPRLAND_INSTANCE_SIGNATURE={}", sig));
        "Hyprland".to_string()
    } else if xdg_current.to_lowercase().contains("hyprland") || desktop_session.to_lowercase().contains("hyprland") {
        evidence.push(format!("env:XDG_CURRENT_DESKTOP={}", xdg_current));
        "Hyprland".to_string()
    } else if let Some(sock) = sway_sock {
        evidence.push(format!("env:SWAYSOCK={}", sock));
        "Sway".to_string()
    } else if xdg_current.to_lowercase().contains("kde") || xdg_current.to_lowercase().contains("plasma") {
        evidence.push(format!("env:XDG_CURRENT_DESKTOP={}", xdg_current));
        "KDE Plasma".to_string()
    } else if xdg_current.to_lowercase().contains("gnome") {
        evidence.push(format!("env:XDG_CURRENT_DESKTOP={}", xdg_current));
        "GNOME".to_string()
    } else if xdg_current.to_lowercase().contains("cosmic") {
        evidence.push(format!("env:XDG_CURRENT_DESKTOP={}", xdg_current));
        "COSMIC".to_string()
    } else if !xdg_current.is_empty() {
        evidence.push(format!("env:XDG_CURRENT_DESKTOP={}", xdg_current));
        xdg_current.clone()
    } else {
        "Standalone WM".to_string()
    };

    // 2. Display Server Detection
    let wayland_display = env::var("WAYLAND_DISPLAY").ok();
    let x11_display = env::var("DISPLAY").ok();
    let display_server = if let Some(ref wd) = wayland_display {
        evidence.push(format!("env:WAYLAND_DISPLAY={}", wd));
        "Wayland".to_string()
    } else if let Some(ref d) = x11_display {
        evidence.push(format!("env:DISPLAY={}", d));
        "X11".to_string()
    } else {
        "TTY".to_string()
    };

    // 3. Idle Provider Detection
    let hypridle_conf = home.join(".config/hypr/hypridle.conf");
    let sway_conf = home.join(".config/sway/config");
    let (has_hypridle, _) = check_binary("hypridle");
    let (has_swayidle, _) = check_binary("swayidle");

    let (idle_provider, idle_config) = if hypridle_conf.exists() {
        evidence.push(format!("file:{} exists", hypridle_conf.display()));
        if has_hypridle {
            evidence.push("binary:hypridle found in PATH".to_string());
        }
        ("hypridle".to_string(), Some(hypridle_conf.to_string_lossy().to_string()))
    } else if has_hypridle {
        evidence.push("binary:hypridle found in PATH".to_string());
        ("hypridle".to_string(), None)
    } else if sway_conf.exists() || has_swayidle {
        if sway_conf.exists() {
            evidence.push(format!("file:{} exists", sway_conf.display()));
        }
        ("swayidle".to_string(), if sway_conf.exists() { Some(sway_conf.to_string_lossy().to_string()) } else { None })
    } else if desktop == "KDE Plasma" {
        evidence.push("desktop:native KDE powerdevil idle management".to_string());
        ("KDE Powerdevil".to_string(), None)
    } else if desktop == "GNOME" {
        evidence.push("desktop:native GNOME mutter idle management".to_string());
        ("GNOME Session".to_string(), None)
    } else {
        ("None".to_string(), None)
    };

    // 4. Session Lock Provider Detection
    let ryzora_active_file = home.join(".local/share/ryzora/active_lockscreen.json");
    let (has_quickshell, _) = check_binary("quickshell");
    let (has_hyprlock, _) = check_binary("hyprlock");
    let (has_swaylock, _) = check_binary("swaylock");

    let (mut session_lock_provider, mut session_lock_entrypoint) = ("None".to_string(), None);

    if ryzora_active_file.exists() {
        if let Ok(raw) = fs::read_to_string(&ryzora_active_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(qs_pkg) = val.get("quickshell").and_then(|v| v.as_str()) {
                    session_lock_provider = "Quickshell".to_string();
                    session_lock_entrypoint = val.get("quickshell_theme_path").and_then(|v| v.as_str()).map(String::from);
                    evidence.push(format!("ryzora:active_lockscreen:quickshell={}", qs_pkg));
                }
            }
        }
    }

    if session_lock_provider == "None" && hypridle_conf.exists() {
        if let Ok(content) = fs::read_to_string(&hypridle_conf) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("lock_cmd") {
                    evidence.push(format!("hypridle:{}", trimmed));
                    if trimmed.contains("quickshell") || trimmed.contains("lock.sh") {
                        session_lock_provider = "Quickshell".to_string();
                        if let Some(pos) = trimmed.find('=') {
                            session_lock_entrypoint = Some(trimmed[pos+1..].trim().to_string());
                        }
                    } else if trimmed.contains("hyprlock") {
                        session_lock_provider = "Hyprlock".to_string();
                    } else if trimmed.contains("swaylock") {
                        session_lock_provider = "Swaylock".to_string();
                    }
                    break;
                }
            }
        }
    }

    if session_lock_provider == "None" {
        if has_quickshell && (desktop == "Hyprland" || desktop == "Sway" || desktop == "COSMIC") {
            session_lock_provider = "Quickshell".to_string();
            evidence.push("binary:quickshell installed, compatible compositor detected".to_string());
        } else if has_hyprlock && desktop == "Hyprland" {
            session_lock_provider = "Hyprlock".to_string();
            evidence.push("binary:hyprlock installed, Hyprland compositor detected".to_string());
        } else if has_swaylock {
            session_lock_provider = "Swaylock".to_string();
            evidence.push("binary:swaylock installed".to_string());
        }
    }

    // 5. Login Screen Detection (SDDM / GDM / LightDM)
    let (dm_detected, dm_service, _) = detect_display_manager();
    let sddm_resolution = crate::sddm_helper::resolve_effective_sddm_theme_in(None);

    let (login_manager, login_theme, login_config) = if dm_detected == "sddm" || sddm_resolution.effective_theme.is_some() {
        let theme = sddm_resolution.effective_theme.clone();
        let config_file = sddm_resolution.effective_file.as_ref().map(|p| p.to_string_lossy().to_string());
        evidence.push(format!("login_manager:sddm active, effective_theme={:?}", theme));
        for entry in &sddm_resolution.entries {
            evidence.push(format!("sddm_dropin:{}:Current={}", entry.path, entry.theme));
        }
        if sddm_resolution.is_overridden {
            warnings.push(format!(
                "Ryzora SDDM configuration is overridden by '{}'",
                sddm_resolution.overridden_by.as_ref().map(|p| p.display().to_string()).unwrap_or_default()
            ));
        }
        ("SDDM".to_string(), theme, config_file)
    } else if dm_detected != "none" {
        evidence.push(format!("login_manager:{} service={:?}", dm_detected, dm_service));
        (dm_detected.to_uppercase(), None, dm_service)
    } else {
        ("None".to_string(), None, None)
    };

    if display_server == "Wayland" && (desktop == "KDE Plasma" || desktop == "GNOME") {
        warnings.push(format!("Desktop '{}' uses native session locking and does not implement ext-session-lock-v1 protocol for third-party lockscreens.", desktop));
    }

    let confidence = if desktop != "Standalone WM" && display_server != "TTY" && idle_provider != "None" && login_manager != "None" {
        "high".to_string()
    } else if display_server != "TTY" {
        "medium".to_string()
    } else {
        "low".to_string()
    };

    SystemIntegrationReport {
        desktop,
        display_server,
        session_lock_provider,
        session_lock_entrypoint,
        idle_provider,
        idle_config,
        login_manager,
        login_theme,
        login_config,
        confidence,
        evidence,
        warnings,
    }
}

#[tauri::command]
pub fn get_system_integration_report() -> SystemIntegrationReport {
    let home = crate::snapshot::get_home_dir();
    detect_system_integration_report_in(&home)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_test_home() -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("ryzora_host_test_{}", nanos));
        let _ = fs::create_dir_all(&path);
        path
    }

    #[test]
    fn test_detect_host_capabilities_returns_valid_structure() {
        let home = make_test_home();
        let caps = detect_host_capabilities_in(&home);
        assert!(!caps.os.is_empty());
        assert!(!caps.distro_id.is_empty());
        assert!(!caps.supported_adapters.is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn test_detect_system_integration_report_valid() {
        let home = make_test_home();
        let hypr_dir = home.join(".config/hypr");
        fs::create_dir_all(&hypr_dir).unwrap();
        fs::write(
            hypr_dir.join("hypridle.conf"),
            "general { lock_cmd = pidof quickshell || quickshell -p ~/.local/share/ryzora/lock.sh }
",
        ).unwrap();

        let report = detect_system_integration_report_in(&home);
        assert!(!report.desktop.is_empty());
        assert!(!report.display_server.is_empty());
        assert_eq!(report.idle_provider, "hypridle");
        assert_eq!(report.session_lock_provider, "Quickshell");
        assert!(!report.evidence.is_empty());
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn test_detect_active_session_lock_hypridle() {
        let home = make_test_home();
        let hypr_dir = home.join(".config/hypr");
        fs::create_dir_all(&hypr_dir).unwrap();
        let hypridle = r#"
general {
    lock_cmd = pidof hyprlock || hyprlock
}
# Managed by Dusky
"#;
        fs::write(hypr_dir.join("hypridle.conf"), hypridle).unwrap();

        let (lock_type, _, _, managed_by) = detect_active_session_lock(&home);
        assert_eq!(lock_type, "hyprlock");
        assert_eq!(managed_by, Some("Dusky".to_string()));
        let _ = fs::remove_dir_all(home);
    }

    #[test]
    fn test_detect_system_integration_report_live() {
        if let Ok(home) = std::env::var("HOME") {
        let home = std::path::PathBuf::from(home);
            let report = detect_system_integration_report_in(&home);
            println!("LIVE SYSTEM INTEGRATION REPORT:");
            println!("  Desktop: {}", report.desktop);
            println!("  Display Server: {}", report.display_server);
            println!("  Session Lock Provider: {}", report.session_lock_provider);
            println!("  Session Lock Entrypoint: {:?}", report.session_lock_entrypoint);
            println!("  Idle Provider: {}", report.idle_provider);
            println!("  Idle Config: {:?}", report.idle_config);
            println!("  Login Manager: {}", report.login_manager);
            println!("  Login Theme: {:?}", report.login_theme);
            println!("  Login Config: {:?}", report.login_config);
            println!("  Evidence ({} items):", report.evidence.len());
            for ev in &report.evidence {
                println!("    - {}", ev);
            }
            if !report.warnings.is_empty() {
                println!("  Warnings ({} items):", report.warnings.len());
                for w in &report.warnings {
                    println!("    - {}", w);
                }
            }
            assert!(!report.desktop.is_empty());
            assert!(!report.display_server.is_empty());
        }
    }
}
