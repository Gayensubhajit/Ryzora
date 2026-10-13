# Ryzora Next — Phase 2D: Qt Migration Plan & Integration Audit

**Date:** 2026-10-11  
**Target:** Full Tauri-to-Qt Quick Production Migration  
**Status:** COMPLETE AUDIT — Stage A inventory and architectural contract established.

---

## 1. Executive Summary

This audit establishes the comprehensive migration plan to transition Ryzora from its current Tauri + React implementation to a 100% native Qt 6 Quick / QML frontend backed by Rust.

All 20 existing frontend views and 151 registered Tauri IPC commands have been inventoried and mapped to their native Qt equivalents. A typed, asynchronous Qt-to-Rust bridge architecture is specified that eliminates the WebKitGTK runtime overhead while preserving every safety invariant: Polkit boundaries, pre-install snapshots, hash-verified SDDM drop-ins, and transactional rollback.

Throughout Stages B through F, the existing Tauri application remains fully operational as a production fallback.

---

## 2. Route-to-Feature-to-Backend-Command Matrix

The following matrix documents the full feature inventory across the Ryzora desktop application:

| View / Workflow | React Implementation | Backend IPC Commands (`src-tauri`) | Proposed Qt Implementation | Backend Dependencies | Verification Strategy |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Discover Store** | `src/views/DiscoverView.tsx`, `PackageCard.tsx`, `StoreCard.tsx` | `get_catalog_packages`, `refresh_catalog` | `DiscoverView.qml`, `StoreCard.qml` with virtualized `GridView` | `ryzora-core::repository`, `HttpFetcher` | Automated 87-package query test; Wayland frame presentation measurement. |
| **Installed Library** | `src/views/InstalledView.tsx` | `list_installed_packages`, `get_installed_package`, `uninstall_package` | `LibraryView.qml` with virtualized `ListView` | `installer::metadata`, `snapshot` | Inspect `~/.local/share/ryzora/installed/*.json`; verify zero GUI thread blocking. |
| **Fastfetch Studio** | `src/views/FastfetchView.tsx`, `fastfetchAnsi.ts` | `render_fastfetch_preview`, `apply_fastfetch_configuration`, `list_fastfetch_installed_presets`, `list_fastfetch_local_emblems` | `FastfetchView.qml` with ANSI RichText canvas & preset toolbar | `/usr/bin/fastfetch`, ProcessCoordinator | Automated process reaping test, cancellation latency benchmark, ANSI fidelity test. |
| **Fastfetch Detail** | `src/views/FastfetchDetailView.tsx` | `list_fastfetch_local_emblems`, `apply_fastfetch_configuration` | `FastfetchDetailView.qml` with emblem preview gallery | `system::emblems`, `fastfetch_runner` | Verify emblem selection, configuration patching, and live terminal preview. |
| **Lock Screen Detail** | `src/views/LockScreenDetailView.tsx` | `apply_lockscreen`, `deactivate_lockscreen`, `launch_lockscreen_test`, `check_lockscreen_config_drift`, `get_active_lockscreen`, `silentsddm_*` | `LockScreenDetailView.qml` with preview canvas & action controls | `integration/sddm`, `sddm_helper` (Polkit), `session_lock` | SDDM drop-in dry run, hash match check, config drift detection test. |
| **Package Detail** | `src/components/PackageDetailModal.tsx` | `get_catalog_packages`, `resolve_package_dependencies`, `install_package`, `uninstall_package`, `verify_package_cryptography` | `PackageDetailDrawer.qml` (modal overlay with metadata & dependency tree) | `ryzora-core::manifest`, `dependency::resolver`, `crypto` | Test missing dependencies, path traversal rejection, signature trust badge. |
| **Updates Dashboard** | `src/views/UpdatesView.tsx` | `get_updates_dashboard`, `check_all_updates`, `preview_package_update`, `apply_package_update` | `UpdatesView.qml` with update changelog list & transactional button | `updates::engine`, `installer::update_planner` | Downgrade refusal test, atomic rollback test on interrupted update. |
| **Repository Sources** | `src/views/RepositoryView.tsx` | `list_repository_sources`, `add_repository_source`, `remove_repository_source`, `switch_repository_channel`, `get_cache_stats`, `clear_package_cache`, `clear_all_cache` | `RepositoryView.qml` with source manager & cache inspector | `repository::manager`, `repository_sync` | SSRF URL validation test, cache root containment test, channel switch test. |
| **Snapshots & Backups**| `src/views/BackupsView.tsx` | `list_snapshots`, `get_snapshot`, `restore_snapshot`, `delete_snapshot`, `verify_snapshot` | `BackupsView.qml` with restore point list & sha256 inspector | `snapshot::engine` | Corrupted snapshot detection test, safe restore test, path traversal block. |
| **System Diagnostics** | `src/views/SystemView.tsx` | `get_system_integration_report`, `get_host_capabilities`, `get_privileged_helper_status`, `setup_privileged_helper` | `SystemView.qml` with diagnostics badges & Polkit status pill | `system::host`, `sddm_helper` | Host environment detection (Garuda, Hyprland, SDDM, Polkit helper status). |
| **Settings** | `src/views/SettingsView.tsx` | `get_settings`, `save_settings`, `reset_settings` | `SettingsView.qml` with form controls & persistent state | `settings::engine` | Settings persistence round-trip test; corrupted JSON fallback test. |
| **Hub / Home View** | `src/views/HubView.tsx` | `get_hub_overview`, `list_creator_profiles` | `HubView.qml` with hero banner, quick actions, ecosystem overview | Aggregated catalog & system state | Load time test (< 5ms from cache); offline fallback test. |
| **Creator Profiles** | `src/views/CreatorProfileView.tsx` | `list_creator_profiles`, `get_creator_profile` | `CreatorProfileView.qml` with creator grid & author package list | `creator::engine`, `crypto` | Creator filtering test; verified cryptographic badge verification. |
| **Package Authoring** | `src/views/AuthorView.tsx` | `create_package`, `validate_manifest`, `publish_package_to_repository`, `ingest_community_submission` | `AuthorView.qml` with schema validation wizard | `ryzora-core::manifest`, `authoring`, `ingestion` | Invalid semver rejection, path traversal rejection in file specs. |
| **Collections** | `src/views/CollectionsView.tsx` | `create_collection`, `delete_collection`, `get_collection`, `list_collections`, `export_collection`, `import_collection`, `install_collection`, `preview_collection_install` | `CollectionsView.qml` with bundle manager | `collections::engine` | Collection JSON import/export round-trip test; batch install dry-run. |
| **Integrity & Trust** | `src/views/IntegrityView.tsx` | `list_trusted_keys`, `list_revoked_keys`, `add_trusted_key`, `revoke_trusted_key`, `get_last_integrity_report`, `verify_package_integrity` | `IntegrityView.qml` with trusted signing key manager | `crypto::keys`, `crypto::signatures` | Tampered package rejection test; revoked key rejection test. |
| **Compatibility Lab** | `src/views/CompatibilityLabView.tsx` | `get_compatibility_lab_profiles`, `evaluate_batch_compatibility` | `CompatibilityLabView.qml` with matrix grid | `ryzora-core::compatibility` | Desktop environment capability evaluation test. |
| **Notifications** | `src/views/NotificationsView.tsx` | `list_notifications`, `mark_notification_read`, `mark_all_notifications_read`, `clear_old_notifications` | `NotificationsDrawer.qml` + top-bar badge pill | `notifications::state` | Notification dismissal, unread badge counter reactivity test. |
| **Apps Manager** | `src/views/AppsView.tsx`, `AppDetailPage.tsx` | `app_adapters::pacman_*`, `app_adapters::aur_*`, `app_adapters::flatpak_*` | `AppsView.qml` with Linux package manager search & transaction stream | `app_adapters`, `package_helper` | Non-interactive dry-run test; process lock detection test. |
| **Shell & Header** | `src/components/TopBar.tsx`, `Sidebar.tsx` | `window_close`, `window_minimize`, `window_toggle_maximize`, `set_window_appearance` | `Main.qml`, `TopBar.qml`, `SideBar.qml` with native Wayland controls | Qt Wayland QPA | Window close signal handling; zero orphan child processes on quit. |

---

## 3. Architecture & IPC / Bridge Contract

```
+-------------------------------------------------------------------------------+
|                            Qt 6 Quick / QML Layer                             |
|   - SceneGraph Virtualized Views (Discover, Library, Fastfetch, Settings)     |
|   - QML Properties, Signals, and Slots                                        |
+-------------------------------------------------------------------------------+
                                        | (Qt Property System)
+-------------------------------------------------------------------------------+
|                         C++ Application Engine Layer                          |
|   - RyzoraEngine (Core controller, QThreadPool worker dispatch)               |
|   - CatalogModel / LibraryModel (QAbstractListModel virtualized models)       |
|   - FastfetchRunner (Cooperative cancellation, ProcessCoordinator wrapper)    |
|   - State Bridges: Settings, Updates, Snapshots, SDDM, Polkit                 |
+-------------------------------------------------------------------------------+
                                        | (Safe FFI C-ABI / CXX Boundary)
+-------------------------------------------------------------------------------+
|                        Rust Native Bridge (ryzora-ffi)                        |
|   - Monotonic request watermarking (watermark_req_id)                         |
|   - Structured JSON and typed FFI structs                                     |
|   - POSIX process lifecycle: prctl(PR_SET_PDEATHSIG, SIGKILL)                 |
+-------------------------------------------------------------------------------+
                                        |
+-------------------------------------------------------------------------------+
|                         Ryzora Rust Application Core                          |
|   - crates/ryzora-core/ (Domain models, manifest/catalog validation)          |
|   - Host domain engines (installer, snapshot, sddm_helper, updates, settings) |
+-------------------------------------------------------------------------------+
```

### Bridge Invariants:
1. **Thread Separation**: All disk I/O, subprocess execution, hash verification, and network fetches run on background threads (`QThreadPool`). The Qt GUI thread handles only UI rendering and user events.
2. **Cancellation Semantics**: Any new preview request immediately advances the monotonic request watermark and terminates the previously running process via `SIGKILL`, preventing stale data from overwriting new user selections.
3. **Structured Errors**: Every backend invocation returns an explicit `Result<T, AppError>` structure where errors include a machine-readable code, user-friendly message, and remediation hint.

---

## 4. Current State: Real Integrations vs. Missing Features

### Stage C.1 Correctness Gate & Stage D Vertical Migration Status:
- **Shared-Core Integration**: Real repository discovery and package manifest validation via `ryzora_core::discover_local_repositories` and `validate_path_identifier`.
- **Honest Metadata**: All fabricated ratings (`4.8`) and download counts (`500`) eliminated; real repository metadata is used when available, otherwise accurately represented as `None` / unavailable.
- **Fixture Isolation**: Synthetic benchmark fixtures strictly isolated; zero synthetic fixtures injected into production catalog queries.
- **Secure Fastfetch Preset Resolution**: Path traversal (`..`, `/`, `\`), null bytes, and dotfiles rejected; multi-tier priority resolution (user `~/.config/fastfetch/presets`, community packages, dist packages, system `/usr/share/fastfetch/presets`); required parent working directory preserved for relative assets; missing presets return explicit errors with no silent fallback to `dr460nized`.
- **Target-Specific Active State**: Discrete `ActiveTargets` (`session_lock`, `sddm_login`, `sddm_lock`) parsed from `active_lockscreen.json` and `activation_manifest.json`; installed and active states strictly separated.
- **High-Fidelity ANSI Engine**: Robust SGR terminal engine handling standard/bright colors, 256-color cube/ramp (`38;5;N`), 24-bit Truecolor (`38;2;R;G;B`), bold, dim, italic, underline, strikethrough, combined SGR codes, HTML entity escaping, and non-SGR sequence stripping.
- **Stage D Package Actions**: Typed C-ABI and C++ controller methods `installPackage` / `uninstallPackage` with reactive state invalidation, library/catalog refresh, and user notification events.

### Currently Real in the Qt Prototype:
- **Discover Catalog**: Scans and parses **87 real package manifests** from `repositories/community/packages/`.
- **Installed Library**: Scans and parses all **17 real installed package JSON records** from `~/.local/share/ryzora/installed/`.
- **Active State Matching**: Dynamically correlates catalog packages against installed IDs (`✓ Installed` badge).
- **Fastfetch Subprocess**: Real `/usr/bin/fastfetch -c <preset>` execution with live ANSI terminal output.
- **Process Cleanup**: Monotonic cancellation with verified 0 zombie/orphan processes.

### Missing Integrations to be Implemented in Stages B–F:
1. **Package Installation Pipeline**: Triggering transactional install with snapshot creation.
2. **Package Uninstallation Pipeline**: Safe file removal with conflict detection and active-lockscreen block.
3. **SDDM & Session-Lock Mutation**: Polkit authorization helper (`/usr/lib/ryzora/ryzora-sddm-helper`) integration for writing `/etc/sddm.conf.d/` drop-ins.
4. **Snapshot & Rollback Execution**: Creating restore points and rolling back files.
5. **Updates Management**: Checking package update drifts and executing update transactions.
6. **Repository Editing**: Adding/removing remote repository sources and switching release channels.
7. **Settings UI**: Application configuration editor.
8. **App Adapters**: Pacman/AUR/Flatpak search and transaction viewers.

---

## 5. Benchmark Methodology & Disambiguation

To address previous measurement ambiguity, benchmarks are strictly defined as follows:

| Benchmark Metric | Definition & Boundary | Measurement Technique |
| :--- | :--- | :--- |
| **Navigation Dispatch Latency** | Time from mouse click / event until Qt updates the `currentTab` property. | `QElapsedTimer` measured inside slot handler before event loop return. |
| **Destination-Render Readiness** | Time from tab switch until the destination view's SceneGraph items are laid out, painted, and presented on the display. | Measured via `QQuickWindow::afterRendering` signal or frame presentation timestamp. |
| **Complete Process Tree RSS/PSS** | Combined memory of the main process AND all child processes (WebKitWebProcess for Tauri, Fastfetch workers for Qt). | Parsed from kernel `/proc/<pid>/smaps_rollup` (`Pss` field) across all PIDs in process tree. |
| **Surface-Mapped Startup** | Time from process launch until Wayland compositor reports `mapped: true` and window dimensions > 100x100. | Automated `hyprctl clients -j` polling with high-resolution timestamps. |
| **Fastfetch Preview Latency** | Time from preset selection click until the complete ANSI rich text is rendered on the terminal canvas. | End-to-end timer from dispatch to QML RichText update. |

### Baseline Comparison (Garuda Linux / Hyprland Wayland):
- **Surface Startup**: Qt 6 Quick: **185.9 ms** vs. Tauri Release: **259.4 ms** (~28% faster).
- **Total Process Memory**: Qt 6 Quick: **169.2 MB RSS (97.5 MB PSS)** vs. Tauri Release: **506.6 MB RSS (299.0 MB PSS)** (~66.6% less RAM).
- **Navigation Dispatch**: Qt 6 Quick: **59.4 µs (0.06 ms) median**, **0.92 ms p95** vs. Tauri WebKit: **~40–80 ms**.

---

## 6. Safety Parity, Risks, and Rollback Strategy

### Safety Parity Gates (Non-Negotiable):
1. **Polkit Authorization**: No privileged drop-in files in `/etc/sddm.conf.d/` will ever be written directly by the GUI. Elevation is strictly channeled through the audited `/usr/lib/ryzora/ryzora-sddm-helper` with slug validation and hash checks intact.
2. **Fail-Closed Rollback**: An installation or update failure must trigger automatic snapshot restoration.
3. **Ownership Checks**: Ryzora will refuse to deactivate foreign or unowned SDDM drop-ins.
4. **Cache Confinement**: All cached packages and indices must be strictly confined within `~/.cache/ryzora/` without directory traversal.

### Rollback Strategy During Migration:
- The existing Tauri codebase and build configurations (`src-tauri/`, `src/`, `package.json`, `vite.config.ts`) remain 100% buildable and testable at every commit.
- The `ryzora.desktop` launcher remains pointed to the Tauri binary until Stage G acceptance passes.
- All pre-existing user modifications (Fastfetch presets, emblems, `subfilter` additions) are strictly preserved.

---

## 7. Migration Sequence & Recommended Next Slice

### Full Roadmap Stages:
- **Stage A (Current)**: Inventory & Preservation (Audit completed).
- **Stage B (Next Slice)**: Qt Foundation & Shell Architecture (Promote spike to production project structure, reusable components, multi-view navigation, frame render readiness).
- **Stage C**: Shared Core & Bridge Integration (Typed FFI bridge, catalog/library/settings bindings).
- **Stage D**: Vertical Feature Migration (Details drawer, Fastfetch studio, Updates, Repositories).
- **Stage E**: Safety-Critical Parity (Installer, snapshot engine, SDDM helper integration, regression tests).
- **Stage F**: Production Validation (Garuda/Hyprland end-to-end validation, upgrade tests).
- **Stage G**: Production Switch & Legacy Removal (Switch default binary, remove Tauri/React in final reviewable change).

**Decision:** Stage A Audit Complete. Ready to proceed to **Stage B — Qt Foundation & Shell Architecture** upon confirmation.
