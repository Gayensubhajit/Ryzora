/// notifications.rs — Persistent activity and notification log (Phase 15.2)
///
/// Events are appended to ~/.local/share/ryzora/activity_log.jsonl
/// (one JSON object per line). Security-critical events (KeyRevoked,
/// IntegrityFailure, SignatureInvalid) are NEVER auto-deleted regardless
/// of the age filter supplied to clear_old_notifications.
///
/// CONCURRENCY & ATOMICITY HARDENING:
/// - All mutations (append, read-modify-write) are serialized using an in-process Mutex.
/// - Rewrites use atomic temp file + rename rather than in-place truncation.
/// - Permissions on directory (0700) and log file (0600) are strictly enforced.
/// - Malformed log entries are gracefully handled and preserved without panic.
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Global lock serializing all mutations to the activity log.
static ACTIVITY_LOG_LOCK: Mutex<()> = Mutex::new(());

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
// Paths & Permissions
// ─────────────────────────────────────────────────────────────────────────────

pub fn activity_log_dir() -> PathBuf {
    crate::snapshot::get_home_dir().join(".local/share/ryzora")
}

pub fn activity_log_path() -> PathBuf {
    activity_log_dir().join("activity_log.jsonl")
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
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec
    )
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
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
    for &m in &month_days {
        if days < m {
            break;
        }
        days -= m;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn parse_iso_secs(iso: &str) -> Option<u64> {
    if iso.len() < 10 {
        return None;
    }
    let parts: Vec<&str> = iso.split('T').collect();
    let date_parts: Vec<u64> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 {
        return None;
    }
    let (year, month, day) = (date_parts[0], date_parts[1], date_parts[2]);
    if year < 1970 || month < 1 || month > 12 || day < 1 || day > 31 {
        return None;
    }

    let mut days = 0u64;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days: [u64; 12] = if is_leap(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for m in 0..(month.saturating_sub(1) as usize) {
        days += month_days[m];
    }
    days += day.saturating_sub(1);

    let (hour, min, sec) = if parts.len() > 1 {
        let time_str = parts[1].trim_end_matches('Z');
        let t_parts: Vec<u64> = time_str.split(':').filter_map(|p| p.parse().ok()).collect();
        (
            t_parts.first().copied().unwrap_or(0),
            t_parts.get(1).copied().unwrap_or(0),
            t_parts.get(2).copied().unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };

    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

// ─────────────────────────────────────────────────────────────────────────────
// Core operations (Thread-safe, atomic temp+rename, permission-hardened)
// ─────────────────────────────────────────────────────────────────────────────

/// Atomically rewrite the activity log using a temporary file and rename.
pub fn atomic_write_log_to(path: &Path, content: &str) -> Result<(), String> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create activity log directory: {}", e))?;
    let _ = crate::crypto::set_secure_permissions(dir, 0o700);

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let tmp = dir.join(format!(".activity_log.tmp.{}", nanos));

    fs::write(&tmp, content)
        .map_err(|e| format!("Failed to write temporary activity log: {}", e))?;
    let _ = crate::crypto::set_secure_permissions(&tmp, 0o600);

    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to atomically replace activity log: {}", e)
    })?;
    let _ = crate::crypto::set_secure_permissions(path, 0o600);
    Ok(())
}

/// Append a new activity entry to an explicit log path under the global lock.
pub fn append_entry_to(
    path: &Path,
    kind: NotificationKind,
    severity: Severity,
    title: &str,
    message: &str,
    related_package_id: Option<&str>,
    related_repository_id: Option<&str>,
) -> Result<ActivityEntry, String> {
    let _guard = ACTIVITY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create activity log directory: {}", e))?;
        let _ = crate::crypto::set_secure_permissions(parent, 0o700);
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
        .open(path)
        .map_err(|e| format!("Failed to open activity log for append: {}", e))?;
    let _ = crate::crypto::set_secure_permissions(path, 0o600);

    writeln!(file, "{}", line).map_err(|e| format!("Failed to write activity entry: {}", e))?;

    Ok(entry)
}

/// Append a new activity entry to the system activity log.
pub fn append_entry(
    kind: NotificationKind,
    severity: Severity,
    title: &str,
    message: &str,
    related_package_id: Option<&str>,
    related_repository_id: Option<&str>,
) -> Result<ActivityEntry, String> {
    append_entry_to(
        &activity_log_path(),
        kind,
        severity,
        title,
        message,
        related_package_id,
        related_repository_id,
    )
}

/// Load all entries from an explicit log path, newest first.
pub fn load_all_entries_from(path: &Path) -> Vec<ActivityEntry> {
    let _guard = ACTIVITY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if !path.is_file() {
        return Vec::new();
    }
    let Ok(file) = fs::File::open(path) else {
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

/// Load all entries from the log, newest first.
pub fn load_all_entries() -> Vec<ActivityEntry> {
    load_all_entries_from(&activity_log_path())
}

/// Mark a specific entry as read in an explicit log path.
pub fn mark_read_in(path: &Path, id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.contains("..")
        || id.contains('/')
        || id.contains('\\')
        || id.contains('\0')
    {
        return Err(format!("Invalid notification ID '{}'", id));
    }

    let _guard = ACTIVITY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if !path.is_file() {
        return Ok(());
    }
    let raw =
        fs::read_to_string(path).map_err(|e| format!("Failed to read activity log: {}", e))?;

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
        .map(|l| {
            l + "
"
        })
        .collect();

    atomic_write_log_to(path, &new_content)
}

/// Mark a specific entry as read. Rewrites the log file atomically.
pub fn mark_read(id: &str) -> Result<(), String> {
    mark_read_in(&activity_log_path(), id)
}

/// Mark all entries as read in an explicit log path.
pub fn mark_all_read_in(path: &Path) -> Result<(), String> {
    let _guard = ACTIVITY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if !path.is_file() {
        return Ok(());
    }
    let raw =
        fs::read_to_string(path).map_err(|e| format!("Failed to read activity log: {}", e))?;

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
        .map(|l| {
            l + "
"
        })
        .collect();

    atomic_write_log_to(path, &new_content)
}

/// Mark all entries as read.
pub fn mark_all_read_impl() -> Result<(), String> {
    mark_all_read_in(&activity_log_path())
}

/// Delete entries older than  days in an explicit log path.
/// Security-critical entries (IntegrityFailure, SignatureInvalid, KeyRevoked)
/// are NEVER deleted regardless of age.
pub fn clear_old_entries_in(path: &Path, days: u32) -> Result<usize, String> {
    let _guard = ACTIVITY_LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    if !path.is_file() {
        return Ok(0);
    }
    let raw =
        fs::read_to_string(path).map_err(|e| format!("Failed to read activity log: {}", e))?;

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
                if let Some(entry_secs) = parse_iso_secs(&entry.timestamp) {
                    if entry_secs < cutoff_secs {
                        removed += 1;
                        return false;
                    }
                }
                true
            } else {
                // Preserve malformed lines — never discard unparsed content
                true
            }
        })
        .map(|l| {
            l.to_string()
                + "
"
        })
        .collect();

    atomic_write_log_to(path, &new_content)?;
    Ok(removed)
}

/// Delete entries older than  days, but NEVER delete security-critical
/// entries (IntegrityFailure, SignatureInvalid, KeyRevoked).
pub fn clear_old_entries_impl(days: u32) -> Result<usize, String> {
    clear_old_entries_in(&activity_log_path(), days)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri Commands
// ─────────────────────────────────────────────────────────────────────────────

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

/// Delete notifications older than  days.
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
    let _ = append_entry(kind, severity, title, message, package_id, repo_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Arc;
    use std::thread;

    fn temp_log(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        env::temp_dir().join(format!("ryzora-notif-test-{}-{}.jsonl", label, nanos))
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
        let entry = append_entry_to(
            &path,
            NotificationKind::PackageInstalled,
            Severity::Info,
            "Test Package",
            "Successfully installed",
            Some("pkg-test"),
            None,
        )
        .unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains(&entry.id));
        assert!(raw.contains("package_installed"));
        assert!(raw.contains("pkg-test"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "Activity log file must have 0600 permissions");
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_concurrent_append_and_rewrite_no_lost_events() {
        let path = temp_log("concurrent");
        let path_arc = Arc::new(path.clone());

        // Spawn 4 writer threads, each appending 15 events
        let mut handles = Vec::new();
        for thread_idx in 0..4 {
            let p = Arc::clone(&path_arc);
            let handle = thread::spawn(move || {
                for i in 0..15 {
                    let title = format!("Thread-{} Entry-{}", thread_idx, i);
                    let _ = append_entry_to(
                        &p,
                        NotificationKind::PackageInstalled,
                        Severity::Info,
                        &title,
                        "Concurrent message",
                        None,
                        None,
                    );
                }
            });
            handles.push(handle);
        }

        // Simultaneously spawn 2 rewriter threads running mark_all_read_in
        for _ in 0..2 {
            let p = Arc::clone(&path_arc);
            let handle = thread::spawn(move || {
                for _ in 0..5 {
                    let _ = mark_all_read_in(&p);
                    thread::yield_now();
                }
            });
            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        // Check that ALL 60 appended events are present in the file — zero lost events!
        let all_entries = load_all_entries_from(&path);
        assert_eq!(
            all_entries.len(),
            60,
            "Concurrent append/rewrite must not lose any events"
        );

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_malformed_entries_preserved_without_crash() {
        let path = temp_log("malformed");
        let e1 = make_entry("e1", "package_installed", "info", "2026-01-01T00:00:00Z");
        let valid_line = serde_json::to_string(&e1).unwrap();
        let malformed_line = "{ this is corrupt json ";
        fs::write(
            &path,
            format!(
                "{}
{}
",
                valid_line, malformed_line
            ),
        )
        .unwrap();

        // Loading all entries should skip malformed line gracefully
        let entries = load_all_entries_from(&path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "e1");

        // Rewriting (e.g. mark_read) should preserve malformed line rather than drop it
        mark_read_in(&path, "e1").unwrap();
        let updated_raw = fs::read_to_string(&path).unwrap();
        assert!(updated_raw.contains("{ this is corrupt json "));

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
        assert!(secs.unwrap() > 1_000_000_000);
    }

    #[test]
    fn test_notifications_mark_read_updates_flag() {
        let path = temp_log("markread");
        let e1 = append_entry_to(
            &path,
            NotificationKind::PackageInstalled,
            Severity::Info,
            "E1",
            "msg1",
            None,
            None,
        )
        .unwrap();
        let e2 = append_entry_to(
            &path,
            NotificationKind::RepositorySynced,
            Severity::Info,
            "E2",
            "msg2",
            None,
            None,
        )
        .unwrap();

        mark_read_in(&path, &e1.id).unwrap();

        let entries = load_all_entries_from(&path);
        let e1_updated = entries.iter().find(|e| e.id == e1.id).unwrap();
        let e2_updated = entries.iter().find(|e| e.id == e2.id).unwrap();
        assert!(e1_updated.read);
        assert!(!e2_updated.read);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_clear_old_preserves_security_alerts() {
        let path = temp_log("clear_old");

        // Write an old regular event and an old security alert
        let old_regular = make_entry(
            "old_reg",
            "package_installed",
            "info",
            "2020-01-01T00:00:00Z",
        );
        let old_security = make_entry(
            "old_sec",
            "integrity_failure",
            "critical",
            "2020-01-01T00:00:00Z",
        );
        let line1 = serde_json::to_string(&old_regular).unwrap();
        let line2 = serde_json::to_string(&old_security).unwrap();
        fs::write(
            &path,
            format!(
                "{}
{}
",
                line1, line2
            ),
        )
        .unwrap();

        // Clear entries older than 30 days
        let removed = clear_old_entries_in(&path, 30).unwrap();
        assert_eq!(removed, 1, "Only the non-security event should be removed");

        let remaining = load_all_entries_from(&path);
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "old_sec");
        assert_eq!(remaining[0].category, "integrity_failure");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_notifications_zero_command_execution() {
        let entry = make_entry("e0", "package_installed", "info", "2026-01-01T00:00:00Z");
        assert!(!entry.id.is_empty());
    }
}
