//! Ryzora Flatpak & Flathub Adapter — Phase 25
//!
//! Provides native Flatpak application management:
//! - Detection of Flatpak binary and configured remotes (Flathub).
//! - Authoritative listing of installed Flatpak applications via Flatpak CLI.
//! - Search across Flathub remotes.
//! - App execution, installation, and uninstallation using Flatpak's native sandboxing.
//! - Cleanup awareness: exposes user data and cache directories for Phase 26.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

use std::sync::RwLock;
use std::time::{Duration, Instant};

static INSTALLED_FLATPAK_CACHE: RwLock<Option<(Instant, std::collections::HashSet<String>)>> = RwLock::new(None);

pub fn get_installed_flatpak_ids() -> std::collections::HashSet<String> {
    if let Ok(read) = INSTALLED_FLATPAK_CACHE.read() {
        if let Some((instant, ref set)) = *read {
            if instant.elapsed() < Duration::from_secs(30) {
                return set.clone();
            }
        }
    }
    let list = list_installed_flatpak_apps().unwrap_or_default();
    let set: std::collections::HashSet<String> = list.into_iter().map(|a| a.id.to_lowercase()).collect();
    if let Ok(mut write) = INSTALLED_FLATPAK_CACHE.write() {
        *write = Some((Instant::now(), set.clone()));
    }
    set
}

pub fn invalidate_flatpak_cache() {
    if let Ok(mut write) = INSTALLED_FLATPAK_CACHE.write() {
        *write = None;
    }
    if let Ok(mut scope_cache) = SCOPE_CACHE.lock() {
        *scope_cache = None;
    }
}

static SCOPE_CACHE: std::sync::Mutex<Option<(std::time::Instant, bool, bool)>> = std::sync::Mutex::new(None);
static REMOTE_ADD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn get_cached_flathub_scopes() -> (bool, bool) {
    if let Ok(mut cache) = SCOPE_CACHE.lock() {
        if let Some((instant, user, system)) = *cache {
            if instant.elapsed() < std::time::Duration::from_secs(30) {
                return (user, system);
            }
        }
        let user = list_flatpak_remote_names_for_scope("--user")
            .iter()
            .any(|n| n.eq_ignore_ascii_case("flathub"));
        let system = list_flatpak_remote_names_for_scope("--system")
            .iter()
            .any(|n| n.eq_ignore_ascii_case("flathub"));
        *cache = Some((std::time::Instant::now(), user, system));
        (user, system)
    } else {
        (false, false)
    }
}

pub fn ensure_flathub_remote_for_scope<F>(scope: &str, mut progress_cb: Option<&mut F>) -> Result<(), String>
where
    F: FnMut(&str, Option<u8>, &str, bool, Option<&str>, Option<&str>, Option<&str>, Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>),
{
    if scope != "--user" && scope != "--system" {
        return Err(format!("Invalid Flatpak scope '{}'. Must be '--user' or '--system'.", scope));
    }

    let (user, system) = get_cached_flathub_scopes();
    let is_configured = match scope {
        "--user" => user,
        "--system" => system,
        _ => false,
    };
    if is_configured {
        return Ok(());
    }

    let _guard = REMOTE_ADD_LOCK.lock().map_err(|e| format!("Remote lock poisoned: {}", e))?;
    let (user_now, system_now) = get_cached_flathub_scopes();
    if (scope == "--user" && user_now) || (scope == "--system" && system_now) {
        return Ok(());
    }

    if let Some(ref mut cb) = progress_cb {
        cb("resolving", None, "Configuring Flathub remote repository...", false, None, None, None, None, None, None, None, Some(&format!("Setting up Flathub remote for {}", scope)));
    }

    let status = Command::new("flatpak")
        .args(["remote-add", scope, "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"])
        .status()
        .map_err(|e| format!("Failed to configure Flathub remote for {}: {}", scope, e))?;

    invalidate_flatpak_cache();

    if !status.success() {
        return Err(format!("Failed to register Flathub remote for {}", scope));
    }
    Ok(())
}

pub fn detect_installed_flatpak_scope(app_id: &str) -> Option<&'static str> {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak { return None; }
    if Command::new("flatpak")
        .args(["info", "--user", app_id])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("--user");
    }
    if Command::new("flatpak")
        .args(["info", "--system", app_id])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("--system");
    }
    None
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakAppInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub description: String,
    pub is_installed: bool,
    pub runtime: Option<String>,
    pub sdk: Option<String>,
    pub installed_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakStatus {
    pub has_flatpak: bool,
    pub version: Option<String>,
    pub has_flathub: bool,
    pub remotes: Vec<String>,
    pub total_installed_apps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatpakCleanupInfo {
    pub app_id: String,
    pub user_data_path: Option<String>,
    pub cache_path: Option<String>,
    pub system_app_path: Option<String>,
}

/// Detects the host Flatpak installation and remote configurations.
pub fn detect_flatpak_status() -> FlatpakStatus {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return FlatpakStatus {
            has_flatpak: false,
            version: None,
            has_flathub: false,
            remotes: Vec::new(),
            total_installed_apps: 0,
        };
    }

    let version = Command::new("flatpak")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });

    let remotes = list_flatpak_remote_names();
    let has_flathub = remotes.iter().any(|name| name.eq_ignore_ascii_case("flathub"));

    let installed_apps = list_installed_flatpak_apps().unwrap_or_default();

    FlatpakStatus {
        has_flatpak: true,
        version,
        has_flathub,
        remotes,
        total_installed_apps: installed_apps.len(),
    }
}

/// Lists all installed Flatpak applications with metadata.
pub fn list_installed_flatpak_apps() -> Result<Vec<FlatpakAppInfo>, String> {
    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let app_dir = PathBuf::from(&sys_root).join("var/lib/flatpak/app");
        let mut apps = Vec::new();
        if app_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&app_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let id = entry.file_name().to_string_lossy().to_string();
                        if is_valid_flatpak_id(&id) {
                            let ver_file = entry.path().join("version");
                            let version = std::fs::read_to_string(&ver_file)
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| "1.0.0".to_string());
                            let name = id.split('.').last().unwrap_or(&id).to_string();
                            apps.push(FlatpakAppInfo {
                                id,
                                name,
                                version,
                                branch: "stable".to_string(),
                                origin: "flathub".to_string(),
                                description: "Flatpak application".to_string(),
                                is_installed: true,
                                runtime: None,
                                sdk: None,
                                installed_size: None,
                            });
                        }
                    }
                }
            }
        }
        return Ok(apps);
    }

    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(["list", "--app", "--columns=application,name,version,branch,origin,description"])
        .output()
        .map_err(|e| format!("Failed to run flatpak list: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("flatpak list exited with error: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 2 {
            let id = cols[0].trim().to_string();
            let name = cols[1].trim().to_string();
            let version = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let branch = cols.get(3).map(|s| s.trim().to_string()).unwrap_or_else(|| "stable".to_string());
            let origin = cols.get(4).map(|s| s.trim().to_string()).unwrap_or_else(|| "flathub".to_string());
            let description = cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default();

            apps.push(FlatpakAppInfo {
                id,
                name,
                version,
                branch,
                origin,
                description,
                is_installed: true,
                runtime: None,
                sdk: None,
                installed_size: None,
            });
        }
    }

    Ok(apps)
}

/// Searches Flathub applications matching the query.
pub fn search_flathub_apps(query: &str) -> Result<Vec<FlatpakAppInfo>, String> {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Ok(Vec::new());
    }

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(["search", "--columns=application,name,version,branch,remotes,description", trimmed])
        .output()
        .map_err(|e| format!("Failed to run flatpak search: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let installed_ids = get_installed_flatpak_ids();

    let mut results = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 2 {
            let id = cols[0].trim().to_string();
            // Skip sub-plugins or runtimes if searching applications
            if id.contains(".Plugin.") {
                continue;
            }
            let name = cols[1].trim().to_string();
            let version = cols.get(2).map(|s| s.trim().to_string()).unwrap_or_default();
            let branch = cols.get(3).map(|s| s.trim().to_string()).unwrap_or_else(|| "stable".to_string());
            let origin = cols.get(4).map(|s| s.trim().to_string()).unwrap_or_else(|| "flathub".to_string());
            let description = cols.get(5).map(|s| s.trim().to_string()).unwrap_or_default();
            let is_installed = installed_ids.contains(&id.to_lowercase());

            results.push(FlatpakAppInfo {
                id,
                name,
                version,
                branch,
                origin,
                description,
                is_installed,
                runtime: None,
                sdk: None,
                installed_size: None,
            });
        }
    }

    Ok(results)
}

/// Retrieves detailed information about a specific Flatpak app.
pub fn get_flatpak_app_info(app_id: &str) -> Result<Option<FlatpakAppInfo>, String> {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Ok(None);
    }

    // First check installed apps
    let installed = list_installed_flatpak_apps()?;
    if let Some(app) = installed.into_iter().find(|a| a.id.eq_ignore_ascii_case(app_id)) {
        return Ok(Some(app));
    }

    // Otherwise search Flathub
    let search_res = search_flathub_apps(app_id)?;
    Ok(search_res.into_iter().find(|a| a.id.eq_ignore_ascii_case(app_id)))
}

/// Launches a Flatpak application unprivileged.
pub fn run_flatpak_app(app_id: &str) -> Result<(), String> {
    if !is_valid_flatpak_id(app_id) {
        return Err("Invalid flatpak application ID".to_string());
    }

    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let app_dir = PathBuf::from(&sys_root).join("var/lib/flatpak/app").join(app_id);
        if !app_dir.is_dir() {
            return Err(format!("Flatpak application '{}' is not installed", app_id));
        }
        return Ok(());
    }

    let installed = get_installed_flatpak_ids();
    if !installed.contains(&app_id.to_lowercase()) {
        return Err(format!("Flatpak application '{}' is not installed", app_id));
    }

    Command::new("flatpak")
        .arg("run")
        .arg(app_id)
        .spawn()
        .map_err(|e| format!("Failed to launch flatpak application '{}': {}", app_id, e))?;

    Ok(())
}

/// Installs a Flatpak application from Flathub.
pub fn install_flatpak_app_streamed<F>(
    app_id: &str,
    mut progress_cb: F,
    on_spawn: Option<&dyn Fn(u32)>,
) -> Result<String, String>
where
    F: FnMut(&str, Option<u8>, &str, bool, Option<&str>, Option<&str>, Option<&str>, Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>),
{
    if !is_valid_flatpak_id(app_id) {
        return Err("Invalid Flatpak application ID. Select a verified Flathub source before continuing.".to_string());
    }

    if std::env::var("RYZORA_TEST_NO_FLATPAK").as_deref() == Ok("1") {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        progress_cb(
            "resolving",
            Some(10),
            "Resolving Flathub remotes and runtime dependencies...",
            true,
            None,
            None,
            None,
            Some(1),
            Some(3),
            Some("org.freedesktop.Platform"),
            Some("~45 sec"),
            Some("Elapsed: 1s | Resolving remote dependencies"),
        );
        progress_cb(
            "downloading",
            Some(45),
            "Downloading runtime archive...",
            true,
            Some("120.5 MB"),
            Some("269.0 MB"),
            Some("3.4 MB/s"),
            Some(2),
            Some(3),
            Some("org.freedesktop.Platform 24.08"),
            Some("~44 sec"),
            Some("Elapsed: 15s | Downloading shared Flatpak runtime dependency (org.freedesktop.Platform 24.08). Runtimes provide isolated base OS libraries and are shared across all Flatpak applications."),
        );
        progress_cb(
            "installing",
            Some(85),
            "Deploying sandbox files...",
            true,
            None,
            None,
            None,
            Some(3),
            Some(3),
            Some(app_id),
            Some("~5 sec"),
            Some("Elapsed: 35s | Deploying sandbox files"),
        );
        let app_dir = PathBuf::from(&sys_root).join("var/lib/flatpak/app").join(app_id);
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| format!("Failed to create sandbox flatpak dir: {}", e))?;
        let _ = std::fs::write(app_dir.join("version"), "1.0.0\n");
        invalidate_flatpak_cache();
        progress_cb(
            "completed",
            Some(100),
            "Flatpak application installed successfully",
            true,
            None,
            None,
            None,
            Some(3),
            Some(3),
            Some(app_id),
            None,
            Some("Elapsed: 40s | Completed"),
        );
        return Ok(format!("Successfully installed Flatpak application '{}' (test mode)", app_id));
    }

    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    let scope = match resolve_flathub_install_scope() {
        Some(s) => s,
        None => {
            ensure_flathub_remote_for_scope("--user", Some(&mut progress_cb))?;
            "--user"
        }
    };

    let args = vec!["install".to_string(), scope.to_string(), "-y".to_string(), "flathub".to_string(), app_id.to_string()];
    progress_cb(
        "resolving",
        None,
        "Resolving Flathub remotes and runtime dependencies...",
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&format!("Scope: {} | Remote: flathub | Target: {}", scope, app_id)),
    );

    let mut cmd = Command::new("flatpak");
    cmd.args(&args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to invoke flatpak install: {}", e))?;
    let pid = child.id();
    if let Some(register) = on_spawn {
        register(pid);
    }

    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout of flatpak".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr of flatpak".to_string())?;

    // Drain stderr concurrently to prevent pipe buffer deadlocks
    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut err_msg = String::new();
        let mut r = std::io::BufReader::new(stderr);
        let _ = r.read_to_string(&mut err_msg);
        err_msg
    });

    let mut state = crate::app_adapters::transaction::FlatpakTransactionParserState::new(
        format!("flatpak-{}", pid),
        "install".to_string(),
        app_id.to_string(),
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
                    if let Some(ev) = crate::app_adapters::transaction::parse_flatpak_line(&line, &mut state) {
                        let is_det = ev.percentage.is_some() || (ev.bytes_downloaded_str.is_some() && ev.bytes_total_str.is_some());
                        progress_cb(
                            &ev.stage,
                            ev.percentage,
                            &ev.message,
                            is_det,
                            ev.bytes_downloaded_str.as_deref(),
                            ev.bytes_total_str.as_deref(),
                            ev.download_speed.as_deref(),
                            ev.current_package_index,
                            ev.total_packages,
                            ev.current_package.as_deref(),
                            state.eta_str.as_deref(),
                            state.diagnostic_details.as_deref(),
                        );
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                let _ = child.kill();
                let _ = stderr_handle.join();
                return Err(format!("Error reading Flatpak progress: {}", e));
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait on flatpak process: {}", e))?;
    let err_msg = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        let trimmed_err = err_msg.trim();
        #[cfg(unix)]
        let killed_by_sig = {
            use std::os::unix::process::ExitStatusExt;
            matches!(status.signal(), Some(2) | Some(9) | Some(15))
        };
        #[cfg(not(unix))]
        let killed_by_sig = false;

        if trimmed_err.contains("cancelled") || status.code() == Some(130) || status.code() == Some(137) || status.code() == Some(143) || killed_by_sig {
            return Err("Installation cancelled by user.".to_string());
        }
        return Err(if !trimmed_err.is_empty() {
            format!("flatpak install failed: {}", trimmed_err)
        } else {
            format!("flatpak install failed with exit code {:?}", status.code())
        });
    }

    invalidate_flatpak_cache();
    progress_cb("completed", Some(100), "Flatpak application installed successfully", true, None, None, None, state.total_packages, state.total_packages, state.current_package.as_deref(), None, Some("Elapsed: done | Completed successfully"));
    Ok(format!("Successfully installed Flatpak application '{}'", app_id))
}

pub fn install_flatpak_app(app_id: &str) -> Result<String, String> {
    install_flatpak_app_streamed(app_id, |_stage, _pct, _msg, _is_det, _dl, _tot, _spd, _cur_step, _tot_steps, _item, _eta, _diag| {}, None)
}

/// Updates an installed Flatpak application or runtime using the exact installed scope.
pub fn update_flatpak_app_streamed<F>(
    app_id: &str,
    mut progress_cb: F,
    on_spawn: Option<&dyn Fn(u32)>,
) -> Result<String, String>
where
    F: FnMut(&str, Option<u8>, &str, bool, Option<&str>, Option<&str>, Option<&str>, Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>),
{
    if !is_valid_flatpak_id(app_id) {
        return Err("Invalid Flatpak application ID.".to_string());
    }

    if std::env::var("RYZORA_TEST_NO_FLATPAK").as_deref() == Ok("1") {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    if let Ok(_sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        progress_cb(
            "downloading",
            Some(45),
            "Downloading update package delta...",
            true,
            Some("15 MB"),
            Some("45 MB"),
            Some("6.5 MB/s"),
            Some(1),
            Some(2),
            Some(app_id),
            Some("~4 sec"),
            Some("Elapsed: 2s | Pulling updated Ostree delta"),
        );
        progress_cb(
            "installing",
            Some(85),
            "Deploying updated sandbox files...",
            true,
            None,
            None,
            None,
            Some(2),
            Some(2),
            Some(app_id),
            None,
            Some("Elapsed: 4s | Deploying application update"),
        );
        invalidate_flatpak_cache();
        progress_cb(
            "completed",
            Some(100),
            "Flatpak application updated successfully",
            true,
            None,
            None,
            None,
            Some(2),
            Some(2),
            Some(app_id),
            None,
            Some("Elapsed: 5s | Update completed"),
        );
        return Ok(format!("Successfully updated Flatpak application '{}' (test mode)", app_id));
    }

    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    let scope = detect_installed_flatpak_scope(app_id).unwrap_or("--user");
    let args = vec!["update".to_string(), scope.to_string(), "-y".to_string(), app_id.to_string()];
    progress_cb(
        "resolving",
        None,
        "Resolving Flathub remotes and pending updates...",
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(&format!("Scope: {} | Target: {}", scope, app_id)),
    );

    let mut cmd = Command::new("flatpak");
    cmd.args(&args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to invoke flatpak update: {}", e))?;
    let pid = child.id();
    if let Some(register) = on_spawn {
        register(pid);
    }

    let stdout = child.stdout.take().ok_or_else(|| "Failed to capture stdout of flatpak".to_string())?;
    let stderr = child.stderr.take().ok_or_else(|| "Failed to capture stderr of flatpak".to_string())?;

    let stderr_handle = std::thread::spawn(move || {
        use std::io::Read;
        let mut err_str = String::new();
        let mut err_reader = std::io::BufReader::new(stderr);
        let _ = err_reader.read_to_string(&mut err_str);
        err_str
    });

    let mut state = crate::app_adapters::transaction::FlatpakTransactionParserState::new(
        format!("update-{}", app_id),
        "update".to_string(),
        app_id.to_string(),
    );

    let mut reader = std::io::BufReader::new(stdout);
    let mut line_buf = Vec::new();
    let mut byte_buf = [0u8; 1024];

    loop {
        use std::io::Read;
        let n = match reader.read(&mut byte_buf) {
            Ok(0) => break,
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("Error reading flatpak stdout: {}", e)),
        };

        line_buf.extend_from_slice(&byte_buf[..n]);
        let lines = crate::app_adapters::transaction::split_output_lines(&mut line_buf);

        for line in lines {
            if let Some(ev) = crate::app_adapters::transaction::parse_flatpak_line(&line, &mut state) {
                let is_det = ev.percentage.is_some() || (ev.bytes_downloaded_str.is_some() && ev.bytes_total_str.is_some());
                progress_cb(
                    &ev.stage,
                    ev.percentage,
                    &ev.message,
                    is_det,
                    ev.bytes_downloaded_str.as_deref(),
                    ev.bytes_total_str.as_deref(),
                    ev.download_speed.as_deref(),
                    ev.current_package_index,
                    ev.total_packages,
                    ev.current_package.as_deref(),
                    state.eta_str.as_deref(),
                    state.diagnostic_details.as_deref(),
                );
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait on flatpak process: {}", e))?;
    let err_msg = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        let trimmed_err = err_msg.trim();
        #[cfg(unix)]
        let killed_by_sig = {
            use std::os::unix::process::ExitStatusExt;
            matches!(status.signal(), Some(2) | Some(9) | Some(15))
        };
        #[cfg(not(unix))]
        let killed_by_sig = false;

        if trimmed_err.contains("cancelled") || status.code() == Some(130) || status.code() == Some(137) || status.code() == Some(143) || killed_by_sig {
            return Err("Update cancelled by user.".to_string());
        }
        return Err(if !trimmed_err.is_empty() {
            format!("flatpak update failed: {}", trimmed_err)
        } else {
            format!("flatpak update failed with exit code {:?}", status.code())
        });
    }

    invalidate_flatpak_cache();
    crate::app_adapters::updates::invalidate_updates_cache();
    progress_cb("completed", Some(100), "Flatpak application updated successfully", true, None, None, None, state.total_packages, state.total_packages, state.current_package.as_deref(), None, Some("Elapsed: done | Completed successfully"));
    Ok(format!("Successfully updated Flatpak application '{}'", app_id))
}

/// Uninstalls a Flatpak application.
pub fn uninstall_flatpak_app(app_id: &str) -> Result<String, String> {
    if !is_valid_flatpak_id(app_id) {
        return Err("Invalid Flatpak application ID.".to_string());
    }

    if std::env::var("RYZORA_TEST_NO_FLATPAK").as_deref() == Ok("1") {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        let app_dir = PathBuf::from(&sys_root).join("var/lib/flatpak/app").join(app_id);
        if !app_dir.is_dir() {
            return Err(format!("Flatpak application '{}' is not installed", app_id));
        }
        std::fs::remove_dir_all(&app_dir)
            .map_err(|e| format!("Failed to remove sandbox flatpak dir: {}", e))?;
        invalidate_flatpak_cache();
        return Ok(format!("Successfully uninstalled Flatpak application '{}' (test mode)", app_id));
    }

    let output = Command::new("flatpak")
        .args(flatpak_transaction_args(app_id, "uninstall")?)
        .output()
        .map_err(|e| format!("Failed to invoke flatpak uninstall: {}", e))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        let detail = if !err.trim().is_empty() { err.trim() } else { out.trim() };
        return Err(format!("flatpak uninstall failed: {}", detail));
    }

    invalidate_flatpak_cache();
    Ok(format!("Successfully uninstalled Flatpak application '{}'", app_id))
}

/// Builds the exact CLI arguments used by both the compatibility commands and
/// the streamed transaction adapter. Keeping this here preserves the existing
/// scope/remote selection behaviour while letting the caller consume output in
/// real time.
pub fn flatpak_transaction_args(app_id: &str, operation: &str) -> Result<Vec<String>, String> {
    if !is_valid_flatpak_id(app_id) {
        return Err("Invalid Flatpak application ID. Select a verified Flathub source before continuing.".to_string());
    }
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return Err("Flatpak is not installed on this system.".to_string());
    }

    match operation {
        "install" | "reinstall" => {
            let scope = resolve_flathub_install_scope()
                .unwrap_or("--user");
            Ok(vec!["install".into(), scope.into(), "-y".into(), "flathub".into(), app_id.into()])
        }
        "uninstall" => {
            if let Some(scope) = detect_installed_flatpak_scope(app_id) {
                Ok(vec!["uninstall".into(), scope.into(), "-y".into(), app_id.into()])
            } else {
                Ok(vec!["uninstall".into(), "-y".into(), app_id.into()])
            }
        }
        _ => Err(format!("Unsupported Flatpak transaction operation '{}'.", operation)),
    }
}

/// Exposes cleanup metadata for Phase 26 storage management.
pub fn get_flatpak_cleanup_info(app_id: &str) -> FlatpakCleanupInfo {
    let home = crate::snapshot::get_home_dir();
    let user_data = home.join(".var/app").join(app_id);
    let user_data_path = if user_data.exists() {
        Some(user_data.display().to_string())
    } else {
        None
    };

    let cache_dir = user_data.join("cache");
    let cache_path = if cache_dir.exists() {
        Some(cache_dir.display().to_string())
    } else {
        None
    };

    let sys_app = PathBuf::from("/var/lib/flatpak/app").join(app_id);
    let system_app_path = if sys_app.exists() {
        Some(sys_app.display().to_string())
    } else {
        None
    };

    FlatpakCleanupInfo {
        app_id: app_id.to_string(),
        user_data_path,
        cache_path,
        system_app_path,
    }
}

/// Parses `flatpak remotes --columns=name` output. Newer Flatpak builds print
/// names with no header; older builds may still emit a `Name` column title.
fn parse_flatpak_remote_names(stdout: &str) -> Vec<String> {
    let mut remotes = Vec::new();
    for line in stdout.lines() {
        let name = match line.split_whitespace().next() {
            Some(value) if !value.is_empty() => value,
            _ => continue,
        };
        if name.eq_ignore_ascii_case("name") {
            continue;
        }
        if remotes.iter().any(|existing: &String| existing.eq_ignore_ascii_case(name)) {
            continue;
        }
        remotes.push(name.to_string());
    }
    remotes
}

fn list_flatpak_remote_names_for_scope(scope: &str) -> Vec<String> {
    Command::new("flatpak")
        .args(["remotes", scope, "--columns=name"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| parse_flatpak_remote_names(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

fn list_flatpak_remote_names() -> Vec<String> {
    let mut remotes = list_flatpak_remote_names_for_scope("--user");
    for name in list_flatpak_remote_names_for_scope("--system") {
        if !remotes.iter().any(|existing| existing.eq_ignore_ascii_case(&name)) {
            remotes.push(name);
        }
    }
    remotes
}

/// Prefer the user Flathub remote so install does not need root. Fall back to
/// the system remote when that is the only configured Flathub source.
pub(crate) fn resolve_flathub_install_scope() -> Option<&'static str> {
    let (has_user, has_system) = get_cached_flathub_scopes();
    if has_user {
        Some("--user")
    } else if has_system {
        Some("--system")
    } else {
        None
    }
}

/// Validates whether an ID matches reverse-DNS Flatpak application ID requirements
/// (contains at least 2 periods, valid characters, no spaces, e.g. org.kde.amarok).
pub fn is_valid_flatpak_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 3 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    })
}

/// Dynamically discovers an authentic Flathub match for a given package name or query.
/// Never guesses or fabricates Flatpak IDs.
pub fn find_flathub_match(query: &str, _display_name: Option<&str>) -> Option<FlatpakAppInfo> {
    let (has_flatpak, _) = crate::app_adapters::check_binary("flatpak");
    if !has_flatpak {
        return None;
    }

    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 1. If query is already a valid reverse-DNS Flatpak ID, verify it directly
    if is_valid_flatpak_id(trimmed) {
        if let Ok(Some(app)) = get_flatpak_app_info(trimmed) {
            return Some(app);
        }
    }

    // 2. Search Flathub using the package query
    if let Ok(results) = search_flathub_apps(trimmed) {
        let query_lower = trimmed.to_lowercase();
        let target_suffix = format!(".{}", query_lower);

        for app in results {
            // Must be a valid reverse-DNS Flatpak ID (contains >= 2 dots)
            if !is_valid_flatpak_id(&app.id) {
                continue;
            }

            let id_lower = app.id.to_lowercase();
            let name_lower = app.name.to_lowercase();

            // Authentic verification criteria:
            // a) Reverse-DNS ID ends with .<query> (e.g. org.kde.amarok, org.videolan.vlc, org.mozilla.firefox)
            let id_suffix_matches = id_lower.ends_with(&target_suffix);
            // b) Or reverse-DNS ID contains .<query>. (e.g. com.spotify.Client)
            let id_domain_matches = id_lower.contains(&format!(".{}.", query_lower))
                || id_lower.starts_with(&format!("{}.", query_lower));
            // c) Or exact app name match AND the query appears in the ID
            let exact_name_match = name_lower == query_lower && id_lower.contains(&query_lower);

            if id_suffix_matches || id_domain_matches || exact_name_match {
                return Some(app);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flatpak_remote_names_without_header() {
        let remotes = parse_flatpak_remote_names("flathub\tsystem\n");
        assert_eq!(remotes, vec!["flathub".to_string()]);
    }

    #[test]
    fn test_parse_flatpak_remote_names_skips_column_header() {
        let remotes = parse_flatpak_remote_names("Name\tOptions\nflathub\tsystem\nfedora\tsystem\n");
        assert_eq!(remotes, vec!["flathub".to_string(), "fedora".to_string()]);
    }

    #[test]
    fn test_is_valid_flatpak_id() {
        assert!(is_valid_flatpak_id("org.kde.amarok"));
        assert!(is_valid_flatpak_id("com.spotify.Client"));
        assert!(is_valid_flatpak_id("org.videolan.VLC"));
        assert!(is_valid_flatpak_id("org.blender.Blender"));
        assert!(!is_valid_flatpak_id("amarok"));
        assert!(!is_valid_flatpak_id("spotify"));
        assert!(!is_valid_flatpak_id("vlc"));
        assert!(!is_valid_flatpak_id("com."));
        assert!(!is_valid_flatpak_id(".."));
        assert!(!is_valid_flatpak_id("org..amarok"));
    }

    #[test]
    fn test_flatpak_transaction_args_explicit_scopes() {
        let args = flatpak_transaction_args("org.videolan.VLC", "install").expect("Must generate args");
        assert_eq!(args[0], "install");
        assert!(args[1] == "--user" || args[1] == "--system", "Scope must be explicit");
        assert_eq!(args[2], "-y");
        assert_eq!(args[3], "flathub");
        assert_eq!(args[4], "org.videolan.VLC");

        let uninst = flatpak_transaction_args("org.videolan.VLC", "uninstall").expect("Must generate uninst args");
        assert_eq!(uninst[0], "uninstall");
        assert!(uninst.contains(&"-y".to_string()));
        assert!(uninst.contains(&"org.videolan.VLC".to_string()));
    }

    #[test]
    fn test_scope_cache_invalidation() {
        {
            let mut cache = SCOPE_CACHE.lock().unwrap();
            *cache = Some((std::time::Instant::now(), true, true));
        }
        assert!(SCOPE_CACHE.lock().unwrap().is_some());
        invalidate_flatpak_cache();
        assert!(SCOPE_CACHE.lock().unwrap().is_none(), "Must invalidate scope cache");
    }

    #[test]
    fn test_ensure_flathub_remote_avoid_redundant_work() {
        // Pre-seed cache to simulate already configured remote
        {
            let mut cache = SCOPE_CACHE.lock().unwrap();
            *cache = Some((std::time::Instant::now(), true, true));
        }
        let mut called = false;
        let mut dummy_cb = |_st: &str, _p: Option<u8>, _m: &str, _d: bool, _dl: Option<&str>, _tot: Option<&str>, _spd: Option<&str>, _c: Option<usize>, _t: Option<usize>, _item: Option<&str>, _eta: Option<&str>, _diag: Option<&str>| {
            called = true;
        };
        let res = ensure_flathub_remote_for_scope("--user", Some(&mut dummy_cb));
        assert!(res.is_ok());
        drop(dummy_cb);
        assert!(!called, "Configured remote must not invoke setup or emit progress events");

        let err_res = ensure_flathub_remote_for_scope::<fn(&str, Option<u8>, &str, bool, Option<&str>, Option<&str>, Option<&str>, Option<usize>, Option<usize>, Option<&str>, Option<&str>, Option<&str>)>("--invalid", None);
        assert!(err_res.is_err(), "Must reject invalid scope");
    }
}
