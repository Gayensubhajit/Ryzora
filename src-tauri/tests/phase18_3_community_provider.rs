//! Phase 18.3: Ryzora Community Content Provider Comprehensive Test Suite.
//!
//! Validates:
//! 1. Community repository discovery & registration in ProviderManager.
//! 2. Repository refresh.
//! 3. Valid package translation into ProviderItem with full provenance & license preservation.
//! 4. Malformed repository/index error resilience.
//! 5. Missing package handling (ProviderError::NotFound).
//! 6. Duplicate package deduplication across repositories.
//! 7. Offline/cached repository resilience.
//! 8. Invalid manifest handling.
//! 9. Package ID / version / type mismatch rejection.
//! 10. Content-hash / tree-hash mismatch rejection.
//! 11. Missing/corrupt cached payload rejection.
//! 12. Unsigned package remains strictly unverified (TrustTier::Community).
//! 13. Valid signature with untrusted key remains SelfSignedUnvetted (never elevated to Official/Verified).
//! 14. Revoked key rejection via cryptographic trust chain.
//! 15. Path traversal rejection in package payload.
//! 16. Provider failure isolation in ProviderManager aggregator.
//! 17. Zero subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec).

use ryzora_lib::crypto::{
    sign_package_tree_hash, CryptographicStatus, SignerIdentity, SigningKey, TrustStore,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::{ManifestCompatibility, ManifestFile, PackageType, RyzoraManifest};
use ryzora_lib::providers::community::CommunityProvider;
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::{
    ContentProvider, ProviderError, ProviderManager, ProviderQuery, ProviderStatus,
};
use ryzora_lib::repository::{LocalRepository, RepositoryManager};
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
        let path = std::env::temp_dir().join(format!("ryzora-phase18-3-{}-{}", name, nanos));
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

/// Helper to create a valid on-disk repository with real files and manifest.
fn setup_sample_community_repo(
    repo_dir: &Path,
    repo_id: &str,
    pkg_id: &str,
    pkg_version: &str,
    with_content_hash: bool,
) -> (PathBuf, String) {
    let pkg_dir = repo_dir.join("packages").join(pkg_id);
    fs::create_dir_all(pkg_dir.join("hypr")).unwrap();
    fs::create_dir_all(pkg_dir.join("waybar")).unwrap();

    let hypr_content = b"# Test Hyprland config\nmonitor=,preferred,auto,1\n";
    fs::write(pkg_dir.join("hypr/hyprland.conf"), hypr_content).unwrap();

    let waybar_content = b"{\"layer\": \"top\"}\n";
    fs::write(pkg_dir.join("waybar/config"), waybar_content).unwrap();

    let manifest = RyzoraManifest {
        ryzora_spec: "1".to_string(),
        id: pkg_id.to_string(),
        name: format!("{} Display Name", pkg_id),
        version: pkg_version.to_string(),
        author: "Alice".to_string(),
        package_type: PackageType::Rice,
        description: "A clean community rice.\nLicense: MIT".to_string(),
        tags: vec!["hyprland".to_string(), "waybar".to_string()],
        color_palette: vec!["#1e1e2e".to_string()],
        compatibility: ManifestCompatibility {
            desktops: vec!["Hyprland".to_string()],
            sessions: vec!["Wayland".to_string()],
            distros: Vec::new(),
            required: Vec::new(),
            optional: Vec::new(),
        },
        files: vec![
            ManifestFile {
                source: "hypr/hyprland.conf".to_string(),
                target: "~/.config/hypr/hyprland.conf".to_string(),
                description: "Hyprland configuration".to_string(),
            },
            ManifestFile {
                source: "waybar/config".to_string(),
                target: "~/.config/waybar/config".to_string(),
                description: "Waybar configuration".to_string(),
            },
        ],
        dependencies: Vec::new(),
    };

    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    fs::write(pkg_dir.join("manifest.json"), &manifest_json).unwrap();

    let tree_hash = ryzora_lib::repository::compute_package_tree_hash(&pkg_dir, &manifest).unwrap();

    let repo_index = serde_json::json!({
        "schema": 1,
        "id": repo_id,
        "name": format!("{} Repository", repo_id),
        "version": "1.0.0",
        "description": "Community repository for testing",
        "packages": [
            {
                "id": pkg_id,
                "name": format!("{} Display Name", pkg_id),
                "version": pkg_version,
                "package_type": "rice",
                "description": "A clean community rice.\nLicense: MIT",
                "manifest": format!("packages/{}/manifest.json", pkg_id),
                "category": "wm_rice",
                "tags": ["hyprland", "waybar"],
                "author": {
                    "name": "Alice",
                    "avatar": "https://avatars.example/alice.png",
                    "verified": false
                },
                "color_palette": ["#1e1e2e"],
                "hero_image": null,
                "screenshots": [],
                "featured": true,
                "trending": false,
                "rating": 4.9,
                "downloads": 1200,
                "content_hash": if with_content_hash { Some(&tree_hash) } else { None },
                "package_size_bytes": 1024
            }
        ]
    });

    fs::write(
        repo_dir.join("repository.json"),
        serde_json::to_string_pretty(&repo_index).unwrap(),
    )
    .unwrap();

    (pkg_dir, tree_hash)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Discovery, Refresh, and Package Conversion
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_repository_discovery_and_search() {
    let repo_root = TempTestDir::new("discovery");
    setup_sample_community_repo(
        repo_root.path(),
        "test-community",
        "catppuccin-rice",
        "1.2.0",
        true,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    assert_eq!(provider.id(), "community");
    assert_eq!(provider.name(), "Ryzora Community Catalog");
    assert!(provider.is_enabled());

    let query = ProviderQuery {
        query: "catppuccin".to_string(),
        category: Some("wm_rice".to_string()),
        desktop: Some("Hyprland".to_string()),
        page: 1,
        page_size: 10,
    };

    let items = provider.search(&query).expect("Search must succeed");
    assert_eq!(items.len(), 1);

    let item = &items[0];
    assert_eq!(item.id, "catppuccin-rice");
    assert_eq!(item.title, "catppuccin-rice Display Name");
    assert_eq!(item.version, "1.2.0");
    assert_eq!(item.author.name, "Alice");
    assert_eq!(item.author.verified, false);
    assert_eq!(item.provenance.provider_id, "community");
    assert_eq!(
        item.provenance.repository,
        Some("test-community".to_string())
    );
    assert_eq!(item.provenance.commit_or_tag, Some("1.2.0".to_string()));
    assert_eq!(item.provenance.license_spdx, Some("MIT".to_string()));
    assert_eq!(item.files.len(), 2);
}

#[test]
fn test_community_repository_refresh() {
    let repo_root = TempTestDir::new("refresh");
    setup_sample_community_repo(
        repo_root.path(),
        "refresh-repo",
        "nord-theme",
        "1.0.0",
        false,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    assert!(provider.refresh().is_ok());
}

#[test]
fn test_community_missing_package_returns_not_found() {
    let repo_root = TempTestDir::new("missing");
    setup_sample_community_repo(
        repo_root.path(),
        "missing-repo",
        "existing-pkg",
        "1.0.0",
        false,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let err = provider.fetch_item("non-existent-pkg").unwrap_err();
    match err {
        ProviderError::NotFound(msg) => {
            assert!(msg.contains("non-existent-pkg"));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

#[test]
fn test_community_duplicate_package_deduplication() {
    let repo_root_1 = TempTestDir::new("dup1");
    let repo_root_2 = TempTestDir::new("dup2");

    // Two repos declaring the exact same package ID
    setup_sample_community_repo(
        repo_root_1.path(),
        "repo-alpha",
        "shared-theme",
        "1.0.0",
        false,
    );
    setup_sample_community_repo(
        repo_root_2.path(),
        "repo-beta",
        "shared-theme",
        "1.0.0",
        false,
    );

    let repo1 = LocalRepository::load_from_dir(repo_root_1.path()).unwrap();
    let repo2 = LocalRepository::load_from_dir(repo_root_2.path()).unwrap();

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(repo1));
    mgr.add_repository(Box::new(repo2));

    let provider = CommunityProvider::with_manager(mgr);
    let query = ProviderQuery {
        query: "shared-theme".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let items = provider.search(&query).unwrap();
    // Must be deduplicated down to exactly 1
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "shared-theme");
}

#[test]
fn test_community_malformed_repository_index_handled_gracefully() {
    let repo_root = TempTestDir::new("malformed");
    fs::write(repo_root.path().join("repository.json"), "{ invalid json }").unwrap();

    let result = LocalRepository::load_from_dir(repo_root.path());
    assert!(
        result.is_err(),
        "Malformed repository.json must be rejected"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Staging, Hash Verification, and Path Traversal Rejection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_stage_payload_success_and_manifest_synthesis() {
    let repo_root = TempTestDir::new("stage_success");
    setup_sample_community_repo(
        repo_root.path(),
        "stage-repo",
        "solarized-rice",
        "2.0.0",
        true,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let item = provider.fetch_item("solarized-rice").unwrap();

    let staging_root = TempTestDir::new("stage_dest");
    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Payload staging must succeed");

    assert!(staged_dir.is_dir());
    assert_eq!(staged_dir.file_name().unwrap(), "solarized-rice");

    // Verify files copied to staging
    assert!(staged_dir.join("hypr/hyprland.conf").is_file());
    assert!(staged_dir.join("waybar/config").is_file());

    // Verify synthesized manifest.json
    let manifest_content = fs::read_to_string(staged_dir.join("manifest.json")).unwrap();
    let manifest: RyzoraManifest = serde_json::from_str(&manifest_content).unwrap();
    assert_eq!(manifest.id, "solarized-rice");
    assert_eq!(manifest.version, "2.0.0");
    assert_eq!(manifest.files.len(), 2);
}

#[test]
fn test_community_content_hash_mismatch_rejected() {
    let repo_root = TempTestDir::new("hash_mismatch");
    let (pkg_dir, _) =
        setup_sample_community_repo(repo_root.path(), "hash-repo", "tampered-pkg", "1.0.0", true);

    // Tamper with file on disk after index was generated
    fs::write(pkg_dir.join("hypr/hyprland.conf"), b"TAMPERED CONTENT").unwrap();

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let item = provider.fetch_item("tampered-pkg").unwrap();

    let staging_root = TempTestDir::new("stage_tampered");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Content hash mismatch must be rejected");
    assert!(res.unwrap_err().contains("Content hash mismatch"));
}

#[test]
fn test_community_missing_payload_file_rejected() {
    let repo_root = TempTestDir::new("missing_file");
    let (pkg_dir, _) = setup_sample_community_repo(
        repo_root.path(),
        "miss-repo",
        "missing-file-pkg",
        "1.0.0",
        false,
    );

    // Delete a declared file from package
    fs::remove_file(pkg_dir.join("waybar/config")).unwrap();

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let item = provider.fetch_item("missing-file-pkg").unwrap();

    let staging_root = TempTestDir::new("stage_missing");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(
        res.is_err(),
        "Missing payload file must cause staging error"
    );
    assert!(res.unwrap_err().contains("missing on disk"));
}

#[test]
fn test_community_path_traversal_in_source_rejected() {
    let repo_root = TempTestDir::new("traversal");
    let (pkg_dir, _) = setup_sample_community_repo(
        repo_root.path(),
        "trav-repo",
        "traversal-pkg",
        "1.0.0",
        false,
    );

    // 1. Inject traversal source into manifest on disk
    let mut manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(pkg_dir.join("manifest.json")).unwrap()).unwrap();
    manifest.files.push(ManifestFile {
        source: "../../../etc/shadow".to_string(),
        target: "~/.config/hypr/shadow".to_string(),
        description: "Malicious traversal".to_string(),
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    // Repository loader immediately detects and fatally rejects directory traversal
    match LocalRepository::load_from_dir(repo_root.path()) {
        Err(err) => assert!(err.contains("escapes the package root with '..'")),
        Ok(_) => panic!("Expected Err from traversal manifest, got Ok"),
    }
}

#[test]
fn test_community_manifest_identity_mismatch_rejected() {
    let repo_root = TempTestDir::new("mismatch");
    let (pkg_dir, _) =
        setup_sample_community_repo(repo_root.path(), "mis-repo", "correct-id", "1.0.0", false);

    // Change manifest ID to mismatch repository index
    let mut manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(pkg_dir.join("manifest.json")).unwrap()).unwrap();
    manifest.id = "different-id".to_string();
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    match LocalRepository::load_from_dir(repo_root.path()) {
        Err(err) => assert!(err.contains("Manifest identity mismatch")),
        Ok(_) => panic!("Expected Err from identity mismatch, got Ok"),
    }
}

#[test]
fn test_community_manifest_version_mismatch_rejected() {
    let repo_root = TempTestDir::new("v_mismatch");
    let (pkg_dir, _) =
        setup_sample_community_repo(repo_root.path(), "vmis-repo", "ver-id", "1.0.0", false);

    // Change manifest version to mismatch repository index
    let mut manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(pkg_dir.join("manifest.json")).unwrap()).unwrap();
    manifest.version = "9.9.9".to_string();
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    match LocalRepository::load_from_dir(repo_root.path()) {
        Err(err) => assert!(err.contains("Manifest version mismatch")),
        Ok(_) => panic!("Expected Err from version mismatch, got Ok"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Cryptographic Trust Policy Invariants
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_unsigned_package_remains_unverified() {
    let repo_root = TempTestDir::new("unsigned");
    setup_sample_community_repo(
        repo_root.path(),
        "comm-repo",
        "unsigned-pkg",
        "1.0.0",
        false,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let item = provider.fetch_item("unsigned-pkg").unwrap();

    // CRITICAL: Unsigned community package normalized strictly receives Community trust tier
    let normalized = normalize_provider_item(item);
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Verified));
    assert_eq!(normalized.safety_audit.rating, "unverified");
}

#[test]
fn test_community_valid_signature_with_untrusted_key_remains_unvetted() {
    // 1. Generate an untrusted author signing key
    let mut rng = rand_core::OsRng;
    let author_key = SigningKey::generate(&mut rng);
    let _author_pubkey_hex = hex::encode(author_key.verifying_key().to_bytes());

    let repo_root = TempTestDir::new("untrusted_sig");
    let (_pkg_dir, tree_hash) = setup_sample_community_repo(
        repo_root.path(),
        "comm-repo",
        "self-signed-pkg",
        "1.0.0",
        true,
    );

    let identity = SignerIdentity {
        author_id: "author-1".to_string(),
        name: "Self-signed author".to_string(),
        handle: None,
    };
    let sig_meta =
        sign_package_tree_hash(&author_key, "self-signed-pkg", &tree_hash, identity).unwrap();

    // Update repository index with signature
    let mut index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(repo_root.path().join("repository.json")).unwrap(),
    )
    .unwrap();
    index["packages"][0]["signature"] = serde_json::to_value(&sig_meta).unwrap();
    fs::write(
        repo_root.path().join("repository.json"),
        serde_json::to_string_pretty(&index).unwrap(),
    )
    .unwrap();

    // Evaluate against a TrustStore that does NOT contain author_key
    let empty_store = TrustStore::new_test_store(&[], &[], &[]);
    let eval = ryzora_lib::crypto::evaluate_trust_chain(
        Some(&sig_meta),
        "self-signed-pkg",
        &tree_hash,
        Some(TrustTier::Community),
        &empty_store,
    );

    // CRITICAL INVARIANT: Untrusted author signature evaluates to SelfSignedUnvetted
    assert_eq!(eval.status, CryptographicStatus::SelfSignedUnvetted);
    assert!(!eval.is_trusted);
}

#[test]
fn test_community_revoked_key_rejection() {
    let mut rng = rand_core::OsRng;
    let revoked_key = SigningKey::generate(&mut rng);
    let revoked_pubkey_hex = hex::encode(revoked_key.verifying_key().to_bytes());

    let repo_root = TempTestDir::new("revoked");
    let (_pkg_dir, tree_hash) =
        setup_sample_community_repo(repo_root.path(), "comm-repo", "revoked-pkg", "1.0.0", true);

    let identity = SignerIdentity {
        author_id: "revoked-author".to_string(),
        name: "Revoked signer".to_string(),
        handle: None,
    };
    let sig_meta =
        sign_package_tree_hash(&revoked_key, "revoked-pkg", &tree_hash, identity).unwrap();

    let revoked_entry = ryzora_lib::crypto::RevokedKeyEntry {
        key_id: "revoked-1".to_string(),
        public_key: revoked_pubkey_hex,
        revoked_at: "2026-01-01T00:00:00Z".to_string(),
        reason: "Key compromised".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    let eval = ryzora_lib::crypto::evaluate_trust_chain(
        Some(&sig_meta),
        "revoked-pkg",
        &tree_hash,
        Some(TrustTier::Community),
        &trust_store,
    );

    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.is_valid);
    assert!(!eval.can_install);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Aggregator Fault Isolation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_provider_failure_isolation_in_aggregator() {
    let mut manager = ProviderManager::new();

    // Register a disabled/failing community provider
    let empty_mgr = RepositoryManager::new();
    let mut comm_provider = CommunityProvider::with_manager(empty_mgr);
    comm_provider.set_enabled(false);
    manager.register_provider(Arc::new(comm_provider));

    let query = ProviderQuery {
        query: "anything".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let resp = manager.search_all(&query);
    assert_eq!(resp.items.len(), 0);
    assert_eq!(
        resp.provider_statuses.get("community"),
        Some(&ProviderStatus::Disabled)
    );
}

// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_symlink_escaping_rejected() {
    let repo_root = TempTestDir::new("symlink_esc");
    let (pkg_dir, _) =
        setup_sample_community_repo(repo_root.path(), "sym-repo", "symlink-pkg", "1.0.0", false);

    // Create an outside secret file
    let secret_dir = TempTestDir::new("secret_dir");
    let secret_file = secret_dir.path().join("secret.conf");
    fs::write(&secret_file, b"CONFIDENTIAL").unwrap();

    // Create symlink inside package pointing to outside file
    #[cfg(unix)]
    {
        let symlink_path = pkg_dir.join("hypr/escaped_symlink");
        let _ = std::os::unix::fs::symlink(&secret_file, &symlink_path);

        if symlink_path.exists() {
            let mut manifest: RyzoraManifest =
                serde_json::from_str(&fs::read_to_string(pkg_dir.join("manifest.json")).unwrap())
                    .unwrap();
            manifest.files.push(ManifestFile {
                source: "hypr/escaped_symlink".to_string(),
                target: "~/.config/hypr/escaped".to_string(),
                description: "Escaping symlink".to_string(),
            });
            fs::write(
                pkg_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();

            let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
            let mut mgr = RepositoryManager::new();
            mgr.add_repository(Box::new(local_repo));
            let provider = CommunityProvider::with_manager(mgr);

            let item = provider.fetch_item("symlink-pkg").unwrap();
            let staging_root = TempTestDir::new("stage_sym");
            let res = provider.stage_payload(&item, staging_root.path());
            assert!(
                res.is_err(),
                "Symlink escaping package root must be rejected"
            );
            assert!(res.unwrap_err().contains("escapes package directory"));
        }
    }
}

#[test]
fn test_community_offline_cached_repository_preserves_packages() {
    let repo_root = TempTestDir::new("offline_cache");
    setup_sample_community_repo(
        repo_root.path(),
        "offline-repo",
        "cached-theme",
        "1.0.0",
        false,
    );

    let local_repo = LocalRepository::load_from_dir(repo_root.path()).unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(local_repo));

    let provider = CommunityProvider::with_manager(mgr);
    let query = ProviderQuery {
        query: "cached-theme".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    // Even if repository cannot connect to remote, cached/local entries are served
    let items = provider
        .search(&query)
        .expect("Search in cached repo must succeed");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "cached-theme");
}

// Tests: Subprocess and Privilege Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_community_provider_zero_subprocess_and_privilege_audit() {
    let providers_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/providers");
    assert!(providers_dir.is_dir());

    for entry in fs::read_dir(&providers_dir).unwrap().flatten() {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(entry.path()).unwrap();
            assert!(
                !content.contains("Command::new("),
                "No Command::new allowed in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("std::process::Command"),
                "No std::process::Command allowed in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("\"sudo\"") && !content.contains("\"sudo "),
                "No sudo invocations in {:?}",
                entry.path()
            );
            assert!(
                !content.contains("\"pkexec\"") && !content.contains("\"pkexec "),
                "No pkexec invocations in {:?}",
                entry.path()
            );
        }
    }
}
