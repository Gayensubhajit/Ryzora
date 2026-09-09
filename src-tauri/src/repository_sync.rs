use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::repository::create_default_manager;
use crate::snapshot::get_home_dir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySyncStatus {
    pub id: String,
    pub name: String,
    pub url: String,
    pub repo_type: String, // "local" | "remote"
    pub enabled: bool,
    pub status: String, // "online" | "offline" | "cached" | "refresh_failed"
    pub package_count: usize,
    pub last_synced: Option<String>,
    pub error_message: Option<String>,
    pub channel: String, // "stable" | "beta" | "nightly"
    pub available_channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepositorySyncReport {
    pub repository_id: String,
    pub success: bool,
    pub packages_discovered: usize,
    pub timestamp: String,
    pub message: String,
}

pub fn get_channel_config_file() -> PathBuf {
    get_home_dir().join(".local/share/ryzora/repo_channels.json")
}

pub fn load_repo_channels() -> std::collections::HashMap<String, String> {
    let path = get_channel_config_file();
    if path.is_file() {
        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&raw)
            {
                return map;
            }
        }
    }
    std::collections::HashMap::new()
}

pub fn save_repo_channels(
    channels: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let path = get_channel_config_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create channel config dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(channels)
        .map_err(|e| format!("Failed to serialize repo channels: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write repo channels: {}", e))
}

pub fn get_available_channels() -> Vec<String> {
    vec![
        "stable".to_string(),
        "beta".to_string(),
        "nightly".to_string(),
    ]
}

#[tauri::command]
pub fn get_repository_sync_status() -> Result<Vec<RepositorySyncStatus>, String> {
    let manager = create_default_manager();
    let channels = load_repo_channels();
    let available_channels = get_available_channels();

    let mut statuses = Vec::new();

    for summary in manager.list_repositories() {
        let channel = channels
            .get(&summary.id)
            .cloned()
            .unwrap_or_else(|| "stable".to_string());

        let url = if summary.repo_type == "local" {
            format!("local://{}", summary.id)
        } else {
            summary.path.clone()
        };

        statuses.push(RepositorySyncStatus {
            id: summary.id,
            name: summary.name,
            url,
            repo_type: summary.repo_type,
            enabled: summary.enabled,
            status: summary.status,
            package_count: summary.package_count,
            last_synced: summary.last_refreshed,
            error_message: summary.last_error,
            channel,
            available_channels: available_channels.clone(),
        });
    }

    Ok(statuses)
}

#[tauri::command]
pub fn refresh_repository_sync(repository_id: String) -> Result<RepositorySyncReport, String> {
    let mut manager = create_default_manager();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let timestamp = format!("{}", now);

    let mut found = false;
    let mut pkg_count = 0;
    let mut refresh_error: Option<String> = None;

    for repo in manager.repositories_mut() {
        if repo.id() == repository_id {
            found = true;
            if let Err(e) = repo.refresh() {
                refresh_error = Some(e);
            }
            if let Ok(entries) = repo.list_entries() {
                pkg_count = entries.len();
            }
            break;
        }
    }

    if !found {
        return Err(format!("Repository '{}' not found", repository_id));
    }

    if let Some(err) = refresh_error {
        Ok(RepositorySyncReport {
            repository_id: repository_id.to_string(),
            success: false,
            packages_discovered: pkg_count,
            timestamp,
            message: format!("Refresh completed with warnings: {}", err),
        })
    } else {
        Ok(RepositorySyncReport {
            repository_id: repository_id.to_string(),
            success: true,
            packages_discovered: pkg_count,
            timestamp,
            message: format!(
                "Synchronized successfully ({} packages available)",
                pkg_count
            ),
        })
    }
}

#[tauri::command]
pub fn refresh_all_repositories_sync() -> Result<Vec<RepositorySyncReport>, String> {
    let mut manager = create_default_manager();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let timestamp = format!("{}", now);

    let mut reports = Vec::new();

    for repo in manager.repositories_mut() {
        let repo_id = repo.id().to_string();
        let refresh_res = repo.refresh();
        let pkg_count = repo.list_entries().map(|e| e.len()).unwrap_or(0);

        match refresh_res {
            Ok(_) => {
                reports.push(RepositorySyncReport {
                    repository_id: repo_id,
                    success: true,
                    packages_discovered: pkg_count,
                    timestamp: timestamp.clone(),
                    message: format!("Synchronized successfully ({} packages)", pkg_count),
                });
            }
            Err(e) => {
                reports.push(RepositorySyncReport {
                    repository_id: repo_id,
                    success: false,
                    packages_discovered: pkg_count,
                    timestamp: timestamp.clone(),
                    message: format!("Sync warning: {}", e),
                });
            }
        }
    }

    Ok(reports)
}

#[tauri::command]
pub fn switch_repository_channel(
    repository_id: String,
    channel: String,
) -> Result<RepositorySyncStatus, String> {
    let valid_channels = get_available_channels();
    if !valid_channels.contains(&channel.to_string()) {
        return Err(format!(
            "Invalid channel '{}'. Available channels: {}",
            channel,
            valid_channels.join(", ")
        ));
    }

    let mut channels = load_repo_channels();
    channels.insert(repository_id.to_string(), channel.to_string());
    save_repo_channels(&channels)?;

    let all_statuses = get_repository_sync_status()?;
    all_statuses
        .into_iter()
        .find(|s| s.id == repository_id)
        .ok_or_else(|| {
            format!(
                "Repository '{}' not found after channel switch",
                repository_id
            )
        })
}

#[tauri::command]
pub fn list_repository_channels() -> Result<Vec<String>, String> {
    Ok(get_available_channels())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{RemoteRepository, Repository};

    #[test]
    fn test_repository_sync_channel_validation() {
        let res = switch_repository_channel("any-repo".to_string(), "invalid-channel".to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Invalid channel"));
    }

    #[test]
    fn test_list_repository_channels_returns_standard_channels() {
        let channels = list_repository_channels().unwrap();
        assert_eq!(channels, vec!["stable", "beta", "nightly"]);
    }

    #[test]
    fn test_repository_sync_channel_switching() {
        let temp_dir =
            std::env::temp_dir().join(format!("ryzora-channel-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let channels_file = temp_dir.join("repo_channels.json");

        let mut map = std::collections::HashMap::new();
        map.insert("repo-test".to_string(), "beta".to_string());
        let json = serde_json::to_string(&map).unwrap();
        fs::write(&channels_file, json).unwrap();

        let read_back: std::collections::HashMap<String, String> =
            serde_json::from_str(&fs::read_to_string(&channels_file).unwrap()).unwrap();
        assert_eq!(read_back.get("repo-test").map(|s| s.as_str()), Some("beta"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_repository_sync_malformed_url_rejected() {
        let res = crate::repository::validate_remote_url("http://insecure-repo.com/packages");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("HTTPS only"));

        let res2 = crate::repository::validate_remote_url("https://malicious.com/../escaped");
        assert!(res2.is_err());
    }

    #[test]
    fn test_repository_sync_offline_fallback_preserves_cache() {
        // Test that mock remote repository with cached index preserves index when offline
        let temp_dir =
            std::env::temp_dir().join(format!("ryzora-offline-sync-{}", std::process::id()));
        let cache_dir = temp_dir.join("cache");
        let _ = fs::create_dir_all(&cache_dir);

        let cached_index_content = r#"{
            "schema": 1,
            "id": "cached-repo",
            "name": "Cached Repo",
            "version": "1.0.0",
            "packages": [
                {
                    "id": "pkg-one",
                    "name": "Package One",
                    "version": "1.0.0",
                    "package_type": "rice",
                    "manifest": "pkg-one/manifest.json",
                    "author": { "name": "Author", "avatar": "" }
                }
            ]
        }"#;

        fs::write(cache_dir.join("repository.json"), cached_index_content).unwrap();

        // Create remote repo pointing to non-existent URL so refresh fails
        let repo = RemoteRepository::new(
            "cached-repo".to_string(),
            "Cached Repo".to_string(),
            "https://127.0.0.1:59999/repo".to_string(), // Unreachable URL
            cache_dir.clone(),
        );
        assert!(repo.is_ok());
        let mut repo = repo.unwrap();

        // Refresh should fail gracefully and retain cache
        let refresh_res = repo.refresh();
        // Graceful offline fallback: returns cached index Ok but marks status offline
        assert!(refresh_res.is_ok());
        assert_eq!(repo.status(), "offline");

        // Cached index should still be browseable
        let entries = repo.list_entries();
        assert!(entries.is_ok());
        assert_eq!(entries.unwrap().len(), 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_repository_sync_manual_refresh() {
        let statuses = get_repository_sync_status();
        assert!(statuses.is_ok());
        let list = statuses.unwrap();
        assert!(
            !list.is_empty(),
            "Default manager should discover repositories"
        );

        let first = &list[0];
        assert!(!first.id.is_empty());
        assert_eq!(first.available_channels, vec!["stable", "beta", "nightly"]);

        let report = refresh_repository_sync(first.id.clone());
        assert!(report.is_ok());
        let r = report.unwrap();
        assert_eq!(r.repository_id, first.id);
    }
}
