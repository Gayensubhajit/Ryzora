//! Ryzora Flatpak & Flathub Adapter — Phase 25
//!
//! Provides native Flatpak application management:
//! - Detection of Flatpak binary and configured remotes (Flathub).
//! - Authoritative listing of installed Flatpak applications via Flatpak CLI.
//! - Search across Flathub remotes.
//! - App execution, installation, and uninstallation using Flatpak's native sandboxing.
//! - Cleanup awareness: exposes user data and cache directories for Phase 26.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakAppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub description: String,
    pub is_installed: bool,
    pub runtime: Option<String>,
    pub sdk: Option<String>,
    pub installed_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakStatus {
    pub has_flatpak: bool,
    pub version: Option<String>,
    pub has_flathub: bool,
    pub remotes: Vec<String>,
    pub total_installed_apps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakCleanupInfo {
    pub app_id: String,
    pub user_data_path: Option<String>,
    pub cache_path: Option<String>,
    pub system_app_path: Option<String>,
}

/// Detects the host Flatpak installation and remote configurations.
pub fn detect_flatpak_status() -> FlatpakStatus {
    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if !has_flatpak {
        return FlatpakStatus {
            has_flatpak: false,
            version: None,
            has_flathub: false,
            remotes: Vec::new(),
            total_installed_apps: 0,
        };
    }

    let version = Command::new("flatpak")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let mut remotes = Vec::new();
    let mut has_flathub = false;

    if let Ok(output) = Command::new("flatpak").arg("remotes").output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(name) = parts.first() {
                    let r = name.to_string();
                    if r.to_lowercase() == "flathub" {
                        has_flathub = true;
                    }
                    remotes.push(r);
                }
            }
        }
    }

    let installed_apps = list_installed_flatpak_apps().unwrap_or_default();

    FlatpakStatus {
        has_flatpak: true,
        version,
        has_flathub,
        remotes,
        total_installed_apps: installed_apps.len(),
    }
}

/// Lists all installed Flatpak applications with metadata.
pub fn list_installed_flatpak_apps() -> Result<Vec<FlatpakAppInfo>, String> {
    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if !has_flatpak {
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application,name,version,branch,origin,description"])
        .output()
        .map_err(|e| format!("Failed to run flatpak list: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("flatpak list exited with error: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 2 {
            let id = cols[0].trim().to_string();
            let name = cols[1].trim().to_string();
            let version = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let branch = cols.get(3).map(|s| s.trim().to_string()).unwrap_or_else(|| "stable".to_string());
            let origin = cols.get(4).map(|s| s.trim().to_string()).unwrap_or_else(|| "flathub".to_string());
            let description = cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default();

            apps.push(FlatpakAppInfo {
                id,
                name,
                version,
                branch,
                origin,
                description,
                is_installed: true,
                runtime: None,
                sdk: None,
                installed_size: None,
            });
        }
    }

    Ok(apps)
}

/// Searches Flathub applications matching the query.
pub fn search_flathub_apps(query: &str) -> Result<Vec<FlatpakAppInfo>, String> {
    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if !has_flatpak {
        return Ok(Vec::new());
    }

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(["search", "--columns=application,name,version,branch,remotes,description", trimmed])
        .output()
        .map_err(|e| format!("Failed to run flatpak search: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let installed_list = list_installed_flatpak_apps().unwrap_or_default();
    let installed_ids: std::collections::HashSet<String> = installed_list.into_iter().map(|a| a.id.to_lowercase()).collect();

    let mut results = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 2 {
            let id = cols[0].trim().to_string();
            // Skip sub-plugins or runtimes if searching applications
            if id.contains(".Plugin.") {
                continue;
            }
            let name = cols[1].trim().to_string();
            let version = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let branch = cols.get(3).map(|s| s.trim().to_string()).unwrap_or_else(|| "stable".to_string());
            let origin = cols.get(4).map(|s| s.trim().to_string()).unwrap_or_else(|| "flathub".to_string());
            let description = cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default();
            let is_installed = installed_ids.contains(&id.to_lowercase());

            results.push(FlatpakAppInfo {
                id,
                name,
                version,
                branch,
                origin,
                description,
                is_installed,
                runtime: None,
                sdk: None,
                installed_size: None,
            });
        }
    }

    Ok(results)
}

/// Retrieves detailed information about a specific Flatpak app.
pub fn get_flatpak_app_info(app_id: &str) -> Result<Option<FlatpakAppInfo>, String> {
    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if !has_flatpak {
        return Ok(None);
    }

    // First check installed apps
    let installed = list_installed_flatpak_apps()?;
    if let Some(app) = installed.into_iter().find(|a| a.id.eq_ignore_ascii_case(app_id)) {
        return Ok(Some(app));
    }

    // Otherwise search Flathub
    let search_res = search_flathub_apps(app_id)?;
    Ok(search_res.into_iter().find(|a| a.id.eq_ignore_ascii_case(app_id)))
}

/// Launches a Flatpak application unprivileged.
pub fn run_flatpak_app(app_id: &str) -> Result<(), String> {
    if !app_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
        return Err("Invalid flatpak application ID".to_string());
    }

    Command::new("flatpak")
        .arg("run")
        .arg(app_id)
        .spawn()
        .map_err(|e| format!("Failed to launch flatpak application '{}': {}", app_id, e))?;

    Ok(())
}

/// Installs a Flatpak application from Flathub.
pub fn install_flatpak_app(app_id: &str) -> Result<String, String> {
    if !app_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
        return Err("Invalid flatpak application ID".to_string());
    }

    let output = Command::new("flatpak")
        .args(["install", "-y", "flathub", app_id])
        .output()
        .map_err(|e| format!("Failed to invoke flatpak install: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("flatpak install failed: {}", err.trim()));
    }

    Ok(format!("Successfully installed Flatpak application '{}'", app_id))
}

/// Uninstalls a Flatpak application.
pub fn uninstall_flatpak_app(app_id: &str) -> Result<String, String> {
    if !app_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_') {
        return Err("Invalid flatpak application ID".to_string());
    }

    let output = Command::new("flatpak")
        .args(["uninstall", "-y", app_id])
        .output()
        .map_err(|e| format!("Failed to invoke flatpak uninstall: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("flatpak uninstall failed: {}", err.trim()));
    }

    Ok(format!("Successfully uninstalled Flatpak application '{}'", app_id))
}

/// Exposes cleanup metadata for Phase 26 storage management.
pub fn get_flatpak_cleanup_info(app_id: &str) -> FlatpakCleanupInfo {
    let home = crate::snapshot::get_home_dir();
    let user_data = home.join(".var/app").join(app_id);
    let user_data_path = if user_data.exists() {
        Some(user_data.display().to_string())
    } else {
        None
    };

    let cache_dir = user_data.join("cache");
    let cache_path = if cache_dir.exists() {
        Some(cache_dir.display().to_string())
    } else {
        None
    };

    let sys_app = PathBuf::from("/var/lib/flatpak/app").join(app_id);
    let system_app_path = if sys_app.exists() {
        Some(sys_app.display().to_string())
    } else {
        None
    };

    FlatpakCleanupInfo {
        app_id: app_id.to_string(),
        user_data_path,
        cache_path,
        system_app_path,
    }
}
