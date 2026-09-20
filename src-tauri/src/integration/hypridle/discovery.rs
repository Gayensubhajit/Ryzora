use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{
    HypridleDiscoveryReport, HypridleGeneralBlock, HypridleGeneralConfig, HypridleListener,
    HypridleParseStatus,
};
use crate::integration::calculate_sha256;
use crate::integration::session_lock::is_hypridle_service_active;

pub const NATIVE_CONFIG_REL: &str = ".config/hypr/hypridle.conf";

/// Discovers the native Hypridle configuration for the given home directory.
/// This operation is strictly read-only and isolated from systemd/runtime state.
pub fn discover_native_hypridle_config(home: &Path) -> HypridleDiscoveryReport {
    discover_native_hypridle_config_internal(home, None)
}

/// Discovers native configuration and explicitly attaches an injected or queried service status.
pub fn discover_native_hypridle_config_with_service_state(
    home: &Path,
    service_active: bool,
) -> HypridleDiscoveryReport {
    discover_native_hypridle_config_internal(home, Some(service_active))
}

/// Standalone check for whether hypridle.service is active in user systemd.
pub fn get_hypridle_service_status() -> bool {
    is_hypridle_service_active()
}

fn discover_native_hypridle_config_internal(
    home: &Path,
    service_active: Option<bool>,
) -> HypridleDiscoveryReport {
    let config_path = home.join(NATIVE_CONFIG_REL);

    if !config_path.exists() {
        return HypridleDiscoveryReport {
            config_path,
            exists: false,
            readable: false,
            sha256: None,
            parse_status: HypridleParseStatus::Invalid,
            general: None,
            duplicate_general_blocks: Vec::new(),
            listeners: Vec::new(),
            duplicate_listener_timeouts: Vec::new(),
            duplicate_general_properties: Vec::new(),
            duplicate_listener_properties: Vec::new(),
            warnings: vec!["Native Hypridle configuration file does not exist".to_string()],
            service_active,
        };
    }

    let bytes = match fs::read(&config_path) {
        Ok(b) => b,
        Err(e) => {
            return HypridleDiscoveryReport {
                config_path,
                exists: true,
                readable: false,
                sha256: None,
                parse_status: HypridleParseStatus::Invalid,
                general: None,
                duplicate_general_blocks: Vec::new(),
                listeners: Vec::new(),
                duplicate_listener_timeouts: Vec::new(),
                duplicate_general_properties: Vec::new(),
                duplicate_listener_properties: Vec::new(),
                warnings: vec![format!("Failed to read native Hypridle configuration: {}", e)],
                service_active,
            };
        }
    };

    let sha256 = Some(calculate_sha256(&bytes));
    let content = String::from_utf8_lossy(&bytes);

    parse_hypridle_config_str(&content, config_path, sha256, service_active)
}

/// Parses the contents of a hypridle.conf string into structured discovery metadata.
pub fn parse_hypridle_config_str(
    content: &str,
    config_path: PathBuf,
    sha256: Option<String>,
    service_active: Option<bool>,
) -> HypridleDiscoveryReport {
    let mut general: Option<HypridleGeneralConfig> = None;
    let mut duplicate_general_blocks = Vec::new();
    let mut listeners = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_timeouts = HashSet::new();
    let mut duplicate_listener_timeouts = Vec::new();
    let mut duplicate_general_properties = Vec::new();
    let mut duplicate_listener_properties = Vec::new();
    let mut has_syntax_errors = false;

    let mut current_block: Option<(String, usize)> = None;
    let mut current_block_entries: Vec<(String, String)> = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == "}" {
            if let Some((block_name, start_line)) = current_block.take() {
                // Check for duplicate keys in this block
                let mut seen_keys = HashSet::new();
                let mut duplicate_keys_in_block = Vec::new();
                for (k, _) in &current_block_entries {
                    if !seen_keys.insert(k.clone()) && !duplicate_keys_in_block.contains(k) {
                        duplicate_keys_in_block.push(k.clone());
                    }
                }

                match block_name.as_str() {
                    "general" => {
                        let mut lock_cmd = None;
                        let mut before_sleep_cmd = None;
                        let mut after_sleep_cmd = None;
                        let mut extra_properties = HashMap::new();

                        for (k, v) in &current_block_entries {
                            match k.as_str() {
                                "lock_cmd" => {
                                    if lock_cmd.is_none() {
                                        lock_cmd = Some(v.clone());
                                    }
                                }
                                "before_sleep_cmd" => {
                                    if before_sleep_cmd.is_none() {
                                        before_sleep_cmd = Some(v.clone());
                                    }
                                }
                                "after_sleep_cmd" => {
                                    if after_sleep_cmd.is_none() {
                                        after_sleep_cmd = Some(v.clone());
                                    }
                                }
                                _ => {
                                    extra_properties.entry(k.clone()).or_insert_with(|| v.clone());
                                }
                            }
                        }

                        if !duplicate_keys_in_block.is_empty() {
                            for k in &duplicate_keys_in_block {
                                if !duplicate_general_properties.contains(k) {
                                    duplicate_general_properties.push(k.clone());
                                }
                                warnings.push(format!(
                                    "Duplicate property '{}' detected in 'general' block at line {}",
                                    k, start_line
                                ));
                            }
                        }

                        if general.is_none() {
                            general = Some(HypridleGeneralConfig {
                                lock_cmd,
                                before_sleep_cmd,
                                after_sleep_cmd,
                                extra_properties,
                                raw_entries: current_block_entries.clone(),
                                duplicate_keys: duplicate_keys_in_block,
                            });
                        } else {
                            warnings.push(format!(
                                "Duplicate 'general' block detected at line {}; preserved in duplicate_general_blocks",
                                start_line
                            ));
                            duplicate_general_blocks.push(HypridleGeneralBlock {
                                start_line,
                                lock_cmd,
                                before_sleep_cmd,
                                after_sleep_cmd,
                                extra_properties,
                                raw_entries: current_block_entries.clone(),
                                duplicate_keys: duplicate_keys_in_block,
                            });
                        }
                    }
                    "listener" => {
                        let mut timeout_str = None;
                        let mut on_timeout = None;
                        let mut on_resume = None;
                        let mut extra_properties = HashMap::new();

                        for (k, v) in &current_block_entries {
                            match k.as_str() {
                                "timeout" => {
                                    if timeout_str.is_none() {
                                        timeout_str = Some(v.clone());
                                    }
                                }
                                "on-timeout" | "on_timeout" => {
                                    if on_timeout.is_none() {
                                        on_timeout = Some(v.clone());
                                    }
                                }
                                "on-resume" | "on_resume" => {
                                    if on_resume.is_none() {
                                        on_resume = Some(v.clone());
                                    }
                                }
                                _ => {
                                    extra_properties.entry(k.clone()).or_insert_with(|| v.clone());
                                }
                            }
                        }

                        if !duplicate_keys_in_block.is_empty() {
                            for k in &duplicate_keys_in_block {
                                if !duplicate_listener_properties.contains(k) {
                                    duplicate_listener_properties.push(k.clone());
                                }
                                warnings.push(format!(
                                    "Duplicate property '{}' detected in 'listener' block at line {}",
                                    k, start_line
                                ));
                            }
                        }

                        match (timeout_str, on_timeout) {
                            (Some(t_str), Some(on_t)) => {
                                match t_str.parse::<u64>() {
                                    Ok(timeout) => {
                                        if !seen_timeouts.insert(timeout) {
                                            if !duplicate_listener_timeouts.contains(&timeout) {
                                                duplicate_listener_timeouts.push(timeout);
                                            }
                                            warnings.push(format!(
                                                "Duplicate listener timeout detected: {}s (at line {})",
                                                timeout, start_line
                                            ));
                                        }
                                        listeners.push(HypridleListener {
                                            start_line,
                                            timeout,
                                            on_timeout: on_t,
                                            on_resume,
                                            extra_properties,
                                            raw_entries: current_block_entries.clone(),
                                            duplicate_keys: duplicate_keys_in_block,
                                        });
                                    }
                                    Err(_) => {
                                        has_syntax_errors = true;
                                        warnings.push(format!(
                                            "Malformed listener at line {}: invalid timeout value '{}'",
                                            start_line, t_str
                                        ));
                                    }
                                }
                            }
                            (None, _) => {
                                has_syntax_errors = true;
                                warnings.push(format!(
                                    "Malformed listener at line {}: missing 'timeout' property",
                                    start_line
                                ));
                            }
                            (_, None) => {
                                has_syntax_errors = true;
                                warnings.push(format!(
                                    "Malformed listener at line {}: missing 'on-timeout' property",
                                    start_line
                                ));
                            }
                        }
                    }
                    other => {
                        warnings.push(format!(
                            "Unknown block '{}' at line {} preserved but unhandled",
                            other, start_line
                        ));
                    }
                }
                current_block_entries.clear();
            } else {
                has_syntax_errors = true;
                warnings.push(format!("Unexpected closing brace '}}' at line {}", line_num));
            }
            continue;
        }

        if trimmed.ends_with('{') {
            let block_name = trimmed.trim_end_matches('{').trim().to_string();
            if current_block.is_some() {
                has_syntax_errors = true;
                warnings.push(format!(
                    "Nested block '{}' at line {} is not supported in hypridle",
                    block_name, line_num
                ));
            } else {
                current_block = Some((block_name, line_num));
                current_block_entries.clear();
            }
            continue;
        }

        if current_block.is_some() {
            if let Some((key, value)) = trimmed.split_once('=') {
                let k = key.trim().to_string();
                let v = value.trim().to_string();
                current_block_entries.push((k, v));
            } else {
                has_syntax_errors = true;
                warnings.push(format!(
                    "Malformed directive at line {}: missing '='",
                    line_num
                ));
            }
        } else {
            if !trimmed.starts_with("source") {
                warnings.push(format!(
                    "Top-level directive outside of any block at line {}: '{}'",
                    line_num, trimmed
                ));
            }
        }
    }

    if let Some((unclosed_block, start_line)) = current_block {
        has_syntax_errors = true;
        warnings.push(format!(
            "Unclosed block '{}' starting at line {}",
            unclosed_block, start_line
        ));
    }

    // Invalid if syntax errors, duplicate general blocks, or duplicate keys within a block
    let parse_status = if has_syntax_errors
        || !duplicate_general_blocks.is_empty()
        || !duplicate_general_properties.is_empty()
        || !duplicate_listener_properties.is_empty()
    {
        HypridleParseStatus::Invalid
    } else if !warnings.is_empty() || !duplicate_listener_timeouts.is_empty() {
        HypridleParseStatus::ValidWithWarnings
    } else {
        HypridleParseStatus::Valid
    };

    HypridleDiscoveryReport {
        config_path,
        exists: true,
        readable: true,
        sha256,
        parse_status,
        general,
        duplicate_general_blocks,
        listeners,
        duplicate_listener_timeouts,
        duplicate_general_properties,
        duplicate_listener_properties,
        warnings,
        service_active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immutability_native_config_not_modified() {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push(format!("ryzora_hypridle_immut_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let conf_dir = temp_dir.join(".config/hypr");
        fs::create_dir_all(&conf_dir).unwrap();

        let conf_file = conf_dir.join("hypridle.conf");
        let sample_content = "# Header
general {
    lock_cmd = hyprlock
}
listener {
    timeout = 300
    on-timeout = lock
}
";
        fs::write(&conf_file, sample_content).unwrap();

        let hash_before = calculate_sha256(sample_content.as_bytes());

        let report = discover_native_hypridle_config(&temp_dir);
        assert!(report.exists);
        assert_eq!(report.parse_status, HypridleParseStatus::Valid);

        let content_after = fs::read_to_string(&conf_file).unwrap();
        let hash_after = calculate_sha256(content_after.as_bytes());

        assert_eq!(sample_content, content_after);
        assert_eq!(hash_before, hash_after);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_isolated_from_systemd() {
        let mut temp_dir = std::env::temp_dir();
        temp_dir.push(format!("ryzora_hypridle_iso_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let conf_dir = temp_dir.join(".config/hypr");
        fs::create_dir_all(&conf_dir).unwrap();
        fs::write(conf_dir.join("hypridle.conf"), "general { lock_cmd = test }
").unwrap();

        let report = discover_native_hypridle_config(&temp_dir);
        assert!(report.service_active.is_none(), "Pure discovery must be decoupled from systemd state");

        let report_injected = discover_native_hypridle_config_with_service_state(&temp_dir, true);
        assert_eq!(report_injected.service_active, Some(true));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_clean_valid_config() {
        let conf = r#"
        general {
            lock_cmd = /usr/bin/hyprlock
            before_sleep_cmd = loginctl lock-session
            after_sleep_cmd = hyprctl dispatch dpms on
        }

        listener {
            timeout = 300
            on-timeout = loginctl lock-session
            on-resume = notify-send 'welcome back'
        }

        listener {
            timeout = 600
            on_timeout = systemctl suspend
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Valid);
        assert!(report.warnings.is_empty());
        assert!(report.duplicate_general_blocks.is_empty());
        assert!(report.duplicate_listener_timeouts.is_empty());
        assert!(report.duplicate_general_properties.is_empty());
        assert!(report.duplicate_listener_properties.is_empty());

        let gen = report.general.unwrap();
        assert_eq!(gen.lock_cmd.as_deref(), Some("/usr/bin/hyprlock"));
        assert_eq!(gen.before_sleep_cmd.as_deref(), Some("loginctl lock-session"));
        assert_eq!(gen.after_sleep_cmd.as_deref(), Some("hyprctl dispatch dpms on"));
        assert_eq!(gen.raw_entries.len(), 3);

        assert_eq!(report.listeners.len(), 2);
        assert_eq!(report.listeners[0].timeout, 300);
        assert_eq!(report.listeners[0].on_timeout, "loginctl lock-session");
        assert_eq!(report.listeners[0].on_resume.as_deref(), Some("notify-send 'welcome back'"));
        assert_eq!(report.listeners[0].raw_entries.len(), 3);

        assert_eq!(report.listeners[1].timeout, 600);
        assert_eq!(report.listeners[1].on_timeout, "systemctl suspend");
        assert_eq!(report.listeners[1].on_resume, None);
    }

    #[test]
    fn test_parse_duplicate_general_blocks_preserves_both() {
        let conf = r#"
        general {
            lock_cmd = hyprlock1
        }
        general {
            lock_cmd = hyprlock2
            custom_extra = custom_val
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert_eq!(report.duplicate_general_blocks.len(), 1);

        // First general block preserved
        let primary = report.general.unwrap();
        assert_eq!(primary.lock_cmd.as_deref(), Some("hyprlock1"));

        // Second duplicate block preserved with full contents and line number
        let dup = &report.duplicate_general_blocks[0];
        assert_eq!(dup.lock_cmd.as_deref(), Some("hyprlock2"));
        assert_eq!(dup.extra_properties.get("custom_extra").map(|s| s.as_str()), Some("custom_val"));
        assert_eq!(dup.start_line, 5);
        assert_eq!(dup.raw_entries.len(), 2);
    }

    #[test]
    fn test_parse_duplicate_general_property_marks_invalid() {
        let conf = r#"
        general {
            lock_cmd = hyprlock1
            lock_cmd = hyprlock2
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert!(report.duplicate_general_properties.contains(&"lock_cmd".to_string()));

        let gen = report.general.unwrap();
        // Both values preserved in raw_entries
        assert_eq!(gen.raw_entries.len(), 2);
        assert_eq!(gen.raw_entries[0], ("lock_cmd".to_string(), "hyprlock1".to_string()));
        assert_eq!(gen.raw_entries[1], ("lock_cmd".to_string(), "hyprlock2".to_string()));
        assert!(gen.duplicate_keys.contains(&"lock_cmd".to_string()));
    }

    #[test]
    fn test_parse_duplicate_listener_property_marks_invalid() {
        let conf = r#"
        listener {
            timeout = 100
            timeout = 200
            on-timeout = lock
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert!(report.duplicate_listener_properties.contains(&"timeout".to_string()));

        let listener = &report.listeners[0];
        assert_eq!(listener.raw_entries.len(), 3);
        assert!(listener.duplicate_keys.contains(&"timeout".to_string()));
    }

    #[test]
    fn test_parse_duplicate_listener_timeouts_preserves_both() {
        let conf = r#"
        listener {
            timeout = 300
            on-timeout = loginctl lock-session
        }
        listener {
            timeout = 300
            on-timeout = /usr/bin/custom-lock
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::ValidWithWarnings);
        assert_eq!(report.listeners.len(), 2, "Both listeners must be preserved for inspection");
        assert_eq!(report.duplicate_listener_timeouts, vec![300]);
        assert_eq!(report.listeners[0].on_timeout, "loginctl lock-session");
        assert_eq!(report.listeners[1].on_timeout, "/usr/bin/custom-lock");
    }

    #[test]
    fn test_parse_valid_with_unknown_properties_and_blocks() {
        let conf = r#"
        source = ~/.config/hypr/extra.conf

        general {
            lock_cmd = hyprlock
            custom_extra_key = custom_value
        }

        custom_block {
            foo = bar
        }

        listener {
            timeout = 100
            on-timeout = echo 1
            listener_extra = hello
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::ValidWithWarnings);

        let gen = report.general.unwrap();
        assert_eq!(gen.extra_properties.get("custom_extra_key").map(|s| s.as_str()), Some("custom_value"));

        let l = &report.listeners[0];
        assert_eq!(l.extra_properties.get("listener_extra").map(|s| s.as_str()), Some("hello"));

        assert!(report.warnings.iter().any(|w| w.contains("Unknown block 'custom_block'")));
    }

    #[test]
    fn test_parse_malformed_syntax_marks_invalid() {
        let conf = r#"
        general {
            lock_cmd
        }

        listener {
            timeout = not_a_number
            on-timeout = lock
        }

        listener {
            on-timeout = no_timeout_specified
        }

        listener {
            timeout = 500
        }
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert!(report.warnings.iter().any(|w| w.contains("missing '='")));
        assert!(report.warnings.iter().any(|w| w.contains("invalid timeout value 'not_a_number'")));
        assert!(report.warnings.iter().any(|w| w.contains("missing 'timeout' property")));
        assert!(report.warnings.iter().any(|w| w.contains("missing 'on-timeout' property")));
    }

    #[test]
    fn test_parse_unclosed_block_marks_invalid() {
        let conf = r#"
        general {
            lock_cmd = hyprlock
        "#;

        let report = parse_hypridle_config_str(conf, PathBuf::from("/test/hypridle.conf"), None, None);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert!(report.warnings.iter().any(|w| w.contains("Unclosed block 'general'")));
    }

    #[test]
    fn test_parse_missing_file() {
        let dummy_home = PathBuf::from("/nonexistent/directory/path");
        let report = discover_native_hypridle_config(&dummy_home);
        assert!(!report.exists);
        assert!(!report.readable);
        assert_eq!(report.parse_status, HypridleParseStatus::Invalid);
        assert!(report.general.is_none());
        assert!(report.listeners.is_empty());
        assert!(!report.warnings.is_empty());
    }

    #[test]
    fn test_parse_empty_config() {
        let report = parse_hypridle_config_str("", PathBuf::from("/test/hypridle.conf"), None, None);
        assert!(report.exists);
        assert!(report.readable);
        assert_eq!(report.parse_status, HypridleParseStatus::Valid);
        assert!(report.general.is_none());
        assert!(report.listeners.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    #[ignore = "Live host acceptance test: expects specific host Dusky baseline"]
    fn test_discover_native_config_live() {
        let home_str = std::env::var("HOME").unwrap();
        let home = PathBuf::from(&home_str);

        let report = discover_native_hypridle_config(&home);
        assert!(report.exists, "Native config must exist on test host");
        assert!(report.readable, "Native config must be readable");
        assert_eq!(report.parse_status, HypridleParseStatus::Valid);
        assert_eq!(
            report.sha256.as_deref(),
            Some("9d117906fb409c9f707a7d1408dbf8c6006e0c804b83194e9ce9b041a1b8c402")
        );

        let gen = report.general.expect("General block must be present in Dusky config");
        assert_eq!(gen.lock_cmd.as_deref(), Some("pidof hyprlock || hyprlock"));
        assert_eq!(gen.before_sleep_cmd.as_deref(), Some("loginctl lock-session"));
        assert_eq!(gen.after_sleep_cmd.as_deref(), Some("hyprctl dispatch dpms on"));

        assert_eq!(report.listeners.len(), 5);
        assert_eq!(report.listeners[0].timeout, 140);
        assert_eq!(report.listeners[1].timeout, 150);
        assert_eq!(report.listeners[2].timeout, 300);
        assert_eq!(report.listeners[3].timeout, 330);
        assert_eq!(report.listeners[4].timeout, 600);

        assert!(report.warnings.is_empty());
    }
}
