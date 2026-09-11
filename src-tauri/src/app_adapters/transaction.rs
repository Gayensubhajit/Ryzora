use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionProgressEvent {
    pub transaction_id: String,
    pub operation: String,
    pub target_package: String,
    pub stage: String,
    /// Weighted overall monotonic transaction percentage (0-100)
    pub percentage: Option<u8>,
    /// Raw download stage percentage (0-100)
    pub download_percentage: Option<u8>,
    /// Raw package installation step percentage (0-100)
    pub install_percentage: Option<u8>,
    pub current_package: Option<String>,
    pub current_package_index: Option<usize>,
    pub total_packages: Option<usize>,
    pub bytes_downloaded_str: Option<String>,
    pub bytes_total_str: Option<String>,
    pub download_speed: Option<String>,
    pub message: String,
    pub raw_line: Option<String>,
    pub error: Option<String>,
    pub is_db_locked: bool,
    pub is_auth_cancelled: bool,
    pub is_cached: bool,
}

#[derive(Debug, Clone)]
pub struct TransactionParserState {
    pub transaction_id: String,
    pub operation: String,
    pub target_package: String,
    pub current_stage: String,
    pub current_package: Option<String>,
    pub current_package_index: Option<usize>,
    pub total_packages: Option<usize>,
    pub bytes_total_str: Option<String>,
    pub download_pct: Option<u8>,
    pub install_pct: Option<u8>,
    pub max_overall_pct: u8,
    pub is_cached: bool,
}

impl TransactionParserState {
    pub fn new(transaction_id: String, operation: String, target_package: String) -> Self {
        Self {
            transaction_id,
            operation,
            target_package,
            current_stage: "preparing".to_string(),
            current_package: None,
            current_package_index: None,
            total_packages: None,
            bytes_total_str: None,
            download_pct: None,
            install_pct: None,
            max_overall_pct: 0,
            is_cached: false,
        }
    }

    /// Computes a monotonic weighted overall percentage:
    /// - preparing: 2%
    /// - resolving: 5%
    /// - downloading: 5% -> 60%
    /// - installing: 60% -> 95%
    /// - finalizing: 97%
    /// - completed: 100%
    pub fn compute_overall_percentage(&mut self, stage: &str) -> Option<u8> {
        let raw = match stage {
            "preparing" => 2,
            "resolving" => 5,
            "downloading" => {
                let dp = self.download_pct.unwrap_or(0) as f32;
                (5.0 + (dp * 0.55)).round() as u8
            }
            "installing" => {
                let ip = self.install_pct.unwrap_or(0) as f32;
                (60.0 + (ip * 0.35)).round() as u8
            }
            "finalizing" => 97,
            "completed" => 100,
            _ => self.max_overall_pct,
        };

        let monotonic = std::cmp::max(self.max_overall_pct, raw);
        self.max_overall_pct = monotonic;
        Some(monotonic)
    }
}

/// Throttles high-frequency progress events to keep UI at ~10-15 updates/sec (60-100ms)
/// while guaranteeing instant dispatch for state changes, package transitions, and errors.
#[derive(Debug)]
pub struct TransactionThrottler {
    last_emitted_at: Option<Instant>,
    last_stage: String,
    last_pkg_idx: Option<usize>,
    last_overall_pct: Option<u8>,
    min_interval_ms: u128,
}

impl TransactionThrottler {
    pub fn new(min_interval_ms: u128) -> Self {
        Self {
            last_emitted_at: None,
            last_stage: String::new(),
            last_pkg_idx: None,
            last_overall_pct: None,
            min_interval_ms,
        }
    }

    pub fn should_emit(&mut self, event: &TransactionProgressEvent) -> bool {
        // Immediate priority events: errors, locks, cancellations, completions
        if event.is_db_locked
            || event.is_auth_cancelled
            || event.error.is_some()
            || event.stage == "completed"
            || event.stage == "failed"
        {
            self.record_emission(event);
            return true;
        }

        // Immediate stage transition
        if event.stage != self.last_stage {
            self.record_emission(event);
            return true;
        }

        // Immediate package index transition
        if event.current_package_index != self.last_pkg_idx && event.current_package_index.is_some() {
            self.record_emission(event);
            return true;
        }

        // Throttled high-frequency progress updates
        let now = Instant::now();
        if let Some(last_time) = self.last_emitted_at {
            if now.duration_since(last_time).as_millis() >= self.min_interval_ms {
                if event.percentage != self.last_overall_pct {
                    self.record_emission(event);
                    return true;
                }
            }
            false
        } else {
            self.record_emission(event);
            true
        }
    }

    fn record_emission(&mut self, event: &TransactionProgressEvent) {
        self.last_emitted_at = Some(Instant::now());
        self.last_stage = event.stage.clone();
        self.last_pkg_idx = event.current_package_index;
        self.last_overall_pct = event.percentage;
    }
}

/// Splits stream bytes on BOTH `\n` and `\r` (pacman progress lines),
/// retaining incomplete trailing fragments in buffer.
pub fn split_output_lines(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    let mut start = 0;

    for i in 0..buffer.len() {
        let b = buffer[i];
        if b == b'\n' || b == b'\r' {
            if i > start {
                if let Ok(s) = std::str::from_utf8(&buffer[start..i]) {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        lines.push(trimmed.to_string());
                    }
                }
            }
            start = i + 1;
        }
    }

    if start < buffer.len() {
        buffer.drain(0..start);
    } else {
        buffer.clear();
    }

    lines
}

/// Parses genuine Pacman output lines into structured transaction events.
pub fn parse_pacman_line(line: &str, state: &mut TransactionParserState) -> Option<TransactionProgressEvent> {
    let line_clean = line.trim();
    if line_clean.is_empty() {
        return None;
    }

    let line_lower = line_clean.to_lowercase();

    // 1. Pacman DB Lock error detection
    if line_lower.contains("unable to lock database")
        || line_lower.contains("db.lck")
        || line_lower.contains("failed to init transaction (unable to lock database)")
    {
        state.current_stage = "failed".to_string();
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "failed".to_string(),
            percentage: None,
            download_percentage: None,
            install_percentage: None,
            current_package: state.current_package.clone(),
            current_package_index: state.current_package_index,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Package database is locked by another process.".to_string(),
            raw_line: Some(line.to_string()),
            error: Some(line.to_string()),
            is_db_locked: true,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    // 2. Polkit authorization dismissal or cancellation
    if line_lower.contains("request dismissed")
        || line_lower.contains("not authorized")
        || line_lower.contains("authorization cancelled")
    {
        state.current_stage = "failed".to_string();
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "failed".to_string(),
            percentage: None,
            download_percentage: None,
            install_percentage: None,
            current_package: state.current_package.clone(),
            current_package_index: state.current_package_index,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Administrator authorization was cancelled.".to_string(),
            raw_line: Some(line.to_string()),
            error: Some(line.to_string()),
            is_db_locked: false,
            is_auth_cancelled: true,
            is_cached: false,
        });
    }

    // 3. Dependency resolution stage
    if line_lower.starts_with("resolving dependencies") {
        state.current_stage = "resolving".to_string();
        let overall = state.compute_overall_percentage("resolving");
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "resolving".to_string(),
            percentage: overall,
            download_percentage: None,
            install_percentage: None,
            current_package: state.current_package.clone(),
            current_package_index: None,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Resolving dependencies...".to_string(),
            raw_line: Some(line.to_string()),
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    // 4. Inter-conflicts
    if line_lower.starts_with("looking for conflicting packages") {
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "resolving".to_string(),
            percentage: state.compute_overall_percentage("resolving"),
            download_percentage: None,
            install_percentage: None,
            current_package: state.current_package.clone(),
            current_package_index: None,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: "Checking for conflicting packages...".to_string(),
            raw_line: Some(line.to_string()),
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    // 5. Total packages summary: "Packages (3) dbus-1.14 glib2-2.80 firefox-155.0"
    if line_clean.starts_with("Packages (") {
        if let Some(open_paren) = line_clean.find('(') {
            if let Some(close_paren) = line_clean.find(')') {
                let count_str = &line_clean[open_paren + 1..close_paren];
                if let Ok(count) = count_str.parse::<usize>() {
                    state.total_packages = Some(count);
                    return Some(TransactionProgressEvent {
                        transaction_id: state.transaction_id.clone(),
                        operation: state.operation.clone(),
                        target_package: state.target_package.clone(),
                        stage: "resolving".to_string(),
                        percentage: state.compute_overall_percentage("resolving"),
                        download_percentage: None,
                        install_percentage: None,
                        current_package: state.current_package.clone(),
                        current_package_index: None,
                        total_packages: Some(count),
                        bytes_downloaded_str: None,
                        bytes_total_str: None,
                        download_speed: None,
                        message: format!("Identified {} package(s) for transaction", count),
                        raw_line: Some(line.to_string()),
                        error: None,
                        is_db_locked: false,
                        is_auth_cancelled: false,
            is_cached: state.is_cached,
                    });
                }
            }
        }
    }

    // 6. Total Download Size
    if line_clean.starts_with("Total Download Size:") {
        let parts: Vec<&str> = line_clean.split(':').collect();
        if parts.len() == 2 {
            let size_str = parts[1].trim().to_string();
            state.bytes_total_str = Some(size_str.clone());
            return Some(TransactionProgressEvent {
                transaction_id: state.transaction_id.clone(),
                operation: state.operation.clone(),
                target_package: state.target_package.clone(),
                stage: "resolving".to_string(),
                percentage: state.compute_overall_percentage("resolving"),
                download_percentage: None,
                install_percentage: None,
                current_package: state.current_package.clone(),
                current_package_index: None,
                total_packages: state.total_packages,
                bytes_downloaded_str: None,
                bytes_total_str: Some(size_str.clone()),
                download_speed: None,
                message: format!("Total download size: {}", size_str),
                raw_line: Some(line.to_string()),
                error: None,
                is_db_locked: false,
                is_auth_cancelled: false,
            is_cached: state.is_cached,
            });
        }
    }

    // 6b. Cache detection: checking integrity/loading without prior download
    if line_lower.starts_with("checking package integrity")
        || line_lower.starts_with("loading package files")
        || line_lower.starts_with("checking for file conflicts")
        || line_lower.starts_with("checking available disk space")
    {
        if state.download_pct.is_none() && (state.operation == "install" || state.operation == "reinstall") {
            state.is_cached = true;
        }
        let msg = if state.is_cached {
            "Using cached package (verifying local archive)...".to_string()
        } else {
            "Verifying package files...".to_string()
        };
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "preparing".to_string(),
            percentage: state.compute_overall_percentage("preparing"),
            download_percentage: None,
            install_percentage: None,
            current_package: state.current_package.clone(),
            current_package_index: None,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: msg,
            raw_line: Some(line.to_string()),
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    // 7. Package installation step: e.g. "(1/3) installing dbus... [###] 100%" or "(2/3) upgrading glib2..."
    if line_clean.starts_with('(') {
        if let Some(close_paren) = line_clean.find(')') {
            let inside_paren = &line_clean[1..close_paren];
            let step_tokens: Vec<&str> = inside_paren.split('/').collect();
            if step_tokens.len() == 2 {
                if let (Ok(curr_idx), Ok(tot_count)) = (
                    step_tokens[0].trim().parse::<usize>(),
                    step_tokens[1].trim().parse::<usize>(),
                ) {
                    state.current_stage = "installing".to_string();
                    state.current_package_index = Some(curr_idx);
                    state.total_packages = Some(tot_count);

                    let remainder = line_clean[close_paren + 1..].trim();
                    let step_words: Vec<&str> = remainder.split_whitespace().collect();
                    let action = step_words.first().copied().unwrap_or("processing");
                    let pkg_name = step_words.get(1).map(|s| s.trim_end_matches("...")).unwrap_or("");

                    if !pkg_name.is_empty() {
                        state.current_package = Some(pkg_name.to_string());
                    }

                    let raw_install_pct = if tot_count > 0 {
                        ((curr_idx as f32 / tot_count as f32) * 100.0).round() as u8
                    } else {
                        100
                    };
                    state.install_pct = Some(raw_install_pct);
                    let overall_pct = state.compute_overall_percentage("installing");

                    return Some(TransactionProgressEvent {
                        transaction_id: state.transaction_id.clone(),
                        operation: state.operation.clone(),
                        target_package: state.target_package.clone(),
                        stage: "installing".to_string(),
                        percentage: overall_pct,
                        download_percentage: state.download_pct,
                        install_percentage: Some(raw_install_pct),
                        current_package: state.current_package.clone(),
                        current_package_index: Some(curr_idx),
                        total_packages: Some(tot_count),
                        bytes_downloaded_str: None,
                        bytes_total_str: None,
                        download_speed: None,
                        message: format!(
                            "{} {} ({}/{})",
                            capitalize_first(action),
                            state.current_package.as_deref().unwrap_or("package"),
                            curr_idx,
                            tot_count
                        ),
                        raw_line: Some(line.to_string()),
                        error: None,
                        is_db_locked: false,
                        is_auth_cancelled: false,
            is_cached: state.is_cached,
                    });
                }
            }
        }
    }

    // 8. Post-transaction hooks: "running '30-systemd-daemon-reload.hook'..."
    if line_lower.starts_with("running") && (line_lower.contains(".hook") || line_lower.contains("post-transaction")) {
        state.current_stage = "finalizing".to_string();
        let overall = state.compute_overall_percentage("finalizing");
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: "finalizing".to_string(),
            percentage: overall,
            download_percentage: state.download_pct,
            install_percentage: state.install_pct,
            current_package: state.current_package.clone(),
            current_package_index: state.current_package_index,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: if let Some(hook_name) = line_clean.split('\'').nth(1) {
                format!("Running hook: {}", hook_name)
            } else {
                "Finalizing system changes...".to_string()
            },
            raw_line: Some(line.to_string()),
            error: None,
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    // 9. Download progress bar with percentage: e.g. "firefox-155.0.1-1-x86_64 12.4 MiB 14.2 MiB/s 72%"
    if state.current_stage == "downloading" || line.contains('%') {
        if let Some(pct_idx) = line.rfind('%') {
            let before_pct = line[..pct_idx].trim_end();
            let num_str: String = before_pct
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();

            if let Ok(pct) = num_str.parse::<u8>() {
                if pct <= 100 {
                    state.current_stage = "downloading".to_string();
                    state.download_pct = Some(pct);
                    let overall_pct = state.compute_overall_percentage("downloading");

                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    let pkg_name = tokens.first().copied();
                    let size_info = if tokens.len() >= 3 {
                        format!("{} {}", tokens[1], tokens[2])
                    } else {
                        "".to_string()
                    };

                    let speed = tokens.iter().enumerate().find_map(|(idx, &t)| {
                        if t.contains("/s") {
                            if idx > 0 && tokens[idx - 1].chars().all(|c| c.is_ascii_digit() || c == '.') {
                                Some(format!("{} {}", tokens[idx - 1], t))
                            } else {
                                Some(t.to_string())
                            }
                        } else {
                            None
                        }
                    });

                    return Some(TransactionProgressEvent {
                        transaction_id: state.transaction_id.clone(),
                        operation: state.operation.clone(),
                        target_package: state.target_package.clone(),
                        stage: "downloading".to_string(),
                        percentage: overall_pct,
                        download_percentage: Some(pct),
                        install_percentage: None,
                        current_package: pkg_name.map(|s| s.to_string()).or_else(|| state.current_package.clone()),
                        current_package_index: None,
                        total_packages: state.total_packages,
                        bytes_downloaded_str: if !size_info.is_empty() { Some(size_info) } else { None },
                        bytes_total_str: state.bytes_total_str.clone(),
                        download_speed: speed,
                        message: if let Some(p) = pkg_name {
                            format!("Downloading {}: {}%", p, pct)
                        } else {
                            format!("Downloading: {}%", pct)
                        },
                        raw_line: Some(line.to_string()),
                        error: None,
                        is_db_locked: false,
                        is_auth_cancelled: false,
            is_cached: state.is_cached,
                    });
                }
            }
        }
    }

    // 10. General errors
    if line_lower.starts_with("error:") {
        return Some(TransactionProgressEvent {
            transaction_id: state.transaction_id.clone(),
            operation: state.operation.clone(),
            target_package: state.target_package.clone(),
            stage: state.current_stage.clone(),
            percentage: None,
            download_percentage: state.download_pct,
            install_percentage: state.install_pct,
            current_package: state.current_package.clone(),
            current_package_index: state.current_package_index,
            total_packages: state.total_packages,
            bytes_downloaded_str: None,
            bytes_total_str: None,
            download_speed: None,
            message: line.to_string(),
            raw_line: Some(line.to_string()),
            error: Some(line.to_string()),
            is_db_locked: false,
            is_auth_cancelled: false,
            is_cached: state.is_cached,
        });
    }

    None
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_output_lines_handles_both_newline_and_carriage_return() {
        let mut buffer = b"line1\nline2\rline3\r\nline4\r".to_vec();
        let lines = split_output_lines(&mut buffer);
        assert_eq!(lines, vec!["line1", "line2", "line3", "line4"]);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_split_output_lines_retains_incomplete_fragment() {
        let mut buffer = b"complete\npartial".to_vec();
        let lines = split_output_lines(&mut buffer);
        assert_eq!(lines, vec!["complete"]);
        assert_eq!(buffer, b"partial");
    }

    #[test]
    fn test_monotonic_overall_progress_never_regresses() {
        let mut state = TransactionParserState::new("txn-1".into(), "install".into(), "blender".into());

        let p1 = state.compute_overall_percentage("preparing").unwrap();
        assert_eq!(p1, 2);

        let p2 = state.compute_overall_percentage("resolving").unwrap();
        assert_eq!(p2, 5);

        // Downloading 90%
        state.download_pct = Some(90);
        let p3 = state.compute_overall_percentage("downloading").unwrap();
        // 5 + 90 * 0.55 = 54.5 -> 55
        assert_eq!(p3, 55);

        // Transitioning to installing step 1 of 10
        state.install_pct = Some(10);
        let p4 = state.compute_overall_percentage("installing").unwrap();
        // 60 + 10 * 0.35 = 63.5 -> 64
        assert!(p4 >= p3, "Progress must not jump backwards when switching from download to install");

        state.install_pct = Some(100);
        let p5 = state.compute_overall_percentage("installing").unwrap();
        assert_eq!(p5, 95);

        let p6 = state.compute_overall_percentage("finalizing").unwrap();
        assert_eq!(p6, 97);

        let p7 = state.compute_overall_percentage("completed").unwrap();
        assert_eq!(p7, 100);
    }

    #[test]
    fn test_throttler_coalesces_rapid_events() {
        let mut throttler = TransactionThrottler::new(75);
        let mut state = TransactionParserState::new("txn-1".into(), "install".into(), "blender".into());

        // Initial event emitted
        state.download_pct = Some(10);
        let ev1 = parse_pacman_line("blender 10.0 MiB 10.0 MiB/s 10%", &mut state).unwrap();
        assert!(throttler.should_emit(&ev1));

        // Rapid immediate subsequent event within 1ms -> should be throttled (false)
        state.download_pct = Some(11);
        let ev2 = parse_pacman_line("blender 11.0 MiB 10.0 MiB/s 11%", &mut state).unwrap();
        assert!(!throttler.should_emit(&ev2));

        // Package step change -> must emit immediately
        let ev3 = parse_pacman_line("(1/5) installing alembic...", &mut state).unwrap();
        assert!(throttler.should_emit(&ev3));

        // Error -> must emit immediately
        let ev4 = parse_pacman_line("error: failed to commit transaction", &mut state).unwrap();
        assert!(throttler.should_emit(&ev4));
    }

    #[test]
    fn test_parse_step_installing_multi_package() {
        let mut state = TransactionParserState::new("txn-1".into(), "install".into(), "blender".into());
        let ev = parse_pacman_line("(2/4) installing ceres-solver... [######################] 100%", &mut state).unwrap();
        assert_eq!(ev.stage, "installing");
        assert_eq!(ev.current_package, Some("ceres-solver".to_string()));
        assert_eq!(ev.current_package_index, Some(2));
        assert_eq!(ev.total_packages, Some(4));
        assert_eq!(ev.install_percentage, Some(50));
    }
}
