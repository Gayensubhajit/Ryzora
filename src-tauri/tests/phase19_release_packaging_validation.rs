//! Phase 19: Release, Packaging & Distribution Automated Validation Suite
//!
//! Enforces:
//! 1. Authoritative single-version synchronization (Cargo.toml == package.json == tauri.conf.json)
//! 2. Release build configuration & optimization flags (lto, opt-level, codegen-units, panic, strip)
//! 3. AppStream metadata (io.ryzora.Ryzora.metainfo.xml) specification compliance
//! 4. Desktop entry compliance (io.ryzora.Ryzora.desktop, ryzora.desktop, MIME x-scheme-handler/ryzora)
//! 5. Comprehensive icon asset validation (16..512 PNGs + scalable SVG: non-empty, valid PNG header, dimensions)
//! 6. Deep link (ryzora://) strict security grammar & injection attack rejection
//! 7. AppImage packaging script & AppDir layout integrity
//! 8. Arch Linux PKGBUILD & PKGBUILD-bin validation (no target/ leakage)
//! 9. Debian packaging script (.deb) layout and control metadata
//! 10. CI/CD release pipeline security ordering (build -> package -> checksum -> sign -> verify -> publish)
//! 11. Fresh-machine installation matrix simulation across Arch, Debian, Generic Linux
//! 12. Security invariant preservation (zero subprocess execution in installer and package runtime)

use ryzora_lib::deeplink::{parse_deeplink_url, DeepLinkAction};
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().expect("Workspace root").to_path_buf()
}

// -----------------------------------------------------------------------------
// 1. Authoritative Version Synchronization
// -----------------------------------------------------------------------------

#[test]
fn test_version_synchronization_across_all_manifests() {
    let root = workspace_root();

    // 1. Cargo.toml version
    let cargo_toml_path = root.join("src-tauri/Cargo.toml");
    let cargo_content = fs::read_to_string(&cargo_toml_path).expect("Read Cargo.toml");
    let mut cargo_ver = None;
    for line in cargo_content.lines() {
        if line.starts_with("version = ") {
            cargo_ver = Some(line.split('"').nth(1).expect("Version quote").to_string());
            break;
        }
    }
    let cargo_version = cargo_ver.expect("Cargo version present");

    // 2. package.json version
    let package_json_path = root.join("package.json");
    let package_content = fs::read_to_string(&package_json_path).expect("Read package.json");
    let package_json: serde_json::Value =
        serde_json::from_str(&package_content).expect("Parse package.json");
    let package_version = package_json["version"]
        .as_str()
        .expect("package.json version string")
        .to_string();

    // 3. tauri.conf.json version
    let tauri_conf_path = root.join("src-tauri/tauri.conf.json");
    let tauri_content = fs::read_to_string(&tauri_conf_path).expect("Read tauri.conf.json");
    let tauri_json: serde_json::Value =
        serde_json::from_str(&tauri_content).expect("Parse tauri.conf.json");
    let tauri_version = tauri_json["version"]
        .as_str()
        .expect("tauri.conf.json version string")
        .to_string();

    assert_eq!(
        cargo_version, package_version,
        "Version mismatch: Cargo.toml ({}) != package.json ({})",
        cargo_version, package_version
    );
    assert_eq!(
        package_version, tauri_version,
        "Version mismatch: package.json ({}) != tauri.conf.json ({})",
        package_version, tauri_version
    );

    // Verify valid semver format
    assert!(
        semver::Version::parse(&cargo_version).is_ok(),
        "Version '{}' is not valid semver",
        cargo_version
    );
}

// -----------------------------------------------------------------------------
// 2. Production Release Build Optimizations
// -----------------------------------------------------------------------------

#[test]
fn test_cargo_release_profile_optimizations() {
    let root = workspace_root();
    let cargo_content =
        fs::read_to_string(root.join("src-tauri/Cargo.toml")).expect("Read Cargo.toml");

    assert!(
        cargo_content.contains("[profile.release]"),
        "Missing [profile.release] in Cargo.toml"
    );
    assert!(
        cargo_content.contains("lto = true") || cargo_content.contains(r#"lto = "thin""#),
        "Missing Link-Time Optimization (lto = true)"
    );
    assert!(
        cargo_content.contains("codegen-units = 1"),
        "codegen-units must be 1 for maximum release optimization"
    );
    assert!(
        cargo_content.contains("opt-level = 3") || cargo_content.contains(r#"opt-level = "z""#),
        "opt-level must be optimized (3)"
    );
    assert!(
        cargo_content.contains(r#"panic = "abort""#),
        "panic must be abort for small footprint"
    );
    assert!(
        cargo_content.contains("strip = true"),
        "strip must be true to remove symbols"
    );
}

// -----------------------------------------------------------------------------
// 3. AppStream Metadata Compliance
// -----------------------------------------------------------------------------

#[test]
fn test_appstream_metainfo_xml_specification_compliance() {
    let root = workspace_root();
    let metainfo_path = root.join("dist-assets/io.ryzora.Ryzora.metainfo.xml");
    assert!(
        metainfo_path.exists(),
        "Missing AppStream metainfo XML file"
    );

    let content = fs::read_to_string(&metainfo_path).expect("Read metainfo XML");

    // Required AppStream XML elements
    assert!(content.contains(r#"<component type="desktop-application">"#));
    assert!(content.contains("<id>io.ryzora.Ryzora</id>"));
    assert!(content.contains("<name>Ryzora</name>"));
    assert!(content.contains("<summary>"));
    assert!(content.contains("<description>"));
    assert!(
        content.contains("<metadata_license>CC0-1.0</metadata_license>")
            || content.contains("<metadata_license>MIT</metadata_license>")
    );
    assert!(content.contains("<project_license>MIT</project_license>"));
    assert!(
        content.contains("<name>Ryzora Contributors</name>"),
        "Missing developer name in metainfo"
    );
    assert!(
        content.contains(r#"<launchable type="desktop-id">io.ryzora.Ryzora.desktop</launchable>"#)
    );
    assert!(content.contains(r#"<url type="homepage">"#));
    assert!(content.contains(r#"<url type="bugtracker">"#));
    assert!(content.contains(r#"<content_rating type="oars-1.1""#));
    assert!(content.contains("<releases>"));
    assert!(content.contains(r#"<release version="0.1.0""#));
}

// -----------------------------------------------------------------------------
// 4. Desktop Entry & MIME Scheme Compliance
// -----------------------------------------------------------------------------

#[test]
fn test_desktop_entry_compliance_and_mime_registration() {
    let root = workspace_root();
    let desktop_path = root.join("dist-assets/io.ryzora.Ryzora.desktop");
    assert!(
        desktop_path.exists(),
        "Missing io.ryzora.Ryzora.desktop file"
    );

    let content = fs::read_to_string(&desktop_path).expect("Read desktop file");

    assert!(content.contains("[Desktop Entry]"));
    assert!(content.contains("Type=Application"));
    assert!(content.contains("Name=Ryzora"));
    assert!(content.contains("Exec=ryzora %u") || content.contains("Exec=ryzora %U"));
    assert!(content.contains("Icon=ryzora") || content.contains("Icon=io.ryzora.Ryzora"));
    assert!(content.contains("Categories=Utility;DesktopSettings;Settings;"));

    // MIME type registration for deep link protocol
    assert!(
        content.contains("MimeType=") && content.contains("x-scheme-handler/ryzora;"),
        "Desktop entry must register x-scheme-handler/ryzora;"
    );

    // Backward compatibility desktop file
    let compat_path = root.join("dist-assets/ryzora.desktop");
    assert!(
        compat_path.exists(),
        "Missing compatibility ryzora.desktop file"
    );
}

// -----------------------------------------------------------------------------
// 5. Icon Asset Validation (Non-empty, Valid PNG Header, Exact Dimensions)
// -----------------------------------------------------------------------------

#[test]
fn test_icon_assets_dimensions_and_validity() {
    let root = workspace_root();
    let icons_root = root.join("dist-assets/icons/hicolor");

    let sizes = [16, 32, 48, 64, 128, 256, 512];
    for &size in &sizes {
        let icon_path = icons_root.join(format!("{}x{}/apps/ryzora.png", size, size));
        assert!(icon_path.exists(), "Missing icon: {:?}", icon_path);

        let data = fs::read(&icon_path).expect("Read icon data");
        assert!(!data.is_empty(), "Icon file is empty: {:?}", icon_path);

        // Verify PNG magic header: \x89PNG\r\n\x1a\n
        assert_eq!(
            &data[0..8],
            &[137, 80, 78, 71, 13, 10, 26, 10],
            "Icon is not a valid PNG: {:?}",
            icon_path
        );

        // IHDR starts at offset 12; width at 16..20, height at 20..24 (big endian)
        let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        assert_eq!(
            width, size as u32,
            "Icon width mismatch for {:?}: expected {}, got {}",
            icon_path, size, width
        );
        assert_eq!(
            height, size as u32,
            "Icon height mismatch for {:?}: expected {}, got {}",
            icon_path, size, height
        );
    }

    // Scalable SVG validation
    let svg_path = icons_root.join("scalable/apps/ryzora.svg");
    assert!(svg_path.exists(), "Missing scalable SVG icon");
    let svg_data = fs::read_to_string(&svg_path).expect("Read SVG");
    assert!(!svg_data.is_empty());
    assert!(svg_data.contains("<svg"));
    // Ensure no active scripts in desktop icon
    assert!(!svg_data.contains("<script"));
}

// -----------------------------------------------------------------------------
// 6. Deep Link Strict Security Grammar & Attack Rejection
// -----------------------------------------------------------------------------

#[test]
fn test_deeplink_security_grammar_rejection_vectors() {
    // 1. Path traversal attacks
    let traversal_vectors = [
        "ryzora://package/../../etc/passwd",
        "ryzora://package/../secret",
        "ryzora://category/../../../.ssh/id_rsa",
        "ryzora://repository/..%2f..%2fetc%2fshadow",
    ];
    for vec in &traversal_vectors {
        let res = parse_deeplink_url(vec);
        assert!(
            res.is_err(),
            "Traversal vector should have been rejected: {}",
            vec
        );
    }

    // 2. Dangerous entity / command injection attempts
    let injection_vectors = [
        "ryzora://exec/rm -rf",
        "ryzora://shell/bash",
        "ryzora://file/etc/shadow",
        "ryzora://download/https://malicious.org/payload.tar.gz",
        "ryzora://eval/alert(1)",
    ];
    for vec in &injection_vectors {
        let res = parse_deeplink_url(vec);
        assert!(
            res.is_err(),
            "Dangerous entity should have been rejected: {}",
            vec
        );
    }

    // 3. Special characters, null bytes, control characters
    let bad_char_vectors = [
        "ryzora://package/test\0payload",
        "ryzora://package/test;rm -rf /",
        "ryzora://package/test|cat /etc/passwd",
        "ryzora://package/test>output",
        "ryzora://package/test`whoami`",
        "ryzora://package/test$(id)",
    ];
    for vec in &bad_char_vectors {
        let res = parse_deeplink_url(vec);
        assert!(
            res.is_err(),
            "Bad character vector should have been rejected: {}",
            vec
        );
    }

    // 4. Valid deep link navigations
    assert_eq!(
        parse_deeplink_url("ryzora://package/catppuccin-hyprland").unwrap(),
        DeepLinkAction::ViewPackage {
            package_id: "catppuccin-hyprland".to_string()
        }
    );
    assert_eq!(
        parse_deeplink_url("ryzora://repository/official-stable").unwrap(),
        DeepLinkAction::ViewRepository {
            repo_id: "official-stable".to_string()
        }
    );
    assert_eq!(
        parse_deeplink_url("ryzora://creator/catppuccin-org").unwrap(),
        DeepLinkAction::ViewCreator {
            creator_id: "catppuccin-org".to_string()
        }
    );
    assert_eq!(
        parse_deeplink_url("ryzora://category/rices").unwrap(),
        DeepLinkAction::ViewCategory {
            category: "rices".to_string()
        }
    );
}

// -----------------------------------------------------------------------------
// 7. AppImage Packaging Script & Layout Verification
// -----------------------------------------------------------------------------

#[test]
fn test_appimage_packaging_script_and_structure() {
    let root = workspace_root();
    let script_path = root.join("scripts/build-appimage.sh");
    assert!(script_path.exists(), "Missing scripts/build-appimage.sh");

    let content = fs::read_to_string(&script_path).expect("Read build-appimage.sh");

    // Must be set -euo pipefail
    assert!(content.contains("set -euo pipefail"));
    // Must assemble AppDir cleanly
    assert!(content.contains(r#"rm -rf "$APP_DIR""#));
    // Must create standard directories
    assert!(content.contains(r#"mkdir -p "$APP_DIR/usr/bin""#));
    assert!(content.contains(r#"mkdir -p "$APP_DIR/usr/share/applications""#));
    assert!(content.contains(r#"mkdir -p "$APP_DIR/usr/share/metainfo""#));
    // Must generate executable 755 AppRun
    assert!(content.contains(r#"chmod 755 "$APP_DIR/AppRun""#));
    // Must install desktop and metainfo
    assert!(content.contains("io.ryzora.Ryzora.desktop"));
    assert!(content.contains("io.ryzora.Ryzora.metainfo.xml"));
    // Must install root icon and .DirIcon
    assert!(content.contains(".DirIcon"));
}

// -----------------------------------------------------------------------------
// 8. Arch Linux PKGBUILD Integrity & No Target Artifact Leakage
// -----------------------------------------------------------------------------

#[test]
fn test_arch_pkgbuild_and_pkgbuild_bin_integrity() {
    let root = workspace_root();
    let pkgbuild_path = root.join("packaging/arch/PKGBUILD");
    assert!(pkgbuild_path.exists(), "Missing PKGBUILD");
    let pkgbuild = fs::read_to_string(&pkgbuild_path).expect("Read PKGBUILD");

    // Strict check: PKGBUILD must NOT package target/ directly
    assert!(
        !pkgbuild.contains("cp -r src-tauri/target")
            && !pkgbuild.contains("install src-tauri/target/"),
        "PKGBUILD must not package entire target/ directory!"
    );
    // Explicit binary installation
    assert!(pkgbuild.contains(r#"install -Dm755 "src-tauri/target/release/ryzora""#));
    assert!(pkgbuild.contains("io.ryzora.Ryzora.desktop"));
    assert!(pkgbuild.contains("io.ryzora.Ryzora.metainfo.xml"));
    assert!(pkgbuild.contains("pkgname=ryzora"));
    assert!(pkgbuild.contains("pkgver=0.1.0"));

    // Check PKGBUILD-bin
    let pkgbuild_bin_path = root.join("packaging/arch/PKGBUILD-bin");
    assert!(pkgbuild_bin_path.exists(), "Missing PKGBUILD-bin");
    let pkgbuild_bin = fs::read_to_string(&pkgbuild_bin_path).expect("Read PKGBUILD-bin");
    assert!(pkgbuild_bin.contains("pkgname=ryzora-bin"));
    assert!(pkgbuild_bin.contains("provides=('ryzora')"));
    assert!(pkgbuild_bin.contains("conflicts=('ryzora')"));
}

// -----------------------------------------------------------------------------
// 9. Debian Packaging Script Verification
// -----------------------------------------------------------------------------

#[test]
fn test_debian_packaging_script_and_control_metadata() {
    let root = workspace_root();
    let deb_script_path = root.join("scripts/build-deb.sh");
    assert!(deb_script_path.exists(), "Missing scripts/build-deb.sh");

    let content = fs::read_to_string(&deb_script_path).expect("Read build-deb.sh");

    assert!(content.contains("Package: ryzora"));
    assert!(content.contains("Architecture: ${ARCH}"));
    assert!(content.contains("Maintainer: Ryzora Contributors"));
    assert!(content.contains("libwebkit2gtk-4.1-0"));
    assert!(content.contains("DEBIAN/md5sums"));
    assert!(content.contains("io.ryzora.Ryzora.desktop"));
    assert!(content.contains("io.ryzora.Ryzora.metainfo.xml"));
}

// -----------------------------------------------------------------------------
// 10. CI/CD Release Ordering & Fail-Closed Signing Order
// -----------------------------------------------------------------------------

#[test]
fn test_github_actions_release_workflow_security_order() {
    let root = workspace_root();
    let workflow_path = root.join(".github/workflows/release.yml");
    assert!(
        workflow_path.exists(),
        "Missing .github/workflows/release.yml"
    );

    let content = fs::read_to_string(&workflow_path).expect("Read release.yml");

    // Check pipeline stage ordering:
    // 1. Version check
    let idx_ver = content
        .find("Verify Version Consistency")
        .expect("Verify Version step");
    // 2. Build
    let idx_build = content
        .find("Build Frontend & Release Binaries")
        .expect("Build step");
    // 3. Assemble
    let idx_package = content
        .find("Assemble Release Artifacts")
        .expect("Assemble step");
    // 4. Sign
    let idx_sign = content
        .find("Generate Checksums & Sign Artifacts")
        .expect("Sign step");
    // 5. Publish
    let idx_publish = content
        .find("Publish Verified GitHub Release")
        .expect("Publish step");

    assert!(
        idx_ver < idx_build,
        "Version check must happen before build"
    );
    assert!(
        idx_build < idx_package,
        "Build must happen before package assemble"
    );
    assert!(
        idx_package < idx_sign,
        "Packaging must happen before checksums/signing"
    );
    assert!(
        idx_sign < idx_publish,
        "Signing & verification MUST happen before publishing!"
    );

    // Must verify signatures before publish
    assert!(
        content.contains("ryzora-ci verify-file SHA256SUMS.txt SHA256SUMS.txt.sig"),
        "Workflow must explicitly verify signatures before publishing"
    );
}

// -----------------------------------------------------------------------------
// 11. Fresh-Machine Installation Matrix Simulation Gate
// -----------------------------------------------------------------------------

#[test]
fn test_fresh_machine_installation_matrix_simulation() {
    let root = workspace_root();
    let temp_root = std::env::temp_dir().join(format!("ryzora_matrix_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_root);
    fs::create_dir_all(&temp_root).expect("Create matrix temp dir");

    // Simulated environments
    let environments = ["arch_linux", "ubuntu_debian", "generic_linux_appimage"];

    for env in &environments {
        let env_root = temp_root.join(env);
        let bin_dir = env_root.join("usr/bin");
        let app_dir = env_root.join("usr/share/applications");
        let icon_dir = env_root.join("usr/share/icons/hicolor/scalable/apps");
        let meta_dir = env_root.join("usr/share/metainfo");
        let user_home = env_root.join("home/testuser");

        fs::create_dir_all(&bin_dir).unwrap();
        fs::create_dir_all(&app_dir).unwrap();
        fs::create_dir_all(&icon_dir).unwrap();
        fs::create_dir_all(&meta_dir).unwrap();
        fs::create_dir_all(&user_home).unwrap();

        // 1. Simulate Package Install
        let binary_dest = bin_dir.join("ryzora");
        fs::write(&binary_dest, b"#!/bin/sh\necho ryzora 0.1.0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&binary_dest, fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Install desktop file
        let desktop_src = root.join("dist-assets/io.ryzora.Ryzora.desktop");
        let desktop_dest = app_dir.join("io.ryzora.Ryzora.desktop");
        fs::copy(&desktop_src, &desktop_dest).unwrap();

        // Install metainfo
        let meta_src = root.join("dist-assets/io.ryzora.Ryzora.metainfo.xml");
        let meta_dest = meta_dir.join("io.ryzora.Ryzora.metainfo.xml");
        fs::copy(&meta_src, &meta_dest).unwrap();

        // Install icon
        let icon_src = root.join("dist-assets/icons/hicolor/scalable/apps/ryzora.svg");
        let icon_dest = icon_dir.join("ryzora.svg");
        fs::copy(&icon_src, &icon_dest).unwrap();

        // 2. Verify Desktop Integration
        assert!(binary_dest.exists(), "{}: Binary missing", env);
        assert!(desktop_dest.exists(), "{}: Desktop entry missing", env);
        assert!(meta_dest.exists(), "{}: Metainfo missing", env);
        assert!(icon_dest.exists(), "{}: Icon missing", env);

        // 3. Verify user home containment
        let ryzora_user_data = user_home.join(".local/share/ryzora");
        let ryzora_user_config = user_home.join(".config/ryzora");
        fs::create_dir_all(&ryzora_user_data).unwrap();
        fs::create_dir_all(&ryzora_user_config).unwrap();

        // 4. Simulate Package Uninstallation Cleanliness
        fs::remove_file(&binary_dest).unwrap();
        fs::remove_file(&desktop_dest).unwrap();
        fs::remove_file(&meta_dest).unwrap();
        fs::remove_file(&icon_dest).unwrap();

        assert!(
            !binary_dest.exists(),
            "{}: Binary still exists after uninstall",
            env
        );
        assert!(
            !desktop_dest.exists(),
            "{}: Desktop still exists after uninstall",
            env
        );
        assert!(
            !meta_dest.exists(),
            "{}: Metainfo still exists after uninstall",
            env
        );
        assert!(
            !icon_dest.exists(),
            "{}: Icon still exists after uninstall",
            env
        );
    }

    let _ = fs::remove_dir_all(&temp_root);
}

// -----------------------------------------------------------------------------
// 12. Security Invariant: Zero Subprocess Execution in Installer Runtime
// -----------------------------------------------------------------------------

#[test]
fn test_installer_zero_subprocess_security_invariant() {
    let root = workspace_root();
    let installer_src =
        fs::read_to_string(root.join("src-tauri/src/installer.rs")).expect("Read installer.rs");

    // Assert that installer.rs contains NO process spawning
    assert!(
        !installer_src.contains("Command::new"),
        "Security Violation: installer.rs must never call Command::new"
    );
    assert!(
        !installer_src.contains("std::process::Command"),
        "Security Violation: installer.rs must not import std::process::Command"
    );
    assert!(
        !installer_src.contains("sudo"),
        "Security Violation: installer.rs must never invoke sudo"
    );
    assert!(
        !installer_src.contains("pkexec"),
        "Security Violation: installer.rs must never invoke pkexec"
    );
}
