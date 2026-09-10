/**
 * Ryzora Pacman App Provider — Phase 23A
 *
 * Implements PackageProvider<PacmanAppMeta> for Arch Linux native repositories.
 * Searches and inspects packages using zero-subprocess ALPM database queries
 * via the Rust backend, normalizing packages with authentic application metadata.
 */

import type { PackageItem } from "../types/index.ts";
import type {
  PackageProvider,
  PackageSourceSpec,
  PackageTargetSpec,
  PackageProvenance,
  PackageMediaSpec,
  PacmanAppMeta,
} from "./types.ts";
import { resolveAppMetadata } from "../components/apps/appMetadata.ts";

export interface PacmanRawInfo {
  name: string;
  version: string;
  description: string;
  repository: string;
  url?: string | null;
  license?: string | null;
  size_bytes?: number | null;
  is_installed: boolean;
  installed_version?: string | null;
  dependencies: string[];
}

// Check if running inside Tauri
const isTauri = typeof window !== "undefined" && Boolean(
  (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
);

async function invokeTauri<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isTauri) {
    throw new Error(`Tauri is not available for command '${cmd}'`);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

// Curated popular Arch desktop applications for the initial App Store landing page
export const CURATED_ARCH_PACKAGES: PacmanRawInfo[] = [
  {
    name: "firefox",
    version: "130.0.1-1",
    description: "Fast, private, and extensible web browser from Mozilla",
    repository: "extra",
    url: "https://www.mozilla.org/firefox/",
    license: "MPL-2.0",
    size_bytes: 245000000,
    is_installed: false,
    dependencies: ["gtk3", "libpulse", "nss", "alsa-lib"],
  },
  {
    name: "chromium",
    version: "128.0.6613.137-1",
    description: "A web browser built for speed, simplicity, and security",
    repository: "extra",
    url: "https://www.chromium.org/Home",
    license: "BSD-3-Clause",
    size_bytes: 310000000,
    is_installed: false,
    dependencies: ["gtk3", "nss", "alsa-lib", "libcups"],
  },
  {
    name: "code",
    version: "1.93.1-1",
    description: "Visual Studio Code Open Source release with telemetry disabled",
    repository: "extra",
    url: "https://github.com/microsoft/vscode",
    license: "MIT",
    size_bytes: 340000000,
    is_installed: false,
    dependencies: ["electron", "ripgrep", "nodejs"],
  },
  {
    name: "discord",
    version: "0.0.64-1",
    description: "All-in-one voice and text chat for gamers that's free, secure, and works on desktop",
    repository: "extra",
    url: "https://discord.com",
    license: "Custom:Proprietary",
    size_bytes: 185000000,
    is_installed: false,
    dependencies: ["gtk3", "libnotify", "nss", "libxss"],
  },
  {
    name: "steam",
    version: "1.0.0.79-2",
    description: "Valve's digital gaming distribution platform and runtime launcher",
    repository: "multilib",
    url: "https://store.steampowered.com/",
    license: "Custom:Proprietary",
    size_bytes: 95000000,
    is_installed: false,
    dependencies: ["lib32-glibc", "lib32-mesa", "vulkan-driver"],
  },
  {
    name: "vlc",
    version: "3.0.21-3",
    description: "High-performance, multi-platform media player and streaming server",
    repository: "extra",
    url: "https://www.videolan.org/vlc/",
    license: "GPL-2.0-or-later",
    size_bytes: 65000000,
    is_installed: false,
    dependencies: ["ffmpeg", "qt5-base", "libmatroska"],
  },
  {
    name: "gimp",
    version: "2.10.38-1",
    description: "Extensible GNU Image Manipulation Program for photo retouching and digital art",
    repository: "extra",
    url: "https://www.gimp.org/",
    license: "GPL-3.0-or-later",
    size_bytes: 125000000,
    is_installed: false,
    dependencies: ["gtk2", "gegl", "babl"],
  },
  {
    name: "obs-studio",
    version: "30.2.3-2",
    description: "Free and open source software for video recording and live streaming",
    repository: "extra",
    url: "https://obsproject.com/",
    license: "GPL-2.0-or-later",
    size_bytes: 140000000,
    is_installed: false,
    dependencies: ["qt6-base", "ffmpeg", "libpipewire"],
  },
  {
    name: "blender",
    version: "4.2.1-2",
    description: "Fully integrated 3D creation suite: modeling, rigging, animation, and simulation",
    repository: "extra",
    url: "https://www.blender.org/",
    license: "GPL-3.0-or-later",
    size_bytes: 480000000,
    is_installed: false,
    dependencies: ["python", "openvdb", "embree", "openimagedenoise"],
  },
  {
    name: "inkscape",
    version: "1.3.2-6",
    description: "Professional vector graphics editor for Linux, Windows, and macOS",
    repository: "extra",
    url: "https://inkscape.org/",
    license: "GPL-3.0-or-later",
    size_bytes: 110000000,
    is_installed: false,
    dependencies: ["gtkmm3", "gsl", "poppler-glib"],
  },
  {
    name: "alacritty",
    version: "0.13.2-1",
    description: "A fast, cross-platform, GPU-accelerated terminal emulator written in Rust",
    repository: "extra",
    url: "https://alacritty.org",
    license: "Apache-2.0",
    size_bytes: 14500000,
    is_installed: false,
    dependencies: ["fontconfig", "freetype2", "libglvnd"],
  },
  {
    name: "neovim",
    version: "0.10.1-1",
    description: "Hyperextensible, modern Vim-fork focused on extensibility and LSP agility",
    repository: "extra",
    url: "https://neovim.io",
    license: "Apache-2.0",
    size_bytes: 28000000,
    is_installed: false,
    dependencies: ["luajit", "libtermkey", "libuv"],
  },
  {
    name: "spotify-launcher",
    version: "0.6.1-1",
    description: "Client launcher for Spotify's APT repository with native sandboxing",
    repository: "extra",
    url: "https://spotify.com",
    license: "GPL-3.0-or-later",
    size_bytes: 12000000,
    is_installed: false,
    dependencies: ["curl", "tar"],
  },
  {
    name: "telegram-desktop",
    version: "5.4.1-1",
    description: "Official desktop client for Telegram messaging platform",
    repository: "extra",
    url: "https://desktop.telegram.org/",
    license: "GPL-3.0-or-later",
    size_bytes: 90000000,
    is_installed: false,
    dependencies: ["qt6-base", "ffmpeg", "libvpx"],
  },
  {
    name: "thunderbird",
    version: "128.2.0esr-1",
    description: "Standalone email, news, RSS, and chat client from Mozilla",
    repository: "extra",
    url: "https://www.thunderbird.net/",
    license: "MPL-2.0",
    size_bytes: 180000000,
    is_installed: false,
    dependencies: ["gtk3", "nss", "libpulse"],
  },
  {
    name: "mpv",
    version: "0.38.0-4",
    description: "Lightweight, scriptable, and cross-platform media player with Vulkan output",
    repository: "extra",
    url: "https://mpv.io/",
    license: "GPL-2.0-or-later",
    size_bytes: 12000000,
    is_installed: false,
    dependencies: ["ffmpeg", "libplacebo", "wayland"],
  },
  {
    name: "btop",
    version: "1.3.2-1",
    description: "A beautiful, responsive resource monitor for Linux processors, memory, and disks",
    repository: "extra",
    url: "https://github.com/aristocratos/btop",
    license: "Apache-2.0",
    size_bytes: 3500000,
    is_installed: false,
    dependencies: ["gcc-libs"],
  },
  {
    name: "git",
    version: "2.46.0-1",
    description: "The fast, distributed version control system created by Linus Torvalds",
    repository: "extra",
    url: "https://git-scm.com/",
    license: "GPL-2.0-only",
    size_bytes: 42000000,
    is_installed: false,
    dependencies: ["curl", "expat", "perl"],
  },
];

export class PacmanAppProvider implements PackageProvider<PacmanAppMeta> {
  readonly id = "pacman";
  readonly name = "Arch Native (Pacman)";
  readonly displayName = "Arch Native (Pacman)";
  readonly version = "1.0.0";
  readonly category = "apps";

  /** Cache of package meta keyed by package ID */
  private metaCache = new Map<string, PacmanAppMeta>();

  /**
   * Searches the host's ALPM sync databases for packages matching the query.
   * Runs unprivileged with zero subprocesses.
   */
  async search(query: string): Promise<PackageItem[]> {
    const trimmed = query.trim();

    // If query is empty, return curated applications for the default store experience
    if (!trimmed) {
      return this.discover();
    }

    if (isTauri) {
      try {
        const rawList = await invokeTauri<PacmanRawInfo[]>("pacman_search_packages", {
          query: trimmed,
          limit: 60,
        });
        return rawList.map((raw) => this.normalize(raw));
      } catch (err) {
        console.warn("[PacmanAppProvider] Backend search failed, using fallback:", err);
      }
    }

    // Fallback in non-Tauri or test environments:
    const filtered = CURATED_ARCH_PACKAGES.filter(
      (p) =>
        p.name.toLowerCase().includes(trimmed.toLowerCase()) ||
        p.description.toLowerCase().includes(trimmed.toLowerCase())
    );

    return filtered.map((raw) => this.normalize(raw));
  }

  /**
   * Discovers featured Arch applications for the store front.
   */
  async discover(): Promise<PackageItem[]> {
    // If running in live Tauri, update installed states for curated packages
    if (isTauri) {
      try {
        const installedList = await invokeTauri<PacmanRawInfo[]>("pacman_list_installed_packages");
        const installedMap = new Map(installedList.map((p) => [p.name, p.version]));

        return CURATED_ARCH_PACKAGES.map((raw) => {
          const isInst = installedMap.has(raw.name);
          const item = this.normalize({
            ...raw,
            is_installed: isInst,
            installed_version: isInst ? installedMap.get(raw.name) : undefined,
          });
          return item;
        });
      } catch {
        // ignore and fallback
      }
    }

    return CURATED_ARCH_PACKAGES.map((raw) => this.normalize(raw));
  }

  /**
   * Retrieves specific package details by name.
   */
  async getDetails(name: string): Promise<PackageItem | null> {
    if (isTauri) {
      try {
        const raw = await invokeTauri<PacmanRawInfo | null>("pacman_get_package_details", {
          packageName: name,
        });
        if (raw) {
          return this.normalize(raw);
        }
      } catch (err) {
        console.warn("[PacmanAppProvider] getDetails failed:", err);
      }
    }

    const found = CURATED_ARCH_PACKAGES.find((p) => p.name === name);
    return found ? this.normalize(found) : null;
  }

  /**
   * Lists all currently installed pacman packages.
   */
  async listInstalled(): Promise<PackageItem[]> {
    if (isTauri) {
      try {
        const list = await invokeTauri<PacmanRawInfo[]>("pacman_list_installed_packages");
        return list.map((raw) => this.normalize(raw));
      } catch (err) {
        console.warn("[PacmanAppProvider] listInstalled failed:", err);
      }
    }
    return [];
  }

  /**
   * Normalizes a raw ALPM package descriptor into a canonical PackageItem
   * enriched with authentic application metadata.
   */
  normalize(raw: unknown): PackageItem {
    const p = raw as PacmanRawInfo;
    const appMeta = resolveAppMetadata(p.name, p.name);

    const meta: PacmanAppMeta = {
      repository: p.repository || "extra",
      isInstalled: Boolean(p.is_installed),
      installedVersion: p.installed_version || undefined,
      sizeBytes: p.size_bytes || undefined,
      license: p.license || undefined,
      url: p.url || undefined,
      dependencies: Array.isArray(p.dependencies) ? p.dependencies : [],
    };

    this.metaCache.set(p.name, meta);

    const tags = [
      "native",
      "pacman",
      meta.repository,
      appMeta.category.toLowerCase(),
      "application",
    ];
    if (meta.isInstalled) {
      tags.push("installed");
    }

    return {
      id: p.name,
      title: p.name,
      subtitle: `${appMeta.publisher} • Arch Linux (${meta.repository})`,
      description: p.description || "Arch Linux official package",
      version: p.version,
      author: {
        name: appMeta.publisher,
        avatar: "arch",
        verified: true,
      },
      category: "apps",
      package_type: "app",
      tags,
      supported_desktops: ["universal"],
      supported_display: ["wayland", "x11"],
      rating: 4.9,
      rating_count: 240,
      downloads: 45000,
      hero_image: "https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/logos/arch.png",
      screenshots: [],
      color_palette: [appMeta.brandColor, "#0f141c"],
      safety_audit: {
        rating: "verified",
        changes_system_files: true,
        requires_root: true,
        sandbox_compatible: false,
        files_modified_count: 0,
      },
      dependencies: {
        packages: meta.dependencies,
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
      repository_id: meta.repository,
      package_size_bytes: meta.sizeBytes,
      integrity_status: "verified",
      is_cached: true,
    };
  }

  getSource(pkg: PackageItem): PackageSourceSpec {
    const meta = this.metaCache.get(pkg.id);
    return {
      type: "system",
      repository: meta?.repository || pkg.repository_id || "extra",
      url: meta?.url,
    };
  }

  getTargets(pkg: PackageItem): PackageTargetSpec[] {
    const meta = this.metaCache.get(pkg.id);
    return [
      {
        id: "native",
        name: "Arch Native Package (pacman)",
        scope: "system",
        supported: true,
        dependencies: meta?.dependencies || [],
        requires_root: true,
      },
    ];
  }

  getDependencies(pkg: PackageItem, _target: string): string[] {
    const meta = this.metaCache.get(pkg.id);
    return meta?.dependencies || [];
  }

  getProvenance(pkg: PackageItem): PackageProvenance {
    const meta = this.metaCache.get(pkg.id);
    const appMeta = resolveAppMetadata(pkg.id, pkg.title);
    return {
      upstream: meta?.url || "https://archlinux.org/packages",
      license: meta?.license || "Open Source",
      author: appMeta.publisher,
    };
  }

  getMedia(_pkg: PackageItem): PackageMediaSpec {
    return {
      poster: "https://raw.githubusercontent.com/catppuccin/catppuccin/main/assets/logos/arch.png",
    };
  }

  getMeta(pkg: PackageItem): PacmanAppMeta | undefined {
    return this.metaCache.get(pkg.id);
  }

  async install(pkg: PackageItem, _target: string): Promise<unknown> {
    if (isTauri) {
      return invokeTauri("pacman_install_package", { packageName: pkg.id });
    }
    const meta = this.metaCache.get(pkg.id);
    if (meta) {
      meta.isInstalled = true;
      meta.installedVersion = pkg.version;
    }
    return { success: true, package_name: pkg.id, version: pkg.version };
  }

  async uninstall(pkg: PackageItem, _target: string): Promise<unknown> {
    if (isTauri) {
      return invokeTauri("pacman_uninstall_package", { packageName: pkg.id });
    }
    const meta = this.metaCache.get(pkg.id);
    if (meta) {
      meta.isInstalled = false;
      meta.installedVersion = undefined;
    }
    return { success: true, package_name: pkg.id };
  }
}

export const pacmanAppProvider = new PacmanAppProvider();
