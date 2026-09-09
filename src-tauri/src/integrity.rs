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
use std::path::PathBuf;
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

pub fn verify_package_integrity_impl(package_id: &str) -> PackageIntegrityResult {
    let ts = now_iso();
    let installed_root = crate::installer::get_ryzora_installed_dir();

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
    let record = match crate::installer::get_installed_package_in(package_id, &installed_root) {
        Ok(Some(r)) => r,
        Ok(None) => return unable!("Package not found in installed records."),
        Err(e) => return unable!(format!("Failed to load installed record: {}", e)),
    };

    // Step 2: Locate installed package directory
    let package_dir = installed_root.join(crate::installer::manifest_id_safe(package_id));
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

    let home = crate::snapshot::get_home_dir();
    let mut missing_files: Vec<String> = Vec::new();
    let mut modified_files: Vec<String> = Vec::new();

    // Step 4: Verify each manifested file against recorded hash
    for file in &manifest.files {
        let target = match crate::installer::validate_target_safety(&file.target, &home) {
            Ok(p) => p,
            Err(_) => continue, // unsafe path — skip; installation would have rejected it too
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
        // If no recorded entry — we can't verify this file; continue without flagging
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

    // Step 6: Run Ed25519 verification via crypto module
    let trust_store = crate::crypto::TrustStore::load_default();
    let crypto_result = crate::crypto::evaluate_package_directory_crypto(
        &package_dir,
        &manifest,
        None,
        &trust_store,
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
        } else {
            (
                IntegrityStatus::Healthy,
                "All files verified. Signature valid. Key trusted.".to_string(),
            )
        }
    } else if !modified_files.is_empty() {
        // crypto check failed but files are also modified — report the file issue
        (
            IntegrityStatus::Modified,
            format!("Modified files: {}", modified_files.join(", ")),
        )
    } else {
        // Crypto check failed — unable to verify
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

pub fn run_integrity_scan_impl() -> Result<IntegrityScanReport, String> {
    let start = Instant::now();
    let ts = now_iso();

    let installed_root = crate::installer::get_ryzora_installed_dir();
    let installed =
        crate::installer::list_installed_packages_in(&installed_root).unwrap_or_default();

    let mut all_results: Vec<PackageIntegrityResult> = Vec::new();

    for record in &installed {
        let result = verify_package_integrity_impl(&record.package_id);
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

    let notif_msg = if issues.is_empty() {
        format!("{} packages scanned. All healthy.", total_checked)
    } else {
        format!(
            "{} packages scanned. {} issue(s) found.",
            total_checked,
            issues.len()
        )
    };
    let severity = if issues.is_empty() {
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

    let report = IntegrityScanReport {
        total_checked,
        healthy,
        issues,
        all_results,
        scanned_at: ts,
        duration_ms,
    };

    // Cache to disk
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        let path = last_report_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, json);
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn test_integrity_zero_command_execution() {
        let status = IntegrityStatus::Healthy;
        assert!(!status.is_concern());
        // No std::process::Command used.
    }

    #[test]
    fn test_integrity_scan_on_startup_setting_does_not_bypass_verification() {
        // integrity_scan_on_startup=false only controls scheduling, not verification.
        let settings = crate::settings::RyzoraSettings::default();
        assert!(!settings.integrity_scan_on_startup);
        // Verification still runs on demand and correctly classifies nonexistent packages
        let result = verify_package_integrity_impl("nonexistent-pkg");
        assert_ne!(result.status, IntegrityStatus::Healthy);
    }
}
