/**
 * Ryzora Provider Registry — Phase 22
 *
 * Exports all provider types and the engine singleton.
 * Registers all built-in providers with the PackageEngine on import.
 */

export * from "./types.ts";
export { pacmanAppProvider, PacmanAppProvider } from "./pacmanProvider.ts";
export { flatpakAppProvider, FlatpakAppProvider } from "./flatpakProvider.ts";
export { aurAppProvider, AurAppProvider } from "./aurProvider.ts";
export {
  qylockLockscreenProvider,
  normalizeQylockTheme,
  resolveLockscreenCapabilities,
  getCatalogueLockScreens,
  RAW_QYLOCK_THEMES,
} from "./qylockProvider.ts";

import { qylockLockscreenProvider } from "./qylockProvider.ts";
import { pacmanAppProvider } from "./pacmanProvider.ts";
import { flatpakAppProvider } from "./flatpakProvider.ts";
import { aurAppProvider } from "./aurProvider.ts";
import type { LockscreenProvider, LockscreenTargetsSpec, PackageItem } from "./types.ts";

// ─────────────────────────────────────────────────────────────────────────────
// Lockscreen-specific provider stubs (backward compatibility)
// ─────────────────────────────────────────────────────────────────────────────

export const hyprlockLockscreenProvider: LockscreenProvider = {
  id: "hyprlock",
  name: "Hyprlock Native Provider",
  category: "lockscreen",
  discover: () => [],
  normalize: (raw: unknown) => raw as PackageItem,
  getPreview: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getMedia: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getSource: (_pkg: PackageItem) => ({
    type: "git",
    repository: "https://github.com/hyprwm/hyprlock",
    revision: "main",
  }),
  getTargets: (): LockscreenTargetsSpec => ({
    quickshell: false,
    sddm: false,
    hyprlock: true,
  }),
  getDependencies: () => ["hyprlock"],
  getProvenance: (_pkg: PackageItem) => ({
    upstream: "https://github.com/hyprwm/hyprlock",
    revision: "main",
    license: "BSD-3-Clause",
  }),
};

export const swaylockLockscreenProvider: LockscreenProvider = {
  id: "swaylock",
  name: "Swaylock Effects Provider",
  category: "lockscreen",
  discover: () => [],
  normalize: (raw: unknown) => raw as PackageItem,
  getPreview: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getMedia: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getSource: (_pkg: PackageItem) => ({
    type: "git",
    repository: "https://github.com/mortie/swaylock-effects",
    revision: "master",
  }),
  getTargets: (): LockscreenTargetsSpec => ({
    quickshell: false,
    sddm: false,
    swaylock: true,
  }),
  getDependencies: () => ["swaylock-effects"],
  getProvenance: (_pkg: PackageItem) => ({
    upstream: "https://github.com/mortie/swaylock-effects",
    revision: "master",
    license: "MIT",
  }),
};

// ─────────────────────────────────────────────────────────────────────────────
// Legacy registry (lockscreen-specific, kept for backward compatibility)
// ─────────────────────────────────────────────────────────────────────────────

const _lockscreenProviders: Record<string, LockscreenProvider> = {
  qylock: qylockLockscreenProvider,
  hyprlock: hyprlockLockscreenProvider,
  swaylock: swaylockLockscreenProvider,
};

export function getLockscreenProvider(providerId?: string): LockscreenProvider | undefined {
  if (!providerId) return undefined;
  return _lockscreenProviders[providerId.toLowerCase()];
}

export function getAllLockscreenProviders(): LockscreenProvider[] {
  return Object.values(_lockscreenProviders);
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 22: Register all providers with the universal PackageEngine
// ─────────────────────────────────────────────────────────────────────────────

import { packageEngine } from "../engine/PackageEngine.ts";

// Qylock is the primary provider — registered first (highest priority)
// eslint-disable-next-line @typescript-eslint/no-explicit-any
packageEngine.register(qylockLockscreenProvider as any);
// Stub providers for hyprlock/swaylock (discover() returns [])
// eslint-disable-next-line @typescript-eslint/no-explicit-any
packageEngine.register(hyprlockLockscreenProvider as any);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
packageEngine.register(swaylockLockscreenProvider as any);

// Register pacman app provider (Phase 23A)
packageEngine.register(pacmanAppProvider as any);
packageEngine.register(flatpakAppProvider as any);
packageEngine.register(aurAppProvider as any);

// Re-export the engine so consumers can import it from the providers module
export { packageEngine } from "../engine/PackageEngine.ts";
export { PackageEngine } from "../engine/PackageEngine.ts";
