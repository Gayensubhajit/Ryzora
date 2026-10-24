//! Ryzora Privileged Package Helper Integration — Phase 23E
//!
//! Shared implementation provided by `ryzora_core::package_helper`.

use std::path::PathBuf;
pub use ryzora_core::package_helper::{
    detect_privileged_package_helper_status,
    detect_privileged_package_helper_status_in,
    find_bundled_package_helper_resources,
    setup_privileged_package_helper_in,
    PrivilegedPackageHelperStatus,
    PACKAGE_HELPER_SYSTEM_PATH,
    PACKAGE_POLICY_SYSTEM_PATH,
    PACKAGE_RULES_SYSTEM_PATH,
};

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
    use std::fs;

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
