//! Ryzora Updates Engine — Multi-Provider Update Discovery & Management
//!
//! Discovers and coordinates updates across Flatpak (apps & runtimes), official Arch
//! repositories (Pacman), AUR packages, and Ryzora catalog presets/themes.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateInfo {
    pub package_id: String,
    pub name: String,
    pub installed_version: String,
    pub available_version: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub package_type: String, // "app", "runtime", "system", "theme"
    #[serde(default)]
    pub download_size: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub can_update_individually: bool,
    #[serde(default)]
    pub safety_warning: Option<String>,
}

static UPDATES_CACHE: Mutex<Option<(Instant, Vec<UpdateInfo>)>> = Mutex::new(None);
static TEST_MOCK_UPDATES: Mutex<Option<Vec<UpdateInfo>>> = Mutex::new(None);

pub fn set_test_mock_updates(updates: Option<Vec<UpdateInfo>>) {
    if let Ok(mut lock) = TEST_MOCK_UPDATES.lock() {
        *lock = updates;
    }
}

pub fn has_test_mock_updates() -> bool {
    TEST_MOCK_UPDATES.lock().map(|m| m.is_some()).unwrap_or(false)
}

pub fn invalidate_updates_cache() {
    if let Ok(mut lock) = UPDATES_CACHE.lock() {
        *lock = None;
    }
}

/// Discovers Flatpak updates across both `--user` and `--system` scopes.
pub fn discover_flatpak_updates() -> Vec<UpdateInfo> {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Vec::new();
    }

    if std::env::var("RYZORA_TEST_NO_FLATPAK").as_deref() == Ok("1") {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut seen_ids = HashSet::new();

    for scope in ["--user", "--system"] {
        let output = Command::new("flatpak")
            .args(["remote-ls", "--updates", "--columns=application,name,version,branch,arch,origin,ref", scope])
            .output();

        let output = match output {
            Ok(o) if o.status.success() => o,
            _ => continue,
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('\t').map(|s| s.trim()).collect();
            let (app_id, name, avail_ver, origin, ref_str) = if parts.len() >= 7 {
                (parts[0], parts[1], parts[2], parts[5], parts[6])
            } else {
                let ws: Vec<&str> = line.split_whitespace().collect();
                if ws.len() >= 4 {
                    (ws[0], ws[0], ws[1], "flathub", ws[ws.len() - 1])
                } else {
                    continue;
                }
            };

            if app_id.is_empty() || seen_ids.contains(app_id) {
                continue;
            }
            seen_ids.insert(app_id.to_string());

            // Query installed version
            let installed_ver = Command::new("flatpak")
                .args(["info", scope, app_id])
                .output()
                .ok()
                .and_then(|o| {
                    if o.status.success() {
                        let text = String::from_utf8_lossy(&o.stdout);
                        for l in text.lines() {
                            let lt = l.trim();
                            if lt.starts_with("Version:") {
                                return Some(lt.trim_start_matches("Version:").trim().to_string());
                            }
                        }
                    }
                    None
                })
                .unwrap_or_else(|| "installed".to_string());

            let is_runtime = ref_str.starts_with("runtime/") || app_id.contains(".Platform") || app_id.contains(".Sdk") || app_id.contains(".GL.");
            let package_type = if is_runtime { "runtime" } else { "app" };
            let scope_name = if scope == "--user" { "user" } else { "system" };

            let icon_url = if !is_runtime {
                crate::app_adapters::desktop_icons::resolve_desktop_icon_for_app(app_id)
                    .and_then(|info| info.icon_data_uri)
            } else {
                None
            };

            results.push(UpdateInfo {
                package_id: format!("flatpak:{}", app_id),
                name: if !name.is_empty() { name.to_string() } else { app_id.to_string() },
                installed_version: installed_ver,
                available_version: if !avail_ver.is_empty() { avail_ver.to_string() } else { "latest".to_string() },
                category: if is_runtime { "Runtime".to_string() } else { "Application".to_string() },
                description: format!("Flatpak {} update from {}", package_type, origin),
                provider: "flatpak".to_string(),
                source: if !origin.is_empty() { origin.to_string() } else { "flathub".to_string() },
                scope: scope_name.to_string(),
                package_type: package_type.to_string(),
                download_size: None,
                icon_url,
                can_update_individually: true,
                safety_warning: None,
            });
        }
    }

    results
}

/// Discovers official Arch Linux updates via `checkupdates`.
pub fn discover_pacman_updates() -> Vec<UpdateInfo> {
    let (has_pacman, _) = crate::app_adapters::check_binary("pacman");
    if !has_pacman {
        return Vec::new();
    }

    let (has_checkupdates, _) = crate::app_adapters::check_binary("checkupdates");
    if !has_checkupdates {
        return Vec::new();
    }

    let output = Command::new("checkupdates")
        .arg("--nocolor")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() && output.status.code() != Some(0) {
        // Code 2 from checkupdates means no updates pending
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Format: <pkgname> <installed_ver> -> <available_ver>
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 || parts[2] != "->" {
            continue;
        }

        let pkgname = parts[0];
        let installed_ver = parts[1];
        let available_ver = parts[3];

        let desktop_info = crate::app_adapters::desktop_icons::resolve_desktop_icon_for_app(pkgname);
        let is_app = desktop_info.is_some() || PathBuf::from(format!("/usr/share/applications/{}.desktop", pkgname)).exists();

        let package_type = if is_app { "app" } else { "system" };
        let icon_url = desktop_info.and_then(|i| i.icon_data_uri);

        results.push(UpdateInfo {
            package_id: format!("pacman:{}", pkgname),
            name: pkgname.to_string(),
            installed_version: installed_ver.to_string(),
            available_version: available_ver.to_string(),
            category: if is_app { "Application".to_string() } else { "System".to_string() },
            description: format!("Arch Linux {} package update", package_type),
            provider: "pacman".to_string(),
            source: "arch-linux".to_string(),
            scope: "system".to_string(),
            package_type: package_type.to_string(),
            download_size: None,
            icon_url,
            can_update_individually: false,
            safety_warning: Some("Updating individual Arch repository packages without a full system upgrade can cause dependency issues.".to_string()),
        });
    }

    results
}

/// Discovers AUR updates via the detected helper (`paru` or `yay`).
pub fn discover_aur_updates() -> Vec<UpdateInfo> {
    let helper = if crate::app_adapters::check_binary("paru").0 {
        Some("paru")
    } else if crate::app_adapters::check_binary("yay").0 {
        Some("yay")
    } else {
        None
    };

    let helper = match helper {
        Some(h) => h,
        None => return Vec::new(),
    };

    let output = Command::new(helper)
        .arg("-Qua")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 || parts[2] != "->" {
            continue;
        }

        let pkgname = parts[0];
        let installed_ver = parts[1];
        let available_ver = parts[3];

        let desktop_info = crate::app_adapters::desktop_icons::resolve_desktop_icon_for_app(pkgname);
        let is_app = desktop_info.is_some() || PathBuf::from(format!("/usr/share/applications/{}.desktop", pkgname)).exists();

        let package_type = if is_app { "app" } else { "system" };
        let icon_url = desktop_info.and_then(|i| i.icon_data_uri);

        results.push(UpdateInfo {
            package_id: format!("aur:{}", pkgname),
            name: pkgname.to_string(),
            installed_version: installed_ver.to_string(),
            available_version: available_ver.to_string(),
            category: if is_app { "Application".to_string() } else { "AUR Package".to_string() },
            description: "Arch User Repository community package update".to_string(),
            provider: "aur".to_string(),
            source: "aur".to_string(),
            scope: "system".to_string(),
            package_type: package_type.to_string(),
            download_size: None,
            icon_url,
            can_update_individually: true,
            safety_warning: None,
        });
    }

    results
}

/// Discovers all available package updates across providers.
/// Uses a 30s in-memory cache to avoid redundant external process spawns.
pub fn discover_all_updates(force_refresh: bool) -> Vec<UpdateInfo> {
    if let Ok(test_mock) = TEST_MOCK_UPDATES.lock() {
        if let Some(ref mock) = *test_mock {
            return mock.clone();
        }
    }

    if !force_refresh {
        if let Ok(cache) = UPDATES_CACHE.lock() {
            if let Some((instant, ref list)) = *cache {
                if instant.elapsed() < Duration::from_secs(30) {
                    return list.clone();
                }
            }
        }
    }

    let mut all = Vec::new();
    all.extend(discover_flatpak_updates());
    all.extend(discover_pacman_updates());
    all.extend(discover_aur_updates());

    if let Ok(mut cache) = UPDATES_CACHE.lock() {
        *cache = Some((Instant::now(), all.clone()));
    }

    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_update_discovery() {
        let mock = vec![
            UpdateInfo {
                package_id: "flatpak:org.mozilla.Thunderbird".to_string(),
                name: "Thunderbird".to_string(),
                installed_version: "128.0".to_string(),
                available_version: "128.1".to_string(),
                category: "Application".to_string(),
                description: "Email client".to_string(),
                provider: "flatpak".to_string(),
                source: "flathub".to_string(),
                scope: "user".to_string(),
                package_type: "app".to_string(),
                download_size: Some("85 MB".to_string()),
                icon_url: None,
                can_update_individually: true,
                safety_warning: None,
            },
            UpdateInfo {
                package_id: "pacman:glibc".to_string(),
                name: "glibc".to_string(),
                installed_version: "2.39".to_string(),
                available_version: "2.40".to_string(),
                category: "System".to_string(),
                description: "Core C library".to_string(),
                provider: "pacman".to_string(),
                source: "core".to_string(),
                scope: "system".to_string(),
                package_type: "system".to_string(),
                download_size: None,
                icon_url: None,
                can_update_individually: false,
                safety_warning: Some("Updating individual Arch repository packages without a full system upgrade can cause dependency issues.".to_string()),
            },
        ];

        set_test_mock_updates(Some(mock));
        let discovered = discover_all_updates(true);
        assert_eq!(discovered.len(), 2);
        assert_eq!(discovered[0].package_id, "flatpak:org.mozilla.Thunderbird");
        assert!(discovered[0].can_update_individually);
        assert!(!discovered[1].can_update_individually);
        assert!(discovered[1].safety_warning.is_some());

        set_test_mock_updates(None);
    }

    #[test]
    fn test_cache_invalidation() {
        {
            let mut cache = UPDATES_CACHE.lock().unwrap();
            *cache = Some((Instant::now(), vec![]));
        }
        assert!(UPDATES_CACHE.lock().unwrap().is_some());
        invalidate_updates_cache();
        assert!(UPDATES_CACHE.lock().unwrap().is_none());
    }
}
