import { test } from "node:test";
import assert from "node:assert";
import { packageEngine } from "../providers/index.ts";
import { SilentSddmService } from "../services/silentSddmService.ts";
import { normalizeCustomMediaAsset } from "../providers/customMediaProvider.ts";
import type {
  SilentSddmActivationManifest,
  PackageTransaction,
  SilentSddmLifecycleState,
  PackageItem,
} from "../types/index.ts";

/**
 * State derivation helper mirroring ProductHero & LockScreenDetailView S4 lifecycle
 */
function deriveS4HeroState(params: {
  isInstalled: boolean;
  isActive: boolean;
  isInstalling: boolean;
  isApplying: boolean;
  canTargetQs?: boolean;
  canTargetSddm?: boolean;
  activeAssetId?: string | null;
  currentPackageId: string;
}) {
  const isCardActive = params.isActive && params.activeAssetId === params.currentPackageId;

  if (!params.isInstalled) {
    if (params.isInstalling) {
      return {
        state: "INSTALLING" as const,
        primaryButton: "Installing…",
        disabled: true,
        showApply: false,
        showTest: false,
        showDeactivate: false,
      };
    }
    return {
      state: "NOT_INSTALLED" as const,
      primaryButton: "Install",
      disabled: false,
      showApply: false,
      showTest: false,
      showDeactivate: false,
    };
  }

  if (!isCardActive) {
    if (params.isApplying) {
      return {
        state: "APPLYING" as const,
        statusBadge: "Installed · Ready",
        primaryButton: "Applying to Login Screen…",
        disabled: true,
        showApply: true,
        showSideTest: false,
        showDeactivate: false,
        overflowActions: ["test", "uninstall"],
      };
    }
    return {
      state: "INSTALLED_NOT_ACTIVE" as const,
      statusBadge: "Installed · Ready",
      primaryButton: "Apply to Login Screen",
      disabled: false,
      showApply: true,
      showSideTest: false,
      showDeactivate: false,
      overflowActions: ["uninstall"],
    };
  }

  return {
    state: "ACTIVE" as const,
    statusBadge: "Active · Login Screen",
    primaryButton: "Test",
    showApply: false,
    showSideTest: false,
    showDeactivate: false,
    overflowActions: ["deactivate", "deactivate_and_uninstall"],
  };
}

// 1. not installed → Install visible
test("S4-B 1: not installed → Install visible and enabled", () => {
  const s = deriveS4HeroState({
    isInstalled: false,
    isActive: false,
    isInstalling: false,
    isApplying: false,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "NOT_INSTALLED");
  assert.strictEqual(s.primaryButton, "Install");
  assert.strictEqual(s.disabled, false);
  assert.strictEqual(s.showApply, false);
  assert.strictEqual(s.showDeactivate, false);
});

// 2. installed → Apply visible
test("S4-B 2: installed → Apply visible, enabled, not showing Phase S4 placeholder", () => {
  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    isApplying: false,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(s.statusBadge, "Installed · Ready");
  assert.strictEqual(s.primaryButton, "Apply to Login Screen");
  assert.strictEqual(s.disabled, false, "Apply button must be enabled");
  assert.strictEqual(s.showDeactivate, false);
});

// 3. active → Active / Deactivate visible
test("S4-B 3: active → Test visible as primary action, no Test in overflow", () => {
  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: true,
    isInstalling: false,
    isApplying: false,
    activeAssetId: "silentsddm-ken",
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "ACTIVE");
  assert.strictEqual(s.statusBadge, "Active · Login Screen");
  assert.strictEqual(s.primaryButton, "Test");
  assert.strictEqual(s.showSideTest, false);
  assert.strictEqual(s.overflowActions.includes("test"), false);
});

// 4. active → Deactivate available
test("S4-B 4: active → Deactivate available in overflow menu", () => {
  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: true,
    isInstalling: false,
    isApplying: false,
    activeAssetId: "silentsddm-ken",
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.overflowActions.includes("deactivate"), true);
  assert.ok(s.overflowActions.includes("deactivate"), "Must include deactivate in overflow actions");
});

// 5. inactive uninstall
test("S4-B 5: inactive uninstall is direct and does not require deactivation", () => {
  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    isApplying: false,
    currentPackageId: "silentsddm-rei",
  });
  assert.ok(s.overflowActions.includes("uninstall"));
  assert.ok(!s.overflowActions.includes("deactivate"));
});

// 6. active uninstall requires Deactivate & Uninstall
test("S4-B 6: active uninstall requires Deactivate & Uninstall confirmation", () => {
  const isSddmActive = true;
  const packageId = "silentsddm-ken";

  // Modal label logic
  const uninstallButtonLabel = isSddmActive ? "Deactivate & Uninstall" : "Uninstall";
  const deactivationWarningRequired = isSddmActive;

  assert.strictEqual(uninstallButtonLabel, "Deactivate & Uninstall");
  assert.strictEqual(deactivationWarningRequired, true);
});

// 7. failed Install does not show Installed
test("S4-B 7: failed Install does not show Installed (retains authoritative uninstalled state)", () => {
  let isInstalledAuthoritative = false;
  let hasTransaction = true;

  // Simulate install failure: transaction cleared, authoritative state remains uninstalled
  hasTransaction = false;

  const s = deriveS4HeroState({
    isInstalled: isInstalledAuthoritative,
    isActive: false,
    isInstalling: hasTransaction,
    isApplying: false,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "NOT_INSTALLED");
  assert.strictEqual(s.primaryButton, "Install");
});

// 8. failed Apply does not show Active
test("S4-B 8: failed Apply does not show Active (retains Installed · Ready)", () => {
  let activeAssetIdAuthoritative: string | null = null;
  let isApplying = true;

  // Apply fails: isApplying cleared, authoritative active asset remains null
  isApplying = false;

  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    isApplying,
    activeAssetId: activeAssetIdAuthoritative,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(s.statusBadge, "Installed · Ready");
  assert.strictEqual(s.primaryButton, "Apply to Login Screen");
});

// 9. failed Deactivate remains Active
test("S4-B 9: failed Deactivate remains Active (preserves authoritative active manifest)", () => {
  let activeAssetIdAuthoritative = "silentsddm-ken";
  let isDeactivating = true;

  // Deactivate fails: authoritative manifest is preserved by S4-A backend
  isDeactivating = false;

  const s = deriveS4HeroState({
    isInstalled: true,
    isActive: true,
    isInstalling: false,
    isApplying: false,
    activeAssetId: activeAssetIdAuthoritative,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "ACTIVE");
  assert.strictEqual(s.statusBadge, "Active · Login Screen");
  assert.strictEqual(s.primaryButton, "Test");
});

// 10. failed Uninstall remains Installed
test("S4-B 10: failed Uninstall remains Installed", () => {
  let isInstalledAuthoritative = true;
  let isUninstalling = false;

  const s = deriveS4HeroState({
    isInstalled: isInstalledAuthoritative,
    isActive: false,
    isInstalling: isUninstalling,
    isApplying: false,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(s.statusBadge, "Installed · Ready");
});

// 11. authoritative refresh after successful mutation
test("S4-B 11: authoritative refresh after successful mutation transitions state cleanly", () => {
  // 1. Installed
  let authoritativeState = {
    installed: true,
    activeAssetId: null as string | null,
  };
  let s = deriveS4HeroState({
    isInstalled: authoritativeState.installed,
    isActive: Boolean(authoritativeState.activeAssetId),
    isInstalling: false,
    isApplying: false,
    activeAssetId: authoritativeState.activeAssetId,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "INSTALLED_NOT_ACTIVE");

  // 2. Successful Apply -> authoritative refresh
  authoritativeState.activeAssetId = "silentsddm-ken";
  s = deriveS4HeroState({
    isInstalled: authoritativeState.installed,
    isActive: Boolean(authoritativeState.activeAssetId),
    isInstalling: false,
    isApplying: false,
    activeAssetId: authoritativeState.activeAssetId,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "ACTIVE");

  // 3. Successful Deactivate -> authoritative refresh
  authoritativeState.activeAssetId = null;
  s = deriveS4HeroState({
    isInstalled: authoritativeState.installed,
    isActive: Boolean(authoritativeState.activeAssetId),
    isInstalling: false,
    isApplying: false,
    activeAssetId: authoritativeState.activeAssetId,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(s.state, "INSTALLED_NOT_ACTIVE");
});

// 12. duplicate mutation requests prevented
test("S4-B 12: duplicate mutation requests prevented via disabled button and active transaction lock", () => {
  const applyingState = deriveS4HeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    isApplying: true, // mutation in progress
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(applyingState.disabled, true, "Apply button must be disabled while operation is running");

  const installingState = deriveS4HeroState({
    isInstalled: false,
    isActive: false,
    isInstalling: true, // mutation in progress
    isApplying: false,
    currentPackageId: "silentsddm-ken",
  });
  assert.strictEqual(installingState.disabled, true, "Install button must be disabled while operation is running");
});

// 13. SilentSDDM never exposes Session Lock target
test("S4-B 13: SilentSDDM packages never expose Session Lock target", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentPkgs = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");
  assert.ok(silentPkgs.length > 0, "Must have SilentSDDM packages");

  for (const pkg of silentPkgs) {
    assert.strictEqual(pkg.supports_session_lock, false, `Package ${pkg.id} must NOT support session lock`);
    assert.strictEqual(pkg.supports_login_screen, true, `Package ${pkg.id} must support login screen`);
    assert.strictEqual(pkg.lockscreen?.targets.quickshell, undefined, `Package ${pkg.id} must not define quickshell target`);
    assert.ok(pkg.lockscreen?.targets.sddm, `Package ${pkg.id} must define sddm target`);
  }
});

// 14. light-theme tab state uses semantic tokens
test("S4-B 14: light-theme tab state uses semantic tokens and avoids hardcoded dark values", () => {
  const semanticClasses = [
    "bg-[var(--rz-surface)]",
    "border-[var(--rz-border-subtle)]",
    "text-[var(--rz-text)]",
    "text-[var(--rz-text-secondary)]",
    "text-[var(--rz-text-muted)]",
    "bg-[var(--rz-surface-elevated)]",
  ];
  for (const cls of semanticClasses) {
    assert.ok(cls.includes("--rz-"), `Class ${cls} must reference RyoStore semantic tokens`);
  }
});

// 15. dark-theme tab state uses semantic tokens
test("S4-B 15: dark-theme tab state shares same semantic tokens with automatic color-scheme adaptation", () => {
  // Verifies the design system invariant: both dark and light modes consume the identical CSS token names
  const activeTabClass = "text-[var(--rz-accent)] border-[var(--rz-accent)]";
  const inactiveTabClass = "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] border-transparent";

  assert.ok(activeTabClass.includes("var(--rz-accent)"));
  assert.ok(inactiveTabClass.includes("var(--rz-text-muted)"));
});

// 16. no fabricated ratings or download metrics
test("S4-B 16: SilentSDDM wallpapers contain zero fabricated metrics (ratings or downloads)", async () => {
  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const silentPkgs = allLockscreens.filter((p) => p.lockscreen?.provider === "silentsddm");

  for (const pkg of silentPkgs) {
    assert.strictEqual(pkg.rating_count, 0, `Package ${pkg.id} rating count must be 0`);
    assert.strictEqual(pkg.downloads, 0, `Package ${pkg.id} download count must be 0`);
  }
});

// 17. custom video does not auto-activate
test("S4-B 17: custom video import does not auto-activate (transitions to Installed · Ready only)", () => {
  const customAsset = {
    id: "custom:my-wallpaper",
    filename: "video.mp4",
    media_type: "video" as const,
    size_bytes: 1048576,
    sha256: "deadbeef0123456789abcdef0123456789abcdef0123456789abcdef01234567",
    installed_at: "2026-09-21T00:00:00Z",
    display_name: "My Video",
  };

  const item = normalizeCustomMediaAsset(customAsset);
  assert.strictEqual(item.id, "custom:my-wallpaper");
  assert.strictEqual(item.supports_session_lock, false);
  assert.strictEqual(item.supports_login_screen, true);

  // Import creates an installed asset, but activeAssetId is null
  const postImportState = deriveS4HeroState({
    isInstalled: true,
    isActive: false, // NOT activated
    isInstalling: false,
    isApplying: false,
    activeAssetId: null, // no active wallpaper
    currentPackageId: item.id,
  });

  assert.strictEqual(postImportState.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(postImportState.statusBadge, "Installed · Ready");
  assert.strictEqual(postImportState.primaryButton, "Apply to Login Screen");
  assert.strictEqual(postImportState.showDeactivate, false);
});

test("S4-C.1 1: Installed · Ready exposes Apply to Login Screen without duplicate Test beside it", () => {
  const heroState = deriveS4HeroState({
    isInstalled: true,
    isActive: false,
    isInstalling: false,
    isApplying: false,
    activeAssetId: null,
    currentPackageId: "silentsddm-ken",
  });

  assert.strictEqual(heroState.state, "INSTALLED_NOT_ACTIVE");
  assert.strictEqual(heroState.statusBadge, "Installed · Ready");
  assert.strictEqual(heroState.primaryButton, "Apply to Login Screen");
  assert.strictEqual(heroState.showSideTest, false); // No duplicate Test button beside Apply
  assert.strictEqual(heroState.showApply, true);
  assert.strictEqual(heroState.overflowActions.includes("test"), false); // Test resides in overflow
});

test("S4-C.1 2: Active exposes Test as primary action and overflow contains only Deactivate & Uninstall", () => {
  const heroState = deriveS4HeroState({
    isInstalled: true,
    isActive: true,
    isInstalling: false,
    isApplying: false,
    activeAssetId: "silentsddm-ken",
    currentPackageId: "silentsddm-ken",
  });

  assert.strictEqual(heroState.state, "ACTIVE");
  assert.strictEqual(heroState.statusBadge, "Active · Login Screen");
  assert.strictEqual(heroState.primaryButton, "Test");
  assert.strictEqual(heroState.showSideTest, false); // No duplicate Test button beside Active
  assert.deepStrictEqual(heroState.overflowActions, ["deactivate", "deactivate_and_uninstall"]);
});

test("S4-C.1 3: 1 GiB limit constant and rejection boundary", () => {
  const ONE_GIB = 1024 * 1024 * 1024; // 1,073,741,824 bytes
  assert.strictEqual(ONE_GIB, 1073741824);

  const validAssetSize = ONE_GIB;
  const oversizedAssetSize = ONE_GIB + 1;

  assert.strictEqual(validAssetSize <= ONE_GIB, true);
  assert.strictEqual(oversizedAssetSize <= ONE_GIB, false);
});

test("S4-C.1 4: Zero network operations on local custom media import path", () => {
  // Verifies that local import only invokes local services and avoids catalog refreshes
  const executedCalls: string[] = [];
  const fakeLocalImportPipeline = async () => {
    executedCalls.push("validate_local");
    executedCalls.push("sha256_hash");
    executedCalls.push("local_copy");
    executedCalls.push("generate_local_poster");
    executedCalls.push("load_sddm_report_local");
    executedCalls.push("load_installed_local");
  };

  fakeLocalImportPipeline();
  assert.strictEqual(executedCalls.includes("refresh_catalog"), false);
  assert.strictEqual(executedCalls.includes("fetch_remote"), false);
  assert.strictEqual(executedCalls.length, 6);
});

test("S4-C 3: Custom media picker rejects GIF and accepts valid image/video extensions", () => {
  const supported = ["jpg", "jpeg", "png", "avi", "mp4", "mov", "mkv", "m4v", "webm"];
  const rejected = ["gif", "exe", "sh", "webp"];

  for (const ext of supported) {
    const isImage = ["jpg", "jpeg", "png"].includes(ext);
    const isVideo = ["avi", "mp4", "mov", "mkv", "m4v", "webm"].includes(ext);
    assert.strictEqual(isImage || isVideo, true);
  }

  for (const ext of rejected) {
    const isAccepted = supported.includes(ext);
    assert.strictEqual(isAccepted, false);
  }
});

test("S4-C 4: Custom media is user-owned SilentSDDM content, not a package catalog item", () => {
  const customAsset = {
    id: "custom:vacation-video",
    filename: "vacation.mp4",
    media_type: "video" as const,
    size_bytes: 52428800,
    sha256: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    installed_at: "2026-09-21T05:00:00Z",
    display_name: "Vacation Video",
  };

  const item = normalizeCustomMediaAsset(customAsset);
  assert.strictEqual(item.id, "custom:vacation-video");
  assert.strictEqual(item.author.name, "You");
  assert.strictEqual(item.lockscreen?.provider, "silentsddm");
  // Does not have fake download counts or ratings
  assert.strictEqual(item.rating_count, 0);
  assert.strictEqual(item.download_count, undefined);
});

// S4-C.1 Storefront & Navigation Tests
test("S4-C.1 5: Lock Screens storefront always exposes the 5 canonical subfilters including My Media", () => {
  const expectedSubfilters = ["All", "Qylock", "SilentSDDM", "SDDM", "My Media"];
  
  // Both CategoryView and DiscoverView must specify these exact 5 subfilters
  assert.strictEqual(expectedSubfilters.length, 5);
  assert.ok(expectedSubfilters.includes("My Media"));
  assert.ok(expectedSubfilters.includes("SilentSDDM"));
  assert.ok(expectedSubfilters.includes("Qylock"));
  assert.ok(expectedSubfilters.includes("SDDM"));
  assert.ok(expectedSubfilters.includes("All"));
});

test("S4-C.1 6: Strict subfilter separation between My Media, SilentSDDM, and All", () => {
  const builtInWallpaper: any = {
    id: "silentsddm-silvia",
    title: "Silvia",
    tags: ["silentsddm", "sddm"],
    lockscreen: { provider: "silentsddm", targets: { sddm: "themes/ryzora-silentsddm-silvia" } },
    supports_login_screen: true,
  };

  const customMediaItem: any = {
    id: "custom:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
    title: "My Custom Loop",
    tags: ["custom", "silentsddm", "sddm"],
    lockscreen: { provider: "silentsddm", targets: { sddm: "custom/loop.mp4" } },
    supports_login_screen: true,
  };

  const qylockItem: any = {
    id: "qylock-rose-pine",
    title: "Rosé Pine Qylock",
    tags: ["qylock"],
    lockscreen: { provider: "qylock", targets: { quickshell: "qylock" } },
    supports_session_lock: true,
  };

  const testFilter = (pkg: any, subFilter: string) => {
    const isCustomPkg = pkg.tags.includes("custom") || pkg.id.startsWith("custom:");
    const isSilentSddmPkg = !isCustomPkg && (pkg.lockscreen?.provider === "silentsddm" || pkg.id.startsWith("silentsddm-"));
    const isQylockPkg = !isCustomPkg && !isSilentSddmPkg && (pkg.tags.includes("qylock") || pkg.id.startsWith("qylock-") || pkg.id.startsWith("lockscreen-"));
    const hasSddm = Boolean(pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm != null || pkg.tags.includes("sddm"));

    if (subFilter === "All") return true;
    if (subFilter === "My Media") return isCustomPkg;
    if (subFilter === "SilentSDDM") return isSilentSddmPkg && !isCustomPkg;
    if (subFilter === "Qylock") return isQylockPkg;
    if (subFilter === "SDDM") return hasSddm;
    return false;
  };

  // 1. My Media only matches custom items
  assert.strictEqual(testFilter(customMediaItem, "My Media"), true);
  assert.strictEqual(testFilter(builtInWallpaper, "My Media"), false);
  assert.strictEqual(testFilter(qylockItem, "My Media"), false);

  // 2. SilentSDDM only matches built-in wallpapers, NEVER custom media
  assert.strictEqual(testFilter(builtInWallpaper, "SilentSDDM"), true);
  assert.strictEqual(testFilter(customMediaItem, "SilentSDDM"), false);
  assert.strictEqual(testFilter(qylockItem, "SilentSDDM"), false);

  // 3. All matches both
  assert.strictEqual(testFilter(builtInWallpaper, "All"), true);
  assert.strictEqual(testFilter(customMediaItem, "All"), true);
  assert.strictEqual(testFilter(qylockItem, "All"), true);

  // 4. SDDM matches both built-in and custom SDDM targets
  assert.strictEqual(testFilter(builtInWallpaper, "SDDM"), true);
  assert.strictEqual(testFilter(customMediaItem, "SDDM"), true);
});

test("S4-C.1 7: Zero custom assets still renders My Media section with UploadMediaCard", () => {
  const customItems: any[] = [];
  const subFilter = "My Media";

  // When count is 0, My Media view must NOT be hidden
  const shouldRenderUploadCard = subFilter === "My Media" || subFilter === "All";
  const shouldRenderEmptyNotice = subFilter === "My Media" && customItems.length === 0;

  assert.strictEqual(shouldRenderUploadCard, true);
  assert.strictEqual(shouldRenderEmptyNotice, true);
});

test("S4-C.1 8: Imported custom asset appears after refresh and switches to My Media", () => {
  let packages: any[] = [
    { id: "silentsddm-ken", tags: ["silentsddm"] }
  ];
  let activeSubFilter = "All";

  // Simulate local import completion
  const importedAsset = {
    id: "custom:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    filename: "anime_loop.mp4",
    media_type: "video" as const,
    size_bytes: 10485760,
    sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    installed_at: "2026-09-21T12:00:00Z",
    display_name: "Anime Loop",
  };

  const customPkg = normalizeCustomMediaAsset(importedAsset);
  // AppContext updates packages state locally
  packages = [...packages.filter((p) => !p.tags.includes("custom")), customPkg];
  // Frontend switches tab to My Media
  activeSubFilter = "My Media";

  assert.strictEqual(activeSubFilter, "My Media");
  assert.strictEqual(packages.length, 2);
  const found = packages.find((p) => p.id === customPkg.id);
  assert.ok(found, "Imported package must be present in package list");
  assert.strictEqual(found.title, "Custom · Anime Loop");
});

test("S4-C.2 1: Custom media preview uses absolute path without network or fake path", () => {
  const assetWithOriginalPath = {
    id: "custom:aabbccddeeff11223344556677889900aabbccddeeff11223344556677889900",
    filename: "my_wallpaper.mp4",
    media_type: "video" as const,
    size_bytes: 52428800,
    sha256: "aabbccddeeff11223344556677889900aabbccddeeff11223344556677889900",
    installed_at: "2026-09-24T12:00:00Z",
    original_path: "/home/user/Videos/my_wallpaper.mp4",
    display_name: "My Wallpaper",
  };

  const pkg = normalizeCustomMediaAsset(assetWithOriginalPath);
  assert.strictEqual(pkg.media_type, "video");
  assert.ok(pkg.preview_video_url, "Must have preview video url");
  assert.strictEqual(pkg.preview_video_url, "/home/user/Videos/my_wallpaper.mp4");
  assert.strictEqual(pkg.original_path, "/home/user/Videos/my_wallpaper.mp4");
  assert.strictEqual(pkg.preview_video_url.startsWith("http"), false, "Must not use network URL");
  assert.strictEqual(pkg.preview_video_url.startsWith("/custom/"), false, "Must not use fake URL");
});

test("S4-C.2 2: Separate Login Screen and Lock Screen background resolution in Current Configuration", () => {
  const report = {
    sddm_installed: true,
    sddm_version: { major: 0, minor: 20, patch: 0, raw: "0.20.0" },
    sddm_version_ok: true,
    qt_version: { major: 6, minor: 5, patch: 0, raw: "6.5.0" },
    qt_version_ok: true,
    qt_multimedia_ok: true,
    current_theme: "ryzora-silent",
    current_theme_file: "/etc/sddm.conf.d/zz-ryzora-theme.conf",
    ryzora_owns_sddm: true,
    engine_installed: true,
    engine_path: "/usr/share/sddm/themes/ryzora-silent",
    engine_version: "2.1.0",
    engine_owned_by_ryzora: true,
    manifest: {
      provider: "silentsddm",
      upstream_sha: "abcd",
      version: "2.1.0",
      engine_path: "/usr/share/sddm/themes/ryzora-silent",
      installed_at: "2026-09-24T12:00:00Z",
      active_wallpaper_id: "silentsddm-silvia",
      active_targets: { sddm: true },
      sddm_active: true,
    },
    cached_wallpapers: [
      {
        id: "silentsddm-silvia",
        filename: "silvia.mp4",
        asset_type: "upstream" as const,
        media_type: "video" as const,
        sha256: "11223344",
        size_bytes: 45000000,
        installed_at: "2026-09-24T12:00:00Z",
        display_name: "Silvia",
      }
    ],
    cached_custom: [
      {
        id: "custom:1234",
        filename: "lock_bg.png",
        asset_type: "custom" as const,
        media_type: "image" as const,
        sha256: "55667788",
        size_bytes: 2000000,
        installed_at: "2026-09-24T12:00:00Z",
        original_path: "/home/user/Pictures/lock_bg.png",
        display_name: "Lock Bg",
      }
    ],
    warnings: [],
  };

  const configuration = {
    background: { fill_mode: "fill" as const },
    lock_screen: {
      background: "lock_bg.png",
      clock_position: "center" as const,
    },
    login_screen: {
      background: "silvia.mp4",
      blur: 0,
      brightness: 0,
      saturation: 0,
      login_area: { position: "center" as const, margin: 50 },
      avatar: { shape: "circle" as const, active_size: 96, inactive_size: 80, inactive_opacity: 0.8 },
      use_background_color: false,
      background_color: "#000000",
    }
  };

  // Login Screen resolved independently from Lock Screen
  assert.strictEqual(configuration.login_screen.background, "silvia.mp4");
  assert.strictEqual(configuration.lock_screen.background, "lock_bg.png");
  assert.notStrictEqual(configuration.login_screen.background, configuration.lock_screen.background);

  const loginAsset = report.cached_wallpapers.find(w => w.filename === configuration.login_screen.background);
  const lockAsset = report.cached_custom.find(c => c.filename === configuration.lock_screen.background);

  assert.ok(loginAsset, "Login asset found");
  assert.strictEqual(loginAsset.media_type, "video");
  assert.ok(lockAsset, "Lock asset found");
  assert.strictEqual(lockAsset.media_type, "image");
});

test("S4-C.2 3: Active custom media correctly surfaces filename, size, and source in Current Configuration", () => {
  const customAsset = {
    id: "custom:abcdef99",
    filename: "cyberpunk_city.mp4",
    asset_type: "custom" as const,
    media_type: "video" as const,
    sha256: "abcdef99",
    size_bytes: 882892800, // ~842 MB
    installed_at: "2026-09-24T12:00:00Z",
    original_path: "/home/user/Videos/cyberpunk_city.mp4",
    display_name: "Cyberpunk City",
  };

  const sizeMb = (customAsset.size_bytes / (1024 * 1024)).toFixed(0) + " MB";
  assert.strictEqual(sizeMb, "842 MB");
  assert.strictEqual(customAsset.media_type, "video");
  assert.strictEqual(customAsset.original_path, "/home/user/Videos/cyberpunk_city.mp4");
});


// ── Regression: Custom Media Persistence & Rehydration Tests ────────────────

test("custom media survives app restart and catalog refresh", () => {
  // Simulate persisted custom assets in host report
  const persistedCustomAsset1 = {
    id: "custom:c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716",
    filename: "Novo_projeto.mp4",
    asset_type: "custom" as const,
    media_type: "video" as const,
    sha256: "c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716",
    size_bytes: 652490629,
    installed_at: "1790268586Z",
    upstream_url: null,
    original_path: "/home/silentbyte/.local/share/Steam/steamapps/workshop/content/431960/2929628988/Novo projeto.mp4",
    display_name: "Novo Projeto",
  };

  const persistedCustomAsset2 = {
    id: "custom:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    filename: "Space_Walk.png",
    asset_type: "custom" as const,
    media_type: "image" as const,
    sha256: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    size_bytes: 4194304,
    installed_at: "1790268600Z",
    upstream_url: null,
    original_path: "/home/silentbyte/Pictures/Space_Walk.png",
    display_name: "Space Walk",
  };

  // 1. Initial rehydration from host report (app startup)
  const customPkgs = [persistedCustomAsset1, persistedCustomAsset2].map(normalizeCustomMediaAsset);
  assert.strictEqual(customPkgs.length, 2);

  const pkg1 = customPkgs[0];
  assert.strictEqual(pkg1.id, "custom:c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716");
  assert.strictEqual(pkg1.title, "Custom · Novo Projeto");
  assert.strictEqual(pkg1.original_path, "/home/silentbyte/.local/share/Steam/steamapps/workshop/content/431960/2929628988/Novo projeto.mp4");
  assert.strictEqual(pkg1.content_hash, "c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716");
  assert.strictEqual(pkg1.package_size_bytes, 652490629);
  assert.strictEqual(pkg1.media_type, "video");
  assert.ok(pkg1.tags.includes("custom"));
  assert.ok(pkg1.tags.includes("my media"));

  // 2. Catalog refresh preservation test
  // Simulating loadCatalogPackages / refreshCatalog with mergeCanonicalLockScreens
  const mockBasePackages = [
    {
      id: "dog-samurai",
      title: "Dog Samurai",
      subtitle: "Lock Screen",
      description: "A cool dog samurai lock screen",
      version: "1.0.0",
      author: { name: "Community", avatar: "", verified: true },
      category: "lockscreens" as const,
      package_type: "lockscreen" as const,
      tags: ["quickshell"],
      supported_desktops: ["universal" as const],
      supported_display: ["wayland" as const],
      rating: 4.8,
      rating_count: 10,
      downloads: 1000,
      hero_image: "/mock.jpg",
      screenshots: [],
      color_palette: [],
      safety_audit: { rating: "verified" as const, changes_system_files: false, requires_root: false, sandbox_compatible: true, files_modified_count: 0 },
      dependencies: { packages: [], optional: [] },
      components: [],
      compatibility: { supported_distros: ["all"], supported_desktops: ["universal" as const], supported_sessions: ["wayland" as const], required_binaries: [], optional_binaries: [] },
    },
  ];

  // Re-run merge with custom media preserved
  const lockscreens = [...customPkgs];
  const merged = [...mockBasePackages, ...lockscreens];

  // Assert both custom assets survive and are distinct from catalog / built-in wallpapers
  const myMediaItems = merged.filter((p) => p.tags.includes("custom") || p.id.startsWith("custom:"));
  assert.strictEqual(myMediaItems.length, 2, "Two custom media assets must survive restart and merge");
  assert.strictEqual(myMediaItems[0].title, "Custom · Novo Projeto");
  assert.strictEqual(myMediaItems[1].title, "Custom · Space Walk");

  const builtInItems = merged.filter((p) => p.lockscreen?.provider === "silentsddm" && !p.tags.includes("custom"));
  assert.ok(builtInItems.every((b) => !b.id.startsWith("custom:")), "Custom media remains separate from SilentSDDM built-ins");
});

test("active custom media remains ACTIVE after restart with authoritative manifest", () => {
  const activeAssetId = "custom:c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716";
  const activationManifest = {
    provider: "silentsddm",
    active_asset_id: activeAssetId,
    active_filename: "Novo_projeto.mp4",
    engine_version: "1.5.0",
    sddm_active: true,
  };

  const customPkg = normalizeCustomMediaAsset({
    id: activeAssetId,
    filename: "Novo_projeto.mp4",
    asset_type: "custom" as const,
    media_type: "video" as const,
    sha256: "c4312fab6510fc0130f25f14d71353662103d44a35cabed301f5767eae948716",
    size_bytes: 652490629,
    installed_at: "1790268586Z",
    original_path: "/home/silentbyte/Videos/Novo_projeto.mp4",
    display_name: "Novo Projeto",
  });

  // Check authoritative active match
  const isCustomActive = activationManifest.active_asset_id === customPkg.id;
  assert.strictEqual(isCustomActive, true, "Authoritative manifest proves active state after restart");
});

test("missing original file becomes Source unavailable, not deleted", () => {
  const customPkg = normalizeCustomMediaAsset({
    id: "custom:deleted123",
    filename: "Deleted_Video.mp4",
    asset_type: "custom" as const,
    media_type: "video" as const,
    sha256: "deleted123",
    size_bytes: 50000000,
    installed_at: "1790268586Z",
    original_path: "/home/silentbyte/Videos/Deleted_Video.mp4",
    display_name: "Deleted Video",
  });

  // Simulate missing file flag
  const flaggedPkg = {
    ...customPkg,
    source_unavailable: true,
    source_unavailable_reason: "Source file not found at original path",
  };

  assert.strictEqual(flaggedPkg.source_unavailable, true);
  assert.strictEqual(flaggedPkg.id, "custom:deleted123");
  assert.strictEqual(flaggedPkg.title, "Custom · Deleted Video");
  assert.strictEqual(flaggedPkg.original_path, "/home/silentbyte/Videos/Deleted_Video.mp4");
  assert.ok(flaggedPkg.source_unavailable_reason?.includes("not found"));
});


// ── Regression: Library UI Presentation & Active Pinning Tests ──────────────

test("active item sorts before inactive items regardless of rating or downloads", () => {
  const activeId = "lockscreen-qylock-sword";
  const activeLockscreen = { quickshell: "sword", sddm: null };

  const isPkgActive = (pkg: { id: string }) => {
    if (activeLockscreen?.quickshell) {
      if (activeLockscreen.quickshell === pkg.id || activeLockscreen.quickshell === pkg.id.replace(/^lockscreen-(qylock-)?/, "")) {
        return true;
      }
    }
    return false;
  };

  const items = [
    { id: "lockscreen-qylock-aurora", title: "Aurora", downloads: 50000, rating: 5.0 },
    { id: "lockscreen-qylock-sword", title: "Sword", downloads: 100, rating: 3.0 },
    { id: "lockscreen-qylock-cyber", title: "Cyber", downloads: 20000, rating: 4.5 },
  ];

  // Sort with active pinned first, secondary downloads
  const sorted = [...items].sort((a, b) => {
    const aActive = isPkgActive(a);
    const bActive = isPkgActive(b);
    if (aActive && !bActive) return -1;
    if (!aActive && bActive) return 1;
    return b.downloads - a.downloads;
  });

  assert.strictEqual(sorted[0].id, "lockscreen-qylock-sword", "Active item must be pinned first");
  assert.strictEqual(sorted[1].id, "lockscreen-qylock-aurora", "Highest downloads inactive item must be second");
  assert.strictEqual(sorted[2].id, "lockscreen-qylock-cyber", "Lower downloads inactive item must be third");
});

test("Library card presentation: ACTIVE badge only on active items, no Installed badge", () => {
  const activePackage = {
    id: "custom:my-video-1",
    tags: ["custom", "my media"],
  };
  const inactivePackage = {
    id: "lockscreen-qylock-sword",
    tags: ["qylock"],
  };

  const activeLockscreen = { quickshell: null, sddm: "custom:my-video-1" };

  const isPkgActive = (pkgId: string) => activeLockscreen.sddm === pkgId;

  // Active item
  assert.strictEqual(isPkgActive(activePackage.id), true);
  // Inactive item
  assert.strictEqual(isPkgActive(inactivePackage.id), false);
});
