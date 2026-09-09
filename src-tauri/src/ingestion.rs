//! Community Repository Ingestion & CI Automation Engine
//!
//! Provides automated validation, static security scanning, cryptographic
//! signature verification, and atomic publishing for community package submissions.
//!
//! ZERO COMMAND EXECUTION INVARIANT:
//! This engine strictly inspects AST, magic bytes, and cryptographic signatures.
//! Under no circumstances does it invoke subprocesses (sh, bash, sudo, pkexec,
//! pacman, yay, paru, apt, etc.).

use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::crypto::{
    evaluate_trust_chain, load_package_signature, CryptographicStatus, TrustStore,
};
use crate::distribution::{
    compute_file_sha256, DistributionRelease, ModerationStatus, ReleaseChannel, TrustTier,
};
use crate::manifest::{validate_manifest_internal, RyzoraManifest};
use crate::repository::{
    compute_package_tree_hash, AuthorInfo, RepositoryIndex, RepositoryPackageEntry,
};

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Result of an individual automated CI check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IngestionCheckResult {
    pub check_id: String,
    pub name: String,
    pub passed: bool,
    pub level: String, // "error" | "warning" | "info"
    pub message: String,
}

/// Comprehensive CI Ingestion & Security Audit Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionReport {
    pub package_id: String,
    pub package_name: String,
    pub version: String,
    pub passed: bool,
    pub audit_score: u32, // 0 - 100
    pub computed_trust_tier: TrustTier,
    pub moderation_status: ModerationStatus,
    pub checks: Vec<IngestionCheckResult>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub pr_comment_markdown: String,
    pub tree_hash: String,
    pub signature_status: CryptographicStatus,
    pub signer_key_id: Option<String>,
}

/// Outcome of ingesting a community submission into a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionResult {
    pub success: bool,
    pub package_id: String,
    pub version: String,
    pub repository_path: String,
    pub report: IngestionReport,
}

// ─────────────────────────────────────────────────────────────────────────────
// Prohibited Byte Patterns & Limits
// ─────────────────────────────────────────────────────────────────────────────

const MAX_PACKAGE_FILES: usize = 200;
const MAX_SINGLE_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB
const MAX_TOTAL_PACKAGE_BYTES: u64 = 100 * 1024 * 1024; // 100 MB

const PROHIBITED_FILE_NAMES: &[&str] = &[
    ".git",
    ".env",
    ".ssh",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "authorized_keys",
    "known_hosts",
    "sudoers",
    ".bash_history",
    ".zsh_history",
    "passwd",
    "shadow",
];

const FORBIDDEN_TARGET_PREFIXES: &[&str] = &[
    "/etc",
    "/usr",
    "/boot",
    "/root",
    "/var",
    "/opt",
    "/sys",
    "/proc",
    "~/.ssh",
    "~/.gnupg",
    "~/.bash_history",
];

/// Recursively copies a directory tree cleanly in pure Rust.
/// Symlinks, sockets, FIFOs, and devices are strictly rejected to prevent
/// dereferencing, traversal, and local filesystem escape attacks.
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory '{}': {}", dst.display(), e))?;
    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory '{}': {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("File type error for '{}': {}", entry_path.display(), e))?;

        if file_type.is_symlink() {
            return Err(format!(
                "Symlinks are strictly prohibited in community submissions: '{}'",
                entry_path.display()
            ));
        } else if file_type.is_dir() {
            copy_dir_all(&entry_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&entry_path, &target_path).map_err(|e| {
                format!(
                    "Failed to copy file '{}' to '{}': {}",
                    entry_path.display(),
                    target_path.display(),
                    e
                )
            })?;
        } else {
            return Err(format!(
                "Special or unsupported file type rejected: '{}'",
                entry_path.display()
            ));
        }
    }
    Ok(())
}

/// Recursively scans the submission directory tree for symlinks, executables, prohibited files,
/// and oversized payloads. Rejects any symlinks to prevent dereferencing/traversal attacks.
fn scan_directory_tree_security(
    current_dir: &Path,
    submission_root: &Path,
    total_files: &mut usize,
    total_bytes: &mut u64,
    errors: &mut Vec<String>,
) -> Result<bool, String> {
    let mut passed = true;
    let entries = fs::read_dir(current_dir).map_err(|e| {
        format!(
            "Failed to read directory '{}': {}",
            current_dir.display(),
            e
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("File type error for '{}': {}", entry_path.display(), e))?;

        let rel_path = entry_path
            .strip_prefix(submission_root)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        // 1. Strict Symlink Invariant: NO symlinks anywhere in submission
        if file_type.is_symlink() {
            errors.push(format!(
                "Symlinks are strictly prohibited in community submissions. Detected symlink at '{}'",
                rel_path
            ));
            passed = false;
            continue;
        }

        if file_type.is_dir() {
            if !scan_directory_tree_security(
                &entry_path,
                submission_root,
                total_files,
                total_bytes,
                errors,
            )? {
                passed = false;
            }
        } else if file_type.is_file() {
            *total_files += 1;
            let file_name = entry_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            // Check prohibited file names
            for prob in PROHIBITED_FILE_NAMES {
                if file_name.eq_ignore_ascii_case(prob) {
                    errors.push(format!(
                        "Prohibited sensitive file name '{}' found at '{}'",
                        prob, rel_path
                    ));
                    passed = false;
                }
            }

            // Check file size
            let meta = fs::symlink_metadata(&entry_path)
                .map_err(|e| format!("Failed to read metadata for '{}': {}", rel_path, e))?;
            let file_len = meta.len();
            *total_bytes += file_len;

            if file_len > MAX_SINGLE_FILE_BYTES {
                errors.push(format!(
                    "File '{}' exceeds maximum allowed file size of 50 MB (size: {} bytes)",
                    rel_path, file_len
                ));
                passed = false;
            }

            // Magic byte scanning for executables & scripts
            let is_metadata_file = rel_path == "ryzora.json"
                || rel_path == "release.json"
                || rel_path == "release.sig"
                || rel_path == "checksums.sha256"
                || rel_path == "SUBMISSION.md"
                || rel_path == "README.md";

            let mut head_buf = [0u8; 16];
            if let Ok(mut handle) = fs::File::open(&entry_path) {
                use std::io::Read;
                let read_bytes = handle.read(&mut head_buf).unwrap_or(0);
                let head = &head_buf[..read_bytes];

                // ELF check
                if head.len() >= 4 && head[..4] == [0x7f, b'E', b'L', b'F'] {
                    errors.push(format!(
                        "Executable binary rejected (ELF binary): '{}'",
                        rel_path
                    ));
                    passed = false;
                }

                // Mach-O check
                if head.len() >= 4
                    && (head[..4] == [0xfe, 0xed, 0xfa, 0xce]
                        || head[..4] == [0xce, 0xfa, 0xed, 0xfe]
                        || head[..4] == [0xfe, 0xed, 0xfa, 0xcf]
                        || head[..4] == [0xcf, 0xfa, 0xed, 0xfe])
                {
                    errors.push(format!(
                        "Executable binary rejected (Mach-O binary): '{}'",
                        rel_path
                    ));
                    passed = false;
                }

                // PE/MZ check
                if head.len() >= 2 && head[..2] == [b'M', b'Z'] {
                    errors.push(format!(
                        "Executable binary rejected (Windows PE executable): '{}'",
                        rel_path
                    ));
                    passed = false;
                }

                // Script hook shebang check
                if !is_metadata_file && head.len() >= 2 && head[..2] == [b'#', b'!'] {
                    errors.push(format!(
                        "Executable script with shebang rejected: '{}'",
                        rel_path
                    ));
                    passed = false;
                }
            }
        } else {
            errors.push(format!(
                "Special or unsupported file type rejected: '{}'",
                rel_path
            ));
            passed = false;
        }
    }

    Ok(passed)
}

/// Validates checksums.sha256 manifest:
/// Ensures format is strictly `<64-hex-sha256> <path>`, paths are safe and non-traversing,
/// no duplicate entries, no symlinks, all actual files match hashes, and all declared files
/// are represented. Mandatory for distribution release bundles.
fn validate_checksums_file(
    checksums_file: &Path,
    submission_dir: &Path,
    manifest: &RyzoraManifest,
    is_distribution_bundle: bool,
    errors: &mut Vec<String>,
) -> IngestionCheckResult {
    if !checksums_file.is_file() {
        if is_distribution_bundle {
            errors.push(
                "Missing required 'checksums.sha256' manifest in distribution release bundle."
                    .to_string(),
            );
            return IngestionCheckResult {
                check_id: "checksums_sha256".to_string(),
                name: "SHA-256 Checksum Manifest Verification".to_string(),
                passed: false,
                level: "error".to_string(),
                message: "Missing required 'checksums.sha256' in distribution release bundle."
                    .to_string(),
            };
        } else {
            return IngestionCheckResult {
                check_id: "checksums_sha256".to_string(),
                name: "SHA-256 Checksum Manifest Verification".to_string(),
                passed: true,
                level: "info".to_string(),
                message: "No checksums.sha256 file present (standalone manifest package)."
                    .to_string(),
            };
        }
    }

    let content = match fs::read_to_string(checksums_file) {
        Ok(c) => c,
        Err(e) => {
            errors.push(format!("Failed to read checksums.sha256: {}", e));
            return IngestionCheckResult {
                check_id: "checksums_sha256".to_string(),
                name: "SHA-256 Checksum Manifest Verification".to_string(),
                passed: false,
                level: "error".to_string(),
                message: format!("Unreadable checksums.sha256: {}", e),
            };
        }
    };

    let mut seen_paths = HashSet::new();
    let mut recorded_files = HashMap::new();
    let mut valid = true;

    for (line_no, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            errors.push(format!(
                "Malformed line {} in checksums.sha256: expected '<sha256> <path>', found '{}'",
                line_no + 1,
                raw_line
            ));
            valid = false;
            continue;
        }

        let expected_sha = parts[0].trim();
        let raw_rel_path = parts[1].trim();

        // 1. Verify hash format: exactly 64 hex characters
        if expected_sha.len() != 64 || !expected_sha.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(format!(
                "Invalid SHA-256 format on line {} of checksums.sha256: '{}'",
                line_no + 1,
                expected_sha
            ));
            valid = false;
            continue;
        }

        // 2. Verify relative path safety: no leading slash, no '..', no null byte, no backslash
        if raw_rel_path.starts_with('/')
            || raw_rel_path.starts_with('\\')
            || raw_rel_path.contains("..")
            || raw_rel_path.contains(' ')
        {
            errors.push(format!(
                "Path traversal or invalid path in checksums.sha256 line {}: '{}'",
                line_no + 1,
                raw_rel_path
            ));
            valid = false;
            continue;
        }

        let clean_rel_path = raw_rel_path.trim_start_matches("./").replace('\\', "/");

        // 3. Reject duplicate entries
        if !seen_paths.insert(clean_rel_path.clone()) {
            errors.push(format!(
                "Duplicate entry in checksums.sha256 for path '{}'",
                clean_rel_path
            ));
            valid = false;
            continue;
        }

        // 4. Verify the target file exists on disk inside submission_dir
        let disk_file = submission_dir.join(&clean_rel_path);
        if !disk_file.is_file() {
            errors.push(format!(
                "File listed in checksums.sha256 does not exist on disk: '{}'",
                clean_rel_path
            ));
            valid = false;
            continue;
        }

        // Verify symlink rejection
        if let Ok(sym_meta) = fs::symlink_metadata(&disk_file) {
            if sym_meta.file_type().is_symlink() {
                errors.push(format!(
                    "File listed in checksums.sha256 is a symlink: '{}'",
                    clean_rel_path
                ));
                valid = false;
                continue;
            }
        }

        // 5. Verify actual SHA-256 matches expected
        match compute_file_sha256(&disk_file) {
            Ok(actual_sha) => {
                if !expected_sha.eq_ignore_ascii_case(&actual_sha) {
                    errors.push(format!(
                        "Checksum mismatch for '{}': expected {}, computed {}",
                        clean_rel_path, expected_sha, actual_sha
                    ));
                    valid = false;
                }
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to compute SHA-256 for '{}': {}",
                    clean_rel_path, e
                ));
                valid = false;
            }
        }

        recorded_files.insert(clean_rel_path, expected_sha.to_string());
    }

    // 6. Completeness check: Ensure EVERY file declared in manifest.files is present in checksums.sha256
    for f in &manifest.files {
        let norm_src = f.source.trim_start_matches("./").replace('\\', "/");
        if !recorded_files.contains_key(&norm_src) {
            errors.push(format!(
                "Manifest declared file '{}' is missing from checksums.sha256",
                f.source
            ));
            valid = false;
        }
    }

    IngestionCheckResult {
        check_id: "checksums_sha256".to_string(),
        name: "SHA-256 Checksum Manifest Verification".to_string(),
        passed: valid,
        level: if valid {
            "info".to_string()
        } else {
            "error".to_string()
        },
        message: if valid {
            format!(
                "All {} checksum manifest entries verified successfully with zero discrepancies.",
                recorded_files.len()
            )
        } else {
            "Checksum manifest failed validation (mismatch, missing, malformed, or traversal detected)."
                .to_string()
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CI Audit Execution
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the complete automated CI audit matrix on a submitted package directory.
/// Strictly static inspection: zero subprocess execution, zero script execution.
pub fn run_ci_audit(
    submission_dir: &Path,
    repo_dir: Option<&Path>,
    trust_store: &TrustStore,
) -> Result<IngestionReport, String> {
    if !submission_dir.is_dir() {
        return Err(format!(
            "Submission directory not found: '{}'",
            submission_dir.display()
        ));
    }

    let mut checks = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut audit_score: u32 = 100;

    // ── Check 1: Manifest Presence & Schema Validation ───────────────────────
    let manifest_file = if submission_dir.join("ryzora.json").is_file() {
        submission_dir.join("ryzora.json")
    } else if submission_dir.join("manifest.json").is_file() {
        submission_dir.join("manifest.json")
    } else {
        return Err(format!(
            "Neither ryzora.json nor manifest.json found in submission '{}'",
            submission_dir.display()
        ));
    };

    let manifest_raw = fs::read_to_string(&manifest_file).map_err(|e| {
        format!(
            "Failed to read manifest '{}': {}",
            manifest_file.display(),
            e
        )
    })?;

    let val_res = validate_manifest_internal(&manifest_raw);
    if !val_res.valid {
        for err in &val_res.errors {
            errors.push(format!("Manifest schema error: {}", err));
        }
        checks.push(IngestionCheckResult {
            check_id: "manifest_schema".to_string(),
            name: "Manifest Schema Validation".to_string(),
            passed: false,
            level: "error".to_string(),
            message: format!("Manifest validation failed: {}", val_res.errors.join("; ")),
        });
    } else {
        checks.push(IngestionCheckResult {
            check_id: "manifest_schema".to_string(),
            name: "Manifest Schema Validation".to_string(),
            passed: true,
            level: "info".to_string(),
            message: "Manifest is structurally valid and adheres to Ryzora spec v1.".to_string(),
        });
    }

    let manifest: RyzoraManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("Failed to parse manifest JSON: {}", e))?;

    // Validate Package ID slug format
    let id_clean = manifest.id.trim();
    let id_valid = !id_clean.is_empty()
        && id_clean.len() <= 64
        && id_clean
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !id_valid {
        errors.push(format!(
            "Package ID '{}' must consist only of lowercase alphanumeric characters and hyphens (max 64 chars)",
            manifest.id
        ));
        checks.push(IngestionCheckResult {
            check_id: "package_id_format".to_string(),
            name: "Package ID Slug Format".to_string(),
            passed: false,
            level: "error".to_string(),
            message: format!("Invalid package ID slug: '{}'", manifest.id),
        });
    } else {
        checks.push(IngestionCheckResult {
            check_id: "package_id_format".to_string(),
            name: "Package ID Slug Format".to_string(),
            passed: true,
            level: "info".to_string(),
            message: format!("Package ID '{}' is a valid canonical slug.", manifest.id),
        });
    }

    // Validate SemVer version
    let semver_valid = Version::parse(&manifest.version).is_ok();
    if !semver_valid {
        errors.push(format!(
            "Package version '{}' is not valid SemVer",
            manifest.version
        ));
        checks.push(IngestionCheckResult {
            check_id: "semver_format".to_string(),
            name: "SemVer Compliance".to_string(),
            passed: false,
            level: "error".to_string(),
            message: format!(
                "Version '{}' does not follow SemVer format.",
                manifest.version
            ),
        });
    } else {
        checks.push(IngestionCheckResult {
            check_id: "semver_format".to_string(),
            name: "SemVer Compliance".to_string(),
            passed: true,
            level: "info".to_string(),
            message: format!("Version '{}' is compliant with SemVer.", manifest.version),
        });
    }

    // ── Check 2: Path Confinement & Target Sandboxing ─────────────────────────
    let mut path_confinement_passed = true;
    for (i, f) in manifest.files.iter().enumerate() {
        let t = f.target.trim();
        if !t.starts_with("~/") {
            errors.push(format!(
                "File [{}] target '{}' must be relative to user home (~/)",
                i, t
            ));
            path_confinement_passed = false;
        }

        if t.contains("..") || t.contains(' ') {
            errors.push(format!(
                "File [{}] target '{}' contains prohibited traversal components ('..' or null)",
                i, t
            ));
            path_confinement_passed = false;
        }

        for forbidden in FORBIDDEN_TARGET_PREFIXES {
            if t.starts_with(forbidden) {
                errors.push(format!(
                    "File [{}] target '{}' attempts to modify protected path prefix '{}'",
                    i, t, forbidden
                ));
                path_confinement_passed = false;
            }
        }
    }

    checks.push(IngestionCheckResult {
        check_id: "path_confinement".to_string(),
        name: "Target Path Confinement & Sandbox".to_string(),
        passed: path_confinement_passed,
        level: if path_confinement_passed {
            "info".to_string()
        } else {
            "error".to_string()
        },
        message: if path_confinement_passed {
            "All declared file targets are securely confined to user dotfile directories."
                .to_string()
        } else {
            "Target paths violate sandbox confinement rules.".to_string()
        },
    });

    // ── Check 3: Static Executable & Hook Scanning (Zero Command Execution) ──
    let mut zero_exec_passed = true;
    let mut total_package_bytes: u64 = 0;
    let mut total_files_count: usize = 0;

    // 3.1: Verify declared manifest files exist and are not symlinks
    for (i, f) in manifest.files.iter().enumerate() {
        let file_path = submission_dir.join(&f.source);
        let sym_meta = match fs::symlink_metadata(&file_path) {
            Ok(m) => m,
            Err(_) => {
                errors.push(format!(
                    "Declared file [{}] '{}' was not found on disk at '{}'",
                    i,
                    f.source,
                    file_path.display()
                ));
                path_confinement_passed = false;
                zero_exec_passed = false;
                continue;
            }
        };

        if sym_meta.file_type().is_symlink() {
            errors.push(format!(
                "Symlinks are strictly prohibited in community submissions. Detected symlink at '{}'",
                f.source
            ));
            zero_exec_passed = false;
        }
    }

    // 3.2: Comprehensive recursive security scan of entire submission directory tree
    match scan_directory_tree_security(
        submission_dir,
        submission_dir,
        &mut total_files_count,
        &mut total_package_bytes,
        &mut errors,
    ) {
        Ok(scan_passed) => {
            if !scan_passed {
                zero_exec_passed = false;
            }
        }
        Err(e) => {
            errors.push(format!("Failed to scan submission directory tree: {}", e));
            zero_exec_passed = false;
        }
    }

    if total_files_count > MAX_PACKAGE_FILES {
        errors.push(format!(
            "Total file count ({}) exceeds maximum allowable package limit of {}",
            total_files_count, MAX_PACKAGE_FILES
        ));
        zero_exec_passed = false;
    }

    if total_package_bytes > MAX_TOTAL_PACKAGE_BYTES {
        errors.push(format!(
            "Total package size ({} bytes) exceeds maximum allowable package limit of 100 MB",
            total_package_bytes
        ));
        zero_exec_passed = false;
    }

    checks.push(IngestionCheckResult {
        check_id: "zero_executable_scan".to_string(),
        name: "Zero-Executable & Zero-Hook Pre-Flight Scan".to_string(),
        passed: zero_exec_passed,
        level: if zero_exec_passed {
            "info".to_string()
        } else {
            "error".to_string()
        },
        message: if zero_exec_passed {
            "Package strictly contains declarative dotfiles. Zero binaries, script hooks, or symlinks detected.".to_string()
        } else {
            "Security violation: executable binaries, script hooks, or symlinks detected.".to_string()
        },
    });

    // ── Check 4: Deterministic Canonical Tree Hash & Checksums ───────────────
    let tree_hash_res = compute_package_tree_hash(submission_dir, &manifest);
    let tree_hash = match tree_hash_res {
        Ok(th) => {
            checks.push(IngestionCheckResult {
                check_id: "tree_hash_calculation".to_string(),
                name: "Canonical Tree Hash Computation".to_string(),
                passed: true,
                level: "info".to_string(),
                message: format!("Canonical tree hash computed: {}", th),
            });
            th
        }
        Err(e) => {
            errors.push(format!("Failed to compute canonical tree hash: {}", e));
            checks.push(IngestionCheckResult {
                check_id: "tree_hash_calculation".to_string(),
                name: "Canonical Tree Hash Computation".to_string(),
                passed: false,
                level: "error".to_string(),
                message: format!("Tree hash calculation error: {}", e),
            });
            String::new()
        }
    };

    // ── Check 5: SHA-256 Checksum Manifest Verification ──────────────────────
    let release_meta_opt: Option<DistributionRelease> = {
        let rel_file = submission_dir.join("release.json");
        if rel_file.is_file() {
            fs::read_to_string(&rel_file)
                .ok()
                .and_then(|r| serde_json::from_str(&r).ok())
        } else {
            None
        }
    };

    let is_distribution_bundle = release_meta_opt.is_some();
    let checksums_file = submission_dir.join("checksums.sha256");
    let checksums_check = validate_checksums_file(
        &checksums_file,
        submission_dir,
        &manifest,
        is_distribution_bundle,
        &mut errors,
    );
    let checksums_passed = checksums_check.passed;
    checks.push(checksums_check);

    // ── Check 6: Ed25519 Cryptographic Verification & Trust Chain ────────────

    // Untrusted PR metadata rule: claimed trust tier in PR/release.json cannot elevate package!
    let claimed_tier = release_meta_opt.as_ref().map(|r| r.trust_tier);

    let sig_opt = load_package_signature(submission_dir);
    let crypto_eval = evaluate_trust_chain(
        sig_opt.as_ref(),
        &manifest.id,
        &tree_hash,
        claimed_tier,
        trust_store,
    );

    if crypto_eval.status == CryptographicStatus::SelfSignedUnvetted {
        warnings.push("Package is signed with an unvetted community author key.".to_string());
    } else if crypto_eval.status == CryptographicStatus::Unsigned {
        warnings.push(
            "Package contains no cryptographic signature metadata (Unsigned Community)."
                .to_string(),
        );
    }

    let mut crypto_check_passed = crypto_eval.can_install;
    if !crypto_eval.can_install {
        let msg = crypto_eval.error_message.clone().unwrap_or_else(|| {
            "Cryptographic verification failed or package claims unauthorized tier.".to_string()
        });
        errors.push(format!("Cryptographic Trust Error: {}", msg));
        crypto_check_passed = false;
    }

    checks.push(IngestionCheckResult {
        check_id: "ed25519_signature".to_string(),
        name: "Ed25519 Cryptographic Authenticity & Trust Chain".to_string(),
        passed: crypto_check_passed,
        level: if crypto_check_passed {
            "info".to_string()
        } else {
            "error".to_string()
        },
        message: match crypto_eval.status {
            CryptographicStatus::OfficialVerified => {
                "Cryptographically verified with authentic Ryzora Core Root Key (Official Tier granted).".to_string()
            }
            CryptographicStatus::AuthorVerified => {
                "Verified with registered author key in trusted keyring (Verified Tier granted).".to_string()
            }
            CryptographicStatus::SelfSignedUnvetted => {
                "Mathematically valid Ed25519 signature from unvetted author key (Community Tier assigned).".to_string()
            }
            CryptographicStatus::Unsigned => {
                "Unsigned community package. Permitted under Community Tier.".to_string()
            }
            CryptographicStatus::InvalidSignature => {
                "Mathematical signature verification or canonical tree hash mismatch (Tampering detected).".to_string()
            }
            CryptographicStatus::RevokedKey => {
                "Signing key is listed on Ryzora Revoked Key List. Submissions fail closed.".to_string()
            }
            CryptographicStatus::OfficialImpersonation => {
                "Submission claims Official tier without Ryzora Core Root signature. Rejected fail-closed.".to_string()
            }
        },
    });

    // ── Check 6: Repository Consistency & SemVer Downgrade Prevention ─────────
    let mut repo_consistency_passed = true;
    if let Some(repo_p) = repo_dir {
        let repo_index_file = repo_p.join("repository.json");
        if repo_index_file.is_file() {
            if let Ok(raw_idx) = fs::read_to_string(&repo_index_file) {
                if let Ok(repo_index) = serde_json::from_str::<RepositoryIndex>(&raw_idx) {
                    if let Some(existing) = repo_index.packages.iter().find(|p| p.id == manifest.id)
                    {
                        // Check 6.1: Package Type cannot mutate
                        if existing.package_type != manifest.package_type {
                            errors.push(format!(
                                "Package type mismatch: existing package in repo is '{:?}', submission claims '{:?}'",
                                existing.package_type, manifest.package_type
                            ));
                            repo_consistency_passed = false;
                        }

                        // Check 6.2: SemVer Downgrade Prevention
                        let v_new = Version::parse(&manifest.version);
                        let v_old = Version::parse(&existing.version);
                        if let (Ok(new_ver), Ok(old_ver)) = (v_new, v_old) {
                            if new_ver <= old_ver {
                                errors.push(format!(
                                    "SemVer downgrade or duplicate rejected: submitted version '{}' is not higher than existing repository version '{}'",
                                    manifest.version, existing.version
                                ));
                                repo_consistency_passed = false;
                            }
                        }
                    }
                }
            }
        }
    }

    checks.push(IngestionCheckResult {
        check_id: "downgrade_prevention".to_string(),
        name: "SemVer Upgrade & Downgrade Prevention".to_string(),
        passed: repo_consistency_passed,
        level: if repo_consistency_passed {
            "info".to_string()
        } else {
            "error".to_string()
        },
        message: if repo_consistency_passed {
            "Version strictly upgrades existing repository packages or introduces a new package."
                .to_string()
        } else {
            "Version conflicts with existing repository packages (downgrade or type mismatch)."
                .to_string()
        },
    });

    // ── Derive Trust Tier & Moderation Decision ──────────────────────────────
    let computed_trust_tier = match crypto_eval.status {
        CryptographicStatus::OfficialVerified => TrustTier::Official,
        CryptographicStatus::AuthorVerified => TrustTier::Verified,
        CryptographicStatus::SelfSignedUnvetted => TrustTier::Community,
        CryptographicStatus::Unsigned => TrustTier::Community,
        _ => TrustTier::Untrusted,
    };

    let all_passed = errors.is_empty()
        && val_res.valid
        && id_valid
        && semver_valid
        && path_confinement_passed
        && zero_exec_passed
        && checksums_passed
        && crypto_check_passed
        && repo_consistency_passed;

    // Calculate Audit Score
    if !all_passed {
        audit_score = 0;
    } else {
        if !warnings.is_empty() {
            audit_score = audit_score.saturating_sub((warnings.len() as u32) * 10);
        }
        if crypto_eval.status == CryptographicStatus::SelfSignedUnvetted {
            audit_score = audit_score.min(90);
        }
        if crypto_eval.status == CryptographicStatus::Unsigned {
            audit_score = audit_score.min(80);
        }
    }

    let moderation_status = if !all_passed {
        ModerationStatus::Flagged
    } else if computed_trust_tier == TrustTier::Official
        || computed_trust_tier == TrustTier::Verified
    {
        ModerationStatus::Approved
    } else if crypto_eval.status == CryptographicStatus::SelfSignedUnvetted {
        ModerationStatus::Approved
    } else {
        ModerationStatus::PendingReview
    };

    let signer_key_id = sig_opt.as_ref().map(|s| s.key_id.clone());

    let mut report = IngestionReport {
        package_id: manifest.id.clone(),
        package_name: manifest.name.clone(),
        version: manifest.version.clone(),
        passed: all_passed,
        audit_score,
        computed_trust_tier,
        moderation_status,
        checks,
        errors,
        warnings,
        pr_comment_markdown: String::new(),
        tree_hash,
        signature_status: crypto_eval.status,
        signer_key_id,
    };

    report.pr_comment_markdown = generate_pr_comment_markdown(&report);

    Ok(report)
}

// ─────────────────────────────────────────────────────────────────────────────
// Atomic Repository Ingestion & Rollback
// ─────────────────────────────────────────────────────────────────────────────

/// Ingests a validated package submission into a local repository directory atomically.
/// Rolls back completely if any step fails.
pub fn ingest_submission_into_repository(
    submission_dir: &Path,
    repo_dir: &Path,
    trust_store: &TrustStore,
) -> Result<IngestionResult, String> {
    // 1. Run full CI audit against target repository
    let report = run_ci_audit(submission_dir, Some(repo_dir), trust_store)?;
    if !report.passed {
        return Err(format!(
            "CI Ingestion Audit Failed for package '{}': {}",
            report.package_id,
            report.errors.join("; ")
        ));
    }

    let packages_dir = repo_dir.join("packages");
    fs::create_dir_all(&packages_dir)
        .map_err(|e| format!("Failed to create repository packages directory: {}", e))?;

    let target_pkg_dir = packages_dir.join(&report.package_id);

    // 2. Atomic Staging: copy files to temporary staging folder first
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staging_dir = packages_dir.join(format!(".tmp_stage_{}_{}", report.package_id, nanos));

    copy_dir_all(submission_dir, &staging_dir).map_err(|e| {
        let _ = fs::remove_dir_all(&staging_dir);
        format!("Failed to stage submission files: {}", e)
    })?;

    // 3. Backup repository.json
    let repo_index_path = repo_dir.join("repository.json");
    let backup_index_path = repo_dir.join(format!(".repository.json.bak.{}", nanos));

    if repo_index_path.is_file() {
        fs::copy(&repo_index_path, &backup_index_path)
            .map_err(|e| format!("Failed to backup repository index: {}", e))?;
    }

    // Rollback helper closure
    let rollback = || {
        let _ = fs::remove_dir_all(&staging_dir);
        if backup_index_path.is_file() {
            let _ = fs::copy(&backup_index_path, &repo_index_path);
            let _ = fs::remove_file(&backup_index_path);
        }
    };

    // 4. Load, mutate, and save repository.json
    let mut repo_index = if repo_index_path.is_file() {
        let raw = fs::read_to_string(&repo_index_path).map_err(|e| {
            rollback();
            format!("Failed to read repository index: {}", e)
        })?;
        serde_json::from_str::<RepositoryIndex>(&raw).map_err(|e| {
            rollback();
            format!("Failed to parse repository index JSON: {}", e)
        })?
    } else {
        RepositoryIndex {
            schema: 1,
            id: repo_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("community-repo")
                .to_string(),
            name: "Community Repository".to_string(),
            version: "1.0.0".to_string(),
            description: "Ryzora Community Package Repository".to_string(),
            packages: Vec::new(),
        }
    };

    // Load manifest from submission for metadata
    let manifest = crate::installer::load_package_manifest(submission_dir).map_err(|e| {
        rollback();
        format!("Failed to read manifest for entry: {}", e)
    })?;

    // Load optional signature from submission
    let sig_meta = load_package_signature(submission_dir);

    // Load optional distribution release for release notes / maintainer
    let rel_meta_opt: Option<DistributionRelease> = {
        let rel_file = submission_dir.join("release.json");
        if rel_file.is_file() {
            fs::read_to_string(&rel_file)
                .ok()
                .and_then(|r| serde_json::from_str(&r).ok())
        } else {
            None
        }
    };

    let channel = rel_meta_opt
        .as_ref()
        .map(|r| r.channel)
        .unwrap_or(ReleaseChannel::Stable);

    let release_notes = rel_meta_opt.as_ref().map(|r| r.release_notes.clone());
    let maintainer = rel_meta_opt.as_ref().and_then(|r| r.maintainer.clone());

    // Build or update RepositoryPackageEntry
    let new_entry = RepositoryPackageEntry {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        package_type: manifest.package_type,
        description: manifest.description.clone(),
        manifest: format!("packages/{}/ryzora.json", manifest.id),
        category: "Community".to_string(),
        tags: manifest.tags.clone(),
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: report.computed_trust_tier == TrustTier::Verified
                || report.computed_trust_tier == TrustTier::Official,
        },
        color_palette: manifest.color_palette.clone(),
        hero_image: None,
        screenshots: Vec::new(),
        featured: Some(false), // Untrusted PR metadata discarded
        trending: Some(false), // Untrusted PR metadata discarded
        rating: None,          // Untrusted PR metadata discarded
        downloads: Some(0),    // Untrusted PR metadata discarded
        content_hash: Some(report.tree_hash.clone()),
        package_size_bytes: None,
        release_channel: Some(channel),
        trust_tier: Some(report.computed_trust_tier), // Strictly DERIVED from crypto!
        moderation_status: Some(report.moderation_status),
        trending_score: Some(0.0),
        maintainer,
        release_notes,
        signature: sig_meta,
    };

    if let Some(pos) = repo_index.packages.iter().position(|p| p.id == manifest.id) {
        repo_index.packages[pos] = new_entry;
    } else {
        repo_index.packages.push(new_entry);
    }

    // Write updated repository.json atomically
    let updated_json = serde_json::to_string_pretty(&repo_index).map_err(|e| {
        rollback();
        format!("Failed to serialize updated repository index: {}", e)
    })?;

    let tmp_index_path = repo_dir.join(format!(".repository.json.tmp.{}", nanos));
    if let Err(e) = fs::write(&tmp_index_path, &updated_json) {
        rollback();
        return Err(format!("Failed to write temporary repository index: {}", e));
    }

    // 5. Commit directory rename: replace target_pkg_dir with staging_dir
    if target_pkg_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&target_pkg_dir) {
            rollback();
            return Err(format!("Failed to clean old package directory: {}", e));
        }
    }

    if let Err(e) = fs::rename(&staging_dir, &target_pkg_dir) {
        rollback();
        return Err(format!("Failed to move staged package into place: {}", e));
    }

    // Commit index rename
    if let Err(e) = fs::rename(&tmp_index_path, &repo_index_path) {
        rollback();
        return Err(format!(
            "Failed to atomically commit repository index: {}",
            e
        ));
    }

    // 6. Post-Ingestion Sanity Check
    if let Ok(verify_raw) = fs::read_to_string(&repo_index_path) {
        if let Ok(verified_idx) = serde_json::from_str::<RepositoryIndex>(&verify_raw) {
            if verified_idx.packages.iter().any(|p| p.id == manifest.id) {
                let _ = fs::remove_file(&backup_index_path);
                return Ok(IngestionResult {
                    success: true,
                    package_id: manifest.id,
                    version: manifest.version,
                    repository_path: repo_dir.display().to_string(),
                    report,
                });
            }
        }
    }

    rollback();
    Err("Post-ingestion repository verification failed. Rolled back.".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// PR Comment Markdown Generator
// ─────────────────────────────────────────────────────────────────────────────

/// Generates a GitHub Actions Pull Request summary comment with check indicators.
pub fn generate_pr_comment_markdown(report: &IngestionReport) -> String {
    let status_badge = if report.passed {
        "✅ **CI Ingestion Audit: PASSED**"
    } else {
        "❌ **CI Ingestion Audit: FAILED**"
    };

    let tier_badge = match report.computed_trust_tier {
        TrustTier::Official => "🛡️ **Official** (Ryzora Core Root Key)",
        TrustTier::Verified => "⭐ **Verified** (Registered Author Keyring)",
        TrustTier::Community => "🌐 **Community** (Unvetted / Self-Signed)",
        TrustTier::Untrusted => "⚠️ **Untrusted**",
    };

    let mod_badge = match report.moderation_status {
        ModerationStatus::Approved => "🟢 **Approved for Ingestion**",
        ModerationStatus::PendingReview => "🟡 **Pending Maintainer Review**",
        ModerationStatus::Flagged => "🔴 **Flagged / Rejected**",
        ModerationStatus::Deprecated => "⚪ **Deprecated**",
    };

    let mut md = format!(
        "### {}

| Package | Version | Security Score | Trust Tier | Moderation |
|---|---|---|---|---|
| `{}` | `{}` | **{}/100** | {} | {} |

",
        status_badge, report.package_id, report.version, report.audit_score, tier_badge, mod_badge
    );

    md.push_str(
        "#### 🔍 Automated Inspection Matrix

",
    );
    md.push_str(
        "| Check | Status | Details |
|---|---|---|
",
    );

    for c in &report.checks {
        let icon = if c.passed { "✅ PASS" } else { "❌ FAIL" };
        md.push_str(&format!(
            "| {} | {} | {} |
",
            c.name, icon, c.message
        ));
    }

    if !report.errors.is_empty() {
        md.push_str(
            "
#### 🚨 Security & Validation Blocks
",
        );
        for err in &report.errors {
            md.push_str(&format!(
                "- ❌ {}
",
                err
            ));
        }
    }

    if !report.warnings.is_empty() {
        md.push_str(
            "
#### ⚠️ Warnings
",
        );
        for warn in &report.warnings {
            md.push_str(&format!(
                "- ⚠️ {}
",
                warn
            ));
        }
    }

    md.push_str("
---
*Ryzora CI Automated Audit • Note: CI validation gates integrity; Ryzora Trust Policy determines author identity rights.*
");
    md
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn run_ci_submission_audit(
    submission_dir: String,
    repo_dir: Option<String>,
) -> Result<IngestionReport, String> {
    let sub_path = PathBuf::from(&submission_dir);
    let repo_path = repo_dir.map(PathBuf::from);
    let trust_store = TrustStore::load_default();
    run_ci_audit(&sub_path, repo_path.as_deref(), &trust_store)
}

#[tauri::command]
pub fn ingest_community_submission(
    submission_dir: String,
    repo_dir: String,
) -> Result<IngestionResult, String> {
    let sub_path = PathBuf::from(&submission_dir);
    let repo_path = PathBuf::from(&repo_dir);
    let trust_store = TrustStore::load_default();
    ingest_submission_into_repository(&sub_path, &repo_path, &trust_store)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{
        generate_ed25519_keypair, sign_package_tree_hash, SignerIdentity, TrustedKeyEntry,
    };
    use crate::distribution::build_distribution_release_internal;
    use crate::manifest::PackageType;

    struct TestSandbox {
        root: PathBuf,
    }

    impl TestSandbox {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ryzora-ci-test-{}-{}", label, nanos));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for TestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn create_test_draft(
        sandbox: &TestSandbox,
        id: &str,
        version: &str,
    ) -> crate::authoring::PackageDraft {
        let conf_file = sandbox.root.join(format!("{}-config.conf", id));
        fs::write(
            &conf_file,
            "theme_color = cyan
font = JetBrainsMono
",
        )
        .unwrap();

        crate::authoring::PackageDraft {
            id: id.to_string(),
            name: format!("Test Package {}", id),
            version: version.to_string(),
            author: "TestAuthor".to_string(),
            package_type: PackageType::Theme,
            description: "Test package for CI ingestion audit".to_string(),
            tags: vec!["test".to_string(), "theme".to_string()],
            color_palette: vec!["#00ffff".to_string()],
            compatibility: crate::manifest::ManifestCompatibility {
                desktops: vec!["hyprland".to_string()],
                sessions: vec!["wayland".to_string()],
                distros: vec![],
                required: vec![],
                optional: vec![],
            },
            dependencies: vec![],
            files: vec![crate::authoring::FileMappingDraft {
                source_path: conf_file.to_string_lossy().to_string(),
                target: format!("~/.config/{}/config.conf", id),
                package_rel_path: Some(format!("files/{}/config.conf", id)),
                description: "Main configuration file".to_string(),
            }],
        }
    }

    #[test]
    fn test_ci_clean_signed_submission_passes() {
        let sandbox = TestSandbox::new("clean-ci-pass");
        let (sign, _verify) = generate_ed25519_keypair();
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "clean-theme", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let pkg_dir = PathBuf::from(&pkg_res.package_dir);

        let releases_dir = sandbox.root.join("releases");
        let dist_res = build_distribution_release_internal(
            &pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        let tree_hash = dist_res.tree_hash;

        // Sign release
        let sig = sign_package_tree_hash(
            &sign,
            "clean-theme",
            &tree_hash,
            SignerIdentity {
                author_id: "clean-theme".to_string(),
                name: "TestAuthor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(
            report.passed,
            "Clean submission must pass CI audit: {:?}",
            report.errors
        );
        assert_eq!(report.computed_trust_tier, TrustTier::Community); // Unvetted author is Community
        assert_eq!(report.moderation_status, ModerationStatus::Approved);
        assert_eq!(report.audit_score, 90); // Unvetted author caps at 90
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_ci_clean_official_submission_grants_official() {
        let sandbox = TestSandbox::new("clean-official");
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());

        // Official core root trust store
        let trust_store = TrustStore::new_test_store(&[&pub_hex], &[], &[]);

        let draft = create_test_draft(&sandbox, "official-theme", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let pkg_dir = PathBuf::from(&pkg_res.package_dir);

        let releases_dir = sandbox.root.join("releases");
        let dist_res = build_distribution_release_internal(
            &pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        let tree_hash = dist_res.tree_hash;

        let sig = sign_package_tree_hash(
            &sign,
            "official-theme",
            &tree_hash,
            SignerIdentity {
                author_id: "official-theme".to_string(),
                name: "Ryzora Core".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(report.passed);
        assert_eq!(report.computed_trust_tier, TrustTier::Official);
        assert_eq!(report.moderation_status, ModerationStatus::Approved);
        assert_eq!(report.audit_score, 100);
    }

    #[test]
    fn test_ci_clean_vetted_author_submission_grants_verified() {
        let sandbox = TestSandbox::new("clean-verified");
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());
        let key_id = crate::crypto::compute_key_fingerprint(&verify);

        let trusted_entry = TrustedKeyEntry {
            key_id: key_id.clone(),
            public_key: pub_hex.clone(),
            author_name: "VettedAuthor".to_string(),
            role: "community_verified".to_string(),
            added_at: "1000".to_string(),
            notes: "Vetted contributor".to_string(),
        };

        let trust_store = TrustStore::new_test_store(&[], &[trusted_entry], &[]);

        let draft = create_test_draft(&sandbox, "vetted-theme", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let pkg_dir = PathBuf::from(&pkg_res.package_dir);

        let releases_dir = sandbox.root.join("releases");
        let dist_res = build_distribution_release_internal(
            &pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        let tree_hash = dist_res.tree_hash;

        let sig = sign_package_tree_hash(
            &sign,
            "vetted-theme",
            &tree_hash,
            SignerIdentity {
                author_id: "vetted-theme".to_string(),
                name: "VettedAuthor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(report.passed);
        assert_eq!(report.computed_trust_tier, TrustTier::Verified);
        assert_eq!(report.moderation_status, ModerationStatus::Approved);
        assert_eq!(report.audit_score, 100);
    }

    #[test]
    fn test_ci_binary_executable_detected_and_rejected() {
        let sandbox = TestSandbox::new("binary-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "binary-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        // Inject an ELF binary header into a declared configuration file
        let bad_file = bundle_dir.join("files/binary-pkg/config.conf");
        let mut malicious_bytes = vec![0x7f, b'E', b'L', b'F'];
        malicious_bytes.extend(b"fake elf binary payload");
        fs::write(&bad_file, malicious_bytes).unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed, "ELF binary must cause CI audit to FAIL");
        assert_eq!(report.moderation_status, ModerationStatus::Flagged);
        assert_eq!(report.audit_score, 0);
        assert!(report.errors.iter().any(|e| e.contains("ELF binary")));
    }

    #[test]
    fn test_ci_script_hook_detected_and_rejected() {
        let sandbox = TestSandbox::new("hook-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "hook-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        // Inject a shell script header
        let bad_file = bundle_dir.join("files/hook-pkg/config.conf");
        fs::write(
            &bad_file,
            "#!/bin/bash
exec rm -rf /
",
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed, "Script hook must cause CI audit to FAIL");
        assert_eq!(report.moderation_status, ModerationStatus::Flagged);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("Executable script with shebang")));
    }

    #[test]
    fn test_ci_prohibited_sensitive_files_rejected() {
        let sandbox = TestSandbox::new("sensitive-file-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let mut draft = create_test_draft(&sandbox, "sensitive-pkg", "1.0.0");
        let secret_file = sandbox.root.join("id_rsa");
        fs::write(
            &secret_file,
            "-----BEGIN RSA PRIVATE KEY-----
fake
",
        )
        .unwrap();

        draft.files.push(crate::authoring::FileMappingDraft {
            source_path: secret_file.to_string_lossy().to_string(),
            target: "~/.config/app/id_rsa".to_string(),
            package_rel_path: Some("files/sensitive-pkg/id_rsa".to_string()),
            description: "Secret key".to_string(),
        });

        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("Prohibited sensitive file name")));
    }

    #[test]
    fn test_ci_path_traversal_targets_rejected() {
        let sandbox = TestSandbox::new("traversal-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let mut draft = create_test_draft(&sandbox, "traversal-pkg", "1.0.0");
        draft.files[0].target = "/etc/shadow".to_string();

        let val_draft = crate::authoring::validate_package_draft_internal(&draft);
        assert!(!val_draft.valid, "Draft validation must reject /etc/shadow");

        // Even if an attacker hand-crafts a directory:
        let fake_pkg_dir = sandbox.root.join("fake_pkg");
        fs::create_dir_all(fake_pkg_dir.join("files")).unwrap();
        fs::write(fake_pkg_dir.join("files/test.conf"), "data").unwrap();

        let evil_manifest = crate::manifest::RyzoraManifest {
            id: "traversal-pkg".to_string(),
            name: "Traversal Pkg".to_string(),
            version: "1.0.0".to_string(),
            ryzora_spec: "1".to_string(),
            author: "Attacker".to_string(),
            package_type: PackageType::Theme,
            description: "Path attack".to_string(),
            tags: vec![],
            color_palette: vec![],
            compatibility: crate::manifest::ManifestCompatibility {
                distros: vec![],
                desktops: vec![],
                sessions: vec![],
                required: vec![],
                optional: vec![],
            },
            dependencies: vec![],
            files: vec![crate::manifest::ManifestFile {
                source: "files/test.conf".to_string(),
                target: "/etc/passwd".to_string(),
                description: "Escape".to_string(),
            }],
        };

        fs::write(
            fake_pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&evil_manifest).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&fake_pkg_dir, None, &trust_store).unwrap();
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("attempts to modify protected path prefix")));
    }

    #[test]
    fn test_ci_tampered_payload_fails_tree_hash_and_signature() {
        let sandbox = TestSandbox::new("tamper-ci-fail");
        let (sign, _) = generate_ed25519_keypair();
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "tamper-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let manifest = crate::installer::load_package_manifest(&bundle_dir).unwrap();
        let tree_hash = compute_package_tree_hash(&bundle_dir, &manifest).unwrap();

        let sig = sign_package_tree_hash(
            &sign,
            "tamper-pkg",
            &tree_hash,
            SignerIdentity {
                author_id: "tamper-pkg".to_string(),
                name: "TestAuthor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        // Tamper with payload file
        let conf_file = bundle_dir.join(&manifest.files[0].source);
        fs::write(
            &conf_file,
            "theme_color = red
# Tampered line
",
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed);
        assert_eq!(
            report.signature_status,
            CryptographicStatus::InvalidSignature
        );
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("Tampering detected")));
    }

    #[test]
    fn test_ci_revoked_key_fails_closed() {
        let sandbox = TestSandbox::new("revoked-ci-fail");
        let (sign, verify) = generate_ed25519_keypair();
        let pub_hex = hex::encode(verify.as_bytes());
        let key_id = crate::crypto::compute_key_fingerprint(&verify);

        let mut trust_store = TrustStore::new_test_store(&[], &[], &[]);
        trust_store
            .revoke_key(&key_id, &pub_hex, "Key compromised in security incident")
            .unwrap();

        let draft = create_test_draft(&sandbox, "revoked-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let manifest = crate::installer::load_package_manifest(&bundle_dir).unwrap();
        let tree_hash = compute_package_tree_hash(&bundle_dir, &manifest).unwrap();

        let sig = sign_package_tree_hash(
            &sign,
            "revoked-pkg",
            &tree_hash,
            SignerIdentity {
                author_id: "revoked-pkg".to_string(),
                name: "RevokedAuthor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed);
        assert_eq!(report.signature_status, CryptographicStatus::RevokedKey);
        assert!(report.errors.iter().any(|e| e.contains("revoked")));
    }

    #[test]
    fn test_ci_official_impersonation_fails_closed() {
        let sandbox = TestSandbox::new("impersonate-ci-fail");
        let (sign, _) = generate_ed25519_keypair();
        let (_core_sign, core_verify) = generate_ed25519_keypair();
        let core_pub_hex = hex::encode(core_verify.as_bytes());

        // Core Root trust store has core_pub_hex, but package is signed by a different non-core key
        let trust_store = TrustStore::new_test_store(&[&core_pub_hex], &[], &[]);

        let draft = create_test_draft(&sandbox, "impersonator-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let manifest = crate::installer::load_package_manifest(&bundle_dir).unwrap();
        let tree_hash = compute_package_tree_hash(&bundle_dir, &manifest).unwrap();

        let sig = sign_package_tree_hash(
            &sign,
            "impersonator-pkg",
            &tree_hash,
            SignerIdentity {
                author_id: "impersonator-pkg".to_string(),
                name: "BadActor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        // Write release.json claiming TrustTier::Official
        let fake_release = DistributionRelease {
            schema_version: 1,
            package_id: "impersonator-pkg".to_string(),
            name: "Impersonator Pkg".to_string(),
            version: "1.0.0".to_string(),
            package_type: PackageType::Theme,
            channel: ReleaseChannel::Stable,
            author: AuthorInfo {
                name: "BadActor".to_string(),
                avatar: String::new(),
                verified: false,
            },
            maintainer: None,
            release_notes: "Claims official status".to_string(),
            published_at: "1000".to_string(),
            tree_hash: tree_hash.clone(),
            min_ryzora_version: "0.1.0".to_string(),
            trust_tier: TrustTier::Official, // MALICIOUS CLAIM
            files_count: 1,
            total_bytes: 100,
            signature: Some(sig),
        };

        fs::write(
            bundle_dir.join("release.json"),
            serde_json::to_string_pretty(&fake_release).unwrap(),
        )
        .unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!report.passed);
        assert_eq!(
            report.signature_status,
            CryptographicStatus::OfficialImpersonation
        );
        assert!(report.errors.iter().any(|e| e.contains("claims Official")));
    }

    #[test]
    fn test_ci_downgrade_prevention() {
        let sandbox = TestSandbox::new("downgrade-ci-test");
        let (_sign, _) = generate_ed25519_keypair();
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        // Setup repository with existing version 1.2.0
        let repo_dir = sandbox.root.join("repo");
        fs::create_dir_all(repo_dir.join("packages")).unwrap();
        let repo_index = RepositoryIndex {
            schema: 1,
            id: "test-repo".to_string(),
            name: "Test Repo".to_string(),
            version: "1.0.0".to_string(),
            description: "Repo for downgrade test".to_string(),
            packages: vec![RepositoryPackageEntry {
                id: "my-app-theme".to_string(),
                name: "My App Theme".to_string(),
                version: "1.2.0".to_string(), // EXISTING IS 1.2.0
                package_type: PackageType::Theme,
                description: "Existing package".to_string(),
                manifest: "packages/my-app-theme/ryzora.json".to_string(),
                category: "Theme".to_string(),
                tags: vec![],
                author: AuthorInfo {
                    name: "Author".to_string(),
                    avatar: String::new(),
                    verified: false,
                },
                color_palette: vec![],
                hero_image: None,
                screenshots: vec![],
                featured: None,
                trending: None,
                rating: None,
                downloads: None,
                content_hash: None,
                package_size_bytes: None,
                release_channel: None,
                trust_tier: None,
                moderation_status: None,
                trending_score: None,
                maintainer: None,
                release_notes: None,
                signature: None,
            }],
        };
        fs::write(
            repo_dir.join("repository.json"),
            serde_json::to_string_pretty(&repo_index).unwrap(),
        )
        .unwrap();

        // Submission attempts to publish older version 1.1.0
        let draft_older = create_test_draft(&sandbox, "my-app-theme", "1.1.0");
        let build_dir = sandbox.root.join("build_older");
        let pkg_res = crate::authoring::create_package_bundle(draft_older, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let report = run_ci_audit(&bundle_dir, Some(&repo_dir), &trust_store).unwrap();
        assert!(!report.passed);
        assert!(report
            .errors
            .iter()
            .any(|e| e.contains("SemVer downgrade or duplicate rejected")));

        // Submission attempts to publish duplicate version 1.2.0
        let draft_same = create_test_draft(&sandbox, "my-app-theme", "1.2.0");
        let build_dir2 = sandbox.root.join("build_same");
        let pkg_res2 = crate::authoring::create_package_bundle(draft_same, &build_dir2).unwrap();
        let bundle_dir2 = PathBuf::from(&pkg_res2.package_dir);

        let report2 = run_ci_audit(&bundle_dir2, Some(&repo_dir), &trust_store).unwrap();
        assert!(!report2.passed);
        assert!(report2
            .errors
            .iter()
            .any(|e| e.contains("SemVer downgrade or duplicate rejected")));

        // Submission publishes newer version 1.3.0 -> PASSES
        let draft_newer = create_test_draft(&sandbox, "my-app-theme", "1.3.0");
        let build_dir3 = sandbox.root.join("build_newer");
        let pkg_res3 = crate::authoring::create_package_bundle(draft_newer, &build_dir3).unwrap();
        let bundle_dir3 = PathBuf::from(&pkg_res3.package_dir);

        let report3 = run_ci_audit(&bundle_dir3, Some(&repo_dir), &trust_store).unwrap();
        assert!(
            report3.passed,
            "Upgrade to 1.3.0 must pass: {:?}",
            report3.errors
        );
    }

    #[test]
    fn test_ci_atomic_ingestion_and_rollback() {
        let sandbox = TestSandbox::new("atomic-ingest-test");
        let (sign, _verify) = generate_ed25519_keypair();
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let repo_dir = sandbox.root.join("community-repo");
        fs::create_dir_all(repo_dir.join("packages")).unwrap();
        let initial_index = RepositoryIndex {
            schema: 1,
            id: "community-repo".to_string(),
            name: "Community Repo".to_string(),
            version: "1.0.0".to_string(),
            description: "Testing atomic ingestion".to_string(),
            packages: vec![],
        };
        fs::write(
            repo_dir.join("repository.json"),
            serde_json::to_string_pretty(&initial_index).unwrap(),
        )
        .unwrap();

        // 1. Build and sign clean submission
        let draft = create_test_draft(&sandbox, "atomic-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let manifest = crate::installer::load_package_manifest(&bundle_dir).unwrap();
        let tree_hash = compute_package_tree_hash(&bundle_dir, &manifest).unwrap();

        let sig = sign_package_tree_hash(
            &sign,
            "atomic-pkg",
            &tree_hash,
            SignerIdentity {
                author_id: "atomic-pkg".to_string(),
                name: "AtomicAuthor".to_string(),
                handle: None,
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig).unwrap(),
        )
        .unwrap();

        // Ingest into repository
        let res = ingest_submission_into_repository(&bundle_dir, &repo_dir, &trust_store).unwrap();
        assert!(res.success);

        // Verify repository index was atomically updated
        let raw = fs::read_to_string(repo_dir.join("repository.json")).unwrap();
        let updated_index: RepositoryIndex = serde_json::from_str(&raw).unwrap();
        assert_eq!(updated_index.packages.len(), 1);
        let entry = &updated_index.packages[0];
        assert_eq!(entry.id, "atomic-pkg");
        assert_eq!(entry.trust_tier, Some(TrustTier::Community));
        assert_eq!(entry.moderation_status, Some(ModerationStatus::Approved));
        assert_eq!(entry.content_hash, Some(tree_hash));
        assert_eq!(entry.downloads, Some(0)); // PR metadata discarded

        // Verify package directory exists in repository
        assert!(repo_dir.join("packages/atomic-pkg/ryzora.json").is_file());
        assert!(repo_dir
            .join("packages/atomic-pkg/files/atomic-pkg/config.conf")
            .is_file());

        // Verify no lingering temp files or backups exist
        assert!(!repo_dir.join(".repository.json.tmp").exists());
        for entry in fs::read_dir(repo_dir.join("packages")).unwrap() {
            let p = entry.unwrap().path();
            let fname = p.file_name().unwrap().to_string_lossy();
            assert!(
                !fname.starts_with(".tmp_"),
                "Lingering temp folder: {}",
                fname
            );
        }
    }

    #[test]
    fn test_ci_pr_comment_markdown_generation() {
        let sandbox = TestSandbox::new("markdown-test");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "md-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        let md = &report.pr_comment_markdown;

        assert!(md.contains("CI Ingestion Audit: PASSED"));
        assert!(md.contains("md-pkg"));
        assert!(md.contains("1.0.0"));
        assert!(md.contains("Automated Inspection Matrix"));
        assert!(md.contains("Manifest Schema Validation"));
        assert!(md.contains("Zero-Executable & Zero-Hook Pre-Flight Scan"));
        assert!(md.contains("Canonical Tree Hash Computation"));
        assert!(md.contains("Ed25519 Cryptographic Authenticity & Trust Chain"));
        assert!(md.contains("Ryzora CI Automated Audit"));
    }

    #[test]
    fn test_ci_symlink_in_submission_rejected() {
        use std::os::unix::fs::symlink;

        let sandbox = TestSandbox::new("symlink-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "symlink-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let bundle_dir = PathBuf::from(&pkg_res.package_dir);

        // Inject an unsafe symlink into the package files
        let target_file = sandbox.root.join("external_secret.conf");
        fs::write(&target_file, "SUPER_SECRET_TOKEN=xyz123").unwrap();
        let symlink_path = bundle_dir.join("files/symlink-pkg/secret_symlink.conf");
        symlink(&target_file, &symlink_path).unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(
            !report.passed,
            "Symlinks in package submission must cause CI audit to FAIL"
        );
        assert_eq!(report.moderation_status, ModerationStatus::Flagged);
        assert_eq!(report.audit_score, 0);
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.contains("Symlinks are strictly prohibited")),
            "Expected symlink rejection error, got: {:?}",
            report.errors
        );
    }

    #[test]
    fn test_copy_dir_all_refuses_symlinks_and_avoids_dereferencing() {
        use std::os::unix::fs::symlink;

        let sandbox = TestSandbox::new("copy-symlink-refuse");
        let src_dir = sandbox.root.join("src");
        let dst_dir = sandbox.root.join("dst");
        fs::create_dir_all(&src_dir).unwrap();

        // Create regular file and symlink pointing outside src_dir
        fs::write(src_dir.join("regular.txt"), "hello").unwrap();
        let external_file = sandbox.root.join("host_secret.txt");
        fs::write(&external_file, "SECRET_DATA_DO_NOT_LEAK").unwrap();
        symlink(&external_file, src_dir.join("leaked_link.txt")).unwrap();

        // copy_dir_all must fail and refuse to dereference
        let res = copy_dir_all(&src_dir, &dst_dir);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Symlinks are strictly prohibited"));

        // Ensure host_secret was NOT copied as regular file into dst_dir
        assert!(!dst_dir.join("leaked_link.txt").exists());
    }

    #[test]
    fn test_ci_checksums_malformed_and_traversal_rejected() {
        let sandbox = TestSandbox::new("checksums-reject");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "chk-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let pkg_dir = PathBuf::from(&pkg_res.package_dir);

        let releases_dir = sandbox.root.join("releases");
        let dist_res = build_distribution_release_internal(
            &pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        let checksums_file = bundle_dir.join("checksums.sha256");
        let orig_checksums = fs::read_to_string(&checksums_file).unwrap();

        // Test Case A: Invalid non-hex SHA-256
        fs::write(
            &checksums_file,
            "not-a-valid-hex files/chk-pkg/chk-pkg-config.conf\n",
        )
        .unwrap();
        let rep_a = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!rep_a.passed);
        assert!(rep_a
            .errors
            .iter()
            .any(|e| e.contains("Invalid SHA-256 format")));

        // Test Case B: Path Traversal in checksums line
        let valid_dummy_sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        fs::write(
            &checksums_file,
            format!("{} ../../etc/passwd\n", valid_dummy_sha),
        )
        .unwrap();
        let rep_b = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!rep_b.passed);
        assert!(rep_b
            .errors
            .iter()
            .any(|e| e.contains("Path traversal or invalid path")));

        // Test Case C: Duplicate entry in checksums
        let dup_content = format!("{}{}", orig_checksums, orig_checksums);
        fs::write(&checksums_file, dup_content).unwrap();
        let rep_c = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!rep_c.passed);
        assert!(rep_c
            .errors
            .iter()
            .any(|e| e.contains("Duplicate entry in checksums.sha256")));

        // Test Case D: Checksum refers to nonexistent file
        fs::write(
            &checksums_file,
            format!("{} files/chk-pkg/nonexistent.conf\n", valid_dummy_sha),
        )
        .unwrap();
        let rep_d = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!rep_d.passed);
        assert!(rep_d
            .errors
            .iter()
            .any(|e| e.contains("does not exist on disk")));

        // Test Case E: Omission of declared manifest file
        fs::write(&checksums_file, "# empty checksum file\n").unwrap();
        let rep_e = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(!rep_e.passed);
        assert!(rep_e
            .errors
            .iter()
            .any(|e| e.contains("is missing from checksums.sha256")));
    }

    #[test]
    fn test_ci_distribution_bundle_missing_checksums_rejected() {
        let sandbox = TestSandbox::new("dist-no-chk");
        let trust_store = TrustStore::new_test_store(&[], &[], &[]);

        let draft = create_test_draft(&sandbox, "nochk-pkg", "1.0.0");
        let build_dir = sandbox.root.join("build");
        let pkg_res = crate::authoring::create_package_bundle(draft, &build_dir).unwrap();
        let pkg_dir = PathBuf::from(&pkg_res.package_dir);

        let releases_dir = sandbox.root.join("releases");
        let dist_res = build_distribution_release_internal(
            &pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        // Delete checksums.sha256 from the distribution bundle
        fs::remove_file(bundle_dir.join("checksums.sha256")).unwrap();

        let report = run_ci_audit(&bundle_dir, None, &trust_store).unwrap();
        assert!(
            !report.passed,
            "Missing checksums.sha256 in distribution bundle must fail audit"
        );
        assert_eq!(report.moderation_status, ModerationStatus::Flagged);
        assert!(report.errors.iter().any(|e| e.contains(
            "Missing required 'checksums.sha256' manifest in distribution release bundle"
        )));
    }

    #[test]
    fn test_ci_zero_command_execution() {
        let src_dir = Path::new("src");
        let forbidden = [
            "std::process::Command",
            "process::Command",
            "Command::new",
            r#""sh""#,
            r#""bash""#,
            r#""sudo""#,
            r#""pkexec""#,
            r#""pacman""#,
            r#""yay""#,
            r#""paru""#,
            r#""apt""#,
            r#""dnf""#,
            r#""gpg""#,
            r#""openssl""#,
        ];

        let file_path = src_dir.join("ingestion.rs");
        let code = fs::read_to_string(&file_path).unwrap();
        let prod_code = code.split("#[cfg(test)]").next().unwrap_or(&code);
        for pat in &forbidden {
            assert!(
                !prod_code.contains(pat),
                "Production code in ingestion.rs contains forbidden call '{}'",
                pat
            );
        }
    }
}
