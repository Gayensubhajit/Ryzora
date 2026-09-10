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
pub struct HostProfileFixture {
    pub id: String,
    pub name: String,
    pub is_live: bool,
    pub description: String,
    pub capabilities: HostCapabilities,
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
    let has_gdm = *installed_commands.get("gdm").unwrap_or(&false);

    // Authentication check: probe for valid PAM service
    let has_pam_auth = Path::new("/etc/pam.d/hyprlock").exists()
        || Path::new("/etc/pam.d/login").exists()
        || Path::new("/etc/pam.d/system-auth").exists()
        || Path::new("/etc/pam.d/system-local-login").exists()
        || Path::new("/etc/pam.d/gdm-password").exists();

    let mut supported_adapters = Vec::new();

    // 1. Quickshell Adapter
    let has_ext_session_lock = compositor == "Hyprland" || compositor == "Sway" || compositor == "COSMIC Comp" || compositor == "River";
    let qs_supported = xdg_session == "wayland" && has_quickshell && has_ext_session_lock && has_pam_auth;
    let qs_reason = if !has_quickshell {
        "Quickshell binary is not installed on this host system (pacman -S quickshell or build from source)".to_string()
    } else if xdg_session != "wayland" {
        "Quickshell lockscreen requires a Wayland session with ext-session-lock-v1 protocol".to_string()
    } else if !has_ext_session_lock {
        format!("Desktop compositor '{}' does not support ext-session-lock-v1 with Quickshell (Mutter and KWin use compositor-native lockscreens)", compositor)
    } else if !has_pam_auth {
        "Host PAM configuration lacks compatible login/hyprlock authentication service for Quickshell".to_string()
    } else {
        "Fully compatible with your Wayland compositor, ext-session-lock-v1, and PAM authentication".to_string()
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
    let is_gdm = dm == "gdm";
    let sddm_supported = dm == "sddm";
    let sddm_reason = if is_gdm {
        "Your system uses GDM as its active display manager. SDDM themes cannot be applied to GDM.".to_string()
    } else if dm == "lightdm" {
        "Your system uses LightDM as its active display manager. SDDM themes cannot be applied to LightDM.".to_string()
    } else if dm != "sddm" {
        "SDDM is not active as your system display manager.".to_string()
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

    // 5. GDM Login Screen Adapter (Universal Linux Architecture)
    supported_adapters.push(LockscreenTargetCapability {
        adapter: "gdm".to_string(),
        name: "GDM Login Screen".to_string(),
        category: "login_screen".to_string(),
        supported: false,
        reason: if is_gdm {
            "GDM is your active display manager, but Qylock does not provide GDM packages (only SDDM login screens). Ryzora protects GDM configuration from being modified.".to_string()
        } else {
            "GDM is not active as your system display manager.".to_string()
        },
        required_privilege: "administrator".to_string(),
        runtime_binary: "gdm".to_string(),
        binary_installed: has_gdm,
        protocol: "gdm-shell".to_string(),
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


pub fn get_compatibility_lab_profiles_in(home: &Path) -> Vec<HostProfileFixture> {
    let live_caps = detect_host_capabilities_in(home);

    let mut profiles = Vec::new();

    // 1. Live Host Profile (Real System Detection)
    profiles.push(HostProfileFixture {
        id: "live".to_string(),
        name: "Live Host System".to_string(),
        is_live: true,
        description: "Real-time capability detection from your current machine session".to_string(),
        capabilities: live_caps,
    });

    // 2. Simulated Profile: Ubuntu 24.04 LTS (GNOME / Mutter / GDM)
    let mut ubuntu_cmds = HashMap::new();
    ubuntu_cmds.insert("quickshell".to_string(), true);
    ubuntu_cmds.insert("hyprlock".to_string(), false);
    ubuntu_cmds.insert("swaylock".to_string(), false);
    ubuntu_cmds.insert("sddm".to_string(), false);
    ubuntu_cmds.insert("gdm".to_string(), true);
    ubuntu_cmds.insert("lightdm".to_string(), false);
    ubuntu_cmds.insert("pkexec".to_string(), true); // probe binary

    profiles.push(HostProfileFixture {
        id: "simulated_ubuntu_gnome".to_string(),
        name: "Simulated: Ubuntu 24.04 (GNOME 46 / Mutter / GDM)".to_string(),
        is_live: false,
        description: "Reference Ubuntu desktop with Mutter Wayland compositor and GDM display manager".to_string(),
        capabilities: HostCapabilities {
            os: "Ubuntu 24.04 LTS".to_string(),
            distro_id: "ubuntu".to_string(),
            distro_name: "Ubuntu 24.04 LTS".to_string(),
            desktop_environment: "GNOME".to_string(),
            compositor: "Mutter".to_string(),
            compositor_version: Some("46.0".to_string()),
            session_type: "wayland".to_string(),
            session_lock_protocol: "compositor-native".to_string(),
            display_manager: "gdm".to_string(),
            display_manager_service: Some("gdm.service".to_string()),
            display_manager_theme: None,
            active_lockscreen: ActiveLockscreenInfo {
                session_lock_type: "gnome-shell".to_string(),
                session_lock_name: Some("GNOME Screen Shield".to_string()),
                session_lock_config: None,
                login_screen_type: "gdm".to_string(),
                login_screen_theme: None,
                login_screen_config: Some("/etc/gdm3/custom.conf".to_string()),
                managed_by: Some("System Default".to_string()),
            },
            installed_commands: ubuntu_cmds,
            supported_adapters: vec![
                LockscreenTargetCapability {
                    adapter: "quickshell".to_string(),
                    name: "Quickshell Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: false,
                    reason: "Desktop compositor 'Mutter' does not support ext-session-lock-v1 with Quickshell (Mutter and KWin use compositor-native lockscreens)".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "quickshell".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "hyprlock".to_string(),
                    name: "Hyprlock Native Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: false,
                    reason: "hyprlock requires the Hyprland compositor".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "hyprlock".to_string(),
                    binary_installed: false,
                    protocol: "compositor-native".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "swaylock".to_string(),
                    name: "Swaylock Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: false,
                    reason: "swaylock binary is not installed on this system".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "swaylock".to_string(),
                    binary_installed: false,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "sddm".to_string(),
                    name: "SDDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "Your system uses GDM as its active display manager. SDDM themes cannot be applied to GDM.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "sddm".to_string(),
                    binary_installed: false,
                    protocol: "sddm-greeter".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "gdm".to_string(),
                    name: "GDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "GDM is your active display manager, but Qylock does not provide GDM packages (only SDDM login screens). Ryzora protects GDM configuration from being modified.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "gdm".to_string(),
                    binary_installed: true,
                    protocol: "gdm-shell".to_string(),
                },
            ],
        },
    });

    // 3. Simulated Profile: Fedora 40 (KDE Plasma 6 / KWin / SDDM)
    let mut fedora_cmds = HashMap::new();
    fedora_cmds.insert("quickshell".to_string(), true);
    fedora_cmds.insert("hyprlock".to_string(), false);
    fedora_cmds.insert("swaylock".to_string(), false);
    fedora_cmds.insert("sddm".to_string(), true);
    fedora_cmds.insert("gdm".to_string(), false);
    fedora_cmds.insert("lightdm".to_string(), false);
    fedora_cmds.insert("pkexec".to_string(), true); // probe binary

    profiles.push(HostProfileFixture {
        id: "simulated_fedora_kde".to_string(),
        name: "Simulated: Fedora 40 (KDE Plasma 6 / KWin / SDDM)".to_string(),
        is_live: false,
        description: "Reference KDE Plasma desktop with KWin compositor and SDDM display manager".to_string(),
        capabilities: HostCapabilities {
            os: "Fedora Linux 40".to_string(),
            distro_id: "fedora".to_string(),
            distro_name: "Fedora Linux 40 (Workstation Edition)".to_string(),
            desktop_environment: "KDE Plasma".to_string(),
            compositor: "KWin".to_string(),
            compositor_version: Some("6.1.0".to_string()),
            session_type: "wayland".to_string(),
            session_lock_protocol: "compositor-native".to_string(),
            display_manager: "sddm".to_string(),
            display_manager_service: Some("sddm.service".to_string()),
            display_manager_theme: Some("breeze".to_string()),
            active_lockscreen: ActiveLockscreenInfo {
                session_lock_type: "kscreenlocker".to_string(),
                session_lock_name: Some("Plasma Screen Locker".to_string()),
                session_lock_config: None,
                login_screen_type: "sddm".to_string(),
                login_screen_theme: Some("breeze".to_string()),
                login_screen_config: Some("/etc/sddm.conf".to_string()),
                managed_by: Some("System Default".to_string()),
            },
            installed_commands: fedora_cmds,
            supported_adapters: vec![
                LockscreenTargetCapability {
                    adapter: "quickshell".to_string(),
                    name: "Quickshell Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: false,
                    reason: "Desktop compositor 'KWin' does not support ext-session-lock-v1 with Quickshell (Mutter and KWin use compositor-native lockscreens)".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "quickshell".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "sddm".to_string(),
                    name: "SDDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: true,
                    reason: "SDDM is your active system display manager (requires administrator elevation to install system theme)".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "sddm".to_string(),
                    binary_installed: true,
                    protocol: "sddm-greeter".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "gdm".to_string(),
                    name: "GDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "GDM is not active as your system display manager.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "gdm".to_string(),
                    binary_installed: false,
                    protocol: "gdm-shell".to_string(),
                },
            ],
        },
    });

    // 4. Simulated Profile: Arch Linux (Hyprland / SDDM)
    let mut arch_cmds = HashMap::new();
    arch_cmds.insert("quickshell".to_string(), true);
    arch_cmds.insert("hyprlock".to_string(), true);
    arch_cmds.insert("swaylock".to_string(), true);
    arch_cmds.insert("sddm".to_string(), true);
    arch_cmds.insert("gdm".to_string(), false);
    arch_cmds.insert("lightdm".to_string(), false);
    arch_cmds.insert("pkexec".to_string(), true); // probe binary

    profiles.push(HostProfileFixture {
        id: "simulated_arch_hyprland".to_string(),
        name: "Simulated: Arch Linux (Hyprland / SDDM)".to_string(),
        is_live: false,
        description: "Reference dynamic tiling desktop with Hyprland compositor, ext-session-lock-v1, and SDDM".to_string(),
        capabilities: HostCapabilities {
            os: "Arch Linux".to_string(),
            distro_id: "arch".to_string(),
            distro_name: "Arch Linux".to_string(),
            desktop_environment: "Hyprland".to_string(),
            compositor: "Hyprland".to_string(),
            compositor_version: Some("0.55.x".to_string()),
            session_type: "wayland".to_string(),
            session_lock_protocol: "ext-session-lock-v1".to_string(),
            display_manager: "sddm".to_string(),
            display_manager_service: Some("sddm.service".to_string()),
            display_manager_theme: Some("winter".to_string()),
            active_lockscreen: ActiveLockscreenInfo {
                session_lock_type: "hyprlock".to_string(),
                session_lock_name: Some("hyprlock.conf".to_string()),
                session_lock_config: Some("~/.config/hypr/hypridle.conf".to_string()),
                login_screen_type: "sddm".to_string(),
                login_screen_theme: Some("winter".to_string()),
                login_screen_config: Some("/etc/sddm.conf.d/zz-ryzora-theme.conf".to_string()),
                managed_by: Some("Dusky".to_string()),
            },
            installed_commands: arch_cmds,
            supported_adapters: vec![
                LockscreenTargetCapability {
                    adapter: "quickshell".to_string(),
                    name: "Quickshell Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: true,
                    reason: "Fully compatible with your Wayland compositor, ext-session-lock-v1, and PAM authentication".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "quickshell".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "hyprlock".to_string(),
                    name: "Hyprlock Native Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: true,
                    reason: "Native Hyprland lockscreen runtime available".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "hyprlock".to_string(),
                    binary_installed: true,
                    protocol: "compositor-native".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "swaylock".to_string(),
                    name: "Swaylock Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: true,
                    reason: "Wayland swaylock session lock runtime available".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "swaylock".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "sddm".to_string(),
                    name: "SDDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: true,
                    reason: "SDDM is your active system display manager (requires administrator elevation to install system theme)".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "sddm".to_string(),
                    binary_installed: true,
                    protocol: "sddm-greeter".to_string(),
                },
            ],
        },
    });

    // 5. Simulated Profile: Arch Linux (Sway / GDM)
    let mut sway_cmds = HashMap::new();
    sway_cmds.insert("quickshell".to_string(), true);
    sway_cmds.insert("hyprlock".to_string(), false);
    sway_cmds.insert("swaylock".to_string(), true);
    sway_cmds.insert("sddm".to_string(), false);
    sway_cmds.insert("gdm".to_string(), true);
    sway_cmds.insert("lightdm".to_string(), false);
    sway_cmds.insert("pkexec".to_string(), true); // probe binary

    profiles.push(HostProfileFixture {
        id: "simulated_arch_sway".to_string(),
        name: "Simulated: Arch Linux (Sway / GDM)".to_string(),
        is_live: false,
        description: "Reference Sway compositor desktop running with GDM display manager".to_string(),
        capabilities: HostCapabilities {
            os: "Arch Linux".to_string(),
            distro_id: "arch".to_string(),
            distro_name: "Arch Linux".to_string(),
            desktop_environment: "Sway".to_string(),
            compositor: "Sway".to_string(),
            compositor_version: Some("1.9".to_string()),
            session_type: "wayland".to_string(),
            session_lock_protocol: "ext-session-lock-v1".to_string(),
            display_manager: "gdm".to_string(),
            display_manager_service: Some("gdm.service".to_string()),
            display_manager_theme: None,
            active_lockscreen: ActiveLockscreenInfo {
                session_lock_type: "swaylock".to_string(),
                session_lock_name: Some("swaylock".to_string()),
                session_lock_config: None,
                login_screen_type: "gdm".to_string(),
                login_screen_theme: None,
                login_screen_config: None,
                managed_by: Some("System Default".to_string()),
            },
            installed_commands: sway_cmds,
            supported_adapters: vec![
                LockscreenTargetCapability {
                    adapter: "quickshell".to_string(),
                    name: "Quickshell Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: true,
                    reason: "Fully compatible with your Wayland compositor, ext-session-lock-v1, and PAM authentication".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "quickshell".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "swaylock".to_string(),
                    name: "Swaylock Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: true,
                    reason: "Wayland swaylock session lock runtime available".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "swaylock".to_string(),
                    binary_installed: true,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "sddm".to_string(),
                    name: "SDDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "Your system uses GDM as its active display manager. SDDM themes cannot be applied to GDM.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "sddm".to_string(),
                    binary_installed: false,
                    protocol: "sddm-greeter".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "gdm".to_string(),
                    name: "GDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "GDM is your active display manager, but Qylock does not provide GDM packages (only SDDM login screens). Ryzora protects GDM configuration from being modified.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "gdm".to_string(),
                    binary_installed: true,
                    protocol: "gdm-shell".to_string(),
                },
            ],
        },
    });

    // 6. Simulated Profile: Debian 12 (X11 / LightDM / XFCE)
    let mut debian_cmds = HashMap::new();
    debian_cmds.insert("quickshell".to_string(), false);
    debian_cmds.insert("hyprlock".to_string(), false);
    debian_cmds.insert("swaylock".to_string(), false);
    debian_cmds.insert("sddm".to_string(), false);
    debian_cmds.insert("gdm".to_string(), false);
    debian_cmds.insert("lightdm".to_string(), true);
    debian_cmds.insert("pkexec".to_string(), true); // probe binary

    profiles.push(HostProfileFixture {
        id: "simulated_debian_x11".to_string(),
        name: "Simulated: Debian 12 (X11 / LightDM / XFCE)".to_string(),
        is_live: false,
        description: "Reference legacy X11 desktop environment with LightDM display manager".to_string(),
        capabilities: HostCapabilities {
            os: "Debian GNU/Linux 12 (bookworm)".to_string(),
            distro_id: "debian".to_string(),
            distro_name: "Debian GNU/Linux 12 (bookworm)".to_string(),
            desktop_environment: "XFCE".to_string(),
            compositor: "xfwm4".to_string(),
            compositor_version: None,
            session_type: "x11".to_string(),
            session_lock_protocol: "unavailable".to_string(),
            display_manager: "lightdm".to_string(),
            display_manager_service: Some("lightdm.service".to_string()),
            display_manager_theme: None,
            active_lockscreen: ActiveLockscreenInfo {
                session_lock_type: "xflock4".to_string(),
                session_lock_name: Some("xflock4".to_string()),
                session_lock_config: None,
                login_screen_type: "lightdm".to_string(),
                login_screen_theme: None,
                login_screen_config: Some("/etc/lightdm/lightdm.conf".to_string()),
                managed_by: Some("System Default".to_string()),
            },
            installed_commands: debian_cmds,
            supported_adapters: vec![
                LockscreenTargetCapability {
                    adapter: "quickshell".to_string(),
                    name: "Quickshell Session Lock".to_string(),
                    category: "session_lock".to_string(),
                    supported: false,
                    reason: "Quickshell lockscreen requires a Wayland session with ext-session-lock-v1 protocol".to_string(),
                    required_privilege: "user".to_string(),
                    runtime_binary: "quickshell".to_string(),
                    binary_installed: false,
                    protocol: "ext-session-lock-v1".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "sddm".to_string(),
                    name: "SDDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "Your system uses LightDM as its active display manager. SDDM themes cannot be applied to LightDM.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "sddm".to_string(),
                    binary_installed: false,
                    protocol: "sddm-greeter".to_string(),
                },
                LockscreenTargetCapability {
                    adapter: "lightdm".to_string(),
                    name: "LightDM Login Screen".to_string(),
                    category: "login_screen".to_string(),
                    supported: false,
                    reason: "LightDM is your active display manager, but Qylock does not provide LightDM greeter packages. Ryzora protects LightDM configuration from being modified.".to_string(),
                    required_privilege: "administrator".to_string(),
                    runtime_binary: "lightdm".to_string(),
                    binary_installed: true,
                    protocol: "lightdm-greeter".to_string(),
                },
            ],
        },
    });

    profiles
}

#[tauri::command]
pub fn get_compatibility_lab_profiles() -> Vec<HostProfileFixture> {
    let home = crate::snapshot::get_home_dir();
    get_compatibility_lab_profiles_in(&home)
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
    let state_file = home.join(".local/share/ryzora/state/active_lockscreen.json");
    let (has_quickshell, _) = check_binary("quickshell");
    let (has_hyprlock, _) = check_binary("hyprlock");
    let (has_swaylock, _) = check_binary("swaylock");

    let (mut session_lock_provider, mut session_lock_entrypoint) = ("None".to_string(), None);

    let active_symlink = home.join(".local/share/ryzora/active/lockscreen/quickshell");
    let ryzora_lock = home.join(crate::hypridle::RYZORA_LOCK_PATH);
    let user_lock_script = home.join(crate::hypridle::USER_LOCK_SCRIPT_PATH);
    let hypridle_dropin = home.join(crate::hypridle::HYPRIDLE_DROPIN_PATH);
    let hypridle_config = home.join(crate::hypridle::HYPRIDLE_RYZORA_CONFIG);

    let mut ryzora_active_pkg = None;
    let mut ryzora_theme_path = None;

    if state_file.exists() {
        if let Ok(raw) = fs::read_to_string(&state_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(qs_pkg) = val.get("quickshell").and_then(|v| v.as_str()) {
                    ryzora_active_pkg = Some(qs_pkg.to_string());
                    ryzora_theme_path = val.get("quickshell_theme_path").and_then(|v| v.as_str()).map(String::from);
                }
            }
        }
    }

    let user_script_hooked = crate::hypridle::verify_user_lock_script_hook(home);
    let hypridle_hooked = hypridle_dropin.exists() && hypridle_config.exists();

    // Ryzora Quickshell session lock is ONLY active if the on-disk triggers actually point to Ryzora:
    if ryzora_active_pkg.is_some()
        && active_symlink.exists()
        && ryzora_lock.exists()
        && (hypridle_hooked || !hypridle_conf.exists())
        && user_script_hooked
    {
        session_lock_provider = "Quickshell".to_string();
        session_lock_entrypoint = ryzora_theme_path;
        evidence.push(format!("ryzora:active_session_lock={}", ryzora_active_pkg.as_deref().unwrap_or_default()));
        evidence.push(format!("ryzora:lock_launcher={}", ryzora_lock.display()));
        if hypridle_hooked {
            evidence.push("ryzora:hypridle_override=active".to_string());
        }
        if user_lock_script.exists() {
            evidence.push(format!("ryzora:user_lock_hook={}", user_lock_script.display()));
        }
    } else if user_lock_script.exists() {
        if let Ok(content) = fs::read_to_string(&user_lock_script) {
            if content.contains("hyprlock") {
                session_lock_provider = "Hyprlock".to_string();
                session_lock_entrypoint = Some(user_lock_script.display().to_string());
                evidence.push(format!("host:user_lock_script={}", user_lock_script.display()));
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
    fn test_detect_session_lock_dusky_user_lock_script_and_ryzora_hook() {
        let home = make_test_home();
        let user_script_dir = home.join("user_scripts/hyprlock");
        fs::create_dir_all(&user_script_dir).unwrap();
        let lock_sh = user_script_dir.join("lock.sh");
        fs::write(&lock_sh, "#!/bin/bash
cp wallpaper /tmp/bg
hyprlock
").unwrap();

        // Initially Dusky Hyprlock is detected from user_scripts
        let report = detect_system_integration_report_in(&home);
        assert_eq!(report.session_lock_provider, "Hyprlock");

        // Now simulate Ryzora Apply: write state, symlink, wrapper, hook
        let state_dir = home.join(".local/share/ryzora/state");
        fs::create_dir_all(&state_dir).unwrap();
        fs::write(
            state_dir.join("active_lockscreen.json"),
            r#"{"quickshell": "lockscreen-qylock-dog-samurai", "quickshell_theme_path": "/path/theme"}"#,
        ).unwrap();

        let active_dir = home.join(".local/share/ryzora/active/lockscreen");
        fs::create_dir_all(&active_dir).unwrap();
        let theme_dir = home.join(".local/share/ryzora/lockscreens/qylock/dog-samurai");
        fs::create_dir_all(&theme_dir).unwrap();
        #[cfg(unix)]
        let _ = std::os::unix::fs::symlink(&theme_dir, active_dir.join("quickshell"));

        let bin_dir = home.join(".local/bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(bin_dir.join("ryzora-lock"), "#!/bin/bash
exit 0
").unwrap();

        // Hook the script
        crate::hypridle::hook_user_lock_script(&home).unwrap();

        // Now Quickshell must be detected as the real active session locker
        let report_active = detect_system_integration_report_in(&home);
        assert_eq!(report_active.session_lock_provider, "Quickshell");

        // Deactivate: unhook
        crate::hypridle::unhook_user_lock_script(&home).unwrap();
        let _ = fs::remove_file(state_dir.join("active_lockscreen.json"));

        // Must revert back to Dusky Hyprlock
        let report_deact = detect_system_integration_report_in(&home);
        assert_eq!(report_deact.session_lock_provider, "Hyprlock");

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

    #[test]
    fn test_get_compatibility_lab_profiles_returns_live_and_simulated_fixtures() {
        let home = make_test_home();
        let profiles = get_compatibility_lab_profiles_in(&home);
        assert!(profiles.len() >= 6, "Expected at least 6 lab profiles");

        // 1. Live profile
        let live = profiles.iter().find(|p| p.id == "live").expect("Live profile must exist");
        assert!(live.is_live, "Live profile must be marked is_live: true");
        assert_eq!(live.name, "Live Host System");

        // 2. Simulated profiles must all be marked is_live: false
        for sim in profiles.iter().filter(|p| p.id != "live") {
            assert!(!sim.is_live, "Simulated profile {} must not be marked live", sim.id);
            assert!(sim.name.starts_with("Simulated:"));
        }

        // 3. Ubuntu profile: Mutter + GDM -> Quickshell unsupported, SDDM unsupported, GDM protected
        let ubuntu = profiles.iter().find(|p| p.id == "simulated_ubuntu_gnome").expect("Ubuntu profile must exist");
        assert_eq!(ubuntu.capabilities.desktop_environment, "GNOME");
        assert_eq!(ubuntu.capabilities.compositor, "Mutter");
        assert_eq!(ubuntu.capabilities.display_manager, "gdm");
        let u_qs = ubuntu.capabilities.supported_adapters.iter().find(|a| a.adapter == "quickshell").unwrap();
        assert!(!u_qs.supported, "Quickshell must be unsupported on Mutter");
        assert!(u_qs.reason.contains("Mutter"));
        let u_sddm = ubuntu.capabilities.supported_adapters.iter().find(|a| a.adapter == "sddm").unwrap();
        assert!(!u_sddm.supported, "SDDM must be unsupported on GDM");
        assert!(u_sddm.reason.contains("GDM"));

        // 4. Fedora profile: KWin + SDDM -> Quickshell unsupported, SDDM supported
        let fedora = profiles.iter().find(|p| p.id == "simulated_fedora_kde").expect("Fedora profile must exist");
        assert_eq!(fedora.capabilities.desktop_environment, "KDE Plasma");
        assert_eq!(fedora.capabilities.compositor, "KWin");
        assert_eq!(fedora.capabilities.display_manager, "sddm");
        let f_qs = fedora.capabilities.supported_adapters.iter().find(|a| a.adapter == "quickshell").unwrap();
        assert!(!f_qs.supported, "Quickshell must be unsupported on KWin");
        let f_sddm = fedora.capabilities.supported_adapters.iter().find(|a| a.adapter == "sddm").unwrap();
        assert!(f_sddm.supported, "SDDM must be supported on SDDM");

        // 5. Arch Hyprland profile: Hyprland + SDDM -> Quickshell supported, SDDM supported
        let arch = profiles.iter().find(|p| p.id == "simulated_arch_hyprland").expect("Arch profile must exist");
        assert_eq!(arch.capabilities.compositor, "Hyprland");
        assert_eq!(arch.capabilities.display_manager, "sddm");
        let a_qs = arch.capabilities.supported_adapters.iter().find(|a| a.adapter == "quickshell").unwrap();
        assert!(a_qs.supported, "Quickshell must be supported on Hyprland");
        let a_sddm = arch.capabilities.supported_adapters.iter().find(|a| a.adapter == "sddm").unwrap();
        assert!(a_sddm.supported, "SDDM must be supported on SDDM");

        // 6. Arch Sway profile: Sway + GDM -> Quickshell supported, SDDM unsupported
        let sway = profiles.iter().find(|p| p.id == "simulated_arch_sway").expect("Sway profile must exist");
        assert_eq!(sway.capabilities.compositor, "Sway");
        let s_qs = sway.capabilities.supported_adapters.iter().find(|a| a.adapter == "quickshell").unwrap();
        assert!(s_qs.supported, "Quickshell must be supported on Sway (ext-session-lock-v1)");
        let s_sddm = sway.capabilities.supported_adapters.iter().find(|a| a.adapter == "sddm").unwrap();
        assert!(!s_sddm.supported, "SDDM must be unsupported on GDM");

        // 7. Debian profile: X11 + LightDM -> Quickshell unsupported, SDDM unsupported
        let debian = profiles.iter().find(|p| p.id == "simulated_debian_x11").expect("Debian profile must exist");
        assert_eq!(debian.capabilities.session_type, "x11");
        let d_qs = debian.capabilities.supported_adapters.iter().find(|a| a.adapter == "quickshell").unwrap();
        assert!(!d_qs.supported);
        let d_sddm = debian.capabilities.supported_adapters.iter().find(|a| a.adapter == "sddm").unwrap();
        assert!(!d_sddm.supported);

        let _ = fs::remove_dir_all(home);
    }
}
