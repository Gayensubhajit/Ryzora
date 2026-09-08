pub mod compatibility;
pub mod manifest;
pub mod system;

use manifest::SnapshotState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(SnapshotState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            system::detect_system_info,
            manifest::get_backups,
            manifest::create_backup_snapshot,
            manifest::rollback_snapshot,
            compatibility::evaluate_package_compatibility,
            compatibility::evaluate_batch_compatibility,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ryzora application");
}
