use serde::{Deserialize, Serialize};

use crate::creator::{list_creator_profiles, CreatorProfile};
pub use crate::installer::InstalledHistoryEntry;
use crate::installer::{get_installed_package, list_installed_packages, InstalledPackageRecord};
use crate::repository::{create_default_manager, RepositoryPackageEntry};
use crate::repository_sync::{get_repository_sync_status, RepositorySyncStatus};
use crate::updates::{get_updates_dashboard, UpdatesDashboardSummary};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubOverview {
    pub total_installed: usize,
    pub available_updates_count: usize,
    pub security_updates_count: usize,
    pub total_repositories: usize,
    pub online_repositories_count: usize,
    pub featured_creators: Vec<CreatorProfile>,
    pub recent_installs: Vec<InstalledPackageRecord>,
    pub latest_releases: Vec<RepositoryPackageEntry>,
    pub sync_summary: Vec<RepositorySyncStatus>,
}

#[tauri::command]
pub fn get_hub_overview() -> Result<HubOverview, String> {
    let mut installed_pkgs = list_installed_packages().unwrap_or_default();
    let updates_summary = get_updates_dashboard().unwrap_or(UpdatesDashboardSummary {
        total_updates: 0,
        security_updates_count: 0,
        feature_updates_count: 0,
        optional_updates_count: 0,
        updates: Vec::new(),
    });

    let sync_summary = get_repository_sync_status().unwrap_or_default();
    let creators = list_creator_profiles().unwrap_or_default();

    let total_installed = installed_pkgs.len();
    // Sort recent installs by installed_at descending
    installed_pkgs.sort_by(|a, b| b.installed_at.cmp(&a.installed_at));
    let recent_installs: Vec<InstalledPackageRecord> = installed_pkgs.into_iter().take(5).collect();

    // Featured creators: top 4 (sorted with Verified/Official first, then downloads)
    let featured_creators: Vec<CreatorProfile> = creators.into_iter().take(4).collect();

    // Latest / trending releases across repositories
    let manager = create_default_manager();
    let mut catalog_entries = Vec::new();
    for repo in manager.repositories() {
        if let Ok(entries) = repo.list_entries() {
            catalog_entries.extend(entries);
        }
    }
    // Sort by downloads descending
    catalog_entries.sort_by(|a, b| b.downloads.unwrap_or(0).cmp(&a.downloads.unwrap_or(0)));
    let latest_releases: Vec<RepositoryPackageEntry> =
        catalog_entries.into_iter().take(6).collect();

    let total_repositories = sync_summary.len();
    let online_repositories_count = sync_summary.iter().filter(|s| s.status == "online").count();

    Ok(HubOverview {
        total_installed,
        available_updates_count: updates_summary.total_updates,
        security_updates_count: updates_summary.security_updates_count,
        total_repositories,
        online_repositories_count,
        featured_creators,
        recent_installs,
        latest_releases,
        sync_summary,
    })
}

#[tauri::command]
pub fn get_installed_package_history(
    package_id: String,
) -> Result<Vec<InstalledHistoryEntry>, String> {
    let rec_opt = get_installed_package(package_id.clone())?;
    match rec_opt {
        Some(record) => Ok(record.history),
        None => Err(format!("Package '{}' is not installed", package_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_overview_data_aggregation() {
        let overview = get_hub_overview();
        assert!(overview.is_ok());
        let o = overview.unwrap();
        assert!(o.total_repositories >= 1);
        assert!(!o.sync_summary.is_empty());
    }

    #[test]
    fn test_installed_package_history_lookup_nonexistent() {
        let res = get_installed_package_history("nonexistent-pkg-xyz-999".to_string());
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("not installed"));
    }

    #[test]
    fn test_hub_zero_command_execution() {
        // Strict invariant: verify that none of the Phase 14 modules contain subprocess execution
        let files = ["hub.rs", "creator.rs", "updates.rs", "repository_sync.rs"];

        let forbidden_patterns = [
            "Command::new",
            "std::process::Command",
            r#""sudo""#,
            r#""pkexec""#,
            r#""pacman""#,
            r#""yay""#,
            r#""paru""#,
            r#""apt""#,
            r#""dnf""#,
            r#""sh""#,
            r#""bash""#,
        ];

        for file_name in &files {
            let candidates = [
                format!("src/{}", file_name),
                format!("src-tauri/src/{}", file_name),
                format!("../src-tauri/src/{}", file_name),
            ];
            let mut found_content = None;
            for c in &candidates {
                if let Ok(data) = std::fs::read_to_string(c) {
                    found_content = Some(data);
                    break;
                }
            }
            let content = found_content.unwrap_or_else(|| panic!("Failed to find {}", file_name));
            // Only scan production code (preceding #[cfg(test)])
            let prod_content = content.split("#[cfg(test)]").next().unwrap_or(&content);

            for pattern in &forbidden_patterns {
                assert!(
                    !prod_content.contains(pattern),
                    "Security violation: {} contains forbidden pattern '{}'",
                    file_name,
                    pattern
                );
            }
        }
    }
}
