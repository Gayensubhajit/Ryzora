//! Ryzora Application Adapters — Phase 23
//!
//! Orchestrates native and universal package managers across Linux distributions.
//! Phase 23A implements the Pacman adapter for Arch Linux.

pub mod pacman;
pub mod transaction;
pub mod catalog;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;
use crate::package_helper::PACKAGE_HELPER_SYSTEM_PATH;
use crate::app_adapters::transaction::{parse_pacman_line, split_output_lines, TransactionParserState, TransactionProgressEvent};

static ACTIVE_TRANSACTION: AtomicBool = AtomicBool::new(false);

struct TransactionGuard;
impl Drop for TransactionGuard {
    fn drop(&mut self) {
        ACTIVE_TRANSACTION.store(false, Ordering::SeqCst);
    }
}

/// Information about the host application ecosystem and available package managers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostAppEcosystem {
    pub distro_id: String,
    pub distro_name: String,
    pub distro_family: String,
    pub available_managers: Vec<String>,
    pub primary_manager: String,
    pub has_pacman: bool,
    pub has_aur_helper: bool,
    pub aur_helper: Option<String>,
    pub has_flatpak: bool,
}

/// Detects available package managers on the host system using zero-subprocess file checks.
pub fn detect_host_app_ecosystem_in(home: &Path) -> HostAppEcosystem {
    let host_caps = crate::host::detect_host_capabilities_in(home);

    let distro_id = host_caps.distro_id.to_lowercase();
    let distro_family = if distro_id == "arch"
        || distro_id == "garuda"
        || distro_id == "endeavouros"
        || distro_id == "manjaro"
    {
        "arch".to_string()
    } else if distro_id == "debian"
        || distro_id == "ubuntu"
        || distro_id == "pop"
        || distro_id == "linuxmint"
    {
        "debian".to_string()
    } else if distro_id == "fedora" || distro_id == "rhel" || distro_id == "nobara" {
        "fedora".to_string()
    } else if distro_id == "opensuse" || distro_id.contains("suse") {
        "opensuse".to_string()
    } else if distro_id == "nixos" {
        "nixos".to_string()
    } else {
        "generic".to_string()
    };

    let mut available = Vec::new();

    let (has_pacman, _) = crate::system::check_binary("pacman");
    if has_pacman {
        available.push("pacman".to_string());
    }

    let (has_paru, _) = crate::system::check_binary("paru");
    let (has_yay, _) = crate::system::check_binary("yay");
    let aur_helper = if has_paru {
        available.push("paru".to_string());
        Some("paru".to_string())
    } else if has_yay {
        available.push("yay".to_string());
        Some("yay".to_string())
    } else {
        None
    };

    let (has_flatpak, _) = crate::system::check_binary("flatpak");
    if has_flatpak {
        available.push("flatpak".to_string());
    }

    let (has_apt, _) = crate::system::check_binary("apt");
    if has_apt {
        available.push("apt".to_string());
    }

    let (has_dnf, _) = crate::system::check_binary("dnf");
    if has_dnf {
        available.push("dnf".to_string());
    }

    let (has_zypper, _) = crate::system::check_binary("zypper");
    if has_zypper {
        available.push("zypper".to_string());
    }

    let primary = if has_pacman {
        "pacman".to_string()
    } else if has_flatpak {
        "flatpak".to_string()
    } else if has_apt {
        "apt".to_string()
    } else if has_dnf {
        "dnf".to_string()
    } else {
        "none".to_string()
    };

    HostAppEcosystem {
        distro_id: host_caps.distro_id,
        distro_name: host_caps.distro_name,
        distro_family,
        available_managers: available,
        primary_manager: primary,
        has_pacman,
        has_aur_helper: aur_helper.is_some(),
        aur_helper,
        has_flatpak,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_host_app_ecosystem() -> Result<HostAppEcosystem, String> {
    let home = crate::snapshot::get_home_dir();
    Ok(detect_host_app_ecosystem_in(&home))
}

#[tauri::command]
pub fn pacman_search_packages(query: String, limit: Option<usize>) -> Result<Vec<pacman::PacmanPackageInfo>, String> {
    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);
    pacman::search_pacman_packages_in(&query, &sync_dir, &local_dir, limit.unwrap_or(50))
}

#[tauri::command]
pub fn pacman_get_package_details(package_name: String) -> Result<Option<pacman::PacmanPackageInfo>, String> {
    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);
    pacman::get_pacman_package_details_in(&package_name, &sync_dir, &local_dir)
}

#[tauri::command]
pub fn pacman_list_installed_packages() -> Result<Vec<pacman::PacmanPackageInfo>, String> {
    let local_dir = pacman::resolve_local_dir(None);
    pacman::list_installed_pacman_packages_in(&local_dir)
}

fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.starts_with('.')
        && !name.starts_with('+')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '+' || c == '@')
}

#[tauri::command]
pub fn pacman_install_package(package_name: String) -> Result<serde_json::Value, String> {
    if !is_valid_package_name(&package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }

    let sync_dir = pacman::resolve_sync_dir(None);
    let local_dir = pacman::resolve_local_dir(None);

    let pkg_opt = pacman::get_pacman_package_details_in(&package_name, &sync_dir, &local_dir)?;

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate install into sandbox local db
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        let pkg = pkg_opt.ok_or_else(|| format!("Package '{}' not found in pacman repositories", package_name))?;

        let pkg_dir = local_dir.join(format!("{}-{}", pkg.name, pkg.version));
        fs::create_dir_all(&pkg_dir)
            .map_err(|e| format!("Failed to create mock package dir: {}", e))?;
        let desc_content = format!(
            "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\n{}\n\n%URL%\n{}\n\n%LICENSE%\n{}\n",
            pkg.name,
            pkg.version,
            pkg.description,
            pkg.url.as_deref().unwrap_or(""),
            pkg.license.as_deref().unwrap_or("")
        );
        fs::write(pkg_dir.join("desc"), desc_content)
            .map_err(|e| format!("Failed to write mock desc file: {}", e))?;

        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Installed '{}' successfully (test mode)", package_name),
            "package_name": package_name,
            "version": pkg.version
        }));
    }

    // Live mode: invoke pkexec pacman -S --noconfirm <package_name>
    let output = std::process::Command::new("pkexec")
        .arg("pacman")
        .arg("-S")
        .arg("--noconfirm")
        .arg(&package_name)
        .output()
        .map_err(|e| format!("Failed to invoke pkexec: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("pacman exited with status code {:?}", output.status.code())
        };
        return Err(format!("Installation failed: {}", detail));
    }

    // Determine installed version from local db
    let (_, installed_version) = pacman::check_installed_status_in(&package_name, &local_dir);

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Installed '{}' successfully", package_name),
        "package_name": package_name,
        "version": installed_version.unwrap_or_else(|| pkg_opt.map(|p| p.version).unwrap_or_else(|| "latest".to_string()))
    }))
}

#[tauri::command]
pub fn pacman_uninstall_package(package_name: String) -> Result<serde_json::Value, String> {
    if !is_valid_package_name(&package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }

    let local_dir = pacman::resolve_local_dir(None);
    let (is_installed, ver) = pacman::check_installed_status_in(&package_name, &local_dir);

    // In test environment or test mode (RYZORA_SYSTEM_ROOT set), simulate uninstall by removing mock package dir
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        if !is_installed {
            return Err(format!("Package '{}' is not installed", package_name));
        }
        if let Some(v) = ver {
            let pkg_dir = local_dir.join(format!("{}-{}", package_name, v));
            if pkg_dir.exists() {
                let _ = fs::remove_dir_all(&pkg_dir);
            }
        }
        return Ok(serde_json::json!({
            "success": true,
            "message": format!("Uninstalled '{}' successfully (test mode)", package_name),
            "package_name": package_name
        }));
    }

    if !is_installed {
        return Err(format!("Package '{}' is not installed", package_name));
    }

    // Live mode: invoke pkexec pacman -R --noconfirm <package_name>
    let output = std::process::Command::new("pkexec")
        .arg("pacman")
        .arg("-R")
        .arg("--noconfirm")
        .arg(&package_name)
        .output()
        .map_err(|e| format!("Failed to invoke pkexec: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("pacman exited with status code {:?}", output.status.code())
        };
        return Err(format!("Uninstallation failed: {}", detail));
    }

    Ok(serde_json::json!({
        "success": true,
        "message": format!("Uninstalled '{}' successfully", package_name),
        "package_name": package_name
    }))
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartTransactionResponse {
    pub success: bool,
    pub transaction_id: String,
    pub operation: String,
    pub package_name: String,
}

#[tauri::command]
pub fn pacman_start_transaction(
    app: tauri::AppHandle,
    package_name: String,
    operation: String,
) -> Result<StartTransactionResponse, String> {
    if !is_valid_package_name(&package_name) {
        return Err(format!("Invalid package name '{}'", package_name));
    }

    let op = operation.to_lowercase();
    if op != "install" && op != "uninstall" && op != "reinstall" {
        return Err(format!("Unsupported transaction operation '{}'. Must be install, uninstall, or reinstall", op));
    }

    // 1. Single active transaction enforcement
    if ACTIVE_TRANSACTION.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err("Another package transaction is currently running in Ryzora. Please wait until it finishes.".to_string());
    }

    let txn_id = format!("txn-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());

    // 2. Database lock pre-flight check
    let db_lock_path = if let Ok(sys_root) = std::env::var("RYZORA_SYSTEM_ROOT") {
        PathBuf::from(sys_root).join("var/lib/pacman/db.lck")
    } else {
        PathBuf::from("/var/lib/pacman/db.lck")
    };

    if db_lock_path.exists() {
        ACTIVE_TRANSACTION.store(false, Ordering::SeqCst);
        let lock_event = TransactionProgressEvent {
            transaction_id: txn_id.clone(),
            operation: op.clone(),
            target_package: package_name.clone(),
            stage: "failed".to_string(),
            percentage: None,
            download_percentage: None,
            install_percentage: None,
            current_package: None,
            current_package_index: None,
            total_packages: None,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Another package manager is currently using the database. Waiting for database to become available.".to_string(),
            raw_line: None,
            error: Some("Pacman database lock file exists (/var/lib/pacman/db.lck).".to_string()),
            is_db_locked: true,
            is_auth_cancelled: false,
            is_cached: false,
        };
        let _ = app.emit("ryzora:transaction_progress", &lock_event);
        return Err("Another package manager is currently using the database. Please wait until it finishes.".to_string());
    }

    // 3. Privileged helper check
    let helper_path = PathBuf::from(PACKAGE_HELPER_SYSTEM_PATH);
    let helper_installed = helper_path.exists();
    let dev_fallback = std::env::var("RYZORA_DEV_PACMAN_FALLBACK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if !helper_installed && !dev_fallback && std::env::var("RYZORA_SYSTEM_ROOT").is_err() {
        ACTIVE_TRANSACTION.store(false, Ordering::SeqCst);
        let fail_event = TransactionProgressEvent {
            transaction_id: txn_id.clone(),
            operation: op.clone(),
            target_package: package_name.clone(),
            stage: "failed".to_string(),
            percentage: None,
            download_percentage: None,
            install_percentage: None,
            current_package: Some(package_name.clone()),
            current_package_index: None,
            total_packages: None,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Ryzora's package service is not installed.".to_string(),
            raw_line: None,
            error: Some("Ryzora's package service is not installed.".to_string()),
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: false,
        };
        let _ = app.emit("ryzora:transaction_progress", &fail_event);
        return Err("Ryzora's package service is not installed.".to_string());
    }

    // 4. Initial preparing event
    let init_event = TransactionProgressEvent {
        transaction_id: txn_id.clone(),
        operation: op.clone(),
        target_package: package_name.clone(),
        stage: "preparing".to_string(),
        percentage: Some(2),
        download_percentage: None,
        install_percentage: None,
        current_package: Some(package_name.clone()),
        current_package_index: None,
        total_packages: None,
        bytes_downloaded_str: None,
        bytes_total_str: None,
        download_speed: None,
        message: format!("Preparing to {} {}...", op, package_name),
        raw_line: None,
        error: None,
        is_db_locked: false,
        is_auth_cancelled: false,
            is_cached: false,
    };
    let _ = app.emit("ryzora:transaction_progress", &init_event);

    let app_worker = app.clone();
    let txn_id_worker = txn_id.clone();
    let op_worker = op.clone();
    let pkg_name_worker = package_name.clone();

    // 5. Spawn background transaction worker
    std::thread::spawn(move || {
        let _guard = TransactionGuard;
        run_transaction_worker(app_worker, txn_id_worker, op_worker, pkg_name_worker, helper_installed);
    });

    Ok(StartTransactionResponse {
        success: true,
        transaction_id: txn_id,
        operation: op,
        package_name,
    })
}

#[tauri::command]
pub fn pacman_stream_transaction(
    app: tauri::AppHandle,
    package_name: String,
    operation: String,
) -> Result<serde_json::Value, String> {
    let res = pacman_start_transaction(app, package_name, operation)?;
    Ok(serde_json::json!({
        "success": res.success,
        "transaction_id": res.transaction_id,
        "operation": res.operation,
        "package_name": res.package_name,
    }))
}

fn run_transaction_worker(
    app: tauri::AppHandle,
    txn_id: String,
    op: String,
    package_name: String,
    helper_installed: bool,
) {
    let mut state = TransactionParserState::new(txn_id.clone(), op.clone(), package_name.clone());

    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        let resolving_event = TransactionProgressEvent {
            transaction_id: txn_id.clone(),
            operation: op.clone(),
            target_package: package_name.clone(),
            stage: "resolving".to_string(),
            percentage: state.compute_overall_percentage("resolving"),
            download_percentage: None,
            install_percentage: None,
            current_package: Some(package_name.clone()),
            current_package_index: None,
            total_packages: Some(1),
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Resolving dependencies...".to_string(),
            raw_line: Some("resolving dependencies...".to_string()),
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: false,
        };
        let _ = app.emit("ryzora:transaction_progress", &resolving_event);

        if op == "install" || op == "reinstall" {
            let sync_dir = pacman::resolve_sync_dir(None);
            let local_dir = pacman::resolve_local_dir(None);
            if let Ok(Some(pkg)) = pacman::get_pacman_package_details_in(&package_name, &sync_dir, &local_dir) {
                let pkg_dir = local_dir.join(format!("{}-{}", pkg.name, pkg.version));
                let _ = fs::create_dir_all(&pkg_dir);
                let desc_content = format!(
                    "%NAME%\n{}\n\n%VERSION%\n{}\n\n%DESC%\n{}\n\n%URL%\n{}\n\n%LICENSE%\n{}\n",
                    pkg.name,
                    pkg.version,
                    pkg.description,
                    pkg.url.as_deref().unwrap_or(""),
                    pkg.license.as_deref().unwrap_or("")
                );
                let _ = fs::write(pkg_dir.join("desc"), desc_content);
            }
        }

        let comp_event = TransactionProgressEvent {
            transaction_id: txn_id.clone(),
            operation: op.clone(),
            target_package: package_name.clone(),
            stage: "completed".to_string(),
            percentage: Some(100),
            download_percentage: Some(100),
            install_percentage: Some(100),
            current_package: Some(package_name.clone()),
            current_package_index: Some(1),
            total_packages: Some(1),
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: format!("Completed {} for {} (test mode)", op, package_name),
            raw_line: None,
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: false,
        };
        let _ = app.emit("ryzora:transaction_progress", &comp_event);
        return;
    }

    let mut cmd = if helper_installed {
        let mut c = std::process::Command::new("pkexec");
        c.arg(PACKAGE_HELPER_SYSTEM_PATH);
        c.arg(&op);
        c.arg(&package_name);
        c
    } else {
        let mut c = std::process::Command::new("pkexec");
        c.arg("pacman");
        match op.as_str() {
            "install" | "reinstall" => {
                c.arg("-S").arg("--noconfirm").arg(&package_name);
            }
            "uninstall" => {
                c.arg("-R").arg("--noconfirm").arg(&package_name);
            }
            _ => unreachable!(),
        }
        c
    };

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let fail_event = TransactionProgressEvent {
                transaction_id: txn_id.clone(),
                operation: op.clone(),
                target_package: package_name.clone(),
                stage: "failed".to_string(),
                percentage: None,
                download_percentage: None,
                install_percentage: None,
                current_package: Some(package_name.clone()),
                current_package_index: None,
                total_packages: None,
                bytes_downloaded_str: None,
                bytes_total_str: None,
                download_speed: None,
                message: format!("Failed to spawn package manager process: {}", e),
                raw_line: None,
                error: Some(e.to_string()),
                is_db_locked: false,
                is_auth_cancelled: false,
            is_cached: false,
            };
            let _ = app.emit("ryzora:transaction_progress", &fail_event);
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            return;
        }
    };

    let stderr = match child.stderr.take() {
        Some(s) => s,
        None => {
            let _ = child.kill();
            return;
        }
    };

    let app_stdout = app.clone();
    let mut state_out = state.clone();

    let stdout_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 2048];
        let mut reader = stdout;
        let mut throttler = crate::app_adapters::transaction::TransactionThrottler::new(75);
        let mut all_logs = Vec::new();

        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..n]);
            let lines = split_output_lines(&mut buffer);
            for line in lines {
                all_logs.push(line.clone());
                if let Some(event) = parse_pacman_line(&line, &mut state_out) {
                    if throttler.should_emit(&event) {
                        let _ = app_stdout.emit("ryzora:transaction_progress", &event);
                    }
                }
            }
        }
        all_logs
    });

    let app_stderr = app.clone();
    let mut state_err = state.clone();

    let stderr_handle = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 2048];
        let mut reader = stderr;
        let mut throttler = crate::app_adapters::transaction::TransactionThrottler::new(75);
        let mut all_errs = Vec::new();

        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..n]);
            let lines = split_output_lines(&mut buffer);
            for line in lines {
                all_errs.push(line.clone());
                if let Some(event) = parse_pacman_line(&line, &mut state_err) {
                    if throttler.should_emit(&event) {
                        let _ = app_stderr.emit("ryzora:transaction_progress", &event);
                    }
                }
            }
        }
        all_errs
    });

    let _stdout_lines = stdout_handle.join().unwrap_or_default();
    let stderr_lines = stderr_handle.join().unwrap_or_default();

    let status = child.wait();

    let local_dir = pacman::resolve_local_dir(None);
    match status {
        Ok(exit_status) if exit_status.success() => {
            let (_, _installed_version) = pacman::check_installed_status_in(&package_name, &local_dir);
            let comp_event = TransactionProgressEvent {
                transaction_id: txn_id.clone(),
                operation: op.clone(),
                target_package: package_name.clone(),
                stage: "completed".to_string(),
                percentage: Some(100),
                download_percentage: Some(100),
                install_percentage: Some(100),
                current_package: Some(package_name.clone()),
                current_package_index: state.total_packages,
                total_packages: state.total_packages,
                bytes_downloaded_str: None,
                bytes_total_str: None,
                download_speed: None,
                message: format!("Successfully completed {} for {}.", op, package_name),
                raw_line: None,
                error: None,
                is_db_locked: false,
                is_auth_cancelled: false,
            is_cached: false,
            };
            let _ = app.emit("ryzora:transaction_progress", &comp_event);
        }
        Ok(exit_status) => {
            let err_msg = stderr_lines.join("\n");
            let is_db_lock = err_msg.to_lowercase().contains("lock");
            let is_auth_cancel = err_msg.contains("Request dismissed") || err_msg.to_lowercase().contains("cancelled");

            let fail_event = TransactionProgressEvent {
                transaction_id: txn_id.clone(),
                operation: op.clone(),
                target_package: package_name.clone(),
                stage: "failed".to_string(),
                percentage: None,
                download_percentage: None,
                install_percentage: None,
                current_package: Some(package_name.clone()),
                current_package_index: None,
                total_packages: state.total_packages,
                bytes_downloaded_str: None,
                bytes_total_str: None,
                download_speed: None,
                message: if is_db_lock {
                    "Another package manager is currently using the database.".to_string()
                } else if is_auth_cancel {
                    "Administrator authorization was cancelled.".to_string()
                } else if !err_msg.trim().is_empty() {
                    err_msg.trim().to_string()
                } else {
                    format!("Transaction failed with exit code {:?}", exit_status.code())
                },
                raw_line: None,
                error: Some(err_msg),
                is_db_locked: is_db_lock,
                is_auth_cancelled: is_auth_cancel,
                is_cached: false,
            };
            let _ = app.emit("ryzora:transaction_progress", &fail_event);
        }
        Err(e) => {
            let fail_event = TransactionProgressEvent {
                transaction_id: txn_id.clone(),
                operation: op.clone(),
                target_package: package_name.clone(),
                stage: "failed".to_string(),
                percentage: None,
                download_percentage: None,
                install_percentage: None,
                current_package: Some(package_name.clone()),
                current_package_index: None,
                total_packages: state.total_packages,
                bytes_downloaded_str: None,
                bytes_total_str: None,
                download_speed: None,
                message: format!("Process wait error: {}", e),
                raw_line: None,
                error: Some(e.to_string()),
                is_db_locked: false,
                is_auth_cancelled: false,
            is_cached: false,
            };
            let _ = app.emit("ryzora:transaction_progress", &fail_event);
        }
    }
}

pub mod desktop_icons;

#[tauri::command]
pub fn resolve_desktop_app_icon(package_id: String) -> Option<desktop_icons::DesktopAppIconInfo> {
    desktop_icons::resolve_desktop_icon_for_app(&package_id)
}

/// Batch-resolve icons for multiple package IDs in a single IPC call.
/// Returns a map of package_id -> DesktopAppIconInfo for all IDs that resolved successfully.
/// IDs that fail resolution are omitted from the map (frontend treats absent keys as no icon).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum IconBatchQuery {
    IdOnly(String),
    WithHints {
        id: String,
        icon_name: Option<String>,
        icon_path: Option<String>,
    },
}

/// Batch-resolve icons for multiple packages with optional hints in a single IPC call.
#[tauri::command]
pub fn resolve_desktop_icon_batch(
    package_ids: Option<Vec<String>>,
    items: Option<Vec<IconBatchQuery>>,
) -> std::collections::HashMap<String, desktop_icons::DesktopAppIconInfo> {
    let mut result = std::collections::HashMap::new();

    if let Some(items_list) = items {
        for query in items_list {
            match query {
                IconBatchQuery::IdOnly(id) => {
                    if let Some(info) = desktop_icons::resolve_desktop_icon_for_app(&id) {
                        result.insert(id, info);
                    }
                }
                IconBatchQuery::WithHints { id, icon_name, icon_path } => {
                    if let Some(info) = desktop_icons::resolve_desktop_icon_with_hints(
                        &id,
                        icon_name.as_deref(),
                        icon_path.as_deref(),
                    ) {
                        result.insert(id, info);
                    }
                }
            }
        }
    } else if let Some(ids) = package_ids {
        for id in ids {
            if let Some(info) = desktop_icons::resolve_desktop_icon_for_app(&id) {
                result.insert(id, info);
            }
        }
    }

    result
}

#[tauri::command]
pub fn get_installed_package_files(package_name: String) -> Result<Vec<String>, String> {
    let local_dir = pacman::resolve_local_dir(None);
    if !local_dir.exists() {
        return Ok(Vec::new());
    }

    if let Ok(entries) = fs::read_dir(&local_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if name_str == package_name || name_str.starts_with(&format!("{}-", package_name)) {
                let files_path = entry.path().join("files");
                if files_path.exists() {
                    if let Ok(content) = fs::read_to_string(&files_path) {
                        let mut in_files = false;
                        let mut files = Vec::new();
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed == "%FILES%" {
                                in_files = true;
                                continue;
                            }
                            if in_files {
                                if trimmed.starts_with('%') {
                                    break;
                                }
                                if !trimmed.is_empty() {
                                    files.push(format!("/{}", trimmed));
                                }
                            }
                        }
                        return Ok(files);
                    }
                }
            }
        }
    }

    Ok(Vec::new())
}

#[tauri::command]
pub fn launch_desktop_app(package_id: String) -> Result<bool, String> {
    if std::env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok(true);
    }

    if let Some(info) = desktop_icons::resolve_desktop_icon_for_app(&package_id) {
        let desktop_file_name = std::path::Path::new(&info.desktop_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&info.desktop_file);

        // 1. Try launching with gtk-launch using desktop file name
        if let Ok(_child) = std::process::Command::new("gtk-launch")
            .arg(desktop_file_name)
            .spawn()
        {
            return Ok(true);
        }

        // 2. Parse exec command from desktop entry, stripping Freedesktop field codes (%u, %F, etc.)
        if let Some(exec_cmd) = &info.exec {
            let tokens: Vec<&str> = exec_cmd
                .split_whitespace()
                .filter(|tok| !tok.starts_with('%'))
                .collect();

            if let Some((binary, args)) = tokens.split_first() {
                if !binary.is_empty() {
                    if let Ok(_child) = std::process::Command::new(binary).args(args).spawn() {
                        return Ok(true);
                    }
                }
            }
        }
    }

    // 3. Fallback: try spawning binary by package_id directly
    if let Ok(_child) = std::process::Command::new(&package_id).spawn() {
        return Ok(true);
    }

    Err(format!(
        "Could not launch application '{}'. Desktop entry or executable was not found on system.",
        package_id
    ))
}


#[tauri::command]
pub fn pacman_get_catalog_status() -> Result<catalog::CatalogStatus, String> {
    catalog::ensure_catalog_initialized();
    let store = catalog::get_catalog_store();
    let r = store.read().map_err(|e| e.to_string())?;
    Ok(r.status.clone())
}

#[tauri::command]
pub fn pacman_get_catalog_items(filter: catalog::CatalogQueryFilter) -> Result<catalog::CatalogPageResponse, String> {
    catalog::ensure_catalog_initialized();
    Ok(catalog::query_catalog(filter))
}

#[tauri::command]
pub fn pacman_get_catalog_item_details(package_id: String) -> Result<Option<catalog::CatalogItem>, String> {
    catalog::ensure_catalog_initialized();
    Ok(catalog::get_catalog_item_by_id(&package_id))
}

#[tauri::command]
pub fn pacman_refresh_catalog() -> Result<catalog::CatalogStatus, String> {
    catalog::refresh_catalog_internal()
}

#[tauri::command]
pub fn pacman_refresh_catalog_installed_state() -> Result<catalog::CatalogStatus, String> {
    catalog::refresh_installed_state_internal()
}
