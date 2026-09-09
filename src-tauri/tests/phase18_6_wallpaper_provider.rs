//! Phase 18.6: Wallpaper Content Provider Comprehensive Test Suite.
//!
//! Validates:
//! 1. Provider registration & discovery in ProviderManager.
//! 2. Search & category/tag filtering ("wallpaper", "catppuccin", "4k").
//! 3. Conversion to ProviderItem with complete provenance (wallpaper://presets/...).
//! 4. Target confinement: ~/Pictures/Wallpapers/** accepted.
//! 5. Allowed image extensions (.png, .jpg, .jpeg, .webp, .jxl, .svg, .avif) accepted.
//! 6. Target outside wallpaper directory rejected (~/.config/, ~/.bashrc, etc.).
//! 7. Non-image extensions rejected (.txt, .json, .theme, .conf).
//! 8. Scripts and installer hooks rejected (*.sh, *.py, install.sh, setup.sh).
//! 9. Binaries and executables rejected (*.bin, *.exe, *.so, *.elf).
//! 10. Target traversal ('..') and null bytes rejected.
//! 11. Absolute targets rejected (/etc/..., /usr/share/backgrounds/...).
//! 12. Source traversal ('..') and absolute sources rejected.
//! 13. Duplicate normalized target collision rejected (//, ./foo.png).
//! 14. SVG active script injection rejected (<script>).
//! 15. Oversized wallpaper payload rejected (>50MB limit).
//! 16. Manifest identity / version mismatch rejected.
//! 17. Content tree-hash verification and tampering rejection.
//! 18. Symlink escape from package root rejected during staging.
//! 19. Transactional staging: atomic rollback/purge on partial failure (zero leftovers).
//! 20. Successful multi-file wallpaper pack staging and manifest synthesis.
//! 21. Unsigned content remains unverified (TrustTier::Community).
//! 22. Revoked key signature rejected.
//! 23. Provider failure isolation in ProviderManager aggregator.
//! 24. Offline behavior serves built-in presets without network.
//! 25. Zero-subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec).

use ryzora_lib::crypto::{
    evaluate_trust_chain, sign_package_tree_hash, CryptographicStatus, RevokedKeyEntry,
    SignerIdentity, SigningKey, TrustStore,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::{PackageType, RyzoraManifest};
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::wallpaper::{
    load_disk_wallpaper_item, validate_wallpaper_target_and_source, WallpaperProvider,
    MAX_WALLPAPER_FILE_SIZE, WALLPAPER_TARGET_PREFIX,
};
use ryzora_lib::providers::{
    ContentProvider, ProviderFileSpec, ProviderManager, ProviderQuery, ProviderStatus,
};
use std::collections::HashSet;
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
        let path = std::env::temp_dir().join(format!("ryzora-phase18-6-{}-{}", name, nanos));
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
// Test Vectors 1 - 3: Registration, Search, and Provenance
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallpaper_provider_registration_and_capabilities() {
    let mut manager = ProviderManager::new();
    let provider = Arc::new(WallpaperProvider::new());
    manager.register_provider(provider);

    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);

    let info = &providers[0];
    assert_eq!(info.id, "wallpaper");
    assert_eq!(info.name, "Curated Wallpaper Collections");
    assert!(info.enabled);
    assert_eq!(info.status, ProviderStatus::Online);
    assert!(info.capabilities.can_search);
    assert!(info.capabilities.can_fetch_item);
    assert!(info.capabilities.can_stage_payload);
    assert!(info.capabilities.supports_categories);
}

#[test]
fn test_wallpaper_search_and_tag_filtering() {
    let provider = WallpaperProvider::new();

    // Query match "twilight"
    let q_twilight = ProviderQuery {
        query: "twilight".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res = provider.search(&q_twilight).expect("Search must succeed");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "wallpaper-catppuccin-twilight");
    assert_eq!(res[0].package_type, PackageType::Wallpaper);

    // Case insensitivity "CYBERPUNK"
    let q_case = ProviderQuery {
        query: "CYBERPUNK".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_case = provider.search(&q_case).expect("Search must succeed");
    assert_eq!(res_case.len(), 1);
    assert_eq!(res_case[0].id, "wallpaper-tokyo-nightfall");

    // Category filter "wallpaper"
    let q_cat = ProviderQuery {
        query: "".to_string(),
        category: Some("wallpaper".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_cat = provider.search(&q_cat).expect("Search must succeed");
    assert_eq!(res_cat.len(), 5);

    // Irrelevant category filter returns empty
    let q_other = ProviderQuery {
        query: "".to_string(),
        category: Some("window_manager".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_other = provider.search(&q_other).expect("Search must succeed");
    assert!(res_other.is_empty());
}

#[test]
fn test_wallpaper_valid_package_conversion_and_provenance() {
    let provider = WallpaperProvider::new();
    let item = provider
        .fetch_item("wallpaper-catppuccin-twilight")
        .expect("Must fetch twilight wallpaper");

    assert_eq!(item.id, "wallpaper-catppuccin-twilight");
    assert_eq!(item.package_type, PackageType::Wallpaper);
    assert_eq!(item.category, "wallpaper");
    assert_eq!(item.provenance.provider_id, "wallpaper");
    assert_eq!(item.provenance.license_spdx, Some("CC-BY-4.0".to_string()));
    assert!(item
        .provenance
        .source_url
        .starts_with("wallpaper://presets/"));

    assert_eq!(item.files.len(), 1);
    assert_eq!(item.files[0].source, "catppuccin-twilight.png");
    assert_eq!(
        item.files[0].target,
        "~/Pictures/Wallpapers/catppuccin-twilight.png"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 4 - 9: Target Confinement & Format Allowlisting
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallpaper_target_confinement_accepts_valid_paths() {
    let valid_targets = [
        "~/Pictures/Wallpapers/sunset.png",
        "~/Pictures/Wallpapers/4k/mountain.jpg",
        "~/Pictures/Wallpapers/nord/winter.webp",
        "~/Pictures/Wallpapers/pack/aurora.avif",
        "~/Pictures/Wallpapers/vector/minimal.svg",
    ];

    for tgt in &valid_targets {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "img.png".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(res.is_ok(), "Valid target must be accepted: {}", tgt);
    }
}

#[test]
fn test_wallpaper_allowed_image_extensions() {
    let allowed_exts = [".png", ".jpg", ".jpeg", ".webp", ".jxl", ".svg", ".avif"];

    for ext in &allowed_exts {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: format!("test{}", ext),
            target: format!("~/Pictures/Wallpapers/test{}", ext),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_ok(),
            "Allowed image extension '{}' must be accepted",
            ext
        );
    }
}

#[test]
fn test_wallpaper_rejects_target_outside_wallpaper_dir() {
    let bad_targets = [
        "~/.config/hypr/wallpaper.png",
        "~/.bashrc",
        "~/Downloads/wallpaper.png",
        "~/.local/share/wallpaper.png",
        "/usr/share/backgrounds/default.png",
    ];

    for tgt in &bad_targets {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "img.png".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_err(),
            "Target outside wallpaper directory must be rejected: {}",
            tgt
        );
        assert!(res.unwrap_err().contains(WALLPAPER_TARGET_PREFIX));
    }
}

#[test]
fn test_wallpaper_rejects_non_image_extensions() {
    let invalid_extensions = [
        "notes.txt",
        "metadata.json",
        "config.conf",
        "style.css",
        "theme.rasi",
        "palette.xml",
    ];

    for file_name in &invalid_extensions {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: file_name.to_string(),
            target: format!("~/Pictures/Wallpapers/{}", file_name),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_err(),
            "Non-image extension must be rejected: {}",
            file_name
        );
        assert!(res.unwrap_err().contains("allowed image extension"));
    }
}

#[test]
fn test_wallpaper_rejects_scripts_and_installer_hooks() {
    let script_files = [
        "setup.sh",
        "install.sh",
        "hook.py",
        "run.bash",
        "script.fish",
        "post_install.sh",
    ];

    for script in &script_files {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: script.to_string(),
            target: format!("~/Pictures/Wallpapers/{}", script),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(res.is_err(), "Script file must be rejected: {}", script);
    }
}

#[test]
fn test_wallpaper_rejects_binaries_and_executables() {
    let binary_files = [
        "downloader.exe",
        "payload.bin",
        "libhook.so",
        "agent.dll",
        "malware.elf",
    ];

    for bin in &binary_files {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: bin.to_string(),
            target: format!("~/Pictures/Wallpapers/{}", bin),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(res.is_err(), "Binary file must be rejected: {}", bin);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 10 - 15: Traversal, Collisions, SVG Security, Oversized Limits
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallpaper_rejects_target_traversal() {
    let traversal_targets = [
        "~/Pictures/Wallpapers/../hyprland.conf",
        "~/Pictures/Wallpapers/../../.bashrc",
        "~/Pictures/Wallpapers/pack/../../shadow",
    ];

    for tgt in &traversal_targets {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "bg.png".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(res.is_err(), "Target traversal must be rejected: {}", tgt);
    }
}

#[test]
fn test_wallpaper_rejects_absolute_target() {
    let mut seen = HashSet::new();
    let spec = ProviderFileSpec {
        source: "bg.png".to_string(),
        target: "/usr/share/backgrounds/bg.png".to_string(),
    };
    let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
    assert!(res.is_err(), "Absolute target must be rejected");
}

#[test]
fn test_wallpaper_rejects_source_traversal() {
    let traversal_sources = [
        "../bg.png",
        "../../etc/passwd",
        "/home/user/bg.png",
        "sub/../../../etc/shadow",
    ];

    for src in &traversal_sources {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: src.to_string(),
            target: "~/Pictures/Wallpapers/bg.png".to_string(),
        };
        let res = validate_wallpaper_target_and_source(&spec, "pkg", &mut seen);
        assert!(res.is_err(), "Source traversal must be rejected: {}", src);
    }
}

#[test]
fn test_wallpaper_duplicate_normalized_target_collision_rejected() {
    let mut seen = HashSet::new();

    let spec1 = ProviderFileSpec {
        source: "img1.png".to_string(),
        target: "~/Pictures/Wallpapers/background.png".to_string(),
    };
    assert!(validate_wallpaper_target_and_source(&spec1, "pkg", &mut seen).is_ok());

    // Duplicate exact target
    let spec2 = ProviderFileSpec {
        source: "img2.png".to_string(),
        target: "~/Pictures/Wallpapers/background.png".to_string(),
    };
    let res2 = validate_wallpaper_target_and_source(&spec2, "pkg", &mut seen);
    assert!(res2.is_err(), "Duplicate target must be rejected");
    assert!(res2
        .unwrap_err()
        .contains("Conflict violation: Duplicate target"));

    // Redundant slash normalized duplicate
    let spec3 = ProviderFileSpec {
        source: "img3.png".to_string(),
        target: "~/Pictures/Wallpapers//background.png".to_string(),
    };
    let res3 = validate_wallpaper_target_and_source(&spec3, "pkg", &mut seen);
    assert!(res3.is_err(), "Redundant slash collision must be rejected");

    // Redundant ./ normalized duplicate
    let spec4 = ProviderFileSpec {
        source: "img4.png".to_string(),
        target: "~/Pictures/Wallpapers/./background.png".to_string(),
    };
    let res4 = validate_wallpaper_target_and_source(&spec4, "pkg", &mut seen);
    assert!(res4.is_err(), "Redundant ./ collision must be rejected");
}

#[test]
fn test_wallpaper_svg_with_script_rejected() {
    let custom_root = TempTestDir::new("wp_svg_script");
    let pkg_dir = custom_root.path().join("evil-svg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let svg_content =
        r#"<svg xmlns="http://www.w3.org/2000/svg"><script>alert('pwn')</script></svg>"#;
    fs::write(pkg_dir.join("vector.svg"), svg_content).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "evil-svg",
        "name": "Evil SVG Wallpaper",
        "version": "1.0.0",
        "author": "Attacker",
        "package_type": "wallpaper",
        "description": "SVG script test",
        "tags": ["wallpaper"],
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "vector.svg", "target": "~/Pictures/Wallpapers/vector.svg", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let provider = WallpaperProvider::with_custom_dir(custom_root.path().to_path_buf());
    let item = provider.fetch_item("evil-svg").unwrap();

    let staging_root = TempTestDir::new("stage_svg");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "SVG with script tag must be rejected");
    assert!(res
        .unwrap_err()
        .contains("prohibited active script elements"));
}

#[test]
fn test_wallpaper_oversized_file_rejected() {
    let custom_root = TempTestDir::new("wp_oversized");
    let pkg_dir = custom_root.path().join("oversized-wp");
    fs::create_dir_all(&pkg_dir).unwrap();

    // Create a mock large file exceeding MAX_WALLPAPER_FILE_SIZE (50MB + 1MB)
    let large_file = pkg_dir.join("huge.png");
    let f = fs::File::create(&large_file).unwrap();
    f.set_len(MAX_WALLPAPER_FILE_SIZE + 1024 * 1024).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "oversized-wp",
        "name": "Oversized Wallpaper",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "wallpaper",
        "description": "Size limit test",
        "tags": ["wallpaper"],
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "huge.png", "target": "~/Pictures/Wallpapers/huge.png", "description": "huge" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let provider = WallpaperProvider::with_custom_dir(custom_root.path().to_path_buf());
    let item = provider.fetch_item("oversized-wp").unwrap();

    let staging_root = TempTestDir::new("stage_huge");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Oversized wallpaper file must be rejected");
    assert!(res.unwrap_err().contains("exceeds maximum allowed size"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 16 - 20: Identity, Hash, Symlinks, Rollback, and Multi-Pack
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallpaper_manifest_identity_mismatch_rejected() {
    let custom_root = TempTestDir::new("wp_id_mismatch");
    let pkg_dir = custom_root.path().join("dir-id");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "manifest-id",
        "name": "Mismatch WP",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "wallpaper",
        "description": "Mismatch test",
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

    let res = load_disk_wallpaper_item(&pkg_dir);
    assert!(
        res.is_err(),
        "Directory vs manifest ID mismatch must be rejected"
    );
    assert!(res.unwrap_err().contains("Identity mismatch"));
}

#[test]
fn test_wallpaper_content_tree_hash_mismatch_rejected() {
    let custom_root = TempTestDir::new("wp_tampered");
    let pkg_dir = custom_root.path().join("tampered-wp");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "tampered-wp",
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

    let provider = WallpaperProvider::with_custom_dir(custom_root.path().to_path_buf());
    let mut item = provider.fetch_item("tampered-wp").expect("Must fetch item");

    // Alter expected tree hash in provenance
    item.provenance.commit_or_tag =
        Some("2222222222222222222222222222222222222222222222222222222222222222".to_string());

    let staging_root = TempTestDir::new("stage_hash_mismatch");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Tree hash mismatch must fail staging");
    assert!(res.unwrap_err().contains("Tree hash mismatch"));
}

#[test]
fn test_wallpaper_rejects_symlink_escape() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let custom_root = TempTestDir::new("wp_symlink");
        let pkg_dir = custom_root.path().join("evil-symlink");
        fs::create_dir_all(&pkg_dir).unwrap();

        let secret_file = custom_root.path().join("secret.png");
        fs::write(&secret_file, "SENSITIVE_DATA").unwrap();

        let sym_file = pkg_dir.join("bg.png");
        symlink(&secret_file, &sym_file).unwrap();

        let manifest = serde_json::json!({
            "ryzora_spec": "1",
            "id": "evil-symlink",
            "name": "Symlink Wallpaper",
            "version": "1.0.0",
            "author": "Attacker",
            "package_type": "wallpaper",
            "description": "Symlink test",
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

        let provider = WallpaperProvider::with_custom_dir(custom_root.path().to_path_buf());
        let item = provider.fetch_item("evil-symlink").unwrap();

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

#[test]
fn test_wallpaper_transactional_staging_atomic_purge_on_failure() {
    let custom_root = TempTestDir::new("wp_tx_fail");
    let pkg_dir = custom_root.path().join("partial-fail-wp");
    fs::create_dir_all(&pkg_dir).unwrap();

    // Create both files initially
    fs::write(pkg_dir.join("img1.png"), b"PNG_DATA_1").unwrap();
    fs::write(pkg_dir.join("img2.png"), b"PNG_DATA_2").unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "partial-fail-wp",
        "name": "Partial Fail Pack",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "wallpaper",
        "description": "Tx test",
        "tags": ["wallpaper"],
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "img1.png", "target": "~/Pictures/Wallpapers/img1.png", "description": "file 1" },
            { "source": "img2.png", "target": "~/Pictures/Wallpapers/img2.png", "description": "file 2" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let provider = WallpaperProvider::with_custom_dir(custom_root.path().to_path_buf());
    let item = provider.fetch_item("partial-fail-wp").unwrap();

    // Delete img2.png right before staging to simulate payload corruption
    fs::remove_file(pkg_dir.join("img2.png")).unwrap();

    let staging_root = TempTestDir::new("tx_stage_root");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Missing file must cause staging failure");

    // CRITICAL: Verify NO leftover temporary directories or files in staging_root
    let entries: Vec<_> = fs::read_dir(staging_root.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "Transactional staging must purge all temporary files on failure! Found: {:?}",
        entries
    );
}

#[test]
fn test_wallpaper_successful_multi_pack_staging_and_manifest_synthesis() {
    let provider = WallpaperProvider::new();
    let item = provider.fetch_item("wallpaper-studio-gradients").unwrap();

    let staging_root = TempTestDir::new("stage_gradients");
    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Multi-wallpaper pack staging must succeed");

    assert!(staged_dir.exists());
    assert!(staged_dir.join("gradient-aurora.png").is_file());
    assert!(staged_dir.join("gradient-dusk.png").is_file());

    // Verify synthesized manifest
    let manifest_path = staged_dir.join("manifest.json");
    assert!(manifest_path.is_file());
    let manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.id, "wallpaper-studio-gradients");
    assert_eq!(manifest.package_type, PackageType::Wallpaper);
    assert_eq!(manifest.files.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 21 - 25: Trust, Signatures, Aggregator Resilience & Security Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_wallpaper_unsigned_content_remains_unverified() {
    let provider = WallpaperProvider::new();
    let item = provider.fetch_item("wallpaper-nord-lake").unwrap();

    let normalized = normalize_provider_item(item);
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Verified));
    assert_eq!(normalized.safety_audit.rating, "unverified");
    assert!(normalized.tags.contains(&"provider:wallpaper".to_string()));
}

#[test]
fn test_wallpaper_revoked_key_signature_rejected() {
    let mut rng = rand_core::OsRng;
    let revoked_key = SigningKey::generate(&mut rng);
    let revoked_pubkey_hex = hex::encode(revoked_key.verifying_key().to_bytes());

    let identity = SignerIdentity {
        author_id: "revoked-author".to_string(),
        name: "Revoked signer".to_string(),
        handle: None,
    };
    let tree_hash = "abc123wphash";
    let sig_meta =
        sign_package_tree_hash(&revoked_key, "wallpaper-nord-lake", tree_hash, identity).unwrap();

    let revoked_entry = RevokedKeyEntry {
        key_id: "revoked-wp".to_string(),
        public_key: revoked_pubkey_hex,
        revoked_at: "2026-01-01T00:00:00Z".to_string(),
        reason: "Compromised".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    let eval = evaluate_trust_chain(
        Some(&sig_meta),
        "wallpaper-nord-lake",
        tree_hash,
        Some(TrustTier::Community),
        &trust_store,
    );

    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.is_valid);
}

#[test]
fn test_wallpaper_provider_failure_isolation_in_aggregator() {
    let mut manager = ProviderManager::new();

    let bad_provider = WallpaperProvider::with_custom_dir(PathBuf::from("/nonexistent/wp/dir"));
    manager.register_provider(Arc::new(bad_provider));

    let good_provider = WallpaperProvider::new();
    manager.register_provider(Arc::new(good_provider));

    let query = ProviderQuery {
        query: "twilight".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let response = manager.search_all(&query);
    assert!(
        !response.items.is_empty(),
        "Aggregator must succeed despite faulty provider"
    );
    assert!(response
        .items
        .iter()
        .any(|i| i.id == "wallpaper-catppuccin-twilight"));
}

#[test]
fn test_wallpaper_offline_behavior_serves_presets() {
    let provider = WallpaperProvider::new();
    let query = ProviderQuery {
        query: "eclipse".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let items = provider
        .search(&query)
        .expect("Offline search must succeed");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "wallpaper-solar-eclipse");
}

#[test]
fn test_wallpaper_provider_zero_subprocess_and_privilege_audit() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wp_source_path = manifest_dir
        .join("src")
        .join("providers")
        .join("wallpaper.rs");

    let content = fs::read_to_string(&wp_source_path)
        .expect("Must be able to read src/providers/wallpaper.rs for security audit");

    // Strictly verify zero subprocess invocation in wallpaper provider
    assert!(
        !content.contains("Command::new("),
        "Security audit failed: Command::new( found in wallpaper.rs"
    );
    assert!(
        !content.contains("std::process::Command"),
        "Security audit failed: std::process::Command found in wallpaper.rs"
    );
    assert!(
        !content.contains("exec("),
        "Security audit failed: exec call found in wallpaper.rs"
    );

    // Verify no privilege escalation keywords outside comments
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        assert!(
            !trimmed.contains("sudo"),
            "Security audit failed: sudo found in line: {}",
            line
        );
        assert!(
            !trimmed.contains("pkexec"),
            "Security audit failed: pkexec found in line: {}",
            line
        );
    }
}
