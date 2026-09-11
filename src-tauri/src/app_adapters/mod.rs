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


#[tauri::command]
pub fn get_installed_package_files(package_name: String) -> Result<Vec<String>, String> {
    let local_dir = pacman::resolve_local_dir(None);
    if !local_dir.exists() {
        return Ok(Vec::new());
    }

    if let Ok(entries) = fs::read_dir(&local_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str == package_name || name_str.starts_with(&format!("{}-", package_name)) {
                let files_path = entry.path().join("files");
                if files_path.exists() {
                    if let Ok(content) = fs::read_to_string(&files_path) {
                        let mut in_files = false;
                        let mut files = Vec::new();
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed == "%FILES%" {
                                in_files = true;
                                continue;
                            }
                            if in_files {
                                if trimmed.starts_with('%') {
                                    break;
                                }
                                if !trimmed.is_empty() {
                                    files.push(format!("/{}", trimmed));
                                }
                            }
                        }
                        return Ok(files);
                    }
                }
            }
        }
    }

    Ok(Vec::new())
}

#[tauri::command]
pub fn launch_desktop_app(package_id: String) -> Result<bool, String> {
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok(true);
    }

    if let Some(info) = desktop_icons::resolve_desktop_icon_for_app(&package_id) {
        let desktop_file_name = std::path::Path::new(&info.desktop_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&info.desktop_file);

        // 1. Try launching with gtk-launch using desktop file name
        if let Ok(_child) = std::process::Command::new("gtk-launch")
            .arg(desktop_file_name)
            .spawn()
        {
            return Ok(true);
        }

        // 2. Parse exec command from desktop entry, stripping Freedesktop field codes (%u, %F, etc.)
        if let Some(exec_cmd) = &info.exec {
            let tokens: Vec<&str> = exec_cmd
                .split_whitespace()
                .filter(|tok| !tok.starts_with('%'))
                .collect();

            if let Some((binary, args)) = tokens.split_first() {
                if !binary.is_empty() {
                    if let Ok(_child) = std::process::Command::new(binary).args(args).spawn() {
                        return Ok(true);
                    }
                }
            }
        }
    }

    // 3. Fallback: try spawning binary by package_id directly
    if let Ok(_child) = std::process::Command::new(&package_id).spawn() {
        return Ok(true);
    }

    Err(format!(
        "Could not launch application '{}'. Desktop entry or executable was not found on system.",
        package_id
    ))
}
