pub mod compatibility;
pub mod installer;
pub mod manifest;
pub mod snapshot;
pub mod system;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // System Probe
            system::detect_system_info,
            // Manifest — validation only (Phase 2)
            manifest::validate_manifest,
            manifest::parse_manifest,
            // Snapshot / Backup Engine (Phase 3)
            snapshot::create_snapshot,
            snapshot::list_snapshots,
            snapshot::get_snapshot,
            snapshot::verify_snapshot,
            snapshot::delete_snapshot,
            snapshot::restore_snapshot,
            // Compatibility Engine (Phase 1)
            compatibility::evaluate_package_compatibility,
            compatibility::evaluate_batch_compatibility,
            // Declarative Installer (Phase 4)
            installer::preview_installation,
            installer::install_package,
            installer::list_installed_packages,
            installer::get_installed_package,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ryzora application");
}
