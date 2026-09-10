export * from "./types.ts";
export { qylockLockscreenProvider, normalizeQylockTheme, resolveLockscreenCapabilities, getCatalogueLockScreens, RAW_QYLOCK_THEMES } from "./qylockProvider.ts";
import { qylockLockscreenProvider } from "./qylockProvider.ts";
import type { LockscreenProvider, PackageItem } from "./types.ts";

export const hyprlockLockscreenProvider: LockscreenProvider = {
  id: "hyprlock",
  name: "Hyprlock Native Provider",
  discover: () => [],
  normalize: (raw: any) => raw as PackageItem,
  getPreview: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getSource: (_pkg: PackageItem) => ({
    type: "git",
    repository: "https://github.com/hyprwm/hyprlock",
    revision: "main",
  }),
  getTargets: () => ({
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
  discover: () => [],
  normalize: (raw: any) => raw as PackageItem,
  getPreview: (pkg: PackageItem) => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
  }),
  getSource: (_pkg: PackageItem) => ({
    type: "git",
    repository: "https://github.com/mortie/swaylock-effects",
    revision: "master",
  }),
  getTargets: () => ({
    quickshell: false,
    sddm: false,
    swaylock: true,
  }),
  getDependencies: () => ["swaylock"],
  getProvenance: (_pkg: PackageItem) => ({
    upstream: "https://github.com/mortie/swaylock-effects",
    revision: "master",
    license: "MIT",
  }),
};

const providers: Record<string, LockscreenProvider> = {
  qylock: qylockLockscreenProvider,
  hyprlock: hyprlockLockscreenProvider,
  swaylock: swaylockLockscreenProvider,
};

export function getLockscreenProvider(providerId?: string): LockscreenProvider | undefined {
  if (!providerId) return undefined;
  return providers[providerId.toLowerCase()];
}

export function getAllLockscreenProviders(): LockscreenProvider[] {
  return Object.values(providers);
}
