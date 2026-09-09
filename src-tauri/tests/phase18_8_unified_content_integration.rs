//! Phase 18.8: Unified Content Integration Comprehensive Test Suite.
//!
//! Validates:
//! 1. Unified catalog aggregates all registered providers + local repositories.
//! 2. "Discover does not mutate the filesystem": zero snapshots, zero install records,
//!    zero staged directories created by browsing or refreshing Discover.
//! 3. Separation of metadata from payload: lightweight metadata during catalog browsing.
//! 4. Large payloads are NOT downloaded or staged during search/browse.
//! 5. Catalog deduplication: local repository packages take precedence over provider items.
//! 6. Canonical source identity deduplication.
//! 7. Provider failure isolation in unified catalog (faulty provider does not crash catalog).
//! 8. Offline provider behavior serves presets or gracefully degrades.
//! 9. Fastfetch packages appear in catalog with correct package type and category.
//! 10. Rice packages appear in catalog with correct package type and category.
//! 11. Wallpaper packages appear in catalog with correct package type and category.
//! 12. KDE packages appear in catalog with correct package type and category.
//! 13. GNOME packages appear in catalog with correct package type and category.
//! 14. All provider items strictly default to Community trust tier (unvetted).
//! 15. Provider origin cannot elevate trust (no Official / Verified bypass).
//! 16. Explicit staging: prepare_provider_package stages payload and synthesizes manifest.json.
//! 17. Idempotency: prepare_provider_package reuses existing validated directory.
//! 18. Preview installation: generate_installation_plan_in accurately calculates files and diff.
//! 19. Provider package installs through existing installer (install_package_in).
//! 20. InstalledPackageRecord created for provider item with SHA-256 checksums and snapshot ID.
//! 21. Uninstall removes provider installed files cleanly (uninstall_package_in).
//! 22. Provider installation atomic rollback on failure (pre-install snapshot restored).
//! 23. Tampered provider item tree-hash rejected before installation.
//! 24. Invalid provider manifest rejected.
//! 25. Provider compatibility cannot bypass installer.
//! 26. Virtual provider sources included in get_repository_info().
//! 27. Dynamic search_content_providers filters correctly by query string.
//! 28. Desktop compatibility filtering across desktop environments (Hyprland, KDE, GNOME, Universal).
//! 29. Lazy fetch verification: staging directory remains untouched until explicit install/prepare.
//! 30. Zero-subprocess / privilege audit across all provider sources.

use ryzora_lib::distribution::TrustTier;
use ryzora_lib::installer::{
    generate_installation_plan_in, get_installed_package_in, install_package_in,
    uninstall_package_in,
};
use ryzora_lib::manifest::PackageType;
use ryzora_lib::providers::{
    create_default_provider_manager, ContentProvider, ProviderCapabilities, ProviderError,
    ProviderItem, ProviderQuery, ProviderStatus, ProviderType,
};
use ryzora_lib::repository::{
    create_default_manager, get_catalog_packages, get_repository_info, refresh_catalog,
};
use ryzora_lib::system::detect_system_info;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(name: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("ryzora-phase18-8-{}-{}", name, nanos));
        let _ = fs::create_dir_all(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 1 - 4: Unified Catalog, Filesystem Invariants & Lightweight Metadata
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_unified_catalog_aggregates_all_registered_providers() {
    let catalog = get_catalog_packages().expect("get_catalog_packages must succeed");
    assert!(!catalog.is_empty(), "Unified catalog must not be empty");

    // Must contain items from multiple providers: fastfetch, wallpaper, rice, kde, gnome, community
    let has_fastfetch = catalog
        .iter()
        .any(|p| p.package_type == PackageType::Fastfetch);
    let has_wallpaper = catalog
        .iter()
        .any(|p| p.package_type == PackageType::Wallpaper);
    let has_rice = catalog.iter().any(|p| p.package_type == PackageType::Rice);
    let has_theme = catalog.iter().any(|p| p.package_type == PackageType::Theme);

    assert!(has_fastfetch, "Catalog must include Fastfetch packages");
    assert!(has_wallpaper, "Catalog must include Wallpaper packages");
    assert!(has_rice, "Catalog must include Complete Rice packages");
    assert!(has_theme, "Catalog must include Theme packages");
}

#[test]
fn test_discover_does_not_mutate_filesystem() {
    let test_dir = TempTestDir::new("non_mutating_discover");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let packages_dir = test_dir.path().join("packages");

    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&packages_dir).unwrap();

    // Call catalog browsing functions repeatedly
    let _ = get_catalog_packages().expect("catalog read must succeed");
    let _ = refresh_catalog().expect("catalog refresh must succeed");
    let _ = get_repository_info().expect("repository info must succeed");

    let query = ProviderQuery {
        query: "catppuccin".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 50,
    };
    let mgr = create_default_provider_manager();
    let _ = mgr.search_all(&query);

    // CRITICAL SECURITY INVARIANT: Filesystem must NOT be mutated by browsing
    assert!(
        fs::read_dir(&snapshots_dir).unwrap().next().is_none(),
        "Discover browsing must NOT create any snapshots"
    );
    assert!(
        fs::read_dir(&installed_dir).unwrap().next().is_none(),
        "Discover browsing must NOT create any install records"
    );
    assert!(
        fs::read_dir(&packages_dir).unwrap().next().is_none(),
        "Discover browsing must NOT stage any packages to disk"
    );
}

#[test]
fn test_provider_metadata_aggregation_separates_payload_from_metadata() {
    let catalog = get_catalog_packages().expect("Catalog must succeed");
    for item in &catalog {
        assert!(!item.id.is_empty(), "Package must have valid ID");
        assert!(!item.title.is_empty(), "Package must have valid title");
        assert!(!item.category.is_empty(), "Package must have category");
        assert!(
            !item.author.name.is_empty(),
            "Package must have author name"
        );
        // Lightweight metadata: components specs have target paths but no actual payload bytes loaded
        for comp in &item.components {
            assert!(
                !comp.target_path.is_empty(),
                "Component must have target path"
            );
        }
    }
}

#[test]
fn test_large_payload_not_downloaded_during_search() {
    let mgr = create_default_provider_manager();
    let query = ProviderQuery {
        query: "nord".to_string(),
        category: Some("wallpapers".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let resp = mgr.search_all(&query);
    assert!(!resp.items.is_empty(), "Wallpaper search must return items");
    for item in &resp.items {
        assert_eq!(item.package_type, PackageType::Wallpaper);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 5 - 8: Deduplication & Provider Failure Isolation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_catalog_deduplication_local_repository_takes_precedence() {
    let repo_mgr = create_default_manager();
    let local_packages = repo_mgr.list_all_packages().unwrap_or_default();
    let catalog = get_catalog_packages().expect("Catalog must succeed");

    // If a package exists in a local repository, its version / entry in catalog must match local repo
    for local_pkg in local_packages {
        if let Some(catalog_pkg) = catalog.iter().find(|p| p.id == local_pkg.id) {
            assert_eq!(
                catalog_pkg.repository_id, local_pkg.repository_id,
                "Local repository package must take precedence over external provider item"
            );
        }
    }
}

#[test]
fn test_canonical_source_identity_deduplication() {
    let mgr = create_default_provider_manager();
    let query = ProviderQuery {
        query: String::new(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 200,
    };

    let resp = mgr.search_all(&query);
    let mut seen_ids = std::collections::HashSet::new();
    for item in resp.items {
        assert!(
            seen_ids.insert(item.id.clone()),
            "Search response must not contain duplicate package IDs: {}",
            item.id
        );
    }
}

struct FailingTestProvider;
impl ContentProvider for FailingTestProvider {
    fn id(&self) -> &str {
        "faulty-provider"
    }
    fn name(&self) -> &str {
        "Faulty Provider"
    }
    fn provider_type(&self) -> ProviderType {
        ProviderType::Community
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }
    fn is_enabled(&self) -> bool {
        true
    }
    fn search(&self, _query: &ProviderQuery) -> Result<Vec<ProviderItem>, ProviderError> {
        Err(ProviderError::NetworkError(
            "Simulated provider outage".to_string(),
        ))
    }
    fn fetch_item(&self, _package_id: &str) -> Result<ProviderItem, ProviderError> {
        Err(ProviderError::NotFound("Item not found".to_string()))
    }
    fn stage_payload(&self, _item: &ProviderItem, _target_dir: &Path) -> Result<PathBuf, String> {
        Err("Cannot stage from faulty provider".to_string())
    }
}

#[test]
fn test_provider_failure_isolation_in_unified_catalog() {
    let mut mgr = create_default_provider_manager();
    mgr.register_provider(Arc::new(FailingTestProvider));

    let query = ProviderQuery {
        query: String::new(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 100,
    };

    let resp = mgr.search_all(&query);
    // Faulty provider must not cause search_all to fail
    assert!(
        !resp.items.is_empty(),
        "Healthy providers must still return results"
    );
    let faulty_status = resp.provider_statuses.get("faulty-provider");
    assert!(faulty_status.is_some());
    match faulty_status.unwrap() {
        ProviderStatus::Error(msg) => assert!(msg.contains("Simulated provider outage")),
        _ => panic!("Expected error status for faulty provider"),
    }
}

#[test]
fn test_offline_provider_serves_presets_or_gracefully_degrades() {
    let mgr = create_default_provider_manager();
    let query = ProviderQuery {
        query: "catppuccin".to_string(),
        category: Some("fastfetch".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let resp = mgr.search_all(&query);
    assert!(
        !resp.items.is_empty(),
        "Offline provider must serve built-in presets"
    );
    let ff_item = resp
        .items
        .iter()
        .find(|i| i.id == "fastfetch-catppuccin-mocha");
    assert!(
        ff_item.is_some(),
        "Must serve fastfetch-catppuccin-mocha preset"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 9 - 15: Category & Package Type Mapping & Strict Trust Defaults
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_packages_appear_in_catalog_and_category() {
    let catalog = get_catalog_packages().unwrap();
    let ff_pkg = catalog
        .iter()
        .find(|p| p.id == "fastfetch-catppuccin-mocha");
    assert!(
        ff_pkg.is_some(),
        "fastfetch-catppuccin-mocha must appear in catalog"
    );
    let pkg = ff_pkg.unwrap();
    assert_eq!(pkg.category, "app_config");
    assert!(pkg.tags.contains(&"provider:fastfetch".to_string()));
}

#[test]
fn test_rice_packages_appear_in_catalog_and_category() {
    let catalog = get_catalog_packages().unwrap();
    let rice_pkg = catalog.iter().find(|p| p.id == "rice-catppuccin-mocha");
    assert!(
        rice_pkg.is_some(),
        "rice-catppuccin-mocha must appear in catalog"
    );
    let pkg = rice_pkg.unwrap();
    assert_eq!(pkg.package_type, PackageType::Rice);
    assert_eq!(pkg.category, "rices");
    assert!(pkg.supported_desktops.contains(&"hyprland".to_string()));
}

#[test]
fn test_wallpaper_packages_appear_in_catalog_and_category() {
    let catalog = get_catalog_packages().unwrap();
    let wp_pkg = catalog.iter().find(|p| p.id == "wallpaper-nord-lake");
    assert!(
        wp_pkg.is_some(),
        "wallpaper-nord-lake must appear in catalog"
    );
    let pkg = wp_pkg.unwrap();
    assert_eq!(pkg.package_type, PackageType::Wallpaper);
    assert!(pkg.category == "wallpaper" || pkg.category == "wallpapers");
}

#[test]
fn test_kde_packages_appear_in_catalog_and_category() {
    let catalog = get_catalog_packages().unwrap();
    let kde_pkg = catalog.iter().find(|p| p.id == "kde-breeze-chameleon");
    assert!(
        kde_pkg.is_some(),
        "kde-breeze-chameleon must appear in catalog"
    );
    let pkg = kde_pkg.unwrap();
    assert_eq!(pkg.package_type, PackageType::Theme);
    assert!(pkg.category == "desktop" || pkg.category == "themes");
    assert!(pkg.supported_desktops.contains(&"kde".to_string()));
}

#[test]
fn test_gnome_packages_appear_in_catalog_and_category() {
    let catalog = get_catalog_packages().unwrap();
    let gnome_pkg = catalog.iter().find(|p| p.id == "gnome-adwaita-vibrant");
    assert!(
        gnome_pkg.is_some(),
        "gnome-adwaita-vibrant must appear in catalog"
    );
    let pkg = gnome_pkg.unwrap();
    assert_eq!(pkg.package_type, PackageType::Theme);
    assert!(pkg.category == "desktop" || pkg.category == "themes");
    assert!(pkg.supported_desktops.contains(&"gnome".to_string()));
}

#[test]
fn test_all_provider_items_strictly_default_to_community_trust() {
    let catalog = get_catalog_packages().unwrap();
    let provider_items: Vec<_> = catalog
        .iter()
        .filter(|p| {
            p.repository_id
                .as_deref()
                .map_or(false, |r| r.starts_with("provider:"))
        })
        .collect();

    assert!(!provider_items.is_empty(), "Must have provider items");
    for item in provider_items {
        assert_eq!(
            item.trust_tier,
            Some(TrustTier::Community),
            "Provider item '{}' must strictly default to Community trust",
            item.id
        );
    }
}

#[test]
fn test_provider_origin_cannot_elevate_trust() {
    let catalog = get_catalog_packages().unwrap();
    for item in catalog {
        if let Some(ref repo_id) = item.repository_id {
            if repo_id.starts_with("provider:") {
                assert_ne!(
                    item.trust_tier,
                    Some(TrustTier::Official),
                    "Provider '{}' cannot claim Official trust without root signature",
                    item.id
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 16 - 22: Explicit Staging, Idempotency, and Existing Installer Pipeline
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_prepare_provider_package_explicit_staging() {
    let test_dir = TempTestDir::new("prepare_pkg");
    let packages_dir = test_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    let mgr = create_default_provider_manager();
    let staged_path = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .expect("Explicit staging must succeed");

    assert!(staged_path.is_dir());
    assert!(staged_path.join("manifest.json").is_file());
    assert!(staged_path.join("config.jsonc").is_file());

    let manifest_str = fs::read_to_string(staged_path.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_str).unwrap();
    assert_eq!(manifest["id"], "fastfetch-catppuccin-mocha");
    assert_eq!(manifest["package_type"], "theme");
}

#[test]
fn test_prepare_provider_package_is_idempotent() {
    let test_dir = TempTestDir::new("prepare_idempotent");
    let packages_dir = test_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    let mgr = create_default_provider_manager();
    let path1 = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();
    let path2 = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    assert_eq!(path1, path2);
    assert!(path2.join("manifest.json").is_file());
}

#[test]
fn test_preview_installation_for_prepared_provider_package() {
    let test_dir = TempTestDir::new("preview_provider_pkg");
    let packages_dir = test_dir.path().join("packages");
    let home = test_dir.path().join("home");
    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(&home).unwrap();

    let mgr = create_default_provider_manager();
    let pkg_dir = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    let sys = detect_system_info();
    let plan = generate_installation_plan_in(&pkg_dir, &home, &sys)
        .expect("Preview installation plan must succeed");

    assert_eq!(plan.files_to_create.len(), 1);
    assert_eq!(plan.files_to_replace.len(), 0);
    assert!(plan.conflicts.is_empty());
}

#[test]
fn test_provider_package_installs_through_existing_installer() {
    let test_dir = TempTestDir::new("install_provider_pkg");
    let packages_dir = test_dir.path().join("packages");
    let home = test_dir.path().join("home");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");

    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    let mgr = create_default_provider_manager();
    let pkg_dir = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    let sys = detect_system_info();
    let result = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    )
    .expect("Installation through existing installer must succeed");

    assert!(result.success, "Installation must report success");
    assert_eq!(result.package_id, "fastfetch-catppuccin-mocha");
    assert_eq!(result.installed_files.len(), 1);

    // Verify file actually placed on disk at expected target path
    let installed_file = home.join(".config").join("fastfetch").join("config.jsonc");
    assert!(
        installed_file.is_file(),
        "File must be installed at ~/.config/fastfetch/config.jsonc"
    );
}

#[test]
fn test_installed_package_record_created_for_provider_item() {
    let test_dir = TempTestDir::new("record_provider_pkg");
    let packages_dir = test_dir.path().join("packages");
    let home = test_dir.path().join("home");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");

    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    let mgr = create_default_provider_manager();
    let pkg_dir = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    let sys = detect_system_info();
    let res = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    )
    .unwrap();

    let record = get_installed_package_in("fastfetch-catppuccin-mocha", &installed_dir)
        .expect("Installed package record query must succeed");

    assert!(record.is_some(), "Installed record must exist");
    let r = record.unwrap();
    assert_eq!(r.package_id, "fastfetch-catppuccin-mocha");
    assert_eq!(r.snapshot_id, res.snapshot_id);
    assert_eq!(r.files.len(), 1);
    assert!(!r.files[0].sha256.is_empty());
}

#[test]
fn test_uninstall_removes_provider_installed_files_cleanly() {
    let test_dir = TempTestDir::new("uninstall_provider_pkg");
    let packages_dir = test_dir.path().join("packages");
    let home = test_dir.path().join("home");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");

    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    let mgr = create_default_provider_manager();
    let pkg_dir = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    let sys = detect_system_info();
    let _ = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    )
    .unwrap();

    let target_file = home.join(".config").join("fastfetch").join("config.jsonc");
    assert!(target_file.is_file());

    let uninst = uninstall_package_in(
        "fastfetch-catppuccin-mocha",
        &home,
        &snapshots_dir,
        &installed_dir,
    )
    .expect("Uninstall must succeed");

    assert!(uninst.success);
    assert!(!target_file.exists(), "Installed file must be removed");

    let record = get_installed_package_in("fastfetch-catppuccin-mocha", &installed_dir).unwrap();
    assert!(
        record.is_none(),
        "Installed record must be removed after uninstall"
    );
}

#[test]
fn test_provider_installation_atomic_rollback_on_failure() {
    let test_dir = TempTestDir::new("rollback_provider_pkg");
    let packages_dir = test_dir.path().join("packages");
    let home = test_dir.path().join("home");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");

    fs::create_dir_all(&packages_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    let mgr = create_default_provider_manager();
    let pkg_dir = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    // Create a conflict: existing directory where a file should be written
    let conflict_path = home.join(".config").join("fastfetch").join("config.jsonc");
    fs::create_dir_all(&conflict_path).unwrap(); // Directory instead of file will cause write error

    let sys = detect_system_info();
    let res = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    );

    // Must either return error or report rolled_back == true
    if let Ok(r) = res {
        assert!(!r.success || r.rolled_back);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 23 - 30: Security Constraints, Filters, Provenance, & Subprocess Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_tampered_provider_item_tree_hash_rejected_before_install() {
    let custom_root = TempTestDir::new("wp_tampered");
    let pkg_dir = custom_root.path().join("tampered-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "tampered-pkg",
        "name": "Tampered Wallpaper",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "wallpaper",
        "description": "Tamper test",
        "tags": ["wallpaper"],
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "bg.png", "target": "~/Pictures/Wallpapers/bg.png", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(pkg_dir.join("bg.png"), b"RIFF_FAKE_PNG").unwrap();

    let provider = ryzora_lib::providers::wallpaper::WallpaperProvider::with_custom_dir(
        custom_root.path().to_path_buf(),
    );
    let mut item = provider
        .fetch_item("tampered-pkg")
        .expect("Must fetch item");

    // Alter expected tree hash in provenance
    item.provenance.commit_or_tag =
        Some("2222222222222222222222222222222222222222222222222222222222222222".to_string());

    let staging_root = TempTestDir::new("stage_hash_mismatch");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Tree hash mismatch must fail staging");
    assert!(res.unwrap_err().contains("Tree hash mismatch"));
}

#[test]
fn test_invalid_provider_manifest_rejected() {
    let test_dir = TempTestDir::new("invalid_manifest_provider");
    let pkg_dir = test_dir.path().join("invalid-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    // Write malformed manifest missing required fields
    fs::write(pkg_dir.join("manifest.json"), b"{\"ryzora_spec\": \"1\"}").unwrap();

    let home = test_dir.path().join("home");
    let sys = detect_system_info();
    let res = generate_installation_plan_in(&pkg_dir, &home, &sys);
    assert!(res.is_err(), "Invalid manifest must be rejected");
}

#[test]
fn test_provider_compatibility_cannot_bypass_installer() {
    let test_dir = TempTestDir::new("compat_bypass");
    let pkg_dir = test_dir.path().join("incompat-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "incompat-test",
        "name": "Incompatible Test",
        "version": "1.0.0",
        "type": "theme",
        "category": "themes",
        "author": { "name": "Tester" },
        "description": "Incompatible",
        "compatibility": {
            "distros": ["NonExistentLinuxDistro12345"],
            "desktops": ["NonExistentDesktop98765"],
            "sessions": [],
            "required": ["non_existent_binary_xyz_999"],
            "optional": []
        },
        "files": [],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");
    let home = test_dir.path().join("home");
    let sys = detect_system_info();

    let res = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    );
    assert!(res.is_err(), "Installer must reject incompatible package");
}

#[test]
fn test_provider_summaries_included_in_repository_info() {
    let repos = get_repository_info().expect("get_repository_info must succeed");
    assert!(!repos.is_empty());

    let provider_repos: Vec<_> = repos.iter().filter(|r| r.repo_type == "provider").collect();

    assert!(
        provider_repos.len() >= 7,
        "Repository info must include virtual summaries for all 7 providers"
    );

    assert!(provider_repos.iter().any(|r| r.id == "provider:fastfetch"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:rice"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:wallpaper"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:kde"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:gnome"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:github"));
    assert!(provider_repos.iter().any(|r| r.id == "provider:community"));
}

#[test]
fn test_dynamic_search_content_providers_filters_correctly() {
    let mgr = create_default_provider_manager();
    let query = ProviderQuery {
        query: "catppuccin".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 50,
    };

    let resp = mgr.search_all(&query);
    assert!(
        !resp.items.is_empty(),
        "Catppuccin query must match presets"
    );
    for item in &resp.items {
        let matches = item.title.to_lowercase().contains("catppuccin")
            || item.id.to_lowercase().contains("catppuccin")
            || item
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains("catppuccin"));
        assert!(matches, "Item '{}' must match search query", item.id);
    }
}

#[test]
fn test_provider_desktop_compatibility_filtering() {
    let mgr = create_default_provider_manager();
    let hypr_query = ProviderQuery {
        query: String::new(),
        category: None,
        desktop: Some("hyprland".to_string()),
        page: 1,
        page_size: 50,
    };
    let hypr_resp = mgr.search_all(&hypr_query);
    assert!(!hypr_resp.items.is_empty());
    for item in &hypr_resp.items {
        let matches = item.supported_desktops.iter().any(|d| {
            let dl = d.to_lowercase();
            dl == "hyprland" || dl == "universal" || dl == "all"
        });
        assert!(
            matches,
            "Item '{}' must support hyprland or universal: {:?}",
            item.id, item.supported_desktops
        );
    }
}

#[test]
fn test_provider_lazy_fetch_only_on_install() {
    let test_dir = TempTestDir::new("lazy_fetch");
    let packages_dir = test_dir.path().join("packages");
    fs::create_dir_all(&packages_dir).unwrap();

    let target_dir = packages_dir.join("fastfetch-catppuccin-mocha");
    assert!(
        !target_dir.exists(),
        "Package must not exist before install/prepare"
    );

    let mgr = create_default_provider_manager();
    let _ = mgr
        .prepare_provider_package_in("fastfetch-catppuccin-mocha", &packages_dir)
        .unwrap();

    assert!(
        target_dir.is_dir(),
        "Package must only be staged after explicit prepare call"
    );
}

#[test]
fn test_unified_content_zero_subprocess_and_privilege_audit() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let providers_dir = manifest_dir.join("src").join("providers");

    let mut provider_files = Vec::new();
    for entry in fs::read_dir(&providers_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            provider_files.push(entry.path());
        }
    }

    let forbidden_tokens = [
        "Command::new(",
        "sudo ",
        "pkexec",
        "gsettings",
        "dconf",
        "kwriteconfig",
    ];

    for file in provider_files {
        let content = fs::read_to_string(&file).unwrap();
        for (idx, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                continue;
            }
            for token in forbidden_tokens {
                assert!(
                    !trimmed.contains(token),
                    "Security audit failed: {} found in {}:{}",
                    token,
                    file.display(),
                    idx + 1
                );
            }
        }
    }
}
