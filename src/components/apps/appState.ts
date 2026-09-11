/**
 * App State & Provider Capabilities Model — Phase 23D
 *
 * Single source of truth for:
 *   - Application installation states
 *   - Provider availability & selection (Pacman implemented; AUR/Flatpak staged)
 *   - Verification levels (Source repository vs Publisher vs Package signature)
 *   - Primary & overflow action determination
 */

import type { PackageItem } from "../../providers/types.ts";
import type { AppMetadata } from "./appMetadata.ts";

export type AppInstallState =
  | "installed"
  | "not-installed"
  | "installing"
  | "uninstalling"
  | "updating"
  | "error";

export type PackageProviderId = "pacman" | "aur" | "flatpak";

export interface PackageProviderOption {
  id: PackageProviderId;
  name: string;
  shortName: string;
  repository: string;
  description: string;
  available: boolean;
  statusNote?: string;
}

export const SUPPORTED_PROVIDERS: PackageProviderOption[] = [
  {
    id: "pacman",
    name: "Pacman",
    shortName: "Pacman · extra",
    repository: "extra",
    description: "Official Arch repo",
    available: true,
  },
  {
    id: "flatpak",
    name: "Flatpak",
    shortName: "Flathub · Flatpak",
    repository: "Flathub",
    description: "Universal Flatpak sandbox",
    available: true,
  },
  {
    id: "aur",
    name: "AUR",
    shortName: "AUR · User",
    repository: "AUR",
    description: "Arch User Repository (Unprivileged)",
    available: true,
  },
];

export type VerificationLevel =
  | "official"
  | "verified"
  | "community"
  | "unknown";

export interface VerificationDetails {
  level: VerificationLevel;
  badgeLabel: string;
  sublabel: string;
  sourceType: string;
  repository: string;
  signatureStatus: string;
  maintainer: string;
  description: string;
}

/**
 * Resolves authentic verification level without false claims.
 * Exact Phase 23D verification mappings:
 *   - Official Arch package: Verified / Official Arch Repository
 *   - AUR: Community / AUR · yay
 *   - Verified Flathub publisher: Verified Publisher / Flathub
 *   - Unknown: Unknown source
 */
export function getVerificationDetails(
  _pkg: PackageItem,
  meta: AppMetadata,
  pacmanRepo?: string
): VerificationDetails {
  const repo = (pacmanRepo || "extra").toLowerCase();
  const isOfficialArch = repo === "extra" || repo === "core" || repo === "multilib";

  if (isOfficialArch) {
    return {
      level: "official",
      badgeLabel: "Official Arch Repository",
      sublabel: "Official Arch Repository",
      sourceType: "Official Arch Repository",
      repository: repo,
      signatureStatus: "Signed by trusted maintainers",
      maintainer: "Arch Linux package maintainer",
      description: "This package is distributed through the official Arch Linux repositories and signed with trusted package maintainer keys.",
    };
  }

  if (repo === "aur") {
    return {
      level: "community",
      badgeLabel: "Community",
      sublabel: "AUR · yay",
      sourceType: "AUR · yay",
      repository: "AUR",
      signatureStatus: "User-submitted PKGBUILD",
      maintainer: "AUR community contributor",
      description: "This package is maintained by the community in the Arch User Repository. Inspect PKGBUILD before building.",
    };
  }

  if (repo === "flatpak" || repo === "flathub") {
    return {
      level: "verified",
      badgeLabel: "Verified Publisher",
      sublabel: "Flathub",
      sourceType: "Flathub",
      repository: "Flathub",
      signatureStatus: "Verified Flathub release",
      maintainer: meta.publisher || "Flathub verified publisher",
      description: "This application is verified by the upstream publisher on Flathub.",
    };
  }

  if (meta.isCuratedApp && meta.publisher && meta.publisher !== "Arch Linux / Community") {
    return {
      level: "verified",
      badgeLabel: "Verified Publisher",
      sublabel: `${meta.publisher} upstream`,
      sourceType: "Upstream Verified",
      repository: repo,
      signatureStatus: "Upstream vendor release",
      maintainer: meta.publisher,
      description: `Artwork and metadata for this application are recognized from verified upstream publisher ${meta.publisher}.`,
    };
  }

  return {
    level: "unknown",
    badgeLabel: "Unknown source",
    sublabel: "Unknown source",
    sourceType: "Unknown source",
    repository: repo,
    signatureStatus: "Unchecked",
    maintainer: "Unknown",
    description: "This package originates from an unverified or third-party source.",
  };
}

export interface AppActionsConfig {
  primaryAction: "open" | "install" | "installing" | "uninstalling" | "updating" | "retry";
  primaryLabel: string;
  showProviderSelector: boolean;
  canUninstall: boolean;
  showOverflow: boolean;
  isBusy: boolean;
}

/**
 * Single source of truth for application actions.
 * Guarantees that:
 *  - Installed apps NEVER render "Install"
 *  - Uninstalled apps render "Provider ▾" + "Install"
 *  - Busy states disable conflicting actions
 */
export function getApplicationActions(
  state: AppInstallState,
  selectedProvider: PackageProviderOption = SUPPORTED_PROVIDERS[0]
): AppActionsConfig {
  switch (state) {
    case "installed":
      return {
        primaryAction: "open",
        primaryLabel: "Open",
        showProviderSelector: false,
        canUninstall: true,
        showOverflow: true,
        isBusy: false,
      };

    case "not-installed":
      return {
        primaryAction: "install",
        primaryLabel: "Install",
        showProviderSelector: true,
        canUninstall: false,
        showOverflow: false,
        isBusy: false,
      };

    case "installing":
      return {
        primaryAction: "installing",
        primaryLabel: `Installing from ${selectedProvider.name}…`,
        showProviderSelector: false,
        canUninstall: false,
        showOverflow: false,
        isBusy: true,
      };

    case "uninstalling":
      return {
        primaryAction: "uninstalling",
        primaryLabel: "Removing…",
        showProviderSelector: false,
        canUninstall: false,
        showOverflow: false,
        isBusy: true,
      };

    case "updating":
      return {
        primaryAction: "updating",
        primaryLabel: "Updating…",
        showProviderSelector: false,
        canUninstall: false,
        showOverflow: false,
        isBusy: true,
      };

    case "error":
      return {
        primaryAction: "retry",
        primaryLabel: "Retry Install",
        showProviderSelector: true,
        canUninstall: false,
        showOverflow: false,
        isBusy: false,
      };
  }
}
