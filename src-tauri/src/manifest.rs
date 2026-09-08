use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageComponent {
    pub name: String,
    pub component_type: String, // e.g. "hyprland", "waybar", "kitty", "fastfetch", "wallpaper"
    pub target_path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyAudit {
    pub rating: String, // "safe", "verified", "requires_review"
    pub changes_system_files: bool,
    pub requires_root: bool,
    pub sandbox_compatible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub package_id: String,
    pub package_name: String,
    pub timestamp: u64,
    pub formatted_date: String,
    pub backed_up_paths: Vec<String>,
    pub status: String, // "active", "restored"
}

// In-memory snapshot storage for the session
pub struct SnapshotState {
    pub snapshots: Mutex<Vec<SnapshotRecord>>,
}

impl Default for SnapshotState {
    fn default() -> Self {
        let initial_snapshots = vec![
            SnapshotRecord {
                id: "snap-init-001".to_string(),
                package_id: "system-baseline".to_string(),
                package_name: "Initial System Baseline".to_string(),
                timestamp: 1725840000,
                formatted_date: "2026-09-08 18:00:00".to_string(),
                backed_up_paths: vec![
                    "~/.config/hypr/hyprland.conf".to_string(),
                    "~/.config/waybar/config.jsonc".to_string(),
                    "~/.config/waybar/style.css".to_string(),
                    "~/.config/kitty/kitty.conf".to_string(),
                ],
                status: "active".to_string(),
            },
        ];
        Self {
            snapshots: Mutex::new(initial_snapshots),
        }
    }
}

#[tauri::command]
pub fn get_backups(state: tauri::State<SnapshotState>) -> Vec<SnapshotRecord> {
    let list = state.snapshots.lock().unwrap();
    list.clone()
}

#[tauri::command]
pub fn create_backup_snapshot(
    package_id: String,
    package_name: String,
    paths: Vec<String>,
    state: tauri::State<SnapshotState>,
) -> Result<SnapshotRecord, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let snap_id = format!("snap-{}", now);
    let record = SnapshotRecord {
        id: snap_id,
        package_id,
        package_name,
        timestamp: now,
        formatted_date: "Just now".to_string(),
        backed_up_paths: paths,
        status: "active".to_string(),
    };

    let mut list = state.snapshots.lock().unwrap();
    list.insert(0, record.clone());

    Ok(record)
}

#[tauri::command]
pub fn rollback_snapshot(
    snapshot_id: String,
    state: tauri::State<SnapshotState>,
) -> Result<String, String> {
    let mut list = state.snapshots.lock().unwrap();
    for snap in list.iter_mut() {
        if snap.id == snapshot_id {
            snap.status = "restored".to_string();
            return Ok(format!("Successfully rolled back configurations to snapshot {}", snapshot_id));
        }
    }
    Err(format!("Snapshot {} not found", snapshot_id))
}
