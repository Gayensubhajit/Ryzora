//! Phase 18.4: Fastfetch Content Provider Comprehensive Test Suite.
//!
//! Validates:
//! 1. Provider registration & discovery in ProviderManager.
//! 2. Search & category filtering ("app_config", "fastfetch", "terminal").
//! 3. Valid Fastfetch package conversion to ProviderItem with provenance.
//! 4. ~/.config/fastfetch/ target accepted (config.jsonc).
//! 5. ~/.config/fastfetch/presets/... accepted.
//! 6. ~/.config/fastfetch/logos/... accepted.
//! 7. Target outside Fastfetch directory rejected (~/.bashrc, ~/.config/hypr/...).
//! 8. Target traversal ('..') rejected.
//! 9. Absolute target rejected (/etc/fastfetch.json).
//! 10. Source traversal ('..') rejected.
//! 11. Symlink escape from package root rejected.
//! 12. Script/hook payload rejected (*.sh, *.py, install.sh).
//! 13. Executable/binary payload rejected (*.exe, *.bin, *.so).
//! 14. Manifest ID / version mismatch rejected.
//! 15. Content / tree-hash verification.
//! 16. Unsigned content remains unverified (TrustTier::Community).
//! 17. Revoked signature rejected.
//! 18. Provider failure isolation in ProviderManager aggregator.
//! 19. Offline / cached behavior works without network.
//! 20. Zero-subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec).

use ryzora_lib::crypto::{
    sign_package_tree_hash, CryptographicStatus, RevokedKeyEntry, SignerIdentity, SigningKey,
    TrustStore,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::{PackageType, RyzoraManifest};
use ryzora_lib::providers::fastfetch::{
    validate_fastfetch_target_and_source, FastfetchProvider, FASTFETCH_TARGET_PREFIX,
};
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::{
    ContentProvider, ProviderFileSpec, ProviderManager, ProviderQuery, ProviderStatus,
};
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
        let path = std::env::temp_dir().join(format!("ryzora-phase18-4-{}-{}", name, nanos));
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
// Tests 1-3: Provider Discovery, Search, and Package Conversion
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_provider_registration_and_capabilities() {
    let provider = FastfetchProvider::new();
    assert_eq!(provider.id(), "fastfetch");
    assert_eq!(provider.name(), "Fastfetch Themes & Presets");
    assert!(provider.is_enabled());

    let caps = provider.capabilities();
    assert!(caps.can_search);
    assert!(caps.can_fetch_item);
    assert!(caps.can_stage_payload);
    assert!(caps.supports_categories);
    assert!(caps.supports_pagination);

    let mut manager = ProviderManager::new();
    manager.register_provider(Arc::new(provider));
    let summaries = manager.list_providers();
    assert!(summaries.iter().any(|s| s.id == "fastfetch"));
}

#[test]
fn test_fastfetch_search_and_category_filtering() {
    let provider = FastfetchProvider::new();

    // 1. Text search
    let query_mocha = ProviderQuery {
        query: "mocha".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let items = provider.search(&query_mocha).expect("Search must succeed");
    assert!(!items.is_empty());
    assert!(items.iter().any(|i| i.id.contains("mocha")));

    // 2. Category filtering: "app_config"
    let query_cat = ProviderQuery {
        query: "".to_string(),
        category: Some("app_config".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let items_cat = provider.search(&query_cat).expect("Search must succeed");
    assert_eq!(items_cat.len(), 3); // 3 default presets

    // 3. Category filtering: irrelevant category should return empty
    let query_other = ProviderQuery {
        query: "".to_string(),
        category: Some("wallpaper".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let items_other = provider.search(&query_other).expect("Search must succeed");
    assert!(items_other.is_empty());
}

#[test]
fn test_fastfetch_valid_package_conversion_and_provenance() {
    let provider = FastfetchProvider::new();
    let item = provider
        .fetch_item("fastfetch-catppuccin-mocha")
        .expect("Must fetch catppuccin preset");

    assert_eq!(item.id, "fastfetch-catppuccin-mocha");
    assert_eq!(item.title, "Catppuccin Mocha Fastfetch");
    assert_eq!(item.category, "app_config");
    assert_eq!(item.package_type, PackageType::Theme);
    assert_eq!(item.version, "1.0.0");
    assert_eq!(item.author.name, "Catppuccin");
    assert_eq!(item.author.verified, false);

    // Provenance
    assert_eq!(item.provenance.provider_id, "fastfetch");
    assert!(item
        .provenance
        .source_url
        .starts_with("fastfetch://presets/"));
    assert_eq!(item.provenance.license_spdx, Some("MIT".to_string()));

    // Files: config.jsonc
    assert!(!item.files.is_empty());
    assert_eq!(item.files[0].source, "config.jsonc");
    assert_eq!(item.files[0].target, "~/.config/fastfetch/config.jsonc");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests 4-6: Allowed Target Placements (config.jsonc, presets/**, logos/**)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_target_confinement_accepts_valid_paths() {
    // 4. Root config.jsonc
    let file1 = ProviderFileSpec {
        source: "config.jsonc".to_string(),
        target: "~/.config/fastfetch/config.jsonc".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&file1, "pkg").is_ok());

    // Root config.json
    let file2 = ProviderFileSpec {
        source: "config.json".to_string(),
        target: "~/.config/fastfetch/config.json".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&file2, "pkg").is_ok());

    // 5. Presets subfolder
    let file3 = ProviderFileSpec {
        source: "presets/compact.jsonc".to_string(),
        target: "~/.config/fastfetch/presets/compact.jsonc".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&file3, "pkg").is_ok());

    // 6. Logos subfolder
    let file4 = ProviderFileSpec {
        source: "logos/catppuccin.txt".to_string(),
        target: "~/.config/fastfetch/logos/catppuccin.txt".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&file4, "pkg").is_ok());

    // ASCII art subfolder
    let file5 = ProviderFileSpec {
        source: "ascii/logo.ansi".to_string(),
        target: "~/.config/fastfetch/ascii/logo.ansi".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&file5, "pkg").is_ok());
}

// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_presets_and_logos_subpaths_accepted() {
    let preset_spec = ProviderFileSpec {
        source: "presets/neon.jsonc".to_string(),
        target: "~/.config/fastfetch/presets/neon.jsonc".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&preset_spec, "neon").is_ok());

    let logo_spec = ProviderFileSpec {
        source: "logos/tux.ansi".to_string(),
        target: "~/.config/fastfetch/logos/tux.ansi".to_string(),
    };
    assert!(validate_fastfetch_target_and_source(&logo_spec, "tux").is_ok());
}

// Tests 7-11: Security Target & Traversal Rejection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_rejects_target_outside_fastfetch_dir() {
    let forbidden_targets = [
        "~/.bashrc",
        "~/.zshrc",
        "~/.config/hypr/hyprland.conf",
        "~/.config/waybar/config",
        "~/.local/bin/run",
        "~/Pictures/Wallpapers/img.png",
    ];

    for tgt in &forbidden_targets {
        let file = ProviderFileSpec {
            source: "config.jsonc".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(
            res.is_err(),
            "Target outside fastfetch dir must be rejected: {}",
            tgt
        );
        assert!(res.unwrap_err().contains(FASTFETCH_TARGET_PREFIX));
    }
}

#[test]
fn test_fastfetch_rejects_target_traversal() {
    let traversal_targets = [
        "~/.config/fastfetch/../hypr/hyprland.conf",
        "~/.config/fastfetch/../../.bashrc",
        "~/.config/fastfetch/presets/../../evil",
    ];

    for tgt in &traversal_targets {
        let file = ProviderFileSpec {
            source: "config.jsonc".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(
            res.is_err(),
            "Target with traversal must be rejected: {}",
            tgt
        );
        assert!(res.unwrap_err().contains("parent traversal"));
    }
}

#[test]
fn test_fastfetch_rejects_absolute_target() {
    let absolute_targets = [
        "/etc/fastfetch.json",
        "/usr/share/fastfetch/config",
        "/etc/shadow",
    ];

    for tgt in &absolute_targets {
        let file = ProviderFileSpec {
            source: "config.jsonc".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(res.is_err(), "Absolute target must be rejected: {}", tgt);
    }
}

#[test]
fn test_fastfetch_rejects_source_traversal() {
    let traversal_sources = [
        "../secret.conf",
        "../../etc/shadow",
        "nested/../../outside.txt",
        "/etc/passwd",
    ];

    for src in &traversal_sources {
        let file = ProviderFileSpec {
            source: src.to_string(),
            target: "~/.config/fastfetch/config.jsonc".to_string(),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(
            res.is_err(),
            "Source with traversal must be rejected: {}",
            src
        );
    }
}

#[test]
fn test_fastfetch_rejects_symlink_escape() {
    let custom_root = TempTestDir::new("symlink_pkg");
    let pkg_dir = custom_root.path().join("evil-ff");
    fs::create_dir_all(&pkg_dir).unwrap();

    let secret_dir = TempTestDir::new("secret_dir");
    let secret_file = secret_dir.path().join("id_rsa");
    fs::write(&secret_file, b"PRIVATE KEY").unwrap();

    #[cfg(unix)]
    {
        let symlink_path = pkg_dir.join("config.jsonc");
        let _ = std::os::unix::fs::symlink(&secret_file, &symlink_path);

        if symlink_path.exists() {
            let manifest = serde_json::json!({
                "ryzora_spec": "1",
                "id": "evil-ff",
                "name": "Evil Fastfetch",
                "version": "1.0.0",
                "author": "Attacker",
                "package_type": "theme",
                "description": "Escapes root",
                "tags": ["fastfetch"],
                "color_palette": [],
                "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
                "files": [
                    { "source": "config.jsonc", "target": "~/.config/fastfetch/config.jsonc", "description": "link" }
                ],
                "dependencies": []
            });
            fs::write(
                pkg_dir.join("manifest.json"),
                serde_json::to_string(&manifest).unwrap(),
            )
            .unwrap();

            let provider = FastfetchProvider::with_custom_dir(custom_root.path().to_path_buf());
            let item = provider.fetch_item("evil-ff").unwrap();
            let staging_dest = TempTestDir::new("sym_dest");

            let res = provider.stage_payload(&item, staging_dest.path());
            assert!(
                res.is_err(),
                "Symlink escape must be rejected during staging"
            );
            assert!(res
                .unwrap_err()
                .contains("escapes package root via symlink"));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests 12-13: Script & Executable Rejection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_rejects_scripts_and_installer_hooks() {
    let scripts = [
        "install.sh",
        "setup.sh",
        "hook.bash",
        "runner.zsh",
        "config.fish",
        "exploit.py",
    ];

    for s in &scripts {
        let file = ProviderFileSpec {
            source: s.to_string(),
            target: format!("~/.config/fastfetch/{}", s),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(res.is_err(), "Script file must be rejected: {}", s);
        assert!(res.unwrap_err().contains("prohibited"));
    }
}

#[test]
fn test_fastfetch_rejects_binaries_and_executables() {
    let binaries = [
        "payload.exe",
        "blob.bin",
        "library.so",
        "driver.dll",
        "virus.scr",
        "script.vbs",
    ];

    for b in &binaries {
        let file = ProviderFileSpec {
            source: b.to_string(),
            target: format!("~/.config/fastfetch/{}", b),
        };
        let res = validate_fastfetch_target_and_source(&file, "pkg");
        assert!(res.is_err(), "Binary/executable must be rejected: {}", b);
        assert!(res.unwrap_err().contains("prohibited"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests 14-17: Manifest, Tree Hash, and Trust Policy
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_manifest_identity_mismatch_rejected() {
    let custom_root = TempTestDir::new("mismatch_pkg");
    let pkg_dir = custom_root.path().join("ff-nord");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "different-id", // ID mismatch!
        "name": "Nord",
        "version": "1.0.0",
        "author": "Arctic",
        "package_type": "theme",
        "description": "Nord",
        "tags": ["fastfetch"],
        "color_palette": [],
        "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
        "files": [
            { "source": "config.jsonc", "target": "~/.config/fastfetch/config.jsonc", "description": "nord" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(pkg_dir.join("config.jsonc"), b"{}").unwrap();

    // load_disk_preset_item immediately catches directory name vs manifest.id mismatch
    let load_res = ryzora_lib::providers::fastfetch::load_disk_preset_item(&pkg_dir);
    assert!(load_res.is_err(), "Identity mismatch must be rejected");
    assert!(load_res.unwrap_err().contains("Identity mismatch"));
}

#[test]
fn test_fastfetch_content_tree_hash_mismatch_rejected() {
    let custom_root = TempTestDir::new("hash_pkg");
    let pkg_dir = custom_root.path().join("ff-tampered");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "ff-tampered",
        "name": "Tampered Fastfetch",
        "version": "1.0.0",
        "author": "Author",
        "package_type": "theme",
        "description": "Tampered",
        "tags": ["fastfetch"],
        "color_palette": [],
        "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
        "files": [
            { "source": "config.jsonc", "target": "~/.config/fastfetch/config.jsonc", "description": "config" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(pkg_dir.join("config.jsonc"), br#"{"real": true}"#).unwrap();

    let provider = FastfetchProvider::with_custom_dir(custom_root.path().to_path_buf());
    let mut item = provider.fetch_item("ff-tampered").expect("Must fetch item");

    // Inject fake/wrong expected hash into provenance commit_or_tag (64-char hex)
    item.provenance.commit_or_tag =
        Some("0000000000000000000000000000000000000000000000000000000000000000".to_string());

    let staging_root = TempTestDir::new("stage_hash_mismatch");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Tree hash mismatch must fail staging");
    assert!(res.unwrap_err().contains("Tree hash mismatch"));
}

#[test]
fn test_fastfetch_stage_payload_flow_and_manifest_synthesis() {
    let provider = FastfetchProvider::new();
    let item = provider.fetch_item("fastfetch-nord-frost").unwrap();

    let staging_root = TempTestDir::new("stage_nord");
    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Payload staging must succeed");

    assert!(staged_dir.is_dir());
    assert_eq!(staged_dir.file_name().unwrap(), "fastfetch-nord-frost");

    // Check config.jsonc written
    let config_path = staged_dir.join("config.jsonc");
    assert!(config_path.is_file());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        content.contains("https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json")
    );

    // Check manifest.json synthesized
    let manifest_path = staged_dir.join("manifest.json");
    assert!(manifest_path.is_file());
    let manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.id, "fastfetch-nord-frost");
    assert_eq!(manifest.files[0].target, "~/.config/fastfetch/config.jsonc");
}

#[test]
fn test_fastfetch_unsigned_content_remains_unverified() {
    let provider = FastfetchProvider::new();
    let item = provider.fetch_item("fastfetch-dracula-neon").unwrap();

    let normalized = normalize_provider_item(item);
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Verified));
    assert_eq!(normalized.safety_audit.rating, "unverified");
    assert!(normalized.tags.contains(&"provider:fastfetch".to_string()));
}

#[test]
fn test_fastfetch_revoked_signature_rejected() {
    let mut rng = rand_core::OsRng;
    let revoked_key = SigningKey::generate(&mut rng);
    let revoked_pubkey_hex = hex::encode(revoked_key.verifying_key().to_bytes());

    let identity = SignerIdentity {
        author_id: "revoked-author".to_string(),
        name: "Revoked signer".to_string(),
        handle: None,
    };
    let tree_hash = "abc123treehash";
    let sig_meta = sign_package_tree_hash(
        &revoked_key,
        "fastfetch-catppuccin-mocha",
        tree_hash,
        identity,
    )
    .unwrap();

    let revoked_entry = RevokedKeyEntry {
        key_id: "revoked-ff".to_string(),
        public_key: revoked_pubkey_hex,
        revoked_at: "2026-01-01T00:00:00Z".to_string(),
        reason: "Compromised".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    let eval = ryzora_lib::crypto::evaluate_trust_chain(
        Some(&sig_meta),
        "fastfetch-catppuccin-mocha",
        tree_hash,
        Some(TrustTier::Community),
        &trust_store,
    );

    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.is_valid);
    assert!(!eval.can_install);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests 18-20: Aggregator Isolation, Offline Behavior, and Security Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_fastfetch_provider_failure_isolation_in_aggregator() {
    let mut manager = ProviderManager::new();

    let mut ff_provider = FastfetchProvider::new();
    ff_provider.set_enabled(false);
    manager.register_provider(Arc::new(ff_provider));

    let query = ProviderQuery {
        query: "mocha".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let resp = manager.search_all(&query);
    assert_eq!(resp.items.len(), 0);
    assert_eq!(
        resp.provider_statuses.get("fastfetch"),
        Some(&ProviderStatus::Disabled)
    );
}

#[test]
fn test_fastfetch_offline_behavior_serves_presets() {
    let provider = FastfetchProvider::new();
    let query = ProviderQuery {
        query: "dracula".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    // Zero network calls needed: serves built-in presets completely offline
    let items = provider
        .search(&query)
        .expect("Offline search must succeed");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "fastfetch-dracula-neon");
}

#[test]
fn test_fastfetch_provider_zero_subprocess_and_privilege_audit() {
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
