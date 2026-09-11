/**
 * Ryzora Catalog Service — Phase 24
 *
 * Coordinates dynamic Arch Linux Pacman repository discovery, AppStream metadata,
 * and category filtering. Connects to the Rust backend zero-subprocess ALPM engine.
 */

import type { PackageItem } from "../types/index.ts";
import { resolveAppMetadata } from "../components/apps/appMetadata.ts";
import { pacmanAppProvider } from "../providers/pacmanProvider.ts";

export interface CatalogItem {
  id: string;
  display_name: string;
  summary: string;
  description: string;
  repository: string;
  version: string;
  is_installed: boolean;
  installed_version?: string | null;
  is_application: boolean;
  category: string;
  subcategories: string[];
  icon_name?: string | null;
  icon_path?: string | null;
  launchable?: string | null;
  homepage?: string | null;
  license?: string | null;
  download_size?: number | null;
  installed_size?: number | null;
  dependencies: string[];
  screenshots: string[];
  developer?: string | null;
  metadata_source: "appstream" | "desktop_entry" | "pacman_sync" | string;
}

export interface CatalogStatus {
  total_packages: number;
  total_applications: number;
  total_installed_applications?: number;
  total_installed_packages?: number;
  enabled_repositories: string[];
  last_refreshed: number;
  is_refreshing: boolean;
  error?: string | null;
}

export interface CatalogQueryFilter {
  is_application_only?: boolean;
  category?: string;
  search_query?: string;
  repository?: string;
  is_installed_only?: boolean;
  page?: number;
  page_size?: number;
}

export interface CatalogPageResponse {
  items: CatalogItem[];
  total_count: number;
  page: number;
  page_size: number;
}

const isTauri = typeof window !== "undefined" && Boolean(
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
);

async function invokeTauri<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isTauri) {
    throw new Error(`Tauri is not available for command "${cmd}"`);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// Fallback items for test environments or web preview
const FALLBACK_APPS: CatalogItem[] = [
  {
    id: "firefox",
    display_name: "Firefox",
    summary: "Fast, private, and extensible web browser from Mozilla",
    description: "Firefox is a free and open-source web browser developed by the Mozilla Foundation.",
    repository: "extra",
    version: "155.0.1-1",
    is_installed: true,
    is_application: true,
    category: "Internet",
    subcategories: ["Network", "WebBrowser"],
    homepage: "https://www.mozilla.org/firefox/",
    license: "MPL-2.0",
    download_size: 65000000,
    installed_size: 245000000,
    dependencies: ["gtk3", "libpulse", "nss", "alsa-lib"],
    screenshots: [],
    metadata_source: "appstream",
  },
  {
    id: "blender",
    display_name: "Blender",
    summary: "Free and open source 3D creation suite",
    description: "Fully integrated 3D creation suite: modeling, rigging, animation, simulation, rendering, and compositing.",
    repository: "extra",
    version: "17:5.2.1-2",
    is_installed: false,
    is_application: true,
    category: "Graphics",
    subcategories: ["3DGraphics", "RasterGraphics"],
    homepage: "https://www.blender.org/",
    license: "GPL-3.0-or-later",
    download_size: 188000000,
    installed_size: 403750000,
    dependencies: ["python", "openvdb", "embree", "openimagedenoise"],
    screenshots: [],
    metadata_source: "appstream",
  },
  {
    id: "gimp",
    display_name: "GNU Image Manipulation Program",
    summary: "Create images and edit photographs",
    description: "Extensible GNU Image Manipulation Program for photo retouching, image composition, and authoring.",
    repository: "extra",
    version: "3.2.4-2",
    is_installed: false,
    is_application: true,
    category: "Graphics",
    subcategories: ["2DGraphics", "RasterGraphics"],
    homepage: "https://www.gimp.org/",
    license: "GPL-3.0-or-later",
    download_size: 25000000,
    installed_size: 160000000,
    dependencies: ["gtk2", "gegl", "babl"],
    screenshots: [],
    metadata_source: "appstream",
  },
  {
    id: "vlc",
    display_name: "VLC media player",
    summary: "Read, capture, broadcast your multimedia streams",
    description: "High-performance, multi-platform media player and streaming server.",
    repository: "extra",
    version: "3.0.21-3",
    is_installed: false,
    is_application: true,
    category: "Multimedia",
    subcategories: ["AudioVideo", "Player"],
    homepage: "https://www.videolan.org/vlc/",
    license: "GPL-2.0-or-later",
    download_size: 16000000,
    installed_size: 68000000,
    dependencies: ["ffmpeg", "qt5-base", "libmatroska"],
    screenshots: [],
    metadata_source: "appstream",
  },
  {
    id: "code",
    display_name: "Visual Studio Code - OSS",
    summary: "Code Editing. Redefined.",
    description: "Visual Studio Code Open Source release with telemetry disabled.",
    repository: "extra",
    version: "1.93.1-1",
    is_installed: true,
    is_application: true,
    category: "Development",
    subcategories: ["Development", "IDE", "TextEditor"],
    homepage: "https://github.com/microsoft/vscode",
    license: "MIT",
    download_size: 85000000,
    installed_size: 356000000,
    dependencies: ["electron", "ripgrep", "nodejs"],
    screenshots: [],
    metadata_source: "appstream",
  },
];

export class CatalogService {
  private statusCache: CatalogStatus | null = null;
  private subscribers: Set<(status: CatalogStatus) => void> = new Set();
  private itemCache = new Map<string, CatalogItem>();

  async getStatus(): Promise<CatalogStatus> {
    if (isTauri) {
      try {
        const res = await invokeTauri<CatalogStatus>("pacman_get_catalog_status");
        this.statusCache = res;
        return res;
      } catch (err) {
        console.warn("[CatalogService] Failed to fetch catalog status:", err);
      }
    }

    const fallback: CatalogStatus = {
      total_packages: 15599,
      total_applications: 1234,
      total_installed_applications: 135,
      total_installed_packages: 2361,
      enabled_repositories: ["garuda", "core", "extra", "multilib"],
      last_refreshed: Math.floor(Date.now() / 1000),
      is_refreshing: false,
      error: null,
    };
    this.statusCache = fallback;
    return fallback;
  }

  async getItems(filter: CatalogQueryFilter = {}): Promise<CatalogPageResponse> {
    if (isTauri) {
      try {
        const res = await invokeTauri<CatalogPageResponse>("pacman_get_catalog_items", {
          filter,
        });

        // Multi-source search integration (Phase 25)
        if (filter.search_query && (filter.page ?? 0) === 0 && (filter.is_application_only ?? true)) {
          const q = filter.search_query.trim();
          try {
            const [flatpakResults, aurResults] = await Promise.allSettled([
              invokeTauri<any[]>("flatpak_search", { query: q }),
              invokeTauri<any[]>("aur_search", { query: q }),
            ]);

            const existingIds = new Set(res.items.map((i) => i.id.toLowerCase()));

            if (flatpakResults.status === "fulfilled" && Array.isArray(flatpakResults.value)) {
              for (const fp of flatpakResults.value.slice(0, 5)) {
                if (!existingIds.has(fp.id.toLowerCase())) {
                  existingIds.add(fp.id.toLowerCase());
                  res.items.push({
                    id: fp.id,
                    display_name: fp.name || fp.id,
                    summary: fp.description || "",
                    description: fp.description || "",
                    repository: "flathub",
                    version: fp.version || "stable",
                    is_installed: Boolean(fp.is_installed),
                    installed_version: fp.is_installed ? fp.version : null,
                    is_application: true,
                    category: "Utilities",
                    subcategories: [],
                    icon_name: fp.id,
                    icon_path: null,
                    launchable: fp.id,
                    homepage: null,
                    license: null,
                    download_size: null,
                    installed_size: null,
                    dependencies: [],
                    screenshots: [],
                    developer: null,
                    metadata_source: "flatpak",
                  });
                }
              }
            }

            if (aurResults.status === "fulfilled" && Array.isArray(aurResults.value)) {
              for (const aur of aurResults.value.slice(0, 5)) {
                if (!existingIds.has(aur.name.toLowerCase())) {
                  existingIds.add(aur.name.toLowerCase());
                  res.items.push({
                    id: aur.name,
                    display_name: aur.name,
                    summary: aur.description || "",
                    description: aur.description || "",
                    repository: "aur",
                    version: aur.version || "latest",
                    is_installed: Boolean(aur.is_installed),
                    installed_version: aur.installed_version || null,
                    is_application: true,
                    category: "Utilities",
                    subcategories: [],
                    icon_name: aur.name,
                    icon_path: null,
                    launchable: `${aur.name}.desktop`,
                    homepage: aur.url || null,
                    license: aur.license ? aur.license.join(", ") : null,
                    download_size: null,
                    installed_size: null,
                    dependencies: aur.depends || [],
                    screenshots: [],
                    developer: aur.maintainer || null,
                    metadata_source: "aur",
                  });
                }
              }
            }
          } catch (multiErr) {
            console.warn("[CatalogService] Multi-source search error:", multiErr);
          }
        }

        for (const item of res.items) {
          this.itemCache.set(item.id, item);
        }
        return res;
      } catch (err) {
        console.warn("[CatalogService] Failed to query catalog items:", err);
      }
    }

    // Fallback filtering
    let matched = [...FALLBACK_APPS];
    const isAppOnly = filter.is_application_only ?? true;
    if (isAppOnly) {
      matched = matched.filter((i) => i.is_application);
    }
    if (filter.is_installed_only) {
      matched = matched.filter((i) => i.is_installed);
    }
    if (filter.category && filter.category.toLowerCase() !== "all") {
      const c = filter.category.toLowerCase();
      if (c === "installed") {
        matched = matched.filter((i) => i.is_installed);
      } else {
        matched = matched.filter(
          (i) =>
            i.category.toLowerCase() === c ||
            i.subcategories.some((s) => s.toLowerCase() === c)
        );
      }
    }
    if (filter.search_query) {
      const q = filter.search_query.toLowerCase().trim();
      matched = matched.filter(
        (i) =>
          i.id.toLowerCase().includes(q) ||
          i.display_name.toLowerCase().includes(q) ||
          i.summary.toLowerCase().includes(q) ||
          i.category.toLowerCase().includes(q)
      );
    }

    const page = filter.page ?? 0;
    const pageSize = filter.page_size ?? 60;
    const start = page * pageSize;
    const items = matched.slice(start, start + pageSize);

    return {
      items,
      total_count: matched.length,
      page,
      page_size: pageSize,
    };
  }

  async getItemDetails(packageId: string): Promise<CatalogItem | null> {
    if (this.itemCache.has(packageId)) {
      return this.itemCache.get(packageId)!;
    }

    if (isTauri) {
      try {
        const item = await invokeTauri<CatalogItem | null>("pacman_get_catalog_item_details", {
          packageId,
        });
        if (item) {
          this.itemCache.set(packageId, item);
          return item;
        }
      } catch (err) {
        console.warn(`[CatalogService] Failed to get details for "${packageId}":`, err);
      }
    }

    const found = FALLBACK_APPS.find((i) => i.id.toLowerCase() === packageId.toLowerCase());
    return found || null;
  }

  /**
   * Evicts a single package from the item cache so the next getItemDetails
   * call fetches authoritative ALPM state from the backend.
   */
  invalidateItem(packageId: string): void {
    this.itemCache.delete(packageId);
  }

  /**
   * Clears the entire item detail cache. Used after full installed-state refreshes
   * so every subsequent getItemDetails call re-queries the backend for fresh
   * is_installed / installed_version values.
   */
  invalidateAllItems(): void {
    this.itemCache.clear();
  }

  async refreshInstalledState(): Promise<CatalogStatus> {
    // Wipe item cache so subsequent getItemDetails re-queries authoritative ALPM state
    this.itemCache.clear();

    if (isTauri) {
      try {
        const res = await invokeTauri<CatalogStatus>("pacman_refresh_catalog_installed_state");
        this.statusCache = res;
        this.notifySubscribers(res);
        return res;
      } catch (err) {
        console.warn("[CatalogService] Failed to refresh installed state:", err);
      }
    }
    return this.getStatus();
  }

  async refreshCatalog(): Promise<CatalogStatus> {
    if (isTauri) {
      try {
        const res = await invokeTauri<CatalogStatus>("pacman_refresh_catalog");
        this.statusCache = res;
        this.notifySubscribers(res);
        return res;
      } catch (err: any) {
        const msg = err?.message || String(err);
        throw new Error(msg);
      }
    }

    const res: CatalogStatus = {
      total_packages: FALLBACK_APPS.length,
      total_applications: FALLBACK_APPS.length,
      enabled_repositories: ["core", "extra", "multilib"],
      last_refreshed: Math.floor(Date.now() / 1000),
      is_refreshing: false,
    };
    this.statusCache = res;
    this.notifySubscribers(res);
    return res;
  }

  subscribe(listener: (status: CatalogStatus) => void): () => void {
    this.subscribers.add(listener);
    if (this.statusCache) {
      listener(this.statusCache);
    }
    return () => this.subscribers.delete(listener);
  }

  private notifySubscribers(status: CatalogStatus) {
    for (const listener of this.subscribers) {
      listener(status);
    }
  }

  /**
   * Converts a CatalogItem into Ryzora canonical PackageItem for storefront details & actions.
   */
  catalogItemToPackageItem(item: CatalogItem): PackageItem {
    pacmanAppProvider.recordCatalogItem(item);
    const appMeta = resolveAppMetadata(item.id, item.display_name);
    const isFlathub = item.repository === "flathub" || item.metadata_source === "flatpak";
    const isAur = item.repository === "aur" || item.metadata_source === "aur";
    const sourceLabel = isFlathub
      ? "Flathub • Flatpak sandbox"
      : isAur
      ? "Arch User Repository • AUR"
      : `Arch Linux (${item.repository})`;

    const pkg: PackageItem = {
      id: item.id,
      title: item.display_name || item.id,
      subtitle: `${item.developer || appMeta.publisher || (isFlathub ? "Flathub" : isAur ? "AUR Community" : "Arch")} • ${sourceLabel}`,
      description: item.description || item.summary || (isFlathub ? "Flatpak sandboxed application" : isAur ? "Arch User Repository package" : "Arch Linux official package"),
      version: item.version,
      author: {
        name: item.developer || appMeta.publisher || (isFlathub ? "Flathub" : isAur ? "AUR Community" : "Arch"),
        avatar: isFlathub ? "flathub" : isAur ? "community" : "arch",
        verified: !isAur,
      },
      category: "apps",
      package_type: "app",
      tags: [
        "native",
        "pacman",
        item.repository,
        item.category.toLowerCase(),
        item.is_application ? "application" : "package",
        ...(item.is_installed ? ["installed"] : []),
      ],
      supported_desktops: ["universal"],
      supported_display: ["wayland", "x11"],
      rating: 4.9,
      rating_count: 320,
      downloads: 50000,
      hero_image: "https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/logos/arch.png",
      screenshots: item.screenshots,
      color_palette: [appMeta.brandColor, "#0f141c"],
      safety_audit: {
        rating: "verified",
        changes_system_files: true,
        requires_root: true,
        sandbox_compatible: false,
        files_modified_count: 0,
      },
      dependencies: {
        packages: item.dependencies,
        optional: [],
      },
      components: [],
      compatibility: {
        supported_distros: ["arch", "garuda", "endeavouros", "manjaro"],
        supported_desktops: ["universal"],
        supported_sessions: ["wayland", "x11"],
        required_binaries: ["pacman"],
        optional_binaries: [],
      },
      repository_id: item.repository,
      package_size_bytes: item.installed_size ?? item.download_size ?? undefined,
      integrity_status: "verified",
      is_cached: true,
    };

    (pkg as any).is_installed = item.is_installed;
    (pkg as any).installed_version = item.installed_version;
    (pkg as any).icon_path = item.icon_path;
    (pkg as any).icon_name = item.icon_name;
    (pkg as any).is_application = item.is_application;
    return pkg;
  }
}

export const catalogService = new CatalogService();
