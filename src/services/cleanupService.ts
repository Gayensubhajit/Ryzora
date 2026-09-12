/**
 * cleanupService.ts — Phase 26: Safe Uninstall, Cleanup & Storage Management
 *
 * Provides authoritative inspection and safe cleanup coordination for applications
 * across Pacman, AUR, and Flathub.
 *
 * Guarantees:
 *  - Authoritative byte sizes from ALPM / Pacman / Flatpak
 *  - Real unused dependencies via native solver (pacman -Rsp preview, -Rs execution)
 *  - Exact package cache archive matching
 *  - Strict safety: never touches user documents (~/Documents, ~/Projects, etc.)
 */

import { isTauri, invokeTauri } from "./tauri.ts";

export interface UnusedDependencyItem {
  name: string;
  size_bytes: number;
}

export interface PackageCacheItem {
  filename: string;
  size_bytes: number;
}

export interface AppCleanupPreview {
  package_id: string;
  provider_id: string;
  display_name: string;
  is_installed: boolean;
  installed_package_bytes?: number | null;
  unused_dependencies: UnusedDependencyItem[];
  total_unused_dependencies_bytes: number;
  package_cache_items: PackageCacheItem[];
  total_package_cache_bytes: number;
  app_config_bytes?: number | null;
  app_config_path?: string | null;
  app_cache_bytes?: number | null;
  app_cache_path?: string | null;
  aur_build_dir_bytes?: number | null;
  aur_build_dir_path?: string | null;
  aur_cache_bytes?: number | null;
  aur_cache_path?: string | null;
  flatpak_data_bytes?: number | null;
  flatpak_data_path?: string | null;
  flatpak_runtime_info?: string | null;
  conflicts: string[];
}

export interface AppCleanupExecutionRequest {
  package_id: string;
  provider_id: string;
  remove_package: boolean;
  remove_unused_dependencies: boolean;
  clean_package_cache: boolean;
  clean_app_cache: boolean;
  clean_app_config: boolean;
  clean_aur_build: boolean;
  clean_flatpak_data: boolean;
}

export interface AppCleanupExecutionResult {
  success: boolean;
  message: string;
  package_removed: boolean;
  dependencies_removed: string[];
  cache_files_removed: number;
  total_bytes_reclaimed: number;
  errors: string[];
}

export function formatBytes(bytes?: number | null): string {
  if (bytes === undefined || bytes === null || isNaN(bytes)) {
    return "Not available";
  }
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  const val = parseFloat((bytes / Math.pow(k, i)).toFixed(i >= 2 ? 2 : 1));
  return `${val} ${sizes[i]}`;
}

export async function inspectAppCleanup(
  packageId: string,
  provider = "pacman"
): Promise<AppCleanupPreview> {
  if (!isTauri) {
    // Non-Tauri fallback for test sandboxes & web preview
    return {
      package_id: packageId,
      provider_id: provider,
      display_name: packageId,
      is_installed: true,
      installed_package_bytes: 45 * 1024 * 1024,
      unused_dependencies: [
        { name: `${packageId}-dep1`, size_bytes: 12 * 1024 * 1024 },
        { name: `${packageId}-dep2`, size_bytes: 8 * 1024 * 1024 },
      ],
      total_unused_dependencies_bytes: 20 * 1024 * 1024,
      package_cache_items: [
        { filename: `${packageId}-1.0.0-1-x86_64.pkg.tar.zst`, size_bytes: 18 * 1024 * 1024 },
      ],
      total_package_cache_bytes: 18 * 1024 * 1024,
      app_config_bytes: 512 * 1024,
      app_config_path: `~/.config/${packageId}`,
      app_cache_bytes: 2 * 1024 * 1024,
      app_cache_path: `~/.cache/${packageId}`,
      conflicts: [],
    };
  }

  return invokeTauri<AppCleanupPreview>("pacman_inspect_cleanup", {
    packageId,
    provider,
  });
}

export async function executeAppCleanup(
  request: AppCleanupExecutionRequest
): Promise<AppCleanupExecutionResult> {
  if (!isTauri) {
    return {
      success: true,
      message: `Cleaned up ${request.package_id} successfully (mock mode)`,
      package_removed: request.remove_package,
      dependencies_removed: request.remove_unused_dependencies ? ["dep1", "dep2"] : [],
      cache_files_removed: request.clean_package_cache ? 1 : 0,
      total_bytes_reclaimed: 65 * 1024 * 1024,
      errors: [],
    };
  }

  return invokeTauri<AppCleanupExecutionResult>("pacman_execute_cleanup", {
    request,
  });
}
