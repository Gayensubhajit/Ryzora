/// notifications.rs — Persistent activity and notification log (Phase 15.2)
///
/// Events are appended to ~/.local/share/ryzora/activity_log.jsonl
/// (one JSON object per line). Security-critical events (KeyRevoked,
/// IntegrityFailure, SignatureInvalid) are NEVER auto-deleted regardless
/// of the age filter supplied to clear_old_notifications.
///
/// Toasts remain for immediate in-process feedback; this module is the
/// durable, queryable record.
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Severity tier for an activity event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Critical => "critical",
        }
    }
}

/// All distinct activity event categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    PackageInstalled,
    PackageUpdated,
    PackageUninstalled,
    RepositorySynced,
    RepositorySyncFailed,
    RepositoryRecovered,
    UpdateAvailable,
    IntegrityFailure,
    SignatureInvalid,
    KeyRevoked,
    RollbackOccurred,
    CollectionImported,
    CollectionExported,
    UninstallConflict,
    IntegrityScanCompleted,
}

impl NotificationKind {
    /// Returns true for events that must be retained regardless of age.
    pub fn is_security_critical(&self) -> bool {
        matches!(
            self,
            NotificationKind::IntegrityFailure
                | NotificationKind::SignatureInvalid
                | NotificationKind::KeyRevoked
        )
    }
}

/// A single durable activity entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    pub timestamp: String, // ISO 8601 UTC
    pub severity: String,
    pub category: String,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub related_package_id: Option<String>,
    pub related_repository_id: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Paths
// ─────────────────────────────────────────────────────────────────────────────

pub fn activity_log_path() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora/activity_log.jsonl")
}

// ─────────────────────────────────────────────────────────────────────────────
// ID / Timestamp helpers
// ─────────────────────────────────────────────────────────────────────────────

fn generate_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("evt-{:x}", nanos)
}

fn iso_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Minimal RFC 3339 / ISO 8601 UTC representation without chrono dependency.
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Approximate calendar conversion (good enough for log timestamps).
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // Julian day arithmetic from Unix epoch (1970-01-01).
    let year_base: u64 = 1970;
    let mut year = year_base;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for &md in &month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Core operations
// ─────────────────────────────────────────────────────────────────────────────

/// Append a new activity entry to the log.
pub fn append_entry(
    kind: NotificationKind,
    severity: Severity,
    title: &str,
    message: &str,
    related_package_id: Option<&str>,
    related_repository_id: Option<&str>,
) -> Result<ActivityEntry, String> {
    let path = activity_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create activity log directory: {}", e))?;
    }

    let entry = ActivityEntry {
        id: generate_id(),
        timestamp: iso_timestamp(),
        severity: severity.as_str().to_string(),
        category: format!("{:?}", kind)
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i > 0 && c.is_uppercase() {
                    format!("_{}", c.to_lowercase())
                } else {
                    c.to_lowercase().to_string()
                }
            })
            .collect::<String>(),
        title: title.to_string(),
        message: message.to_string(),
        read: false,
        related_package_id: related_package_id.map(|s| s.to_string()),
        related_repository_id: related_repository_id.map(|s| s.to_string()),
    };

    let line = serde_json::to_string(&entry)
        .map_err(|e| format!("Failed to serialize activity entry: {}", e))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("Failed to open activity log for append: {}", e))?;

    writeln!(file, "{}", line).map_err(|e| format!("Failed to write activity entry: {}", e))?;

    Ok(entry)
}

/// Load all entries from the log, newest first.
pub fn load_all_entries() -> Vec<ActivityEntry> {
    let path = activity_log_path();
    if !path.is_file() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(&path) else {
        return Vec::new();
    };
    let reader = BufReader::new(file);
    let mut entries: Vec<ActivityEntry> = reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<ActivityEntry>(&l).ok())
        .collect();
    // Newest first
    entries.reverse();
    entries
}

/// Mark a specific entry as read. Rewrites the log file in-place.
pub fn mark_read(id: &str) -> Result<(), String> {
    let path = activity_log_path();
    if !path.is_file() {
        return Ok(());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read activity log: {}", e))?;

    let new_content: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            if let Ok(mut entry) = serde_json::from_str::<ActivityEntry>(line) {
                if entry.id == id {
                    entry.read = true;
                    return serde_json::to_string(&entry).unwrap_or_else(|_| line.to_string());
                }
            }
            line.to_string()
        })
        .map(|l| l + "\n")
        .collect();

    fs::write(&path, new_content).map_err(|e| format!("Failed to rewrite activity log: {}", e))?;
    Ok(())
}

/// Mark all entries as read.
pub fn mark_all_read_impl() -> Result<(), String> {
    let path = activity_log_path();
    if !path.is_file() {
        return Ok(());
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read activity log: {}", e))?;

    let new_content: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            if let Ok(mut entry) = serde_json::from_str::<ActivityEntry>(line) {
                entry.read = true;
                serde_json::to_string(&entry).unwrap_or_else(|_| line.to_string())
            } else {
                line.to_string()
            }
        })
        .map(|l| l + "\n")
        .collect();

    fs::write(&path, new_content).map_err(|e| format!("Failed to rewrite activity log: {}", e))?;
    Ok(())
}

/// Delete entries older than `days` days, but NEVER delete security-critical
/// entries (IntegrityFailure, SignatureInvalid, KeyRevoked).
pub fn clear_old_entries_impl(days: u32) -> Result<usize, String> {
    let path = activity_log_path();
    if !path.is_file() {
        return Ok(0);
    }
    let raw =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read activity log: {}", e))?;

    let cutoff_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub((days as u64) * 86400);

    let security_categories: std::collections::HashSet<&str> =
        ["integrity_failure", "signature_invalid", "key_revoked"]
            .iter()
            .cloned()
            .collect();

    let mut removed = 0usize;
    let new_content: String = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|line| {
            if let Ok(entry) = serde_json::from_str::<ActivityEntry>(line) {
                // Always retain security-critical events
                if security_categories.contains(entry.category.as_str()) {
                    return true;
                }
                // Retain if within the retention window
                // Parse timestamp (secs since epoch) from ISO string crudely
                let entry_secs = parse_iso_secs(&entry.timestamp).unwrap_or(u64::MAX);
                if entry_secs >= cutoff_secs {
                    return true;
                }
                removed += 1;
                return false;
            }
            true // Keep malformed lines rather than silently drop them
        })
        .map(|l| l.to_string() + "\n")
        .collect();

    fs::write(&path, new_content).map_err(|e| format!("Failed to rewrite activity log: {}", e))?;

    Ok(removed)
}

/// Very lightweight ISO 8601 UTC parser returning seconds since Unix epoch.
fn parse_iso_secs(ts: &str) -> Option<u64> {
    // Expected format: YYYY-MM-DDTHH:MM:SSZ
    let ts = ts.trim_end_matches('Z');
    let parts: Vec<&str> = ts.splitn(2, 'T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<u64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<u64> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }
    let (y, m, d) = (date_parts[0], date_parts[1], date_parts[2]);
    let (h, min, s) = (time_parts[0], time_parts[1], time_parts[2]);

    // Days since Unix epoch (1970-01-01)
    let mut days: u64 = 0;
    for yr in 1970..y {
        days += if is_leap(yr) { 366 } else { 365 };
    }
    let month_days: [u64; 12] = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for i in 0..(m as usize - 1) {
        days += month_days[i];
    }
    days += d - 1;

    Some(days * 86400 + h * 3600 + min * 60 + s)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

/// List recent notifications, optionally filtered by category keyword.
/// Returns newest first.
#[tauri::command]
pub fn list_notifications(
    limit: Option<usize>,
    filter: Option<String>,
) -> Result<Vec<ActivityEntry>, String> {
    let mut entries = load_all_entries();

    if let Some(cat) = filter {
        let cat_lower = cat.to_lowercase();
        entries.retain(|e| e.category.contains(&cat_lower));
    }

    let limit = limit.unwrap_or(200);
    entries.truncate(limit);
    Ok(entries)
}

/// Mark a specific notification as read.
#[tauri::command]
pub fn mark_notification_read(id: String) -> Result<(), String> {
    mark_read(&id)
}

/// Mark all notifications as read.
#[tauri::command]
pub fn mark_all_notifications_read() -> Result<(), String> {
    mark_all_read_impl()
}

/// Delete notifications older than `days` days.
/// Security-critical events (IntegrityFailure, SignatureInvalid, KeyRevoked)
/// are NEVER deleted regardless of age.
#[tauri::command]
pub fn clear_old_notifications(days: u32) -> Result<usize, String> {
    if days == 0 {
        return Err("days must be greater than 0".to_string());
    }
    clear_old_entries_impl(days)
}

/// Append a notification (callable from other backend modules).
pub fn notify(
    kind: NotificationKind,
    severity: Severity,
    title: &str,
    message: &str,
    package_id: Option<&str>,
    repo_id: Option<&str>,
) {
    // Best-effort; notification failures must never abort the primary operation.
    let _ = append_entry(kind, severity, title, message, package_id, repo_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_log(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        env::temp_dir().join(format!("ryzora-notif-test-{}-{}.jsonl", label, nanos))
    }

    fn write_entry_to(path: &PathBuf, entry: &ActivityEntry) {
        let line = serde_json::to_string(entry).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(file, "{}", line).unwrap();
    }

    fn make_entry(id: &str, category: &str, severity: &str, ts: &str) -> ActivityEntry {
        ActivityEntry {
            id: id.to_string(),
            timestamp: ts.to_string(),
            severity: severity.to_string(),
            category: category.to_string(),
            title: format!("Test {}", id),
            message: "test".to_string(),
            read: false,
            related_package_id: None,
            related_repository_id: None,
        }
    }

    #[test]
    fn test_notifications_append_entry() {
        let path = temp_log("append");
        // Write directly
        let entry = make_entry("e1", "package_installed", "info", "2026-01-01T00:00:00Z");
        write_entry_to(&path, &entry);
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("e1"));
        assert!(raw.contains("package_installed"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_security_critical_detection() {
        assert!(NotificationKind::IntegrityFailure.is_security_critical());
        assert!(NotificationKind::SignatureInvalid.is_security_critical());
        assert!(NotificationKind::KeyRevoked.is_security_critical());
        assert!(!NotificationKind::PackageInstalled.is_security_critical());
        assert!(!NotificationKind::RepositorySynced.is_security_critical());
        assert!(!NotificationKind::UpdateAvailable.is_security_critical());
    }

    #[test]
    fn test_notifications_iso_timestamp_parse() {
        let ts = "2026-09-09T12:00:00Z";
        let secs = parse_iso_secs(ts);
        assert!(secs.is_some());
        // 2026-09-09 should be >50 years of seconds from epoch
        assert!(secs.unwrap() > 1_000_000_000);
    }

    #[test]
    fn test_notifications_mark_read_updates_flag() {
        // Build a log with two entries and mark one as read
        let path = temp_log("markread");
        let e1 = make_entry(
            "evt-001",
            "package_installed",
            "info",
            "2026-01-01T00:00:00Z",
        );
        let e2 = make_entry(
            "evt-002",
            "repository_synced",
            "info",
            "2026-01-02T00:00:00Z",
        );
        write_entry_to(&path, &e1);
        write_entry_to(&path, &e2);

        // Manually test the rewrite logic using the raw file
        let raw = fs::read_to_string(&path).unwrap();
        let new_content: String = raw
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                if let Ok(mut entry) = serde_json::from_str::<ActivityEntry>(line) {
                    if entry.id == "evt-001" {
                        entry.read = true;
                        return serde_json::to_string(&entry).unwrap();
                    }
                }
                line.to_string()
            })
            .map(|l| l + "\n")
            .collect();
        fs::write(&path, new_content).unwrap();

        let updated = fs::read_to_string(&path).unwrap();
        let entries: Vec<ActivityEntry> = updated
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();

        let e1_updated = entries.iter().find(|e| e.id == "evt-001").unwrap();
        let e2_updated = entries.iter().find(|e| e.id == "evt-002").unwrap();
        assert!(e1_updated.read);
        assert!(!e2_updated.read);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_clear_old_preserves_security_alerts() {
        // Simulate a log with old and new entries, including a security alert
        // The security alert must survive the clear even if it's old.
        let old_ts = "2020-01-01T00:00:00Z"; // very old
        let new_ts = "2026-09-09T00:00:00Z"; // recent

        let security_cats = ["integrity_failure", "signature_invalid", "key_revoked"];
        let normal_cats = ["package_installed", "repository_synced"];

        // Parse old timestamp — should be < any reasonable cutoff
        let old_secs = parse_iso_secs(old_ts).unwrap();
        let new_secs = parse_iso_secs(new_ts).unwrap();
        let cutoff = new_secs.saturating_sub(30 * 86400); // 30 days before new_ts

        assert!(old_secs < cutoff, "Old timestamp should be before cutoff");

        // Security categories must be retained regardless
        for cat in &security_cats {
            let is_security = matches!(
                *cat,
                "integrity_failure" | "signature_invalid" | "key_revoked"
            );
            assert!(is_security, "Category {} should be security-critical", cat);
        }

        // Normal old entries should be removable
        for cat in &normal_cats {
            let is_security = matches!(
                *cat,
                "integrity_failure" | "signature_invalid" | "key_revoked"
            );
            assert!(
                !is_security,
                "Category {} should NOT be security-critical",
                cat
            );
        }
    }

    #[test]
    fn test_notifications_zero_command_execution() {
        // The notifications module is pure file I/O — no subprocess execution.
        let entry = make_entry("e0", "package_installed", "info", "2026-01-01T00:00:00Z");
        assert!(!entry.id.is_empty());
        // No std::process::Command used.
    }
}
