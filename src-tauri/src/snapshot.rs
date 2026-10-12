pub use ryzora_core::snapshot::*;
use std::fs;

#[tauri::command]
pub fn create_snapshot(label: String, paths: Vec<String>) -> Result<SnapshotMetadata, String> {
    let home = get_home_dir();
    let root = get_ryzora_snapshots_dir();
    fs::create_dir_all(&root).map_err(|e| format!("Cannot create snapshots directory: {}", e))?;
    create_snapshot_in(&label, &paths, &home, &root)
}

#[tauri::command]
pub fn list_snapshots() -> Result<Vec<SnapshotMetadata>, String> {
    list_snapshots_in(&get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn get_snapshot(id: String) -> Result<SnapshotMetadata, String> {
    get_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn verify_snapshot(id: String) -> Result<bool, String> {
    verify_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn delete_snapshot(id: String) -> Result<(), String> {
    delete_snapshot_in(&id, &get_ryzora_snapshots_dir())
}

#[tauri::command]
pub fn restore_snapshot(id: String) -> Result<RestoreResult, String> {
    let home = get_home_dir();
    let root = get_ryzora_snapshots_dir();
    restore_snapshot_in(&id, &home, &root)
}
