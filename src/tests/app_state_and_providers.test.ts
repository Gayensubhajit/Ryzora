import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  SUPPORTED_PROVIDERS,
  getApplicationActions,
  getVerificationDetails,
} from "../components/apps/appState.ts";
import type { PackageItem } from "../providers/types.ts";
import type { AppMetadata } from "../components/apps/appMetadata.ts";

describe("Phase 23D — App State, Provider Selection & Storefront Actions", () => {
  const dummyPkg: PackageItem = {
    id: "firefox",
    name: "Firefox",
    description: "Fast, private and extensible web browser",
    category: "apps",
    installed: true,
    providerId: "pacman",
  };

  const dummyMeta: AppMetadata = {
    displayName: "Firefox",
    summary: "Web browser",
    publisher: "Mozilla",
    category: "Internet",
    isCuratedApp: true,
  };

  it("State Model 1: Installed app strictly yields Open + Uninstall + Overflow, NO Install button or Provider selector", () => {
    const actions = getApplicationActions("installed", SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "open");
    assert.equal(actions.primaryLabel, "Open");
    assert.equal(actions.showProviderSelector, false, "Installed app must never display provider selector");
    assert.equal(actions.canUninstall, true);
    assert.equal(actions.showOverflow, true);
    assert.equal(actions.isBusy, false);
  });

  it("State Model 2: Not-installed app strictly yields Install + Provider selector", () => {
    const actions = getApplicationActions("not-installed", SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "install");
    assert.equal(actions.primaryLabel, "Install");
    assert.equal(actions.showProviderSelector, true, "Uninstalled app must display provider selector");
    assert.equal(actions.canUninstall, false);
    assert.equal(actions.showOverflow, false);
    assert.equal(actions.isBusy, false);
  });

  it("State Model 3: Installing & Uninstalling states are busy and lock interactive buttons", () => {
    const installing = getApplicationActions("installing", SUPPORTED_PROVIDERS[0]);
    assert.equal(installing.primaryAction, "installing");
    assert.equal(installing.isBusy, true);
    assert.equal(installing.showProviderSelector, false);
    assert.equal(installing.canUninstall, false);

    const uninstalling = getApplicationActions("uninstalling", SUPPORTED_PROVIDERS[0]);
    assert.equal(uninstalling.primaryAction, "uninstalling");
    assert.equal(uninstalling.isBusy, true);
    assert.equal(uninstalling.showProviderSelector, false);
    assert.equal(uninstalling.canUninstall, false);
  });

  it("State Model 4: Error state yields Retry with provider selector enabled", () => {
    const errorState = getApplicationActions("error", SUPPORTED_PROVIDERS[0]);
    assert.equal(errorState.primaryAction, "retry");
    assert.equal(errorState.showProviderSelector, true);
    assert.equal(errorState.isBusy, false);
  });

  it("Provider Model 1: Pacman, AUR, and Flatpak are supported and actionable", () => {
    const pacman = SUPPORTED_PROVIDERS.find((p) => p.id === "pacman");
    assert.ok(pacman, "Pacman provider must exist");
    assert.equal(pacman.available, true, "Pacman must be actionable");

    const aur = SUPPORTED_PROVIDERS.find((p) => p.id === "aur");
    assert.ok(aur, "AUR provider must exist in list");
    assert.equal(aur.available, true, "AUR must be actionable in Phase 25");

    const flatpak = SUPPORTED_PROVIDERS.find((p) => p.id === "flatpak");
    assert.ok(flatpak, "Flatpak provider must exist in list");
    assert.equal(flatpak.available, true, "Flatpak must be actionable in Phase 25");
  });

  it("Verification 1: Official Arch packages are verified through official repository", () => {
    const verified = getVerificationDetails(dummyPkg, dummyMeta, "extra");
    assert.equal(verified.level, "official");
    assert.equal(verified.badgeLabel, "Official Arch Repository");
    assert.equal(verified.sourceType, "Official Arch Repository");
    assert.equal(verified.signatureStatus, "Signed by trusted maintainers");
  });

  it("Verification 2: AUR packages are strictly classified as Community Maintained, never Official", () => {
    const aurDetails = getVerificationDetails(dummyPkg, dummyMeta, "aur");
    assert.equal(aurDetails.level, "community");
    assert.equal(aurDetails.badgeLabel, "Community");
    assert.equal(aurDetails.sublabel, "AUR · yay");
    assert.equal(aurDetails.sourceType, "AUR · yay");
    assert.notEqual(aurDetails.level, "official", "AUR must never be marked official");
  });

  it("Verification 3: Curated apps with upstream publishers are verified as Upstream Verified", () => {
    const verified = getVerificationDetails(dummyPkg, dummyMeta, "custom_repo");
    assert.equal(verified.level, "verified");
    assert.equal(verified.badgeLabel, "Verified Publisher");
    assert.equal(verified.maintainer, "Mozilla");
  });

  it("Verification 4: Unknown sources resolve to unverified level", () => {
    const uncuratedMeta: AppMetadata = {
      displayName: "Random Utility",
      summary: "Some utility",
      category: "Utilities",
      isCuratedApp: false,
    };
    const unknown = getVerificationDetails(dummyPkg, uncuratedMeta, "unknown_thirdparty");
    assert.equal(unknown.level, "unknown");
    assert.equal(unknown.badgeLabel, "Unknown source");
  });
});

describe("Phase 23E — Real Package Transactions, Application Launch & Navigation", () => {
  it("Phase 25 - 1: All registered providers in SUPPORTED_PROVIDERS are actionable", () => {
    for (const p of SUPPORTED_PROVIDERS) {
      assert.equal(p.available, true, `Provider '${p.name}' must be available`);
    }
  });

  it("Phase 23E - 2: Successful install operation transitions state from not-installed to installed in-place", () => {
    let currentState: "not-installed" | "installing" | "installed" = "not-installed";
    let actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "install");
    assert.equal(actions.showProviderSelector, true);

    // Operation starts
    currentState = "installing";
    actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "installing");
    assert.equal(actions.isBusy, true);
    assert.equal(actions.showProviderSelector, false);

    // Transaction finishes successfully
    currentState = "installed";
    actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "open");
    assert.equal(actions.canUninstall, true);
    assert.equal(actions.showOverflow, true);
    assert.equal(actions.showProviderSelector, false, "Installed app must never show provider selector");
  });

  it("Phase 23E - 3: Successful uninstall operation transitions state from installed to not-installed", () => {
    let currentState: "installed" | "uninstalling" | "not-installed" = "installed";
    let actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "open");

    // Uninstall operation starts
    currentState = "uninstalling";
    actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "uninstalling");
    assert.equal(actions.isBusy, true);

    // Uninstall completes
    currentState = "not-installed";
    actions = getApplicationActions(currentState, SUPPORTED_PROVIDERS[0]);
    assert.equal(actions.primaryAction, "install");
    assert.equal(actions.showProviderSelector, true);
  });

  it("Phase 23E - 4: Reinstall operation uses updating state without flipping to not-installed", () => {
    const updating = getApplicationActions("updating", SUPPORTED_PROVIDERS[0]);
    assert.equal(updating.primaryAction, "updating");
    assert.equal(updating.isBusy, true);
  });
});
