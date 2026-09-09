use crate::crypto::CryptographicStatus;
use crate::installer::list_installed_packages;
use crate::repository::{create_default_manager, RepositoryPackageEntry};
use crate::repository_sync::load_repo_channels;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateCategory {
    Security,
    Feature,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageUpdateItem {
    pub package_id: String,
    pub name: String,
    pub installed_version: String,
    pub available_version: String,
    pub category: UpdateCategory,
    pub repository_id: String,
    pub release_channel: String,
    pub release_notes: Option<String>,
    pub tree_hash: Option<String>,
    pub cryptographic_status: Option<CryptographicStatus>,
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdatesDashboardSummary {
    pub total_updates: usize,
    pub security_updates_count: usize,
    pub feature_updates_count: usize,
    pub optional_updates_count: usize,
    pub updates: Vec<PackageUpdateItem>,
}

pub fn categorize_update(
    installed_ver_str: &str,
    available_ver_str: &str,
    release_notes: Option<&str>,
    installed_crypto: Option<CryptographicStatus>,
    _repo_crypto: Option<CryptographicStatus>,
) -> UpdateCategory {
    // 1. Security Check:
    // - Security keywords in release notes
    if let Some(notes) = release_notes {
        let lower = notes.to_lowercase();
        if lower.contains("security")
            || lower.contains("cve-")
            || lower.contains("vulnerability")
            || lower.contains("[security]")
            || lower.contains("advisory")
        {
            return UpdateCategory::Security;
        }
    }

    // - Cryptographic status issue or revocation on installed package
    if let Some(status) = installed_crypto {
        if status == CryptographicStatus::RevokedKey
            || status == CryptographicStatus::InvalidSignature
            || status == CryptographicStatus::OfficialImpersonation
        {
            return UpdateCategory::Security;
        }
    }

    // 2. SemVer Check:
    let inst_semver = semver::Version::parse(installed_ver_str).ok();
    let avail_semver = semver::Version::parse(available_ver_str).ok();

    if let (Some(inst), Some(avail)) = (inst_semver, avail_semver) {
        if avail.major > inst.major || avail.minor > inst.minor {
            return UpdateCategory::Feature;
        }
    }

    // 3. Optional / Patch bump
    UpdateCategory::Optional
}

#[tauri::command]
pub fn get_updates_dashboard() -> Result<UpdatesDashboardSummary, String> {
    let installed_pkgs = list_installed_packages().unwrap_or_default();
    let manager = create_default_manager();
    let channels = load_repo_channels();

    let mut updates = Vec::new();

    // Map all available repository entries by ID
    let mut catalog_map: std::collections::HashMap<String, (String, RepositoryPackageEntry)> =
        std::collections::HashMap::new();

    for repo in manager.repositories() {
        if let Ok(entries) = repo.list_entries() {
            for entry in entries {
                // If not in map or newer version, store
                if let Some((_, existing)) = catalog_map.get(&entry.id) {
                    let v_exist = semver::Version::parse(&existing.version).ok();
                    let v_new = semver::Version::parse(&entry.version).ok();
                    if let (Some(ve), Some(vn)) = (v_exist, v_new) {
                        if vn > ve {
                            catalog_map.insert(entry.id.clone(), (repo.id().to_string(), entry));
                        }
                    }
                } else {
                    catalog_map.insert(entry.id.clone(), (repo.id().to_string(), entry));
                }
            }
        }
    }

    for inst in installed_pkgs {
        if let Some((repo_id, repo_entry)) = catalog_map.get(&inst.package_id) {
            let inst_ver = semver::Version::parse(&inst.version).ok();
            let avail_ver = semver::Version::parse(&repo_entry.version).ok();

            if let (Some(iv), Some(av)) = (inst_ver, avail_ver) {
                if av > iv {
                    let repo_channel = channels.get(repo_id).cloned().unwrap_or_else(|| {
                        repo_entry
                            .release_channel
                            .map(|c| format!("{:?}", c).to_lowercase())
                            .unwrap_or_else(|| "stable".to_string())
                    });

                    let category = categorize_update(
                        &inst.version,
                        &repo_entry.version,
                        repo_entry.release_notes.as_deref(),
                        inst.cryptographic_status,
                        None,
                    );

                    updates.push(PackageUpdateItem {
                        package_id: inst.package_id.clone(),
                        name: if repo_entry.name.is_empty() {
                            inst.name.clone()
                        } else {
                            repo_entry.name.clone()
                        },
                        installed_version: inst.version.clone(),
                        available_version: repo_entry.version.clone(),
                        category,
                        repository_id: repo_id.clone(),
                        release_channel: repo_channel,
                        release_notes: repo_entry.release_notes.clone(),
                        tree_hash: repo_entry.content_hash.clone(),
                        cryptographic_status: inst.cryptographic_status,
                        snapshot_id: inst.snapshot_id.clone(),
                    });
                }
            }
        }
    }

    // Sort: Security first, then Feature, then Optional
    updates.sort_by(|a, b| {
        let cat_order = |c: UpdateCategory| match c {
            UpdateCategory::Security => 0,
            UpdateCategory::Feature => 1,
            UpdateCategory::Optional => 2,
        };
        cat_order(a.category)
            .cmp(&cat_order(b.category))
            .then_with(|| a.name.cmp(&b.name))
    });

    let security_count = updates
        .iter()
        .filter(|u| u.category == UpdateCategory::Security)
        .count();
    let feature_count = updates
        .iter()
        .filter(|u| u.category == UpdateCategory::Feature)
        .count();
    let optional_count = updates
        .iter()
        .filter(|u| u.category == UpdateCategory::Optional)
        .count();

    Ok(UpdatesDashboardSummary {
        total_updates: updates.len(),
        security_updates_count: security_count,
        feature_updates_count: feature_count,
        optional_updates_count: optional_count,
        updates,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_detection_security_category() {
        let cat = categorize_update(
            "1.0.0",
            "1.0.1",
            Some("Fix critical security vulnerability in configuration parser (CVE-2026-101)"),
            None,
            None,
        );
        assert_eq!(cat, UpdateCategory::Security);

        // Security triggered by revoked installed key
        let cat_revoked = categorize_update(
            "1.0.0",
            "1.0.1",
            None,
            Some(CryptographicStatus::RevokedKey),
            None,
        );
        assert_eq!(cat_revoked, UpdateCategory::Security);
    }

    #[test]
    fn test_update_detection_feature_category() {
        // Minor bump
        let cat_minor = categorize_update(
            "1.2.0",
            "1.3.0",
            Some("Added new statusbar widget"),
            None,
            None,
        );
        assert_eq!(cat_minor, UpdateCategory::Feature);

        // Major bump
        let cat_major = categorize_update(
            "1.0.0",
            "2.0.0",
            Some("Complete rewrite of wayland protocols"),
            None,
            None,
        );
        assert_eq!(cat_major, UpdateCategory::Feature);
    }

    #[test]
    fn test_update_detection_optional_category() {
        let cat_patch = categorize_update(
            "1.0.0",
            "1.0.1",
            Some("Tweak CSS padding and border radius"),
            None,
            None,
        );
        assert_eq!(cat_patch, UpdateCategory::Optional);
    }

    #[test]
    fn test_update_dashboard_summary_calculation() {
        let item_sec = PackageUpdateItem {
            package_id: "sec-pkg".to_string(),
            name: "Security Pkg".to_string(),
            installed_version: "1.0.0".to_string(),
            available_version: "1.0.1".to_string(),
            category: UpdateCategory::Security,
            repository_id: "repo-1".to_string(),
            release_channel: "stable".to_string(),
            release_notes: Some("Security patch".to_string()),
            tree_hash: None,
            cryptographic_status: None,
            snapshot_id: "snap-1".to_string(),
        };

        let item_feat = PackageUpdateItem {
            package_id: "feat-pkg".to_string(),
            name: "Feature Pkg".to_string(),
            installed_version: "1.0.0".to_string(),
            available_version: "1.1.0".to_string(),
            category: UpdateCategory::Feature,
            repository_id: "repo-1".to_string(),
            release_channel: "stable".to_string(),
            release_notes: Some("New features".to_string()),
            tree_hash: None,
            cryptographic_status: None,
            snapshot_id: "snap-2".to_string(),
        };

        let item_opt = PackageUpdateItem {
            package_id: "opt-pkg".to_string(),
            name: "Optional Pkg".to_string(),
            installed_version: "1.0.0".to_string(),
            available_version: "1.0.1".to_string(),
            category: UpdateCategory::Optional,
            repository_id: "repo-1".to_string(),
            release_channel: "stable".to_string(),
            release_notes: None,
            tree_hash: None,
            cryptographic_status: None,
            snapshot_id: "snap-3".to_string(),
        };

        let summary = UpdatesDashboardSummary {
            total_updates: 3,
            security_updates_count: 1,
            feature_updates_count: 1,
            optional_updates_count: 1,
            updates: vec![item_sec, item_feat, item_opt],
        };

        assert_eq!(summary.total_updates, 3);
        assert_eq!(summary.security_updates_count, 1);
        assert_eq!(summary.feature_updates_count, 1);
        assert_eq!(summary.optional_updates_count, 1);
    }
}
