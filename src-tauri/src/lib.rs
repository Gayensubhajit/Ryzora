pub mod authoring;
pub mod compatibility;
pub mod crypto;
pub mod dependency;
pub mod distribution;
pub mod ingestion;
pub mod installer;
pub mod manifest;
pub mod repository;
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
            // Dependency Intelligence & Resolver (Phase 9)
            dependency::resolve_package_dependencies,
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
            // Declarative Installer (Phase 4 & 7)
            installer::preview_installation,
            installer::install_package,
            installer::list_installed_packages,
            installer::get_installed_package,
            installer::uninstall_package,
            installer::check_package_update,
            installer::check_all_updates,
            installer::preview_package_update,
            installer::apply_package_update,
            // Repository & Catalog System (Phase 5 & 6)
            repository::get_catalog_packages,
            repository::refresh_catalog,
            repository::get_repository_info,
            repository::list_repository_sources,
            repository::add_repository_source,
            repository::remove_repository_source,
            // Cache Management (Phase 8)
            repository::get_cache_stats,
            repository::clear_package_cache,
            repository::clear_all_cache,
            // Package Authoring & Store Publishing (Phase 10)
            authoring::validate_package_draft,
            authoring::create_package,
            authoring::publish_package_to_repository,
            // Store & Distribution Infrastructure (Phase 11)
            distribution::audit_store_submission,
            distribution::build_distribution_release,
            // Cryptographic Trust & Signatures (Phase 12)
            crypto::get_author_keypair,
            crypto::list_trusted_keys,
            crypto::list_revoked_keys,
            crypto::add_trusted_key,
            crypto::revoke_trusted_key,
            crypto::verify_package_cryptography,
            // Community Ingestion & CI Automation (Phase 13)
            ingestion::run_ci_submission_audit,
            ingestion::ingest_community_submission,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ryzora application");
}
