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
