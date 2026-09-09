pub mod authoring;
pub mod collections;
pub mod compatibility;
pub mod creator;
pub mod crypto;
pub mod dependency;
pub mod distribution;
pub mod hub;
pub mod ingestion;
pub mod installer;
pub mod integrity;
pub mod manifest;
pub mod notifications;
pub mod providers;
pub mod repository;
pub mod repository_sync;
pub mod settings;
pub mod snapshot;
pub mod system;
pub mod updates;

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
            // Ryzora Hub, Sync, Updates & Creators (Phase 14)
            hub::get_hub_overview,
            hub::get_installed_package_history,
            updates::get_updates_dashboard,
            creator::list_creator_profiles,
            creator::get_creator_profile,
            repository_sync::get_repository_sync_status,
            repository_sync::refresh_repository_sync,
            repository_sync::refresh_all_repositories_sync,
            repository_sync::switch_repository_channel,
            repository_sync::list_repository_channels,
            // Settings & Preferences (Phase 15.1)
            settings::get_settings,
            settings::save_settings,
            settings::reset_settings,
            // Notifications & Activity Log (Phase 15.2)
            notifications::list_notifications,
            notifications::mark_notification_read,
            notifications::mark_all_notifications_read,
            notifications::clear_old_notifications,
            // Collections — .ryzlist (Phase 15.4)
            collections::list_collections,
            collections::get_collection,
            collections::create_collection,
            collections::rename_collection,
            collections::delete_collection,
            collections::add_package_to_collection,
            collections::remove_package_from_collection,
            collections::export_collection,
            collections::import_collection,
            collections::preview_collection_install,
            collections::install_collection,
            // Integrity Health Dashboard (Phase 15.5)
            integrity::run_integrity_scan,
            integrity::verify_package_integrity,
            integrity::get_last_integrity_report,
            // Content Ecosystem & Providers (Phase 18)
            providers::list_content_providers,
            providers::search_content_providers,
            providers::synthesize_provider_manifest,
            providers::prepare_provider_package,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ryzora application");
}
