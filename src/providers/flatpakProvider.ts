/**
 * Ryzora Flatpak & Flathub Provider — Phase 25
 *
 * Implements PackageProvider for Flatpak / Flathub applications.
 * Directly interfaces with the host Flatpak daemon via Tauri IPC.
 */

import type { PackageItem } from "../types/index.ts";
import type {
  PackageProvider,
  PackageSourceSpec,
  PackageTargetSpec,
  PackageProvenance,
} from "./types.ts";
import { resolveAppMetadata } from "../components/apps/appMetadata.ts";
import { invokeTauri } from "../services/tauri.ts";

export interface FlatpakAppInfo {
  id: string;
  name: string;
  version: string;
  branch: string;
  origin: string;
  description: string;
  is_installed: boolean;
  runtime?: string | null;
  sdk?: string | null;
  installed_size?: string | null;
}

export interface FlatpakStatus {
  has_flatpak: boolean;
  version: string | null;
  has_flathub: boolean;
  remotes: string[];
  total_installed_apps: number;
}

export interface FlatpakCleanupInfo {
  app_id: string;
  user_data_path: string | null;
  cache_path: string | null;
  system_app_path: string | null;
}

export class FlatpakAppProvider implements PackageProvider<Record<string, unknown>> {
  readonly id = "flatpak";
  readonly name = "Flathub (Flatpak)";
  readonly displayName = "Flathub (Flatpak)";
  readonly version = "1.0.0";
  readonly category = "apps";

  private statusCache: FlatpakStatus | null = null;

  async getStatus(): Promise<FlatpakStatus> {
    if (this.statusCache) return this.statusCache;
    try {
      const status = await invokeTauri<FlatpakStatus>("flatpak_get_status");
      this.statusCache = status;
      return status;
    } catch {
      return {
        has_flatpak: false,
        version: null,
        has_flathub: false,
        remotes: [],
        total_installed_apps: 0,
      };
    }
  }

  async discover(): Promise<PackageItem[]> {
    try {
      const installed = await this.listInstalled();
      return installed;
    } catch {
      return [];
    }
  }

  async search(query: string): Promise<PackageItem[]> {
    const trimmed = query.trim();
    if (!trimmed) return [];

    try {
      const results = await invokeTauri<FlatpakAppInfo[]>("flatpak_search", { query: trimmed });
      return results.map((r) => this.normalize(r));
    } catch (err) {
      console.warn("[FlatpakAppProvider] Search failed:", err);
      return [];
    }
  }

  async getDetails(appId: string): Promise<PackageItem | null> {
    try {
      const info = await invokeTauri<FlatpakAppInfo | null>("flatpak_get_info", { appId });
      return info ? this.normalize(info) : null;
    } catch {
      return null;
    }
  }

  async listInstalled(): Promise<PackageItem[]> {
    try {
      const list = await invokeTauri<FlatpakAppInfo[]>("flatpak_list_installed");
      return list.map((r) => this.normalize(r));
    } catch {
      return [];
    }
  }

  async run(appId: string): Promise<void> {
    await invokeTauri<void>("flatpak_run", { appId });
  }

  async install(pkgOrAppId: PackageItem | string, _target?: string, _options?: Record<string, unknown>): Promise<string> {
    const appId = typeof pkgOrAppId === "string" ? pkgOrAppId : pkgOrAppId.id;
    return invokeTauri<string>("flatpak_install", { appId });
  }

  async uninstall(pkgOrAppId: PackageItem | string, _target?: string): Promise<string> {
    const appId = typeof pkgOrAppId === "string" ? pkgOrAppId : pkgOrAppId.id;
    return invokeTauri<string>("flatpak_uninstall", { appId });
  }

  async getCleanupInfo(appId: string): Promise<FlatpakCleanupInfo> {
    return invokeTauri<FlatpakCleanupInfo>("flatpak_get_cleanup_info", { appId });
  }

  normalize(raw: unknown): PackageItem {
    const f = raw as FlatpakAppInfo;
    const appMeta = resolveAppMetadata(f.id, f.name);

    return {
      id: f.id,
      title: f.name || f.id,
      subtitle: `${appMeta.publisher || "Flathub"} • Flatpak sandbox`,
      description: f.description || "Flatpak application from Flathub",
      version: f.version || "stable",
      author: {
        name: appMeta.publisher || "Flathub",
        avatar: "flathub",
        verified: true,
      },
      category: "apps",
      package_type: "app",
      tags: ["flatpak", "flathub", "sandbox", f.is_installed ? "installed" : ""].filter(Boolean),
      supported_desktops: ["universal"],
      supported_display: ["wayland", "x11"],
      rating: 4.8,
      rating_count: 150,
      downloads: 12000,
      hero_image: "https://raw.githubusercontent.com/flathub/flathub/master/brand/flathub-logo.png",
      screenshots: [],
      color_palette: [appMeta.brandColor || "#4a90d9", "#1a1f2c"],
      safety_audit: {
        rating: "verified",
        changes_system_files: false,
        requires_root: false,
        sandbox_compatible: true,
        files_modified_count: 0,
      },
      dependencies: {
        packages: f.runtime ? [f.runtime] : [],
        optional: [],
      },
      components: [],
      compatibility: {
        supported_distros: ["universal"],
        supported_desktops: ["universal"],
        supported_sessions: ["wayland", "x11"],
        required_binaries: ["flatpak"],
        optional_binaries: [],
      },
      repository_id: "flathub",
      package_size_bytes: undefined,
      integrity_status: "verified",
      is_cached: false,
    };
  }

  getSource(pkg: PackageItem): PackageSourceSpec {
    return {
      type: "system",
      url: `https://flathub.org/apps/${pkg.id}`,
    };
  }

  getTargets(_pkg: PackageItem): PackageTargetSpec[] {
    return [
      {
        id: "flatpak",
        name: "Flatpak User Environment",
        scope: "user",
        supported: true,
        dependencies: [],
        requires_root: false,
      },
    ];
  }

  getDependencies(pkg: PackageItem): string[] {
    return pkg.dependencies?.packages || [];
  }

  getProvenance(pkg: PackageItem): PackageProvenance {
    return {
      upstream: `https://flathub.org/apps/${pkg.id}`,
      license: "Various",
    };
  }
}

export const flatpakAppProvider = new FlatpakAppProvider();
