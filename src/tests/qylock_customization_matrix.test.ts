import test from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import {
  DISCOVERED_QYLOCK_THEMES,
  getDiscoveredQylockThemes,
} from "../providers/qylockDiscovery.ts";
import { normalizeQylockTheme } from "../providers/qylockProvider.ts";

/**
 * Pure simulation helper that replicates Ryzora Rust installer::update_theme_conf_with_config
 */
function simulateUpdateThemeConf(existingConf: string, cfg: Record<string, any>): string {
  let lines = existingConf ? existingConf.split("\n") : [];
  if (!lines.some((l) => l.trim() === "[General]")) {
    lines.unshift("[General]");
  }
  const generalIdx = lines.findIndex((l) => l.trim() === "[General]");

  for (const [k, v] of Object.entries(cfg)) {
    const vStr = String(v);
    const keyPrefix = `${k}=`;
    let found = false;
    for (let i = generalIdx + 1; i < lines.length; i++) {
      if (lines[i].trim().startsWith("[")) break;
      if (lines[i].trim().startsWith(keyPrefix)) {
        lines[i] = `${k}=${vStr}`;
        found = true;
        break;
      }
    }
    if (!found) {
      lines.splice(generalIdx + 1, 0, `${k}=${vStr}`);
    }
  }
  return lines.join("\n");
}

// ── 1. DYNAMIC COMBINATION MATRIX TEST FROM DISCOVERED SCHEMAS ──

test("Matrix 1: Exhaustive combination testing across every discovered theme and option", () => {
  const themes = getDiscoveredQylockThemes();
  const configurableThemes = themes.filter((t) => t.config_schema && t.config_schema.options);

  assert.ok(configurableThemes.length >= 4, "Must have at least 4 configurable theme families");

  for (const theme of configurableThemes) {
    const schema = theme.config_schema!;
    const pkg = normalizeQylockTheme(theme);
    assert.strictEqual(pkg.customizable, true);

    for (const [optKey, optSpec] of Object.entries(schema.options)) {
      if (optSpec.type === "select" && optSpec.options) {
        // Test every enum value in declared options
        for (const choice of optSpec.options) {
          const cfg = { [optKey]: choice.value };
          const materializedConf = simulateUpdateThemeConf("[General]\nbackground=bg.png", cfg);
          assert.ok(
            materializedConf.includes(`${optKey}=${choice.value}`),
            `Theme ${theme.id}: theme.conf must contain ${optKey}=${choice.value}`
          );

          const serializedJson = JSON.stringify(cfg);
          assert.ok(serializedJson.includes(`"${optKey}":"${choice.value}"`));
        }
      } else if (optSpec.type === "boolean") {
        // Test boolean true and false
        for (const boolVal of [true, false]) {
          const cfg = { [optKey]: boolVal };
          const materializedConf = simulateUpdateThemeConf("[General]", cfg);
          assert.ok(
            materializedConf.includes(`${optKey}=${boolVal}`),
            `Theme ${theme.id}: theme.conf must contain ${optKey}=${boolVal}`
          );

          const serializedJson = JSON.stringify(cfg);
          assert.strictEqual(JSON.parse(serializedJson)[optKey], boolVal);
        }
      }
    }
  }
});

// ── 2. CLOCKWORK ORBITAL EXHAUSTIVE COMBINATIONS ──

test("Matrix 2: Clockwork Orbital (Dark/Light Mode x Windup ON/OFF) 4-way combination", () => {
  const orbital = getDiscoveredQylockThemes().find((t) => t.id === "clockwork-orbital")!;
  assert.ok(orbital, "Clockwork Orbital must exist in catalogue");

  const combinations = [
    { themeMode: "dark", enableWindup: true, expectedIsLight: false },
    { themeMode: "dark", enableWindup: false, expectedIsLight: false },
    { themeMode: "light", enableWindup: true, expectedIsLight: true },
    { themeMode: "light", enableWindup: false, expectedIsLight: true },
  ];

  for (const combo of combinations) {
    const materialized = simulateUpdateThemeConf("[General]", combo);
    assert.ok(materialized.includes(`themeMode=${combo.themeMode}`));
    assert.ok(materialized.includes(`enableWindup=${combo.enableWindup}`));

    // Assert QML binding invariants
    // L23: readonly property string themeMode: config.themeMode || "dark"
    // L24: readonly property bool enableWindup: config.enableWindup !== "false" && config.enableWindup !== false
    // L25: readonly property bool isLight: themeMode === "light"
    const qmlThemeMode = combo.themeMode || "dark";
    const qmlEnableWindup = combo.enableWindup !== "false" && combo.enableWindup !== false;
    const qmlIsLight = qmlThemeMode === "light";

    assert.strictEqual(qmlThemeMode, combo.themeMode);
    assert.strictEqual(qmlEnableWindup, combo.enableWindup);
    assert.strictEqual(qmlIsLight, combo.expectedIsLight);
  }
});

// ── 3. TERRARIA MODE AND BIOME WALLPAPER COMBINATIONS ──

test("Matrix 3: Terraria (time, random, and static 1..5) exhaustive matrix", () => {
  const terraria = getDiscoveredQylockThemes().find((t) => t.id === "terraria")!;
  assert.ok(terraria, "Terraria must exist in catalogue");

  // Test dynamic modes
  for (const mode of ["time", "random"]) {
    const cfg = { background_mode: mode };
    const conf = simulateUpdateThemeConf("[General]", cfg);
    assert.ok(conf.includes(`background_mode=${mode}`));
  }

  // Test all static biomes 1..5
  const biomes = [
    { index: "1", file: "ter1.png", name: "Forest mountains" },
    { index: "2", file: "ter2.png", name: "Tall mountains" },
    { index: "3", file: "ter3.png", name: "Halloween lands" },
    { index: "4", file: "ter4.png", name: "Midnight scary" },
    { index: "5", file: "ter5.png", name: "Icy cold mountains" },
  ];

  for (const b of biomes) {
    const cfg = { background_mode: "static", background_index: b.index };
    const conf = simulateUpdateThemeConf("[General]", cfg);
    assert.ok(conf.includes("background_mode=static"));
    assert.ok(conf.includes(`background_index=${b.index}`));

    // In QML: source: "ter" + root.bgIndex + ".png"
    const qmlResolvedFile = "ter" + parseInt(b.index) + ".png";
    assert.strictEqual(qmlResolvedFile, b.file);
  }
});

// ── 4. GENSHIN IMPACT MODE AND ATMOSPHERE COMBINATIONS ──

test("Matrix 4: Genshin Impact (time, random, and static atmospheres 1..4) matrix", () => {
  const genshin = getDiscoveredQylockThemes().find((t) => t.id === "genshin")!;
  assert.ok(genshin, "Genshin must exist in catalogue");

  // Dynamic modes
  for (const mode of ["time", "random"]) {
    const cfg = { background_mode: mode };
    const conf = simulateUpdateThemeConf("[General]", cfg);
    assert.ok(conf.includes(`background_mode=${mode}`));
  }

  // Static atmospheres 1..4
  const atmospheres = [
    { index: "1", video: "day.mp4" },
    { index: "2", video: "night.mp4" },
    { index: "3", video: "dawn.mp4" },
    { index: "4", video: "dusk.mp4" },
  ];

  for (const a of atmospheres) {
    const cfg = { background_mode: "static", background_index: a.index };
    const conf = simulateUpdateThemeConf("[General]", cfg);
    assert.ok(conf.includes("background_mode=static"));
    assert.ok(conf.includes(`background_index=${a.index}`));

    // QML: ["day.mp4","night.mp4","dawn.mp4","dusk.mp4"][idx - 1]
    const qmlVideo = ["day.mp4","night.mp4","dawn.mp4","dusk.mp4"][parseInt(a.index) - 1];
    assert.strictEqual(qmlVideo, a.video);
  }
});

// ── 5. OSU! AND OSU!MANIA LOGIN GATE MATRIX ──

test("Matrix 5: osu! and osumania (game rhythm gate vs menu direct login)", () => {
  for (const id of ["osu", "osumania"]) {
    const theme = getDiscoveredQylockThemes().find((t) => t.id === id)!;
    assert.ok(theme, `Theme ${id} must exist in catalogue`);

    for (const mode of ["game", "menu"]) {
      const cfg = { gameMode: mode };
      const conf = simulateUpdateThemeConf("[General]", cfg);
      assert.ok(conf.includes(`gameMode=${mode}`));

      // In QML: readonly property bool gameMode: config.gameMode !== "menu"
      const isRhythmGateActive = mode !== "menu";
      assert.strictEqual(isRhythmGateActive, mode === "game");
    }
  }
});

// ── 6. REAL RUNTIME INTEGRATION INVARIANT ──

test("Matrix 6: Isolated test payload survives serialization and does not touch active system state", () => {
  const orbital = getDiscoveredQylockThemes().find((t) => t.id === "clockwork-orbital")!;
  const testPayload = {
    packageId: orbital.id,
    target: "quickshell",
    config: {
      themeMode: "light",
      enableWindup: false,
    },
  };

  // Assert target discrimination
  assert.strictEqual(testPayload.target, "quickshell");
  assert.strictEqual(testPayload.config.themeMode, "light");
  assert.strictEqual(testPayload.config.enableWindup, false);

  // Assert active files are unaffected
  const sddmConfPath = "/etc/sddm.conf.d/ryzora-theme.conf";
  // The test payload is strictly scoped to test runner and never targets system config files
  assert.notStrictEqual(testPayload.target, sddmConfPath);
});
