//! Ryzora Restricted SDDM Privileged Helper
//!
//! This module is the ONLY code path that invokes privileged SDDM operations.
//! All operations go through:
//!   pkexec /usr/lib/ryzora/ryzora-sddm-helper <operation> <validated-args>
//!
//! The helper is authorized by Polkit (io.ryzora.sddm.manage-theme).
//! No arbitrary shell commands, no arbitrary paths — only named operations.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const HELPER_SYSTEM_PATH: &str = "/usr/lib/ryzora/ryzora-sddm-helper";
const SDDM_THEMES_DIR: &str = "/usr/share/sddm/themes";
const SDDM_CONF_FILE: &str = "/etc/sddm.conf.d/zz-ryzora-theme.conf";
pub const SDDM_LEGACY_CONF_FILE: &str = "/etc/sddm.conf.d/ryzora-theme.conf";
const SLUG_MAX_LEN: usize = 64;

/// Status and integrity of the Ryzora privileged SDDM helper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedHelperStatus {
    pub installed: bool,
    pub helper_path: String,
    pub helper_exists: bool,
    pub helper_executable: bool,
    pub helper_valid: bool,
    pub policy_path: String,
    pub policy_exists: bool,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub error: Option<String>,
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        if let Ok(meta) = fs::metadata(path) {
            return meta.permissions().mode() & 0o111 != 0;
        }
        false
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

pub fn sha256_file(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Some(format!("{:x}", result))
}

/// Validate a theme slug: [a-zA-Z0-9_-]{1,64}
pub fn validate_slug(slug: &str) -> Result<(), String> {
    if slug.is_empty() || slug.len() > SLUG_MAX_LEN {
        return Err(format!(
            "Theme slug '{}' must be 1–{} characters",
            slug, SLUG_MAX_LEN
        ));
    }
    if !slug
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!(
            "Theme slug '{}' contains invalid characters (allowed: a-z A-Z 0-9 - _)",
            slug
        ));
    }
    Ok(())
}

/// Result from a helper invocation
#[derive(Debug, Clone)]
pub struct HelperResult {
    pub success: bool,
    pub output: String,
    pub exit_code: i32,
}

/// Locate bundled helper script and Polkit action file
pub fn find_bundled_helper_resources() -> Result<(PathBuf, PathBuf), String> {
    let mut candidate_dirs = Vec::new();

    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        candidate_dirs.push(PathBuf::from(manifest_dir).join("resources"));
    }
    candidate_dirs.push(PathBuf::from("/usr/share/ryzora/resources"));
    candidate_dirs.push(PathBuf::from("/usr/lib/ryzora"));

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidate_dirs.push(parent.join("resources"));
            candidate_dirs.push(parent.join("../Resources/resources"));
            candidate_dirs.push(parent.join("../resources"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidate_dirs.push(cwd.join("src-tauri/resources"));
        candidate_dirs.push(cwd.join("resources"));
    }

    candidate_dirs.push(PathBuf::from("/home/silentbyte/Documents/Code Playground/Ryzora/src-tauri/resources"));

    for dir in &candidate_dirs {
        let helper = dir.join("ryzora-sddm-helper");
        let policy = dir.join("io.ryzora.sddm.policy");
        if helper.is_file() && policy.is_file() {
            return Ok((helper, policy));
        }
    }

    Err(format!(
        "Could not locate bundled 'ryzora-sddm-helper' and 'io.ryzora.sddm.policy'. Checked directories: {:?}",
        candidate_dirs
    ))
}

/// Detect whether the privileged helper and Polkit policy are installed and healthy
pub fn detect_privileged_helper_status_in(sys_root: Option<&Path>) -> PrivilegedHelperStatus {
    let (helper_path, policy_path) = if let Some(root) = sys_root {
        (
            root.join("usr/lib/ryzora/ryzora-sddm-helper"),
            root.join("usr/share/polkit-1/actions/io.ryzora.sddm.policy"),
        )
    } else {
        (
            PathBuf::from(HELPER_SYSTEM_PATH),
            PathBuf::from("/usr/share/polkit-1/actions/io.ryzora.sddm.policy"),
        )
    };

    let helper_exists = helper_path.is_file();
    let helper_executable = if helper_exists { is_executable(&helper_path) } else { false };
    let helper_content = if helper_exists { fs::read_to_string(&helper_path).ok() } else { None };
    let helper_valid = helper_content.as_deref().map_or(false, |c| {
        c.contains("ryzora-sddm-helper") && c.contains("validate_slug") && c.contains("install_theme")
    });
    let sha256 = if helper_exists { sha256_file(&helper_path) } else { None };

    let policy_exists = policy_path.is_file();
    let policy_content = if policy_exists { fs::read_to_string(&policy_path).ok() } else { None };
    let policy_valid = policy_content.as_deref().map_or(false, |c| {
        c.contains("io.ryzora.sddm")
    });

    let installed = helper_exists && helper_executable && helper_valid && policy_exists && policy_valid;

    let error = if !helper_exists {
        Some(format!("Helper binary not found at '{}'", helper_path.display()))
    } else if !helper_executable {
        Some(format!("Helper binary at '{}' is not executable (mode 755 required)", helper_path.display()))
    } else if !helper_valid {
        Some(format!("Helper binary at '{}' failed integrity check", helper_path.display()))
    } else if !policy_exists {
        Some(format!("Polkit policy file not found at '{}'", policy_path.display()))
    } else if !policy_valid {
        Some(format!("Polkit policy at '{}' is invalid", policy_path.display()))
    } else {
        None
    };

    PrivilegedHelperStatus {
        installed,
        helper_path: helper_path.display().to_string(),
        helper_exists,
        helper_executable,
        helper_valid,
        policy_path: policy_path.display().to_string(),
        policy_exists,
        version: Some("1.0.0".to_string()),
        sha256,
        error,
    }
}

pub fn detect_privileged_helper_status() -> PrivilegedHelperStatus {
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    detect_privileged_helper_status_in(sys_root.as_deref())
}

/// Explicit setup of the Ryzora privileged integration (helper + Polkit policy)
pub fn setup_privileged_helper_in(sys_root: Option<&Path>) -> Result<PrivilegedHelperStatus, String> {
    let (helper_src, policy_src) = find_bundled_helper_resources()?;

    if let Some(root) = sys_root {
        // Test mode: direct installation into test root
        let target_helper = root.join("usr/lib/ryzora/ryzora-sddm-helper");
        let target_policy = root.join("usr/share/polkit-1/actions/io.ryzora.sddm.policy");

        if let Some(parent) = target_helper.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create helper dir: {}", e))?;
        }
        if let Some(parent) = target_policy.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create policy dir: {}", e))?;
        }

        fs::copy(&helper_src, &target_helper)
            .map_err(|e| format!("Failed to copy helper: {}", e))?;
        #[cfg(unix)]
        {
            fs::set_permissions(&target_helper, fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("Failed to chmod helper: {}", e))?;
        }
        fs::copy(&policy_src, &target_policy)
            .map_err(|e| format!("Failed to copy policy: {}", e))?;

        let status = detect_privileged_helper_status_in(Some(root));
        if !status.installed {
            return Err(status.error.unwrap_or_else(|| "Validation failed after test installation".to_string()));
        }
        return Ok(status);
    }

    // Live development mode: single Polkit authentication dialog
    let helper_str = helper_src.to_str().ok_or("Invalid UTF-8 in helper source path")?;
    let policy_str = policy_src.to_str().ok_or("Invalid UTF-8 in policy source path")?;

    let script = r#"
set -euo pipefail
mkdir -p /usr/lib/ryzora /usr/share/polkit-1/actions
cp "$1" /usr/lib/ryzora/ryzora-sddm-helper
chmod 755 /usr/lib/ryzora/ryzora-sddm-helper
chown root:root /usr/lib/ryzora/ryzora-sddm-helper 2>/dev/null || true
cp "$2" /usr/share/polkit-1/actions/io.ryzora.sddm.policy
chmod 644 /usr/share/polkit-1/actions/io.ryzora.sddm.policy
chown root:root /usr/share/polkit-1/actions/io.ryzora.sddm.policy 2>/dev/null || true
"#;

    let output = Command::new("pkexec")
        .args(["bash", "-c", script, "--", helper_str, policy_str])
        .output()
        .map_err(|e| format!("Failed to invoke pkexec for system integration setup: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Administrator authorization was cancelled or failed: {}",
            err_msg.trim()
        ));
    }

    let status = detect_privileged_helper_status_in(None);
    if !status.installed {
        return Err(format!(
            "Helper setup executed but post-install verification failed: {}",
            status.error.unwrap_or_else(|| "Unknown validation error".to_string())
        ));
    }

    Ok(status)
}

#[tauri::command]
pub fn get_privileged_helper_status() -> PrivilegedHelperStatus {
    detect_privileged_helper_status()
}

#[tauri::command]
pub fn setup_privileged_helper() -> Result<PrivilegedHelperStatus, String> {
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    setup_privileged_helper_in(sys_root.as_deref())
}

/// Invoke the Ryzora SDDM privileged helper via pkexec.
/// In test mode (RYZORA_SYSTEM_ROOT set), invokes the helper directly without pkexec.
fn invoke_helper(operation: &str, args: &[&str]) -> Result<HelperResult, String> {
    let helper_path = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root).join("usr/lib/ryzora/ryzora-sddm-helper")
    } else {
        PathBuf::from(HELPER_SYSTEM_PATH)
    };

    if !helper_path.exists() {
        return Err(format!(
            "Ryzora SDDM helper not found at '{}'. \
             Please set up System Integration before performing SDDM operations.",
            helper_path.display()
        ));
    }

    let use_pkexec = std::env::var("RYZORA_SYSTEM_ROOT").is_err();

    let mut cmd = if use_pkexec {
        let mut c = Command::new("pkexec");
        c.arg(helper_path.as_os_str());
        c
    } else {
        Command::new(helper_path.as_os_str())
    };

    cmd.arg(operation);
    for arg in args {
        cmd.arg(arg);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to invoke SDDM helper: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(HelperResult {
        success: output.status.success(),
        output: combined,
        exit_code,
    })
}

/// Install an SDDM theme from a staging directory.
pub fn install_sddm_theme(staging_dir: &Path, slug: &str) -> Result<(), String> {
    validate_slug(slug)?;

    let staging_str = staging_dir
        .to_str()
        .ok_or("Staging path contains invalid UTF-8")?;

    let effective_staging = if let Ok(_sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        staging_str.to_string()
    } else {
        staging_str.to_string()
    };

    let result = invoke_helper("install_theme", &[&effective_staging, slug])?;

    if !result.success {
        return Err(format!(
            "SDDM theme installation failed (exit {}): {}",
            result.exit_code, result.output
        ));
    }

    // Physical verification
    let dest = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root)
            .join("usr/share/sddm/themes")
            .join(format!("ryzora-{}", slug))
    } else {
        PathBuf::from(SDDM_THEMES_DIR).join(format!("ryzora-{}", slug))
    };

    if !dest.exists() {
        return Err(format!(
            "Physical verification failed: theme directory '{}' was not created",
            dest.display()
        ));
    }
    if !dest.join("Main.qml").exists() {
        return Err("Physical verification failed: Main.qml is missing from installed theme".to_string());
    }
    if !dest.join("metadata.desktop").exists() && !dest.join("theme.conf").exists() {
        return Err("Physical verification failed: metadata file is missing from installed theme".to_string());
    }

    Ok(())
}

/// Remove a Ryzora-managed SDDM theme.
pub fn remove_sddm_theme(slug: &str) -> Result<(), String> {
    validate_slug(slug)?;
    let result = invoke_helper("remove_theme", &[slug])?;
    if !result.success {
        return Err(format!(
            "SDDM theme removal failed (exit {}): {}",
            result.exit_code, result.output
        ));
    }
    Ok(())
}

/// Activate a Ryzora SDDM theme by writing /etc/sddm.conf.d/ryzora-theme.conf
/// Activate a Ryzora SDDM theme by writing /etc/sddm.conf.d/zz-ryzora-theme.conf
pub fn activate_sddm_theme(slug: &str) -> Result<(), String> {
    validate_slug(slug)?;
    let result = invoke_helper("activate_theme", &[slug])?;
    if !result.success {
        return Err(format!(
            "SDDM theme activation failed (exit {}): {}",
            result.exit_code, result.output
        ));
    }

    // Physical verification
    let conf_path = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root).join("etc/sddm.conf.d/zz-ryzora-theme.conf")
    } else {
        PathBuf::from(SDDM_CONF_FILE)
    };

    if !conf_path.exists() {
        return Err(format!(
            "Physical verification failed: SDDM config was not created at '{}'",
            conf_path.display()
        ));
    }

    let content = fs::read_to_string(&conf_path)
        .map_err(|e| format!("Failed to read active SDDM conf for verification: {}", e))?;

    let expected = format!("Current=ryzora-{}", slug);
    if !content.contains(&expected) {
        return Err(format!(
            "Physical verification failed: active SDDM conf does not contain '{}'. Got:\n{}",
            expected, content
        ));
    }

    // Effective theme verification according to SDDM configuration precedence
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    let resolution = resolve_effective_sddm_theme_in(sys_root.as_deref());
    let expected_theme = format!("ryzora-{}", slug);

    if resolution.effective_theme.as_deref() != Some(&expected_theme) {
        let override_info = if let Some(ref over_file) = resolution.overridden_by {
            format!("Overridden by '{}'", over_file.display())
        } else if let Some(ref eff_file) = resolution.effective_file {
            format!("Effective theme is defined in '{}'", eff_file.display())
        } else {
            "No effective SDDM theme could be resolved".to_string()
        };
        return Err(format!(
            "SDDM configuration was written to '{}', but is not effective. Effective theme is '{:?}' ({}). Ensure Ryzora configuration has precedence.",
            conf_path.display(),
            resolution.effective_theme,
            override_info
        ));
    }

    Ok(())
}

/// Deactivate Ryzora SDDM theme by removing Ryzora-owned drop-in configuration files
pub fn deactivate_sddm_theme() -> Result<(), String> {
    let result = invoke_helper("deactivate_theme", &[])?;
    if !result.success {
        return Err(format!(
            "SDDM theme deactivation failed (exit {}): {}",
            result.exit_code, result.output
        ));
    }
    Ok(())
}

/// Restore a previous SDDM theme (not necessarily Ryzora-managed)
pub fn restore_sddm_theme(previous_theme_name: &str) -> Result<(), String> {
    if previous_theme_name.is_empty() || previous_theme_name == "default" || previous_theme_name == "none" {
        return deactivate_sddm_theme();
    }
    validate_slug(previous_theme_name)?;
    let result = invoke_helper("restore_theme", &[previous_theme_name])?;
    if !result.success {
        return Err(format!(
            "SDDM theme restoration failed (exit {}): {}",
            result.exit_code, result.output
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SddmConfigEntrySummary {
    pub path: String,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SddmEffectiveResolution {
    pub effective_theme: Option<String>,
    pub effective_file: Option<PathBuf>,
    pub entries: Vec<SddmConfigEntrySummary>,
    pub is_ryzora: bool,
    pub is_overridden: bool,
    pub overridden_by: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SddmRuntimeStatus {
    pub available: bool,
    pub helper_installed: bool,
    pub installed: bool,
    pub applied: bool,
    pub active: bool,
    pub ryzora_theme: Option<String>,
    pub effective_theme: Option<String>,
    pub effective_file: Option<String>,
    pub is_overridden: bool,
    pub overridden_by: Option<String>,
    pub previous_theme: Option<String>,
    pub config_entries: Vec<SddmConfigEntrySummary>,
    pub error: Option<String>,
}

fn parse_sddm_conf_current(file: &Path, entries: &mut Vec<SddmConfigEntrySummary>) {
    if let Ok(content) = fs::read_to_string(file) {
        let mut in_theme_section = false;
        let has_sections = content.lines().any(|l| {
            let t = l.trim();
            t.starts_with('[') && t.ends_with(']')
        });

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                in_theme_section = trimmed.eq_ignore_ascii_case("[Theme]");
                continue;
            }
            if in_theme_section || !has_sections {
                if trimmed.starts_with("Current=") || trimmed.starts_with("Current =") {
                    if let Some(val) = trimmed.splitn(2, '=').nth(1) {
                        let val = val.trim();
                        if !val.is_empty() {
                            entries.push(SddmConfigEntrySummary {
                                path: file.display().to_string(),
                                theme: val.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
}

/// Resolves the EFFECTIVE SDDM theme according to SDDM configuration precedence:
/// 1. /usr/lib/sddm/sddm.conf.d/*.conf in alphabetical order
/// 2. /etc/sddm.conf.d/*.conf in alphabetical order (later files override earlier)
/// 3. /etc/sddm.conf (overrides all drop-ins)
pub fn resolve_effective_sddm_theme_in(sys_root: Option<&Path>) -> SddmEffectiveResolution {
    let mut entries = Vec::new();

    // 1. /usr/lib/sddm/sddm.conf.d/
    let sys_conf_dir = if let Some(root) = sys_root {
        root.join("usr/lib/sddm/sddm.conf.d")
    } else {
        PathBuf::from("/usr/lib/sddm/sddm.conf.d")
    };
    if let Ok(rd) = fs::read_dir(&sys_conf_dir) {
        let mut sys_files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("conf"))
            .collect();
        sys_files.sort();
        for file in sys_files {
            parse_sddm_conf_current(&file, &mut entries);
        }
    }

    // 2. /etc/sddm.conf.d/
    let local_conf_dir = if let Some(root) = sys_root {
        root.join("etc/sddm.conf.d")
    } else {
        PathBuf::from("/etc/sddm.conf.d")
    };
    if let Ok(rd) = fs::read_dir(&local_conf_dir) {
        let mut local_files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("conf"))
            .collect();
        local_files.sort();
        for file in local_files {
            parse_sddm_conf_current(&file, &mut entries);
        }
    }

    // 3. /etc/sddm.conf
    let root_conf = if let Some(root) = sys_root {
        root.join("etc/sddm.conf")
    } else {
        PathBuf::from("/etc/sddm.conf")
    };
    if root_conf.is_file() {
        parse_sddm_conf_current(&root_conf, &mut entries);
    }

    let last_entry = entries.last().cloned();
    let effective_theme = last_entry.as_ref().map(|e| e.theme.clone());
    let effective_file = last_entry.as_ref().map(|e| PathBuf::from(&e.path));

    let is_ryzora = effective_theme
        .as_deref()
        .map_or(false, |t| t.starts_with("ryzora-"));

    let mut ryzora_entry = None;
    for entry in &entries {
        if entry.path.contains("zz-ryzora-theme.conf") || entry.path.contains("ryzora-theme.conf") {
            ryzora_entry = Some(entry.clone());
        }
    }

    let (is_overridden, overridden_by) = if let Some(ref r_entry) = ryzora_entry {
        if let Some(ref last) = last_entry {
            if last.path != r_entry.path {
                (true, Some(PathBuf::from(&last.path)))
            } else {
                (false, None)
            }
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    SddmEffectiveResolution {
        effective_theme,
        effective_file,
        entries,
        is_ryzora,
        is_overridden,
        overridden_by,
    }
}

/// Read the current SDDM active theme from SDDM configuration files.
/// Returns (theme_name, source_file_path)
pub fn read_current_sddm_theme_in(sys_root: Option<&Path>) -> (Option<String>, Option<PathBuf>) {
    let res = resolve_effective_sddm_theme_in(sys_root);
    (res.effective_theme, res.effective_file)
}

/// Comprehensive runtime diagnostic of SDDM state on the machine
pub fn get_sddm_runtime_status_in(
    sys_root: Option<&Path>,
    state: &crate::installer::ActiveLockscreenState,
    package_id: Option<&str>,
) -> SddmRuntimeStatus {
    let (has_sddm, _) = crate::system::check_binary("sddm");
    let helper_status = detect_privileged_helper_status_in(sys_root);
    let resolution = resolve_effective_sddm_theme_in(sys_root);

    let sddm_themes_dir = if let Some(root) = sys_root {
        root.join("usr/share/sddm/themes")
    } else {
        PathBuf::from(SDDM_THEMES_DIR)
    };

    let (is_installed, _pkg_slug) = if let Some(pkg) = package_id {
        let slug = pkg.strip_prefix("lockscreen-qylock-").unwrap_or(pkg);
        (sddm_themes_dir.join(format!("ryzora-{}", slug)).is_dir(), Some(slug.to_string()))
    } else {
        let any_installed = fs::read_dir(&sddm_themes_dir).ok().map_or(false, |rd| {
            rd.flatten().any(|entry| entry.file_name().to_string_lossy().starts_with("ryzora-"))
        });
        (any_installed, None)
    };

    let applied = state.sddm.is_some();
    let expected_ryzora_theme = state.sddm.as_deref().map(|s| {
        let slug = s.strip_prefix("lockscreen-qylock-").unwrap_or(s);
        format!("ryzora-{}", slug)
    });

    let active = applied
        && resolution.effective_theme.is_some()
        && resolution.effective_theme == expected_ryzora_theme;

    let error = if !helper_status.installed {
        Some("Ryzora SDDM privileged helper not installed".to_string())
    } else if applied && resolution.is_overridden {
        Some(format!(
            "Ryzora configuration is overridden by '{}'",
            resolution.overridden_by.as_deref().map_or("another file", |p| p.to_str().unwrap_or("unknown"))
        ))
    } else {
        None
    };

    SddmRuntimeStatus {
        available: has_sddm,
        helper_installed: helper_status.installed,
        installed: is_installed,
        applied,
        active,
        ryzora_theme: expected_ryzora_theme,
        effective_theme: resolution.effective_theme,
        effective_file: resolution.effective_file.map(|p| p.display().to_string()),
        is_overridden: resolution.is_overridden,
        overridden_by: resolution.overridden_by.map(|p| p.display().to_string()),
        previous_theme: state.sddm_previous_theme.clone(),
        config_entries: resolution.entries,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    use crate::TEST_ENV_MUTEX as ENV_MUTEX;

    fn unique_test_dir(name: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::path::PathBuf::from("/tmp").join(format!("ryzora-staging-test-{}-{}", name, id))
    }

    struct TestDir(std::path::PathBuf);
    impl TestDir {
        fn new(name: &str) -> Self {
            let p = unique_test_dir(name);
            fs::create_dir_all(&p).unwrap();
            TestDir(p)
        }
        fn path(&self) -> &std::path::Path { &self.0 }
    }
    impl Drop for TestDir {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    fn setup_test_helper(sys_root: &std::path::Path) {
        let helper_dir = sys_root.join("usr/lib/ryzora");
        fs::create_dir_all(&helper_dir).unwrap();
        let helper_path = helper_dir.join("ryzora-sddm-helper");

        let real_helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/ryzora-sddm-helper");

        if real_helper.exists() {
            fs::copy(&real_helper, &helper_path).unwrap();
        } else {
            fs::write(&helper_path, r#"#!/usr/bin/env bash
set -euo pipefail
# ryzora-sddm-helper
OPERATION="$1"; shift
SYS="${RYZORA_SYSTEM_ROOT:-}"
validate_slug() { :; }
install_theme() { :; }
case "$OPERATION" in
  install_theme)
    staging="$1"; slug="$2"
    dest="${SYS}/usr/share/sddm/themes/ryzora-${slug}"
    [ -f "${staging}/Main.qml" ] || { echo "ERROR: missing Main.qml" >&2; exit 1; }
    mkdir -p "$dest"
    cp -r "${staging}/." "$dest/"
    echo "OK:installed:$dest" ;;
  remove_theme)
    slug="$1"
    dest="${SYS}/usr/share/sddm/themes/ryzora-${slug}"
    rm -rf "$dest"
    echo "OK:removed:$dest" ;;
  activate_theme)
    slug="$1"
    dest="${SYS}/usr/share/sddm/themes/ryzora-${slug}"
    [ -d "$dest" ] || { echo "ERROR: theme not installed" >&2; exit 1; }
    conf="${SYS}/etc/sddm.conf.d/zz-ryzora-theme.conf"
    mkdir -p "$(dirname "$conf")"
    printf '[Theme]\nCurrent=ryzora-%s\n' "$slug" > "$conf"
    echo "OK:activated:ryzora-$slug" ;;
  restore_theme)
    theme="$1"
    conf="${SYS}/etc/sddm.conf.d/zz-ryzora-theme.conf"
    mkdir -p "$(dirname "$conf")"
    printf '[Theme]\nCurrent=%s\n' "$theme" > "$conf"
    echo "OK:restored:$theme" ;;
  *) echo "unknown op" >&2; exit 1 ;;
esac
"#).unwrap();
        }
        #[cfg(unix)]
        {
            fs::set_permissions(&helper_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Also setup dummy polkit policy
        let polkit_dir = sys_root.join("usr/share/polkit-1/actions");
        fs::create_dir_all(&polkit_dir).unwrap();
        fs::write(polkit_dir.join("io.ryzora.sddm.policy"), "<policyconfig><action id=\"io.ryzora.sddm\"></action></policyconfig>").unwrap();
    }

    fn make_valid_staging(base: &std::path::Path, slug: &str) -> std::path::PathBuf {
        let theme_dir = base.join(format!("ryzora-{}", slug));
        fs::create_dir_all(&theme_dir).unwrap();
        fs::write(theme_dir.join("Main.qml"), "import QtQuick\nItem {}").unwrap();
        fs::write(theme_dir.join("metadata.desktop"), "[SddmGreeterTheme]\nName=Test\n").unwrap();
        fs::write(theme_dir.join("theme.conf"), "[General]\nbackground=\n").unwrap();
        theme_dir
    }

    #[test]
    fn test_validate_slug_accepts_valid() {
        assert!(validate_slug("dog-samurai").is_ok());
        assert!(validate_slug("my_theme_01").is_ok());
        assert!(validate_slug("winter").is_ok());
        assert!(validate_slug("CAPS").is_ok());
        assert!(validate_slug("a").is_ok());
        assert!(validate_slug(&"a".repeat(64)).is_ok());
    }

    #[test]
    fn test_validate_slug_rejects_invalid() {
        assert!(validate_slug("").is_err());
        assert!(validate_slug("theme with spaces").is_err());
        assert!(validate_slug("theme/slash").is_err());
        assert!(validate_slug("../../etc/passwd").is_err());
        assert!(validate_slug(&"a".repeat(65)).is_err());
    }

    #[test]
    fn test_detect_privileged_helper_status_uninstalled() {
        let sys_root = TestDir::new("sddm-detect-missing");
        let status = detect_privileged_helper_status_in(Some(sys_root.path()));
        assert!(!status.installed);
        assert!(!status.helper_exists);
        assert!(!status.policy_exists);
    }

    #[test]
    fn test_detect_privileged_helper_status_installed() {
        let sys_root = TestDir::new("sddm-detect-installed");
        setup_test_helper(sys_root.path());
        let status = detect_privileged_helper_status_in(Some(sys_root.path()));
        assert!(status.installed, "Status error: {:?}", status.error);
        assert!(status.helper_exists);
        assert!(status.helper_executable);
        assert!(status.helper_valid);
        assert!(status.policy_exists);
    }

    #[test]
    fn test_setup_privileged_helper_in_test_root() {
        let sys_root = TestDir::new("sddm-setup-root");
        let status = setup_privileged_helper_in(Some(sys_root.path())).unwrap();
        assert!(status.installed);
        assert!(sys_root.path().join("usr/lib/ryzora/ryzora-sddm-helper").exists());
        assert!(sys_root.path().join("usr/share/polkit-1/actions/io.ryzora.sddm.policy").exists());
    }

    #[test]
    fn test_sddm_install_activate_restore_transaction() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sys_root = TestDir::new("sddm-txn");
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());
        setup_test_helper(sys_root.path());

        let staging_base = TestDir::new("staging");
        let staging_theme = make_valid_staging(staging_base.path(), "dog-samurai");

        // Install
        install_sddm_theme(&staging_theme, "dog-samurai").unwrap();

        let installed = sys_root.path().join("usr/share/sddm/themes/ryzora-dog-samurai");
        assert!(installed.exists(), "Theme dir must exist after install");
        assert!(installed.join("Main.qml").exists());

        // Activate
        activate_sddm_theme("dog-samurai").unwrap();

        let conf = sys_root.path().join("etc/sddm.conf.d/zz-ryzora-theme.conf");
        assert!(conf.exists());
        let content = fs::read_to_string(&conf).unwrap();
        assert!(content.contains("Current=ryzora-dog-samurai"), "conf: {}", content);

        // Restore
        restore_sddm_theme("winter").unwrap();
        let content2 = fs::read_to_string(&conf).unwrap();
        assert!(content2.contains("Current=winter"), "conf: {}", content2);

        // Remove
        remove_sddm_theme("dog-samurai").unwrap();
        assert!(!installed.exists(), "Theme dir must be gone after remove");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_sddm_install_fails_on_missing_main_qml() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sys_root = TestDir::new("sddm-nomain");
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());
        setup_test_helper(sys_root.path());

        let staging_base = TestDir::new("staging-bad");
        let staging_theme = staging_base.path().join("ryzora-bad-theme");
        fs::create_dir_all(&staging_theme).unwrap();
        fs::write(staging_theme.join("random.txt"), "not a theme").unwrap();

        let result = install_sddm_theme(&staging_theme, "bad-theme");
        assert!(result.is_err(), "Install must fail without Main.qml");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }


    #[test]
    fn test_resolve_effective_sddm_theme_order() {
        let sys_root = TestDir::new("sddm-order");
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();

        fs::write(conf_dir.join("10-greeter.conf"), "[General]\nGreeterEnvironment=QML\n").unwrap();
        fs::write(conf_dir.join("kde_settings.conf"), "[Theme]\nCurrent=sugar-candy\n").unwrap();
        fs::write(conf_dir.join("theme.conf"), "[Theme]\nCurrent=winter\n").unwrap();

        let res = resolve_effective_sddm_theme_in(Some(sys_root.path()));
        assert_eq!(res.effective_theme, Some("winter".to_string()));
        assert_eq!(res.effective_file, Some(conf_dir.join("theme.conf")));
        assert!(!res.is_overridden);
        assert!(!res.is_ryzora);

        // Add Ryzora zz- entry
        fs::write(conf_dir.join("zz-ryzora-theme.conf"), "[Theme]\nCurrent=ryzora-dog-samurai\n").unwrap();
        let res2 = resolve_effective_sddm_theme_in(Some(sys_root.path()));
        assert_eq!(res2.effective_theme, Some("ryzora-dog-samurai".to_string()));
        assert_eq!(res2.effective_file, Some(conf_dir.join("zz-ryzora-theme.conf")));
        assert!(res2.is_ryzora);
        assert!(!res2.is_overridden);

        // Remove Ryzora zz- entry (deactivation)
        fs::remove_file(conf_dir.join("zz-ryzora-theme.conf")).unwrap();
        let res3 = resolve_effective_sddm_theme_in(Some(sys_root.path()));
        assert_eq!(res3.effective_theme, Some("winter".to_string()));
        assert_eq!(res3.effective_file, Some(conf_dir.join("theme.conf")));
        assert!(!res3.is_ryzora);
    }

    #[test]
    fn test_resolve_effective_sddm_theme_detects_overridden() {
        let sys_root = TestDir::new("sddm-overridden");
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();

        // Ryzora written as ryzora-theme.conf (which is alphabetically before theme.conf)
        fs::write(conf_dir.join("ryzora-theme.conf"), "[Theme]\nCurrent=ryzora-forest\n").unwrap();
        fs::write(conf_dir.join("theme.conf"), "[Theme]\nCurrent=winter\n").unwrap();

        let res = resolve_effective_sddm_theme_in(Some(sys_root.path()));
        assert_eq!(res.effective_theme, Some("winter".to_string()));
        assert_eq!(res.effective_file, Some(conf_dir.join("theme.conf")));
        assert!(res.is_overridden);
        assert_eq!(res.overridden_by, Some(conf_dir.join("theme.conf")));
    }

    #[test]
    fn test_read_current_sddm_theme() {
        let sys_root = TestDir::new("sddm-read");
        let conf_dir = sys_root.path().join("etc/sddm.conf.d");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("kde_settings.conf"), "[Theme]\nCurrent=winter\n").unwrap();

        let (theme, path) = read_current_sddm_theme_in(Some(sys_root.path()));
        assert_eq!(theme, Some("winter".to_string()));
        assert!(path.is_some());
    }

    #[test]
    fn test_activate_theme_requires_installed_theme() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sys_root = TestDir::new("sddm-noinstall");
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());
        setup_test_helper(sys_root.path());

        let result = activate_sddm_theme("nonexistent-theme");
        assert!(result.is_err(), "Activate must fail if theme not installed");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_double_install_rejected() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|p| p.into_inner());
        let sys_root = TestDir::new("sddm-dbl");
        std::env::set_var("RYZORA_SYSTEM_ROOT", sys_root.path());
        setup_test_helper(sys_root.path());

        let staging_base = TestDir::new("staging-dbl");
        let staging_theme = make_valid_staging(staging_base.path(), "dog-samurai");

        install_sddm_theme(&staging_theme, "dog-samurai").unwrap();

        let result = install_sddm_theme(&staging_theme, "dog-samurai");
        assert!(result.is_err(), "Second install of same slug must be rejected");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }
}
