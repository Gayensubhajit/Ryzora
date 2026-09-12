//! Ryzora Safe Application Cleanup & Storage Management Engine — Phase 26
//!
//! Provides authoritative, provider-aware inspection and cleanup:
//! 1. Normal uninstall: Removes only the application package (ALPM / Pacman -R).
//! 2. Unused dependencies: Discovers and removes only packages confirmed unneeded by Pacman (-Rs).
//! 3. Package cache: Discovers exact archives under /var/cache/pacman/pkg/$PKG-[0-9]* and cleans via helper.
//! 4. Application configuration & cache: Reliably verified user directories (~/.config, ~/.cache).
//! 5. AUR build directory & helper cache: Unprivileged user cache isolation (~/.cache/ryzora/aur-build, ~/.cache/yay).
//! 6. Flatpak application data & runtime sharing: Distinguishes shared runtimes from isolated app data.
//!
//! Security Invariants:
//! - No unrestricted root operations (no `sudo rm -rf`).
//! - No filename guessing in ~/Documents, ~/Projects, etc. Personal files are never touched.
//! - Authoritative real byte sizes only (no arbitrary multipliers).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnusedDependencyItem {
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageCacheItem {
    pub filename: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCleanupPreview {
    pub package_id: String,
    pub provider_id: String,
    pub display_name: String,
    pub is_installed: bool,
    pub installed_package_bytes: Option<u64>,
    pub unused_dependencies: Vec<UnusedDependencyItem>,
    pub total_unused_dependencies_bytes: u64,
    pub package_cache_items: Vec<PackageCacheItem>,
    pub total_package_cache_bytes: u64,
    pub app_config_bytes: Option<u64>,
    pub app_config_path: Option<String>,
    pub app_cache_bytes: Option<u64>,
    pub app_cache_path: Option<String>,
    pub aur_build_dir_bytes: Option<u64>,
    pub aur_build_dir_path: Option<String>,
    pub aur_cache_bytes: Option<u64>,
    pub aur_cache_path: Option<String>,
    pub flatpak_data_bytes: Option<u64>,
    pub flatpak_data_path: Option<String>,
    pub flatpak_runtime_info: Option<String>,
    pub conflicts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCleanupExecutionRequest {
    pub package_id: String,
    pub provider_id: String,
    pub remove_package: bool,
    pub remove_unused_dependencies: bool,
    pub clean_package_cache: bool,
    pub clean_app_cache: bool,
    pub clean_app_config: bool,
    pub clean_aur_build: bool,
    pub clean_flatpak_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppCleanupExecutionResult {
    pub success: bool,
    pub message: String,
    pub package_removed: bool,
    pub dependencies_removed: Vec<String>,
    pub cache_files_removed: usize,
    pub total_bytes_reclaimed: u64,
    pub errors: Vec<String>,
}

/// Recursively calculates the exact byte size and file count of a directory.
/// Follows no symlinks outside the target.
pub fn calculate_dir_size(dir: &Path) -> (u64, usize) {
    if !dir.exists() || !dir.is_dir() {
        return (0, 0);
    }

    let mut total_bytes = 0u64;
    let mut total_files = 0usize;

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_symlink() {
                    // Do not follow symlinks outside target
                    continue;
                }
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file() {
                    if let Ok(meta) = fs::symlink_metadata(&p) {
                        total_bytes += meta.len();
                        total_files += 1;
                    }
                }
            }
        }
    }

    (total_bytes, total_files)
}

/// Strict safety check for user cleanup directories.
/// Ensures directory is strictly a direct child or verified subfolder under
/// $HOME/.config or $HOME/.cache, and never root, $HOME, or personal folders.
pub fn is_safe_user_cleanup_path(path: &Path, expected_base: &Path) -> bool {
    // Must be absolute, exist, and not be a symlink
    if !path.is_absolute() || !path.exists() || path.is_symlink() {
        return false;
    }

    // Must strictly start with expected base (~/.config or ~/.cache)
    if !path.starts_with(expected_base) {
        return false;
    }

    // Cannot be equal to the base directory itself (~/.config itself must never be removed!)
    if path == expected_base {
        return false;
    }

    // Cannot contain '..' traversal
    for comp in path.components() {
        if let std::path::Component::ParentDir = comp {
            return false;
        }
    }

    // Must be under the user's home directory
    let home = crate::snapshot::get_home_dir();
    path.starts_with(&home)
}

/// Curated registry of verified application configuration and cache subdirectory names.
/// Prevents false assumptions like ~/.config/{package-name}.
pub fn get_known_app_config_and_cache_subdirs(pkg_name: &str) -> (Option<&'static str>, Option<&'static str>) {
    let lower = pkg_name.to_lowercase();
    match lower.as_str() {
        "blender" => (Some("blender"), Some("blender")),
        "cursor" | "cursor-bin" | "cursor-appimage" => (Some("cursor"), Some("cursor")),
        "visual-studio-code" | "visual-studio-code-bin" | "code" | "code-oss" => (Some("Code"), Some("Code")),
        "chromium" | "chromium-browser" => (Some("chromium"), Some("chromium")),
        "google-chrome" | "google-chrome-stable" => (Some("google-chrome"), Some("google-chrome")),
        "alacritty" => (Some("alacritty"), None),
        "kitty" => (Some("kitty"), None),
        "btop" => (Some("btop"), None),
        "gimp" | "gimp2" => (Some("GIMP"), None),
        "inkscape" => (Some("inkscape"), Some("inkscape")),
        "obs-studio" | "obs" => (Some("obs-studio"), None),
        "spotify" | "spotify-launcher" => (Some("spotify"), Some("spotify")),
        "telegram-desktop" | "telegram" => (Some("TelegramDesktop"), None),
        "discord" | "discord-ptb" | "discord-canary" => (Some("discord"), Some("discord")),
        "neovim" | "nvim" => (Some("nvim"), None),
        "bottles" => (Some("bottles"), Some("bottles")),
        "vlc" => (Some("vlc"), Some("vlc")),
        _ => (None, None),
    }
}

/// Discovers exact package cache archives in /var/cache/pacman/pkg matching $PKG-[0-9]*
pub fn find_package_cache_archives(pkg_name: &str, cache_dir: &Path) -> Vec<PackageCacheItem> {
    let mut items = Vec::new();
    if !cache_dir.exists() || !cache_dir.is_dir() {
        return items;
    }

    let prefix = format!("{}-", pkg_name.to_lowercase());

    if let Ok(entries) = fs::read_dir(cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                let name_lower = name.to_lowercase();
                if let Some(rest) = name_lower.strip_prefix(&prefix) {
                    // Must start with digit (package version/epoch), e.g., '1.0' or '17:5.2'
                    if rest.starts_with(|c: char| c.is_ascii_digit())
                        && (rest.contains(".pkg.tar.") || rest.ends_with(".sig"))
                    {
                        if let Ok(meta) = fs::metadata(&path) {
                            items.push(PackageCacheItem {
                                filename: name.to_string(),
                                size_bytes: meta.len(),
                            });
                        }
                    }
                }
            }
        }
    }

    items.sort_by(|a, b| a.filename.cmp(&b.filename));
    items
}

/// Discovers unused dependencies that would be removed by pacman -Rs.
/// Relies authoritatively on pacman's dependency solver.
pub fn discover_pacman_unused_dependencies(pkg_name: &str) -> (Vec<UnusedDependencyItem>, Vec<String>) {
    let mut items = Vec::new();
    let mut conflicts = Vec::new();

    // Check if dry run pacman -Rsp is supported
    let output = match Command::new("pacman")
        .args(["-Rsp", "--print-format", "%n|%s", pkg_name])
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return (items, conflicts),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            let trimmed = line.trim();
            if trimmed.contains("could not satisfy dependencies")
                || trimmed.contains("failed to prepare transaction")
                || trimmed.contains("breaking dependency")
            {
                conflicts.push(trimmed.to_string());
            }
        }
        return (items, conflicts);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pkg_lower = pkg_name.to_lowercase();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('|').collect();
        if parts.len() >= 2 {
            let name = parts[0].trim().to_string();
            // Skip the target package itself
            if name.to_lowercase() == pkg_lower {
                continue;
            }
            let size = parts[1].trim().parse::<u64>().unwrap_or(0);
            items.push(UnusedDependencyItem {
                name,
                size_bytes: size,
            });
        }
    }

    (items, conflicts)
}

/// Inspects potential cleanup for an application without performing any removal.
pub fn inspect_app_cleanup(package_id: &str, provider: &str) -> Result<AppCleanupPreview, String> {
    let trimmed = package_id.trim();
    if trimmed.is_empty() {
        return Err("Package ID cannot be empty".to_string());
    }

    let provider_norm = provider.to_lowercase();
    let home = crate::snapshot::get_home_dir();
    let cfg_base = home.join(".config");
    let cache_base = home.join(".cache");

    let mut installed_package_bytes = None;
    let mut unused_dependencies = Vec::new();
    let mut conflicts = Vec::new();
    let mut package_cache_items = Vec::new();
    let mut is_installed = false;
    let mut display_name = trimmed.to_string();

    // 1. Pacman / ALPM Inspection
    if provider_norm == "pacman" || provider_norm == "aur" {
        let local_dir = crate::app_adapters::pacman::resolve_local_dir(None);
        let (installed, ver) = crate::app_adapters::pacman::check_installed_status_in(trimmed, &local_dir);
        is_installed = installed;

        if installed {
            if let Some(v) = ver {
                let pkg_dir = local_dir.join(format!("{}-{}", trimmed, v));
                let desc_file = pkg_dir.join("desc");
                if desc_file.exists() {
                    if let Ok(content) = fs::read_to_string(desc_file) {
                        if let Some(parsed) = crate::app_adapters::pacman::parse_alpm_desc(&content, "local") {
                            installed_package_bytes = parsed.size_bytes;
                        }
                    }
                }
            }

            // Query authoritative unused dependencies
            let (deps, errs) = discover_pacman_unused_dependencies(trimmed);
            unused_dependencies = deps;
            conflicts = errs;
        }

        // Check /var/cache/pacman/pkg
        let pacman_cache_dir = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
            PathBuf::from(sys_root).join("var/cache/pacman/pkg")
        } else {
            PathBuf::from("/var/cache/pacman/pkg")
        };
        package_cache_items = find_package_cache_archives(trimmed, &pacman_cache_dir);
    }

    // 2. User Configuration & Cache Inspection (verified paths only)
    let (known_cfg, known_cache) = get_known_app_config_and_cache_subdirs(trimmed);

    let (app_config_bytes, app_config_path) = if let Some(sub) = known_cfg {
        let target = cfg_base.join(sub);
        if is_safe_user_cleanup_path(&target, &cfg_base) {
            let (sz, _) = calculate_dir_size(&target);
            (Some(sz), Some(target.display().to_string()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let (app_cache_bytes, app_cache_path) = if let Some(sub) = known_cache {
        let target = cache_base.join(sub);
        if is_safe_user_cleanup_path(&target, &cache_base) {
            let (sz, _) = calculate_dir_size(&target);
            (Some(sz), Some(target.display().to_string()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // 3. AUR Build & Helper Cache
    let (aur_build_dir_bytes, aur_build_dir_path, aur_cache_bytes, aur_cache_path) = if provider_norm == "aur" {
        let build_dir = cache_base.join("ryzora/aur-build").join(trimmed);
        let yay_cache = cache_base.join("yay").join(trimmed);
        let paru_cache = cache_base.join("paru/clone").join(trimmed);

        let build_info = if build_dir.exists() {
            let (sz, _) = calculate_dir_size(&build_dir);
            (Some(sz), Some(build_dir.display().to_string()))
        } else {
            (None, None)
        };

        let helper_info = if yay_cache.exists() {
            let (sz, _) = calculate_dir_size(&yay_cache);
            (Some(sz), Some(yay_cache.display().to_string()))
        } else if paru_cache.exists() {
            let (sz, _) = calculate_dir_size(&paru_cache);
            (Some(sz), Some(paru_cache.display().to_string()))
        } else {
            (None, None)
        };

        (build_info.0, build_info.1, helper_info.0, helper_info.1)
    } else {
        (None, None, None, None)
    };

    // 4. Flatpak Inspection
    let (flatpak_data_bytes, flatpak_data_path, flatpak_runtime_info) = if provider_norm == "flatpak" {
        let var_app = home.join(".var/app").join(trimmed);
        let data_info = if var_app.exists() {
            let (sz, _) = calculate_dir_size(&var_app);
            (Some(sz), Some(var_app.display().to_string()))
        } else {
            (None, None)
        };

        // Check if Flatpak app is installed
        let (has_flatpak, _) = crate::system::check_binary("flatpak");
        if has_flatpak {
            if let Ok(info) = crate::app_adapters::flatpak::get_flatpak_app_info(trimmed) {
                if let Some(app_meta) = info {
                    is_installed = app_meta.is_installed;
                    display_name = app_meta.name;
                    if let Some(rt) = app_meta.runtime {
                        // Check if runtime is shared
                        let rt_output = Command::new("flatpak")
                            .args(["list", &format!("--app-runtime={}", rt)])
                            .output();
                        if let Ok(o) = rt_output {
                            let count = String::from_utf8_lossy(&o.stdout).lines().count();
                            if count > 1 {
                                (data_info.0, data_info.1, Some(format!("Shared with {} other application(s) (Kept)", count - 1)))
                            } else {
                                (data_info.0, data_info.1, Some("Dedicated runtime (Removable if unused)".to_string()))
                            }
                        } else {
                            (data_info.0, data_info.1, None)
                        }
                    } else {
                        (data_info.0, data_info.1, None)
                    }
                } else {
                    (data_info.0, data_info.1, None)
                }
            } else {
                (data_info.0, data_info.1, None)
            }
        } else {
            (data_info.0, data_info.1, None)
        }
    } else {
        (None, None, None)
    };

    let total_unused_dependencies_bytes = unused_dependencies.iter().map(|d| d.size_bytes).sum();
    let total_package_cache_bytes = package_cache_items.iter().map(|c| c.size_bytes).sum();

    Ok(AppCleanupPreview {
        package_id: trimmed.to_string(),
        provider_id: provider_norm,
        display_name,
        is_installed,
        installed_package_bytes,
        unused_dependencies,
        total_unused_dependencies_bytes,
        package_cache_items,
        total_package_cache_bytes,
        app_config_bytes,
        app_config_path,
        app_cache_bytes,
        app_cache_path,
        aur_build_dir_bytes,
        aur_build_dir_path,
        aur_cache_bytes,
        aur_cache_path,
        flatpak_data_bytes,
        flatpak_data_path,
        flatpak_runtime_info,
        conflicts,
    })
}

/// Executes the verified cleanup operations based on explicit user selections.
pub fn execute_app_cleanup(request: AppCleanupExecutionRequest) -> Result<AppCleanupExecutionResult, String> {
    let trimmed = request.package_id.trim();
    if trimmed.is_empty() {
        return Err("Package ID cannot be empty".to_string());
    }

    let provider = request.provider_id.to_lowercase();
    let mut total_reclaimed = 0u64;
    let deps_removed = Vec::new();
    let mut cache_removed_count = 0usize;
    let mut errors = Vec::new();
    let mut package_removed = false;

    // 1. Package Uninstallation (Pacman, AUR, Flatpak)
    if request.remove_package {
        if provider == "pacman" || provider == "aur" {
            // Determine transaction operation: 'uninstall' or 'uninstall-deps'
            let op = if request.remove_unused_dependencies {
                "uninstall-deps"
            } else {
                "uninstall"
            };

            // Test mode check
            if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
                let local_dir = crate::app_adapters::pacman::resolve_local_dir(None);
                let (installed, ver) = crate::app_adapters::pacman::check_installed_status_in(trimmed, &local_dir);
                if installed {
                    if let Some(v) = ver {
                        let pkg_dir = local_dir.join(format!("{}-{}", trimmed, v));
                        let _ = fs::remove_dir_all(&pkg_dir);
                    }
                    package_removed = true;
                }
            } else {
                let helper_path = PathBuf::from(crate::package_helper::PACKAGE_HELPER_SYSTEM_PATH);
                let output = if helper_path.exists() {
                    Command::new("pkexec")
                        .arg(crate::package_helper::PACKAGE_HELPER_SYSTEM_PATH)
                        .arg(op)
                        .arg(trimmed)
                        .stdin(std::process::Stdio::null())
                        .output()
                } else {
                    let mut cmd = Command::new("pkexec");
                    cmd.arg("pacman");
                    if request.remove_unused_dependencies {
                        cmd.arg("-Rs");
                    } else {
                        cmd.arg("-R");
                    }
                    cmd.arg("--noconfirm").arg(trimmed);
                    cmd.stdin(std::process::Stdio::null());
                    cmd.output()
                };

                match output {
                    Ok(o) if o.status.success() => {
                        package_removed = true;
                    }
                    Ok(o) => {
                        let err = String::from_utf8_lossy(&o.stderr);
                        errors.push(format!("Package removal failed: {}", err.trim()));
                    }
                    Err(e) => {
                        errors.push(format!("Failed to execute uninstallation: {}", e));
                    }
                }
            }
        } else if provider == "flatpak" {
            let mut args = vec!["uninstall", "-y"];
            if request.clean_flatpak_data {
                args.push("--delete-data");
            }
            args.push(trimmed);

            match Command::new("flatpak").args(&args).output() {
                Ok(o) if o.status.success() => {
                    package_removed = true;
                    crate::app_adapters::flatpak::invalidate_flatpak_cache();
                }
                Ok(o) => {
                    let err = String::from_utf8_lossy(&o.stderr);
                    errors.push(format!("Flatpak uninstall failed: {}", err.trim()));
                }
                Err(e) => {
                    errors.push(format!("Failed to invoke flatpak uninstall: {}", e));
                }
            }
        }
    }

    // 2. Package Cache Cleanup (via privileged helper 'clean-cache')
    if request.clean_package_cache && (provider == "pacman" || provider == "aur") {
        if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
            // Mock mode
            cache_removed_count += 1;
        } else {
            let helper_path = PathBuf::from(crate::package_helper::PACKAGE_HELPER_SYSTEM_PATH);
            let res = if helper_path.exists() {
                Command::new("pkexec")
                    .arg(crate::package_helper::PACKAGE_HELPER_SYSTEM_PATH)
                    .arg("clean-cache")
                    .arg(trimmed)
                    .stdin(std::process::Stdio::null())
                    .output()
            } else {
                // Fallback direct cleanup of /var/cache/pacman/pkg/$PKG-[0-9]*
                let script = format!(
                    r#"for f in /var/cache/pacman/pkg/{}-[0-9]*.pkg.tar.*; do [ -f "$f" ] && rm -f -- "$f"; done"#,
                    trimmed
                );
                Command::new("pkexec")
                    .args(["bash", "-c", &script])
                    .stdin(std::process::Stdio::null())
                    .output()
            };

            match res {
                Ok(o) if o.status.success() => {
                    cache_removed_count += 1;
                }
                Ok(o) => {
                    let err = String::from_utf8_lossy(&o.stderr);
                    errors.push(format!("Package cache cleanup failed: {}", err.trim()));
                }
                Err(e) => {
                    errors.push(format!("Failed to clean package cache: {}", e));
                }
            }
        }
    }

    // 3. Application Cache Cleanup (~/.cache/..., unprivileged user space)
    if request.clean_app_cache {
        let home = crate::snapshot::get_home_dir();
        let cache_base = home.join(".cache");
        let (_, known_cache) = get_known_app_config_and_cache_subdirs(trimmed);
        if let Some(sub) = known_cache {
            let target = cache_base.join(sub);
            if is_safe_user_cleanup_path(&target, &cache_base) {
                let (sz, _) = calculate_dir_size(&target);
                if fs::remove_dir_all(&target).is_ok() {
                    total_reclaimed += sz;
                }
            }
        }
    }

    // 4. Application Configuration Cleanup (~/.config/..., unprivileged user space)
    if request.clean_app_config {
        let home = crate::snapshot::get_home_dir();
        let cfg_base = home.join(".config");
        let (known_cfg, _) = get_known_app_config_and_cache_subdirs(trimmed);
        if let Some(sub) = known_cfg {
            let target = cfg_base.join(sub);
            if is_safe_user_cleanup_path(&target, &cfg_base) {
                let (sz, _) = calculate_dir_size(&target);
                if fs::remove_dir_all(&target).is_ok() {
                    total_reclaimed += sz;
                }
            }
        }
    }

    // 5. AUR Build Directory Cleanup (~/.cache/ryzora/aur-build/..., unprivileged)
    if request.clean_aur_build && provider == "aur" {
        let home = crate::snapshot::get_home_dir();
        let build_dir = home.join(".cache/ryzora/aur-build").join(trimmed);
        if build_dir.exists() && build_dir.is_dir() {
            let (sz, _) = calculate_dir_size(&build_dir);
            if fs::remove_dir_all(&build_dir).is_ok() {
                total_reclaimed += sz;
            }
        }

        let yay_cache = home.join(".cache/yay").join(trimmed);
        if yay_cache.exists() && yay_cache.is_dir() {
            let (sz, _) = calculate_dir_size(&yay_cache);
            if fs::remove_dir_all(&yay_cache).is_ok() {
                total_reclaimed += sz;
            }
        }
    }

    let success = errors.is_empty() && (package_removed || !request.remove_package);
    let message = if success {
        format!("Successfully completed cleanup for '{}'", trimmed)
    } else {
        format!("Cleanup encountered issues: {}", errors.join("; "))
    };

    Ok(AppCleanupExecutionResult {
        success,
        message,
        package_removed,
        dependencies_removed: deps_removed,
        cache_files_removed: cache_removed_count,
        total_bytes_reclaimed: total_reclaimed,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_user_cleanup_path() {
        let home = crate::snapshot::get_home_dir();
        let cfg_base = home.join(".config");
        let _test_cfg = cfg_base.join("blender");

        // The base directory itself must NEVER be considered safe to remove
        assert!(!is_safe_user_cleanup_path(&cfg_base, &cfg_base));

        // Random path outside base is rejected
        assert!(!is_safe_user_cleanup_path(&home.join("Documents"), &cfg_base));
        assert!(!is_safe_user_cleanup_path(Path::new("/etc"), &cfg_base));
        assert!(!is_safe_user_cleanup_path(Path::new("/var"), &cfg_base));
    }

    #[test]
    fn test_get_known_app_config_and_cache_subdirs() {
        assert_eq!(get_known_app_config_and_cache_subdirs("blender"), (Some("blender"), Some("blender")));
        assert_eq!(get_known_app_config_and_cache_subdirs("cursor-bin"), (Some("cursor"), Some("cursor")));
        assert_eq!(get_known_app_config_and_cache_subdirs("alacritty"), (Some("alacritty"), None));
        assert_eq!(get_known_app_config_and_cache_subdirs("unknown-utility"), (None, None));
    }

    #[test]
    fn test_inspect_app_cleanup_real_machine() {
        let res = inspect_app_cleanup("kate", "pacman");
        assert!(res.is_ok(), "inspect_app_cleanup should succeed: {:?}", res.err());
        let preview = res.unwrap();
        assert_eq!(preview.package_id, "kate");
        assert_eq!(preview.provider_id, "pacman");
    }

    #[test]
    fn test_find_package_cache_archives_matching() {
        let temp_dir = std::env::temp_dir().join("ryzora-cache-test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let f1 = temp_dir.join("blender-17:5.2.1-2-x86_64.pkg.tar.zst");
        let f2 = temp_dir.join("blender-17:5.2.1-2-x86_64.pkg.tar.zst.sig");
        let f3 = temp_dir.join("blender-plugin-1.0-1-any.pkg.tar.zst"); // Non-matching subpackage

        fs::write(&f1, b"archive data 123456").unwrap();
        fs::write(&f2, b"signature").unwrap();
        fs::write(&f3, b"plugin data").unwrap();

        let items = find_package_cache_archives("blender", &temp_dir);
        assert_eq!(items.len(), 2, "Must match exactly the package archives, not plugins: {:?}", items);
        assert!(items.iter().any(|i| i.filename.contains("17:5.2.1")));
        assert!(!items.iter().any(|i| i.filename.contains("blender-plugin")));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
