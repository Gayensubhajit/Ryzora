import { catalogService } from "../services/catalogService.ts";
import { formatBytes, inspectAppCleanup, executeAppCleanup } from "../services/cleanupService.ts";
/**
 * Phase 23A — Pacman App Provider & Engine Integration Tests
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { packageEngine, pacmanAppProvider, PacmanAppProvider } from "../providers/index.ts";
import {
  resolveAppMetadata,
  resolveCanonicalAppId,
  deduplicateAppPackages,
  POPULAR_CANONICAL_IDS,
  RECOMMENDED_CANONICAL_IDS,
} from "../components/apps/appMetadata.ts";
import type { PackageTargetSpec } from "../providers/types.ts";

describe("Phase 23A — Pacman App Provider", () => {
  it("Pacman 1: Provider contract and metadata properties", () => {
    assert.equal(pacmanAppProvider.id, "pacman");
    assert.equal(pacmanAppProvider.name, "Arch Native (Pacman)");
    assert.equal(pacmanAppProvider.category, "apps");
    assert.equal(pacmanAppProvider.version, "1.0.0");
  });

  it("Pacman 2: discover() returns curated Arch packages", async () => {
    const pkgs = await pacmanAppProvider.discover();
    assert.ok(Array.isArray(pkgs));
    assert.ok(pkgs.length >= 5, `Expected at least 5 packages, got ${pkgs.length}`);

    const firefox = pkgs.find((p) => p.id === "firefox");
    assert.ok(firefox, "firefox must be in discovered packages");
    assert.equal(firefox.category, "apps");
    assert.equal(firefox.package_type, "app");
  });

  it("Pacman 3: search() filters packages by name and description", async () => {
    const ffResults = await pacmanAppProvider.search("firefox");
    assert.ok(ffResults.length >= 1);
    assert.equal(ffResults[0].id, "firefox");

    const termResults = await pacmanAppProvider.search("terminal");
    assert.ok(termResults.length >= 1);
    assert.ok(termResults.some((p) => p.id === "alacritty"));
  });

  it("Pacman 4: normalize() creates valid PackageItem with required fields", () => {
    const raw = {
      name: "htop",
      version: "3.3.0-1",
      description: "Interactive process viewer",
      repository: "extra",
      url: "https://htop.dev",
      license: "GPL-2.0-or-later",
      size_bytes: 180000,
      is_installed: false,
      dependencies: ["ncurses"],
    };

    const item = pacmanAppProvider.normalize(raw);
    assert.equal(item.id, "htop");
    assert.equal(item.title, "htop");
    assert.equal(item.version, "3.3.0-1");
    assert.equal(item.category, "apps");
    assert.equal(item.package_type, "app");
    assert.ok(item.tags.includes("native"));
    assert.ok(item.tags.includes("pacman"));
    assert.ok(item.tags.includes("extra"));
    assert.equal(item.dependencies.packages[0], "ncurses");
  });

  it("Pacman 5: getTargets() returns native system target requiring root", async () => {
    const pkgs = await pacmanAppProvider.discover();
    const firefox = pkgs.find((p) => p.id === "firefox")!;

    const targets = pacmanAppProvider.getTargets(firefox) as PackageTargetSpec[];
    assert.ok(Array.isArray(targets));
    assert.equal(targets.length, 1);

    const native = targets[0];
    assert.equal(native.id, "native");
    assert.equal(native.scope, "system");
    assert.equal(native.requires_root, true);
    assert.equal(native.supported, true);
  });

  it("Pacman 6: getSource() returns system type and repository", async () => {
    const pkgs = await pacmanAppProvider.discover();
    const firefox = pkgs.find((p) => p.id === "firefox")!;

    const source = pacmanAppProvider.getSource(firefox);
    assert.equal(source.type, "system");
    assert.equal(source.repository, "extra");
  });

  it("Pacman 7: getProvenance() and getMedia() return valid descriptors", async () => {
    const pkgs = await pacmanAppProvider.discover();
    const firefox = pkgs.find((p) => p.id === "firefox")!;

    const prov = pacmanAppProvider.getProvenance(firefox);
    assert.ok(prov.upstream.includes("mozilla.org"));
    assert.equal(prov.license, "MPL-2.0");

    const media = pacmanAppProvider.getMedia(firefox);
    assert.ok(media.poster.length > 0);
  });

  it("Pacman 8: getMeta() returns typed PacmanAppMeta", async () => {
    const pkgs = await pacmanAppProvider.discover();
    const firefox = pkgs.find((p) => p.id === "firefox")!;

    const meta = pacmanAppProvider.getMeta(firefox);
    assert.ok(meta);
    assert.equal(meta.repository, "extra");
    assert.equal(meta.license, "MPL-2.0");
    assert.ok(meta.dependencies.includes("gtk3"));
  });

  it("Pacman 9: Registered with PackageEngine singleton", () => {
    const registered = packageEngine.getProvider("pacman");
    assert.ok(registered);
    assert.equal(registered.id, "pacman");
    assert.equal(registered.category, "apps");
  });

  it("Pacman 10: packageEngine.searchAll() aggregates pacman packages", async () => {
    const results = await packageEngine.searchAll("firefox");
    assert.ok(results.length >= 1);
    const ff = results.find((p) => p.id === "firefox");
    assert.ok(ff);
    assert.equal(ff.category, "apps");
  });

  it("Pacman 11: packageEngine.discoverByCategory('apps') discovers app packages", async () => {
    const appPkgs = await packageEngine.discoverByCategory("apps");
    assert.ok(appPkgs.length >= 5);
    assert.ok(appPkgs.every((p) => p.category === "apps"));
  });

  it("Pacman 12: Simulated install and uninstall lifecycle updates state", async () => {
    const provider = new PacmanAppProvider();
    const pkgs = await provider.discover();
    const btop = pkgs.find((p) => p.id === "btop")!;

    let meta = provider.getMeta(btop);
    assert.equal(meta?.isInstalled, false);

    // Install
    const installRes = (await provider.install(btop, "native")) as { success: boolean };
    assert.equal(installRes.success, true);
    meta = provider.getMeta(btop);
    assert.equal(meta?.isInstalled, true);

    // Uninstall
    const uninstallRes = (await provider.uninstall(btop, "native")) as { success: boolean };
    assert.equal(uninstallRes.success, true);
    meta = provider.getMeta(btop);
    assert.equal(meta?.isInstalled, false);
  });
  it("Phase 23C.1 - 1: Canonical App ID maps package aliases to single identity", () => {
    assert.equal(resolveCanonicalAppId("visual-studio-code-bin"), "code");
    assert.equal(resolveCanonicalAppId("code-oss"), "code");
    assert.equal(resolveCanonicalAppId("firefox-developer-edition"), "firefox");
    assert.equal(resolveCanonicalAppId("google-chrome"), "chromium");
    assert.equal(resolveCanonicalAppId("spotify-launcher"), "spotify");
    assert.equal(resolveCanonicalAppId("obs"), "obs-studio");
    assert.equal(resolveCanonicalAppId("telegram-desktop"), "telegram");
    assert.equal(resolveCanonicalAppId("cursor-bin"), "cursor");
  });

  it("Phase 23C.1 - 2: DeduplicateAppPackages eliminates duplicates by canonical identity", () => {
    const rawList = [
      { id: "firefox", title: "Firefox", category: "apps" as const, package_type: "native" as const, version: "1.0", maintainer: "arch", tags: [] },
      { id: "firefox-developer-edition", title: "Firefox Dev", category: "apps" as const, package_type: "native" as const, version: "1.0", maintainer: "arch", tags: [] },
      { id: "code", title: "Code", category: "apps" as const, package_type: "native" as const, version: "1.0", maintainer: "arch", tags: [] },
      { id: "visual-studio-code-bin", title: "VS Code Bin", category: "apps" as const, package_type: "native" as const, version: "1.0", maintainer: "arch", tags: [] },
      { id: "discord", title: "Discord", category: "apps" as const, package_type: "native" as const, version: "1.0", maintainer: "arch", tags: [] },
    ];
    const deduped = deduplicateAppPackages(rawList);
    assert.equal(deduped.length, 3);
    assert.deepEqual(deduped.map(d => resolveCanonicalAppId(d.id)), ["firefox", "code", "discord"]);
  });

  it("Phase 23C.1 - 3: Popular and Recommended canonical IDs are strictly non-overlapping", () => {
    const popularSet = new Set(POPULAR_CANONICAL_IDS);
    for (const recId of RECOMMENDED_CANONICAL_IDS) {
      assert.ok(!popularSet.has(recId), `Recommended ID '${recId}' must not exist in Popular picks`);
    }
  });

  it("Phase 25.1 - 1: Canonical alias fallback preserves real title while resolving icon", () => {
    const metaChrome = resolveAppMetadata("google-chrome", "Google Chrome");
    assert.equal(metaChrome.displayName, "Google Chrome");
    assert.equal(metaChrome.iconUrl, "/assets/apps/chromium/icon.png");

    const metaEdge = resolveAppMetadata("microsoft-edge-stable-bin", "Microsoft Edge");
    assert.equal(metaEdge.displayName, "Microsoft Edge");
    assert.equal(metaEdge.iconUrl, "/assets/apps/chromium/icon.png");
  });

  it("Phase 25.1 - 2: PackageItem preserves repository and metadata_source fields", () => {
    const pkg = catalogService.catalogItemToPackageItem({
      id: "spotify",
      display_name: "Spotify",
      summary: "Music player",
      description: "Music streaming service",
      repository: "aur",
      version: "1.2.0",
      is_installed: false,
      installed_version: null,
      is_application: true,
      category: "Multimedia",
      subcategories: [],
      icon_name: "spotify",
      icon_path: null,
      launchable: "spotify.desktop",
      homepage: "https://spotify.com",
      license: "Proprietary",
      download_size: null,
      installed_size: null,
      dependencies: [],
      screenshots: [],
      developer: "Spotify AB",
      metadata_source: "aur",
    });

    assert.equal((pkg as any).repository, "aur");
    assert.equal((pkg as any).metadata_source, "aur");
    assert.equal(pkg.repository_id, "aur");
  });

  it("Phase 26 - 1: formatBytes formats byte sizes accurately without guesswork", () => {
    assert.equal(formatBytes(undefined), "Not available");
    assert.equal(formatBytes(null), "Not available");
    assert.equal(formatBytes(0), "0 B");
    assert.equal(formatBytes(1024), "1 KB");
    assert.equal(formatBytes(1024 * 1024 * 1.5), "1.5 MB");
    assert.equal(formatBytes(1024 * 1024 * 1024 * 1.21), "1.21 GB");
  });

  it("Phase 26 - 2: inspectAppCleanup provides structured preview with zero arbitrary estimates", async () => {
    const preview = await inspectAppCleanup("blender", "pacman");
    assert.equal(preview.package_id, "blender");
    assert.equal(preview.provider_id, "pacman");
    assert.ok(Array.isArray(preview.unused_dependencies));
    assert.ok(Array.isArray(preview.package_cache_items));
    assert.ok(Array.isArray(preview.conflicts));
    assert.equal(typeof preview.total_unused_dependencies_bytes, "number");
    assert.equal(typeof preview.total_package_cache_bytes, "number");
  });

  it("Phase 26 - 3: executeAppCleanup returns structured result and reclaimed byte accounting", async () => {
    const res = await executeAppCleanup({
      package_id: "blender",
      provider_id: "pacman",
      remove_package: true,
      remove_unused_dependencies: false,
      clean_package_cache: true,
      clean_app_cache: false,
      clean_app_config: false,
      clean_aur_build: false,
      clean_flatpak_data: false,
    });
    assert.equal(res.success, true);
    assert.equal(res.package_removed, true);
    assert.equal(typeof res.total_bytes_reclaimed, "number");
    assert.ok(Array.isArray(res.errors));
  });
});
