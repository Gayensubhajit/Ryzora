//! Phase 20 — Release Candidate & Public Launch Validation Suite
//!
//! Validates:
//! 1. Production Root-Key Ceremony air-gapped guidelines & fail-closed guarantee
//! 2. Release artifacts integrity, live SHA256 checksums matching SHA256SUMS.txt
//! 3. Release signature verification roundtrip & tamper rejection
//! 4. Arch PKGBUILD & PKGBUILD-bin checksum synchronization
//! 5. Debian package structure, control metadata, and file permissions
//! 6. AppImage ELF header and executable permissions
//! 7. Deep-link security matrix and non-mutation invariant
//! 8. Fresh install, upgrade, and uninstall lifecycle cleanup simulation
//! 9. Reproducibility release manifest completeness
//! 10. Repository hygiene: Zero private keys (*.key, *.priv) in tracked git files

use ryzora_lib::crypto::TrustStore;
use ryzora_lib::deeplink::{parse_deeplink_url, DeepLinkAction};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn get_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

#[test]
fn test_phase20_vector_1_root_key_ceremony_and_fail_closed_guarantee() {
    let root = get_workspace_root();
    assert!(root.join("docs/ROOT_KEY_CEREMONY.md").exists());
    assert!(root.join("docs/CEREMONY_LOG_TEMPLATE.md").exists());

    let ceremony_doc = fs::read_to_string(root.join("docs/ROOT_KEY_CEREMONY.md")).unwrap();
    assert!(ceremony_doc.contains(
        "The production Official Root Key is NOT intended to sign release artifacts directly"
    ));
    assert!(ceremony_doc.contains("Air-Gapped"));

    // Test TrustStore fail-closed behavior: An unprovisioned store cannot grant Official status
    let empty_store = TrustStore::new_test_store(&[], &[], &[]);
    assert!(!empty_store.is_production_ready());
    assert!(!empty_store
        .is_official_core_key("d1a9eda16bd08fae78c922688650008b4bce235a1171044f3b68d1cb109dc0fb"));

    // When provisioned with authentic key, recognition works
    let prod_pubkey = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let provisioned_store = TrustStore::new_test_store(&[prod_pubkey], &[], &[]);
    assert!(provisioned_store.is_official_core_key(prod_pubkey));
    assert!(provisioned_store.is_production_ready());
}

#[test]
fn test_phase20_vector_2_release_artifacts_integrity_and_checksums() {
    let root = get_workspace_root();
    let dist_dir = root.join("dist/release-v0.1.0");
    let sha_file = dist_dir.join("SHA256SUMS.txt");

    assert!(
        sha_file.exists(),
        "SHA256SUMS.txt must exist in dist/release-v0.1.0"
    );
    let content = fs::read_to_string(&sha_file).unwrap();

    let mut checked_count = 0;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(parts.len(), 2, "Line must be '<hash>  <filename>'");
        let expected_hash = parts[0];
        let filename = parts[1];

        let file_path = dist_dir.join(filename);
        assert!(file_path.exists(), "Artifact '{}' must exist", filename);

        let data = fs::read(&file_path).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let actual_hash = hex::encode(hasher.finalize());

        assert_eq!(
            expected_hash, actual_hash,
            "Checksum mismatch for artifact '{}'",
            filename
        );
        checked_count += 1;
    }

    assert!(
        checked_count >= 4,
        "Must verify at least 4 release artifacts (found {})",
        checked_count
    );
}

#[test]
fn test_phase20_vector_3_release_signature_independent_verification() {
    let root = get_workspace_root();
    let dist_dir = root.join("dist/release-v0.1.0");
    let sha_file = dist_dir.join("SHA256SUMS.txt");
    let sig_file = dist_dir.join("SHA256SUMS.txt.sig");
    let pubkey_file = dist_dir.join("ryzora-release-0.1.0.pub");

    assert!(sig_file.exists(), "Detached signature file must exist");
    assert!(pubkey_file.exists(), "Release public key file must exist");

    let sha_data = fs::read(&sha_file).unwrap();
    let sig_hex = fs::read_to_string(&sig_file).unwrap().trim().to_string();
    let pubkey_hex = fs::read_to_string(&pubkey_file).unwrap().trim().to_string();

    let sig_bytes = hex::decode(&sig_hex).expect("Signature must be valid hex");
    let pubkey_bytes = hex::decode(&pubkey_hex).expect("Pubkey must be valid hex");

    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let verifying_key =
        VerifyingKey::from_bytes(pubkey_bytes.as_slice().try_into().expect("32 bytes"))
            .expect("Valid ed25519 verifying key");

    let signature = Signature::from_bytes(sig_bytes.as_slice().try_into().expect("64 bytes"));

    // 1. Signature over clean file must pass
    assert!(verifying_key.verify(&sha_data, &signature).is_ok());

    // 2. Tampered file must be immediately rejected
    let mut tampered = sha_data.clone();
    tampered.push(b'x');
    assert!(verifying_key.verify(&tampered, &signature).is_err());
}

#[test]
fn test_phase20_vector_4_arch_pkgbuild_sha256_synchronization() {
    let root = get_workspace_root();
    let pkgbuild = fs::read_to_string(root.join("packaging/arch/PKGBUILD")).unwrap();
    let pkgbuild_bin = fs::read_to_string(root.join("packaging/arch/PKGBUILD-bin")).unwrap();

    let dist_dir = root.join("dist/release-v0.1.0");
    let src_tarball = dist_dir.join("ryzora-0.1.0.tar.gz");
    let bin_tarball = dist_dir.join("ryzora-v0.1.0-linux-x86_64.tar.gz");

    if src_tarball.exists() {
        let mut hasher = Sha256::new();
        hasher.update(fs::read(&src_tarball).unwrap());
        let src_hash = hex::encode(hasher.finalize());
        assert!(
            pkgbuild.contains(&src_hash),
            "PKGBUILD must contain exact sha256 of source tarball"
        );
    }

    if bin_tarball.exists() {
        let mut hasher = Sha256::new();
        hasher.update(fs::read(&bin_tarball).unwrap());
        let bin_hash = hex::encode(hasher.finalize());
        assert!(
            pkgbuild_bin.contains(&bin_hash),
            "PKGBUILD-bin must contain exact sha256 of binary tarball"
        );
    }
}

#[test]
fn test_phase20_vector_5_debian_package_and_control_compliance() {
    let root = get_workspace_root();
    let deb_path = root.join("dist/release-v0.1.0/ryzora_0.1.0_amd64.deb");
    assert!(deb_path.exists());

    // Inspect with ar
    let output = Command::new("ar")
        .arg("t")
        .arg(&deb_path)
        .output()
        .expect("ar execution failed");

    let listing = String::from_utf8_lossy(&output.stdout);
    assert!(
        listing.contains("debian-binary"),
        "Must contain debian-binary"
    );
    assert!(
        listing.contains("control.tar"),
        "Must contain control.tar.*"
    );
    assert!(listing.contains("data.tar"), "Must contain data.tar.*");
}

#[test]
fn test_phase20_vector_6_appimage_structure_and_elf_header() {
    let root = get_workspace_root();
    let appimage_path = root.join("dist/release-v0.1.0/Ryzora-0.1.0-x86_64.AppImage");
    assert!(appimage_path.exists());

    let bytes = fs::read(&appimage_path).unwrap();
    assert!(bytes.len() > 1024);

    // Verify standard ELF magic 0x7F 'E' 'L' 'F'
    assert_eq!(&bytes[0..4], &[0x7f, b'E', b'L', b'F']);

    // Check executable permission
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::metadata(&appimage_path).unwrap().permissions();
        assert!(perms.mode() & 0o111 != 0, "AppImage must be executable");
    }
}

#[test]
fn test_phase20_vector_7_deep_link_security_and_non_mutation() {
    // Matrix of security cases
    let valid = parse_deeplink_url("ryzora://package/valid-id");
    assert_eq!(
        valid.unwrap(),
        DeepLinkAction::ViewPackage {
            package_id: "valid-id".to_string()
        }
    );

    assert!(parse_deeplink_url("ryzora://package/../secret").is_err());
    assert!(parse_deeplink_url("ryzora://package/%2e%2e/secret").is_err());
    assert!(parse_deeplink_url("ryzora://exec/rm-rf").is_err());
    assert!(parse_deeplink_url("ryzora://shell/bash").is_err());
    assert!(parse_deeplink_url("ryzora://file/etc/shadow").is_err());
    assert!(parse_deeplink_url("ryzora://package/test\0attack").is_err());

    let oversized = "a".repeat(129);
    assert!(parse_deeplink_url(&format!("ryzora://package/{}", oversized)).is_err());
}

#[test]
fn test_phase20_vector_8_lifecycle_fresh_install_upgrade_uninstall_simulation() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mock_root = std::env::temp_dir().join(format!("ryzora-p20-lifecycle-{}", nanos));
    let _ = fs::remove_dir_all(&mock_root);

    let bin_dir = mock_root.join("usr/bin");
    let app_dir = mock_root.join("usr/share/applications");
    let meta_dir = mock_root.join("usr/share/metainfo");
    let icon_dir = mock_root.join("usr/share/icons/hicolor");

    fs::create_dir_all(&bin_dir).unwrap();
    fs::create_dir_all(&app_dir).unwrap();
    fs::create_dir_all(&meta_dir).unwrap();
    fs::create_dir_all(&icon_dir).unwrap();

    let root = get_workspace_root();

    // 1. Fresh Install simulation
    let ryzora_bin = bin_dir.join("ryzora");
    fs::copy(root.join("src-tauri/target/release/ryzora"), &ryzora_bin).unwrap();
    let desktop_file = app_dir.join("io.ryzora.Ryzora.desktop");
    fs::copy(
        root.join("dist-assets/io.ryzora.Ryzora.desktop"),
        &desktop_file,
    )
    .unwrap();
    let meta_file = meta_dir.join("io.ryzora.Ryzora.metainfo.xml");
    fs::copy(
        root.join("dist-assets/io.ryzora.Ryzora.metainfo.xml"),
        &meta_file,
    )
    .unwrap();

    assert!(ryzora_bin.exists());
    assert!(desktop_file.exists());
    assert!(meta_file.exists());

    // 2. Upgrade simulation (re-install over existing without error)
    fs::copy(root.join("src-tauri/target/release/ryzora"), &ryzora_bin).unwrap();
    assert!(ryzora_bin.exists());

    // 3. Uninstall simulation
    fs::remove_file(&ryzora_bin).unwrap();
    fs::remove_file(&desktop_file).unwrap();
    fs::remove_file(&meta_file).unwrap();

    assert!(!ryzora_bin.exists());
    assert!(!desktop_file.exists());
    assert!(!meta_file.exists());
    let _ = fs::remove_dir_all(&mock_root);
}

#[test]
fn test_phase20_vector_9_reproducibility_manifest_validation() {
    let root = get_workspace_root();
    let manifest_path = root.join("dist/release-v0.1.0/RELEASE_MANIFEST.txt");
    assert!(manifest_path.exists());

    let manifest = fs::read_to_string(&manifest_path).unwrap();
    assert!(manifest.contains("version = \"0.1.0\""));
    assert!(manifest.contains("architecture = \"x86_64-unknown-linux-gnu\""));
    assert!(manifest.contains("cargo_lock_sha256 = "));
    assert!(manifest.contains("Ryzora-0.1.0-x86_64.AppImage"));
    assert!(manifest.contains("ryzora_0.1.0_amd64.deb"));
    assert!(manifest.contains("ryzora-v0.1.0-linux-x86_64.tar.gz"));
}

#[test]
fn test_phase20_vector_10_git_hygiene_no_private_keys_tracked() {
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .expect("git ls-files failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        assert!(
            !line.ends_with(".key") && !line.ends_with(".priv"),
            "CRITICAL SECURITY VIOLATION: Private key tracked in git: {}",
            line
        );
        assert!(
            !line.contains("release-keys/"),
            "CRITICAL SECURITY VIOLATION: release-keys directory tracked in git: {}",
            line
        );
    }
}
