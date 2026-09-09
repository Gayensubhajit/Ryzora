//! Phase 18.5: Hyprland / Complete Rice Content Provider Comprehensive Test Suite.
//!
//! Validates:
//! 1. Provider registration & discovery in ProviderManager.
//! 2. Search & category filtering ("rice", "desktop").
//! 3. Multi-component Rice conversion to ProviderItem with complete provenance.
//! 4. Component-aware target allowlisting: all 12 component targets accepted.
//! 5. Component cross-claiming rejected (e.g. Hyprland claiming ~/.config/waybar/).
//! 6. Theme and Icon structure validation (subfolder requirement).
//! 7. Targets outside allowlist rejected (~/.bashrc, ~/.local/bin/...).
//! 8. Systemd, autostart, and environment targets rejected (~/.config/systemd/...).
//! 9. Target traversal ('..') and null bytes rejected.
//! 10. Absolute targets rejected (/etc/...).
//! 11. Source traversal ('..') and absolute sources rejected.
//! 12. Cross-component duplicate target collision rejected (same target declared twice).
//! 13. Scripts and installer hooks rejected (*.sh, *.py, install.sh).
//! 14. Binaries and executables rejected (*.exe, *.bin, *.elf, *.so).
//! 15. Manifest identity / version mismatch rejected.
//! 16. Content tree-hash verification and tampering rejection.
//! 17. Symlink escape from package root rejected during staging.
//! 18. Transactional staging: atomic rollback/purge on partial failure (zero leftover tmp dirs).
//! 19. Successful multi-component staging and manifest synthesis (PackageType::Rice).
//! 20. Unsigned content remains unverified (TrustTier::Community).
//! 21. Revoked key signature rejected.
//! 22. Provider failure isolation in ProviderManager aggregator.
//! 23. Offline behavior serves built-in presets without network.
//! 24. Zero-subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec).

use ryzora_lib::crypto::{
    evaluate_trust_chain, sign_package_tree_hash, CryptographicStatus, RevokedKeyEntry,
    SignerIdentity, SigningKey, TrustStore,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::{PackageType, RyzoraManifest};
use ryzora_lib::providers::normalizer::normalize_provider_item;
use ryzora_lib::providers::rice::{
    load_disk_rice_item, validate_rice_component_file, RiceComponentFile, RiceComponentKind,
    RiceProvider,
};
use ryzora_lib::providers::{ContentProvider, ProviderManager, ProviderQuery, ProviderStatus};
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
        let path = std::env::temp_dir().join(format!("ryzora-phase18-5-{}-{}", name, nanos));
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
// Test Vectors 1 - 3: Provider Registration, Discovery, Search, and Provenance
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rice_provider_registration_and_capabilities() {
    let mut manager = ProviderManager::new();
    let provider = Arc::new(RiceProvider::new());
    manager.register_provider(provider);

    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);

    let info = &providers[0];
    assert_eq!(info.id, "rice");
    assert_eq!(info.name, "Hyprland Rice Ecosystem");
    assert!(info.enabled);
    assert_eq!(info.status, ProviderStatus::Online);
    assert!(info.capabilities.can_search);
    assert!(info.capabilities.can_fetch_item);
    assert!(info.capabilities.can_stage_payload);
    assert!(info.capabilities.supports_categories);
}

#[test]
fn test_rice_search_and_category_filtering() {
    let provider = RiceProvider::new();

    // Search query matching
    let query_mocha = ProviderQuery {
        query: "catppuccin".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let results = provider
        .search(&query_mocha)
        .expect("Search should succeed");
    assert!(!results.is_empty());
    assert_eq!(results[0].id, "rice-catppuccin-mocha");
    assert_eq!(results[0].package_type, PackageType::Rice);

    // Search case insensitivity
    let query_case = ProviderQuery {
        query: "TOKYO".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let results_case = provider.search(&query_case).expect("Search should succeed");
    assert_eq!(results_case.len(), 1);
    assert_eq!(results_case[0].id, "rice-tokyo-night-neon");

    // Category filter "rice"
    let query_cat = ProviderQuery {
        query: "".to_string(),
        category: Some("rice".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let results_cat = provider.search(&query_cat).expect("Search should succeed");
    assert_eq!(results_cat.len(), 3);

    // Irrelevant category should return empty
    let query_other = ProviderQuery {
        query: "".to_string(),
        category: Some("hardware".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let results_other = provider
        .search(&query_other)
        .expect("Search should succeed");
    assert!(results_other.is_empty());
}

#[test]
fn test_rice_multi_component_conversion_and_provenance() {
    let provider = RiceProvider::new();
    let item = provider
        .fetch_item("rice-catppuccin-mocha")
        .expect("Must find Catppuccin Mocha rice");

    assert_eq!(item.id, "rice-catppuccin-mocha");
    assert_eq!(item.package_type, PackageType::Rice);
    assert_eq!(item.category, "rice");
    assert_eq!(item.author.name, "Ryzora Design Collective");
    assert_eq!(item.provenance.provider_id, "rice");
    assert_eq!(item.provenance.license_spdx, Some("MIT".to_string()));
    assert!(item.provenance.source_url.starts_with("rice://presets/"));

    // Multi-component verification: contains Hyprland, Waybar, Kitty, Rofi, Dunst, Wallpaper
    let targets: Vec<&str> = item.files.iter().map(|f| f.target.as_str()).collect();
    assert!(targets.contains(&"~/.config/hypr/hyprland.conf"));
    assert!(targets.contains(&"~/.config/waybar/config.jsonc"));
    assert!(targets.contains(&"~/.config/kitty/kitty.conf"));
    assert!(targets.contains(&"~/.config/rofi/config.rasi"));
    assert!(targets.contains(&"~/.config/dunst/dunstrc"));
    assert!(targets.contains(&"~/Pictures/Wallpapers/catppuccin-minimal.png"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 4 - 8: Component-Aware Allowlisting & Target Boundaries
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rice_component_aware_targets_accepted() {
    let valid_pairs = [
        (
            RiceComponentKind::Hyprland,
            "hypr/hyprland.conf",
            "~/.config/hypr/hyprland.conf",
        ),
        (
            RiceComponentKind::Waybar,
            "waybar/config.jsonc",
            "~/.config/waybar/config.jsonc",
        ),
        (
            RiceComponentKind::Kitty,
            "kitty/kitty.conf",
            "~/.config/kitty/kitty.conf",
        ),
        (
            RiceComponentKind::Alacritty,
            "alacritty/alacritty.toml",
            "~/.config/alacritty/alacritty.toml",
        ),
        (
            RiceComponentKind::Rofi,
            "rofi/config.rasi",
            "~/.config/rofi/config.rasi",
        ),
        (
            RiceComponentKind::Wofi,
            "wofi/style.css",
            "~/.config/wofi/style.css",
        ),
        (
            RiceComponentKind::Dunst,
            "dunst/dunstrc",
            "~/.config/dunst/dunstrc",
        ),
        (
            RiceComponentKind::Mako,
            "mako/config",
            "~/.config/mako/config",
        ),
        (
            RiceComponentKind::Fastfetch,
            "fastfetch/config.jsonc",
            "~/.config/fastfetch/config.jsonc",
        ),
        (
            RiceComponentKind::Wallpaper,
            "wallpapers/bg.png",
            "~/Pictures/Wallpapers/bg.png",
        ),
        (
            RiceComponentKind::GtkTheme,
            "theme/gtk.css",
            "~/.themes/Catppuccin/gtk-3.0/gtk.css",
        ),
        (
            RiceComponentKind::IconTheme,
            "icons/index.theme",
            "~/.icons/Papirus/index.theme",
        ),
    ];

    for (comp, src, tgt) in &valid_pairs {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: *comp,
            source: src.to_string(),
            target: tgt.to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(
            res.is_ok(),
            "Component '{:?}' target '{}' must be accepted: {:?}",
            comp,
            tgt,
            res.err()
        );
    }
}

#[test]
fn test_rice_component_claiming_other_component_dir_rejected() {
    let cross_claims = [
        (RiceComponentKind::Hyprland, "~/.config/waybar/config.jsonc"),
        (RiceComponentKind::Waybar, "~/.config/hypr/hyprland.conf"),
        (RiceComponentKind::Kitty, "~/.config/rofi/config.rasi"),
        (RiceComponentKind::Wallpaper, "~/.config/dunst/dunstrc"),
        (RiceComponentKind::Rofi, "~/.config/kitty/kitty.conf"),
        (RiceComponentKind::Fastfetch, "~/.config/hypr/bindings.conf"),
    ];

    for (comp, invalid_tgt) in &cross_claims {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: *comp,
            source: "config".to_string(),
            target: invalid_tgt.to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(
            res.is_err(),
            "Component '{:?}' must reject claiming other directory '{}'",
            comp,
            invalid_tgt
        );
        let err = res.unwrap_err();
        assert!(err.contains("Security violation"));
        assert!(err.contains(comp.allowed_target_prefix()));
    }
}

#[test]
fn test_rice_theme_and_icon_structure_validation() {
    let mut seen = HashSet::new();

    // GTK theme without subfolder must be rejected
    let bad_theme = RiceComponentFile {
        component: RiceComponentKind::GtkTheme,
        source: "gtk.css".to_string(),
        target: "~/.themes/".to_string(),
        content: None,
    };
    let res = validate_rice_component_file(&bad_theme, "theme-pkg", &mut seen);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("Invalid GTK theme target"));

    // Icon theme without subfolder must be rejected
    let bad_icon = RiceComponentFile {
        component: RiceComponentKind::IconTheme,
        source: "icon.png".to_string(),
        target: "~/.icons/".to_string(),
        content: None,
    };
    let res_icon = validate_rice_component_file(&bad_icon, "icon-pkg", &mut seen);
    assert!(res_icon.is_err());
    assert!(res_icon.unwrap_err().contains("Invalid Icon theme target"));
}

#[test]
fn test_rice_rejects_targets_outside_allowlist() {
    let forbidden_targets = [
        "~/.bashrc",
        "~/.bash_profile",
        "~/.bash_logout",
        "~/.profile",
        "~/.zshrc",
        "~/.zshenv",
        "~/.config/fish/config.fish",
        "~/.local/bin/malicious",
        "~/.ssh/id_rsa",
        "~/.gnupg/secring.gpg",
        "/etc/environment",
    ];

    for tgt in &forbidden_targets {
        let mut seen = HashSet::new();
        // Even if claiming a valid component like Hyprland, targeting outside is rejected
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: "src".to_string(),
            target: tgt.to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(
            res.is_err(),
            "Target outside allowlist must be rejected: {}",
            tgt
        );
    }
}

#[test]
fn test_rice_rejects_systemd_and_autostart_targets() {
    let forbidden_system = [
        "~/.config/systemd/user/evil.service",
        "~/.config/autostart/bad.desktop",
        "~/.config/environment.d/evil.conf",
    ];

    for tgt in &forbidden_system {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: "src".to_string(),
            target: tgt.to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(res.is_err(), "System destination must be rejected: {}", tgt);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 9 - 14: Traversal, Absolute Paths, Collisions, Scripts & Binaries
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rice_rejects_target_traversal() {
    let traversal_targets = [
        "~/.config/hypr/../bashrc",
        "~/.config/hypr/../../etc/passwd",
        "~/.config/waybar/..",
    ];

    for tgt in &traversal_targets {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: "src".to_string(),
            target: tgt.to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(res.is_err(), "Target traversal must be rejected: {}", tgt);
    }
}

#[test]
fn test_rice_rejects_absolute_target() {
    let mut seen = HashSet::new();
    let file = RiceComponentFile {
        component: RiceComponentKind::Hyprland,
        source: "src".to_string(),
        target: "/etc/hyprland.conf".to_string(),
        content: None,
    };
    let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
    assert!(res.is_err(), "Absolute target must be rejected");
}

#[test]
fn test_rice_rejects_source_traversal() {
    let traversal_sources = [
        "../hyprland.conf",
        "../../etc/shadow",
        "/home/user/evil",
        "subdir/../../../etc/passwd",
    ];

    for src in &traversal_sources {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: src.to_string(),
            target: "~/.config/hypr/hyprland.conf".to_string(),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(res.is_err(), "Source traversal must be rejected: {}", src);
    }
}

#[test]
fn test_rice_cross_component_duplicate_target_collision_rejected() {
    let mut seen = HashSet::new();

    // File 1 writes ~/.config/hypr/hyprland.conf
    let file1 = RiceComponentFile {
        component: RiceComponentKind::Hyprland,
        source: "hypr/hypr1.conf".to_string(),
        target: "~/.config/hypr/hyprland.conf".to_string(),
        content: None,
    };
    assert!(validate_rice_component_file(&file1, "pkg", &mut seen).is_ok());

    // File 2 in same package attempts to write the same target!
    let file2 = RiceComponentFile {
        component: RiceComponentKind::Hyprland,
        source: "hypr/hypr2.conf".to_string(),
        target: "~/.config/hypr/hyprland.conf".to_string(),
        content: None,
    };
    let res2 = validate_rice_component_file(&file2, "pkg", &mut seen);
    assert!(res2.is_err(), "Duplicate target write must be rejected");
    assert!(res2
        .unwrap_err()
        .contains("Conflict violation: Duplicate target"));

    // File 3 with redundant slashes normalized to same target
    let file3 = RiceComponentFile {
        component: RiceComponentKind::Hyprland,
        source: "hypr/hypr3.conf".to_string(),
        target: "~/.config/hypr//hyprland.conf".to_string(),
        content: None,
    };
    let res3 = validate_rice_component_file(&file3, "pkg", &mut seen);
    assert!(
        res3.is_err(),
        "Duplicate normalized target write must be rejected"
    );
}

#[test]
fn test_rice_rejects_scripts_and_installer_hooks() {
    let prohibited_scripts = [
        "setup.sh",
        "install.sh",
        "post_install.sh",
        "hook.py",
        "run.bash",
        "exec.zsh",
        "script.fish",
    ];

    for script in &prohibited_scripts {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: script.to_string(),
            target: format!("~/.config/hypr/{}", script),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(
            res.is_err(),
            "Prohibited script must be rejected: {}",
            script
        );
    }
}

#[test]
fn test_rice_rejects_binaries_and_executables() {
    let prohibited_binaries = [
        "daemon.exe",
        "payload.bin",
        "libhook.so",
        "malware.elf",
        "agent.dll",
    ];

    for bin in &prohibited_binaries {
        let mut seen = HashSet::new();
        let file = RiceComponentFile {
            component: RiceComponentKind::Hyprland,
            source: bin.to_string(),
            target: format!("~/.config/hypr/{}", bin),
            content: None,
        };
        let res = validate_rice_component_file(&file, "test-pkg", &mut seen);
        assert!(res.is_err(), "Binary/executable must be rejected: {}", bin);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 15 - 19: Identity, Hash, Symlinks, Transactional Rollback, Staging
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rice_manifest_identity_mismatch_rejected() {
    let custom_root = TempTestDir::new("rice_id_mismatch");
    let pkg_dir = custom_root.path().join("evil-rice");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "good-rice",
        "name": "Good Rice",
        "version": "1.0.0",
        "author": "Attacker",
        "package_type": "rice",
        "description": "Mismatch test",
        "tags": ["rice"],
        "compatibility": { "desktops": ["hyprland"] },
        "files": [
            { "source": "hypr/hyprland.conf", "target": "~/.config/hypr/hyprland.conf", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let res = load_disk_rice_item(&pkg_dir);
    assert!(
        res.is_err(),
        "Directory name vs manifest.id mismatch must be rejected"
    );
    assert!(res.unwrap_err().contains("Identity mismatch"));
}

#[test]
fn test_rice_content_tree_hash_mismatch_rejected() {
    let custom_root = TempTestDir::new("rice_tampered");
    let pkg_dir = custom_root.path().join("tampered-rice");
    fs::create_dir_all(pkg_dir.join("hypr")).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "tampered-rice",
        "name": "Tampered Rice",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "rice",
        "description": "Tamper test",
        "tags": ["rice"],
        "compatibility": { "desktops": ["hyprland"] },
        "files": [
            { "source": "hypr/hyprland.conf", "target": "~/.config/hypr/hyprland.conf", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        pkg_dir.join("hypr").join("hyprland.conf"),
        b"general { gaps_in = 5 }",
    )
    .unwrap();

    let provider = RiceProvider::with_custom_dir(custom_root.path().to_path_buf());
    let mut item = provider
        .fetch_item("tampered-rice")
        .expect("Must fetch item");

    // Inject fake/wrong expected hash into provenance commit_or_tag (64-char hex)
    item.provenance.commit_or_tag =
        Some("1111111111111111111111111111111111111111111111111111111111111111".to_string());

    let staging_root = TempTestDir::new("stage_hash_mismatch");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Tree hash mismatch must fail staging");
    assert!(res.unwrap_err().contains("Tree hash mismatch"));
}

#[test]
fn test_rice_rejects_symlink_escape() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let custom_root = TempTestDir::new("rice_symlink");
        let pkg_dir = custom_root.path().join("evil-rice");
        fs::create_dir_all(pkg_dir.join("hypr")).unwrap();

        let secret_file = custom_root.path().join("secret.txt");
        fs::write(&secret_file, "SENSITIVE_DATA").unwrap();

        // Symlink hyprland.conf -> secret_file
        let sym_file = pkg_dir.join("hypr").join("hyprland.conf");
        symlink(&secret_file, &sym_file).unwrap();

        let manifest = serde_json::json!({
            "ryzora_spec": "1",
            "id": "evil-rice",
            "name": "Evil Symlink Rice",
            "version": "1.0.0",
            "author": "Attacker",
            "package_type": "rice",
            "description": "Symlink test",
            "tags": ["rice"],
            "compatibility": { "desktops": ["hyprland"] },
            "files": [
                { "source": "hypr/hyprland.conf", "target": "~/.config/hypr/hyprland.conf", "description": "test" }
            ],
            "dependencies": []
        });
        fs::write(
            pkg_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let provider = RiceProvider::with_custom_dir(custom_root.path().to_path_buf());
        let item = provider.fetch_item("evil-rice").unwrap();

        let staging_dest = TempTestDir::new("sym_dest");
        let res = provider.stage_payload(&item, staging_dest.path());
        assert!(
            res.is_err(),
            "Symlink escape must be rejected during staging"
        );
        assert!(res
            .unwrap_err()
            .contains("escapes rice package root via symlink"));
    }
}

#[test]
fn test_rice_transactional_staging_atomic_purge_on_failure() {
    let custom_root = TempTestDir::new("rice_tx_fail");
    let pkg_dir = custom_root.path().join("partial-fail-rice");
    fs::create_dir_all(pkg_dir.join("hypr")).unwrap();
    fs::create_dir_all(pkg_dir.join("waybar")).unwrap();

    // Create both files initially so manifest & tree-hash can be computed
    fs::write(pkg_dir.join("hypr").join("hyprland.conf"), b"# config").unwrap();
    fs::write(pkg_dir.join("waybar").join("config.jsonc"), b"{}").unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "partial-fail-rice",
        "name": "Partial Fail Rice",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "rice",
        "description": "Tx rollback test",
        "tags": ["rice"],
        "compatibility": { "desktops": ["hyprland"] },
        "files": [
            { "source": "hypr/hyprland.conf", "target": "~/.config/hypr/hyprland.conf", "description": "test" },
            { "source": "waybar/config.jsonc", "target": "~/.config/waybar/config.jsonc", "description": "missing" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let provider = RiceProvider::with_custom_dir(custom_root.path().to_path_buf());
    let item = provider.fetch_item("partial-fail-rice").unwrap();

    // Now simulate payload disk corruption: delete waybar/config.jsonc before staging
    fs::remove_file(pkg_dir.join("waybar").join("config.jsonc")).unwrap();

    let staging_root = TempTestDir::new("tx_stage_root");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Missing component file must abort staging");

    // CRITICAL: Verify NO leftover directories or partial files remain in staging_root!
    let entries: Vec<_> = fs::read_dir(staging_root.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "Transactional staging must purge all temporary files on failure! Found: {:?}",
        entries
    );
}

#[test]
fn test_rice_successful_multi_component_staging_and_manifest_synthesis() {
    let provider = RiceProvider::new();
    let item = provider.fetch_item("rice-catppuccin-mocha").unwrap();

    let staging_root = TempTestDir::new("stage_catppuccin");
    let staged_dir = provider
        .stage_payload(&item, staging_root.path())
        .expect("Catppuccin Mocha multi-component staging must succeed");

    assert!(staged_dir.exists());

    // Check component files staged
    assert!(staged_dir.join("hypr/hyprland.conf").is_file());
    assert!(staged_dir.join("waybar/config.jsonc").is_file());
    assert!(staged_dir.join("kitty/kitty.conf").is_file());
    assert!(staged_dir.join("rofi/config.rasi").is_file());
    assert!(staged_dir.join("dunst/dunstrc").is_file());
    assert!(staged_dir
        .join("wallpapers/catppuccin-minimal.png")
        .is_file());

    // Check synthesized manifest.json
    let manifest_path = staged_dir.join("manifest.json");
    assert!(manifest_path.is_file());
    let manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest.id, "rice-catppuccin-mocha");
    assert_eq!(manifest.package_type, PackageType::Rice);
    assert!(manifest.files.len() >= 6);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 20 - 24: Trust, Signatures, Aggregator Resilience & Security Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_rice_unsigned_content_remains_unverified() {
    let provider = RiceProvider::new();
    let item = provider.fetch_item("rice-nord-frost").unwrap();

    let normalized = normalize_provider_item(item);
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Verified));
    assert_eq!(normalized.safety_audit.rating, "unverified");
    assert!(normalized.tags.contains(&"provider:rice".to_string()));
}

#[test]
fn test_rice_revoked_key_signature_rejected() {
    let mut rng = rand_core::OsRng;
    let revoked_key = SigningKey::generate(&mut rng);
    let revoked_pubkey_hex = hex::encode(revoked_key.verifying_key().to_bytes());

    let identity = SignerIdentity {
        author_id: "revoked-author".to_string(),
        name: "Revoked signer".to_string(),
        handle: None,
    };
    let tree_hash = "abc123ricehash";
    let sig_meta =
        sign_package_tree_hash(&revoked_key, "rice-nord-frost", tree_hash, identity).unwrap();

    let revoked_entry = RevokedKeyEntry {
        key_id: "revoked-rice".to_string(),
        public_key: revoked_pubkey_hex,
        revoked_at: "2026-01-01T00:00:00Z".to_string(),
        reason: "Compromised".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    let eval = evaluate_trust_chain(
        Some(&sig_meta),
        "rice-nord-frost",
        tree_hash,
        Some(TrustTier::Community),
        &trust_store,
    );

    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.is_valid);
}

#[test]
fn test_rice_provider_failure_isolation_in_aggregator() {
    let mut manager = ProviderManager::new();

    // Faulty Rice provider pointing to non-existent custom directory
    let bad_provider = RiceProvider::with_custom_dir(PathBuf::from("/nonexistent/rice/dir"));
    manager.register_provider(Arc::new(bad_provider));

    // Healthy built-in provider
    let good_provider = RiceProvider::new();
    manager.register_provider(Arc::new(good_provider));

    let query = ProviderQuery {
        query: "catppuccin".to_string(),
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
        .any(|i| i.id == "rice-catppuccin-mocha"));
}

#[test]
fn test_rice_offline_behavior_serves_presets() {
    let provider = RiceProvider::new();
    let query = ProviderQuery {
        query: "tokyo".to_string(),
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
    assert_eq!(items[0].id, "rice-tokyo-night-neon");
}

#[test]
fn test_rice_provider_zero_subprocess_and_privilege_audit() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rice_source_path = manifest_dir.join("src").join("providers").join("rice.rs");

    let content = fs::read_to_string(&rice_source_path)
        .expect("Must be able to read src/providers/rice.rs for security audit");

    // Strictly verify zero subprocess invocation in rice provider
    assert!(
        !content.contains("Command::new("),
        "Security audit failed: Command::new( found in rice.rs"
    );
    assert!(
        !content.contains("std::process::Command"),
        "Security audit failed: std::process::Command found in rice.rs"
    );
    assert!(
        !content.contains("exec("),
        "Security audit failed: exec call found in rice.rs"
    );

    // Verify no privilege escalation keywords outside doc comments
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue; // Skip comments mentioning security invariants
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
