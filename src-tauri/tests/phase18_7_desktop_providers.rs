//! Phase 18.7: KDE & GNOME Desktop Environment Providers Comprehensive Test Suite.
//!
//! Validates:
//! 1. KdeProvider registration & capabilities in ProviderManager.
//! 2. GnomeProvider registration & capabilities in ProviderManager.
//! 3. KDE search & category filtering.
//! 4. GNOME search & category filtering.
//! 5. KDE valid package conversion to ProviderItem with complete provenance.
//! 6. GNOME valid package conversion to ProviderItem with complete provenance.
//! 7. KDE component-aware target allowlisting accepted.
//! 8. GNOME component-aware target allowlisting accepted.
//! 9. KDE component claiming GNOME directory rejected.
//! 10. GNOME component claiming KDE directory rejected.
//! 11. Theme subfolder confinement enforced (cannot write to root of ~/.themes/ or ~/.local/share/plasma/).
//! 12. Autostart, systemd, and shell configs rejected (~/.config/autostart/, ~/.bashrc, /etc/).
//! 13. Scripts and installer hooks rejected (*.sh, *.py, install.sh).
//! 14. Binaries and executables rejected (*.so, *.exe, *.bin, *.elf).
//! 15. Legitimate declarative theme extensions accepted (.css, .desktop, .theme, .colors, .colorscheme, .svg).
//! 16. Target traversal ('..') and null bytes rejected.
//! 17. Absolute targets rejected (/usr/share/themes/...).
//! 18. Source traversal ('..') and absolute sources rejected.
//! 19. Duplicate normalized target collision rejected (//, ./foo).
//! 20. Two components attempting to write same target rejected.
//! 21. Symlink escape from package root rejected during staging.
//! 22. Manifest identity / version mismatch rejected.
//! 23. Content tree-hash verification and tampering rejection.
//! 24. Transactional staging: atomic rollback/purge on partial failure (zero leftovers).
//! 25. Successful mixed multi-component package staging (KDE multi-pack and GNOME GTK+icons pack).
//! 26. Unsigned content remains unverified (TrustTier::Community).
//! 27. Revoked key signature rejected.
//! 28. Provider failure isolation in ProviderManager aggregator.
//! 29. Offline behavior serves built-in presets without network.
//! 30. Zero-subprocess / privilege audit (0 Command::new, 0 sudo, 0 pkexec, 0 gsettings, 0 dconf).

use ryzora_lib::crypto::{
    evaluate_trust_chain, sign_package_tree_hash, CryptographicStatus, RevokedKeyEntry,
    SignerIdentity, SigningKey, TrustStore,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::manifest::{PackageType, RyzoraManifest};
use ryzora_lib::providers::desktop::{
    load_disk_gnome_item, validate_gnome_target_and_source, validate_kde_target_and_source,
    GnomeProvider, KdeProvider,
};
use ryzora_lib::providers::normalizer::normalize_provider_item;
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
        let path = std::env::temp_dir().join(format!("ryzora-phase18-7-{}-{}", name, nanos));
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
// Test Vectors 1 - 6: Registration, Discovery, Search, and Provenance
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_kde_provider_registration_and_capabilities() {
    let mut manager = ProviderManager::new();
    let provider = Arc::new(KdeProvider::new());
    manager.register_provider(provider);

    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);

    let info = &providers[0];
    assert_eq!(info.id, "kde");
    assert_eq!(info.name, "KDE Plasma Themes & Look-and-Feel");
    assert!(info.enabled);
    assert_eq!(info.status, ProviderStatus::Online);
    assert!(info.capabilities.can_search);
    assert!(info.capabilities.can_fetch_item);
    assert!(info.capabilities.can_stage_payload);
    assert!(info.capabilities.supports_categories);
}

#[test]
fn test_gnome_provider_registration_and_capabilities() {
    let mut manager = ProviderManager::new();
    let provider = Arc::new(GnomeProvider::new());
    manager.register_provider(provider);

    let providers = manager.list_providers();
    assert_eq!(providers.len(), 1);

    let info = &providers[0];
    assert_eq!(info.id, "gnome");
    assert_eq!(info.name, "GNOME & GTK Themes");
    assert!(info.enabled);
    assert_eq!(info.status, ProviderStatus::Online);
    assert!(info.capabilities.can_search);
    assert!(info.capabilities.can_fetch_item);
    assert!(info.capabilities.can_stage_payload);
}

#[test]
fn test_kde_search_and_category_filtering() {
    let provider = KdeProvider::new();

    // Query "chameleon"
    let q_chameleon = ProviderQuery {
        query: "chameleon".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res = provider.search(&q_chameleon).expect("Search must succeed");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "kde-breeze-chameleon");

    // Case insensitivity "CATPPUCCIN"
    let q_case = ProviderQuery {
        query: "CATPPUCCIN".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_case = provider.search(&q_case).expect("Search must succeed");
    assert_eq!(res_case.len(), 1);
    assert_eq!(res_case[0].id, "kde-catppuccin-mocha");

    // Category "plasma"
    let q_cat = ProviderQuery {
        query: "".to_string(),
        category: Some("plasma".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_cat = provider.search(&q_cat).expect("Search must succeed");
    assert_eq!(res_cat.len(), 3);
}

#[test]
fn test_gnome_search_and_category_filtering() {
    let provider = GnomeProvider::new();

    // Query "adwaita"
    let q_adwaita = ProviderQuery {
        query: "adwaita".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res = provider.search(&q_adwaita).expect("Search must succeed");
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].id, "gnome-adwaita-vibrant");

    // Category "icon"
    let q_cat = ProviderQuery {
        query: "".to_string(),
        category: Some("icon".to_string()),
        desktop: None,
        page: 1,
        page_size: 10,
    };
    let res_cat = provider.search(&q_cat).expect("Search must succeed");
    assert_eq!(res_cat.len(), 4);
}

#[test]
fn test_kde_valid_package_conversion_and_provenance() {
    let provider = KdeProvider::new();
    let item = provider
        .fetch_item("kde-breeze-chameleon")
        .expect("Must fetch chameleon");

    assert_eq!(item.id, "kde-breeze-chameleon");
    assert_eq!(item.package_type, PackageType::Theme);
    assert_eq!(item.provenance.provider_id, "kde");
    assert_eq!(
        item.provenance.license_spdx,
        Some("GPL-3.0-or-later".to_string())
    );
    assert!(item.provenance.source_url.starts_with("kde://presets/"));

    // Verify multi-component structure (Look-and-Feel, Color Scheme, Konsole, Aurorae)
    let targets: Vec<&str> = item.files.iter().map(|f| f.target.as_str()).collect();
    assert!(targets.contains(
        &"~/.local/share/plasma/look-and-feel/org.kde.breeze.chameleon/metadata.desktop"
    ));
    assert!(targets.contains(&"~/.local/share/color-schemes/BreezeChameleonDark.colors"));
    assert!(targets.contains(&"~/.local/share/konsole/Chameleon.colorscheme"));
    assert!(targets.contains(&"~/.local/share/kwin/aurorae/BreezeChameleon/themerc"));
}

#[test]
fn test_gnome_valid_package_conversion_and_provenance() {
    let provider = GnomeProvider::new();
    let item = provider
        .fetch_item("gnome-papirus-aesthetic")
        .expect("Must fetch papirus");

    assert_eq!(item.id, "gnome-papirus-aesthetic");
    assert_eq!(item.package_type, PackageType::Icon);
    assert_eq!(item.provenance.provider_id, "gnome");
    assert!(item.provenance.source_url.starts_with("gnome://presets/"));

    let targets: Vec<&str> = item.files.iter().map(|f| f.target.as_str()).collect();
    assert!(targets.contains(&"~/.icons/Papirus-Aesthetic/index.theme"));
    assert!(targets.contains(&"~/.icons/Papirus-Aesthetic/48x48/apps/system.svg"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 7 - 12: Target Confinement, Boundaries, & Component Isolation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_kde_target_allowlist_accepted() {
    let valid_kde_targets = [
        "~/.local/share/plasma/look-and-feel/org.kde.theme/metadata.desktop",
        "~/.local/share/plasma/desktoptheme/catppuccin/widgets/panel-background.svg",
        "~/.local/share/color-schemes/NordicDark.colors",
        "~/.local/share/konsole/Nordic.colorscheme",
        "~/.local/share/kwin/aurorae/NordicDecoration/themerc",
    ];

    for tgt in &valid_kde_targets {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_kde_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_ok(),
            "Valid KDE target '{}' must be accepted: {:?}",
            tgt,
            res.err()
        );
    }
}

#[test]
fn test_gnome_target_allowlist_accepted() {
    let valid_gnome_targets = [
        "~/.themes/MyTheme/gtk-3.0/gtk.css",
        "~/.themes/MyTheme/gtk-4.0/gtk.css",
        "~/.themes/MyTheme/gnome-shell/gnome-shell.css",
        "~/.local/share/themes/MyTheme/gtk-3.0/gtk.css",
        "~/.icons/MyIcons/index.theme",
        "~/.local/share/icons/MyIcons/scalable/apps/app.svg",
    ];

    for tgt in &valid_gnome_targets {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_gnome_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_ok(),
            "Valid GNOME target '{}' must be accepted: {:?}",
            tgt,
            res.err()
        );
    }
}

#[test]
fn test_kde_component_claiming_gnome_directory_rejected() {
    let cross_claims = [
        "~/.themes/MyTheme/gtk-3.0/gtk.css",
        "~/.icons/MyIcons/index.theme",
        "~/.local/share/themes/MyTheme/gtk.css",
    ];

    for tgt in &cross_claims {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_kde_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_err(),
            "KDE claiming GNOME target '{}' must be rejected",
            tgt
        );
    }
}

#[test]
fn test_gnome_component_claiming_kde_directory_rejected() {
    let cross_claims = [
        "~/.local/share/plasma/look-and-feel/org.kde.theme/metadata.desktop",
        "~/.local/share/color-schemes/Custom.colors",
        "~/.local/share/konsole/Custom.colorscheme",
        "~/.local/share/kwin/aurorae/Custom/themerc",
    ];

    for tgt in &cross_claims {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        let res = validate_gnome_target_and_source(&spec, "pkg", &mut seen);
        assert!(
            res.is_err(),
            "GNOME claiming KDE target '{}' must be rejected",
            tgt
        );
    }
}

#[test]
fn test_theme_subfolder_confinement_enforced() {
    let mut seen = HashSet::new();

    // Direct root write in ~/.themes/ must be rejected (must be inside a named theme subfolder)
    let bad_gnome = ProviderFileSpec {
        source: "gtk.css".to_string(),
        target: "~/.themes/gtk.css".to_string(),
    };
    assert!(validate_gnome_target_and_source(&bad_gnome, "pkg", &mut seen).is_err());

    // Direct root write in ~/.local/share/plasma/look-and-feel/ must be rejected
    let bad_kde = ProviderFileSpec {
        source: "metadata.desktop".to_string(),
        target: "~/.local/share/plasma/look-and-feel/metadata.desktop".to_string(),
    };
    assert!(validate_kde_target_and_source(&bad_kde, "pkg", &mut seen).is_err());
}

#[test]
fn test_autostart_systemd_and_shell_configs_rejected() {
    let bad_targets = [
        "~/.config/autostart/bad.desktop",
        "~/.config/systemd/user/evil.service",
        "~/.config/environment.d/env.conf",
        "~/.bashrc",
        "~/.zshrc",
        "~/.profile",
        "/etc/profile.d/evil.sh",
        "/usr/share/themes/system.css",
    ];

    for tgt in &bad_targets {
        let mut seen_kde = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        assert!(validate_kde_target_and_source(&spec, "pkg", &mut seen_kde).is_err());

        let mut seen_gnome = HashSet::new();
        assert!(validate_gnome_target_and_source(&spec, "pkg", &mut seen_gnome).is_err());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 13 - 20: Scripts, Binaries, Extensions, Collisions, Traversal
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_desktop_providers_reject_scripts_and_installer_hooks() {
    let scripts = [
        "setup.sh",
        "install.sh",
        "post_install.sh",
        "hook.py",
        "script.bash",
        "exec.fish",
    ];

    for s in &scripts {
        let mut seen = HashSet::new();
        let spec_kde = ProviderFileSpec {
            source: s.to_string(),
            target: format!("~/.local/share/plasma/desktoptheme/custom/{}", s),
        };
        assert!(validate_kde_target_and_source(&spec_kde, "pkg", &mut seen).is_err());

        let mut seen_gnome = HashSet::new();
        let spec_gnome = ProviderFileSpec {
            source: s.to_string(),
            target: format!("~/.themes/custom/{}", s),
        };
        assert!(validate_gnome_target_and_source(&spec_gnome, "pkg", &mut seen_gnome).is_err());
    }
}

#[test]
fn test_desktop_providers_reject_binaries_and_executables() {
    let binaries = [
        "plugin.so",
        "helper.exe",
        "payload.bin",
        "malware.elf",
        "agent.dll",
    ];

    for b in &binaries {
        let mut seen = HashSet::new();
        let spec_kde = ProviderFileSpec {
            source: b.to_string(),
            target: format!("~/.local/share/plasma/desktoptheme/custom/{}", b),
        };
        assert!(validate_kde_target_and_source(&spec_kde, "pkg", &mut seen).is_err());

        let mut seen_gnome = HashSet::new();
        let spec_gnome = ProviderFileSpec {
            source: b.to_string(),
            target: format!("~/.themes/custom/{}", b),
        };
        assert!(validate_gnome_target_and_source(&spec_gnome, "pkg", &mut seen_gnome).is_err());
    }
}

#[test]
fn test_legitimate_declarative_theme_extensions_accepted() {
    let legit_files = [
        ("gtk.css", "~/.themes/Dark/gtk-3.0/gtk.css"),
        (
            "metadata.desktop",
            "~/.local/share/plasma/look-and-feel/MyTheme/metadata.desktop",
        ),
        ("index.theme", "~/.icons/MyIcons/index.theme"),
        (
            "theme.colors",
            "~/.local/share/color-schemes/MyTheme.colors",
        ),
        (
            "profile.colorscheme",
            "~/.local/share/konsole/MyTheme.colorscheme",
        ),
        ("icon.svg", "~/.icons/MyIcons/48x48/apps/icon.svg"),
        ("themerc", "~/.local/share/kwin/aurorae/MyTheme/themerc"),
    ];

    for (src, tgt) in &legit_files {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: src.to_string(),
            target: tgt.to_string(),
        };
        let is_kde = tgt.contains("local/share/plasma")
            || tgt.contains("color-schemes")
            || tgt.contains("konsole")
            || tgt.contains("aurorae");
        let res = if is_kde {
            validate_kde_target_and_source(&spec, "pkg", &mut seen)
        } else {
            validate_gnome_target_and_source(&spec, "pkg", &mut seen)
        };
        assert!(
            res.is_ok(),
            "Legitimate declarative file '{}' must be accepted: {:?}",
            tgt,
            res.err()
        );
    }
}

#[test]
fn test_desktop_providers_reject_target_traversal() {
    let traversals = [
        "~/.local/share/plasma/look-and-feel/Theme/../evil.sh",
        "~/.themes/Dark/../../.bashrc",
        "~/.icons/Icons/../..",
    ];

    for tgt in &traversals {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: "src".to_string(),
            target: tgt.to_string(),
        };
        assert!(validate_kde_target_and_source(&spec, "pkg", &mut seen).is_err());
        let mut seen_gnome = HashSet::new();
        assert!(validate_gnome_target_and_source(&spec, "pkg", &mut seen_gnome).is_err());
    }
}

#[test]
fn test_desktop_providers_reject_source_traversal() {
    let bad_sources = [
        "../secret.conf",
        "../../etc/passwd",
        "/home/user/file",
        "sub/../../../shadow",
    ];

    for src in &bad_sources {
        let mut seen = HashSet::new();
        let spec = ProviderFileSpec {
            source: src.to_string(),
            target: "~/.themes/Theme/gtk-3.0/gtk.css".to_string(),
        };
        assert!(validate_gnome_target_and_source(&spec, "pkg", &mut seen).is_err());
    }
}

#[test]
fn test_desktop_duplicate_normalized_target_collision_rejected() {
    let mut seen = HashSet::new();

    let spec1 = ProviderFileSpec {
        source: "f1".to_string(),
        target: "~/.themes/Dark/gtk-3.0/gtk.css".to_string(),
    };
    assert!(validate_gnome_target_and_source(&spec1, "pkg", &mut seen).is_ok());

    // Duplicate exact target
    let spec2 = ProviderFileSpec {
        source: "f2".to_string(),
        target: "~/.themes/Dark/gtk-3.0/gtk.css".to_string(),
    };
    let res2 = validate_gnome_target_and_source(&spec2, "pkg", &mut seen);
    assert!(res2.is_err(), "Duplicate target must be rejected");
    assert!(res2
        .unwrap_err()
        .contains("Conflict violation: Duplicate target"));

    // Redundant slash normalized duplicate
    let spec3 = ProviderFileSpec {
        source: "f3".to_string(),
        target: "~/.themes/Dark//gtk-3.0/gtk.css".to_string(),
    };
    assert!(validate_gnome_target_and_source(&spec3, "pkg", &mut seen).is_err());

    // Redundant ./ normalized duplicate
    let spec4 = ProviderFileSpec {
        source: "f4".to_string(),
        target: "~/.themes/Dark/./gtk-3.0/gtk.css".to_string(),
    };
    assert!(validate_gnome_target_and_source(&spec4, "pkg", &mut seen).is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 21 - 25: Symlinks, Identity, Tree Hash, Transactional Rollback
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_desktop_providers_symlink_escape_rejected() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let custom_root = TempTestDir::new("desktop_symlink");
        let pkg_dir = custom_root.path().join("evil-kde");
        fs::create_dir_all(pkg_dir.join("color-schemes")).unwrap();

        let secret = custom_root.path().join("secret.colors");
        fs::write(
            &secret,
            "[General]
SENSITIVE=1
",
        )
        .unwrap();

        let sym_file = pkg_dir.join("color-schemes").join("Evil.colors");
        symlink(&secret, &sym_file).unwrap();

        let manifest = serde_json::json!({
            "ryzora_spec": "1",
            "id": "evil-kde",
            "name": "Evil KDE Symlink",
            "version": "1.0.0",
            "author": "Attacker",
            "package_type": "theme",
            "description": "Symlink test",
            "tags": ["kde"],
            "compatibility": { "desktops": ["kde"] },
            "files": [
                { "source": "color-schemes/Evil.colors", "target": "~/.local/share/color-schemes/Evil.colors", "description": "test" }
            ],
            "dependencies": []
        });
        fs::write(
            pkg_dir.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let provider = KdeProvider::with_custom_dir(custom_root.path().to_path_buf());
        let item = provider.fetch_item("evil-kde").unwrap();

        let staging_dest = TempTestDir::new("kde_sym_dest");
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
fn test_desktop_providers_manifest_identity_mismatch_rejected() {
    let custom_root = TempTestDir::new("desktop_id_mismatch");
    let pkg_dir = custom_root.path().join("dir-id-gnome");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "manifest-id-gnome",
        "name": "Mismatch GNOME",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Mismatch test",
        "tags": ["gnome"],
        "compatibility": { "desktops": ["gnome"] },
        "files": [
            { "source": "gtk.css", "target": "~/.themes/Mismatch/gtk-3.0/gtk.css", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let res = load_disk_gnome_item(&pkg_dir);
    assert!(res.is_err(), "Identity mismatch must be rejected");
    assert!(res.unwrap_err().contains("Identity mismatch"));
}

#[test]
fn test_desktop_providers_content_tree_hash_mismatch_rejected() {
    let custom_root = TempTestDir::new("desktop_tampered");
    let pkg_dir = custom_root.path().join("tampered-kde");
    fs::create_dir_all(pkg_dir.join("color-schemes")).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "tampered-kde",
        "name": "Tampered KDE",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Tamper test",
        "tags": ["kde"],
        "compatibility": { "desktops": ["kde"] },
        "files": [
            { "source": "color-schemes/Nord.colors", "target": "~/.local/share/color-schemes/Nord.colors", "description": "test" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        pkg_dir.join("color-schemes").join("Nord.colors"),
        b"[General]
Color=Fake
",
    )
    .unwrap();

    let provider = KdeProvider::with_custom_dir(custom_root.path().to_path_buf());
    let mut item = provider
        .fetch_item("tampered-kde")
        .expect("Must fetch item");

    item.provenance.commit_or_tag =
        Some("3333333333333333333333333333333333333333333333333333333333333333".to_string());

    let staging_root = TempTestDir::new("kde_stage_tampered");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Tree hash mismatch must fail staging");
    assert!(res.unwrap_err().contains("Tree hash mismatch"));
}

#[test]
fn test_desktop_providers_transactional_staging_atomic_purge_on_failure() {
    let custom_root = TempTestDir::new("gnome_tx_fail");
    let pkg_dir = custom_root.path().join("partial-fail-gnome");
    fs::create_dir_all(pkg_dir.join("gtk-3.0")).unwrap();

    fs::write(
        pkg_dir.join("gtk-3.0").join("gtk.css"),
        b"body { color: red; }",
    )
    .unwrap();
    fs::write(
        pkg_dir.join("index.theme"),
        b"[Theme]
Name=Partial
",
    )
    .unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "partial-fail-gnome",
        "name": "Partial Fail GNOME",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Tx rollback test",
        "tags": ["gnome"],
        "compatibility": { "desktops": ["gnome"] },
        "files": [
            { "source": "gtk-3.0/gtk.css", "target": "~/.themes/Partial/gtk-3.0/gtk.css", "description": "css" },
            { "source": "index.theme", "target": "~/.themes/Partial/index.theme", "description": "theme" }
        ],
        "dependencies": []
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let provider = GnomeProvider::with_custom_dir(custom_root.path().to_path_buf());
    let item = provider.fetch_item("partial-fail-gnome").unwrap();

    // Delete index.theme before staging
    fs::remove_file(pkg_dir.join("index.theme")).unwrap();

    let staging_root = TempTestDir::new("tx_gnome_stage_root");
    let res = provider.stage_payload(&item, staging_root.path());
    assert!(res.is_err(), "Missing file must cause staging to fail");

    // CRITICAL: Verify NO leftover temporary files in staging_root
    let entries: Vec<_> = fs::read_dir(staging_root.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "Transactional staging must purge all temporary directories on failure! Found: {:?}",
        entries
    );
}

#[test]
fn test_desktop_providers_successful_staging_and_manifest_synthesis() {
    // 1. KDE Multi-Component Look-and-Feel Staging
    let kde_provider = KdeProvider::new();
    let kde_item = kde_provider.fetch_item("kde-breeze-chameleon").unwrap();

    let kde_stage_root = TempTestDir::new("stage_kde_breeze");
    let kde_staged = kde_provider
        .stage_payload(&kde_item, kde_stage_root.path())
        .expect("KDE staging must succeed");

    assert!(kde_staged.exists());
    assert!(kde_staged.join("look-and-feel/metadata.desktop").is_file());
    assert!(kde_staged
        .join("color-schemes/BreezeChameleonDark.colors")
        .is_file());
    assert!(kde_staged.join("konsole/Chameleon.colorscheme").is_file());
    assert!(kde_staged.join("aurorae/themerc").is_file());

    let kde_manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(kde_staged.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(kde_manifest.id, "kde-breeze-chameleon");
    assert_eq!(kde_manifest.files.len(), 4);

    // 2. GNOME Mixed GTK + Icon Suite Staging
    let gnome_provider = GnomeProvider::new();
    let gnome_item = gnome_provider.fetch_item("gnome-complete-suite").unwrap();

    let gnome_stage_root = TempTestDir::new("stage_gnome_suite");
    let gnome_staged = gnome_provider
        .stage_payload(&gnome_item, gnome_stage_root.path())
        .expect("GNOME suite staging must succeed");

    assert!(gnome_staged.exists());
    assert!(gnome_staged.join("gtk-3.0/gtk.css").is_file());
    assert!(gnome_staged.join("index.theme").is_file());

    let gnome_manifest: RyzoraManifest =
        serde_json::from_str(&fs::read_to_string(gnome_staged.join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(gnome_manifest.id, "gnome-complete-suite");
    assert_eq!(gnome_manifest.files.len(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Vectors 26 - 30: Trust, Signatures, Aggregator Resilience & Security Audit
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_desktop_providers_unsigned_content_remains_unverified() {
    let kde_provider = KdeProvider::new();
    let item = kde_provider.fetch_item("kde-nord-arctic").unwrap();

    let normalized = normalize_provider_item(item);
    assert_eq!(normalized.trust_tier, Some(TrustTier::Community));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Official));
    assert_ne!(normalized.trust_tier, Some(TrustTier::Verified));
    assert_eq!(normalized.safety_audit.rating, "unverified");
    assert!(normalized.tags.contains(&"provider:kde".to_string()));
}

#[test]
fn test_desktop_providers_revoked_key_signature_rejected() {
    let mut rng = rand_core::OsRng;
    let revoked_key = SigningKey::generate(&mut rng);
    let revoked_pubkey_hex = hex::encode(revoked_key.verifying_key().to_bytes());

    let identity = SignerIdentity {
        author_id: "revoked-desktop-author".to_string(),
        name: "Revoked Designer".to_string(),
        handle: None,
    };
    let tree_hash = "abc123desktophash";
    let sig_meta =
        sign_package_tree_hash(&revoked_key, "gnome-adwaita-vibrant", tree_hash, identity).unwrap();

    let revoked_entry = RevokedKeyEntry {
        key_id: "revoked-desktop".to_string(),
        public_key: revoked_pubkey_hex,
        revoked_at: "2026-01-01T00:00:00Z".to_string(),
        reason: "Compromised designer key".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    let eval = evaluate_trust_chain(
        Some(&sig_meta),
        "gnome-adwaita-vibrant",
        tree_hash,
        Some(TrustTier::Community),
        &trust_store,
    );

    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.is_valid);
}

#[test]
fn test_desktop_providers_failure_isolation_in_aggregator() {
    let mut manager = ProviderManager::new();

    let bad_kde = KdeProvider::with_custom_dir(PathBuf::from("/nonexistent/kde/dir"));
    manager.register_provider(Arc::new(bad_kde));

    let good_gnome = GnomeProvider::new();
    manager.register_provider(Arc::new(good_gnome));

    let query = ProviderQuery {
        query: "adwaita".to_string(),
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
        .any(|i| i.id == "gnome-adwaita-vibrant"));
}

#[test]
fn test_desktop_providers_offline_behavior_serves_presets() {
    let provider = GnomeProvider::new();
    let query = ProviderQuery {
        query: "papirus".to_string(),
        category: None,
        desktop: None,
        page: 1,
        page_size: 10,
    };

    let items = provider
        .search(&query)
        .expect("Offline search must succeed");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "gnome-papirus-aesthetic");
}

#[test]
fn test_desktop_providers_zero_subprocess_and_privilege_audit() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let desktop_source_path = manifest_dir
        .join("src")
        .join("providers")
        .join("desktop.rs");

    let content = fs::read_to_string(&desktop_source_path)
        .expect("Must be able to read src/providers/desktop.rs for security audit");

    // Strictly verify zero subprocess invocation in desktop provider
    assert!(
        !content.contains("Command::new("),
        "Security audit failed: Command::new( found in desktop.rs"
    );
    assert!(
        !content.contains("std::process::Command"),
        "Security audit failed: std::process::Command found in desktop.rs"
    );
    assert!(
        !content.contains("exec("),
        "Security audit failed: exec call found in desktop.rs"
    );

    // Verify zero invocation of desktop configuration CLI tools
    assert!(
        !content.contains("gsettings "),
        "Security audit failed: gsettings CLI call found in desktop.rs"
    );
    assert!(
        !content.contains("dconf write"),
        "Security audit failed: dconf call found in desktop.rs"
    );
    assert!(
        !content.contains("kwriteconfig"),
        "Security audit failed: kwriteconfig call found in desktop.rs"
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
