import test from "node:test";
import assert from "node:assert";
import {
  SILENTSDDM_UPSTREAM,
  SILENTSDDM_WALLPAPERS,
  getDiscoveredSilentSddmWallpapers,
  isSupportedVideoExtension,
  isSupportedImageExtension,
  isRejectedExtension,
} from "../providers/silentSddmDiscovery.ts";
import {
  silentSddmLockscreenProvider,
  normalizeSilentSddmWallpaper,
  packageEngine,
} from "../providers/index.ts";

const SHA256_HEX_REGEX = /^[a-f0-9]{64}$/;

// ── 1. SILENTSDDM CATALOG & INTEGRITY TESTS ──

test("SilentSDDM 1: Upstream metadata is correctly pinned", () => {
  assert.strictEqual(SILENTSDDM_UPSTREAM.repo, "uiriansan/SilentSDDM");
  assert.strictEqual(
    SILENTSDDM_UPSTREAM.sha,
    "73380d331fe0f8b8a3b17991bb917a0d438a4d34"
  );
  assert.strictEqual(SILENTSDDM_UPSTREAM.version, "1.5.0");
  assert.strictEqual(SILENTSDDM_UPSTREAM.themeLicense, "GPL-3.0-or-later");
  assert.strictEqual(SILENTSDDM_UPSTREAM.author, "uiriansan");
  assert.ok(
    SILENTSDDM_UPSTREAM.authorAvatar.startsWith("https://avatars.githubusercontent.com/"),
    "Author avatar must be authentic GitHub profile image, not stock Unsplash"
  );
  assert.ok(
    SILENTSDDM_UPSTREAM.rawBase.includes(SILENTSDDM_UPSTREAM.sha),
    "rawBase must contain pinned commit SHA"
  );
});

test("SilentSDDM 2: Complete inventory of backgrounds/ directory", () => {
  const wallpapers = getDiscoveredSilentSddmWallpapers();
  assert.strictEqual(wallpapers.length, 6);

  const expectedIds = [
    "silentsddm-default",
    "silentsddm-smoky",
    "silentsddm-mountain",
    "silentsddm-ken",
    "silentsddm-rei",
    "silentsddm-silvia",
  ];

  for (const id of expectedIds) {
    const found = wallpapers.find((w) => w.id === id);
    assert.ok(found, `Expected wallpaper ${id} in catalog`);
    assert.ok(found.sizeBytes > 0, `${id} sizeBytes must be > 0`);
    assert.ok(
      found.downloadUrl.includes(SILENTSDDM_UPSTREAM.sha),
      `${id} downloadUrl must include pinned SHA`
    );
    assert.ok(
      found.downloadUrl.startsWith("https://raw.githubusercontent.com/uiriansan/SilentSDDM/"),
      `${id} downloadUrl must point to raw GitHub repository`
    );
    assert.ok(found.posterUrl, `${id} must have posterUrl`);
    assert.ok(found.path.startsWith("backgrounds/"), `${id} path must be relative to repo`);
  }
});

test("SilentSDDM 3: Every catalog asset has a real 64-character lowercase SHA-256 hash", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    assert.notStrictEqual(
      w.sha256,
      "pinned",
      `Wallpaper ${w.id} must not use placeholder 'pinned'`
    );
    assert.match(
      w.sha256,
      SHA256_HEX_REGEX,
      `Wallpaper ${w.id} sha256 '${w.sha256}' must be a 64-char lowercase hex string`
    );

    // If video with poster, poster must also have a real SHA-256 hash
    if (w.posterSha256) {
      assert.match(
        w.posterSha256,
        SHA256_HEX_REGEX,
        `Poster for ${w.id} sha256 must be a 64-char lowercase hex string`
      );
      assert.ok((w.posterSizeBytes ?? 0) > 0, `Poster size for ${w.id} must be > 0`);
    }
  }
});

test("SilentSDDM 4: Every filename and path in the catalog is unique", () => {
  const ids = new Set<string>();
  const paths = new Set<string>();

  for (const w of SILENTSDDM_WALLPAPERS) {
    assert.ok(!ids.has(w.id), `Duplicate ID found: ${w.id}`);
    ids.add(w.id);

    assert.ok(!paths.has(w.path), `Duplicate path found: ${w.path}`);
    paths.add(w.path);
  }
});

test("SilentSDDM 5: Media type strictly matches file extension", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    if (w.type === "video") {
      assert.ok(
        w.filename.endsWith(".mp4") ||
          w.filename.endsWith(".webm") ||
          w.filename.endsWith(".mkv") ||
          w.filename.endsWith(".avi") ||
          w.filename.endsWith(".mov") ||
          w.filename.endsWith(".m4v"),
        `Video wallpaper ${w.id} must have a video extension`
      );
    } else if (w.type === "image") {
      assert.ok(
        w.filename.endsWith(".jpg") ||
          w.filename.endsWith(".jpeg") ||
          w.filename.endsWith(".png"),
        `Image wallpaper ${w.id} must have an image extension`
      );
    } else {
      assert.fail(`Unknown media type for ${w.id}`);
    }
  }
});

test("SilentSDDM 6: Asset licenses are separated from theme license and not assumed GPL", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    assert.strictEqual(
      w.license.theme,
      "GPL-3.0-or-later",
      "Theme license must be GPL-3.0-or-later"
    );

    // Third-party wallpapers must not claim GPL without evidence
    if (w.id === "silentsddm-mountain") {
      assert.ok(w.license.asset.includes("Pexels"), "Mountain must cite Pexels license");
      assert.strictEqual(w.license.credit, "Joyston Judah");
      assert.ok(w.license.sourceUrl?.includes("pexels.com"));
    }

    if (w.id === "silentsddm-rei") {
      assert.ok(!w.license.asset.includes("GPL"), "Rei must not falsely claim GPL");
      assert.strictEqual(w.license.credit, "DesktopHut");
    }

    if (w.id === "silentsddm-ken") {
      assert.ok(!w.license.asset.includes("GPL"), "Ken must not falsely claim GPL");
      assert.strictEqual(w.license.credit, "MoeWalls");
    }

    if (w.id === "silentsddm-silvia") {
      assert.ok(!w.license.asset.includes("GPL"), "Silvia must not falsely claim GPL");
      assert.strictEqual(w.license.credit, "MoeWalls");
    }
  }
});

test("SilentSDDM 7: Extension validators correctly classify media formats and reject GIF", () => {
  const validVideos = [".mp4", ".webm", ".mkv", ".avi", ".mov", ".m4v", "MP4", "webm"];
  for (const ext of validVideos) {
    assert.strictEqual(isSupportedVideoExtension(ext), true, `Expected ${ext} to be supported video`);
  }

  const validImages = [".jpg", ".jpeg", ".png", "PNG", "JPG"];
  for (const ext of validImages) {
    assert.strictEqual(isSupportedImageExtension(ext), true, `Expected ${ext} to be supported image`);
  }

  // GIF is explicitly rejected (crashes SDDM)
  assert.strictEqual(isRejectedExtension(".gif"), true);
  assert.strictEqual(isRejectedExtension("gif"), true);
  assert.strictEqual(isSupportedVideoExtension(".gif"), false);
  assert.strictEqual(isSupportedImageExtension(".gif"), false);

  assert.strictEqual(isSupportedVideoExtension(".exe"), false);
  assert.strictEqual(isSupportedVideoExtension(".sh"), false);
  assert.strictEqual(isSupportedImageExtension(".exe"), false);
});

// ── 2. PROVIDER CONTRACT & STORE METADATA INTEGRITY ──

test("SilentSDDM 8: Provider conforms to LockscreenProvider interface", () => {
  assert.strictEqual(silentSddmLockscreenProvider.id, "silentsddm");
  assert.strictEqual(silentSddmLockscreenProvider.name, "SilentSDDM Provider");
  assert.strictEqual(silentSddmLockscreenProvider.category, "lockscreen");

  const targets = silentSddmLockscreenProvider.getTargets();
  assert.strictEqual(targets.quickshell, false, "SilentSDDM is NOT quickshell session lock");
  assert.strictEqual(targets.sddm, true, "SilentSDDM supports SDDM target");
  assert.strictEqual(targets.hyprlock, undefined);

  const deps = silentSddmLockscreenProvider.getDependencies();
  assert.ok(deps.includes("sddm"));
  assert.ok(deps.includes("qt6-multimedia"));

  const firstWallpaper = SILENTSDDM_WALLPAPERS[0];
  const firstPkg = normalizeSilentSddmWallpaper(firstWallpaper);
  const source = silentSddmLockscreenProvider.getSource(firstPkg);
  assert.strictEqual(source.type, "git");
  assert.ok(source.repository.includes("SilentSDDM"));

  const prov = silentSddmLockscreenProvider.getProvenance(firstPkg);
  assert.strictEqual(prov.license, "GPL-3.0-or-later");
});

test("SilentSDDM 9: normalizeSilentSddmWallpaper contains zero fabricated metrics", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    const pkg = normalizeSilentSddmWallpaper(w);
    assert.strictEqual(pkg.id, w.id);
    assert.strictEqual(pkg.package_type, "lockscreen");
    assert.strictEqual(pkg.category, "lockscreens");
    assert.strictEqual(pkg.supports_session_lock, false);
    assert.strictEqual(pkg.supports_login_screen, true);
    assert.strictEqual(pkg.author.name, "uiriansan");

    // Zero fabricated store ratings or downloads
    assert.strictEqual(pkg.rating, 0, "No fabricated ratings permitted");
    assert.strictEqual(pkg.rating_count, 0, "No fabricated rating counts permitted");
    assert.strictEqual(pkg.downloads, 0, "No fabricated download counts permitted");

    // No Unsplash avatars
    assert.ok(
      !pkg.author.avatar.includes("unsplash.com"),
      "Must not use unrelated Unsplash avatar"
    );

    // Exact content hash and size preserved
    assert.strictEqual(pkg.content_hash, w.sha256);
    assert.strictEqual(pkg.package_size_bytes, w.sizeBytes);

    assert.ok(pkg.lockscreen);
    assert.strictEqual(pkg.lockscreen.provider, "silentsddm");
    assert.strictEqual(pkg.lockscreen.targets.quickshell, undefined);
    assert.ok(pkg.lockscreen.targets.sddm);
    assert.strictEqual(
      pkg.lockscreen.targets.sddm.install_destination,
      "/usr/share/sddm/themes/ryzora-silent"
    );
    assert.strictEqual(
      pkg.lockscreen.targets.sddm.config_file,
      "/etc/sddm.conf.d/zz-ryzora-theme.conf"
    );
  }
});

// ── 3. PACKAGE ENGINE INTEGRATION TESTS ──

test("SilentSDDM 10: Registered with PackageEngine singleton and discoverable", async () => {
  const provider = packageEngine.getProvider("silentsddm");
  assert.ok(provider, "SilentSDDM provider must be registered in PackageEngine");
  assert.strictEqual(provider.id, "silentsddm");

  const discovered = await provider.discover();
  assert.strictEqual(discovered.length, 6);

  const allLockscreens = await packageEngine.discoverByCategory("lockscreens");
  const sddmItems = allLockscreens.filter(
    (p) => p.lockscreen?.provider === "silentsddm"
  );
  assert.strictEqual(
    sddmItems.length,
    6,
    "PackageEngine should aggregate all 6 SilentSDDM wallpapers"
  );
});
