//! Ryzora Privileged Package Helper Integration — Phase 23E
//!
//! Exclusively coordinates privileged package operations via:
//!   pkexec /usr/lib/ryzora/ryzora-package-helper <operation> <package_name>
//!
//! Authorized by Polkit (io.ryzora.package.manage).
//! Scoped strictly to Ryzora's approved helper; zero arbitrary root execution.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub const PACKAGE_HELPER_SYSTEM_PATH: &str = "/usr/lib/ryzora/ryzora-package-helper";
pub const PACKAGE_POLICY_SYSTEM_PATH: &str = "/usr/share/polkit-1/actions/io.ryzora.package.policy";
pub const PACKAGE_RULES_SYSTEM_PATH: &str = "/usr/share/polkit-1/rules.d/io.ryzora.package.rules";

/// Status and integrity of the Ryzora privileged package helper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivilegedPackageHelperStatus {
    pub installed: bool,
    pub helper_path: String,
    pub helper_exists: bool,
    pub helper_executable: bool,
    pub helper_valid: bool,
    pub policy_path: String,
    pub policy_exists: bool,
    pub rules_path: String,
    pub rules_exists: bool,
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

fn compute_sha256(path: &Path) -> Option<String> {
    fs::read(path).ok().map(|bytes| {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hex::encode(hasher.finalize())
    })
}

/// Detects the installation and integrity status of the package helper
pub fn detect_privileged_package_helper_status_in(sys_root: Option<&Path>) -> PrivilegedPackageHelperStatus {
    let helper_path = match sys_root {
        Some(root) => root.join("usr/lib/ryzora/ryzora-package-helper"),
        None => PathBuf::from(PACKAGE_HELPER_SYSTEM_PATH),
    };

    let policy_path = match sys_root {
        Some(root) => root.join("usr/share/polkit-1/actions/io.ryzora.package.policy"),
        None => PathBuf::from(PACKAGE_POLICY_SYSTEM_PATH),
    };

    let rules_path = match sys_root {
        Some(root) => root.join("usr/share/polkit-1/rules.d/io.ryzora.package.rules"),
        None => PathBuf::from(PACKAGE_RULES_SYSTEM_PATH),
    };

    let helper_exists = helper_path.exists();
    let helper_executable = is_executable(&helper_path);
    let policy_exists = policy_path.exists();
    let rules_exists = rules_path.exists();

    let mut helper_valid = false;
    let mut error = None;

    if helper_exists {
        if !helper_executable {
            error = Some("Package helper file exists but is not executable".to_string());
        } else if let Ok(content) = fs::read_to_string(&helper_path) {
            if content.contains("ryzora-package-helper") && content.contains("pacman") {
                helper_valid = true;
            } else {
                error = Some("Package helper file content does not match Ryzora signature".to_string());
            }
        } else {
            error = Some("Could not read package helper file".to_string());
        }
    } else {
        error = Some("Ryzora package helper is not installed".to_string());
    }

    if policy_exists {
        if let Ok(content) = fs::read_to_string(&policy_path) {
            if !content.contains("io.ryzora.package.manage") {
                error = Some("Polkit policy file does not declare io.ryzora.package.manage action".to_string());
            }
        }
    } else if error.is_none() {
        error = Some("Polkit policy file for Ryzora package management is not installed".to_string());
    }

    let installed = helper_exists && helper_executable && helper_valid && policy_exists;
    let sha256 = if helper_exists { compute_sha256(&helper_path) } else { None };

    PrivilegedPackageHelperStatus {
        installed,
        helper_path: helper_path.display().to_string(),
        helper_exists,
        helper_executable,
        helper_valid,
        policy_path: policy_path.display().to_string(),
        policy_exists,
        rules_path: rules_path.display().to_string(),
        rules_exists,
        sha256,
        error,
    }
}

pub fn detect_privileged_package_helper_status() -> PrivilegedPackageHelperStatus {
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    detect_privileged_package_helper_status_in(sys_root.as_deref())
}

/// Locates bundled package helper resources from binary directory, manifest dir, or current dir
pub fn find_bundled_package_helper_resources() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let candidate_dirs = [
        PathBuf::from("resources"),
        PathBuf::from("src-tauri/resources"),
        PathBuf::from("../resources"),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("resources")))
            .unwrap_or_else(|| PathBuf::from("resources")),
    ];

    for dir in &candidate_dirs {
        let helper = dir.join("ryzora-package-helper");
        let policy = dir.join("io.ryzora.package.policy");
        let rules = dir.join("io.ryzora.package.rules");
        if helper.exists() && policy.exists() {
            return Ok((helper, policy, rules));
        }
    }

    Err("Could not find bundled package helper resources".to_string())
}

/// Explicit setup of the Ryzora privileged package helper integration
pub fn setup_privileged_package_helper_in(sys_root: Option<&Path>) -> Result<PrivilegedPackageHelperStatus, String> {
    let (helper_src, policy_src, rules_src) = find_bundled_package_helper_resources()?;

    if let Some(root) = sys_root {
        // Test mode: install into test root
        let target_helper = root.join("usr/lib/ryzora/ryzora-package-helper");
        let target_policy = root.join("usr/share/polkit-1/actions/io.ryzora.package.policy");
        let target_rules = root.join("usr/share/polkit-1/rules.d/io.ryzora.package.rules");

        if let Some(parent) = target_helper.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create helper dir: {}", e))?;
        }
        if let Some(parent) = target_policy.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create policy dir: {}", e))?;
        }
        if let Some(parent) = target_rules.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create rules dir: {}", e))?;
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
        #[cfg(unix)]
        {
            fs::set_permissions(&target_policy, fs::Permissions::from_mode(0o644))
                .map_err(|e| format!("Failed to chmod policy: {}", e))?;
        }

        if rules_src.exists() {
            fs::copy(&rules_src, &target_rules)
                .map_err(|e| format!("Failed to copy rules: {}", e))?;
            #[cfg(unix)]
            {
                fs::set_permissions(&target_rules, fs::Permissions::from_mode(0o644))
                    .map_err(|e| format!("Failed to chmod rules: {}", e))?;
            }
        }

        return Ok(detect_privileged_package_helper_status_in(Some(root)));
    }

    // Live mode: invoke setup script via pkexec once
    let helper_str = helper_src.to_string_lossy();
    let policy_str = policy_src.to_string_lossy();
    let rules_str = rules_src.to_string_lossy();

    let script = r#"
mkdir -p /usr/lib/ryzora
mkdir -p /usr/share/polkit-1/actions
mkdir -p /usr/share/polkit-1/rules.d

cp "$1" /usr/lib/ryzora/ryzora-package-helper
chmod 755 /usr/lib/ryzora/ryzora-package-helper
chown root:root /usr/lib/ryzora/ryzora-package-helper 2>/dev/null || true

cp "$2" /usr/share/polkit-1/actions/io.ryzora.package.policy
chmod 644 /usr/share/polkit-1/actions/io.ryzora.package.policy
chown root:root /usr/share/polkit-1/actions/io.ryzora.package.policy 2>/dev/null || true

if [ -f "$3" ]; then
    cp "$3" /usr/share/polkit-1/rules.d/io.ryzora.package.rules
    chmod 644 /usr/share/polkit-1/rules.d/io.ryzora.package.rules
    chown root:root /usr/share/polkit-1/rules.d/io.ryzora.package.rules 2>/dev/null || true
fi
"#;

    let output = Command::new("pkexec")
        .args(["bash", "-c", script, "--", &helper_str, &policy_str, &rules_str])
        .output()
        .map_err(|e| format!("Failed to invoke pkexec for package service setup: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Administrator authorization was cancelled or failed: {}",
            err_msg.trim()
        ));
    }

    let status = detect_privileged_package_helper_status_in(None);
    if !status.installed {
        return Err(format!(
            "Helper setup executed but post-install verification failed: {}",
            status.error.unwrap_or_else(|| "Unknown validation error".to_string())
        ));
    }

    Ok(status)
}

#[tauri::command]
pub fn get_privileged_package_helper_status() -> PrivilegedPackageHelperStatus {
    detect_privileged_package_helper_status()
}

#[tauri::command]
pub fn setup_privileged_package_helper() -> Result<PrivilegedPackageHelperStatus, String> {
    let sys_root = std::env::var("RYZORA_SYSTEM_ROOT").ok().map(PathBuf::from);
    setup_privileged_package_helper_in(sys_root.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_package_helper_missing() {
        let temp = std::env::temp_dir().join("ryzora-pkg-test-missing");
        let _ = fs::remove_dir_all(&temp);
        let status = detect_privileged_package_helper_status_in(Some(&temp));
        assert!(!status.installed);
        assert!(!status.helper_exists);
    }

    #[test]
    fn test_setup_package_helper_in_test_root() {
        let temp = std::env::temp_dir().join("ryzora-pkg-test-setup");
        let _ = fs::remove_dir_all(&temp);
        let status = setup_privileged_package_helper_in(Some(&temp)).unwrap();
        assert!(status.installed);
        assert!(temp.join("usr/lib/ryzora/ryzora-package-helper").exists());
        assert!(temp.join("usr/share/polkit-1/actions/io.ryzora.package.policy").exists());
        let _ = fs::remove_dir_all(&temp);
    }
}
