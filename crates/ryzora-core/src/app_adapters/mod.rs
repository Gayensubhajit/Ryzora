//! Ryzora Application Adapters — Shared Core Engine
//!
//! Orchestrates native and universal package managers across Linux distributions
//! (Pacman, AUR via paru/yay/makepkg, and Flatpak/Flathub) for both Tauri and Qt/QML frontends.

pub mod fastfetch_runner;
pub mod pacman;
pub mod transaction;
pub mod catalog;
pub mod aur;
pub mod flatpak;
pub mod cleanup;
pub mod desktop_icons;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::package_helper::{
    detect_privileged_package_helper_status_in, setup_privileged_package_helper_in,
    PACKAGE_HELPER_SYSTEM_PATH,
};

/// Zero-subprocess check for whether a binary exists in PATH or standard Linux bin directories.
pub fn check_binary(bin: &str) -> (bool, Option<String>) {
    if bin.is_empty() || bin.contains('/') {
        return (false, None);
    }

    if let Ok(test_bin_dir) = env::var("RYZORA_TEST_BIN_DIR") {
        let candidate = Path::new(&test_bin_dir).join(bin);
        if candidate.is_file() {
            return (true, Some(candidate.to_string_lossy().to_string()));
        }
    }

    if let Ok(path_var) = env::var("PATH") {
        for dir in path_var.split(':') {
            if dir.is_empty() {
                continue;
            }
            let full_path = Path::new(dir).join(bin);
            if full_path.is_file() {
                return (true, Some(full_path.to_string_lossy().to_string()));
            }
        }
    }

    let home = env::var("HOME").unwrap_or_default();
    let fallbacks = [
        format!("/usr/bin/{}", bin),
        format!("/usr/local/bin/{}", bin),
        format!("{}/.cargo/bin/{}", home, bin),
        format!("{}/.local/bin/{}", home, bin),
    ];

    for fb in &fallbacks {
        if Path::new(fb).is_file() {
            return (true, Some(fb.clone()));
        }
    }

    (false, None)
}

fn parse_os_release() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for p in &["/etc/os-release", "/usr/lib/os-release"] {
        if let Ok(content) = fs::read_to_string(p) {
            for line in content.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    map.insert(
                        k.trim().to_string(),
                        v.trim().trim_matches('"').trim_matches('\'').to_string(),
                    );
                }
            }
            if !map.is_empty() {
                break;
            }
        }
    }
    map
}

/// Information about the host application ecosystem and available package managers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostAppEcosystem {
    pub distro_id: String,
    pub distro_name: String,
    pub distro_family: String,
    pub available_managers: Vec<String>,
    pub primary_manager: String,
    pub has_pacman: bool,
    pub has_aur_helper: bool,
    pub aur_helper: Option<String>,
    pub has_flatpak: bool,
}

/// Detects available package managers on the host system using zero-subprocess file checks.
pub fn detect_host_app_ecosystem_in(_home: &Path) -> HostAppEcosystem {
    let os_info = parse_os_release();
    let distro_id = os_info
        .get("ID")
        .cloned()
        .unwrap_or_else(|| "linux".to_string())
        .to_lowercase();
    let distro_name = os_info
        .get("PRETTY_NAME")
        .or_else(|| os_info.get("NAME"))
        .cloned()
        .unwrap_or_else(|| "Linux".to_string());
    let id_like = os_info
        .get("ID_LIKE")
        .cloned()
        .unwrap_or_default()
        .to_lowercase();

    let distro_family = if distro_id == "arch"
        || distro_id == "cachyos"
        || distro_id == "garuda"
        || distro_id == "endeavouros"
        || distro_id == "manjaro"
        || id_like.contains("arch")
    {
        "arch".to_string()
    } else if distro_id == "debian"
        || distro_id == "ubuntu"
        || distro_id == "pop"
        || distro_id == "linuxmint"
        || id_like.contains("debian")
    {
        "debian".to_string()
    } else if distro_id == "fedora" || distro_id == "rhel" || distro_id == "nobara" {
        "fedora".to_string()
    } else if distro_id == "opensuse" || distro_id.contains("suse") {
        "opensuse".to_string()
    } else if distro_id == "nixos" {
        "nixos".to_string()
    } else {
        "generic".to_string()
    };

    let mut available = Vec::new();

    let (has_pacman, _) = check_binary("pacman");
    if has_pacman {
        available.push("pacman".to_string());
    }

    let (has_paru, _) = check_binary("paru");
    let (has_yay, _) = check_binary("yay");
    let aur_helper = if has_paru {
        available.push("paru".to_string());
        Some("paru".to_string())
    } else if has_yay {
        available.push("yay".to_string());
        Some("yay".to_string())
    } else {
        None
    };

    let (has_flatpak, _) = check_binary("flatpak");
    if has_flatpak {
        available.push("flatpak".to_string());
    }

    let (has_apt, _) = check_binary("apt");
    if has_apt {
        available.push("apt".to_string());
    }

    let (has_dnf, _) = check_binary("dnf");
    if has_dnf {
        available.push("dnf".to_string());
    }

    let (has_zypper, _) = check_binary("zypper");
    if has_zypper {
        available.push("zypper".to_string());
    }

    let primary = if has_pacman {
        "pacman".to_string()
    } else if has_flatpak {
        "flatpak".to_string()
    } else if has_apt {
        "apt".to_string()
    } else if has_dnf {
        "dnf".to_string()
    } else {
        "none".to_string()
    };

    HostAppEcosystem {
        distro_id,
        distro_name,
        distro_family,
        available_managers: available,
        primary_manager: primary,
        has_pacman,
        has_aur_helper: aur_helper.is_some(),
        aur_helper,
        has_flatpak,
    }
}

/// Validates a Pacman/AUR package name strictly against flag or command injection.
pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.starts_with('.')
        && !name.starts_with('+')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+' || c == '@')
}

/// Builds the exact command and arguments for privileged Pacman operations.
/// Prefers `/usr/lib/ryzora/ryzora-package-helper` when installed; falls back to `/usr/bin/pacman` with `--`.
pub fn build_pacman_privileged_args(
    operation: &str,
    package_name: &str,
    helper_installed: bool,
) -> Result<(String, Vec<String>), String> {
    if !is_valid_package_name(package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }
    let op = operation.to_lowercase();
    if op != "install" && op != "uninstall" && op != "reinstall" && op != "uninstall-deps" {
        return Err(format!(
            "Unsupported transaction operation '{}'. Must be install, uninstall, reinstall, or uninstall-deps",
            op
        ));
    }

    if helper_installed {
        Ok((
            "pkexec".to_string(),
            vec![
                PACKAGE_HELPER_SYSTEM_PATH.to_string(),
                op,
                package_name.to_string(),
            ],
        ))
    } else {
        let mut args = vec!["/usr/bin/pacman".to_string()];
        match op.as_str() {
            "install" | "reinstall" => {
                args.push("-S".to_string());
                args.push("--noconfirm".to_string());
                args.push("--".to_string());
                args.push(package_name.to_string());
            }
            "uninstall" => {
                args.push("-R".to_string());
                args.push("--noconfirm".to_string());
                args.push("--".to_string());
                args.push(package_name.to_string());
            }
            "uninstall-deps" => {
                args.push("-Rs".to_string());
                args.push("--noconfirm".to_string());
                args.push("--".to_string());
                args.push(package_name.to_string());
            }
            _ => unreachable!(),
        }
        Ok(("pkexec".to_string(), args))
    }
}

/// Installs a Pacman package using the privileged helper or Polkit-authorized Pacman.
pub fn pacman_install_package_core(package_name: &str) -> Result<serde_json::Value, String> {
    if !is_valid_package_name(package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }

    let db_lock_path = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root).join("var/lib/pacman/db.lck")
    } else {
        PathBuf::from("/var/lib/pacman/db.lck")
    };
    if db_lock_path.exists() {
        return Err("Another package manager is currently using the database (/var/lib/pacman/db.lck). Please wait until it finishes.".to_string());
    }

    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);

    let pkg_opt = pacman::get_pacman_package_details_in(package_name, &sync_dir, &local_dir)?;

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate install into sandbox local db
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        if std::env::var("RYZORA_REQUIRE_HELPER").as_deref() == Ok("1") {
            let sys_root = PathBuf::from(std::env::var("RYZORA_SYSTEM_ROOT").unwrap_or_default());
            let status = detect_privileged_package_helper_status_in(Some(&sys_root));
            if !status.installed {
                return Err("Privileged package helper not found at /usr/lib/ryzora/ryzora-package-helper. Please authorize or install the Ryzora package helper to manage system packages.".to_string());
            }
        }

        // If sync_dir contains .db files, require the package to exist in sync_dir;
        // if sync_dir is empty/absent (e.g. GUI sandbox test without mock .db archive), synthesize a sandbox record for known curated apps.
        let has_sync_dbs = sync_dir.is_dir()
            && fs::read_dir(&sync_dir)
                .map(|entries| {
                    entries
                        .flatten()
                        .any(|e| e.path().extension().and_then(|s| s.to_str()) == Some("db"))
                })
                .unwrap_or(false);

        let (name, version, description, url, license) = if let Some(pkg) = pkg_opt {
            (
                pkg.name,
                pkg.version,
                pkg.description,
                pkg.url.unwrap_or_default(),
                pkg.license.unwrap_or_default(),
            )
        } else if !has_sync_dbs {
            (
                package_name.to_string(),
                "1.0.0-1".to_string(),
                format!("Installed package {}", package_name),
                String::new(),
                "GPL".to_string(),
            )
        } else {
            return Err(format!("Package '{}' not found in pacman repositories", package_name));
        };

        let pkg_dir = local_dir.join(format!("{}-{}", name, version));
        fs::create_dir_all(&pkg_dir)
            .map_err(|e| format!("Failed to create mock package dir: {}", e))?;
        let desc_content = format!(
            "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\n{}\n\n%URL%\n{}\n\n%LICENSE%\n{}\n",
            name, version, description, url, license
        );
        fs::write(pkg_dir.join("desc"), desc_content)
            .map_err(|e| format!("Failed to write mock desc file: {}", e))?;

        invalidate_app_provider_cache(package_name);
        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Installed '{}' successfully (test mode)", package_name),
            "package_name": package_name,
            "version": version
        }));
    }

    let (has_pacman, _) = check_binary("pacman");
    if !has_pacman {
        return Err("Pacman package manager is not installed on this system.".to_string());
    }

    let mut helper_status = detect_privileged_package_helper_status_in(None);
    if !helper_status.installed && std::env::var("RYZORA_REQUIRE_HELPER").as_deref() == Ok("1") {
        return Err("Privileged package helper not found at /usr/lib/ryzora/ryzora-package-helper.".to_string());
    }
    if !helper_status.installed {
        if let Ok(st) = setup_privileged_package_helper_in(None) {
            helper_status = st;
        }
    }

    let (program, args) = build_pacman_privileged_args("install", package_name, helper_status.installed)?;
    let output = Command::new(&program)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to invoke {}: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("pacman exited with status code {:?}", output.status.code())
        };
        if detail.contains("Request dismissed")
            || detail.to_lowercase().contains("cancelled")
            || output.status.code() == Some(126)
        {
            return Err("Administrator authorization was cancelled.".to_string());
        }
        return Err(format!("Installation failed: {}", detail));
    }

    let (is_installed, installed_version) = pacman::check_installed_status_in(package_name, &local_dir);
    if !is_installed {
        return Err(format!(
            "Installation command finished, but '{}' was not detected in the local Pacman database.",
            package_name
        ));
    }

    invalidate_app_provider_cache(package_name);
    let _ = catalog::refresh_installed_state_internal();

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Installed '{}' successfully", package_name),
        "package_name": package_name,
        "version": installed_version.unwrap_or_else(|| pkg_opt.map(|p| p.version).unwrap_or_else(|| "latest".to_string()))
    }))
}

/// Uninstalls an installed Pacman or foreign (AUR) package from the local ALPM database.
pub fn pacman_uninstall_package_core(package_name: &str) -> Result<serde_json::Value, String> {
    if !is_valid_package_name(package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }

    let db_lock_path = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root).join("var/lib/pacman/db.lck")
    } else {
        PathBuf::from("/var/lib/pacman/db.lck")
    };
    if db_lock_path.exists() {
        return Err("Another package manager is currently using the database (/var/lib/pacman/db.lck). Please wait until it finishes.".to_string());
    }

    let local_dir = pacman::resolve_local_dir(None);
    let (is_installed, ver) = pacman::check_installed_status_in(package_name, &local_dir);

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate uninstall by removing mock package dir
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        if !is_installed {
            return Err(format!("Package '{}' is not installed", package_name));
        }
        if let Some(v) = ver {
            let pkg_dir = local_dir.join(format!("{}-{}", package_name, v));
            if pkg_dir.exists() {
                let _ = fs::remove_dir_all(&pkg_dir);
            }
        }
        invalidate_app_provider_cache(package_name);
        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Uninstalled '{}' successfully (test mode)", package_name),
            "package_name": package_name
        }));
    }

    if !is_installed {
        return Err(format!("Package '{}' is not installed", package_name));
    }

    let mut helper_status = detect_privileged_package_helper_status_in(None);
    if !helper_status.installed && std::env::var("RYZORA_REQUIRE_HELPER").as_deref() == Ok("1") {
        return Err("Privileged package helper not found at /usr/lib/ryzora/ryzora-package-helper.".to_string());
    }
    if !helper_status.installed {
        if let Ok(st) = setup_privileged_package_helper_in(None) {
            helper_status = st;
        }
    }

    let (program, args) = build_pacman_privileged_args("uninstall", package_name, helper_status.installed)?;
    let output = Command::new(&program)
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to invoke {}: {}", program, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("pacman exited with status code {:?}", output.status.code())
        };

        if detail.contains("Request dismissed")
            || detail.to_lowercase().contains("cancelled")
            || output.status.code() == Some(126)
        {
            return Err("Administrator authorization was cancelled.".to_string());
        }

        let detail_lower = detail.to_ascii_lowercase();
        if detail_lower.contains("target not found") || detail_lower.contains("was not found") {
            let (still_installed, _) = pacman::check_installed_status_in(package_name, &local_dir);
            if !still_installed {
                invalidate_app_provider_cache(package_name);
                return Ok(serde_json::json!({
                    "success": true,
                    "message": format!("Package '{}' is already absent (no-op)", package_name),
                    "package_name": package_name,
                    "already_absent": true
                }));
            }
        }

        return Err(format!("Uninstallation failed: {}", detail));
    }

    let (still_installed, _) = pacman::check_installed_status_in(package_name, &local_dir);
    if still_installed {
        return Err(format!(
            "Uninstall command finished, but '{}' is still present in the local Pacman database.",
            package_name
        ));
    }

    invalidate_app_provider_cache(package_name);
    let _ = catalog::refresh_installed_state_internal();

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Uninstalled '{}' successfully", package_name),
        "package_name": package_name
    }))
}

/// Launches a desktop application by resolving its authentic XDG `.desktop` entry,
/// stripping Freedesktop field codes, or checking verified binaries.
/// Never blindly invokes `gtk-launch` when no `.desktop` entry exists on disk.
pub fn launch_desktop_app_core(package_id: &str) -> Result<bool, String> {
    if package_id.trim().is_empty() {
        return Err("Invalid application identifier".to_string());
    }

    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        // In sandbox mode, verify the package is actually installed in sandbox ALPM or Flatpak
        let local_dir = pacman::resolve_local_dir(None);
        let (alpm_installed, _) = pacman::check_installed_status_in(package_id, &local_dir);
        let fp_installed = flatpak::get_installed_flatpak_ids().contains(&package_id.to_lowercase());
        if alpm_installed || fp_installed {
            return Ok(true);
        }
        return Err(format!(
            "Could not launch application '{}': package is not installed.",
            package_id
        ));
    }

    // 1. If package_id is an installed Flatpak ID, launch via `flatpak run`
    if flatpak::is_valid_flatpak_id(package_id)
        && flatpak::get_installed_flatpak_ids().contains(&package_id.to_lowercase())
    {
        flatpak::run_flatpak_app(package_id)?;
        return Ok(true);
    }

    // 2. Resolve authentic desktop entry on disk
    if let Some(info) = desktop_icons::resolve_desktop_icon_for_app(package_id) {
        if !info.desktop_file.is_empty() && Path::new(&info.desktop_file).is_file() {
            // Prefer parsing Exec= command and verifying binary exists
            if let Some(exec_cmd) = &info.exec {
                let tokens: Vec<&str> = exec_cmd
                    .split_whitespace()
                    .filter(|tok| !tok.starts_with('%'))
                    .collect();

                if let Some((binary, args)) = tokens.split_first() {
                    let clean_bin = binary.trim_matches('"').trim_matches('\'');
                    if !clean_bin.is_empty() {
                        let bin_exists = if clean_bin.starts_with('/') {
                            Path::new(clean_bin).is_file()
                        } else {
                            check_binary(clean_bin).0
                        };
                        if bin_exists {
                            if Command::new(clean_bin).args(args).spawn().is_ok() {
                                return Ok(true);
                            }
                        }
                    }
                }
            }

            // Or launch via gtk-launch ONLY because we verified info.desktop_file exists on disk
            if let Some(desktop_file_name) = Path::new(&info.desktop_file).file_name().and_then(|n| n.to_str()) {
                if check_binary("gtk-launch").0 {
                    if Command::new("gtk-launch").arg(desktop_file_name).spawn().is_ok() {
                        return Ok(true);
                    }
                }
            }
        }
    }

    // 3. Check known binary aliases ONLY if the binary actually exists on disk
    let candidate_bins: Vec<&str> = match package_id {
        "visual-studio-code-bin" | "code" => vec!["code", "code-oss"],
        "cursor" | "cursor-bin" => vec!["cursor"],
        "obs-studio" => vec!["obs"],
        "telegram-desktop" => vec!["telegram-desktop"],
        "neovim" => vec!["nvim", "neovim"],
        _ => vec![package_id],
    };

    for bin in candidate_bins {
        let (exists, resolved_path) = check_binary(bin);
        if exists {
            let target = resolved_path.as_deref().unwrap_or(bin);
            if Command::new(target).spawn().is_ok() {
                return Ok(true);
            }
        }
    }

    Err(format!(
        "Could not launch application '{}'. Desktop entry or executable was not found on system.",
        package_id
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppProviderSource {
    pub provider_id: String, // "pacman" | "aur" | "flatpak"
    pub name: String,        // "Pacman" | "AUR" | "Flatpak"
    pub short_name: String,  // "Pacman · extra" | "AUR · User contributed" | "Flathub · Flatpak"
    pub repository: String,  // "extra" | "aur" | "flathub"
    pub description: String,
    pub target_id: String,   // actual ID to install/query
    pub version: Option<String>,
    pub is_installed: bool,
}

static RESOLVE_PROVIDERS_CACHE: std::sync::LazyLock<
    std::sync::RwLock<HashMap<String, Vec<AppProviderSource>>>,
> = std::sync::LazyLock::new(|| std::sync::RwLock::new(HashMap::new()));

pub fn invalidate_app_provider_cache(package_id: &str) {
    if let Ok(mut cache) = RESOLVE_PROVIDERS_CACHE.write() {
        if package_id.is_empty() || package_id == "*" {
            cache.clear();
        } else {
            cache.remove(&package_id.trim().to_lowercase());
        }
    }
}

/// Known canonical Flatpak IDs for popular desktop applications so offline/fast
/// lookups can also match installed Flatpaks without waiting on network search.
pub fn known_flatpak_id_for_app(app_id: &str) -> Option<&'static str> {
    match app_id.trim().to_lowercase().as_str() {
        "firefox" => Some("org.mozilla.firefox"),
        "chromium" => Some("org.chromium.Chromium"),
        "code" | "visual-studio-code-bin" => Some("com.visualstudio.code"),
        "discord" => Some("com.discordapp.Discord"),
        "steam" => Some("com.valvesoftware.Steam"),
        "vlc" => Some("org.videolan.VLC"),
        "gimp" => Some("org.gimp.GIMP"),
        "obs-studio" => Some("com.obsproject.Studio"),
        "blender" => Some("org.blender.Blender"),
        "inkscape" => Some("org.inkscape.Inkscape"),
        "spotify" => Some("com.spotify.Client"),
        "telegram-desktop" => Some("org.telegram.desktop"),
        "thunderbird" => Some("org.mozilla.Thunderbird"),
        "mpv" => Some("io.mpv.Mpv"),
        "amarok" => Some("org.kde.amarok"),
        _ => None,
    }
}

/// Known canonical AUR package names for curated apps that live in AUR when not in sync repos.
pub fn known_aur_id_for_app(app_id: &str) -> Option<&'static str> {
    match app_id.trim().to_lowercase().as_str() {
        "cursor" | "cursor-bin" => Some("cursor-bin"),
        "visual-studio-code-bin" => Some("visual-studio-code-bin"),
        "spotify" => Some("spotify"),
        _ => None,
    }
}

/// Dynamically discovers authentic installation sources for an application.
/// Cross-queries Pacman ALPM, AUR RPC v5, and Flathub metadata.
pub fn resolve_app_providers_sync(package_id: &str, display_name: Option<&str>) -> Vec<AppProviderSource> {
    let pkg_trimmed = package_id.trim();
    if pkg_trimmed.is_empty() {
        return Vec::new();
    }

    let cache_key = pkg_trimmed.to_lowercase();
    if std::env::var("RYZORA_SYSTEM_ROOT").is_err() {
        if let Ok(cache) = RESOLVE_PROVIDERS_CACHE.read() {
            if let Some(cached) = cache.get(&cache_key) {
                return cached.clone();
            }
        }
    }

    // Parallel resolution across Pacman, AUR, and Flatpak
    let (pacman_match, aur_match, flatpak_match) = std::thread::scope(|s| {
        // 1. Pacman resolution
        let pacman_handle = s.spawn(|| {
            let candidates = if pkg_trimmed.contains('.') {
                let tail = pkg_trimmed.split('.').last().unwrap_or(pkg_trimmed);
                vec![pkg_trimmed, tail]
            } else {
                vec![pkg_trimmed]
            };

            for candidate in candidates {
                if std::env::var("RYZORA_SYSTEM_ROOT").is_err() {
                    if let Some(item) = catalog::get_catalog_item_by_id(candidate) {
                        let repo = item.repository.to_lowercase();
                        if !repo.is_empty() && repo != "installed" && repo != "local" && repo != "aur" {
                            let short_name = format!("Pacman · {}", item.repository);
                            return Some(AppProviderSource {
                                provider_id: "pacman".to_string(),
                                name: "Pacman".to_string(),
                                short_name,
                                repository: item.repository,
                                description: "Official Arch Linux repository".to_string(),
                                target_id: item.id,
                                version: Some(item.version),
                                is_installed: item.is_installed,
                            });
                        }
                    }
                }

                let sync_dir = pacman::resolve_sync_dir(None);
                let local_dir = pacman::resolve_local_dir(None);
                if let Ok(Some(pkg)) = pacman::get_pacman_package_details_in(candidate, &sync_dir, &local_dir) {
                    let repo = pkg.repository.to_lowercase();
                    if repo != "installed" && repo != "local" && repo != "aur" {
                        let short_name = format!("Pacman · {}", pkg.repository);
                        return Some(AppProviderSource {
                            provider_id: "pacman".to_string(),
                            name: "Pacman".to_string(),
                            short_name,
                            repository: pkg.repository,
                            description: "Official Arch Linux repository".to_string(),
                            target_id: pkg.name,
                            version: Some(pkg.version),
                            is_installed: pkg.is_installed,
                        });
                    }
                }
            }
            None
        });

        // 2. AUR resolution (strict: must return real record from official AUR RPC or known AUR mapping in offline/test mode)
        let aur_handle = s.spawn(|| {
            let aur_candidate = if let Some(known) = known_aur_id_for_app(pkg_trimmed) {
                known
            } else if pkg_trimmed.contains('.') {
                pkg_trimmed.split('.').last().unwrap_or(pkg_trimmed)
            } else {
                pkg_trimmed
            };

            if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
                if let Some(known) = known_aur_id_for_app(pkg_trimmed) {
                    let local_dir = pacman::resolve_local_dir(None);
                    let (is_inst, ver) = pacman::check_installed_status_in(known, &local_dir);
                    return Some(AppProviderSource {
                        provider_id: "aur".to_string(),
                        name: "AUR".to_string(),
                        short_name: "AUR · User contributed".to_string(),
                        repository: "aur".to_string(),
                        description: "Arch User Repository (unprivileged build)".to_string(),
                        target_id: known.to_string(),
                        version: ver.or_else(|| Some("latest".to_string())),
                        is_installed: is_inst,
                    });
                }
                return None;
            }

            if let Ok(Some(info)) = aur::aur_get_info_rpc(aur_candidate) {
                return Some(AppProviderSource {
                    provider_id: "aur".to_string(),
                    name: "AUR".to_string(),
                    short_name: "AUR · User contributed".to_string(),
                    repository: "aur".to_string(),
                    description: "Arch User Repository (unprivileged build)".to_string(),
                    target_id: info.name,
                    version: Some(info.version),
                    is_installed: info.is_installed,
                });
            }
            None
        });

        // 3. Flatpak / Flathub resolution (strict: valid reverse-DNS ID verified on Flathub or installed locally)
        let flatpak_handle = s.spawn(|| {
            if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
                let candidate_fp = if flatpak::is_valid_flatpak_id(pkg_trimmed) {
                    Some(pkg_trimmed.to_string())
                } else {
                    known_flatpak_id_for_app(pkg_trimmed).map(|s| s.to_string())
                };
                if let Some(fp_id) = candidate_fp {
                    let is_inst = flatpak::get_installed_flatpak_ids().contains(&fp_id.to_lowercase());
                    return Some(AppProviderSource {
                        provider_id: "flatpak".to_string(),
                        name: "Flatpak".to_string(),
                        short_name: "Flathub · Flatpak".to_string(),
                        repository: "flathub".to_string(),
                        description: "Universal Flatpak container".to_string(),
                        target_id: fp_id,
                        version: Some("1.0.0".to_string()),
                        is_installed: is_inst,
                    });
                }
                return None;
            }

            if let Some(app) = flatpak::find_flathub_match(pkg_trimmed, display_name) {
                return Some(AppProviderSource {
                    provider_id: "flatpak".to_string(),
                    name: "Flatpak".to_string(),
                    short_name: "Flathub · Flatpak".to_string(),
                    repository: "flathub".to_string(),
                    description: "Universal Flatpak container".to_string(),
                    target_id: app.id,
                    version: Some(app.version),
                    is_installed: app.is_installed,
                });
            }
            None
        });

        (
            pacman_handle.join().unwrap_or(None),
            aur_handle.join().unwrap_or(None),
            flatpak_handle.join().unwrap_or(None),
        )
    });

    let mut results = Vec::new();
    if let Some(p) = pacman_match {
        results.push(p);
    }
    if let Some(a) = aur_match {
        results.push(a);
    }
    if let Some(f) = flatpak_match {
        results.push(f);
    }

    if std::env::var("RYZORA_SYSTEM_ROOT").is_err() {
        if let Ok(mut cache) = RESOLVE_PROVIDERS_CACHE.write() {
            cache.insert(cache_key, results.clone());
        }
    }

    results
}

/// Fast local (zero-network) provider list construction for catalogue rendering.
/// Inspects local ALPM sync/installed databases and local Flatpak installations.
pub fn resolve_local_app_providers(
    app_id: &str,
    default_repo: &str,
    default_version: &str,
) -> Vec<AppProviderSource> {
    let local_dir = pacman::resolve_local_dir(None);
    let sync_dir = pacman::resolve_sync_dir(None);
    let installed_flatpaks = flatpak::get_installed_flatpak_ids();

    let mut providers = Vec::new();

    // 1. Check if it's an AUR-first curated app or official Pacman app
    if let Some(aur_target) = known_aur_id_for_app(app_id) {
        // Also check if a CachyOS/custom sync repo actually provides it via Pacman
        if let Ok(Some(sync_pkg)) = pacman::get_pacman_package_details_in(aur_target, &sync_dir, &local_dir) {
            let repo_low = sync_pkg.repository.to_lowercase();
            if repo_low != "installed" && repo_low != "local" && repo_low != "aur" {
                providers.push(AppProviderSource {
                    provider_id: "pacman".to_string(),
                    name: "Pacman".to_string(),
                    short_name: format!("Pacman · {}", sync_pkg.repository),
                    repository: sync_pkg.repository,
                    description: "Official repository package".to_string(),
                    target_id: sync_pkg.name,
                    version: Some(sync_pkg.version),
                    is_installed: sync_pkg.is_installed,
                });
            }
        }
        let (aur_inst, aur_ver) = pacman::check_installed_status_in(aur_target, &local_dir);
        providers.push(AppProviderSource {
            provider_id: "aur".to_string(),
            name: "AUR".to_string(),
            short_name: "AUR · User contributed".to_string(),
            repository: "aur".to_string(),
            description: "Arch User Repository (unprivileged build)".to_string(),
            target_id: aur_target.to_string(),
            version: aur_ver.or_else(|| Some(default_version.to_string())),
            is_installed: aur_inst,
        });
    } else if !flatpak::is_valid_flatpak_id(app_id) {
        let (pac_inst, pac_ver) = pacman::check_installed_status_in(app_id, &local_dir);
        let repo = if default_repo.is_empty() || default_repo == "system" {
            "extra".to_string()
        } else {
            default_repo.to_string()
        };
        providers.push(AppProviderSource {
            provider_id: "pacman".to_string(),
            name: "Pacman".to_string(),
            short_name: format!("Pacman · {}", repo),
            repository: repo,
            description: "Official Arch/CachyOS repository".to_string(),
            target_id: app_id.to_string(),
            version: pac_ver.or_else(|| Some(default_version.to_string())),
            is_installed: pac_inst,
        });
    }

    // 2. Check Flatpak option if known reverse-DNS ID or app_id is reverse-DNS
    let fp_target = if flatpak::is_valid_flatpak_id(app_id) {
        Some(app_id.to_string())
    } else {
        known_flatpak_id_for_app(app_id).map(|s| s.to_string())
    };

    if let Some(fp_id) = fp_target {
        let fp_inst = installed_flatpaks.contains(&fp_id.to_lowercase());
        providers.push(AppProviderSource {
            provider_id: "flatpak".to_string(),
            name: "Flatpak".to_string(),
            short_name: "Flathub · Flatpak".to_string(),
            repository: "flathub".to_string(),
            description: "Universal Flatpak container".to_string(),
            target_id: fp_id,
            version: Some(default_version.to_string()),
            is_installed: fp_inst,
        });
    }

    providers
}

/// Checks if an application is installed across Pacman, AUR, or Flatpak.
/// Returns `(is_installed, installed_version, installed_provider_id, installed_target_id)`.
pub fn check_app_installed_across_providers(
    app_id: &str,
) -> (bool, Option<String>, Option<String>, Option<String>) {
    let trimmed = app_id.trim();
    if trimmed.is_empty() {
        return (false, None, None, None);
    }

    let (explicit_prov, clean_id) = parse_provider_prefixed_id(trimmed);
    let local_dir = pacman::resolve_local_dir(None);
    let sync_dir = pacman::resolve_sync_dir(None);

    // 1. If explicit provider requested, check that provider specifically
    if let Some(prov) = explicit_prov {
        match prov.as_str() {
            "flatpak" => {
                if flatpak::get_installed_flatpak_ids().contains(&clean_id.to_lowercase()) {
                    let ver = flatpak::list_installed_flatpak_apps()
                        .ok()
                        .and_then(|list| {
                            list.into_iter()
                                .find(|a| a.id.eq_ignore_ascii_case(&clean_id))
                                .map(|a| a.version)
                        });
                    return (true, ver, Some("flatpak".to_string()), Some(clean_id));
                }
                return (false, None, None, None);
            }
            "pacman" | "aur" => {
                let (inst, ver) = pacman::check_installed_status_in(&clean_id, &local_dir);
                if inst {
                    return (true, ver, Some(prov), Some(clean_id));
                }
                return (false, None, None, None);
            }
            _ => {}
        }
    }

    // 2. Check ALPM local database for direct package name or known AUR package name
    let mut alpm_candidates = vec![clean_id.as_str()];
    if let Some(aur_id) = known_aur_id_for_app(&clean_id) {
        if !alpm_candidates.contains(&aur_id) {
            alpm_candidates.push(aur_id);
        }
    }

    for cand in alpm_candidates {
        let (inst, ver) = pacman::check_installed_status_in(cand, &local_dir);
        if inst {
            // Determine whether it's in official sync repos or foreign (AUR)
            let is_official = pacman::get_pacman_package_details_in(cand, &sync_dir, &local_dir)
                .ok()
                .flatten()
                .map(|p| {
                    let r = p.repository.to_lowercase();
                    r != "installed" && r != "local" && r != "aur"
                })
                .unwrap_or(false);
            let prov = if is_official || known_aur_id_for_app(&clean_id).is_none() {
                "pacman".to_string()
            } else {
                "aur".to_string()
            };
            return (true, ver, Some(prov), Some(cand.to_string()));
        }
    }

    // 3. Check Flatpak installed applications (either direct reverse-DNS ID or known Flatpak mapping)
    let fp_candidate = if flatpak::is_valid_flatpak_id(&clean_id) {
        Some(clean_id.clone())
    } else {
        known_flatpak_id_for_app(&clean_id).map(|s| s.to_string())
    };

    if let Some(fp_id) = fp_candidate {
        if flatpak::get_installed_flatpak_ids().contains(&fp_id.to_lowercase()) {
            let ver = flatpak::list_installed_flatpak_apps()
                .ok()
                .and_then(|list| {
                    list.into_iter()
                        .find(|a| a.id.eq_ignore_ascii_case(&fp_id))
                        .map(|a| a.version)
                });
            return (true, ver, Some("flatpak".to_string()), Some(fp_id));
        }
    }

    (false, None, None, None)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppOperationResult {
    pub success: bool,
    pub app_id: String,
    pub provider_id: String,
    pub target_id: String,
    pub version: Option<String>,
    pub message: String,
}

/// Parses an optional `"<provider>:<target_id>"` composite identifier.
/// Examples:
/// - `"flatpak:com.spotify.Client"` -> `(Some("flatpak"), "com.spotify.Client")`
/// - `"aur:cursor-bin"` -> `(Some("aur"), "cursor-bin")`
/// - `"pacman:firefox"` -> `(Some("pacman"), "firefox")`
/// - `"firefox"` -> `(None, "firefox")`
pub fn parse_provider_prefixed_id(input: &str) -> (Option<String>, String) {
    let trimmed = input.trim();
    if let Some((prefix, rest)) = trimmed.split_once(':') {
        let p = prefix.trim().to_lowercase();
        let r = rest.trim();
        if (p == "pacman" || p == "aur" || p == "flatpak") && !r.is_empty() {
            return (Some(p), r.to_string());
        }
    }
    (None, trimmed.to_string())
}

/// Resolves the provider (`"pacman"`, `"aur"`, `"flatpak"`) and provider-specific target ID
/// for a given application and operation (`"install"`, `"uninstall"`, `"launch"`).
pub fn resolve_app_operation_target(
    app_id: &str,
    provider_override: Option<&str>,
    target_override: Option<&str>,
    operation: &str,
) -> Result<(String, String), String> {
    let (prefixed_prov, base_app_id) = parse_provider_prefixed_id(app_id);
    if base_app_id.is_empty() {
        return Err("Invalid application identifier".to_string());
    }

    let explicit_provider = provider_override
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .or(prefixed_prov);

    let explicit_target = target_override
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // For uninstall and launch, if no explicit provider was given, prefer the provider that is ACTUALLY installed!
    if explicit_provider.is_none() && (operation == "uninstall" || operation == "launch") {
        let (is_inst, _, inst_prov, inst_target) = check_app_installed_across_providers(&base_app_id);
        if is_inst {
            if let (Some(p), Some(t)) = (inst_prov, inst_target) {
                return Ok((p, t));
            }
        }
    }

    let provider = if let Some(p) = explicit_provider {
        p
    } else if flatpak::is_valid_flatpak_id(&base_app_id) {
        "flatpak".to_string()
    } else if let Some(aur_target) = known_aur_id_for_app(&base_app_id) {
        // Check if available in official/CachyOS sync repos first
        let sync_dir = pacman::resolve_sync_dir(None);
        let local_dir = pacman::resolve_local_dir(None);
        let in_sync = pacman::get_pacman_package_details_in(aur_target, &sync_dir, &local_dir)
            .ok()
            .flatten()
            .map(|p| {
                let r = p.repository.to_lowercase();
                r != "installed" && r != "local" && r != "aur"
            })
            .unwrap_or(false);
        if in_sync {
            "pacman".to_string()
        } else {
            "aur".to_string()
        }
    } else {
        "pacman".to_string()
    };

    let target = if let Some(t) = explicit_target {
        t
    } else {
        match provider.as_str() {
            "flatpak" => {
                if flatpak::is_valid_flatpak_id(&base_app_id) {
                    base_app_id.clone()
                } else if let Some(known_fp) = known_flatpak_id_for_app(&base_app_id) {
                    known_fp.to_string()
                } else if let Some(found) = flatpak::find_flathub_match(&base_app_id, None) {
                    found.id
                } else {
                    return Err(format!(
                        "No verified Flatpak application ID found for '{}'",
                        base_app_id
                    ));
                }
            }
            "aur" => known_aur_id_for_app(&base_app_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| base_app_id.clone()),
            "pacman" => base_app_id.clone(),
            other => return Err(format!("Unsupported package provider '{}'", other)),
        }
    };

    Ok((provider, target))
}

/// Unified provider-aware application installation.
pub fn install_app_with_provider(
    app_id: &str,
    provider_id: Option<&str>,
    target_id: Option<&str>,
) -> Result<AppOperationResult, String> {
    let (_, clean_app_id) = parse_provider_prefixed_id(app_id);
    let (provider, target) = resolve_app_operation_target(app_id, provider_id, target_id, "install")?;

    match provider.as_str() {
        "pacman" => {
            let val = pacman_install_package_core(&target)?;
            let version = val.get("version").and_then(|v| v.as_str()).map(|s| s.to_string());
            let message = val
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Application installed successfully")
                .to_string();
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: "pacman".to_string(),
                target_id: target,
                version,
                message,
            })
        }
        "aur" => {
            let msg = aur::aur_build_and_install_package(&target)?;
            let local_dir = pacman::resolve_local_dir(None);
            let (is_inst, ver) = pacman::check_installed_status_in(&target, &local_dir);
            if !is_inst {
                return Err(format!(
                    "AUR build finished, but '{}' was not detected in the local package database.",
                    target
                ));
            }
            invalidate_app_provider_cache(&clean_app_id);
            invalidate_app_provider_cache(&target);
            let _ = catalog::refresh_installed_state_internal();
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: "aur".to_string(),
                target_id: target,
                version: ver,
                message: msg,
            })
        }
        "flatpak" => {
            let msg = flatpak::install_flatpak_app(&target)?;
            if !flatpak::get_installed_flatpak_ids().contains(&target.to_lowercase()) {
                return Err(format!(
                    "Flatpak installation finished, but '{}' is not listed as installed.",
                    target
                ));
            }
            invalidate_app_provider_cache(&clean_app_id);
            invalidate_app_provider_cache(&target);
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: "flatpak".to_string(),
                target_id: target,
                version: None,
                message: msg,
            })
        }
        other => Err(format!("Unsupported package provider '{}'", other)),
    }
}

/// Unified provider-aware application uninstallation.
pub fn uninstall_app_with_provider(
    app_id: &str,
    provider_id: Option<&str>,
    target_id: Option<&str>,
) -> Result<AppOperationResult, String> {
    let (_, clean_app_id) = parse_provider_prefixed_id(app_id);
    let (provider, target) = resolve_app_operation_target(app_id, provider_id, target_id, "uninstall")?;

    match provider.as_str() {
        "flatpak" => {
            let msg = flatpak::uninstall_flatpak_app(&target)?;
            invalidate_app_provider_cache(&clean_app_id);
            invalidate_app_provider_cache(&target);
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: "flatpak".to_string(),
                target_id: target,
                version: None,
                message: msg,
            })
        }
        "pacman" | "aur" => {
            let val = pacman_uninstall_package_core(&target)?;
            let message = val
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Application uninstalled successfully")
                .to_string();
            invalidate_app_provider_cache(&clean_app_id);
            invalidate_app_provider_cache(&target);
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: provider,
                target_id: target,
                version: None,
                message,
            })
        }
        other => Err(format!("Unsupported package provider '{}'", other)),
    }
}

/// Unified provider-aware application launcher.
pub fn launch_app_with_provider(
    app_id: &str,
    provider_id: Option<&str>,
    target_id: Option<&str>,
) -> Result<AppOperationResult, String> {
    let (_, clean_app_id) = parse_provider_prefixed_id(app_id);
    let (provider, target) = resolve_app_operation_target(app_id, provider_id, target_id, "launch")?;

    match provider.as_str() {
        "flatpak" => {
            flatpak::run_flatpak_app(&target)?;
            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: "flatpak".to_string(),
                target_id: target.clone(),
                version: None,
                message: format!("Launched Flatpak application '{}'", target),
            })
        }
        "pacman" | "aur" => {
            // Confirm installed state before attempting launch
            let (is_inst, _, _, _) = check_app_installed_across_providers(&target);
            let (base_inst, _, _, _) = check_app_installed_across_providers(&clean_app_id);
            let has_desktop = desktop_icons::resolve_desktop_icon_for_app(&target)
                .map(|d| !d.desktop_file.is_empty() && Path::new(&d.desktop_file).is_file())
                .unwrap_or(false)
                || desktop_icons::resolve_desktop_icon_for_app(&clean_app_id)
                    .map(|d| !d.desktop_file.is_empty() && Path::new(&d.desktop_file).is_file())
                    .unwrap_or(false);

            if !is_inst && !base_inst && !has_desktop {
                return Err(format!(
                    "Cannot launch '{}': application is not installed.",
                    clean_app_id
                ));
            }

            if launch_desktop_app_core(&target).is_err() {
                launch_desktop_app_core(&clean_app_id)?;
            }

            Ok(AppOperationResult {
                success: true,
                app_id: clean_app_id,
                provider_id: provider,
                target_id: target.clone(),
                version: None,
                message: format!("Launched application '{}'", target),
            })
        }
        other => Err(format!("Unsupported package provider '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_prefixed_id() {
        assert_eq!(
            parse_provider_prefixed_id("flatpak:com.spotify.Client"),
            (Some("flatpak".to_string()), "com.spotify.Client".to_string())
        );
        assert_eq!(
            parse_provider_prefixed_id("aur:cursor-bin"),
            (Some("aur".to_string()), "cursor-bin".to_string())
        );
        assert_eq!(
            parse_provider_prefixed_id("pacman:firefox"),
            (Some("pacman".to_string()), "firefox".to_string())
        );
        assert_eq!(
            parse_provider_prefixed_id("firefox"),
            (None, "firefox".to_string())
        );
    }

    #[test]
    fn test_build_pacman_privileged_args_validation() {
        // With helper installed
        let (prog, args) = build_pacman_privileged_args("install", "firefox", true).unwrap();
        assert_eq!(prog, "pkexec");
        assert_eq!(
            args,
            vec!["/usr/lib/ryzora/ryzora-package-helper", "install", "firefox"]
        );

        // Without helper (direct pacman fallback with -- separator)
        let (prog2, args2) = build_pacman_privileged_args("uninstall", "firefox", false).unwrap();
        assert_eq!(prog2, "pkexec");
        assert_eq!(
            args2,
            vec!["/usr/bin/pacman", "-R", "--noconfirm", "--", "firefox"]
        );

        // Reject flag injection or shell metacharacters
        assert!(build_pacman_privileged_args("install", "-Syu", true).is_err());
        assert!(build_pacman_privileged_args("install", "firefox; rm -rf /", true).is_err());
        assert!(build_pacman_privileged_args("install", "../etc/passwd", true).is_err());
    }

    #[test]
    fn test_aur_helper_install_args_validation() {
        let paru_args = aur::aur_helper_install_args("paru", "cursor-bin").unwrap();
        assert_eq!(
            paru_args,
            vec!["-S", "--noconfirm", "--sudo", "pkexec", "--", "cursor-bin"]
        );

        let yay_args = aur::aur_helper_install_args("yay", "visual-studio-code-bin").unwrap();
        assert_eq!(
            yay_args,
            vec![
                "-S",
                "--noconfirm",
                "--sudo",
                "pkexec",
                "--",
                "visual-studio-code-bin"
            ]
        );

        assert!(aur::aur_helper_install_args("paru", "-- malicious").is_err());
        assert!(aur::aur_helper_install_args("paru", "pkg/../../root").is_err());
    }
}
