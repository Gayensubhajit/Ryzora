/**
 * Ryzora AUR Provider — Phase 25
 *
 * Implements PackageProvider for Arch User Repository (AUR) packages.
 * Fast discovery via official AUR RPC (v5) and dedicated unprivileged build workflows.
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

export interface AurPackageInfo {
  name: string;
  version: string;
  description: string;
  url?: string | null;
  maintainer?: string | null;
  popularity?: number | null;
  votes?: number | null;
  out_of_date?: number | null;
  license?: string[] | null;
  depends: string[];
  make_depends: string[];
  is_installed: boolean;
  installed_version?: string | null;
}

export interface AurStatus {
  has_aur_support: boolean;
  helper: string | null;
  total_installed_foreign: number;
}

export interface AurCleanupInfo {
  package_name: string;
  build_dir: string | null;
  yay_cache: string | null;
  paru_cache: string | null;
}

export class AurAppProvider implements PackageProvider<Record<string, unknown>> {
  readonly id = "aur";
  readonly name = "Arch User Repository (AUR)";
  readonly displayName = "Arch User Repository (AUR)";
  readonly version = "1.0.0";
  readonly category = "apps";

  private statusCache: AurStatus | null = null;

  async getStatus(): Promise<AurStatus> {
    if (this.statusCache) return this.statusCache;
    try {
      const status = await invokeTauri<AurStatus>("aur_get_status");
      this.statusCache = status;
      return status;
    } catch {
      return {
        has_aur_support: false,
        helper: null,
        total_installed_foreign: 0,
      };
    }
  }

  async discover(): Promise<PackageItem[]> {
    return this.listInstalled();
  }

  async search(query: string): Promise<PackageItem[]> {
    const trimmed = query.trim();
    if (!trimmed) return [];

    try {
      const results = await invokeTauri<AurPackageInfo[]>("aur_search", { query: trimmed });
      return results.map((r) => this.normalize(r));
    } catch (err) {
      console.warn("[AurAppProvider] Search failed:", err);
      return [];
    }
  }

  async getDetails(packageName: string): Promise<PackageItem | null> {
    try {
      const info = await invokeTauri<AurPackageInfo | null>("aur_get_info", { packageName });
      return info ? this.normalize(info) : null;
    } catch {
      return null;
    }
  }

  async getPkgbuild(packageName: string): Promise<string> {
    return invokeTauri<string>("aur_get_pkgbuild", { packageName });
  }

  async listInstalled(): Promise<PackageItem[]> {
    try {
      const list = await invokeTauri<AurPackageInfo[]>("aur_list_installed");
      return list.map((r) => this.normalize(r));
    } catch {
      return [];
    }
  }

  async buildAndInstall(packageName: string): Promise<string> {
    return invokeTauri<string>("aur_install", { packageName });
  }

  async getCleanupInfo(packageName: string): Promise<AurCleanupInfo> {
    return invokeTauri<AurCleanupInfo>("aur_get_cleanup_info", { packageName });
  }

  normalize(raw: unknown): PackageItem {
    const p = raw as AurPackageInfo;
    const appMeta = resolveAppMetadata(p.name, p.name);

    return {
      id: p.name,
      title: p.name,
      subtitle: `${p.maintainer ? `by ${p.maintainer}` : "User contributed"} • AUR`,
      description: p.description || "Arch User Repository package",
      version: p.version,
      author: {
        name: p.maintainer || "AUR Community",
        avatar: "community",
        verified: false,
      },
      category: "apps",
      package_type: "app",
      tags: ["aur", "community", "user-contributed", p.is_installed ? "installed" : ""].filter(Boolean),
      supported_desktops: ["universal"],
      supported_display: ["wayland", "x11"],
      rating: 4.5,
      rating_count: p.votes || 50,
      downloads: p.popularity ? Math.round(p.popularity * 100) : 1000,
      hero_image: "https://raw.githubusercontent.com/archlinux/aur/master/web/html/images/favicon.ico",
      screenshots: [],
      color_palette: [appMeta.brandColor || "#1793d1", "#0e1726"],
      safety_audit: {
        rating: "requires_review",
        changes_system_files: true,
        requires_root: true,
        sandbox_compatible: false,
        files_modified_count: 0,
      },
      dependencies: {
        packages: p.depends || [],
        optional: p.make_depends || [],
      },
      components: [],
      compatibility: {
        supported_distros: ["arch", "garuda", "endeavouros", "manjaro"],
        supported_desktops: ["universal"],
        supported_sessions: ["wayland", "x11"],
        required_binaries: ["makepkg"],
        optional_binaries: ["yay", "paru"],
      },
      repository_id: "aur",
      package_size_bytes: undefined,
      integrity_status: "unverified",
      is_cached: false,
    };
  }

  getSource(pkg: PackageItem): PackageSourceSpec {
    return {
      type: "system",
      url: `https://aur.archlinux.org/packages/${pkg.id}`,
    };
  }

  getTargets(_pkg: PackageItem): PackageTargetSpec[] {
    return [
      {
        id: "aur",
        name: "AUR System Target",
        scope: "system",
        supported: true,
        dependencies: [],
        requires_root: true,
      },
    ];
  }

  getDependencies(pkg: PackageItem): string[] {
    return pkg.dependencies?.packages || [];
  }

  getProvenance(pkg: PackageItem): PackageProvenance {
    return {
      upstream: `https://aur.archlinux.org/packages/${pkg.id}`,
      license: "Community",
    };
  }
}

export const aurAppProvider = new AurAppProvider();
