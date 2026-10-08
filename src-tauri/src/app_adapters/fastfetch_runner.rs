//! Fastfetch Subprocess Adapter for Ryzora.
//!
//! Provides process execution for Fastfetch terminal preview and telemetry probe
//! strictly isolated within the app_adapters boundary.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn probe_fastfetch_raw_json() -> Option<String> {
    if env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return None;
    }

    if let Ok(output) = Command::new("fastfetch")
        .args(&["--pipe", "false", "--format", "json"])
        .output()
    {
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
    }
    None
}

pub fn launch_fastfetch_terminal(temp_config_path: &Path) -> Result<bool, String> {
    if env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok(true);
    }

    let temp_str = temp_config_path.to_string_lossy().to_string();
    let run_cmd = format!("fastfetch -c \"{}\"; rm -f \"{}\"", temp_str, temp_str);

    let spawned = if crate::system::check_binary("kitty").0 {
        Command::new("kitty")
            .args(&["--hold", "-e", "sh", "-c", &run_cmd])
            .spawn()
    } else if crate::system::check_binary("foot").0 {
        Command::new("foot")
            .args(&["-H", "sh", "-c", &run_cmd])
            .spawn()
    } else if crate::system::check_binary("konsole").0 {
        Command::new("konsole")
            .args(&["--noclose", "-e", "sh", "-c", &run_cmd])
            .spawn()
    } else if crate::system::check_binary("alacritty").0 {
        Command::new("alacritty")
            .args(&["-e", "sh", "-c", &run_cmd])
            .spawn()
    } else if crate::system::check_binary("xterm").0 {
        Command::new("xterm")
            .args(&["-hold", "-e", "sh", "-c", &run_cmd])
            .spawn()
    } else {
        return Err("No supported terminal emulator (kitty, foot, konsole, alacritty, xterm) found.".to_string());
    };

    match spawned {
        Ok(_) => Ok(true),
        Err(e) => {
            let _ = fs::remove_file(temp_config_path);
            Err(format!("Failed to launch terminal preview: {}", e))
        }
    }
}

pub fn run_fastfetch_preview(config_jsonc: &str) -> Result<String, String> {
    if env::var("RYZORA_SYSTEM_ROOT").is_ok() {
        return Ok("silentbyte@slayer\n-----------------\nOS: Garuda Linux x86_64\nHost: ASUS TUF Gaming F15\nKernel: Linux 7.2.9-1-garuda\nUptime: 2 hours\nShell: zsh\nWM: Hyprland".to_string());
    }

    let temp_dir = env::temp_dir();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_path = temp_dir.join(format!("ryzora_ff_render_{}.jsonc", timestamp));

    fs::write(&temp_path, config_jsonc)
        .map_err(|e| format!("Failed to write temporary preview config: {}", e))?;

    let output_res = Command::new("fastfetch")
        .args(&["-c", &temp_path.to_string_lossy(), "--pipe", "false"])
        .output();

    let _ = fs::remove_file(&temp_path);

    match output_res {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            Ok(stdout)
        }
        Err(e) => Err(format!("Failed to execute fastfetch preview: {}", e)),
    }
}

