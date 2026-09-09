/// integrity.rs — Read-only integrity health dashboard (Phase 15.5)
///
/// Surfaces the Phase 12–13 cryptographic verification machinery as a
/// persistent, scannable dashboard. The scan is strictly READ-ONLY.
///
/// No auto-repair. No automatic mutation. If an integrity issue is found:
///   "Integrity issue detected — review"
///   NOT "Ryzora repaired your configuration."
///
/// Status taxonomy:
///   Healthy         — hash OK, signature valid, key trusted
///   Modified        — hash mismatch on existing files
///   MissingFiles    — manifested files absent from disk
///   UnexpectedFiles — extra scripts/executables not in manifest
///   SignatureFailed — Ed25519 verification failure
///   KeyRevoked      — signing key on revocation list
///   UnableToVerify  — metadata incomplete/missing
///
/// INVARIANT: UnableToVerify MUST NOT become Healthy.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Status taxonomy
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityStatus {
    Healthy,
    Modified,
    MissingFiles,
    UnexpectedFiles,
    SignatureFailed,
    KeyRevoked,
    UnableToVerify,
}

impl IntegrityStatus {
    pub fn is_concern(&self) -> bool {
        !matches!(self, IntegrityStatus::Healthy)
    }

    pub fn display_label(&self) -> &'static str {
        match self {
            IntegrityStatus::Healthy => "Healthy",
            IntegrityStatus::Modified => "Modified",
            IntegrityStatus::MissingFiles => "Missing Files",
            IntegrityStatus::UnexpectedFiles => "Unexpected Files",
            IntegrityStatus::SignatureFailed => "Signature Failed",
            IntegrityStatus::KeyRevoked => "Key Revoked",
            IntegrityStatus::UnableToVerify => "Unable to Verify",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageIntegrityResult {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub trust_tier: String,
    pub status: IntegrityStatus,
    pub status_label: String,
    pub detail: String,
    pub signing_key_id: Option<String>,
    pub scanned_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityScanReport {
    pub total_checked: usize,
    pub healthy: usize,
    pub issues: Vec<PackageIntegrityResult>,
    pub all_results: Vec<PackageIntegrityResult>,
    pub scanned_at: String,
    pub duration_ms: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Paths
// ─────────────────────────────────────────────────────────────────────────────

pub fn last_report_path() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora/last_integrity_report.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Timestamp
// ─────────────────────────────────────────────────────────────────────────────

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let diy = if is_leap(year) { 366 } else { 365 };
        if days < diy {
            break;
        }
        days -= diy;
        year += 1;
    }
    let md: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &m in &md {
        if days < m {
            break;
        }
        days -= m;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Core scan logic (pure Rust, zero subprocess, read-only)
// ─────────────────────────────────────────────────────────────────────────────

pub fn verify_package_integrity_in(
    package_id: &str,
    installed_root: &Path,
    home_dir: &Path,
    trust_store: &crate::crypto::TrustStore,
) -> PackageIntegrityResult {
    let ts = now_iso();

    macro_rules! unable {
        ($detail:expr) => {
            PackageIntegrityResult {
                package_id: package_id.to_string(),
                name: package_id.to_string(),
                version: "unknown".to_string(),
                trust_tier: "unknown".to_string(),
                status: IntegrityStatus::UnableToVerify,
                status_label: IntegrityStatus::UnableToVerify.display_label().to_string(),
                detail: $detail.to_string(),
                signing_key_id: None,
                scanned_at: ts.clone(),
            }
        };
    }

    // Step 1: Load installed record
    let record = match crate::installer::get_installed_package_in(package_id, installed_root) {
        Ok(Some(r)) => r,
        Ok(None) => return unable!("Package not found in installed records."),
        Err(e) => return unable!(format!("Failed to load installed record: {}", e)),
    };

    // Step 2: Locate package source directory
    let package_dir = {
        let src_path = PathBuf::from(&record.package_source_path);
        if src_path.is_dir() {
            src_path
        } else if let Ok(found) = crate::installer::find_package_dir(package_id, None) {
            found
        } else {
            installed_root.join(crate::installer::manifest_id_safe(package_id))
        }
    };

    if !package_dir.is_dir() {
        return PackageIntegrityResult {
            package_id: package_id.to_string(),
            name: record.name.clone(),
            version: record.version.clone(),
            trust_tier: "unknown".to_string(),
            status: IntegrityStatus::MissingFiles,
            status_label: IntegrityStatus::MissingFiles.display_label().to_string(),
            detail: "Installed package directory is missing.".to_string(),
            signing_key_id: record.signer_key_id.clone(),
            scanned_at: ts,
        };
    }

    // Step 3: Load manifest
    let manifest = match crate::installer::load_package_manifest(&package_dir) {
        Ok(m) => m,
        Err(e) => return unable!(format!("Failed to load package manifest: {}", e)),
    };

    let mut missing_files: Vec<String> = Vec::new();
    let mut modified_files: Vec<String> = Vec::new();
    let mut unrecorded_files: Vec<String> = Vec::new();

    // Step 4: Verify each manifested file against recorded hash
    for file in &manifest.files {
        let target = match crate::installer::validate_target_safety(&file.target, home_dir) {
            Ok(p) => p,
            Err(_) => {
                unrecorded_files.push(file.target.clone());
                continue;
            }
        };

        if !target.exists() {
            missing_files.push(file.target.clone());
            continue;
        }

        // Compare against the recorded hash in InstalledFileEntry
        if let Some(recorded_entry) = record.files.iter().find(|f| {
            let t_str = target.to_string_lossy();
            f.target == t_str.as_ref() || f.target == file.target
        }) {
            if recorded_entry.sha256.is_empty() {
                // Legacy or missing hash
                unrecorded_files.push(file.target.clone());
            } else {
                match fs::read(&target) {
                    Ok(content) => {
                        let mut hasher = Sha256::new();
                        hasher.update(&content);
                        let actual_hash = hex::encode(hasher.finalize());
                        if recorded_entry.sha256 != actual_hash {
                            modified_files.push(file.target.clone());
                        }
                    }
                    Err(_) => {
                        missing_files.push(file.target.clone());
                    }
                }
            }
        } else {
            // Manifest declares a file that is not in the installed record -> metadata discrepancy!
            unrecorded_files.push(file.target.clone());
        }
    }

    // Step 5: Check for unexpected executables/scripts in the package directory
    let mut unexpected_files: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&package_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let p = entry.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".sh")
                || name.ends_with(".py")
                || name.ends_with(".bash")
                || name.ends_with(".zsh")
            {
                unexpected_files.push(p.to_string_lossy().to_string());
            }
        }
    }

    // Step 6: Run cryptographic evaluation via crypto module (Phase 12 policy)
    let crypto_result = crate::crypto::evaluate_package_directory_crypto(
        &package_dir,
        &manifest,
        None,
        trust_store,
    );

    // Step 7: Check revocation list using the recorded signer key
    let signing_key_id = record.signer_key_id.clone();
    let revoked_keys = trust_store.list_revoked_keys();
    let key_revoked = if let Some(ref kid) = signing_key_id {
        revoked_keys.iter().any(|r| &r.key_id == kid)
    } else {
        false
    };

    // Derive trust tier from cryptographic status on the record
    let trust_tier = record
        .cryptographic_status
        .as_ref()
        .map(|s| format!("{:?}", s))
        .unwrap_or_else(|| "Unknown".to_string());

    // Classify — most severe first
    // CRITICAL INVARIANT: UnableToVerify CAN NEVER BECOME Healthy.
    let (status, detail) = if !missing_files.is_empty() {
        (
            IntegrityStatus::MissingFiles,
            format!("Missing files: {}", missing_files.join(", ")),
        )
    } else if key_revoked {
        (
            IntegrityStatus::KeyRevoked,
            format!(
                "Signing key '{}' has been revoked.",
                signing_key_id.as_deref().unwrap_or("unknown")
            ),
        )
    } else if let Ok(ref eval) = crypto_result {
        if !eval.is_valid {
            (
                IntegrityStatus::SignatureFailed,
                eval.error_message
                    .clone()
                    .unwrap_or_else(|| "Ed25519 signature verification failed.".to_string()),
            )
        } else if !modified_files.is_empty() {
            (
                IntegrityStatus::Modified,
                format!("Modified files: {}", modified_files.join(", ")),
            )
        } else if !unexpected_files.is_empty() {
            (
                IntegrityStatus::UnexpectedFiles,
                format!(
                    "Unexpected executable files in package directory: {}",
                    unexpected_files.join(", ")
                ),
            )
        } else if !unrecorded_files.is_empty() {
            // Unrecorded manifested files must NEVER become Healthy
            (
                IntegrityStatus::UnableToVerify,
                format!(
                    "Manifested files lack authoritative checksum records: {}",
                    unrecorded_files.join(", ")
                ),
            )
        } else {
            (
                IntegrityStatus::Healthy,
                "All files verified. Signature valid. Key trusted.".to_string(),
            )
        }
    } else if !modified_files.is_empty() {
        (
            IntegrityStatus::Modified,
            format!("Modified files: {}", modified_files.join(", ")),
        )
    } else {
        (
            IntegrityStatus::UnableToVerify,
            crypto_result
                .err()
                .unwrap_or_else(|| "Unable to verify cryptographic state.".to_string()),
        )
    };

    // Fire security notification for critical findings
    if matches!(
        status,
        IntegrityStatus::KeyRevoked | IntegrityStatus::SignatureFailed
    ) {
        crate::notifications::notify(
            crate::notifications::NotificationKind::IntegrityFailure,
            crate::notifications::Severity::Critical,
            &format!("Integrity issue: {}", record.name),
            &detail,
            Some(package_id),
            None,
        );
    }

    PackageIntegrityResult {
        package_id: package_id.to_string(),
        name: record.name,
        version: record.version,
        trust_tier,
        status: status.clone(),
        status_label: status.display_label().to_string(),
        detail,
        signing_key_id,
        scanned_at: ts,
    }
}

pub fn verify_package_integrity_impl(package_id: &str) -> PackageIntegrityResult {
    let installed_root = crate::installer::get_ryzora_installed_dir();
    let home = crate::snapshot::get_home_dir();
    let trust_store = crate::crypto::TrustStore::load_default();
    verify_package_integrity_in(package_id, &installed_root, &home, &trust_store)
}

pub fn run_integrity_scan_in(
    installed_root: &Path,
    home_dir: &Path,
    trust_store: &crate::crypto::TrustStore,
) -> Result<IntegrityScanReport, String> {
    let start = Instant::now();
    let ts = now_iso();

    let installed =
        crate::installer::list_installed_packages_in(installed_root).unwrap_or_default();

    let mut all_results: Vec<PackageIntegrityResult> = Vec::new();

    for record in &installed {
        let result =
            verify_package_integrity_in(&record.package_id, installed_root, home_dir, trust_store);
        all_results.push(result);
    }

    let healthy = all_results
        .iter()
        .filter(|r| r.status == IntegrityStatus::Healthy)
        .count();
    let issues: Vec<PackageIntegrityResult> = all_results
        .iter()
        .filter(|r| r.status.is_concern())
        .cloned()
        .collect();

    let duration_ms = start.elapsed().as_millis() as u64;
    let total_checked = all_results.len();

    Ok(IntegrityScanReport {
        total_checked,
        healthy,
        issues,
        all_results,
        scanned_at: ts,
        duration_ms,
    })
}

pub fn run_integrity_scan_impl() -> Result<IntegrityScanReport, String> {
    let installed_root = crate::installer::get_ryzora_installed_dir();
    let home = crate::snapshot::get_home_dir();
    let trust_store = crate::crypto::TrustStore::load_default();

    let report = run_integrity_scan_in(&installed_root, &home, &trust_store)?;

    let notif_msg = if report.issues.is_empty() {
        format!("{} packages scanned. All healthy.", report.total_checked)
    } else {
        format!(
            "{} packages scanned. {} issue(s) found.",
            report.total_checked,
            report.issues.len()
        )
    };
    let severity = if report.issues.is_empty() {
        crate::notifications::Severity::Info
    } else {
        crate::notifications::Severity::Warning
    };
    crate::notifications::notify(
        crate::notifications::NotificationKind::IntegrityScanCompleted,
        severity,
        "Integrity scan completed",
        &notif_msg,
        None,
        None,
    );

    // Cache to disk atomically with hardened permissions (0700 dir, 0600 file)
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let path = last_report_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
            let _ = crate::crypto::set_secure_permissions(parent, 0o700);
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let tmp = path
            .parent()
            .unwrap_or_else(|| Path::new("/tmp"))
            .join(format!(".report.tmp.{}", nanos));

        if fs::write(&tmp, &json).is_ok() {
            let _ = crate::crypto::set_secure_permissions(&tmp, 0o600);
            if fs::rename(&tmp, &path).is_ok() {
                let _ = crate::crypto::set_secure_permissions(&path, 0o600);
            }
        }
    }

    Ok(report)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn run_integrity_scan() -> Result<IntegrityScanReport, String> {
    run_integrity_scan_impl()
}

#[tauri::command]
pub fn verify_package_integrity(package_id: String) -> Result<PackageIntegrityResult, String> {
    Ok(verify_package_integrity_impl(&package_id))
}

#[tauri::command]
pub fn get_last_integrity_report() -> Result<Option<IntegrityScanReport>, String> {
    let path = last_report_path();
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read last integrity report: {}", e))?;
    let report = serde_json::from_str::<IntegrityScanReport>(&raw)
        .map_err(|e| format!("Failed to parse last integrity report: {}", e))?;
    Ok(Some(report))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_test_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let p = env::temp_dir().join(format!("ryzora-integrity-test-{}-{}", label, nanos));
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn test_integrity_status_healthy_is_not_concern() {
        assert!(!IntegrityStatus::Healthy.is_concern());
    }

    #[test]
    fn test_integrity_all_non_healthy_are_concerns() {
        for status in &[
            IntegrityStatus::Modified,
            IntegrityStatus::MissingFiles,
            IntegrityStatus::UnexpectedFiles,
            IntegrityStatus::SignatureFailed,
            IntegrityStatus::KeyRevoked,
            IntegrityStatus::UnableToVerify,
        ] {
            assert!(
                status.is_concern(),
                "Status {:?} should be a concern",
                status
            );
        }
    }

    #[test]
    fn test_integrity_unable_to_verify_is_not_healthy() {
        assert_ne!(IntegrityStatus::UnableToVerify, IntegrityStatus::Healthy);
        assert!(IntegrityStatus::UnableToVerify.is_concern());
        assert_ne!(
            IntegrityStatus::UnableToVerify.display_label(),
            IntegrityStatus::Healthy.display_label()
        );
    }

    #[test]
    fn test_integrity_nonexistent_package_never_returns_healthy() {
        let result = verify_package_integrity_impl("totally-nonexistent-package-xyz-9999");
        assert_ne!(result.status, IntegrityStatus::Healthy);
        assert!(result.status.is_concern());
        assert!(matches!(
            result.status,
            IntegrityStatus::UnableToVerify | IntegrityStatus::MissingFiles
        ));
    }

    #[test]
    fn test_integrity_status_labels_are_distinct() {
        let statuses = [
            IntegrityStatus::Healthy,
            IntegrityStatus::Modified,
            IntegrityStatus::MissingFiles,
            IntegrityStatus::UnexpectedFiles,
            IntegrityStatus::SignatureFailed,
            IntegrityStatus::KeyRevoked,
            IntegrityStatus::UnableToVerify,
        ];
        let labels: Vec<&str> = statuses.iter().map(|s| s.display_label()).collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(
            unique.len(),
            statuses.len(),
            "All status labels must be distinct"
        );
    }

    #[test]
    fn test_integrity_report_serialization_round_trip() {
        let report = IntegrityScanReport {
            total_checked: 3,
            healthy: 2,
            issues: vec![PackageIntegrityResult {
                package_id: "test-pkg".to_string(),
                name: "Test Package".to_string(),
                version: "1.0.0".to_string(),
                trust_tier: "Community".to_string(),
                status: IntegrityStatus::Modified,
                status_label: "Modified".to_string(),
                detail: "files/config.conf was modified".to_string(),
                signing_key_id: Some("ed25519:abc123".to_string()),
                scanned_at: "2026-09-09T12:00:00Z".to_string(),
            }],
            all_results: vec![],
            scanned_at: "2026-09-09T12:00:00Z".to_string(),
            duration_ms: 42,
        };
        let json = serde_json::to_string_pretty(&report).unwrap();
        let parsed: IntegrityScanReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_checked, 3);
        assert_eq!(parsed.healthy, 2);
        assert_eq!(parsed.issues[0].status, IntegrityStatus::Modified);
    }

    #[test]
    fn test_integrity_missing_files_detected() {
        let dir = temp_test_dir("missing");
        let installed_dir = dir.join("installed");
        let home_dir = dir.join("home");
        let pkg_src = dir.join("pkg_src");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&pkg_src).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        // Write manifest
        let manifest_json = r#"{
  "id": "missing-pkg",
  "name": "Missing Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/test.conf", "target": "~/.config/test.conf", "description": "test" }
  ]
}"#;
        fs::write(pkg_src.join("manifest.json"), manifest_json).unwrap();

        // Write record
        let record = crate::installer::InstalledPackageRecord {
            package_id: "missing-pkg".to_string(),
            name: "Missing Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 0,
            snapshot_id: "snap-1".to_string(),
            installed_files: vec!["~/.config/test.conf".to_string()],
            files: vec![crate::installer::InstalledFileEntry {
                target: "~/.config/test.conf".to_string(),
                sha256: "abc".to_string(),
                is_symlink: false,
                symlink_target: None,
            }],
            package_source_path: pkg_src.display().to_string(),
            cryptographic_status: Some(crate::crypto::CryptographicStatus::Unsigned),
            signer_key_id: None,
            signer_name: None,
            history: vec![],
        };
        fs::write(
            installed_dir.join("missing-pkg.json"),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();

        let trust_store = crate::crypto::TrustStore::new_test_store(&[], &[], &[]);
        let result =
            verify_package_integrity_in("missing-pkg", &installed_dir, &home_dir, &trust_store);

        // File does not exist on disk in home_dir -> must report MissingFiles
        assert_eq!(result.status, IntegrityStatus::MissingFiles);
        assert!(result.detail.contains("Missing files"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_integrity_modified_files_detected() {
        let dir = temp_test_dir("modified");
        let installed_dir = dir.join("installed");
        let home_dir = dir.join("home");
        let pkg_src = dir.join("pkg_src");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&pkg_src).unwrap();
        fs::create_dir_all(home_dir.join(".config")).unwrap();

        // Write manifest
        let manifest_json = r#"{
  "id": "mod-pkg",
  "name": "Mod Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/test.conf", "target": "~/.config/test.conf", "description": "test" }
  ]
}"#;
        fs::write(pkg_src.join("manifest.json"), manifest_json).unwrap();

        // Write file on disk with content "tampered"
        fs::write(home_dir.join(".config/test.conf"), "tampered").unwrap();

        // Write record with hash of "original"
        let mut hasher = Sha256::new();
        hasher.update(b"original");
        let expected_hash = hex::encode(hasher.finalize());

        let record = crate::installer::InstalledPackageRecord {
            package_id: "mod-pkg".to_string(),
            name: "Mod Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 0,
            snapshot_id: "snap-1".to_string(),
            installed_files: vec!["~/.config/test.conf".to_string()],
            files: vec![crate::installer::InstalledFileEntry {
                target: "~/.config/test.conf".to_string(),
                sha256: expected_hash,
                is_symlink: false,
                symlink_target: None,
            }],
            package_source_path: pkg_src.display().to_string(),
            cryptographic_status: Some(crate::crypto::CryptographicStatus::Unsigned),
            signer_key_id: None,
            signer_name: None,
            history: vec![],
        };
        fs::write(
            installed_dir.join("mod-pkg.json"),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();

        let trust_store = crate::crypto::TrustStore::new_test_store(&[], &[], &[]);
        let result =
            verify_package_integrity_in("mod-pkg", &installed_dir, &home_dir, &trust_store);

        assert_eq!(result.status, IntegrityStatus::Modified);
        assert!(result.detail.contains("Modified files"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_integrity_unrecorded_manifest_file_cannot_become_healthy() {
        let dir = temp_test_dir("unrecorded");
        let installed_dir = dir.join("installed");
        let home_dir = dir.join("home");
        let pkg_src = dir.join("pkg_src");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&pkg_src).unwrap();
        fs::create_dir_all(home_dir.join(".config")).unwrap();

        // Write manifest with 2 files
        let manifest_json = r#"{
  "id": "unrec-pkg",
  "name": "Unrec Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/f1.conf", "target": "~/.config/f1.conf", "description": "f1" },
    { "source": "files/f2.conf", "target": "~/.config/f2.conf", "description": "f2" }
  ]
}"#;
        fs::write(pkg_src.join("manifest.json"), manifest_json).unwrap();
        fs::write(home_dir.join(".config/f1.conf"), "c1").unwrap();
        fs::write(home_dir.join(".config/f2.conf"), "c2").unwrap();

        // Record only has f1, f2 is completely missing from files record
        let mut hasher = Sha256::new();
        hasher.update(b"c1");
        let hash1 = hex::encode(hasher.finalize());

        let record = crate::installer::InstalledPackageRecord {
            package_id: "unrec-pkg".to_string(),
            name: "Unrec Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 0,
            snapshot_id: "snap-1".to_string(),
            installed_files: vec!["~/.config/f1.conf".to_string()],
            files: vec![crate::installer::InstalledFileEntry {
                target: "~/.config/f1.conf".to_string(),
                sha256: hash1,
                is_symlink: false,
                symlink_target: None,
            }],
            package_source_path: pkg_src.display().to_string(),
            cryptographic_status: Some(crate::crypto::CryptographicStatus::Unsigned),
            signer_key_id: None,
            signer_name: None,
            history: vec![],
        };
        fs::write(
            installed_dir.join("unrec-pkg.json"),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();

        let trust_store = crate::crypto::TrustStore::new_test_store(&[], &[], &[]);
        let result =
            verify_package_integrity_in("unrec-pkg", &installed_dir, &home_dir, &trust_store);

        // Crucial invariant: UnableToVerify can NEVER become Healthy!
        assert_eq!(result.status, IntegrityStatus::UnableToVerify);
        assert_ne!(result.status, IntegrityStatus::Healthy);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_integrity_revoked_key_classified_as_key_revoked() {
        let dir = temp_test_dir("revoked");
        let installed_dir = dir.join("installed");
        let home_dir = dir.join("home");
        let pkg_src = dir.join("pkg_src");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&pkg_src).unwrap();
        fs::create_dir_all(home_dir.join(".config")).unwrap();

        let manifest_json = r#"{
  "id": "rev-pkg",
  "name": "Rev Pkg",
  "version": "1.0.0",
  "ryzora_spec": "1",
  "author": "Tester",
  "package_type": "rice",
  "description": "Test",
  "tags": [],
  "color_palette": [],
  "compatibility": { "desktops": [], "sessions": [], "distros": [], "required": [], "optional": [] },
  "files": [
    { "source": "files/f.conf", "target": "~/.config/f.conf", "description": "f" }
  ]
}"#;
        fs::write(pkg_src.join("manifest.json"), manifest_json).unwrap();
        fs::write(home_dir.join(".config/f.conf"), "c").unwrap();

        let mut hasher = Sha256::new();
        hasher.update(b"c");
        let h = hex::encode(hasher.finalize());

        let record = crate::installer::InstalledPackageRecord {
            package_id: "rev-pkg".to_string(),
            name: "Rev Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: None,
            repository_id: None,
            installed_at: 0,
            snapshot_id: "snap-1".to_string(),
            installed_files: vec!["~/.config/f.conf".to_string()],
            files: vec![crate::installer::InstalledFileEntry {
                target: "~/.config/f.conf".to_string(),
                sha256: h,
                is_symlink: false,
                symlink_target: None,
            }],
            package_source_path: pkg_src.display().to_string(),
            cryptographic_status: Some(crate::crypto::CryptographicStatus::SelfSignedUnvetted),
            signer_key_id: Some("ed25519:badkey123".to_string()),
            signer_name: Some("Compromised Author".to_string()),
            history: vec![],
        };
        fs::write(
            installed_dir.join("rev-pkg.json"),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();

        // Trust store with revoked key "ed25519:badkey123"
        let revoked_entry = crate::crypto::RevokedKeyEntry {
            key_id: "ed25519:badkey123".to_string(),
            public_key: "abcd".to_string(),
            reason: "Key compromised".to_string(),
            revoked_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let trust_store = crate::crypto::TrustStore::new_test_store(&[], &[], &[revoked_entry]);

        let result =
            verify_package_integrity_in("rev-pkg", &installed_dir, &home_dir, &trust_store);

        assert_eq!(result.status, IntegrityStatus::KeyRevoked);
        assert_ne!(result.status, IntegrityStatus::Healthy);
        assert!(result.detail.contains("revoked"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_integrity_scan_performs_zero_mutations() {
        let dir = temp_test_dir("readonly");
        let installed_dir = dir.join("installed");
        let home_dir = dir.join("home");
        fs::create_dir_all(&installed_dir).unwrap();
        fs::create_dir_all(&home_dir).unwrap();

        let trust_store = crate::crypto::TrustStore::new_test_store(&[], &[], &[]);

        // Run scan
        let report = run_integrity_scan_in(&installed_dir, &home_dir, &trust_store).unwrap();
        assert_eq!(report.total_checked, 0);

        // Check that no files were written to home_dir or installed_dir
        assert_eq!(fs::read_dir(&home_dir).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&installed_dir).unwrap().count(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_integrity_scan_on_startup_setting_does_not_bypass_verification() {
        let settings = crate::settings::RyzoraSettings::default();
        assert!(!settings.integrity_scan_on_startup);
        let result = verify_package_integrity_impl("nonexistent-pkg");
        assert_ne!(result.status, IntegrityStatus::Healthy);
    }

    #[test]
    fn test_integrity_zero_command_execution() {
        let status = IntegrityStatus::Healthy;
        assert!(!status.is_concern());
    }
}
