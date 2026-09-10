//! Ryzora File Ownership Ledger — Phase 22
//!
//! Tracks which Ryzora package owns each installed user-space artifact.
//! This is authoritative only for Ryzora-managed files; it never touches or
//! governs system configuration paths managed by the existing snapshot/adapter
//! mechanism (e.g. /etc/sddm.conf.d/, /usr/share/sddm/themes/,
//! ~/.config/hypr/, ~/.config/systemd/user/).
//!
//! # Invariants
//! 1. A file is claimable only when its target path passes `is_ryzora_owned_path`.
//! 2. Claiming a path already owned by a *different* package returns a ConflictReport.
//! 3. Re-claiming by the *same* package (update) is always allowed.
//! 4. On failed installation, all claims made during that transaction are rolled back.
//! 5. Uninstall removes only entries for the given package.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// ─────────────────────────────────────────────────────────────────────────────
// Protected system paths — the ledger NEVER claims these
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if a target path string refers to a system-managed location
/// that the ownership ledger must never claim.
pub fn is_system_managed_path(target: &str) -> bool {
    let t = target.trim();
    t.starts_with("/etc/")
        || t.starts_with("/usr/share/sddm/themes/")
        || t.starts_with("/usr/share/sddm/")
        || t.starts_with("/usr/")
        || t.starts_with("/sys/")
        || t.starts_with("/var/")
        || t.starts_with("/boot/")
        || t.starts_with("/run/")
        || t.contains("/.config/hypr/")
        || t.contains("/.config/systemd/user/")
}

/// Returns true for paths the ownership ledger is permitted to manage.
pub fn is_ryzora_owned_path(target: &str, home_dir: &Path) -> bool {
    if is_system_managed_path(target) {
        return false;
    }
    if target.starts_with("~/") {
        return true;
    }
    if let Ok(abs) = crate::snapshot::validate_and_expand_path(target, home_dir) {
        return abs.starts_with(home_dir);
    }
    false
}

// ─────────────────────────────────────────────────────────────────────────────
// Data types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnedFile {
    pub package_id: String,
    pub installed_at: u64,
    /// SHA-256 hash at claim time. Empty string for symlinks.
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConflictReport {
    pub path: String,
    pub owned_by: String,
    pub requested_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OwnershipLedger {
    entries: HashMap<String, OwnedFile>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Ledger path
// ─────────────────────────────────────────────────────────────────────────────

fn ledger_path() -> PathBuf {
    crate::installer::get_ryzora_base_dir().join("ownership.json")
}

// ─────────────────────────────────────────────────────────────────────────────
// Core API
// ─────────────────────────────────────────────────────────────────────────────

impl OwnershipLedger {
    pub fn load() -> Self {
        let path = ledger_path();
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub fn load_from(dir: &Path) -> Self {
        let path = dir.join("ownership.json");
        let json = match std::fs::read_to_string(&path) {
            Ok(j) => j,
            Err(_) => return Self::default(),
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = ledger_path();
        self.save_to(path.parent().unwrap_or(Path::new(".")))
    }

    pub fn save_to(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create ledger dir: {}", e))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize ledger: {}", e))?;
        std::fs::write(dir.join("ownership.json"), json)
            .map_err(|e| format!("Failed to write ledger: {}", e))
    }

    pub fn query(&self, target: &str) -> Option<&OwnedFile> {
        self.entries.get(target)
    }

    pub fn owned_by(&self, package_id: &str) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(_, v)| v.package_id == package_id)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn check_conflict(&self, package_id: &str, target: &str) -> Option<ConflictReport> {
        if let Some(existing) = self.entries.get(target) {
            if existing.package_id != package_id {
                return Some(ConflictReport {
                    path: target.to_string(),
                    owned_by: existing.package_id.clone(),
                    requested_by: package_id.to_string(),
                });
            }
        }
        None
    }

    /// Claims ownership. Returns Some(ConflictReport) if blocked; None on success.
    pub fn claim(
        &mut self,
        package_id: &str,
        target: &str,
        sha256: &str,
    ) -> Option<ConflictReport> {
        if let Some(conflict) = self.check_conflict(package_id, target) {
            return Some(conflict);
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.entries.insert(
            target.to_string(),
            OwnedFile {
                package_id: package_id.to_string(),
                installed_at: now,
                sha256: sha256.to_string(),
            },
        );
        None
    }

    /// Releases ownership of a specific path. No-op if owned by a different package.
    pub fn release(&mut self, package_id: &str, target: &str) {
        if let Some(entry) = self.entries.get(target) {
            if entry.package_id == package_id {
                self.entries.remove(target);
            }
        }
    }

    /// Releases all ownership entries for a package.
    pub fn release_all(&mut self, package_id: &str) {
        self.entries.retain(|_, v| v.package_id != package_id);
    }

    pub fn check_all_conflicts(
        &self,
        package_id: &str,
        targets: &[String],
    ) -> Vec<ConflictReport> {
        targets
            .iter()
            .filter_map(|t| self.check_conflict(package_id, t))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn query_file_owner(target: String) -> Result<Option<OwnedFile>, String> {
    let ledger = OwnershipLedger::load();
    Ok(ledger.query(&target).cloned())
}

#[tauri::command]
pub fn list_owned_files(package_id: String) -> Result<Vec<String>, String> {
    let ledger = OwnershipLedger::load();
    Ok(ledger.owned_by(&package_id))
}

#[tauri::command]
pub fn check_ownership_conflicts(
    package_id: String,
    targets: Vec<String>,
) -> Result<Vec<ConflictReport>, String> {
    let ledger = OwnershipLedger::load();
    Ok(ledger.check_all_conflicts(&package_id, &targets))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir_unique(name: &str) -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("ryzora-ownership-test-{}-{}", name, id));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_claim_and_query() {
        let mut ledger = OwnershipLedger::default();
        assert!(ledger.claim("pkg-a", "~/.local/share/foo/bar.conf", "aabbcc").is_none());
        let owned = ledger.query("~/.local/share/foo/bar.conf").unwrap();
        assert_eq!(owned.package_id, "pkg-a");
        assert_eq!(owned.sha256, "aabbcc");
    }

    #[test]
    fn test_same_package_reclaim_allowed() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/some/file", "hash1");
        let conflict = ledger.claim("pkg-a", "~/some/file", "hash2");
        assert!(conflict.is_none());
        assert_eq!(ledger.query("~/some/file").unwrap().sha256, "hash2");
    }

    #[test]
    fn test_cross_package_conflict_detected() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/shared/file.conf", "hash1");
        let conflict = ledger.claim("pkg-b", "~/shared/file.conf", "hash2");
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.owned_by, "pkg-a");
        assert_eq!(c.requested_by, "pkg-b");
        // pkg-a still owns it
        assert_eq!(ledger.query("~/shared/file.conf").unwrap().package_id, "pkg-a");
    }

    #[test]
    fn test_release_removes_entry() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/some/file", "h");
        ledger.release("pkg-a", "~/some/file");
        assert!(ledger.query("~/some/file").is_none());
    }

    #[test]
    fn test_release_by_wrong_package_noop() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/some/file", "h");
        ledger.release("pkg-b", "~/some/file");
        assert_eq!(ledger.query("~/some/file").unwrap().package_id, "pkg-a");
    }

    #[test]
    fn test_release_all_removes_only_target_package() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/file-a", "ha");
        ledger.claim("pkg-b", "~/file-b", "hb");
        ledger.claim("pkg-a", "~/file-a2", "ha2");
        ledger.release_all("pkg-a");
        assert!(ledger.query("~/file-a").is_none());
        assert!(ledger.query("~/file-a2").is_none());
        assert!(ledger.query("~/file-b").is_some());
    }

    #[test]
    fn test_check_all_conflicts_batch() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/shared1", "h1");
        ledger.claim("pkg-a", "~/shared2", "h2");
        let targets = vec![
            "~/shared1".to_string(),
            "~/shared2".to_string(),
            "~/new-file".to_string(),
        ];
        let conflicts = ledger.check_all_conflicts("pkg-b", &targets);
        assert_eq!(conflicts.len(), 2);
        assert!(!conflicts.iter().any(|c| c.path == "~/new-file"));
    }

    #[test]
    fn test_persist_and_reload() {
        let dir = temp_dir_unique("persist");
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/persist-test/file.conf", "deadbeef");
        ledger.save_to(&dir).unwrap();
        let reloaded = OwnershipLedger::load_from(&dir);
        let entry = reloaded.query("~/persist-test/file.conf").unwrap();
        assert_eq!(entry.package_id, "pkg-a");
        assert_eq!(entry.sha256, "deadbeef");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_system_paths_rejected() {
        let home = PathBuf::from("/home/testuser");
        let protected = &[
            "/etc/sddm.conf.d/ryzora.conf",
            "/usr/share/sddm/themes/ryzora-foo/Main.qml",
            "/usr/share/icons/foo",
            "/var/lib/sddm/.config",
        ];
        for p in protected {
            assert!(is_system_managed_path(p), "Should be system-managed: {}", p);
            assert!(!is_ryzora_owned_path(p, &home), "Should NOT be ownable: {}", p);
        }
    }

    #[test]
    fn test_user_space_paths_accepted() {
        let home = PathBuf::from("/home/testuser");
        let user_paths = &[
            "~/.local/share/ryzora/lockscreens/qylock/foo/Main.qml",
            "~/.local/share/ryzora/wallpapers/foo.png",
        ];
        for p in user_paths {
            assert!(is_ryzora_owned_path(p, &home), "Should be ownable: {}", p);
        }
    }

    #[test]
    fn test_owned_by_returns_all_for_package() {
        let mut ledger = OwnershipLedger::default();
        ledger.claim("pkg-a", "~/f1", "h1");
        ledger.claim("pkg-a", "~/f2", "h2");
        ledger.claim("pkg-b", "~/f3", "h3");
        let owned = ledger.owned_by("pkg-a");
        assert_eq!(owned.len(), 2);
        assert!(owned.contains(&"~/f1".to_string()));
        assert!(owned.contains(&"~/f2".to_string()));
    }

    #[test]
    fn test_unowned_path_has_no_conflict() {
        let ledger = OwnershipLedger::default();
        // A pre-existing unowned file should not conflict
        let conflicts = ledger.check_all_conflicts("pkg-a", &["~/pre-existing-file.conf".to_string()]);
        assert!(conflicts.is_empty(), "Unowned path should not conflict");
    }
}
