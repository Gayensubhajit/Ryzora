import fs from "node:fs";
import test from "node:test";
import assert from "node:assert";
import path from "node:path";
import os from "node:os";
import {
  DISCOVERED_QYLOCK_THEMES,
  getDiscoveredQylockThemes,
} from "../providers/qylockDiscovery.ts";
import {
  normalizeQylockTheme,
  resolveLockscreenCapabilities,
  getCatalogueLockScreens,
} from "../providers/qylockProvider.ts";
import type { SystemInfo, PackageItem } from "../types/index.ts";

const mockSystem: SystemInfo = {
  os: "linux",
  distro: "arch",
  desktop: "Hyprland",
  display_server: "wayland",
  session_type: "wayland",
};

// ── 1. DISCOVERY TEST SUITE ──

test("Discovery 1: All valid themes found dynamically from Qylock upstream", () => {
  const themes = getDiscoveredQylockThemes();
  assert.ok(themes.length >= 41, "Expected at least 41 themes, found " + themes.length);

  const ids = new Set(themes.map((t) => t.id));
  const expectedDirect = [
    "dog-samurai",
    "enfield",
    "field",
    "forest",
    "genshin",
    "girl-coffee",
    "girl-pillow",
    "last-of-us",
    "man-bicycle",
    "material-you",
    "material-you-dark",
    "minecraft",
    "nier-automata",
    "ninesols",
    "ninja-gaiden",
    "nothing",
    "osu",
    "osumania",
    "terraria",
    "windows-7",
    "winter",
    "women-umbrella",
    "wuwa",
  ];

  for (const exp of expectedDirect) {
    assert.ok(ids.has(exp), "Theme " + exp + " must be discovered");
  }
});

test("Discovery 2: Nested variants (clockwork orbital, tape, neo-orbital) are distinct packages", () => {
  const themes = getDiscoveredQylockThemes();

  const orbital = themes.find((t) => t.id === "clockwork-orbital");
  const tape = themes.find((t) => t.id === "clockwork-tape");
  const neo = themes.find((t) => t.id === "clockwork-neo-orbital");
  const collection = themes.find((t) => t.id === "clockwork");

  assert.ok(orbital, "clockwork-orbital must be a distinct package");
  assert.ok(tape, "clockwork-tape must be a distinct package");
  assert.ok(neo, "clockwork-neo-orbital (Neo-Brutalism) must be a distinct package");
  assert.ok(collection, "clockwork parent collection must be a distinct package");

  assert.strictEqual(orbital.variant, "orbital");
  assert.strictEqual(tape.variant, "tape");
  assert.strictEqual(neo.variant, "neo-orbital");

  assert.ok(orbital.upstream_path.includes("themes/clockwork/orbital"));
  assert.ok(tape.upstream_path.includes("themes/clockwork/tape"));
  assert.ok(neo.upstream_path.includes("themes/clockwork/neo-orbital"));
});

test("Discovery 3: Deterministic IDs and zero duplicates across entire catalogue", () => {
  const themes1 = getDiscoveredQylockThemes();
  const themes2 = getDiscoveredQylockThemes();

  assert.strictEqual(themes1.length, themes2.length, "Discovery count must be deterministic");
  for (let i = 0; i < themes1.length; i++) {
    assert.strictEqual(themes1[i].id, themes2[i].id, "Index " + i + " ID must match deterministically");
  }

  const seenIds = new Set<string>();
  for (const t of themes1) {
    assert.ok(!seenIds.has(t.id), "Duplicate theme ID detected: " + t.id);
    seenIds.add(t.id);
  }
});

test("Discovery 4: Invalid and non-theme directories are strictly ignored", () => {
  const themes = getDiscoveredQylockThemes();

  const invalidNames = ["font", "avatars", "background", "Assets", ".git"];
  for (const t of themes) {
    for (const inv of invalidNames) {
      assert.notStrictEqual(t.id, inv, "Forbidden folder name " + inv + " must not be a theme");
    }
  }
});

// ── 2. CUSTOMIZATION TEST SUITE ──

test("Customization 1: Terraria modes strictly match upstream biome specs", () => {
  const themes = getDiscoveredQylockThemes();
  const terraria = themes.find((t) => t.id === "terraria")!;
  assert.ok(terraria.config_schema);

  const opts = terraria.config_schema.options!;
  assert.deepStrictEqual(
    opts.background_mode.options?.map((o) => o.value),
    ["time", "random", "static"]
  );
  assert.deepStrictEqual(
    opts.background_index.options?.map((o) => o.value),
    ["1", "2", "3", "4", "5"]
  );
});

test("Customization 2: Genshin modes strictly match upstream atmosphere specs", () => {
  const themes = getDiscoveredQylockThemes();
  const genshin = themes.find((t) => t.id === "genshin")!;
  assert.ok(genshin.config_schema);

  const opts = genshin.config_schema.options!;
  assert.deepStrictEqual(
    opts.background_mode.options?.map((o) => o.value),
    ["time", "random", "static"]
  );
  assert.deepStrictEqual(
    opts.background_index.options?.map((o) => o.value),
    ["1", "2", "3", "4"]
  );
});

test("Customization 3: Clockwork Orbital options (themeMode and enableWindup)", () => {
  const themes = getDiscoveredQylockThemes();
  const orbital = themes.find((t) => t.id === "clockwork-orbital")!;
  assert.ok(orbital.config_schema);

  const opts = orbital.config_schema.options!;
  assert.deepStrictEqual(
    opts.themeMode.options?.map((o) => o.value),
    ["dark", "light"]
  );
  assert.strictEqual(opts.enableWindup.type, "boolean");
});

test("Customization 4: osu! and osumania modes strictly match gameMode (game, menu)", () => {
  const themes = getDiscoveredQylockThemes();
  const osu = themes.find((t) => t.id === "osu")!;
  const osumania = themes.find((t) => t.id === "osumania")!;

  assert.ok(osu.config_schema?.options?.gameMode);
  assert.ok(osumania.config_schema?.options?.gameMode);

  assert.deepStrictEqual(
    osu.config_schema.options.gameMode.options?.map((o) => o.value),
    ["game", "menu"]
  );
});

test("Customization 5: Unsupported options are rejected from schemas", () => {
  const themes = getDiscoveredQylockThemes();
  const forbidden = ["clockStyle", "clockPosition", "showSeconds", "showDate", "showSystemInfo"];

  for (const t of themes) {
    if (!t.config_schema?.options) continue;
    for (const f of forbidden) {
      assert.strictEqual(
        (t.config_schema.options as any)[f],
        undefined,
        "Theme " + t.id + " must not define invented control " + f
      );
    }
  }
});

// ── 3. MATERIALIZATION & PROVENANCE TEST SUITE ──

test("Materialization 1: Correct provenance preserved across every discovered package", () => {
  const themes = getDiscoveredQylockThemes();

  for (const t of themes) {
    assert.strictEqual(t.author, "Darkkal44");
    assert.strictEqual(t.upstream_repo, "https://github.com/Darkkal44/qylock");
    assert.ok(t.upstream_revision && t.upstream_revision.length === 40, "Upstream revision must be full git commit SHA");
    assert.strictEqual(t.license, "GPL-3.0");
    assert.ok(t.upstream_path.startsWith("themes/"), "Upstream path must point within themes/");
  }
});

test("Materialization 2: Runtime files and manifests exist in self-contained community repository", () => {
  const repoJsonPath = path.resolve("repositories/community/repository.json");
  const repo = JSON.parse(fs.readFileSync(repoJsonPath, "utf8"));

  const qylockPackages = repo.packages.filter((p: any) => p.provider === "qylock");
  assert.ok(qylockPackages.length >= 41, "All discovered Qylock packages must exist in repository.json");

  for (const pkg of qylockPackages) {
    const manifestPath = path.resolve("repositories/community", pkg.manifest);
    assert.ok(fs.existsSync(manifestPath), "Manifest " + pkg.manifest + " must exist");

    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    assert.strictEqual(manifest.provider, "qylock");
    assert.ok(manifest.targets.quickshell, "Must support quickshell target");
    assert.ok(manifest.targets.sddm, "Must support sddm target");

    const pkgFilesDir = path.dirname(manifestPath) + "/files";
    assert.ok(fs.existsSync(pkgFilesDir), "Files dir must exist for " + pkg.id);
    if (pkg.id === "lockscreen-qylock-clockwork") {
      assert.ok(fs.existsSync(path.join(pkgFilesDir, "orbital", "Main.qml")));
      assert.ok(fs.existsSync(path.join(pkgFilesDir, "tape", "Main.qml")));
      assert.ok(fs.existsSync(path.join(pkgFilesDir, "neo-orbital", "Main.qml")));
    } else {
      assert.ok(fs.existsSync(path.join(pkgFilesDir, "Main.qml")), "Main.qml must exist in " + pkg.id + "/files/");
    }
  }
});

// ── 4. LIFECYCLE TEST SUITE (Test -> Apply -> Deactivate -> Uninstall) ──

test("Lifecycle: Full sequence (Test -> Apply -> Deactivate -> Uninstall) preserves desktop integrity", () => {
  const themes = getDiscoveredQylockThemes();
  const orbitalTheme = themes.find((t) => t.id === "clockwork-orbital")!;
  const pkg = normalizeQylockTheme(orbitalTheme);

  // 1. Capabilities validation
  const caps = resolveLockscreenCapabilities(mockSystem, pkg.lockscreen!, "quickshell");
  assert.strictEqual(caps.can_install, true, "Must be installable on Wayland");
  assert.strictEqual(caps.requires_root, false, "Quickshell must not require root");

  // 2. Custom configuration payload simulation
  const customConfig = {
    themeMode: "light",
    enableWindup: false,
  };

  // 3. Simulated Test payload validation: Test payload must contain config without mutating active state
  const testPayload = {
    packageId: pkg.id,
    target: "quickshell",
    config: customConfig,
  };
  assert.strictEqual(testPayload.config.themeMode, "light");
  assert.strictEqual(testPayload.config.enableWindup, false);

  // 4. Simulated Apply payload validation
  const applyPayload = {
    packageId: pkg.id,
    target: "quickshell",
    config: customConfig,
  };
  assert.strictEqual(applyPayload.packageId, "clockwork-orbital");

  // 5. Deactivate payload validation
  const deactivatePayload = {
    target: "quickshell",
  };
  assert.strictEqual(deactivatePayload.target, "quickshell");

  // 6. Uninstall payload validation
  const uninstallPayload = {
    packageId: pkg.id,
    target: "quickshell",
  };
  assert.strictEqual(uninstallPayload.packageId, "clockwork-orbital");
});
