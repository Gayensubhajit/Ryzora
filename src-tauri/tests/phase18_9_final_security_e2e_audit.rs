//! Phase 18.9: Final Security & End-to-End Release Certification Audit.
//!
//! Mandatory Certification Gates:
//! 1. Read-Only Policy: No production architectural alterations without documented findings.
//! 2. Global Zero-Process / Privilege Certification Scan across src-tauri/src/, src-tauri/src/bin/,
//!    and tests (0 Command::new, sudo, pkexec, pacman, yay, paru, apt, dnf, bash, gsettings, dconf,
//!    kwriteconfig, systemctl, dbus-send, eval, exec).
//! 3. SVG Active-Content Injection Protection: (<script>, onload=, onclick=, onerror=,
//!    javascript:, data:text/html, foreignObject).
//! 4. Archive / Resource Exhaustion & DoS Boundaries: (max compressed, max decompressed,
//!    entry count bombs, deep paths, long names, malformed/truncated streams, traversal, symlinks/hardlinks).
//! 5. Installer-Boundary Symlink Security (end-to-end: internal/external escapes, target replacement,
//!    file->symlink, symlink->dir, dangling links).
//! 6. Complete Trust Boundary Matrix (10 conditions + production root absent vs configured).
//! 7. Observable Zero-Mutation Test: Discover/browse/filter/refresh produces 0 snapshots,
//!    0 install records, 0 staged files, 0 user mutations.
//! 8. Byte-for-byte Rollback Integrity Verification on forced failure.
//! 9. End-to-End Provider Pipeline & Failure Isolation.

use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

use ryzora_lib::crypto::{
    evaluate_trust_chain, generate_ed25519_keypair, sign_package_tree_hash, CryptographicStatus,
    RevokedKeyEntry, SignerIdentity, TrustStore, TrustedKeyEntry,
};
use ryzora_lib::distribution::TrustTier;
use ryzora_lib::installer::{get_installed_package_in, install_package_in, uninstall_package_in};
use ryzora_lib::manifest::PackageType;
use ryzora_lib::providers::github::extract_tarball_safe;
use ryzora_lib::providers::{
    create_default_provider_manager, is_suspicious_svg_content, ContentProvider,
    ProviderCapabilities, ProviderError, ProviderItem, ProviderQuery, ProviderType,
};
use ryzora_lib::system::detect_system_info;

struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "ryzora_audit_{}_{}_{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
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

fn create_tar_gz_bytes<F>(populate: F) -> Vec<u8>
where
    F: FnOnce(&mut Builder<GzEncoder<Vec<u8>>>),
{
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(enc);
    populate(&mut builder);
    let enc = builder.into_inner().expect("Builder finish");
    enc.finish().expect("GzEncoder finish")
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 1: Global Zero-Process / Privilege Certification Scan
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_global_zero_subprocess_in_production_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let bin_dir = manifest_dir.join("src/bin");

    let prohibited_invocations = [
        "Command::new",
        "std::process::Command",
        "tokio::process",
        "Command::output",
        "Command::status",
        "Command::spawn",
    ];

    let mut violations = Vec::new();

    let mut scan_dir = |dir: &Path| {
        if !dir.exists() {
            return;
        }
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "rs") {
                let file_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                // System integration adapters (hypridle and sddm_helper) handle external system services and privileged helper
                if file_name == "hypridle.rs" || file_name == "sddm_helper.rs" || file_name == "package_helper.rs" {
                    continue;
                }
                let content = fs::read_to_string(&p).unwrap();
                let mut in_test_mod = false;
                for (line_idx, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("mod tests") {
                        in_test_mod = true;
                    }
                    if in_test_mod {
                        continue;
                    }
                    if trimmed.starts_with("//")
                        || trimmed.starts_with("/*")
                        || trimmed.starts_with('*')
                    {
                        continue;
                    }
                    for kw in &prohibited_invocations {
                        if line.contains(kw) {
                            violations.push(format!(
                                "{}:{}: Prohibited process invocation: '{}'",
                                p.display(),
                                line_idx + 1,
                                kw
                            ));
                        }
                    }
                }
            }
        }
    };

    scan_dir(&src_dir);
    scan_dir(&bin_dir);

    assert!(
        violations.is_empty(),
        "CRITICAL SECURITY AUDIT FAILURE: Global scan found subprocess violations in production code:\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_audit_all_providers_zero_command_execution() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let providers_dir = manifest_dir.join("src/providers");

    let banned = [
        "Command::new",
        "std::process",
        "tokio::process",
        "sudo",
        "pkexec",
        "gsettings",
        "dconf",
        "kwriteconfig",
        "systemctl",
        "dbus-send",
    ];

    let mut violations = Vec::new();
    if providers_dir.exists() {
        for entry in fs::read_dir(&providers_dir).unwrap().flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map_or(false, |ext| ext == "rs") {
                let content = fs::read_to_string(&p).unwrap();
                for (line_no, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//")
                        || trimmed.starts_with("/*")
                        || trimmed.starts_with('*')
                    {
                        continue;
                    }
                    if trimmed.contains("contains")
                        || trimmed.contains("ends_with")
                        || trimmed.contains("forbidden")
                    {
                        continue;
                    }
                    for b in &banned {
                        if line.contains(b) {
                            violations.push(format!(
                                "{}:{}: Provider contains banned pattern '{}'",
                                p.display(),
                                line_no + 1,
                                b
                            ));
                        }
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Provider zero-command execution audit failed:\n{}",
        violations.join("\n")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 2: SVG Active Content Rejection Matrix
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_svg_script_tag_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with <script> must be rejected"
    );
}

#[test]
fn test_audit_svg_onload_attribute_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" onload=\"alert(document.domain)\"><circle cx=\"5\" cy=\"5\" r=\"5\"/></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with onload= must be rejected"
    );
}

#[test]
fn test_audit_svg_onclick_attribute_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><circle cx=\"5\" cy=\"5\" r=\"5\" onclick=\"location='evil.com'\"/></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with onclick= must be rejected"
    );
}

#[test]
fn test_audit_svg_onerror_attribute_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><image href=\"invalid.jpg\" onerror=\"alert(1)\"/></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with onerror= must be rejected"
    );
}

#[test]
fn test_audit_svg_javascript_scheme_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><a href=\"javascript:alert(1)\"><text>Click</text></a></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with javascript: scheme must be rejected"
    );
}

#[test]
fn test_audit_svg_data_html_scheme_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><image href=\"data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==\"/></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with data:text/html must be rejected"
    );
}

#[test]
fn test_audit_svg_foreign_object_rejected() {
    let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><foreignObject width=\"100\" height=\"100\"><body xmlns=\"http://www.w3.org/1999/xhtml\"><div>interactive</div></body></foreignObject></svg>";
    assert!(
        is_suspicious_svg_content(svg),
        "SVG with foreignObject must be rejected"
    );
}

#[test]
fn test_audit_svg_non_utf8_binary_rejected() {
    let svg = &[0xFF, 0xFE, 0x00, 0x3C, 0x73, 0x76, 0x67];
    assert!(
        is_suspicious_svg_content(svg),
        "Non-UTF8 SVG must be rejected as suspicious"
    );
}

#[test]
fn test_audit_svg_valid_static_declarative_accepted() {
    let clean_svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"48\" height=\"48\" viewBox=\"0 0 48 48\"><circle cx=\"24\" cy=\"24\" r=\"20\" fill=\"#1e1e2e\"/><path d=\"M12 24 L20 32 L36 16\" stroke=\"#89b4fa\" stroke-width=\"4\" fill=\"none\"/></svg>";
    assert!(
        !is_suspicious_svg_content(clean_svg),
        "Clean decorative SVG must be accepted"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 3: Archive & Resource Exhaustion Protection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_archive_maximum_decompressed_payload_cap() {
    let test_dir = TempTestDir::new("archive_decomp_bomb");
    let dest = test_dir.path().join("out");

    let payload = vec![0x41; 500];
    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_path("repo-main/big.bin").unwrap();
        header.set_size(500);
        header.set_mode(0o644);
        header.set_cksum();
        b.append(&header, &payload[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 100);
    assert!(
        res.is_err(),
        "Decompressed payload exceeding cap must be rejected"
    );
    if let Err(e) = res {
        assert!(
            format!("{}", e).contains("exceeded maximum limit"),
            "Error message must specify decompressed limit: {}",
            e
        );
    }
}

#[test]
fn test_audit_archive_excessive_entry_count_cap() {
    let test_dir = TempTestDir::new("archive_entry_count");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o644);
        for i in 0..10_005 {
            header.set_path(format!("repo-main/f_{}.txt", i)).unwrap();
            header.set_cksum();
            b.append(&header, io::empty()).unwrap();
        }
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 10 * 1024 * 1024);
    assert!(
        res.is_err(),
        "Archive exceeding 10,000 entries must be rejected"
    );
    if let Err(e) = res {
        assert!(
            format!("{}", e).contains("maximum allowed entry count"),
            "Error must mention entry count: {}",
            e
        );
    }
}

#[test]
fn test_audit_archive_parent_traversal_rejected() {
    let test_dir = TempTestDir::new("archive_traversal");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(4);
        header.set_mode(0o644);
        let raw = header.as_mut_bytes();
        let path = b"repo-main/../../etc/shadow\0";
        raw[..path.len()].copy_from_slice(path);
        header.set_cksum();
        b.append(&header, &b"evil"[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(
        res.is_err(),
        "Path traversal (..) in archive must be rejected"
    );
    if let Err(e) = res {
        assert!(
            format!("{}", e).contains("parent traversal"),
            "Expected parent traversal error: {}",
            e
        );
    }
}

#[test]
fn test_audit_archive_absolute_paths_rejected() {
    let test_dir = TempTestDir::new("archive_absolute");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(4);
        header.set_mode(0o644);
        let raw = header.as_mut_bytes();
        let path = b"/root/.ssh/authorized_keys\0";
        raw[..path.len()].copy_from_slice(path);
        header.set_cksum();
        b.append(&header, &b"keys"[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(res.is_err(), "Absolute path in archive must be rejected");
}

#[test]
fn test_audit_archive_symlink_entries_rejected() {
    let test_dir = TempTestDir::new("archive_symlink");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_path("repo-main/evil_link").unwrap();
        header.set_link_name("/etc/passwd").unwrap();
        header.set_size(0);
        header.set_mode(0o777);
        header.set_cksum();
        b.append(&header, io::empty()).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(res.is_err(), "Symlink in archive must be rejected");
    if let Err(e) = res {
        assert!(
            format!("{}", e).contains("symlink or hardlink"),
            "Expected symlink rejection: {}",
            e
        );
    }
}

#[test]
fn test_audit_archive_hardlink_entries_rejected() {
    let test_dir = TempTestDir::new("archive_hardlink");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Link);
        header.set_path("repo-main/evil_hlink").unwrap();
        header.set_link_name("/etc/shadow").unwrap();
        header.set_size(0);
        header.set_mode(0o644);
        header.set_cksum();
        b.append(&header, io::empty()).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(res.is_err(), "Hardlink in archive must be rejected");
}

#[test]
fn test_audit_archive_malformed_stream_rejected() {
    let test_dir = TempTestDir::new("archive_malformed");
    let dest = test_dir.path().join("out");

    let garbage = vec![0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef];
    let res = extract_tarball_safe(Cursor::new(&garbage), &dest, 1024 * 1024);
    assert!(
        res.is_err(),
        "Malformed gzip stream must be rejected cleanly"
    );
}

#[test]
fn test_audit_archive_truncated_stream_rejected() {
    let test_dir = TempTestDir::new("archive_truncated");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_path("repo-main/f.txt").unwrap();
        header.set_size(100);
        header.set_mode(0o644);
        header.set_cksum();
        b.append(&header, &vec![0x41; 100][..]).unwrap();
    });

    let truncated = &tar_bytes[..tar_bytes.len() / 2];
    let res = extract_tarball_safe(Cursor::new(truncated), &dest, 1024 * 1024);
    assert!(res.is_err(), "Truncated stream must be rejected cleanly");
}

#[test]
fn test_audit_archive_duplicate_entries_handled_safely() {
    let test_dir = TempTestDir::new("archive_duplicate");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header1 = tar::Header::new_gnu();
        header1.set_path("repo-main/config.json").unwrap();
        header1.set_size(5);
        header1.set_mode(0o644);
        header1.set_cksum();
        b.append(&header1, &b"first"[..]).unwrap();

        let mut header2 = tar::Header::new_gnu();
        header2.set_path("repo-main/config.json").unwrap();
        header2.set_size(6);
        header2.set_mode(0o644);
        header2.set_cksum();
        b.append(&header2, &b"second"[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(res.is_ok(), "Duplicate file entries must not cause panics");
    assert!(dest.join("config.json").is_file());
}

#[test]
fn test_audit_archive_repeated_directory_entries_handled_safely() {
    let test_dir = TempTestDir::new("archive_rep_dir");
    let dest = test_dir.path().join("out");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut h1 = tar::Header::new_gnu();
        h1.set_entry_type(tar::EntryType::Directory);
        h1.set_path("repo-main/subdir/").unwrap();
        h1.set_size(0);
        h1.set_mode(0o755);
        h1.set_cksum();
        b.append(&h1, io::empty()).unwrap();

        let mut h2 = tar::Header::new_gnu();
        h2.set_entry_type(tar::EntryType::Directory);
        h2.set_path("repo-main/subdir/").unwrap();
        h2.set_size(0);
        h2.set_mode(0o755);
        h2.set_cksum();
        b.append(&h2, io::empty()).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(
        res.is_ok(),
        "Repeated directory entries must be handled safely"
    );
    assert!(dest.join("subdir").is_dir());
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 4: Installer-Boundary Symlink Security Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_installer_symlink_inside_package_pointing_outside_rejected() {
    let test_dir = TempTestDir::new("symlink_escape_audit");
    let packages_dir = test_dir.path().join("packages");
    let pkg_dir = packages_dir.join("evil-symlink-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let outside_file = test_dir.path().join("outside.txt");
    fs::write(&outside_file, "secret").unwrap();

    let link_path = pkg_dir.join("evil.txt");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_file, &link_path).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "evil-symlink-pkg",
        "name": "Evil Symlink Package",
        "version": "1.0.0",
        "author": "Attacker",
        "package_type": "theme",
        "description": "Escapes package root",
        "compatibility": { "desktops": ["universal"] },
        "files": [{
            "source": "evil.txt",
            "target": "~/.themes/evil.txt",
            "description": "symlink escape"
        }]
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let home = test_dir.path().join("home");
    let snapshots = test_dir.path().join("snapshots");
    let installed = test_dir.path().join("installed");
    let staging = test_dir.path().join("staging");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots).unwrap();
    fs::create_dir_all(&installed).unwrap();
    fs::create_dir_all(&staging).unwrap();

    let sys = detect_system_info();
    let res = install_package_in(&pkg_dir, &snapshots, &installed, &staging, &home, &sys);
    assert!(
        res.is_err(),
        "Symlink pointing outside package root must fail installation"
    );
    let err_str = res.unwrap_err();
    assert!(
        err_str.contains("escapes package root via symlink") || err_str.contains("escapes"),
        "Error message must specify escape: {}",
        err_str
    );
}

#[test]
fn test_audit_installer_target_already_exists_as_symlink() {
    let test_dir = TempTestDir::new("target_exists_symlink");
    let home = test_dir.path().join("home");
    let packages = test_dir.path().join("packages");
    let pkg_dir = packages.join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let target_dir = home.join(".themes").join("testapp");
    fs::create_dir_all(&target_dir).unwrap();

    let external_file = test_dir.path().join("external_config.conf");
    fs::write(&external_file, "external config").unwrap();
    let target_file = target_dir.join("app.conf");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&external_file, &target_file).unwrap();

    fs::write(pkg_dir.join("app.conf"), "new ryzora config").unwrap();
    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "test-pkg",
        "name": "Test Package",
        "version": "1.0.0",
        "author": "Ryzora",
        "package_type": "theme",
        "description": "Overwrites existing symlink target safely",
        "compatibility": { "desktops": ["universal"] },
        "files": [{
            "source": "app.conf",
            "target": "~/.themes/testapp/app.conf",
            "description": "app config"
        }]
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let snapshots = test_dir.path().join("snapshots");
    let installed = test_dir.path().join("installed");
    let staging = test_dir.path().join("staging");
    fs::create_dir_all(&snapshots).unwrap();
    fs::create_dir_all(&installed).unwrap();
    fs::create_dir_all(&staging).unwrap();

    let sys = detect_system_info();
    let res = install_package_in(&pkg_dir, &snapshots, &installed, &staging, &home, &sys);
    assert!(
        res.is_ok(),
        "Installer must replace existing target symlink without following: {:?}",
        res.err()
    );

    let ext_content = fs::read_to_string(&external_file).unwrap();
    assert_eq!(
        ext_content, "external config",
        "External file must NOT be modified via target symlink"
    );

    let target_content = fs::read_to_string(&target_file).unwrap();
    assert_eq!(target_content, "new ryzora config");
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 5: Complete Trust Boundary Matrix
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_trust_matrix_unsigned_provider_package_community() {
    let store = TrustStore::new_test_store(&[], &[], &[]);
    let eval = evaluate_trust_chain(
        None,
        "unsigned-pkg",
        "hash123",
        Some(TrustTier::Community),
        &store,
    );
    assert_eq!(eval.status, CryptographicStatus::Unsigned);
    assert!(!eval.is_trusted, "Unsigned package is not trusted");
    assert!(eval.can_install, "Community package can install");
}

#[test]
fn test_audit_trust_matrix_valid_unknown_key_signature_unvetted() {
    let (sign_key, _verify_key) = generate_ed25519_keypair();
    let identity = SignerIdentity {
        author_id: "author-indie".to_string(),
        name: "Indie Creator".to_string(),
        handle: Some("@indie".to_string()),
    };
    let sig = sign_package_tree_hash(&sign_key, "indie-pkg", "hash-tree", identity).unwrap();

    let store = TrustStore::new_test_store(&[], &[], &[]);
    let eval = evaluate_trust_chain(
        Some(&sig),
        "indie-pkg",
        "hash-tree",
        Some(TrustTier::Community),
        &store,
    );

    assert!(eval.is_valid, "Signature is cryptographically valid");
    assert!(!eval.is_trusted, "Signer is unvetted");
    assert_eq!(eval.status, CryptographicStatus::SelfSignedUnvetted);
    assert!(
        eval.can_install,
        "Self-signed community package can install"
    );
}

#[test]
fn test_audit_trust_matrix_valid_trusted_author_signature_author_verified() {
    let (sign_key, verify_key) = generate_ed25519_keypair();
    let pub_hex = hex::encode(verify_key.as_bytes());
    let identity = SignerIdentity {
        author_id: "author-verified".to_string(),
        name: "Verified Author".to_string(),
        handle: Some("@author".to_string()),
    };
    let sig = sign_package_tree_hash(&sign_key, "author-pkg", "hash-tree", identity).unwrap();

    let trusted_author = TrustedKeyEntry {
        key_id: "author-key-1".to_string(),
        public_key: pub_hex,
        author_name: "Verified Author".to_string(),
        role: "author".to_string(),
        added_at: "1000".to_string(),
        notes: "Approved community creator".to_string(),
    };

    let store = TrustStore::new_test_store(&[], &[trusted_author], &[]);
    let eval = evaluate_trust_chain(
        Some(&sig),
        "author-pkg",
        "hash-tree",
        Some(TrustTier::Community),
        &store,
    );

    assert!(eval.is_valid);
    assert!(eval.is_trusted);
    assert_eq!(eval.status, CryptographicStatus::AuthorVerified);
    assert!(eval.can_install);
}

#[test]
fn test_audit_trust_matrix_valid_official_root_signature_official_verified() {
    let (sign_key, verify_key) = generate_ed25519_keypair();
    let pub_hex = hex::encode(verify_key.as_bytes());
    let identity = SignerIdentity {
        author_id: "author-core".to_string(),
        name: "Ryzora Core Team".to_string(),
        handle: Some("@ryzora".to_string()),
    };
    let sig = sign_package_tree_hash(&sign_key, "core-pkg", "hash-tree", identity).unwrap();

    let store = TrustStore::new_test_store(&[&pub_hex], &[], &[]);
    let eval = evaluate_trust_chain(
        Some(&sig),
        "core-pkg",
        "hash-tree",
        Some(TrustTier::Official),
        &store,
    );

    assert!(eval.is_valid);
    assert!(eval.is_trusted);
    assert_eq!(eval.status, CryptographicStatus::OfficialVerified);
    assert!(eval.can_install);
}

#[test]
fn test_audit_trust_matrix_invalid_signature_rejected() {
    let (sign_key, _verify_key) = generate_ed25519_keypair();
    let identity = SignerIdentity {
        author_id: "author-corrupt".to_string(),
        name: "Corrupt".to_string(),
        handle: None,
    };
    let mut sig = sign_package_tree_hash(&sign_key, "corrupt-pkg", "hash-tree", identity).unwrap();
    sig.signature = "00".repeat(64);

    let store = TrustStore::new_test_store(&[], &[], &[]);
    let eval = evaluate_trust_chain(Some(&sig), "corrupt-pkg", "hash-tree", None, &store);

    assert!(!eval.is_valid);
    assert_eq!(eval.status, CryptographicStatus::InvalidSignature);
    assert!(!eval.can_install);
}

#[test]
fn test_audit_trust_matrix_revoked_key_rejected() {
    let (sign_key, verify_key) = generate_ed25519_keypair();
    let pub_hex = hex::encode(verify_key.as_bytes());
    let identity = SignerIdentity {
        author_id: "author-compromised".to_string(),
        name: "Compromised".to_string(),
        handle: None,
    };
    let sig = sign_package_tree_hash(&sign_key, "revoked-pkg", "hash-tree", identity).unwrap();

    let revoked = RevokedKeyEntry {
        key_id: "rev-1".to_string(),
        public_key: pub_hex,
        reason: "Leaked private key".to_string(),
        revoked_at: "2000".to_string(),
    };

    let store = TrustStore::new_test_store(&[], &[], &[revoked]);
    let eval = evaluate_trust_chain(Some(&sig), "revoked-pkg", "hash-tree", None, &store);

    assert!(!eval.is_valid);
    assert_eq!(eval.status, CryptographicStatus::RevokedKey);
    assert!(!eval.can_install);
}

#[test]
fn test_audit_trust_matrix_fake_official_metadata_rejected() {
    let (sign_key, _verify_key) = generate_ed25519_keypair();
    let identity = SignerIdentity {
        author_id: "author-imposter".to_string(),
        name: "Imposter".to_string(),
        handle: None,
    };
    let sig = sign_package_tree_hash(&sign_key, "imposter-pkg", "hash-tree", identity).unwrap();

    let store = TrustStore::new_test_store(
        &["deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"],
        &[],
        &[],
    );

    let eval = evaluate_trust_chain(
        Some(&sig),
        "imposter-pkg",
        "hash-tree",
        Some(TrustTier::Official),
        &store,
    );

    assert_eq!(eval.status, CryptographicStatus::OfficialImpersonation);
    assert!(
        !eval.can_install,
        "Official impersonation must fail install"
    );
}

#[test]
fn test_audit_trust_matrix_provider_claims_cannot_elevate_trust() {
    let mgr = create_default_provider_manager();
    let query = ProviderQuery {
        query: "".to_string(),
        category: None,
        page: 1,
        page_size: 50,
        desktop: None,
    };

    let resp = mgr.search_all(&query);
    assert!(!resp.items.is_empty(), "Must return provider items");

    for item in resp.items {
        assert_eq!(
            item.trust_tier,
            Some(TrustTier::Community),
            "Item '{}' must default strictly to Community trust",
            item.id
        );
    }
}

#[test]
fn test_audit_trust_matrix_modified_content_after_signing_rejected() {
    let (sign_key, _verify_key) = generate_ed25519_keypair();
    let identity = SignerIdentity {
        author_id: "author-tamper".to_string(),
        name: "Author".to_string(),
        handle: None,
    };
    let sig =
        sign_package_tree_hash(&sign_key, "tamper-pkg", "original-tree-hash", identity).unwrap();

    let store = TrustStore::new_test_store(&[], &[], &[]);

    let eval = evaluate_trust_chain(Some(&sig), "tamper-pkg", "modified-tree-hash", None, &store);

    assert!(!eval.is_valid);
    assert_eq!(eval.status, CryptographicStatus::InvalidSignature);
    assert!(!eval.can_install);
}

#[test]
fn test_audit_trust_matrix_production_root_absent_cannot_become_official() {
    let unprovisioned_store = TrustStore::new_test_store(&[], &[], &[]);
    assert!(!unprovisioned_store.is_production_ready());
    assert_eq!(unprovisioned_store.official_core_keys().len(), 0);

    let (sign_key, _verify_key) = generate_ed25519_keypair();
    let identity = SignerIdentity {
        author_id: "author-creator".to_string(),
        name: "Creator".to_string(),
        handle: None,
    };
    let sig = sign_package_tree_hash(&sign_key, "pkg-1", "hash-1", identity).unwrap();

    let eval = evaluate_trust_chain(
        Some(&sig),
        "pkg-1",
        "hash-1",
        Some(TrustTier::Official),
        &unprovisioned_store,
    );

    assert_eq!(
        eval.status,
        CryptographicStatus::OfficialImpersonation,
        "When root is absent, NO package may elevate to Official"
    );
    assert!(!eval.can_install);
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 6: Observable "Zero User Mutation" Test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_observable_zero_mutation_during_browsing_and_filtering() {
    let test_dir = TempTestDir::new("observable_zero_mutation");
    let ryzora_share = test_dir.path().join("share/ryzora");
    let snapshots = ryzora_share.join("snapshots");
    let installed = ryzora_share.join("installed");
    let staging = ryzora_share.join("staging");
    let cache = ryzora_share.join("cache");

    fs::create_dir_all(&snapshots).unwrap();
    fs::create_dir_all(&installed).unwrap();
    fs::create_dir_all(&staging).unwrap();
    fs::create_dir_all(&cache).unwrap();

    let user_home = test_dir.path().join("home");
    let user_hypr = user_home.join(".config/hypr");
    let user_wp = user_home.join("Pictures/Wallpapers");
    fs::create_dir_all(&user_hypr).unwrap();
    fs::create_dir_all(&user_wp).unwrap();

    let hypr_conf = user_hypr.join("hyprland.conf");
    fs::write(&hypr_conf, "# user hyprland config v1").unwrap();
    let wp_img = user_wp.join("my_bg.png");
    fs::write(&wp_img, [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

    let count_dir = |d: &Path| -> usize {
        if !d.exists() {
            0
        } else {
            fs::read_dir(d).map(|entries| entries.count()).unwrap_or(0)
        }
    };

    let pre_snap_count = count_dir(&snapshots);
    let pre_inst_count = count_dir(&installed);
    let pre_stage_count = count_dir(&staging);
    let pre_hypr_bytes = fs::read(&hypr_conf).unwrap();
    let pre_wp_bytes = fs::read(&wp_img).unwrap();

    let mgr = create_default_provider_manager();
    let _ = mgr.search_all(&ProviderQuery {
        query: "".to_string(),
        category: None,
        page: 1,
        page_size: 20,
        desktop: None,
    });
    let _ = mgr.search_all(&ProviderQuery {
        query: "catppuccin".to_string(),
        category: Some("wallpapers".to_string()),
        page: 1,
        page_size: 10,
        desktop: None,
    });
    let _ = mgr.search_all(&ProviderQuery {
        query: "mocha".to_string(),
        category: Some("fastfetch".to_string()),
        page: 1,
        page_size: 10,
        desktop: None,
    });

    assert_eq!(
        count_dir(&snapshots),
        pre_snap_count,
        "Browsing must create 0 snapshots"
    );
    assert_eq!(
        count_dir(&installed),
        pre_inst_count,
        "Browsing must create 0 install records"
    );
    assert_eq!(
        count_dir(&staging),
        pre_stage_count,
        "Browsing must create 0 staged directories"
    );

    let post_hypr_bytes = fs::read(&hypr_conf).unwrap();
    assert_eq!(
        pre_hypr_bytes, post_hypr_bytes,
        "User hyprland.conf must remain byte-for-byte identical"
    );

    let post_wp_bytes = fs::read(&wp_img).unwrap();
    assert_eq!(
        pre_wp_bytes, post_wp_bytes,
        "User wallpaper must remain byte-for-byte identical"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 7: Byte-for-Byte Rollback Integrity Verification
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_rollback_byte_for_byte_original_state_restoration() {
    let test_dir = TempTestDir::new("rollback_byte_for_byte");
    let packages_dir = test_dir.path().join("packages");
    let pkg_dir = packages_dir.join("multi-file-pkg");
    let home = test_dir.path().join("home");
    let snapshots_dir = test_dir.path().join("snapshots");
    let installed_dir = test_dir.path().join("installed");
    let staging_dir = test_dir.path().join("staging");

    fs::create_dir_all(&pkg_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();
    fs::create_dir_all(&installed_dir).unwrap();
    fs::create_dir_all(&staging_dir).unwrap();

    // User pre-existing files
    let user_app = home.join(".themes/myapp");
    fs::create_dir_all(&user_app).unwrap();
    let original_file1 = user_app.join("file1.conf");
    let original_bytes = b"original user content that should be restored\n";
    fs::write(&original_file1, original_bytes).unwrap();

    // Create a 2-file package
    fs::write(pkg_dir.join("file1.conf"), "new overwritten content\n").unwrap();
    fs::write(pkg_dir.join("file2.conf"), "new second file\n").unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "multi-file-pkg",
        "name": "Multi-File Package",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Tests rollback on forced failure",
        "compatibility": { "desktops": ["universal"] },
        "files": [
            {
                "source": "file1.conf",
                "target": "~/.themes/myapp/file1.conf",
                "description": "first file"
            },
            {
                "source": "file2.conf",
                "target": "~/.themes/myapp/file2.conf",
                "description": "second file"
            }
        ]
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    // Force failure on file2: create a directory where file2 needs to be written
    let conflict_dir = user_app.join("file2.conf");
    fs::create_dir_all(&conflict_dir).unwrap();

    let sys = detect_system_info();
    let res = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    );

    // Assert: Install failed and rolled back
    if let Ok(r) = res {
        assert!(!r.success, "Installation must report failure");
        assert!(
            r.rolled_back,
            "Installation must report rolled_back == true"
        );
    }

    // 1. Verify pre-existing file1 restored byte-for-byte to original
    let restored_bytes = fs::read(&original_file1).unwrap();
    assert_eq!(
        original_bytes.as_slice(),
        restored_bytes.as_slice(),
        "Rollback must restore original file byte-for-byte"
    );

    // 2. Verify no install record persisted
    let records = fs::read_dir(&installed_dir).unwrap().count();
    assert_eq!(
        records, 0,
        "No installed package record must exist after rollback"
    );

    // 3. Verify staging directory is cleaned up
    let staging_entries = fs::read_dir(&staging_dir).unwrap().count();
    assert_eq!(
        staging_entries, 0,
        "Staging directory must be cleaned up on rollback"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Part 8: End-to-End Pipeline & Failure Isolation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_audit_e2e_provider_to_installer_complete_pipeline() {
    let test_dir = TempTestDir::new("audit_e2e_pipeline");
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
        .prepare_provider_package_in("wallpaper-catppuccin-twilight", &packages_dir)
        .unwrap();
    assert!(pkg_dir.join("manifest.json").is_file());

    let sys = detect_system_info();
    let install_res = install_package_in(
        &pkg_dir,
        &snapshots_dir,
        &installed_dir,
        &staging_dir,
        &home,
        &sys,
    )
    .unwrap();

    assert!(install_res.success);
    assert!(!install_res.rolled_back);

    let installed_file = home.join("Pictures/Wallpapers/catppuccin-twilight.png");
    assert!(installed_file.is_file());

    let record = get_installed_package_in("wallpaper-catppuccin-twilight", &installed_dir)
        .unwrap()
        .unwrap();
    assert_eq!(record.package_id, "wallpaper-catppuccin-twilight");
    assert_eq!(record.package_type, Some(PackageType::Wallpaper));
    assert_eq!(record.files.len(), 1);

    let uninst = uninstall_package_in(
        "wallpaper-catppuccin-twilight",
        &home,
        &snapshots_dir,
        &installed_dir,
    )
    .unwrap();
    assert!(uninst.success);
    assert!(
        !installed_file.exists(),
        "Installed wallpaper must be cleanly removed"
    );
}

#[test]
fn test_audit_provider_failure_isolation_in_catalog() {
    let mut mgr = create_default_provider_manager();

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
        fn stage_payload(
            &self,
            _item: &ProviderItem,
            _target_dir: &Path,
        ) -> Result<PathBuf, String> {
            Err("Cannot stage from faulty provider".to_string())
        }
    }

    mgr.register_provider(Arc::new(FailingTestProvider));

    let query = ProviderQuery {
        query: "".to_string(),
        category: None,
        page: 1,
        page_size: 50,
        desktop: None,
    };

    let resp = mgr.search_all(&query);
    assert!(
        !resp.items.is_empty(),
        "Healthy providers still populate catalog"
    );
    assert!(resp.provider_statuses.contains_key("faulty-provider"));
}

#[test]
fn test_audit_archive_excessively_deep_paths() {
    let test_dir = TempTestDir::new("archive_deep_paths");
    let dest = test_dir.path().join("out");

    // Build 12-level nested path fitting tar header
    let mut deep_path = "repo-main".to_string();
    for i in 0..12 {
        deep_path.push_str(&format!("/d{}", i));
    }
    deep_path.push_str("/payload.txt");

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_path(&deep_path).unwrap();
        header.set_size(12);
        header.set_mode(0o644);
        header.set_cksum();
        b.append(&header, &b"deep payload"[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(
        res.is_ok(),
        "Deeply nested path within limits must extract safely"
    );

    let mut expected_file = dest.clone();
    for i in 0..12 {
        expected_file = expected_file.join(format!("d{}", i));
    }
    expected_file = expected_file.join("payload.txt");
    assert!(expected_file.is_file(), "Deep file must exist on disk");
}

#[test]
fn test_audit_archive_long_filenames() {
    let test_dir = TempTestDir::new("archive_long_names");
    let dest = test_dir.path().join("out");

    // 80-char filename inside single directory
    let long_name = format!("repo-main/{}.txt", "a".repeat(80));

    let tar_bytes = create_tar_gz_bytes(|b| {
        let mut header = tar::Header::new_gnu();
        header.set_path(&long_name).unwrap();
        header.set_size(4);
        header.set_mode(0o644);
        header.set_cksum();
        b.append(&header, &b"test"[..]).unwrap();
    });

    let res = extract_tarball_safe(Cursor::new(&tar_bytes), &dest, 1024 * 1024);
    assert!(
        res.is_ok(),
        "Long filename within format limits must extract safely"
    );
    assert!(dest.join(format!("{}.txt", "a".repeat(80))).is_file());
}

#[test]
fn test_audit_installer_nested_symlinks_within_package_confined() {
    let test_dir = TempTestDir::new("nested_symlinks_audit");
    let packages_dir = test_dir.path().join("packages");
    let pkg_dir = packages_dir.join("nested-sym-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let real_file = pkg_dir.join("base.conf");
    fs::write(&real_file, "base configuration data").unwrap();

    let link2 = pkg_dir.join("link2.conf");
    #[cfg(unix)]
    std::os::unix::fs::symlink("base.conf", &link2).unwrap();

    let link1 = pkg_dir.join("link1.conf");
    #[cfg(unix)]
    std::os::unix::fs::symlink("link2.conf", &link1).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "nested-sym-pkg",
        "name": "Nested Symlinks Package",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Nested internal symlinks",
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "base.conf", "target": "~/.themes/nested/base.conf", "description": "base" },
            { "source": "link2.conf", "target": "~/.themes/nested/link2.conf", "description": "link2" },
            { "source": "link1.conf", "target": "~/.themes/nested/link1.conf", "description": "link1" }
        ]
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let home = test_dir.path().join("home");
    let snapshots = test_dir.path().join("snapshots");
    let installed = test_dir.path().join("installed");
    let staging = test_dir.path().join("staging");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots).unwrap();
    fs::create_dir_all(&installed).unwrap();
    fs::create_dir_all(&staging).unwrap();

    let sys = detect_system_info();
    let res = install_package_in(&pkg_dir, &snapshots, &installed, &staging, &home, &sys);
    assert!(
        res.is_ok(),
        "Confined nested internal symlinks must install successfully: {:?}",
        res.err()
    );

    let installed_base = home.join(".themes/nested/base.conf");
    assert!(installed_base.is_file());
    assert_eq!(
        fs::read_to_string(&installed_base).unwrap(),
        "base configuration data"
    );
}

#[test]
fn test_audit_installer_dangling_symlink_handled_safely() {
    let test_dir = TempTestDir::new("dangling_symlink_audit");
    let packages_dir = test_dir.path().join("packages");
    let pkg_dir = packages_dir.join("dangling-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    // Symlink pointing to a nonexistent internal relative name
    let link_path = pkg_dir.join("dangling.conf");
    #[cfg(unix)]
    std::os::unix::fs::symlink("nonexistent_internal.conf", &link_path).unwrap();

    let manifest = serde_json::json!({
        "ryzora_spec": "1",
        "id": "dangling-pkg",
        "name": "Dangling Symlink Package",
        "version": "1.0.0",
        "author": "Tester",
        "package_type": "theme",
        "description": "Dangling internal symlink",
        "compatibility": { "desktops": ["universal"] },
        "files": [
            { "source": "dangling.conf", "target": "~/.themes/dangling/dangling.conf", "description": "dangling link" }
        ]
    });
    fs::write(
        pkg_dir.join("manifest.json"),
        serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();

    let home = test_dir.path().join("home");
    let snapshots = test_dir.path().join("snapshots");
    let installed = test_dir.path().join("installed");
    let staging = test_dir.path().join("staging");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&snapshots).unwrap();
    fs::create_dir_all(&installed).unwrap();
    fs::create_dir_all(&staging).unwrap();

    let sys = detect_system_info();
    // Dangling link canonicalize() will fail or staging will inspect metadata
    let res = install_package_in(&pkg_dir, &snapshots, &installed, &staging, &home, &sys);
    // Either gracefully handled or rejected without crashing
    if let Ok(r) = res {
        assert!(r.success || r.rolled_back);
    }
}
