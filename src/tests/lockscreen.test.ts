import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import assert from "node:assert/strict";
import {
  normalizeQylockTheme,
  resolveLockscreenCapabilities,
  getCatalogueLockScreens,
  RAW_QYLOCK_THEMES,
  type RawQylockTheme,
} from "../providers/qylockProvider.ts";
import {
  isLoginScreen,
  isSessionLock,
  getPackageSubtype,
  getConciseCompatibility,
} from "../components/catalogue/catalogueUtils.ts";
import type { PackageItem } from "../types/index.ts";
import type { SystemInfo } from "../types/index.ts";

const mockSystemHyprland: SystemInfo = {
  hostname: "archlinux",
  os_name: "Arch Linux",
  kernel_version: "6.10.3-arch1-1",
  desktop_environment: "hyprland",
  session_type: "wayland",
  active_theme: "default",
  uptime_seconds: 7200,
  cpu_model: "AMD Ryzen 7",
  cpu_cores: 8,
  memory_total_bytes: 34359738368,
  memory_used_bytes: 8589934592,
  distro_id: "arch",
  window_manager: "Hyprland",
};

const mockSystemKde: SystemInfo = {
  ...mockSystemHyprland,
  desktop_environment: "kde",
  window_manager: "KWin",
};

const mockSystemX11: SystemInfo = {
  ...mockSystemHyprland,
  session_type: "x11",
};

test("1. Qylock metadata normalization strictly separates Preview Media from Runtime Assets", () => {
  const rawTheme = RAW_QYLOCK_THEMES.find((t) => t.id === "dog-samurai")!;
  assert.ok(rawTheme, "Dog Samurai raw theme must exist");

  const pkg = normalizeQylockTheme(rawTheme);

  // Identity & core fields
  assert.equal(pkg.id, "dog-samurai");
  assert.equal(pkg.title, "Dog Samurai");
  assert.equal(pkg.category, "lockscreens");
  assert.equal(pkg.package_type, "lockscreen");

  // Lockscreen manifest must exist
  const manifest = pkg.lockscreen;
  assert.ok(manifest, "Lockscreen manifest must be defined");

  // Media preview separation
  assert.ok(manifest.media.poster, "Poster must be defined");
  assert.equal(manifest.media.preview_video, rawTheme.preview_video);
  assert.equal(pkg.media_type, "video");

  // Runtime assets separation
  assert.equal(manifest.runtime.entrypoint, "Main.qml");
  assert.ok(manifest.runtime.assets.includes("bg.mp4"));
  assert.ok(manifest.runtime.assets.includes("theme.conf"));
  assert.equal(manifest.runtime.requires_multimedia, true);

  // Provenance
  assert.equal(manifest.provenance.provider_name, "qylock");
  assert.equal(manifest.provenance.author, "Darkkal44");
});

test("2. Video and animated previews provide valid fallback posters", () => {
  RAW_QYLOCK_THEMES.forEach((raw) => {
    const pkg = normalizeQylockTheme(raw);
    assert.ok(pkg.hero_image, `${pkg.id} must have a valid hero/poster image`);
    assert.ok(pkg.lockscreen?.media.poster, `${pkg.id} manifest must have poster fallback`);
    if (pkg.media_type === "video") {
      assert.ok(pkg.preview_video_url, `${pkg.id} video package must provide preview_video_url`);
    }
  });
});

test("3. Capability detection flags Wayland vs X11 correctly", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemX11, dogSamurai.lockscreen, "quickshell");

  assert.equal(res.session_lock_supported, false);
  assert.ok(
    res.warnings.some((w) => w.includes("Wayland")),
    "Must warn that Quickshell lockscreen requires Wayland"
  );
});

test("4. Capability detection blocks Quickshell on KDE Plasma due to ext-session-lock-v1 protocol absence", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemKde, dogSamurai.lockscreen, "quickshell");

  assert.equal(res.session_lock_supported, false);
  assert.equal(res.can_install, false);
  assert.ok(
    res.warnings.some((w) => w.includes("ext-session-lock-v1 is unsupported on KDE")),
    "Must explicitly explain ext-session-lock-v1 is unsupported on KDE"
  );
});

test("5. Quickshell target selection sets user-space destination and no root requirement", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "quickshell");

  assert.equal(res.can_install, true);
  assert.equal(res.requires_root, false);
  assert.equal(res.privilege_notice, null);

  // Must target user directory
  assert.ok(
    res.target_files.every((f) => !f.is_system),
    "All Quickshell target files must be user-space"
  );
  assert.ok(
    res.target_files.some((f) => f.target.includes(".local/share/ryzora/lockscreens/qylock/dog-samurai")),
    "Target path must be Ryzora-owned user directory"
  );
  assert.ok(
    res.supported_dependencies.includes("quickshell"),
    "Must require Quickshell dependency"
  );
});

test("6. SDDM target selection sets system destination and enforces root boundary", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "sddm");

  assert.equal(res.can_install, true);
  assert.equal(res.requires_root, true);
  assert.ok(res.privilege_notice, "Privilege notice must be present for SDDM");
  assert.ok(
    res.privilege_notice.includes("Administrator permissions (pkexec)"),
    "Must mention pkexec/admin privilege"
  );

  // Must target system directory
  assert.ok(
    res.target_files.every((f) => f.is_system),
    "All SDDM target files must be marked as system files"
  );
  assert.ok(
    res.target_files.some((f) => f.target.startsWith("/usr/share/sddm/themes/ryzora-dog-samurai")),
    "Target path must be in /usr/share/sddm/themes/"
  );
  assert.ok(
    res.config_files.some((c) => c.path === "/etc/sddm.conf.d/ryzora-theme.conf"),
    "Config path must target /etc/sddm.conf.d/ryzora-theme.conf"
  );
});

test("7. Both-target selection combines user session and system greeter files", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "both");

  assert.equal(res.requires_root, true);
  const userFiles = res.target_files.filter((f) => !f.is_system);
  const sysFiles = res.target_files.filter((f) => f.is_system);

  assert.ok(userFiles.length > 0, "Must have user-space Quickshell files");
  assert.ok(sysFiles.length > 0, "Must have system-space SDDM files");
  assert.ok(res.supported_dependencies.includes("quickshell"));
  assert.ok(res.supported_dependencies.includes("sddm"));
});

test("8. Missing dependency state correctly reports required binaries", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "quickshell");

  assert.ok(res.supported_dependencies.includes("quickshell"));
  assert.ok(res.supported_dependencies.includes("qt6-declarative"));
  assert.ok(res.supported_dependencies.includes("qt6-multimedia"));
  assert.ok(res.supported_dependencies.includes("gst-plugins-good"));
});

test("9. Privileged SDDM boundary never silently escalates permissions", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const quickshellRes = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "quickshell");
  const sddmRes = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "sddm");

  assert.equal(quickshellRes.requires_root, false, "Session lock must never request root");
  assert.equal(sddmRes.requires_root, true, "Login screen must strictly declare root requirement");
});

test("10. Self-contained installation: all files target Ryzora-owned directories", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const res = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "quickshell");

  for (const file of res.target_files) {
    assert.ok(
      file.target.startsWith("~/.local/share/ryzora/lockscreens/qylock/"),
      `File ${file.target} must be inside ~/.local/share/ryzora/lockscreens/`
    );
  }
});

test("11. Uninstall ownership/cleanup: targets are namespaced by theme ID", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const tape = normalizeQylockTheme(RAW_QYLOCK_THEMES[1]);

  const dogRes = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen, "quickshell");
  const tapeRes = resolveLockscreenCapabilities(mockSystemHyprland, tape.lockscreen, "quickshell");

  const dogDest = dogRes.target_files[0].target;
  const tapeDest = tapeRes.target_files[0].target;

  assert.ok(dogDest.includes("/dog-samurai/"));
  assert.ok(tapeDest.includes("/clockwork-tape/"));
  assert.notEqual(dogDest, tapeDest, "Different themes must occupy distinct namespaced folders for atomic cleanup");
});

test("12. No external/symlinked runtime dependency: self-contained materialization", () => {
  const dogSamurai = normalizeQylockTheme(RAW_QYLOCK_THEMES[0]);
  const manifest = dogSamurai.lockscreen!;

  // Verify runtime assets contain everything needed to run without external repo
  assert.ok(manifest.runtime.assets.includes("Main.qml"));
  assert.ok(manifest.runtime.assets.includes("theme.conf"));
  assert.ok(manifest.runtime.assets.includes("metadata.desktop"));

  // Destination must not reference /tmp or temporary download directories
  assert.ok(!manifest.targets.quickshell?.install_destination.includes("/tmp"));
  assert.ok(!manifest.targets.quickshell?.install_destination.includes("Downloads"));
});

test("13. Catalogue Data Flow: Provider normalized entries reach the catalogue and pass Lock Screens filter", () => {
  const catalogueLockScreens = getCatalogueLockScreens();

  // Must contain all 6 Qylock/native entries requested by user
  const expectedTitles = [
    "Dog Samurai",
    "Tape (Clockwork)",
    "NieR: Automata",
    "Forest",
    "Material You",
    "Aurora Glass Hyprlock",
    "Swaylock Effects Blur",
  ];

  assert.equal(catalogueLockScreens.length, 7, "Catalogue must have 7 canonical lockscreens");
  for (const title of expectedTitles) {
    const found = catalogueLockScreens.find((p) => p.title === title);
    assert.ok(found, `Expected lockscreen title "${title}" must exist in canonical catalogue`);
    assert.ok(
      found.category === "lockscreens" || found.package_type === "lockscreen",
      `${title} must be categorized as lockscreen`
    );
  }

  // Simulate AppContext mergeCanonicalLockScreens with empty or repository base
  const mockRepoPackages: PackageItem[] = [
    {
      id: "lockscreen-hyprlock-aurora",
      title: "Aurora Glass Hyprlock",
      subtitle: "NorthLights",
      description: "Clean lockscreen",
      version: "1.0.0",
      author: { name: "NorthLights", avatar: "", verified: true },
      category: "lockscreens",
      package_type: "lockscreen",
      tags: ["hyprlock"],
      supported_desktops: ["hyprland"],
      supported_display: ["wayland"],
      rating: 4.8,
      rating_count: 50,
      downloads: 2000,
      hero_image: "",
      screenshots: [],
      color_palette: [],
      safety_audit: { rating: "safe", changes_system_files: false, requires_root: false, sandbox_compatible: true, files_modified_count: 1 },
      dependencies: { packages: [], optional: [] },
      components: [],
      compatibility: { desktops: [], sessions: [], distros: [], required_binaries: [], optional_binaries: [] },
    },
  ];

  const merged: PackageItem[] = [...mockRepoPackages];
  for (const ls of catalogueLockScreens) {
    const existingIdx = merged.findIndex(
      (p) =>
        p.id === ls.id ||
        p.id === `lockscreen-qylock-${ls.id}` ||
        p.id === `lockscreen-${ls.id}` ||
        (ls.id === "aurora-hyprlock" && p.id === "lockscreen-hyprlock-aurora") ||
        (ls.id === "swaylock-blur" && p.id === "lockscreen-swaylock-blur")
    );
    if (existingIdx >= 0) {
      merged[existingIdx] = {
        ...merged[existingIdx],
        ...ls,
        id: merged[existingIdx].id,
      };
    } else {
      merged.push(ls);
    }
  }

  // Merged result must contain 7 items, with Aurora enriched
  assert.equal(merged.length, 7, "Merged packages must contain exactly 7 lockscreen items");
  const auroraEnriched = merged.find((p) => p.title === "Aurora Glass Hyprlock")!;
  assert.ok(auroraEnriched.lockscreen, "Aurora Glass Hyprlock must have lockscreen manifest enriched");

  // Filter for activeTab === "lockscreens"
  const activeTab = "lockscreens";
  const filtered = merged.filter((pkg) => {
    return (
      activeTab === "lockscreens" &&
      (pkg.package_type === "lockscreen" ||
        pkg.category === "lockscreens" ||
        pkg.tags.some((t) => t.toLowerCase().includes("lockscreen")))
    );
  });
  assert.equal(filtered.length, 7, "DiscoverView lockscreen category filter must include all 7 items");
});

test("14. Subfilter isolation across All, Hyprlock, Quickshell, Swaylock, and SDDM", () => {
  const catalogueLockScreens = getCatalogueLockScreens();

  const filterBySub = (sub: string) => {
    return catalogueLockScreens.filter((pkg) => {
      if (sub === "All") return true;
      const sf = sub.toLowerCase();
      if (sf === "sddm") {
        return isLoginScreen(pkg) || pkg.lockscreen?.targets?.sddm != null || pkg.tags.some((t) => t.toLowerCase() === "sddm");
      }
      if (sf === "hyprlock") {
        return (
          pkg.tags.some((t) => t.toLowerCase() === "hyprlock") ||
          pkg.title.toLowerCase().includes("hyprlock") ||
          pkg.lockscreen?.targets?.hyprlock != null
        );
      }
      if (sf === "quickshell") {
        return (
          pkg.tags.some((t) => t.toLowerCase() === "quickshell" || t.toLowerCase() === "qylock") ||
          pkg.title.toLowerCase().includes("quickshell") ||
          pkg.lockscreen?.targets?.quickshell != null
        );
      }
      if (sf === "swaylock") {
        return (
          pkg.tags.some((t) => t.toLowerCase() === "swaylock") ||
          pkg.title.toLowerCase().includes("swaylock") ||
          pkg.lockscreen?.targets?.swaylock != null
        );
      }
      return false;
    });
  };

  const allItems = filterBySub("All");
  assert.equal(allItems.length, 7, "All subfilter must return 7 items");

  const hyprlockItems = filterBySub("Hyprlock");
  assert.equal(hyprlockItems.length, 1, "Hyprlock subfilter must isolate Aurora Glass Hyprlock");
  assert.equal(hyprlockItems[0].title, "Aurora Glass Hyprlock");

  const quickshellItems = filterBySub("Quickshell");
  assert.equal(quickshellItems.length, 5, "Quickshell subfilter must isolate the 5 Qylock themes");
  assert.ok(quickshellItems.every((i) => i.supports_session_lock), "All Quickshell items must support session lock");

  const sddmItems = filterBySub("SDDM");
  assert.equal(sddmItems.length, 5, "SDDM subfilter must isolate the 5 Qylock login greeter themes");
  assert.ok(sddmItems.every((i) => i.supports_login_screen), "All SDDM items must support login screen");

  const swaylockItems = filterBySub("Swaylock");
  assert.equal(swaylockItems.length, 1, "Swaylock subfilter must isolate Swaylock Effects Blur");
  assert.equal(swaylockItems[0].title, "Swaylock Effects Blur");
});

test("15. Dual-target classification and non-destructive desktop integration invariant", () => {
  const catalogueLockScreens = getCatalogueLockScreens();
  const dogSamurai = catalogueLockScreens.find((p) => p.title === "Dog Samurai")!;
  const aurora = catalogueLockScreens.find((p) => p.title === "Aurora Glass Hyprlock")!;
  const swaylock = catalogueLockScreens.find((p) => p.title === "Swaylock Effects Blur")!;

  // Dual-target: Dog Samurai supports both session lock and login screen
  assert.equal(isSessionLock(dogSamurai), true, "Dog Samurai must be identified as session lock");
  assert.equal(isLoginScreen(dogSamurai), true, "Dog Samurai must be identified as login screen");
  assert.equal(getPackageSubtype(dogSamurai), "Quickshell · SDDM", "Subtype must reflect dual-target");
  assert.equal(getConciseCompatibility(dogSamurai).label, "✓ Session & Login");

  // Single-target session locks
  assert.equal(isSessionLock(aurora), true, "Aurora must be a session lock");
  assert.equal(isLoginScreen(aurora), false, "Aurora must NOT be a login screen");

  assert.equal(isSessionLock(swaylock), true, "Swaylock must be a session lock");
  assert.equal(isLoginScreen(swaylock), false, "Swaylock must NOT be a login screen");

  // Keybind / Idle non-destructive invariant
  // Resolving capabilities must never produce writes to user window manager keybindings or idle configurations
  const resolved = resolveLockscreenCapabilities(mockSystemHyprland, dogSamurai.lockscreen!, "both");
  const writtenPaths = [
    ...resolved.target_files.map((f) => f.target),
    ...resolved.config_files.map((c) => c.path),
  ];

  for (const path of writtenPaths) {
    assert.ok(
      !path.includes("hyprland.conf"),
      `Path ${path} must never target or overwrite hyprland.conf`
    );
    assert.ok(
      !path.includes("hypridle.conf"),
      `Path ${path} must never target or overwrite hypridle.conf`
    );
    assert.ok(
      path.startsWith("~/.local/share/ryzora/") || path.startsWith("/usr/share/sddm/") || path.startsWith("/etc/sddm.conf.d/"),
      `Path ${path} must be strictly confined to Ryzora storage or SDDM greeter directories`
    );
  }
});

test("16. Unique Media Identity: Every Qylock theme has a unique poster and preview asset", () => {
  const catalogueLockScreens = getCatalogueLockScreens();
  const qylockThemes = catalogueLockScreens.filter((p) => p.lockscreen?.provider === "qylock");

  assert.equal(qylockThemes.length, 5, "Must have exactly 5 Qylock themes");

  const posters = new Set<string>();
  const previewVideos = new Set<string>();

  for (const theme of qylockThemes) {
    const poster = theme.preview_poster_url || theme.hero_image;
    const video = theme.preview_video_url;

    assert.ok(poster, `Theme ${theme.title} must declare a poster URL`);
    assert.ok(
      !poster.includes("Assets/title.png"),
      `Theme ${theme.title} poster must NOT be the generic QYLOCK title.png collage`
    );

    assert.ok(
      !posters.has(poster),
      `Theme ${theme.title} must have a unique poster; detected collision with ${poster}`
    );
    posters.add(poster);

    if (video) {
      assert.ok(
        !video.includes("Assets/title.png"),
        `Theme ${theme.title} preview video must NOT point to static title.png`
      );
      assert.ok(
        !previewVideos.has(video),
        `Theme ${theme.title} must have a unique preview video; detected collision with ${video}`
      );
      previewVideos.add(video);
    }
  }

  assert.equal(posters.size, 5, "All 5 Qylock themes must have 5 distinct posters");
  assert.equal(previewVideos.size, 5, "All 5 Qylock themes must have distinct preview videos");
});

test("17. Media Asset Resolution & Verification: Preview videos and posters exist on disk and upstream", () => {
  const catalogueLockScreens = getCatalogueLockScreens();
  const qylockThemes = catalogueLockScreens.filter((p) => p.lockscreen?.provider === "qylock");

  const publicDir = path.resolve(process.cwd(), "public");

  for (const theme of qylockThemes) {
    const poster = theme.preview_poster_url!;
    const video = theme.preview_video_url!;
    const animated = theme.lockscreen?.media.preview_animated!;

    // Verify local files exist in public/
    if (poster.startsWith("/")) {
      const localPoster = path.join(publicDir, poster);
      assert.ok(fs.existsSync(localPoster), `Local poster file must exist on disk: ${localPoster}`);
      const stats = fs.statSync(localPoster);
      assert.ok(stats.size > 1000, `Poster ${localPoster} must be a valid image file (> 1KB)`);
    }

    if (video.startsWith("/")) {
      const localVideo = path.join(publicDir, video);
      assert.ok(fs.existsSync(localVideo), `Local video file must exist on disk: ${localVideo}`);
      const stats = fs.statSync(localVideo);
      assert.ok(stats.size > 5000, `Video ${localVideo} must be a valid video stream (> 5KB)`);
      assert.ok(video.endsWith(".mp4"), `Video ${video} must have .mp4 extension for HTML5 video`);
    }

    if (animated && animated.startsWith("/")) {
      const localAnimated = path.join(publicDir, animated);
      assert.ok(fs.existsSync(localAnimated), `Local animated file must exist on disk: ${localAnimated}`);
    }

    // Verify upstream provenance URLs are declared
    if (theme.media_type === "video") {
      assert.ok(
        theme.lockscreen?.media.upstream_video || theme.preview_video_url,
        `Video theme ${theme.title} must declare upstream video asset`
      );
    }
  }
});

test("18. Animated & Video Theme Invariants: Never silently degrade to generic static poster", () => {
  const catalogueLockScreens = getCatalogueLockScreens();

  const dogSamurai = catalogueLockScreens.find((p) => p.title === "Dog Samurai")!;
  const clockworkTape = catalogueLockScreens.find((p) => p.title === "Tape (Clockwork)")!;
  const forest = catalogueLockScreens.find((p) => p.title === "Forest")!;
  const nier = catalogueLockScreens.find((p) => p.title === "NieR: Automata")!;
  const materialYou = catalogueLockScreens.find((p) => p.title === "Material You")!;

  // Must have media_type set to video or animated
  assert.equal(dogSamurai.media_type, "video");
  assert.equal(forest.media_type, "video");
  assert.equal(clockworkTape.media_type, "animated");
  assert.equal(nier.media_type, "animated");
  assert.equal(materialYou.media_type, "animated");

  // Must provide preview_video_url for hardware-accelerated playback
  for (const theme of [dogSamurai, clockworkTape, forest, nier, materialYou]) {
    assert.ok(theme.preview_video_url, `Theme ${theme.title} must have preview_video_url`);
    assert.ok(
      theme.preview_video_url.endsWith(".mp4"),
      `Theme ${theme.title} preview video must be an MP4 stream`
    );
    assert.notEqual(
      theme.preview_video_url,
      theme.preview_poster_url,
      `Theme ${theme.title} preview video must not equal its poster`
    );
    assert.ok(
      theme.lockscreen?.media.preview_animated,
      `Theme ${theme.title} must have animated preview fallback`
    );
  }
});
