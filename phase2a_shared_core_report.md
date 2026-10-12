# Ryzora Next — Phase 2A: Shared Rust Core Extraction Report

**Date:** 2026-10-11  
**Target:** `crates/ryzora-core/`  
**Status:** PASS — Core crate created and integrated; full backward compatibility verified.

---

## 1. Executive Summary

Phase 2A initiated the production backend migration by creating a UI-independent Rust library crate (`crates/ryzora-core/`) and extracting the foundational manifest data models, dependency specifications, and schema validation logic from `src-tauri/src/`.

The existing Tauri desktop application was preserved without regression. All 617 Tauri Rust unit tests, 287 frontend test cases, TypeScript compilation, and release/debug builds continue to pass cleanly.

---

## 2. Module Dependency Inventory (`src-tauri/src/`)

An analysis of `src-tauri/src/` was conducted to categorize modules for subsequent extraction phases:

| Module | Classification | Dependencies / Invariants | Migration Phase |
| :--- | :--- | :--- | :--- |
| `manifest.rs` | **Pure Domain Logic** | Pure serde/validation; no Tauri/GUI deps (commands isolated) | **Phase 2A (Extracted)** |
| `dependency.rs` (models) | **Pure Domain Logic** | Dependency kinds, specs, spec version matching | **Phase 2A (Extracted)** |
| `crypto.rs` | **Pure Domain Logic** | SHA256 tree hashing, Ed25519 signature checks; pure std/crypto | Phase 2B candidate |
| `settings.rs` (data model) | **Pure Domain Logic** | App configuration schema, channel enums, defaults | Phase 2B candidate |
| `distribution.rs` | **Pure Domain Logic** | Repository index structures, catalog formats | Phase 2B candidate |
| `collections.rs` | **Domain Logic** | Presets, tagging, categorization data | Phase 2B candidate |
| `dependency.rs` (resolver) | **Host Adapter** | Queries `which` / system binaries via `system.rs` | Phase 2C (`ryzora-adapters`) |
| `system.rs` | **Host Adapter** | Host commands, desktop session detection, process execution | Phase 2C (`ryzora-adapters`) |
| `app_adapters/*` | **Host Adapter** | Pacman, AUR (RPC/Git), Flatpak CLI adapters | Phase 2C (`ryzora-adapters`) |
| `installer.rs` | **Host / Transaction Engine** | Atomic install/uninstall, transaction log, file conflict checks | Phase 2C/2D |
| `snapshot.rs` | **Host / Rollback Engine** | Filesystem snapshots, file restoration, rollback | Phase 2C/2D |
| `sddm_helper.rs` | **Privileged Integration** | Polkit elevation, drop-in management, hash verification | Phase 2D (Privileged) |
| `integration/sddm/*` | **Host Integration** | SDDM theme metadata, activation transactions | Phase 2D (Privileged) |
| `integration/hypridle.rs` | **Host Integration** | Hypridle/Hyprlock configuration, drop-in safety | Phase 2D (Desktop) |
| `lib.rs` | **Tauri Glue** | `tauri::Builder`, IPC command registration, `AppHandle` state | Retained for Tauri app |

---

## 3. Core Crate Layout (`crates/ryzora-core/`)

The new crate is located at `crates/ryzora-core/` and contains zero GUI, Tauri, or Qt dependencies.

### Dependencies (`Cargo.toml`)
- `serde = { version = "1.0", features = ["derive"] }`
- `serde_json = "1.0"`
- `semver = "1.0"`

### Module Structure
- `crates/ryzora-core/src/lib.rs`: Library root re-exporting domain modules and common structs.
- `crates/ryzora-core/src/dependency.rs`: Pure models `DependencyKind`, `DependencySpec`, and default helpers.
- `crates/ryzora-core/src/manifest.rs`: Pure models `PackageType`, `ManifestCompatibility`, `ManifestFile`, `TargetDefinition`, `TargetEntry`, `RyzoraManifest`, `PackageManifest`, `ManifestValidationResult`, and `validate_manifest_internal()`.

### Public API Exported
```rust
pub use dependency::{DependencyKind, DependencySpec};
pub use manifest::{
    default_true, validate_manifest_internal, ManifestCompatibility, ManifestFile,
    ManifestValidationResult, PackageManifest, PackageType, RyzoraManifest, TargetDefinition,
    TargetEntry,
};
```

---

## 4. Tauri Compatibility Integration

1. **`src-tauri/Cargo.toml`**:
   - Added dependency: `ryzora-core = { path = "../crates/ryzora-core" }`
2. **`src-tauri/src/manifest.rs`**:
   - Re-exports all domain types: `pub use ryzora_core::manifest::*;`
   - Retains thin `#[tauri::command]` adapters:
     - `validate_manifest(manifest_json: String) -> ManifestValidationResult`
     - `parse_manifest(manifest_json: String) -> Result<RyzoraManifest, String>`
3. **`src-tauri/src/dependency.rs`**:
   - Re-exports core dependency models: `pub use ryzora_core::dependency::{default_true, DependencyKind, DependencySpec};`
   - Retains system-probing resolution engine in-place until adapter extraction.

---

## 5. Verification Results

All test suites were executed across the workspace:

### 1. New Core Crate Tests (`crates/ryzora-core`)
- Command: `cargo test --manifest-path crates/ryzora-core/Cargo.toml`
- Result: **11 passed; 0 failed; 0 ignored; duration 0.00s**

### 2. Tauri Backend Unit Tests (`src-tauri`)
- Command: `cargo test --lib --manifest-path src-tauri/Cargo.toml`
- Result: **617 passed; 0 failed; 5 ignored; duration 26.95s**

### 3. Tauri Application Build (`src-tauri`)
- Command: `cargo build --manifest-path src-tauri/Cargo.toml`
- Result: **Success (exited code 0, binary produced)**

### 4. Prototype Backend Concurrency & Lifecycle Tests
- Command: `cargo test --manifest-path prototypes/qt-quick-spike/backend/Cargo.toml`
- Result: **7 passed; 0 failed; 0 ignored; duration 0.10s**

### 5. Frontend Quality Gates
- TypeScript check (`npm run typecheck`): **0 errors**
- Unit & component tests (`npm test`): **287 passed across 8 suites; 0 failed**
- Production bundle build (`npm run build`): **Success (`dist/` built in 2.32s)**

---

## 6. Safety & Protected Invariants

- **Ownership & Rollback Integrity:** `installer.rs` and `snapshot.rs` were left untouched in `src-tauri`. All transaction validation and rollback tests continue to pass (617/617).
- **SDDM & Polkit Invariants:** No changes to privileged helper paths, drop-ins, or checksum verifications.
- **Fail-Closed Behavior:** Malicious manifest path traversals, shell hook rejections, and semver validation remain fully verified by both `ryzora-core` and `src-tauri` test suites.
- **Zero Host Mutation:** No host packages, system configuration files, or user environments were modified.

---

## 7. Next Steps for Phase 2B

1. **Extract `crypto` and `distribution`:** Extract pure SHA256 tree hashing, Ed25519 signature validation, and repository metadata structures into `ryzora-core`.
2. **Extract `settings` schema:** Move configuration definitions and serialization into `ryzora-core` while keeping filesystem persistence in `ryzora-adapters` or Tauri compatibility layer.
3. **Establish `crates/ryzora-adapters/`:** Begin isolating host adapters (`pacman`, `aur`, `flatpak`, `system`) from Tauri IPC commands.
