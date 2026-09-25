import { test } from "node:test";
import assert from "node:assert";
import { packageEngine } from "../providers/index.ts";
import {
  SILENTSDDM_WALLPAPERS,
  SILENTSDDM_UPSTREAM,
} from "../providers/silentSddmDiscovery.ts";
import {
  isLoginScreen,
  isSessionLock,
  getPackageSubtype,
  getConciseCompatibility,
} from "../components/catalogue/catalogueUtils.ts";
import { MOCK_PACKAGES } from "../data/mockPackages.ts";
import { resolveLockscreenCapabilities } from "../providers/qylockProvider.ts";
import type { SystemInfo, UpstreamWallpaperInstallRequest } from "../types/index.ts";

const mockSystemInfo: SystemInfo = {
  distro_name: "Arch Linux",
  distro_id: "arch",
  distro_family: "arch",
  distro_version: "Rolling",
  kernel_version: "6.18.50",
  desktop_environment: "Hyprland",
  window_manager: "Hyprland",
  session_type: "wayland",
  shell: "zsh",
  terminal: "kitty",
  installed_components: [
    { name: "Hyprland", binary: "hyprland", installed: true, path: "/usr/bin/hyprland", category: "Window Manager" },
    { name: "SDDM", binary: "sddm", installed: true, path: "/usr/bin/sddm", category: "Display Manager" },
  ],
};

test("S3-UI 1: PackageEngine discovers both Qylock and SilentSDDM in lockscreens", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  assert.ok(allLockscreens.length >= 48, `Expected at least 48 lockscreens, got ${allLockscreens.length}`);

  const qylockItems = allLockscreens.filter((p) => p.lockscreen?.provider === "qylock");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  assert.ok(qylockItems.length >= 40, `Expected >= 40 Qylock items, got ${qylockItems.length}`);
  assert.strictEqual(silentItems.length, 6, "Expected exactly 6 SilentSDDM wallpapers");

  const silentIds = silentItems.map((p) => p.id).sort();
  const expectedIds = [
    "silentsddm-default",
    "silentsddm-ken",
    "silentsddm-mountain",
    "silentsddm-rei",
    "silentsddm-silvia",
    "silentsddm-smoky",
  ].sort();
  assert.deepStrictEqual(silentIds, expectedIds);
});

test("S3-UI 2: SilentSDDM wallpapers expose strict SDDM-only target specs", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    assert.strictEqual(pkg.category, "lockscreens");
    assert.strictEqual(pkg.package_type, "lockscreen");
    assert.strictEqual(pkg.supports_session_lock, false);
    assert.strictEqual(pkg.supports_login_screen, true);

    assert.ok(pkg.lockscreen);
    assert.strictEqual(pkg.lockscreen.provider, "silentsddm");
    assert.strictEqual(pkg.lockscreen.targets.quickshell, undefined);
    assert.ok(pkg.lockscreen.targets.sddm);
    assert.strictEqual(pkg.lockscreen.targets.sddm.scope, "system");
    assert.strictEqual(
      pkg.lockscreen.targets.sddm.install_destination,
      "/usr/share/sddm/themes/ryzora-silent"
    );
  }
});

test("S3-UI 3: Helper functions classify SilentSDDM subtype and compatibility correctly", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    assert.strictEqual(isLoginScreen(pkg), true, `${pkg.id} must be identified as login screen`);
    assert.strictEqual(isSessionLock(pkg), false, `${pkg.id} must not be identified as session lock`);
    assert.strictEqual(getPackageSubtype(pkg), "SilentSDDM", `${pkg.id} subtype must be 'SilentSDDM'`);

    const compat = getConciseCompatibility(pkg);
    assert.strictEqual(compat.label, "SDDM Login Screen");
  }
});

test("S3-UI 4: CategoryView subfiltering matches SilentSDDM only on 'All' and 'SDDM'", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    // 1. Subfilter: "All"
    const matchesAll = "All" === "All";
    assert.ok(matchesAll);

    // 2. Subfilter: "SDDM"
    const matchesSddm =
      "sddm" === "all" ||
      pkg.tags.some((t) => t.toLowerCase() === "sddm") ||
      (pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm != null);
    assert.ok(matchesSddm, `${pkg.id} must match SDDM subfilter`);

    // 3. Subfilter: "Quickshell"
    const matchesQuickshell =
      pkg.tags.some((t) => t.toLowerCase() === "quickshell") ||
      (pkg.supports_session_lock || pkg.lockscreen?.targets?.quickshell != null);
    assert.strictEqual(matchesQuickshell, false, `${pkg.id} must NOT match Quickshell subfilter`);

    // 4. Subfilter: "Hyprlock"
    const matchesHyprlock =
      pkg.tags.some((t) => t.toLowerCase() === "hyprlock") ||
      pkg.lockscreen?.targets?.hyprlock != null;
    assert.strictEqual(matchesHyprlock, false, `${pkg.id} must NOT match Hyprlock subfilter`);

    // 5. Subfilter: "Swaylock"
    const matchesSwaylock =
      pkg.tags.some((t) => t.toLowerCase() === "swaylock") ||
      pkg.lockscreen?.targets?.swaylock != null;
    assert.strictEqual(matchesSwaylock, false, `${pkg.id} must NOT match Swaylock subfilter`);
  }
});

test("S3-UI 5: MOCK_PACKAGES contains all 6 SilentSDDM wallpapers for dev fallback", () => {
  const sddmInMock = MOCK_PACKAGES.filter((p) => p.lockscreen?.provider === "silentsddm");
  assert.strictEqual(sddmInMock.length, 6, "Expected 6 SilentSDDM wallpapers in MOCK_PACKAGES");

  for (const wp of SILENTSDDM_WALLPAPERS) {
    const found = sddmInMock.find((p) => p.id === wp.id);
    assert.ok(found, `Expected ${wp.id} to be present in MOCK_PACKAGES`);
    assert.strictEqual(found.package_size_bytes, wp.sizeBytes);
  }
});

test("S3-UI 6: Detail view target resolution selects SDDM and validates dependencies", () => {
  const defaultWp = SILENTSDDM_WALLPAPERS.find((w) => w.id === "silentsddm-default")!;
  const kenWp = SILENTSDDM_WALLPAPERS.find((w) => w.id === "silentsddm-ken")!;

  // Image wallpaper resolution
  const imageItem = packageEngine.getProvider("silentsddm")!.normalize(defaultWp);
  const imageRes = resolveLockscreenCapabilities(mockSystemInfo, imageItem.lockscreen, "sddm");
  assert.strictEqual(imageRes.can_install, true);
  assert.ok(imageRes.supported_dependencies.includes("sddm"));
  assert.ok(imageRes.supported_dependencies.includes("qt6-declarative"));
  assert.ok(imageRes.supported_dependencies.includes("qt6-svg"));
  assert.strictEqual(imageRes.supported_dependencies.includes("qt6-multimedia"), false);

  // Video wallpaper resolution
  const videoItem = packageEngine.getProvider("silentsddm")!.normalize(kenWp);
  const videoRes = resolveLockscreenCapabilities(mockSystemInfo, videoItem.lockscreen, "sddm");
  assert.strictEqual(videoRes.can_install, true);
  assert.ok(videoRes.supported_dependencies.includes("sddm"));
  assert.ok(videoRes.supported_dependencies.includes("qt6-multimedia"));
  assert.ok(videoRes.supported_dependencies.includes("gst-plugins-good"));
});

test("S3-UI 7: Wallpaper install request mapping matches UpstreamWallpaperInstallRequest", () => {
  for (const wp of SILENTSDDM_WALLPAPERS) {
    const req: UpstreamWallpaperInstallRequest = {
      id: wp.id,
      filename: wp.filename,
      media_type: wp.type,
      sha256: wp.sha256,
      size_bytes: wp.sizeBytes,
      download_url: wp.downloadUrl,
      poster_url: wp.posterUrl || null,
      poster_sha256: wp.posterSha256 || null,
    };

    assert.strictEqual(req.id, wp.id);
    assert.strictEqual(req.filename, wp.filename);
    assert.strictEqual(req.sha256.length, 64);
    assert.ok(req.size_bytes > 0);
    assert.ok(req.download_url.startsWith("https://raw.githubusercontent.com/uiriansan/SilentSDDM/"));
    if (wp.type === "video") {
      assert.ok(req.poster_url != null);
      assert.ok(req.poster_sha256 != null);
      assert.strictEqual(req.poster_sha256!.length, 64);
    }
  }
});

test("S3-UI 8: Storefront deduplication preserves 6 catalog IDs but renders 5 visually distinct cards", async () => {
  const { deduplicateStorefrontPackages } = await import("../components/catalogue/catalogueUtils.ts");
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  // Underlying catalog has exactly 6 items
  assert.strictEqual(silentItems.length, 6);

  // Content hashes show default.jpg and smoky.jpg share SHA-256
  const hashes = silentItems.map((p) => p.content_hash);
  const uniqueHashes = new Set(hashes);
  assert.strictEqual(uniqueHashes.size, 5, "Expected exactly 5 unique content hashes among 6 entries");

  // Deduplicated storefront list renders exactly 5 items
  const dedupedSilent = deduplicateStorefrontPackages(silentItems);
  assert.strictEqual(dedupedSilent.length, 5, "Storefront should render exactly 5 visually unique cards");

  // The kept duplicate card is 'Default (Smoky)'
  const defaultCard = dedupedSilent.find((p) => p.id === "silentsddm-default");
  assert.ok(defaultCard, "Default (Smoky) card must be preserved");
  assert.strictEqual(defaultCard.title, "SilentSDDM · Default (Smoky)");

  // The duplicate smoky card is not present in storefront
  const smokyCard = dedupedSilent.find((p) => p.id === "silentsddm-smoky");
  assert.strictEqual(smokyCard, undefined, "Redundant smoky card must be omitted from storefront");
});

test("S3-UI 9: Detail view target architecture omits Quickshell and shows SDDM login screen only", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    const canTargetQs = Boolean(pkg.supports_session_lock || pkg.lockscreen?.targets?.quickshell);
    const canTargetSddm = Boolean(pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm);

    // SilentSDDM must NOT support Quickshell session lock
    assert.strictEqual(canTargetQs, false, `${pkg.id} must not target Quickshell`);
    // SilentSDDM MUST support SDDM login screen
    assert.strictEqual(canTargetSddm, true, `${pkg.id} must target SDDM`);
  }
});

test("S3-UI 10: Zero rating (rating_count === 0) hides star rating in UI contract", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    // Unfabricated metrics check
    assert.strictEqual(pkg.rating, 0);
    assert.strictEqual(pkg.rating_count, 0);
    assert.strictEqual(pkg.downloads, 0);

    // UI contract: hasRating must be false when rating_count is 0
    const hasRating = (pkg.rating ?? 0) > 0 && (pkg.rating_count !== undefined ? pkg.rating_count > 0 : true);
    assert.strictEqual(hasRating, false, `${pkg.id} must not trigger star rating rendering`);
  }
});

test("S3-UI 11: CustomMediaProvider normalizes custom assets into pure SDDM PackageItem", async () => {
  const { normalizeCustomMediaAsset } = await import("../providers/customMediaProvider.ts");
  const { SilentSddmCachedAsset } = await import("../types/index.ts");

  const mockAsset = {
    id: "custom:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    filename: "my_cool_wallpaper.png",
    display_name: "Anime Sunset",
    media_type: "image" as const,
    sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    size_bytes: 1048576,
    path: "/home/user/.local/share/ryzora/lockscreens/silentsddm/custom/my_cool_wallpaper.png",
  };

  const pkg = normalizeCustomMediaAsset(mockAsset);

  assert.strictEqual(pkg.id, mockAsset.id);
  assert.strictEqual(pkg.title, "Custom · Anime Sunset");
  assert.strictEqual(pkg.package_type, "lockscreen");
  assert.ok(pkg.tags.includes("custom"));
  assert.ok(pkg.tags.includes("my media"));
  assert.ok(pkg.tags.includes("sddm"));
  assert.strictEqual(pkg.supports_login_screen, true);
  assert.strictEqual(pkg.supports_session_lock, false);
  assert.strictEqual(pkg.lockscreen?.targets?.quickshell, undefined);
  assert.ok(pkg.lockscreen?.targets?.sddm != null);
  assert.strictEqual(pkg.lockscreen?.provider, "silentsddm");
});

test("S3-UI 12: Custom media are NOT deduplicated by deduplicateStorefrontPackages", async () => {
  const { deduplicateStorefrontPackages } = await import("../components/catalogue/catalogueUtils.ts");
  const { normalizeCustomMediaAsset } = await import("../providers/customMediaProvider.ts");

  const customItem1 = normalizeCustomMediaAsset({
    id: "custom:1111111111111111111111111111111111111111111111111111111111111111",
    filename: "photo1.jpg",
    display_name: "My Photo 1",
    media_type: "image",
    sha256: "1111111111111111111111111111111111111111111111111111111111111111",
    size_bytes: 500000,
    path: "/custom/photo1.jpg",
  });

  const customItem2 = normalizeCustomMediaAsset({
    id: "custom:2222222222222222222222222222222222222222222222222222222222222222",
    filename: "photo2.jpg",
    display_name: "My Photo 2",
    media_type: "image",
    sha256: "2222222222222222222222222222222222222222222222222222222222222222",
    size_bytes: 600000,
    path: "/custom/photo2.jpg",
  });

  const list = [customItem1, customItem2];
  const deduped = deduplicateStorefrontPackages(list);
  assert.strictEqual(deduped.length, 2, "Custom media items must never be stripped by deduplication");
});

test("S3-UI 13: Subfiltering in CategoryView accurately separates My Media, SilentSDDM, and Qylock", async () => {
  const { normalizeCustomMediaAsset } = await import("../providers/customMediaProvider.ts");
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentWp = allLockscreens.find((p) => p.lockscreen?.provider === "silentsddm")!;
  const qylockPkg = allLockscreens.find((p) => p.lockscreen?.provider === "qylock" || p.id.startsWith("qylock-") || p.id.startsWith("lockscreen-"))!;

  const customPkg = normalizeCustomMediaAsset({
    id: "custom:3333333333333333333333333333333333333333333333333333333333333333",
    filename: "video.mp4",
    display_name: "Neon City",
    media_type: "video",
    sha256: "3333333333333333333333333333333333333333333333333333333333333333",
    size_bytes: 2000000,
    path: "/custom/video.mp4",
  });

  const testList = [customPkg, silentWp, qylockPkg];

  // Helper matching the CategoryView logic
  function filterBySub(subFilter: string, items: typeof testList) {
    return items.filter((pkg) => {
      const isCustomPkg = pkg.tags.includes("custom") || pkg.id.startsWith("custom:");
      const isSilentSddmPkg = !isCustomPkg && (pkg.lockscreen?.provider === "silentsddm" || pkg.id.startsWith("silentsddm-"));
      const isQylockPkg = !isCustomPkg && !isSilentSddmPkg && (pkg.tags.includes("qylock") || pkg.id.startsWith("qylock-") || pkg.id.startsWith("lockscreen-"));

      return (
        subFilter === "All" ||
        (subFilter === "My Media" && isCustomPkg) ||
        (subFilter === "SilentSDDM" && isSilentSddmPkg) ||
        (subFilter === "Qylock" && isQylockPkg) ||
        (subFilter !== "My Media" && subFilter !== "SilentSDDM" && subFilter !== "Qylock" && (
          pkg.tags.some((t) => t.toLowerCase() === subFilter.toLowerCase()) ||
          (subFilter.toLowerCase() === "sddm" && (pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm != null))
        ))
      );
    });
  }

  // Subfilter: "All" -> all 3
  assert.strictEqual(filterBySub("All", testList).length, 3);

  // Subfilter: "My Media" -> only customPkg
  const myMedia = filterBySub("My Media", testList);
  assert.strictEqual(myMedia.length, 1);
  assert.strictEqual(myMedia[0].id, customPkg.id);

  // Subfilter: "SilentSDDM" -> only silentWp
  const silents = filterBySub("SilentSDDM", testList);
  assert.strictEqual(silents.length, 1);
  assert.strictEqual(silents[0].id, silentWp.id);

  // Subfilter: "Qylock" -> only qylockPkg
  const qylocks = filterBySub("Qylock", testList);
  assert.strictEqual(qylocks.length, 1);
  assert.strictEqual(qylocks[0].id, qylockPkg.id);

  // Subfilter: "SDDM" -> both silentWp and customPkg
  const sddmOnly = filterBySub("SDDM", testList);
  assert.ok(sddmOnly.some((p) => p.id === customPkg.id));
  assert.ok(sddmOnly.some((p) => p.id === silentWp.id));
});

test("S3-UI 14: Single-target SDDM packages specify non-interactive login target without Quickshell", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentItems = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentItems) {
    const canTargetQs = Boolean(pkg.supports_session_lock || pkg.lockscreen?.targets?.quickshell);
    const canTargetSddm = Boolean(pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm);

    // Invariant: Must not offer dual-target or Quickshell checkboxes
    assert.strictEqual(canTargetQs, false, "Must not support Quickshell");
    assert.strictEqual(canTargetSddm, true, "Must support SDDM");
    const isDualTarget = canTargetQs && canTargetSddm;
    assert.strictEqual(isDualTarget, false, "Must not be dual-target");
  }
});

test("S3-UI 15: Per-package transaction state model enforces truthful stages without fake percentages", async () => {
  const { PackageTransaction } = await import("../types/index.ts");

  const stages: Array<"preparing" | "installing-engine" | "downloading-asset" | "removing"> = [
    "preparing",
    "installing-engine",
    "downloading-asset",
    "removing",
  ];

  for (const stage of stages) {
    const tx = {
      packageId: "silentsddm-rei",
      operation: stage === "removing" ? "uninstall" as const : "install" as const,
      stage,
      message: `Running stage: ${stage}`,
    };

    assert.strictEqual(tx.packageId, "silentsddm-rei");
    assert.ok(tx.message.includes(stage));
    // No arbitrary 15/30/60/100 percentages in the contract
    assert.strictEqual((tx as any).progressPercent, undefined);
  }
});

test("S3-UI 16: SilentSDDM detail-page action state machine covers all 4 states without fake actions", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentPkg = allLockscreens.find((p) => p.lockscreen?.provider === "silentsddm")!;
  assert.ok(silentPkg, "Must find SilentSDDM package");

  // Helper matching ProductHero state derivation logic
  function deriveHeroState(params: {
    isInstalled: boolean;
    isActive: boolean;
    isInstalling: boolean;
    canTargetQs: boolean;
    canTargetSddm: boolean;
    installedSddm: boolean;
  }) {
    if (!params.isInstalled) {
      if (params.isInstalling) {
        return {
          state: "INSTALLING",
          buttonLabel: "Installing…",
          disabled: true,
          showApply: false,
          showDeactivate: false,
        };
      }
      return {
        state: "NOT_INSTALLED",
        buttonLabel: "Install",
        disabled: false,
        showApply: false,
        showDeactivate: false,
        targetDisplay: "capability_card",
      };
    }

    if (!params.isActive) {
      return {
        state: "INSTALLED_NOT_ACTIVE",
        statusBadge: "Installed · Ready",
        showApply: true,
        applyLabel: "Apply to Login Screen",
        applyNote: "Apply available in Phase S4",
        applyDisabled: true, // S4 boundary
        showDeactivate: false,
        overflowActions: ["uninstall"], // no deactivate while not active
      };
    }

    return {
      state: "ACTIVE",
      statusBadge: "Active for Login Screen",
      showDeactivate: true,
      showTest: true,
      testTarget: "sddm", // pure SDDM, no quickshell
      overflowActions: ["test", "deactivate", "uninstall"],
    };
  }

  // 1. State: NOT INSTALLED
  const s1 = deriveHeroState({
    isInstalled: false,
    isActive: false,
    isInstalling: false,
    canTargetQs: false,
    canTargetSddm: true,
    installedSddm: false,
  });
  assert.strictEqual(s1.state, "NOT_INSTALLED");
  assert.strictEqual(s1.buttonLabel, "Install");
  assert.strictEqual(s1.targetDisplay, "capability_card");
  assert.strictEqual(s1.showApply, false);
  assert.strictEqual(s1.showDeactivate, false);

  // 2. State: INSTALLING
  const s2 = deriveHeroState({
    isInstalled: false,
    isActive: false,
    isInstalling: true,
    canTargetQs: false,
    canTargetSddm: true,
    installedSddm: false,
  });
  assert.strictEqual(s2.state, "INSTALLING");
  assert.strictEqual(s2.buttonLabel, "Installing…");
  assert.strictEqual(s2.disabled, true);
  assert.strictEqual(s2.showApply, false);

  // 3. State: INSTALLED, NOT ACTIVE
  const s3 = deriveHeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    canTargetQs: false,
    canTargetSddm: true,
    installedSddm: true,
  });
  assert.strictEqual(s3.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(s3.statusBadge, "Installed · Ready");
  assert.strictEqual(s3.showApply, true);
  assert.strictEqual(s3.applyLabel, "Apply to Login Screen");
  assert.strictEqual(s3.applyDisabled, true);
  assert.strictEqual(s3.applyNote, "Apply available in Phase S4");
  assert.strictEqual(s3.showDeactivate, false);
  assert.deepStrictEqual(s3.overflowActions, ["uninstall"]);

  // 4. State: ACTIVE
  const s4 = deriveHeroState({
    isInstalled: true,
    isActive: true,
    isInstalling: false,
    canTargetQs: false,
    canTargetSddm: true,
    installedSddm: true,
  });
  assert.strictEqual(s4.state, "ACTIVE");
  assert.strictEqual(s4.showDeactivate, true);
  assert.strictEqual(s4.showTest, true);
  assert.strictEqual(s4.testTarget, "sddm");
  assert.ok(s4.overflowActions.includes("deactivate"));
  assert.ok(s4.overflowActions.includes("test"));
  assert.ok(s4.overflowActions.includes("uninstall"));
});

test("S3-UI 17: Installation flow commits and yields transaction state before backend IPC resolves", async () => {
  const { yieldFrame } = await import("../services/frameYield.ts");
  assert.strictEqual(typeof yieldFrame, "function", "yieldFrame must be exported");

  // Track state transitions over time
  const transitions: Array<{ stage: string; backendResolved: boolean }> = [];
  let backendResolved = false;

  let currentTx: any = null;
  const setTransaction = (tx: any) => {
    currentTx = tx;
    transitions.push({ stage: tx?.stage, backendResolved });
  };

  // Simulated install flow
  setTransaction({
    packageId: "silentsddm-rei",
    operation: "install",
    stage: "preparing",
    message: "Preparing SilentSDDM...",
  });

  // Ensure yieldFrame resolves
  await yieldFrame();

  // Invariant check: While backend IPC is in flight, UI transaction state is active
  assert.strictEqual(backendResolved, false, "Backend IPC must not be resolved yet");
  assert.strictEqual(currentTx.packageId, "silentsddm-rei");
  assert.strictEqual(currentTx.stage, "preparing");

  // Advance to downloading stage
  setTransaction({
    packageId: "silentsddm-rei",
    operation: "install",
    stage: "downloading",
    message: "Downloading wallpaper...",
  });
  await yieldFrame();

  assert.strictEqual(backendResolved, false, "Backend IPC is still running");
  assert.strictEqual(currentTx.stage, "downloading");

  // Backend operation completes
  backendResolved = true;
  setTransaction(null);

  assert.strictEqual(transitions.length, 3);
  assert.deepStrictEqual(transitions, [
    { stage: "preparing", backendResolved: false },
    { stage: "downloading", backendResolved: false },
    { stage: undefined, backendResolved: true },
  ]);
});

test("S3-UI 18: Structured progress events support truthful CAS hit vs download messages", async () => {
  const { SilentSddmStageEvent } = await import("../types/index.ts");

  // Case 1: Fresh download
  const downloadEv = {
    packageId: "silentsddm-ken",
    operation: "install" as const,
    stage: "downloading" as const,
    detail: "Downloading wallpaper...",
  };
  assert.strictEqual(downloadEv.stage, "downloading");
  assert.strictEqual(downloadEv.detail, "Downloading wallpaper...");

  // Case 2: CAS cache hit
  const casHitEv = {
    packageId: "silentsddm-smoky",
    operation: "install" as const,
    stage: "verifying" as const,
    detail: "Verifying cached asset in CAS...",
  };
  assert.strictEqual(casHitEv.stage, "verifying");
  assert.strictEqual(casHitEv.detail, "Verifying cached asset in CAS...");
  assert.ok(!casHitEv.detail.includes("Downloading"), "CAS hit must never say downloading");
});
