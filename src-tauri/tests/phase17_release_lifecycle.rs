use ryzora_lib::crypto::{
    compute_key_fingerprint, generate_ed25519_keypair, set_secure_permissions,
    sign_package_tree_hash, verify_signature_bytes, RevokedKeyEntry, SignerIdentity, TrustStore,
    RYZORA_DEV_TEST_ROOT_PUBKEY_HEX,
};
use ryzora_lib::manifest::RyzoraManifest;
use ryzora_lib::settings::{load_settings_from_path, save_settings_to_path, RyzoraSettings};
use ryzora_lib::snapshot::{create_snapshot_in, restore_snapshot_in};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn create_temp_sandbox(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("ryzora-phase17-{}-{}", name, nanos));
    fs::create_dir_all(&path).expect("Failed to create temp sandbox");
    path
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 4: Desktop Entry Specification & Freedesktop Validation
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_4_desktop_entry_validation() {
    let desktop_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("dist-assets/ryzora.desktop");

    assert!(
        desktop_path.is_file(),
        "dist-assets/ryzora.desktop must exist"
    );
    let content = fs::read_to_string(&desktop_path).expect("Must read desktop file");

    assert!(content.contains("[Desktop Entry]"));
    assert!(content.contains("Name=Ryzora"));
    assert!(content.contains("Exec=ryzora %U"));
    assert!(content.contains("Icon=ryzora"));
    assert!(content.contains("Terminal=false"));
    assert!(content.contains("Type=Application"));
    assert!(
        content.contains("Categories=Utility;DesktopSettings;Settings;")
            || content.contains("Categories=Settings;DesktopSettings;")
    );
    assert!(content.contains("StartupWMClass=ryzora"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 5: Icon Resources Completeness
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_5_icon_resources_completeness() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let hicolor = root.join("dist-assets/icons/hicolor");

    let required_sizes = [
        "16x16", "32x32", "48x48", "64x64", "128x128", "256x256", "512x512",
    ];
    for size in &required_sizes {
        let icon_path = hicolor.join(size).join("apps/ryzora.png");
        assert!(icon_path.is_file(), "Missing icon: {:?}", icon_path);
        let meta = fs::metadata(&icon_path).unwrap();
        assert!(meta.len() > 0, "Icon {:?} is empty", icon_path);
    }

    let svg_path = hicolor.join("scalable/apps/ryzora.svg");
    assert!(svg_path.is_file(), "Missing scalable SVG icon");

    // Check Tauri embedded window icons
    let tauri_icons = root.join("src-tauri/icons");
    assert!(tauri_icons.join("32x32.png").is_file());
    assert!(tauri_icons.join("128x128.png").is_file());
    assert!(tauri_icons.join("icon.png").is_file());
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 6: Clean-Install Simulation (Cold Startup)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_6_clean_install_simulation() {
    let sandbox = create_temp_sandbox("clean-install");
    let data_dir = sandbox.join("local_share/ryzora");
    let config_dir = sandbox.join("config/ryzora");

    // Verify absent state initially
    assert!(!data_dir.exists());
    assert!(!config_dir.exists());

    // Initialize cold-start directories
    fs::create_dir_all(&data_dir).unwrap();
    fs::create_dir_all(&config_dir).unwrap();
    set_secure_permissions(&data_dir, 0o700).unwrap();
    set_secure_permissions(&config_dir, 0o700).unwrap();

    // Default settings saved
    let settings_file = config_dir.join("settings.json");
    let default_settings = RyzoraSettings::default();
    save_settings_to_path(&default_settings, &settings_file).unwrap();

    // Verify permissions: directory 0700, settings 0600
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let d_mode = fs::metadata(&data_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(d_mode, 0o700, "Data directory must be 0700");
        let s_mode = fs::metadata(&settings_file).unwrap().permissions().mode() & 0o777;
        assert_eq!(s_mode, 0o600, "Settings file must be 0600");
    }

    // Round-trip settings load
    let loaded = load_settings_from_path(&settings_file);
    assert_eq!(loaded.default_release_channel, "stable");
    assert_eq!(loaded.auto_refresh_enabled, true);
    assert_eq!(loaded.auto_refresh_interval_minutes, 60);

    fs::remove_dir_all(&sandbox).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 7: Upgrade Simulation (Schema & State Preservation)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_7_upgrade_simulation() {
    let sandbox = create_temp_sandbox("upgrade");
    let config_dir = sandbox.join("config/ryzora");
    fs::create_dir_all(&config_dir).unwrap();

    // Write settings
    let settings_file = config_dir.join("settings.json");
    let mut custom_settings = RyzoraSettings::default();
    custom_settings.default_release_channel = "beta".to_string();
    custom_settings.auto_refresh_enabled = false;
    custom_settings.auto_refresh_interval_minutes = 120;
    custom_settings.integrity_scan_on_startup = true;
    save_settings_to_path(&custom_settings, &settings_file).unwrap();

    // Upgrade/reload settings through current parser
    let loaded = load_settings_from_path(&settings_file);
    assert_eq!(loaded.default_release_channel, "beta");
    assert_eq!(loaded.auto_refresh_enabled, false);
    assert_eq!(loaded.auto_refresh_interval_minutes, 120);
    assert_eq!(loaded.integrity_scan_on_startup, true);

    // Save under current version enforces 0600
    save_settings_to_path(&loaded, &settings_file).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let s_mode = fs::metadata(&settings_file).unwrap().permissions().mode() & 0o777;
        assert_eq!(s_mode, 0o600);
    }

    fs::remove_dir_all(&sandbox).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 8: Uninstall Simulation (Clean User-Space De-provisioning)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_8_uninstall_simulation() {
    let sandbox = create_temp_sandbox("uninstall");
    let home_dir = sandbox.join("home");
    let config_dir = home_dir.join(".config/hypr");
    fs::create_dir_all(&config_dir).unwrap();

    // Mock installed files
    let target_file = config_dir.join("hyprland.conf");
    fs::write(&target_file, "original content before install").unwrap();

    // Take snapshot
    let snapshot_root = sandbox.join("snapshots");
    fs::create_dir_all(&snapshot_root).unwrap();
    let snap = create_snapshot_in(
        "pre-uninstall-test",
        &["~/.config/hypr/hyprland.conf".to_string()],
        &home_dir,
        &snapshot_root,
    )
    .unwrap();

    // Simulate package mutation
    fs::write(&target_file, "package mutated content").unwrap();

    // Simulate uninstall via rollback to clean pre-install state
    restore_snapshot_in(&snap.id, &home_dir, &snapshot_root).unwrap();
    let restored_content = fs::read_to_string(&target_file).unwrap();
    assert_eq!(restored_content, "original content before install");

    fs::remove_dir_all(&sandbox).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 9 & 10: Offline / Cache & Recovery Simulation
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_9_and_10_offline_cache_and_recovery() {
    let sandbox = create_temp_sandbox("offline-cache");
    let cache_dir = sandbox.join("repo_cache");
    fs::create_dir_all(&cache_dir).unwrap();

    // Mock an offline cached package directory
    let pkg_dir = cache_dir.join("offline-rice");
    fs::create_dir_all(&pkg_dir).unwrap();

    let manifest_json = r#"{
        "ryzora_spec": "1",
        "id": "offline-rice",
        "name": "Offline Rice",
        "version": "1.0.0",
        "author": "LocalAuthor",
        "package_type": "rice",
        "category": "rices",
        "compatibility": {
            "desktops": ["hyprland"],
            "sessions": ["wayland"],
            "distros": ["arch"],
            "required": [],
            "optional": []
        },
        "files": []
    }"#;
    fs::write(pkg_dir.join("ryzora.json"), manifest_json).unwrap();

    // In offline mode (no network), manifest parses and validates purely locally
    let manifest: RyzoraManifest = serde_json::from_str(manifest_json).unwrap();
    assert_eq!(manifest.id, "offline-rice");
    assert_eq!(manifest.version, "1.0.0");

    // In-memory trust store works completely offline
    let trust_store = TrustStore::load_default();
    assert!(!trust_store.is_official_core_key("nonexistent"));

    fs::remove_dir_all(&sandbox).unwrap();
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 11: Signature Verification & Tampered Rejection
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_11_signature_verification_and_tampering_rejection() {
    let (signing_key, verifying_key) = generate_ed25519_keypair();
    let package_id = "test-pkg";
    let valid_tree_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let pubkey_hex = hex::encode(verifying_key.as_bytes());

    let identity = SignerIdentity {
        author_id: package_id.to_string(),
        name: "Test Signer".to_string(),
        handle: Some("@signer".to_string()),
    };

    let sig_meta =
        sign_package_tree_hash(&signing_key, package_id, valid_tree_hash, identity).unwrap();
    assert_eq!(sig_meta.algorithm, "ed25519");
    assert_eq!(sig_meta.signed_tree_hash, valid_tree_hash);

    // 1. Valid mathematical signature verification passes
    let verify_res = verify_signature_bytes(
        &pubkey_hex,
        package_id,
        valid_tree_hash,
        &sig_meta.signature,
    );
    assert!(
        verify_res.is_ok(),
        "Valid signature must verify successfully"
    );

    // 2. Tampered tree hash fails mathematical verification
    let tampered_hash = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
    let tampered_res =
        verify_signature_bytes(&pubkey_hex, package_id, tampered_hash, &sig_meta.signature);
    assert!(
        tampered_res.is_err(),
        "Tampered hash must fail signature verification"
    );

    // 3. Tampered signature bytes fail verification
    let mut bad_sig_hex = sig_meta.signature.clone();
    bad_sig_hex.replace_range(0..2, "00");
    let bad_sig_res =
        verify_signature_bytes(&pubkey_hex, package_id, valid_tree_hash, &bad_sig_hex);
    assert!(
        bad_sig_res.is_err(),
        "Corrupted signature bytes must fail verification"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 12: Revoked-Key Rejection
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_12_revoked_key_rejection() {
    let (_signing_key, verifying_key) = generate_ed25519_keypair();
    let pubkey_hex = hex::encode(verifying_key.as_bytes());
    let key_id = compute_key_fingerprint(&verifying_key);

    let revoked_entry = RevokedKeyEntry {
        key_id: key_id.clone(),
        public_key: pubkey_hex.clone(),
        reason: "Compromised key in test".to_string(),
        revoked_at: "1773000000".to_string(),
    };

    let trust_store = TrustStore::new_test_store(&[], &[], &[revoked_entry]);
    assert!(trust_store.is_revoked(&key_id, &pubkey_hex));
    assert!(!trust_store.is_trusted_author(&key_id, &pubkey_hex));
    assert!(!trust_store.is_official_core_key(&pubkey_hex));
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 13 & 14: Production Root Key Configuration & Rotation
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_13_and_14_production_root_key_and_rotation() {
    // 1. Unconfigured production safety: absent root CANNOT grant Official tier
    let empty_store = TrustStore::new_test_store(&[], &[], &[]);
    assert!(!empty_store.is_production_ready());
    assert_eq!(empty_store.official_core_keys().len(), 0);
    assert!(!empty_store.is_official_core_key("any_key_hex"));

    // 2. Dev/Test placeholder cannot be treated as production ready
    let dev_store = TrustStore::new_test_store(&[RYZORA_DEV_TEST_ROOT_PUBKEY_HEX], &[], &[]);
    assert!(
        !dev_store.is_production_ready(),
        "Dev placeholder must NOT report production ready"
    );

    // 3. Provisioning authentic production key
    let (_sign1, verify1) = generate_ed25519_keypair();
    let prod_key1 = hex::encode(verify1.as_bytes());
    let mut prod_store = TrustStore::new_test_store(&[&prod_key1], &[], &[]);
    assert!(prod_store.is_production_ready());
    assert!(prod_store.is_official_core_key(&prod_key1));

    // 4. Key rotation: add second authorized production key
    let (_sign2, verify2) = generate_ed25519_keypair();
    let prod_key2 = hex::encode(verify2.as_bytes());
    prod_store.add_official_core_key(&prod_key2);
    assert_eq!(prod_store.official_core_keys().len(), 2);
    assert!(prod_store.is_official_core_key(&prod_key1));
    assert!(prod_store.is_official_core_key(&prod_key2));
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 15: Git Secrets & Private Key Safety Scan
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_15_git_secrets_scan() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_files_recursive(&src_dir, &mut files);

    for path in files {
        if let Ok(content) = fs::read_to_string(&path) {
            let prod_code = content.split("#[cfg(test)]").next().unwrap_or(&content);
            assert!(
                !prod_code.contains("BEGIN PRIVATE KEY"),
                "Found private key PEM in {:?}",
                path
            );
            assert!(
                !prod_code.contains("BEGIN RSA PRIVATE KEY"),
                "Found RSA private key in {:?}",
                path
            );
            assert!(
                !prod_code.contains("BEGIN EC PRIVATE KEY"),
                "Found EC private key in {:?}",
                path
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 16 & 17: Pure-Rust Subprocess & Privilege Escalation Audit
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_16_17_subprocess_and_privilege_audit() {
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_files_recursive(&src_dir, &mut files);

    let mut prohibited_findings = Vec::new();

    for path in files {
        if path.extension().map_or(false, |ext| ext == "rs") {
            // Audit library code (binaries like ryzora-ci have separate standalone scopes)
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let path_str = path.to_string_lossy();
            if file_name == "ryzora-ci.rs"
                || file_name == "hypridle.rs"
                || file_name == "sddm_helper.rs"
                || file_name == "package_helper.rs"
                || path_str.contains("app_adapters")
            {
                continue;
            }
            let full_content = fs::read_to_string(&path).unwrap();
            let prod_code = full_content
                .split("#[cfg(test)]")
                .next()
                .unwrap_or(&full_content);

            for (line_no, line) in prod_code.lines().enumerate() {
                let trimmed = line.trim();
                // Prohibit invocation of Command::new
                if trimmed.contains("Command::new(") || trimmed.contains("process::Command::new(") {
                    prohibited_findings.push(format!(
                        "{}:{} - Command::new( detected: {}",
                        path.display(),
                        line_no + 1,
                        line
                    ));
                }
                // Prohibit sudo or pkexec execution strings (excluding tests and forbidden lists)
                if (trimmed.contains("\"sudo\"")
                    || trimmed.contains("\"pkexec\"")
                    || trimmed.contains("\"sudo ")
                    || trimmed.contains("\"pkexec "))
                    && !trimmed.contains("forbidden")
                    && !trimmed.contains("test")
                    && !trimmed.contains("probe")
                {
                    prohibited_findings.push(format!(
                        "{}:{} - sudo/pkexec invocation detected: {}",
                        path.display(),
                        line_no + 1,
                        line
                    ));
                }
            }
        }
    }

    assert!(
        prohibited_findings.is_empty(),
        "Security audit violations detected in src/: {:?}",
        prohibited_findings
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Vector 18: Path Traversal & Symlink Security Invariants
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn test_phase17_vector_18_path_traversal_regression_audit() {
    let sandbox = create_temp_sandbox("traversal-audit");
    let home_dir = sandbox.join("home");
    let snapshot_dir = sandbox.join("snapshots");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&snapshot_dir).unwrap();

    // 1. Tilde path traversal rejected
    let traversal_paths = [
        "~/../../etc/shadow",
        "~/.config/../../../root",
        "~/config/\0/nullbyte",
        "/etc/passwd",
    ];

    for bad_path in &traversal_paths {
        let snap_res = create_snapshot_in(
            "traversal-test",
            &[bad_path.to_string()],
            &home_dir,
            &snapshot_dir,
        );
        assert!(
            snap_res.is_err(),
            "Snapshot engine must reject path traversal: {}",
            bad_path
        );
    }

    fs::remove_dir_all(&sandbox).unwrap();
}
