//! Ryzora Pacman App Adapter — Phase 23A
//!
//! Provides zero-subprocess, read-only discovery, inspection, and installed-state
//! detection for Arch Linux official repository packages using ALPM sync databases
//! (`/var/lib/pacman/sync/*.db`) and local package records (`/var/lib/pacman/local/*/desc`).
//!
//! # Safety Invariants
//! - Zero `Command::new` or shell subprocesses during search, parsing, or metadata extraction.
//! - Direct memory inspection of ALPM tar.gz archives using standard Rust streams (`flate2` + `tar`).
//! - Strictly read-only operations for discovery and status checks.
//! - In test environments (`RYZORA_SYSTEM_ROOT` set), operations run isolated within the test sandbox.

use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

/// Normalized package information extracted from ALPM desc files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacmanPackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub url: Option<String>,
    pub license: Option<String>,
    pub size_bytes: Option<u64>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
    pub dependencies: Vec<String>,
}

/// Default sync database directory on Arch Linux
pub const DEFAULT_PACMAN_SYNC_DIR: &str = "/var/lib/pacman/sync";

/// Default local installed database directory on Arch Linux
pub const DEFAULT_PACMAN_LOCAL_DIR: &str = "/var/lib/pacman/local";

/// Resolves the effective sync directory, taking test sandboxes into account.
pub fn resolve_sync_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        return PathBuf::from(sys_root).join("var/lib/pacman/sync");
    }
    PathBuf::from(DEFAULT_PACMAN_SYNC_DIR)
}

/// Resolves the effective local installed directory, taking test sandboxes into account.
pub fn resolve_local_dir(custom: Option<&Path>) -> PathBuf {
    if let Some(p) = custom {
        return p.to_path_buf();
    }
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        return PathBuf::from(sys_root).join("var/lib/pacman/local");
    }
    PathBuf::from(DEFAULT_PACMAN_LOCAL_DIR)
}

/// Parses an ALPM `%FIELD%` formatted string (from a sync or local `desc` file) into a `PacmanPackageInfo`.
pub fn parse_alpm_desc(content: &str, repo_name: &str) -> Option<PacmanPackageInfo> {
    let mut name = String::new();
    let mut version = String::new();
    let mut desc = String::new();
    let mut url = None;
    let mut licenses = Vec::new();
    let mut size_bytes = None;
    let mut dependencies = Vec::new();

    let mut current_key = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') && trimmed.ends_with('%') {
            current_key = trimmed.trim_matches('%');
            continue;
        }

        if trimmed.is_empty() {
            continue;
        }

        match current_key {
            "NAME" if name.is_empty() => name = trimmed.to_string(),
            "VERSION" if version.is_empty() => version = trimmed.to_string(),
            "DESC" if desc.is_empty() => desc = trimmed.to_string(),
            "URL" if url.is_none() => url = Some(trimmed.to_string()),
            "LICENSE" => licenses.push(trimmed.to_string()),
            "CSIZE" | "SIZE" | "ISIZE" if size_bytes.is_none() => {
                if let Ok(sz) = trimmed.parse::<u64>() {
                    size_bytes = Some(sz);
                }
            }
            "DEPENDS" => dependencies.push(trimmed.to_string()),
            _ => {}
        }
    }

    if name.is_empty() || version.is_empty() {
        return None;
    }

    let license = if !licenses.is_empty() {
        Some(licenses.join(", "))
    } else {
        None
    };

    Some(PacmanPackageInfo {
        name,
        version,
        description: desc,
        repository: repo_name.to_string(),
        url,
        license,
        size_bytes,
        is_installed: false,
        installed_version: None,
        dependencies,
    })
}

/// Checks whether a package is installed in `local_dir`, returning `(is_installed, installed_version)`.
pub fn check_installed_status_in(pkg_name: &str, local_dir: &Path) -> (bool, Option<String>) {
    if !local_dir.exists() {
        return (false, None);
    }

    if let Ok(entries) = fs::read_dir(local_dir) {
        let prefix = format!("{}-", pkg_name);
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let dir_name = file_name.to_string_lossy();
            if dir_name.starts_with(&prefix) {
                // Ensure it's not a package with a longer prefix (e.g. `firefox-developer-edition` for `firefox`)
                let remainder = &dir_name[prefix.len()..];
                // Pacman version strings start with a digit
                if remainder.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    let desc_file = entry.path().join("desc");
                    if desc_file.is_file() {
                        if let Ok(content) = fs::read_to_string(&desc_file) {
                            if let Some(parsed) = parse_alpm_desc(&content, "local") {
                                if parsed.name == pkg_name {
                                    return (true, Some(parsed.version));
                                }
                            }
                        }
                    }
                    // Fallback to directory name slice if desc wasn't fully readable
                    return (true, Some(remainder.to_string()));
                }
            }
        }
    }

    (false, None)
}

/// Searches all ALPM `.db` archives in `sync_dir` for packages matching `query`.
/// Read-only and strictly zero-subprocess.
pub fn search_pacman_packages_in(
    query: &str,
    sync_dir: &Path,
    local_dir: &Path,
    limit: usize,
) -> Result<Vec<PacmanPackageInfo>, String> {
    if !sync_dir.exists() {
        return Ok(Vec::new());
    }

    let q = query.trim().to_lowercase();
    let mut results: Vec<PacmanPackageInfo> = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    let entries = fs::read_dir(sync_dir)
        .map_err(|e| format!("Failed to read pacman sync dir '{}': {}", sync_dir.display(), e))?;

    // Prioritize core, extra, multilib
    let mut db_files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "db") {
            db_files.push(path);
        }
    }

    // Sort to give predictable repo priority: core first, then extra, then community/multilib/garuda
    db_files.sort_by_key(|p| {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        match stem {
            "core" => 0,
            "extra" => 1,
            "multilib" => 2,
            "garuda" => 3,
            _ => 10,
        }
    });

    for db_path in db_files {
        let repo_name = db_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = match File::open(&db_path) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        let entries = match archive.entries() {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry_res in entries {
            let mut entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path_buf = match entry.path() {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            };

            let path_str = path_buf.to_string_lossy();
            if !path_str.ends_with("/desc") {
                continue;
            }

            // Quick check: if query is non-empty, check if folder name or desc contains it
            let folder_name = path_str.split('/').next().unwrap_or("");
            let folder_lower = folder_name.to_lowercase();

            let matches_folder = q.is_empty() || folder_lower.contains(&q);

            let mut content = String::new();
            if let Err(_) = entry.read_to_string(&mut content) {
                continue;
            }

            let matches_desc = !matches_folder && !q.is_empty() && content.to_lowercase().contains(&q);

            if matches_folder || matches_desc {
                if let Some(mut info) = parse_alpm_desc(&content, &repo_name) {
                    if !seen_names.contains(&info.name) {
                        seen_names.insert(info.name.clone());

                        let (is_installed, installed_ver) = check_installed_status_in(&info.name, local_dir);
                        info.is_installed = is_installed;
                        info.installed_version = installed_ver;

                        results.push(info);
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }

        if results.len() >= limit {
            break;
        }
    }

    // Rank results: exact name match first, starts_with second, others after
    results.sort_by(|a, b| {
        let a_exact = a.name.eq_ignore_ascii_case(&q);
        let b_exact = b.name.eq_ignore_ascii_case(&q);
        if a_exact && !b_exact {
            std::cmp::Ordering::Less
        } else if !a_exact && b_exact {
            std::cmp::Ordering::Greater
        } else {
            let a_starts = a.name.to_lowercase().starts_with(&q);
            let b_starts = b.name.to_lowercase().starts_with(&q);
            if a_starts && !b_starts {
                std::cmp::Ordering::Less
            } else if !a_starts && b_starts {
                std::cmp::Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        }
    });

    Ok(results)
}

/// Retrieves details for a specific pacman package by name.
pub fn get_pacman_package_details_in(
    pkg_name: &str,
    sync_dir: &Path,
    local_dir: &Path,
) -> Result<Option<PacmanPackageInfo>, String> {
    let matches = search_pacman_packages_in(pkg_name, sync_dir, local_dir, 10)?;
    for item in matches {
        if item.name.eq_ignore_ascii_case(pkg_name) {
            return Ok(Some(item));
        }
    }

    // If not in sync_dir, check if installed locally
    if local_dir.exists() {
        let (is_installed, ver) = check_installed_status_in(pkg_name, local_dir);
        if is_installed {
            if let Ok(entries) = fs::read_dir(local_dir) {
                let prefix = format!("{}-", pkg_name);
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(&prefix) {
                        let desc_file = entry.path().join("desc");
                        if let Ok(content) = fs::read_to_string(&desc_file) {
                            if let Some(mut info) = parse_alpm_desc(&content, "installed") {
                                info.is_installed = true;
                                info.installed_version = ver;
                                return Ok(Some(info));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Lists all installed packages found in `local_dir`.
pub fn list_installed_pacman_packages_in(local_dir: &Path) -> Result<Vec<PacmanPackageInfo>, String> {
    if !local_dir.exists() {
        return Ok(Vec::new());
    }

    let mut installed = Vec::new();
    let entries = fs::read_dir(local_dir)
        .map_err(|e| format!("Failed to read pacman local dir '{}': {}", local_dir.display(), e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let desc_file = path.join("desc");
            if desc_file.is_file() {
                if let Ok(content) = fs::read_to_string(&desc_file) {
                    if let Some(mut info) = parse_alpm_desc(&content, "installed") {
                        info.is_installed = true;
                        info.installed_version = Some(info.version.clone());
                        installed.push(info);
                    }
                }
            }
        }
    }

    installed.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(installed)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_test_desc(name: &str, version: &str, desc: &str, url: &str, license: &str) -> String {
        format!(
            "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\n{}\n\n%URL%\n{}\n\n%LICENSE%\n{}\n\n%SIZE%\n123456\n\n%DEPENDS%\nglibc\ngtk3\n",
            name, version, desc, url, license
        )
    }

    fn create_mock_sync_db(db_file: &Path, packages: &[(&str, &str, &str, &str, &str)]) {
        let file = File::create(db_file).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(gz);

        for (name, ver, desc, url, lic) in packages {
            let desc_content = make_test_desc(name, ver, desc, url, lic);
            let bytes = desc_content.as_bytes();

            let mut header = tar::Header::new_gnu();
            header.set_path(format!("{}-{}/desc", name, ver)).unwrap();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();

            tar.append(&header, bytes).unwrap();
        }

        tar.finish().unwrap();
    }

    #[test]
    fn test_parse_alpm_desc_extracts_fields_correctly() {
        let desc = make_test_desc(
            "firefox",
            "130.0-1",
            "Fast, private web browser",
            "https://mozilla.org/firefox",
            "MPL-2.0",
        );

        let parsed = parse_alpm_desc(&desc, "extra").expect("Must parse valid desc");
        assert_eq!(parsed.name, "firefox");
        assert_eq!(parsed.version, "130.0-1");
        assert_eq!(parsed.description, "Fast, private web browser");
        assert_eq!(parsed.url, Some("https://mozilla.org/firefox".to_string()));
        assert_eq!(parsed.license, Some("MPL-2.0".to_string()));
        assert_eq!(parsed.repository, "extra");
        assert_eq!(parsed.size_bytes, Some(123456));
        assert_eq!(parsed.dependencies, vec!["glibc", "gtk3"]);
        assert!(!parsed.is_installed);
    }

    #[test]
    fn test_search_pacman_packages_matches_and_ranks() {
        let tmp = std::env::temp_dir().join(format!("pacman_test_search_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let sync_dir = tmp.join("sync");
        let local_dir = tmp.join("local");
        fs::create_dir_all(&sync_dir).unwrap();
        fs::create_dir_all(&local_dir).unwrap();

        let db_file = sync_dir.join("extra.db");
        create_mock_sync_db(
            &db_file,
            &[
                ("firefox", "130.0-1", "Fast web browser", "https://mozilla.org", "MPL-2.0"),
                ("firefox-developer-edition", "131.0b1-1", "Developer edition browser", "https://mozilla.org", "MPL-2.0"),
                ("alacritty", "0.13.0-1", "GPU terminal emulator", "https://alacritty.org", "Apache-2.0"),
            ],
        );

        // Search for 'firefox'
        let results = search_pacman_packages_in("firefox", &sync_dir, &local_dir, 10).unwrap();
        assert_eq!(results.len(), 2);
        // Exact match 'firefox' must rank first
        assert_eq!(results[0].name, "firefox");
        assert_eq!(results[1].name, "firefox-developer-edition");

        // Search for 'terminal' matching description
        let term_results = search_pacman_packages_in("terminal", &sync_dir, &local_dir, 10).unwrap();
        assert_eq!(term_results.len(), 1);
        assert_eq!(term_results[0].name, "alacritty");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_installed_package_detection() {
        let tmp = std::env::temp_dir().join(format!("pacman_test_inst_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let sync_dir = tmp.join("sync");
        let local_dir = tmp.join("local");
        fs::create_dir_all(&sync_dir).unwrap();

        // Create installed package in local_dir
        let inst_pkg = local_dir.join("firefox-130.0-1");
        fs::create_dir_all(&inst_pkg).unwrap();
        let desc = make_test_desc("firefox", "130.0-1", "Web browser", "https://mozilla.org", "MPL-2.0");
        fs::write(inst_pkg.join("desc"), desc).unwrap();

        let (installed, ver) = check_installed_status_in("firefox", &local_dir);
        assert!(installed);
        assert_eq!(ver, Some("130.0-1".to_string()));

        let (not_inst, _) = check_installed_status_in("alacritty", &local_dir);
        assert!(!not_inst);

        let list = list_installed_pacman_packages_in(&local_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "firefox");
        assert!(list[0].is_installed);

        let _ = fs::remove_dir_all(&tmp);
    }
}
