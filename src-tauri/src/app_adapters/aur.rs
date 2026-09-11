//! Ryzora AUR (Arch User Repository) Adapter — Phase 25
//!
//! Provides native AUR catalog discovery, metadata querying, and unprivileged build management:
//! - Direct, fast querying of the official Arch Linux AUR RPC API (v5) via ureq.
//! - Authoritative detection of locally installed foreign (AUR) packages from /var/lib/pacman/local/.
//! - Direct retrieval of raw PKGBUILD for user inspection prior to build/installation.
//! - Dedicated unprivileged AUR build workflow (never executed as root).
//! - Cleanup awareness: exposes build and cache directories for Phase 26.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use crate::app_adapters::pacman::{resolve_local_dir, resolve_sync_dir, list_installed_pacman_packages_in};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AurPackageInfo {
    pub name: String,
    pub package_base: Option<String>,
    pub version: String,
    pub description: String,
    pub url: Option<String>,
    pub maintainer: Option<String>,
    pub popularity: Option<f64>,
    pub votes: Option<i64>,
    pub out_of_date: Option<i64>,
    pub license: Option<Vec<String>>,
    pub depends: Vec<String>,
    pub make_depends: Vec<String>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AurStatus {
    pub has_aur_support: bool,
    pub helper: Option<String>,
    pub total_installed_foreign: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AurCleanupInfo {
    pub package_name: String,
    pub build_dir: Option<String>,
    pub yay_cache: Option<String>,
    pub paru_cache: Option<String>,
}

#[derive(Deserialize)]
struct AurRpcResponse {
    #[allow(dead_code)]
    resultcount: usize,
    results: Vec<AurRpcRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AurRpcRecord {
    name: String,
    package_base: Option<String>,
    version: String,
    description: Option<String>,
    #[serde(rename = "URL")]
    url: Option<String>,
    maintainer: Option<String>,
    popularity: Option<f64>,
    num_votes: Option<i64>,
    out_of_date: Option<i64>,
    license: Option<serde_json::Value>,
    depends: Option<Vec<String>>,
    make_depends: Option<Vec<String>>,
}

/// Detects AUR support and counts installed foreign packages.
pub fn detect_aur_status() -> AurStatus {
    let (has_yay, _) = crate::system::check_binary("yay");
    let (has_paru, _) = crate::system::check_binary("paru");
    let (has_makepkg, _) = crate::system::check_binary("makepkg");

    let helper = if has_paru {
        Some("paru".to_string())
    } else if has_yay {
        Some("yay".to_string())
    } else if has_makepkg {
        Some("makepkg".to_string())
    } else {
        None
    };

    let foreign_pkgs = list_installed_foreign_packages().unwrap_or_default();

    AurStatus {
        has_aur_support: helper.is_some(),
        helper,
        total_installed_foreign: foreign_pkgs.len(),
    }
}

/// Helper to get local foreign installed packages map: name -> version
fn get_foreign_installed_map() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Ok(pkgs) = list_installed_foreign_packages() {
        for p in pkgs {
            map.insert(p.name.to_lowercase(), p.version);
        }
    }
    map
}

/// Searches the Arch User Repository using official AUR RPC v5.
pub fn aur_search_rpc(query: &str) -> Result<Vec<AurPackageInfo>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let encoded = percent_encoding::utf8_percent_encode(trimmed, percent_encoding::NON_ALPHANUMERIC).to_string();
    let url = format!("https://aur.archlinux.org/rpc/v5/search/{}", encoded);

    let resp = ureq::get(&url)
        .set("User-Agent", "Ryzora/0.1.0")
        .timeout(std::time::Duration::from_secs(6))
        .call()
        .map_err(|e| format!("AUR search request failed: {}", e))?;

    let body = resp.into_string()
        .map_err(|e| format!("Failed to read AUR response body: {}", e))?;
    let rpc_res: AurRpcResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse AUR RPC response: {}", e))?;

    let installed_map = get_foreign_installed_map();

    let mut list = Vec::new();
    for r in rpc_res.results {
        let name_lower = r.name.to_lowercase();
        let is_installed = installed_map.contains_key(&name_lower);
        let installed_version = installed_map.get(&name_lower).cloned();

        let license = match r.license {
            Some(serde_json::Value::Array(arr)) => {
                Some(arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            }
            Some(serde_json::Value::String(s)) => Some(vec![s]),
            _ => None,
        };

        list.push(AurPackageInfo {
            name: r.name,
            package_base: r.package_base,
            version: r.version,
            description: r.description.unwrap_or_default(),
            url: r.url,
            maintainer: r.maintainer,
            popularity: r.popularity,
            votes: r.num_votes,
            out_of_date: r.out_of_date,
            license,
            depends: r.depends.unwrap_or_default(),
            make_depends: r.make_depends.unwrap_or_default(),
            is_installed,
            installed_version,
        });
    }

let query_lower = trimmed.to_lowercase();
    list.sort_by(|a, b| {
        let a_name = a.name.to_lowercase();
        let b_name = b.name.to_lowercase();
        let a_exact = a_name == query_lower;
        let b_exact = b_name == query_lower;
        if a_exact != b_exact {
            return b_exact.cmp(&a_exact);
        }
        let a_starts = a_name.starts_with(&query_lower);
        let b_starts = b_name.starts_with(&query_lower);
        if a_starts != b_starts {
            return b_starts.cmp(&a_starts);
        }
        b.popularity.partial_cmp(&a.popularity).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.votes.cmp(&a.votes))
    });

    Ok(list)
}

/// Retrieves AUR package info by name using official AUR RPC v5.
pub fn aur_get_info_rpc(name: &str) -> Result<Option<AurPackageInfo>, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let encoded = percent_encoding::utf8_percent_encode(trimmed, percent_encoding::NON_ALPHANUMERIC).to_string();
    let url = format!("https://aur.archlinux.org/rpc/v5/info/{}", encoded);

    let resp = ureq::get(&url)
        .set("User-Agent", "Ryzora/0.1.0")
        .timeout(std::time::Duration::from_secs(6))
        .call()
        .map_err(|e| format!("AUR info request failed: {}", e))?;

    let body = resp.into_string()
        .map_err(|e| format!("Failed to read AUR response body: {}", e))?;
    let rpc_res: AurRpcResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse AUR RPC response: {}", e))?;

    let r = match rpc_res.results.into_iter().next() {
        Some(r) => r,
        None => return Ok(None),
    };

    let installed_map = get_foreign_installed_map();
    let name_lower = r.name.to_lowercase();
    let is_installed = installed_map.contains_key(&name_lower);
    let installed_version = installed_map.get(&name_lower).cloned();

    let license = match r.license {
        Some(serde_json::Value::Array(arr)) => {
            Some(arr.into_iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        }
        Some(serde_json::Value::String(s)) => Some(vec![s]),
        _ => None,
    };

    Ok(Some(AurPackageInfo {
        name: r.name,
        package_base: r.package_base,
        version: r.version,
        description: r.description.unwrap_or_default(),
        url: r.url,
        maintainer: r.maintainer,
        popularity: r.popularity,
        votes: r.num_votes,
        out_of_date: r.out_of_date,
        license,
        depends: r.depends.unwrap_or_default(),
        make_depends: r.make_depends.unwrap_or_default(),
        is_installed,
        installed_version,
    }))
}

/// Retrieves the raw PKGBUILD script for an AUR package or official Arch package.
/// Checks official AUR cgit if it is an authentic AUR package, or Arch Linux GitLab packaging.
pub fn aur_get_pkgbuild(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Package name cannot be empty".to_string());
    }

    // 1. If it is a real AUR package, retrieve via AUR cgit using package_base
    if let Ok(Some(info)) = aur_get_info_rpc(trimmed) {
        let pkgbase = info.package_base.as_deref().unwrap_or(trimmed);
        let encoded = percent_encoding::utf8_percent_encode(pkgbase, percent_encoding::NON_ALPHANUMERIC).to_string();
        let url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", encoded);

        if let Ok(resp) = ureq::get(&url)
            .set("User-Agent", "Ryzora/0.1.0")
            .timeout(std::time::Duration::from_secs(6))
            .call()
        {
            if let Ok(content) = resp.into_string() {
                return Ok(content);
            }
        }
    }

    // 2. If not found in AUR, check official Arch Linux GitLab packaging
    // Validate package name characters before URL interpolation
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+') {
        return Err(format!("Invalid package name '{}'", trimmed));
    }

    let gitlab_url = format!(
        "https://gitlab.archlinux.org/archlinux/packaging/packages/{}/-/raw/main/PKGBUILD",
        trimmed
    );

    if let Ok(gl_resp) = ureq::get(&gitlab_url)
        .set("User-Agent", "Ryzora/0.1.0")
        .timeout(std::time::Duration::from_secs(6))
        .call()
    {
        if let Ok(content) = gl_resp.into_string() {
            return Ok(content);
        }
    }

    Err(format!("PKGBUILD for '{}' was not found in AUR or official Arch Linux packaging.", trimmed))
}

/// Lists all installed packages on the system that do NOT belong to official sync repositories.
/// These are foreign packages (installed from AUR or local makepkg). Zero subprocesses.
pub fn list_installed_foreign_packages() -> Result<Vec<AurPackageInfo>, String> {
    let local_dir = resolve_local_dir(None);
    let sync_dir = resolve_sync_dir(None);

    let all_installed = list_installed_pacman_packages_in(&local_dir)?;

    // Collect all package names present in official sync databases
    let mut sync_pkg_names: HashSet<String> = HashSet::new();
    if sync_dir.exists() {
        if let Ok(entries) = fs::read_dir(&sync_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if file_name.ends_with(".db") && !file_name.ends_with(".db.sig") {
                    if let Ok(file) = fs::File::open(&path) {
                        let gz = flate2::read::GzDecoder::new(file);
                        let mut archive = tar::Archive::new(gz);
                        if let Ok(entries) = archive.entries() {
                            for e in entries.flatten() {
                                if let Ok(p) = e.path() {
                                    if let Some(first_comp) = p.iter().next() {
                                        let dir_name = first_comp.to_string_lossy();
                                        if let Some(idx) = dir_name.rfind('-') {
                                            if let Some(second_idx) = dir_name[..idx].rfind('-') {
                                                let pkg_name = &dir_name[..second_idx];
                                                sync_pkg_names.insert(pkg_name.to_lowercase());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Filter installed packages that are NOT in sync repositories
    let mut foreign = Vec::new();
    for p in all_installed {
        if !sync_pkg_names.contains(&p.name.to_lowercase()) {
            let ver = p.version.clone();
            let inst_ver = p.installed_version.unwrap_or(ver);
            foreign.push(AurPackageInfo {
                name: p.name,
                package_base: None,
                version: p.version,
                description: p.description,
                url: p.url,
                maintainer: None,
                popularity: None,
                votes: None,
                out_of_date: None,
                license: p.license.map(|l| vec![l]),
                depends: p.dependencies,
                make_depends: Vec::new(),
                is_installed: true,
                installed_version: Some(inst_ver),
            });
        }
    }

    Ok(foreign)
}

/// Builds and installs an AUR package unprivileged.
/// Never executes PKGBUILD or makepkg as root.
pub fn aur_build_and_install_package(pkg_name: &str) -> Result<String, String> {
    let trimmed = pkg_name.trim();
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+') {
        return Err(format!("Invalid AUR package name '{}'", trimmed));
    }

    let (has_yay, _) = crate::system::check_binary("yay");
    let (has_paru, _) = crate::system::check_binary("paru");

    // If yay or paru is available, invoke it unprivileged.
    if has_yay {
        let output = Command::new("yay")
            .args(["-S", "--noconfirm", "--sudo", "pkexec", trimmed])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("Failed to run yay: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AUR installation with yay failed: {}", err.trim()));
        }
        return Ok(format!("Successfully built and installed AUR package '{}' via yay", trimmed));
    } else if has_paru {
        let output = Command::new("paru")
            .args(["-S", "--noconfirm", "--sudo", "pkexec", trimmed])
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|e| format!("Failed to run paru: {}", e))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("AUR installation with paru failed: {}", err.trim()));
        }
        return Ok(format!("Successfully built and installed AUR package '{}' via paru", trimmed));
    }

    // Fallback: dedicated unprivileged makepkg workflow
    let home = crate::snapshot::get_home_dir();
    let build_root = home.join(".cache/ryzora/aur-build").join(trimmed);
    fs::create_dir_all(&build_root).map_err(|e| format!("Failed to create build dir: {}", e))?;

    // Download snapshot tarball
    let tarball_url = format!("https://aur.archlinux.org/cgit/aur.git/snapshot/{}.tar.gz", trimmed);
    let resp = ureq::get(&tarball_url)
        .set("User-Agent", "Ryzora/0.1.0")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| format!("Failed to download AUR tarball: {}", e))?;

    let gz = flate2::read::GzDecoder::new(resp.into_reader());
    let mut archive = tar::Archive::new(gz);
    archive.unpack(&build_root).map_err(|e| format!("Failed to unpack snapshot: {}", e))?;

    let source_dir = build_root.join(trimmed);
    let working_dir = if source_dir.exists() { source_dir } else { build_root.clone() };

    // Run unprivileged makepkg
    let output = Command::new("makepkg")
        .args(["-s", "--noconfirm"])
        .current_dir(&working_dir)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("makepkg execution failed: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("makepkg failed: {}", err.trim()));
    }

    // Find produced .pkg.tar.zst
    let mut pkg_file: Option<PathBuf> = None;
    if let Ok(entries) = fs::read_dir(&working_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.ends_with(".pkg.tar.zst") {
                    pkg_file = Some(p);
                    break;
                }
            }
        }
    }

    let built_package = pkg_file.ok_or_else(|| "Build completed but no .pkg.tar.zst was found".to_string())?;

    // Install built package using pkexec pacman -U
    let install_output = Command::new("pkexec")
        .args(["pacman", "-U", "--noconfirm"])
        .arg(&built_package)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| format!("Failed to invoke pkexec pacman -U: {}", e))?;

    if !install_output.status.success() {
        let err = String::from_utf8_lossy(&install_output.stderr);
        return Err(format!("Failed to install built package: {}", err.trim()));
    }

    Ok(format!("Successfully built and installed '{}'", trimmed))
}

/// Exposes AUR cleanup locations for Phase 26 storage management.
pub fn get_aur_cleanup_info(pkg_name: &str) -> AurCleanupInfo {
    let home = crate::snapshot::get_home_dir();
    let build_dir = home.join(".cache/ryzora/aur-build").join(pkg_name);
    let yay_cache = home.join(".cache/yay").join(pkg_name);
    let paru_cache = home.join(".cache/paru/clone").join(pkg_name);

    AurCleanupInfo {
        package_name: pkg_name.to_string(),
        build_dir: if build_dir.exists() { Some(build_dir.display().to_string()) } else { None },
        yay_cache: if yay_cache.exists() { Some(yay_cache.display().to_string()) } else { None },
        paru_cache: if paru_cache.exists() { Some(paru_cache.display().to_string()) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aur_get_pkgbuild_official_arch_gitlab_fallback() {
        let pkgbuild = aur_get_pkgbuild("gnome-2048");
        assert!(pkgbuild.is_ok(), "gnome-2048 PKGBUILD should be fetched from Arch GitLab: {:?}", pkgbuild.err());
        let content = pkgbuild.unwrap();
        assert!(content.contains("gnome-2048"));
    }

    #[test]
    fn test_aur_get_pkgbuild_aur_package() {
        let pkgbuild = aur_get_pkgbuild("spotify");
        assert!(pkgbuild.is_ok(), "spotify PKGBUILD should be fetched from AUR cgit: {:?}", pkgbuild.err());
        assert!(pkgbuild.unwrap().contains("pkgname=spotify"));
    }
    #[test]
    fn test_aur_search_relevance_sorting() {
        let mut mock_list = vec![
            AurPackageInfo {
                name: "spotify-tui".to_string(),
                package_base: Some("spotify-tui".to_string()),
                version: "0.25.0".to_string(),
                description: "Spotify for the terminal".to_string(),
                url: None,
                maintainer: None,
                popularity: Some(15.0),
                votes: Some(400),
                out_of_date: None,
                license: None,
                depends: vec![],
                make_depends: vec![],
                is_installed: false,
                installed_version: None,
            },
            AurPackageInfo {
                name: "spotify".to_string(),
                package_base: Some("spotify".to_string()),
                version: "1.2.0".to_string(),
                description: "Spotify desktop client".to_string(),
                url: None,
                maintainer: None,
                popularity: Some(85.0),
                votes: Some(3500),
                out_of_date: None,
                license: None,
                depends: vec![],
                make_depends: vec![],
                is_installed: false,
                installed_version: None,
            },
            AurPackageInfo {
                name: "audion-bin".to_string(),
                package_base: Some("audion-bin".to_string()),
                version: "1.0.0".to_string(),
                description: "Spotify player alternative".to_string(),
                url: None,
                maintainer: None,
                popularity: Some(2.0),
                votes: Some(10),
                out_of_date: None,
                license: None,
                depends: vec![],
                make_depends: vec![],
                is_installed: false,
                installed_version: None,
            },
        ];

        let query_lower = "spotify".to_string();
        mock_list.sort_by(|a, b| {
            let a_name = a.name.to_lowercase();
            let b_name = b.name.to_lowercase();
            let a_exact = a_name == query_lower;
            let b_exact = b_name == query_lower;
            if a_exact != b_exact {
                return b_exact.cmp(&a_exact);
            }
            let a_starts = a_name.starts_with(&query_lower);
            let b_starts = b_name.starts_with(&query_lower);
            if a_starts != b_starts {
                return b_starts.cmp(&a_starts);
            }
            b.popularity.partial_cmp(&a.popularity).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.votes.cmp(&a.votes))
        });

        assert_eq!(mock_list[0].name, "spotify", "Exact match must rank first");
        assert_eq!(mock_list[1].name, "spotify-tui", "Prefix match must rank second");
        assert_eq!(mock_list[2].name, "audion-bin", "General substring must rank last");
    }
}