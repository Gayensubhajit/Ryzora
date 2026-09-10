//! Ryzora Application Adapters — Phase 23
//!
//! Orchestrates native and universal package managers across Linux distributions.
//! Phase 23A implements the Pacman adapter for Arch Linux.

pub mod pacman;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
pub fn detect_host_app_ecosystem_in(home: &Path) -> HostAppEcosystem {
    let host_caps = crate::host::detect_host_capabilities_in(home);

    let distro_id = host_caps.distro_id.to_lowercase();
    let distro_family = if distro_id == "arch"
        || distro_id == "garuda"
        || distro_id == "endeavouros"
        || distro_id == "manjaro"
    {
        "arch".to_string()
    } else if distro_id == "debian"
        || distro_id == "ubuntu"
        || distro_id == "pop"
        || distro_id == "linuxmint"
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

    let (has_pacman, _) = crate::system::check_binary("pacman");
    if has_pacman {
        available.push("pacman".to_string());
    }

    let (has_paru, _) = crate::system::check_binary("paru");
    let (has_yay, _) = crate::system::check_binary("yay");
    let aur_helper = if has_paru {
        available.push("paru".to_string());
        Some("paru".to_string())
    } else if has_yay {
        available.push("yay".to_string());
        Some("yay".to_string())
    } else {
        None
    };

    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if has_flatpak {
        available.push("flatpak".to_string());
    }

    let (has_apt, _) = crate::system::check_binary("apt");
    if has_apt {
        available.push("apt".to_string());
    }

    let (has_dnf, _) = crate::system::check_binary("dnf");
    if has_dnf {
        available.push("dnf".to_string());
    }

    let (has_zypper, _) = crate::system::check_binary("zypper");
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
        distro_id: host_caps.distro_id,
        distro_name: host_caps.distro_name,
        distro_family,
        available_managers: available,
        primary_manager: primary,
        has_pacman,
        has_aur_helper: aur_helper.is_some(),
        aur_helper,
        has_flatpak,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_host_app_ecosystem() -> Result<HostAppEcosystem, String> {
    let home = crate::snapshot::get_home_dir();
    Ok(detect_host_app_ecosystem_in(&home))
}

#[tauri::command]
pub fn pacman_search_packages(query: String, limit: Option<usize>) -> Result<Vec<pacman::PacmanPackageInfo>, String> {
    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);
    pacman::search_pacman_packages_in(&query, &sync_dir, &local_dir, limit.unwrap_or(50))
}

#[tauri::command]
pub fn pacman_get_package_details(package_name: String) -> Result<Option<pacman::PacmanPackageInfo>, String> {
    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);
    pacman::get_pacman_package_details_in(&package_name, &sync_dir, &local_dir)
}

#[tauri::command]
pub fn pacman_list_installed_packages() -> Result<Vec<pacman::PacmanPackageInfo>, String> {
    let local_dir = pacman::resolve_local_dir(None);
    pacman::list_installed_pacman_packages_in(&local_dir)
}

#[tauri::command]
pub fn pacman_install_package(package_name: String) -> Result<serde_json::Value, String> {
    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);

    let pkg = pacman::get_pacman_package_details_in(&package_name, &sync_dir, &local_dir)?
        .ok_or_else(|| format!("Package '{}' not found in pacman repositories", package_name))?;

    if pkg.is_installed {
        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Package '{}' is already installed", package_name),
            "package_name": package_name,
            "version": pkg.version
        }));
    }

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate install into sandbox local db
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        let pkg_dir = local_dir.join(format!("{}-{}", pkg.name, pkg.version));
        fs::create_dir_all(&pkg_dir)
            .map_err(|e| format!("Failed to create mock package dir: {}", e))?;
        let desc_content = format!(
            "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\n{}\n\n%URL%\n{}\n\n%LICENSE%\n{}\n",
            pkg.name,
            pkg.version,
            pkg.description,
            pkg.url.as_deref().unwrap_or(""),
            pkg.license.as_deref().unwrap_or("")
        );
        fs::write(pkg_dir.join("desc"), desc_content)
            .map_err(|e| format!("Failed to write mock desc file: {}", e))?;

        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Installed '{}' successfully (test mode)", package_name),
            "package_name": package_name,
            "version": pkg.version
        }));
    }

    // Live mode: Use sddm_helper/polkit invocation or controlled terminal
    Err("Direct package installation requires Polkit privileged helper setup.".to_string())
}

#[tauri::command]
pub fn pacman_uninstall_package(package_name: String) -> Result<serde_json::Value, String> {
    let local_dir = pacman::resolve_local_dir(None);
    let (is_installed, ver) = pacman::check_installed_status_in(&package_name, &local_dir);

    if !is_installed {
        return Err(format!("Package '{}' is not installed", package_name));
    }

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate uninstall by removing mock package dir
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        if let Some(v) = ver {
            let pkg_dir = local_dir.join(format!("{}-{}", package_name, v));
            if pkg_dir.exists() {
                let _ = fs::remove_dir_all(&pkg_dir);
            }
        }
        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Uninstalled '{}' successfully (test mode)", package_name),
            "package_name": package_name
        }));
    }

    Err("Direct package uninstallation requires Polkit privileged helper setup.".to_string())
}

pub mod desktop_icons;

#[tauri::command]
pub fn resolve_desktop_app_icon(package_id: String) -> Option<desktop_icons::DesktopAppIconInfo> {
    desktop_icons::resolve_desktop_icon_for_app(&package_id)
}
