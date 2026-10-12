# Ryzora Next — Phase 2B: Repository and Catalog Extraction Report

**Date:** 2026-10-11  
**Target:** `crates/ryzora-core/`  
**Status:** PASS — Read-only repository, catalog models, validation, discovery, and filtering extracted; full backward compatibility verified.

---

## 1. Executive Summary

Phase 2B successfully extracted the UI-independent repository and catalog data structures, schema validation routines, path and URL safety invariants, local repository discovery logic, and catalog filtering from `src-tauri/src/repository.rs` into the shared library crate `crates/ryzora-core/`.

The extraction established a clean architectural boundary between pure domain catalog logic and host/Tauri networking/IPC adapters. The existing Tauri desktop application continues to function identically without breaking IPC contracts or introducing regression. All 616 Tauri Rust unit tests, 19 core library tests, 287 frontend test cases, TypeScript compilation, and release builds pass cleanly.

---

## 2. Dependency Map & Extraction Boundaries

To prevent entanglement with host-specific dependencies (such as Tauri state, `ureq` networking, and async IPC commands), Phase 2B strictly decoupled catalog logic:

```
+-------------------------------------------------------------------------------+
|                             crates/ryzora-core/                               |
|                                                                               |
|  [distribution.rs]               [compatibility.rs]         [crypto.rs]       |
|  - ReleaseChannel                - CompatibilityRequirements- SignerIdentity  |
|  - TrustTier                                                - PackageSignature|
|  - ModerationStatus                                         - CryptoStatus    |
|                                                                               |
|  [repository.rs]                                                              |
|  - Limits: MAX_INDEX_SIZE, MAX_PACKAGE_FILES, MAX_REPO_NAME_LEN, etc.         |
|  - Models: AuthorInfo, RepositoryPackageEntry, RepositoryIndex,               |
|            FrontendPackageItem, LocalRepository, SearchResult                 |
|  - Traits: HttpFetcher, Repository                                            |
|  - Validation: validate_sha256_hash, validate_path_identifier,                |
|                validate_image_path_or_url, validate_remote_url,               |
|                resolve_repository_url, validate_redirect,                     |
|                validate_repository_package_entry                              |
|  - Discovery: get_default_repository_search_dirs, discover_local_repositories|
|  - Filtering: filter_catalog_items                                            |
+-------------------------------------------------------------------------------+
                                        ^
                                        | consumes via re-exports & traits
+-------------------------------------------------------------------------------+
|                               src-tauri/src/                                  |
|                                                                               |
|  [distribution.rs / compatibility.rs / crypto.rs / repository.rs]             |
|  - Re-exports ryzora_core domain types and validation functions               |
|  - Host Adapters: UreqHttpFetcher (implements ryzora_core::HttpFetcher)       |
|  - Remote Repository: RemoteRepository (implements ryzora_core::Repository)   |
|  - Host State: RepositoryManager, cache cleaning, thread-safe sync            |
|  - IPC Layer: #[tauri::command] handlers exposing items to frontend           |
+-------------------------------------------------------------------------------+
```

---

## 3. Files Created and Modified

### Created / Extended in `crates/ryzora-core/`:
- **`crates/ryzora-core/Cargo.toml`**: Added dependencies for catalog validation and hashing (`sha2 = "0.10"`, `url = "2"`, `percent-encoding = "2.3"`).
- **`crates/ryzora-core/src/distribution.rs`**: Extracted enum definitions for `ReleaseChannel`, `TrustTier`, and `ModerationStatus`.
- **`crates/ryzora-core/src/compatibility.rs`**: Extracted `CompatibilityRequirements` specification and version matching.
- **`crates/ryzora-core/src/crypto.rs`**: Extracted `SignerIdentity`, `PackageSignatureMetadata`, and `CryptographicStatus`.
- **`crates/ryzora-core/src/repository.rs`**: Core module containing 1,550+ lines of catalog models, strict security validators, safe cache path resolution, content tree hash calculation, local repository discovery, and unit test suite.
- **`crates/ryzora-core/src/lib.rs`**: Publicly re-exports `compatibility`, `crypto`, `dependency`, `distribution`, `manifest`, and `repository`.

### Updated in `src-tauri/src/`:
- **`src-tauri/src/distribution.rs`**: Re-exports `ReleaseChannel`, `TrustTier`, `ModerationStatus` from `ryzora_core::distribution`.
- **`src-tauri/src/compatibility.rs`**: Re-exports `CompatibilityRequirements` from `ryzora_core::compatibility`.
- **`src-tauri/src/crypto.rs`**: Re-exports `CryptographicStatus`, `PackageSignatureMetadata`, `SignerIdentity` from `ryzora_core::crypto`.
- **`src-tauri/src/repository.rs`**: Re-exports core repository models, size constants, and validators from `ryzora_core::repository`. Retains `UreqHttpFetcher`, `RemoteRepository`, `RepositoryManager`, and Tauri IPC commands.

---

## 4. Public Core API and Compatibility Approach

1. **Subfilter Invariant**:
   - The user's active branch introduced `pub subfilter: Option<String>` on `RepositoryPackageEntry` and `FrontendPackageItem` for emblem and lock-screen categorization.
   - `ryzora-core` fully preserves this field with `#[serde(default, skip_serializing_if = "Option::is_none")]`, preventing serialization mismatch across the stack.

2. **Decoupled Fetching via `HttpFetcher` Trait**:
   - `crates/ryzora-core` defines:
     ```rust
     pub trait HttpFetcher: Send + Sync {
         fn fetch_bytes(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String>;
     }
     ```
   - Allows headless unit tests and alternative frontends (e.g. Qt/Slint) to plug in custom networking backends without pulling `ureq` or Tauri runtime into core.

3. **Multi-Location Repository Discovery**:
   - `get_default_repository_search_dirs` in `ryzora-core` checks:
     1. Environment variable override (`RYZORA_REPOSITORY`).
     2. Process working directory (`current_dir()`).
     3. Development parent path (`../repositories`).
     4. Ancestor search relative to the executable (`current_exe()`).
     5. User local share (`$HOME/.local/share/ryzora/repositories`).
     6. System distribution path (`/usr/share/ryzora/repositories`).

4. **Security & Validation Invariants**:
   - Strict size boundaries (`MAX_INDEX_SIZE = 10MB`, `MAX_PACKAGE_SIZE = 100MB`, etc.).
   - Path traversal prevention via `validate_path_identifier` and `get_safe_cache_path`.
   - URL scheme allowlisting (`https://` required for remote repos, local overrides strictly checked).
   - SSRF and credential scrubbing via `validate_remote_url` and `resolve_repository_url`.
   - SHA-256 tree hash verification for package directories and cache archives.

---

## 5. Verification Results

| Test Suite | Result | Details |
| :--- | :--- | :--- |
| `crates/ryzora-core` unit tests | **19 passed** | Manifest, dependency, and catalog domain tests passed (0 failed) |
| `src-tauri` Rust test suite | **616 passed** | 616 passed, 0 failed, 5 ignored (tests in sddm integration) |
| Frontend Unit Tests (`npm test`) | **287 passed** | 287 tests passed across all store, catalog, and preview specs |
| TypeScript Check (`npm run typecheck`) | **0 errors** | Strict typing clean across frontend |
| Frontend Production Build (`npm run build`) | **PASS** | Vite built optimized bundle in 2.00s |
| Tauri Backend Release Check (`cargo check --release`) | **PASS** | Completed with clean release profile target |

---

## 6. Safety & Protected Invariants

- **Fail-Closed Validation**: Any malformed index or package failing SHA-256 verification or containing directory traversal paths is dropped immediately.
- **Cache Isolation**: Remote packages are cached using sanitized sha256 directory trees in `$CACHE_DIR/packages/`, never overwriting host root assets.
- **Rollback and Ownership Untouched**: Phase 2B did NOT touch transaction logs, snapshot engines, SDDM drop-in configs, or polkit privilege helpers.
- **Git Safety**: No uncommitted work was committed or modified destructively. All pre-existing working tree modifications (Fastfetch presets, emblems) remain intact.

---

## 7. Next Steps: Phase 2C

The next slice will tackle the **Host Adapters and Package Management Engine**:
1. Create `crates/ryzora-adapters/` (or extend `ryzora-core`) for system queries and package managers (`pacman`, `aur`, `flatpak`).
2. Extract dependency resolution logic (`dependency.rs` resolver).
3. Extract atomic transaction log and state tracking while preserving rollback guarantees.
