import type { PackageItem } from "../types/index.ts";
export type { PackageItem };

// ─────────────────────────────────────────────────────────────────────────────
// Phase 22 — Generic Package Provider contract
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Generic source descriptor. Replaces the lockscreen-specific LockscreenSourceSpec
 * as the universal representation understood by the engine.
 */
export interface PackageSourceSpec {
  type: "git" | "archive" | "local" | "registry" | "system";
  repository?: string;
  revision?: string;
  path?: string;
  url?: string;
  sha256?: string;
}

/**
 * Generic target/adapter descriptor. A package may support multiple targets
 * (e.g. "quickshell", "sddm", "hyprlock") — each is a separate PackageTargetSpec.
 */
export interface PackageTargetSpec {
  id: string;           // "quickshell" | "sddm" | "hyprlock" | "swaylock" | …
  name: string;         // Human-readable adapter name
  scope: "user" | "system";
  supported: boolean;
  entrypoint?: string;
  dependencies: string[];
  requires_root: boolean;
  install_destination?: string;
}

/**
 * Generic media descriptor — covers images, videos, and animated previews.
 */
export interface PackageMediaSpec {
  poster: string;
  preview_video?: string;
  preview_animated?: string;
  upstream_video?: string;
  upstream_animated?: string;
}

/**
 * Generic provenance descriptor — tracks where a package came from.
 */
export interface PackageProvenance {
  upstream: string;
  revision?: string;
  license?: string;
  path?: string;
  author?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Generic PackageProvider interface
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The universal provider contract.
 *
 * TMeta is an optional category-specific extra (e.g. LockscreenMeta for lockscreens,
 * WallpaperMeta for wallpapers). If not needed, omit it and the default is `unknown`.
 *
 * Note: `getTargets` returns `unknown` here so that backward-compatible
 * lockscreen providers (which return `LockscreenTargetsSpec`) satisfy the
 * interface without type coercion. Use `getTargetSpecs` for the typed
 * PackageTargetSpec[] form when category is known.
 */
export interface PackageProvider<TMeta = unknown> {
  /** Unique provider identifier ("qylock", "hyprlock", "swaylock", …). */
  id: string;
  /** Human-readable provider name. */
  name: string;
  /** Package category this provider manages. */
  category: string;

  /** Discovers all packages offered by this provider. */
  discover(): Promise<PackageItem[]> | PackageItem[];
  /** Searches packages offered by this provider (optional). */
  search?(query: string): Promise<PackageItem[]> | PackageItem[];
  /** Normalises a raw provider object into a canonical PackageItem. */
  normalize(raw: unknown): PackageItem;
  /** Returns the canonical source spec for the given package. */
  getSource(pkg: PackageItem): PackageSourceSpec | LockscreenSourceSpec;
  /**
   * Returns the installation target descriptor(s) for the given package.
   * Returns `unknown` to allow both `LockscreenTargetsSpec` (legacy) and
   * `PackageTargetSpec[]` (generic) implementations to satisfy this interface.
   */
  getTargets(pkg: PackageItem): unknown;
  /** Returns all dependencies required for the given package / target combination. */
  getDependencies(pkg: PackageItem, target: string): string[];
  /** Returns provenance metadata for the given package. */
  getProvenance(pkg: PackageItem): PackageProvenance | LockscreenProvenance;
  /** Returns media assets for the given package. */
  getMedia?(pkg: PackageItem): PackageMediaSpec;
  /** Returns category-specific extra metadata (optional). */
  getMeta?(pkg: PackageItem): TMeta | undefined;
  /** Installs the package (optional — may delegate to the Rust engine). */
  install?(pkg: PackageItem, target: string, options?: Record<string, unknown>): Promise<unknown>;
  /** Uninstalls the package (optional — may delegate to the Rust engine). */
  uninstall?(pkg: PackageItem, target: string): Promise<unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Lockscreen-specific extras (used by LockscreenProvider alias)
// ─────────────────────────────────────────────────────────────────────────────

export interface LockscreenMeta {
  targets: LockscreenTargetsSpec;
  media: LockscreenMediaSpec;
  provenance: LockscreenProvenance;
  source: LockscreenSourceSpec;
}

// ─────────────────────────────────────────────────────────────────────────────
// Backward-compatible type aliases (lockscreen-specific, Phase 1-21 compat)
// ─────────────────────────────────────────────────────────────────────────────

/** @deprecated Use PackageSourceSpec. Kept for backward compatibility. */
export interface LockscreenSourceSpec {
  type: "git" | "archive" | "local";
  repository?: string;
  revision?: string;
  path?: string;
  url?: string;
  sha256?: string;
}

/** @deprecated Use PackageTargetSpec[]. Kept for backward compatibility. */
export interface LockscreenTargetsSpec {
  quickshell: boolean;
  sddm: boolean;
  hyprlock?: boolean;
  swaylock?: boolean;
}

/** @deprecated Use PackageMediaSpec. Kept for backward compatibility. */
export interface LockscreenMediaSpec {
  poster: string;
  preview_video?: string;
  preview_animated?: string;
  upstream_video?: string;
  upstream_animated?: string;
}

/** @deprecated Use PackageProvenance. Kept for backward compatibility. */
export interface LockscreenProvenance {
  upstream: string;
  revision: string;
  license?: string;
  path?: string;
}

/**
 * Backward-compatible lockscreen provider interface.
 *
 * Extends PackageProvider with lockscreen-specific method signatures.
 * `getTargets` returns `LockscreenTargetsSpec` (legacy boolean flags)
 * rather than the generic `PackageTargetSpec[]` — this is intentional for
 * backward compatibility.
 */
export interface LockscreenProvider extends Omit<PackageProvider<LockscreenMeta>, 'getTargets' | 'getSource' | 'getProvenance'> {
  // Lockscreen-specific overrides
  getPreview(pkg: PackageItem): LockscreenMediaSpec;
  getSource(pkg: PackageItem): LockscreenSourceSpec;
  getTargets(pkg: PackageItem): LockscreenTargetsSpec;
  getDependencies(pkg: PackageItem, target: string): string[];
  getProvenance(pkg: PackageItem): LockscreenProvenance;
  install?(pkg: PackageItem, target: string, options?: unknown): Promise<unknown>;
  uninstall?(pkg: PackageItem, target: string): Promise<unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider summary (used by PackageEngine.listProviders)
// ─────────────────────────────────────────────────────────────────────────────

export interface FrontendProviderSummary {
  id: string;
  name: string;
  category: string;
  enabled: boolean;
  package_count: number;
}


// ─────────────────────────────────────────────────────────────────────────────
// App Provider Extras (Phase 23)
// ─────────────────────────────────────────────────────────────────────────────

export interface PacmanAppMeta {
  repository: string;
  isInstalled: boolean;
  installedVersion?: string;
  sizeBytes?: number;
  license?: string;
  url?: string;
  dependencies: string[];
}
