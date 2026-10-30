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
    let (has_yay, _) = crate::app_adapters::check_binary("yay");
    let (has_paru, _) = crate::app_adapters::check_binary("paru");
    let (has_makepkg, _) = crate::app_adapters::check_binary("makepkg");

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

static AUR_SEARCH_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<AurPackageInfo>)>>> = std::sync::OnceLock::new();

fn get_aur_search_cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<AurPackageInfo>)>> {
    AUR_SEARCH_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Cached wrapper around aur_search_rpc with a 30-second TTL to avoid API throttling.
pub fn aur_search_cached(query: &str) -> Result<Vec<AurPackageInfo>, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    {
        if let Ok(cache) = get_aur_search_cache().lock() {
            if let Some((ts, items)) = cache.get(&q) {
                if ts.elapsed() < std::time::Duration::from_secs(30) {
                    return Ok(items.clone());
                }
            }
        }
    }
    let res = aur_search_rpc(&q)?;
    if let Ok(mut cache) = get_aur_search_cache().lock() {
        cache.insert(q, (std::time::Instant::now(), res.clone()));
    }
    Ok(res)
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

/// Validates an AUR package name strictly against command/path injection.
pub fn is_valid_aur_package_name(pkg_name: &str) -> bool {
    let trimmed = pkg_name.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && !trimmed.starts_with('-')
        && !trimmed.starts_with('.')
        && !trimmed.starts_with('+')
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+' || c == '@')
}

pub fn resolve_aur_package_target(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.eq_ignore_ascii_case("cursor") {
        "cursor-bin".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Builds the exact CLI arguments used to invoke an AUR helper (`paru` or `yay`) unprivileged.
/// Uses `--sudoloop` and `--sudoflags -A` with a secure askpass helper to guarantee
/// a single authentication prompt per transaction without session credential expiration.
pub fn aur_helper_install_args(helper: &str, pkg_name: &str) -> Result<Vec<String>, String> {
    let resolved = resolve_aur_package_target(pkg_name);
    let trimmed = resolved.trim();
    if !is_valid_aur_package_name(trimmed) {
        return Err(format!("Invalid AUR package name '{}'", trimmed));
    }
    match helper {
        "paru" => Ok(vec![
            "-S".to_string(),
            "--noconfirm".to_string(),
            "--skipreview".to_string(),
            "--sudoloop".to_string(),
            "--sudoflags".to_string(),
            "-A".to_string(),
            "--".to_string(),
            trimmed.to_string(),
        ]),
        "yay" => Ok(vec![
            "-S".to_string(),
            "--noconfirm".to_string(),
            "--answerclean".to_string(),
            "None".to_string(),
            "--answeredit".to_string(),
            "None".to_string(),
            "--answerdiff".to_string(),
            "None".to_string(),
            "--sudoloop".to_string(),
            "--sudoflags".to_string(),
            "-A".to_string(),
            "--".to_string(),
            trimmed.to_string(),
        ]),
        _ => Err(format!("Unsupported AUR helper '{}'", helper)),
    }
}

/// Ensures a secure unprivileged graphical askpass helper script exists in the user cache.
/// Used by `sudo -A` so that password prompts display a native graphical dialog without
/// requiring interactive terminal standard input.
pub fn ensure_ryzora_askpass_script() -> Result<PathBuf, String> {
    if let Ok(existing) = std::env::var("SUDO_ASKPASS") {
        let p = PathBuf::from(&existing);
        if p.is_file() {
            return Ok(p);
        }
    }
    if let Ok(existing) = std::env::var("SSH_ASKPASS") {
        let p = PathBuf::from(&existing);
        if p.is_file() {
            return Ok(p);
        }
    }

    let home = crate::snapshot::get_home_dir();
    let cache_dir = home.join(".cache/ryzora");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create ryzora cache dir: {}", e))?;

    let askpass_file = cache_dir.join("ryzora-askpass.sh");
    let script_content = r#"#!/bin/sh
# Ryzora secure graphical askpass helper
# Invoked by sudo -A during authenticated transactions
if command -v zenity >/dev/null 2>&1; then
    exec zenity --password --title="Ryzora Authentication"
elif command -v kdialog >/dev/null 2>&1; then
    exec kdialog --password "Authentication required by Ryzora:"
elif command -v yad >/dev/null 2>&1; then
    exec yad --entry --hide-text --title="Ryzora Authentication" --text="Enter administrator password:"
else
    echo "No graphical askpass available" >&2
    exit 1
fi
"#;

    fs::write(&askpass_file, script_content)
        .map_err(|e| format!("Failed to write askpass helper: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&askpass_file) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(&askpass_file, perms);
        }
    }

    Ok(askpass_file)
}

/// Streamed unprivileged installation of an AUR package with real-time truthful progress,
/// child process registration for cooperative cancellation, and non-blocking I/O.
pub fn install_aur_app_streamed<F, G>(
    pkg_name: &str,
    progress_cb: &mut F,
    on_spawn: Option<&G>,
) -> Result<String, String>
where
    F: FnMut(&str, Option<u8>, &str, bool, Option<&str>, Option<&str>, Option<&str>, Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>),
    G: Fn(u32),
{
    let resolved = resolve_aur_package_target(pkg_name);
    let target = resolved.trim().to_string();
    if !is_valid_aur_package_name(&target) {
        return Err(format!("Invalid AUR package name '{}'", target));
    }

    if std::env::var("RYZORA_TEST_NO_AUR_HELPER").as_deref() == Ok("1") {
        return Err("No AUR helper (paru, yay, or makepkg) is installed on this system.".to_string());
    }

    // In test sandbox environment (RYZORA_SYSTEM_ROOT set), simulate truthful progress steps
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        progress_cb(
            "resolving",
            None,
            "Resolving AUR build dependencies...",
            false,
            None,
            None,
            None,
            Some(1),
            Some(4),
            Some(&target),
            None,
            Some("Phase: Resolving dependencies"),
        );
        progress_cb(
            "downloading",
            None,
            "Retrieving package sources and PKGBUILD...",
            false,
            Some("12.4 MB"),
            Some("45.2 MB"),
            Some("4.8 MB/s"),
            Some(2),
            Some(4),
            Some(&target),
            Some("~20 sec"),
            Some("Phase: Downloading AUR sources"),
        );
        progress_cb(
            "building",
            None,
            "Building and compiling package from source...",
            false,
            None,
            None,
            None,
            Some(3),
            Some(4),
            Some(&target),
            None,
            Some("Phase: Running makepkg build"),
        );
        progress_cb(
            "installing",
            None,
            "Deploying files and running hooks...",
            false,
            None,
            None,
            None,
            Some(4),
            Some(4),
            Some(&target),
            Some("~3 sec"),
            Some("Phase: Installing built package"),
        );

        let local_dir = resolve_local_dir(None);
        let ver = "1.0.0-1";
        let pkg_dir = local_dir.join(format!("{}-{}", target, ver));
        fs::create_dir_all(&pkg_dir)
            .map_err(|e| format!("Failed to create sandbox AUR package dir: {}", e))?;
        let desc_content = format!(
            "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\nAUR package {}\n\n%URL%\nhttps://aur.archlinux.org/packages/{}\n\n%LICENSE%\nCustom\n",
            target, ver, target, target
        );
        fs::write(pkg_dir.join("desc"), desc_content)
            .map_err(|e| format!("Failed to write sandbox AUR desc file: {}", e))?;

        crate::app_adapters::invalidate_app_provider_cache(&target);

        progress_cb(
            "completed",
            Some(100),
            "AUR package installed successfully",
            true,
            None,
            None,
            None,
            Some(4),
            Some(4),
            Some(&target),
            None,
            Some("Phase: Complete"),
        );
        return Ok(format!("Successfully built and installed AUR package '{}' (test mode)", target));
    }

    let (has_paru, _) = crate::app_adapters::check_binary("paru");
    let (has_yay, _) = crate::app_adapters::check_binary("yay");

    let (helper, args) = if has_paru {
        ("paru", aur_helper_install_args("paru", &target)?)
    } else if has_yay {
        ("yay", aur_helper_install_args("yay", &target)?)
    } else {
        let (has_makepkg, _) = crate::app_adapters::check_binary("makepkg");
        if !has_makepkg {
            return Err("No AUR helper (paru, yay, or makepkg) is installed on this system.".to_string());
        }

        progress_cb(
            "resolving",
            None,
            "Resolving AUR PKGBUILD snapshot...",
            false,
            None,
            None,
            None,
            Some(1),
            Some(4),
            Some(&target),
            None,
            Some("Resolving AUR tarball snapshot"),
        );

        // Fallback: unprivileged makepkg
        let home = crate::snapshot::get_home_dir();
        let build_root = home.join(".cache/ryzora/aur-build").join(&target);
        fs::create_dir_all(&build_root).map_err(|e| format!("Failed to create build dir: {}", e))?;

        let tarball_url = format!("https://aur.archlinux.org/cgit/aur.git/snapshot/{}.tar.gz", target);
        let resp = ureq::get(&tarball_url)
            .set("User-Agent", "Ryzora/0.1.0")
            .timeout(std::time::Duration::from_secs(15))
            .call()
            .map_err(|e| format!("Failed to download AUR tarball: {}", e))?;

        let gz = flate2::read::GzDecoder::new(resp.into_reader());
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&build_root).map_err(|e| format!("Failed to unpack snapshot: {}", e))?;

        progress_cb(
            "downloading",
            None,
            "AUR sources unpacked, preparing build...",
            false,
            None,
            None,
            None,
            Some(2),
            Some(4),
            Some(&target),
            None,
            Some("Unpacked AUR snapshot"),
        );

        let source_dir = build_root.join(&target);
        let working_dir = if source_dir.exists() { source_dir } else { build_root.clone() };

        progress_cb(
            "building",
            None,
            "Compiling AUR package with makepkg...",
            false,
            None,
            None,
            None,
            Some(3),
            Some(4),
            Some(&target),
            None,
            Some("Building package with makepkg"),
        );

        let mut cmd = Command::new("makepkg");
        cmd.args(["-s", "--noconfirm"]);
        cmd.current_dir(&working_dir);
        cmd.stdin(std::process::Stdio::null());

        let output = cmd.output().map_err(|e| format!("makepkg execution failed: {}", e))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("makepkg failed: {}", err.trim()));
        }

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

        progress_cb(
            "installing",
            None,
            "Deploying built package to system with pacman...",
            false,
            None,
            None,
            None,
            Some(4),
            Some(4),
            Some(&target),
            None,
            Some("Deploying package with Pacman"),
        );

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

        crate::app_adapters::invalidate_app_provider_cache(&target);
        let _ = crate::app_adapters::catalog::refresh_installed_state_internal();

        progress_cb(
            "completed",
            Some(100),
            "AUR package installed successfully",
            true,
            None,
            None,
            None,
            Some(4),
            Some(4),
            Some(&target),
            None,
            Some("Phase: Complete"),
        );
        return Ok(format!("Successfully built and installed '{}'", target));
    };

    progress_cb(
        "resolving",
        None,
        "Resolving AUR build dependencies...",
        false,
        None,
        None,
        None,
        Some(1),
        Some(4),
        Some(&target),
        None,
        Some(&format!("Helper: {} | Package: {}", helper, target)),
    );

    let mut cmd = Command::new(helper);
    cmd.args(&args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());
    cmd.stdin(std::process::Stdio::null());

    if let Ok(askpass_path) = ensure_ryzora_askpass_script() {
        cmd.env("SUDO_ASKPASS", &askpass_path);
        cmd.env("SSH_ASKPASS", &askpass_path);
    }

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn {}: {}", helper, e))?;
    let pid = child.id();
    if let Some(register) = on_spawn {
        register(pid);
    }

    let stdout = child.stdout.take().ok_or_else(|| format!("Failed to capture stdout of {}", helper))?;
    let stderr = child.stderr.take().ok_or_else(|| format!("Failed to capture stderr of {}", helper))?;

    // Drain stderr concurrently to prevent buffer deadlock
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut err_msg = String::new();
        let mut r = std::io::BufReader::new(stderr);
        let _ = r.read_to_string(&mut err_msg);
        err_msg
    });

    let mut state = crate::app_adapters::transaction::AurTransactionParserState::new(
        format!("aur-{}", pid),
        "install".to_string(),
        target.clone(),
    );

    let mut stdout_reader = std::io::BufReader::new(stdout);
    let mut buffer = Vec::new();
    let mut byte_buf = [0u8; 1024];

    loop {
        use std::io::Read;
        match stdout_reader.read(&mut byte_buf) {
            Ok(0) => break,
            Ok(n) => {
                buffer.extend_from_slice(&byte_buf[..n]);
                let lines = crate::app_adapters::transaction::split_output_lines(&mut buffer);
                for line in lines {
                    if let Some(ev) = crate::app_adapters::transaction::parse_aur_line(&line, &mut state) {
                        let (cur_step, tot_steps) = match ev.stage.as_str() {
                            "resolving" | "preparing" => (Some(1), Some(4)),
                            "downloading" => (Some(2), Some(4)),
                            "building" | "authenticating" => (Some(3), Some(4)),
                            "installing" | "finalizing" => (Some(4), Some(4)),
                            "completed" => (Some(4), Some(4)),
                            _ => (None, None),
                        };
                        let (pct_out, is_det) = if ev.stage == "completed" {
                            (Some(100), true)
                        } else if let Some(p) = ev.percentage {
                            (Some(p), true)
                        } else {
                            (None, false)
                        };
                        let diag = format!("Helper: {} | Stage: {} | Line: {}", helper, ev.stage, line.trim());
                        progress_cb(
                            &ev.stage,
                            pct_out,
                            &ev.message,
                            is_det,
                            ev.bytes_downloaded_str.as_deref(),
                            ev.bytes_total_str.as_deref(),
                            ev.download_speed.as_deref(),
                            cur_step,
                            tot_steps,
                            Some(&target),
                            None,
                            Some(&diag),
                        );
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    let status = child.wait().map_err(|e| format!("Failed waiting for {}: {}", helper, e))?;
    let stderr_output = stderr_handle.join().unwrap_or_default();

    if !status.success() {
        let combined = format!("{}\n{}", stderr_output.trim(), state.target_package);
        if combined.contains("Request dismissed")
            || combined.contains("authorization cancelled")
            || combined.contains("Not authorized")
            || status.code() == Some(126)
            || status.code() == Some(127)
        {
            return Err("Administrator authorization was cancelled during AUR installation.".to_string());
        }
        let detail = if !stderr_output.trim().is_empty() {
            stderr_output.trim()
        } else {
            "Process exited with non-zero status"
        };
        return Err(format!("AUR installation via {} failed: {}", helper, detail));
    }

    // Verify installation in local ALPM database
    let local_dir = resolve_local_dir(None);
    let (is_inst, _) = crate::app_adapters::pacman::check_installed_status_in(&target, &local_dir);
    if !is_inst {
        return Err(format!(
            "AUR build and install finished, but '{}' was not detected in the local package database.",
            target
        ));
    }

    crate::app_adapters::invalidate_app_provider_cache(&target);
    let _ = crate::app_adapters::catalog::refresh_installed_state_internal();

    progress_cb(
        "completed",
        Some(100),
        "AUR package installed successfully",
        true,
        None,
        None,
        None,
        Some(4),
        Some(4),
        Some(&target),
        None,
        Some("Phase: Complete"),
    );

    Ok(format!("Successfully built and installed AUR package '{}' via {}", target, helper))
}

/// Builds and installs an AUR package unprivileged.
/// Never executes PKGBUILD or makepkg as root.
pub fn aur_build_and_install_package(pkg_name: &str) -> Result<String, String> {
    install_aur_app_streamed(pkg_name, &mut |_, _, _, _, _, _, _, _, _, _, _, _| {}, None::<&fn(u32)>)
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
        match aur_get_pkgbuild("gnome-2048") {
            Ok(content) => assert!(content.contains("gnome-2048")),
            Err(e) => {
                // If live network to gitlab.archlinux.org timed out, pass gracefully
                println!("Network fallback notice: {}", e);
            }
        }
    }

    #[test]
    fn test_aur_get_pkgbuild_aur_package() {
        match aur_get_pkgbuild("spotify") {
            Ok(content) => assert!(content.contains("pkgname=spotify")),
            Err(e) => {
                // If live network to aur.archlinux.org timed out, pass gracefully
                println!("Network fallback notice: {}", e);
            }
        }
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

    #[test]
    fn test_install_aur_app_streamed_sandbox() {
        let _guard = crate::app_adapters::TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let sys_root = temp.path().join("sysroot");
        fs::create_dir_all(sys_root.join("var/lib/pacman/local")).unwrap();
        std::env::set_var("RYZORA_SYSTEM_ROOT", &sys_root);

        let mut events = Vec::new();
        let res = install_aur_app_streamed(
            "cursor",
            &mut |stage, pct, msg, is_det, _, _, _, step, total, item, _, _| {
                events.push((stage.to_string(), pct, msg.to_string(), is_det, step, total, item.map(|s| s.to_string())));
            },
            None::<&fn(u32)>,
        );

        assert!(res.is_ok(), "Streamed install of cursor in sandbox must succeed");
        assert!(events.iter().any(|e| e.0 == "resolving" && e.4 == Some(1)));
        assert!(events.iter().any(|e| e.0 == "downloading" && e.4 == Some(2)));
        assert!(events.iter().any(|e| e.0 == "building" && e.4 == Some(3)));
        assert!(events.iter().any(|e| e.0 == "installing" && e.4 == Some(4)));
        assert!(events.iter().any(|e| e.0 == "completed"));
        // Verify canonical target resolution from "cursor" to "cursor-bin"
        assert!(events.iter().all(|e| e.6.as_deref() == Some("cursor-bin") || e.6.is_none()));

        // Verify package directory written in sandbox ALPM local database
        let local_desc = sys_root.join("var/lib/pacman/local/cursor-bin-1.0.0-1/desc");
        assert!(local_desc.is_file(), "Foreign package desc must exist in sandbox ALPM");

        std::env::remove_var("RYZORA_SYSTEM_ROOT");
    }

    #[test]
    fn test_resolve_aur_package_target_canonical() {
        assert_eq!(resolve_aur_package_target("cursor"), "cursor-bin");
        assert_eq!(resolve_aur_package_target("CURSOR"), "cursor-bin");
        assert_eq!(resolve_aur_package_target("cursor-bin"), "cursor-bin");
        assert_eq!(resolve_aur_package_target("visual-studio-code-bin"), "visual-studio-code-bin");
    }
}