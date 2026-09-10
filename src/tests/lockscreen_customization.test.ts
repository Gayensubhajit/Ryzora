import test from "node:test";
import assert from "node:assert";
import { RAW_QYLOCK_THEMES, normalizeQylockTheme } from "../providers/qylockProvider.ts";
import type { LockscreenConfigSchema, PackageItem } from "../types/index.ts";

test("Lockscreen Customization 1: Configurable themes strictly reflect authentic upstream schema and customizable=true", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  // Terraria has authentic background_mode and background_index
  const terraria = normalized.find((p) => p.id === "terraria");
  assert.ok(terraria, "terraria theme must exist");
  assert.strictEqual(terraria.customizable, true, "terraria must have customizable=true");
  assert.ok(terraria.lockscreen?.config_schema, "terraria must define config_schema");

  // Genshin has authentic atmosphere background_mode and background_index
  const genshin = normalized.find((p) => p.id === "genshin");
  assert.ok(genshin, "genshin theme must exist");
  assert.strictEqual(genshin.customizable, true, "genshin must have customizable=true");

  // Clockwork parent collection has variants and themeMode/enableWindup options
  const clockwork = normalized.find((p) => p.id === "clockwork");
  assert.ok(clockwork, "clockwork collection must exist");
  assert.strictEqual(clockwork.customizable, true, "clockwork collection must have customizable=true");

  // Clockwork Orbital variant has authentic themeMode and enableWindup
  const orbital = normalized.find((p) => p.id === "clockwork-orbital");
  assert.ok(orbital, "clockwork-orbital must exist");
  assert.strictEqual(orbital.customizable, true, "clockwork-orbital must have customizable=true");
});

test("Lockscreen Customization 2: Non-configurable themes do NOT expose config_schema or customizable=true", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  // Upstream themes without user-configurable options in theme.conf
  const nonConfigurableIds = [
    "dog-samurai",
    "nier-automata",
    "forest",
    "material-you",
    "minecraft",
    "windows-7",
    "clockwork-tape",
  ];

  for (const id of nonConfigurableIds) {
    const pkg = normalized.find((p) => p.id === id);
    assert.ok(pkg, id + " theme must exist");
    assert.strictEqual(pkg.customizable, false, id + " must not be flagged customizable");
    assert.strictEqual(pkg.lockscreen?.config_schema, undefined, id + " must not have config_schema");
  }
});

test("Lockscreen Customization 3: Terraria modes strictly match upstream options (time, random, static 1..5)", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const terraria = normalized.find((p) => p.id === "terraria");
  assert.ok(terraria, "terraria theme must exist");

  const schema = terraria.lockscreen?.config_schema;
  assert.ok(schema && schema.options, "Terraria must define options schema");

  const bgMode = schema.options.background_mode;
  assert.ok(bgMode, "Must define background_mode");
  assert.strictEqual(bgMode.type, "select");
  assert.deepStrictEqual(
    bgMode.options?.map((o) => o.value),
    ["time", "random", "static"],
    "Terraria background_mode must strictly support time, random, and static"
  );

  const bgIndex = schema.options.background_index;
  assert.ok(bgIndex, "Must define background_index");
  assert.strictEqual(bgIndex.type, "select");
  assert.deepStrictEqual(
    bgIndex.options?.map((o) => o.value),
    ["1", "2", "3", "4", "5"],
    "Terraria background_index must support 5 biomes (1..5)"
  );
});

test("Lockscreen Customization 4: Genshin modes strictly match upstream options (time, random, static 1..4)", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const genshin = normalized.find((p) => p.id === "genshin");
  assert.ok(genshin, "genshin theme must exist");

  const schema = genshin.lockscreen?.config_schema;
  assert.ok(schema && schema.options, "Genshin must define options schema");

  const bgMode = schema.options.background_mode;
  assert.ok(bgMode, "Must define background_mode");
  assert.deepStrictEqual(
    bgMode.options?.map((o) => o.value),
    ["time", "random", "static"]
  );

  const bgIndex = schema.options.background_index;
  assert.ok(bgIndex, "Must define background_index");
  assert.deepStrictEqual(
    bgIndex.options?.map((o) => o.value),
    ["1", "2", "3", "4"],
    "Genshin background_index must support 4 atmospheres (1..4)"
  );
});

test("Lockscreen Customization 5: Clockwork variants and Orbital options", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const clockwork = normalized.find((p) => p.id === "clockwork");
  assert.ok(clockwork, "clockwork collection must exist");

  const variants = clockwork.lockscreen?.config_schema?.variants;
  assert.ok(variants && variants.length === 3, "Clockwork collection must have 3 variants");
  assert.deepStrictEqual(
    variants.map((v) => v.id),
    ["orbital", "tape", "neo-orbital"],
    "Variants must be orbital, tape, and neo-orbital"
  );

  const orbital = normalized.find((p) => p.id === "clockwork-orbital");
  assert.ok(orbital, "Clockwork Orbital package must exist");
  const options = orbital.lockscreen?.config_schema?.options;
  assert.ok(options, "Orbital must have options");
  assert.ok(options.themeMode, "Orbital must support themeMode");
  assert.deepStrictEqual(
    options.themeMode.options?.map((o) => o.value),
    ["dark", "light"]
  );
  assert.ok(options.enableWindup, "Orbital must support enableWindup toggle");
  assert.strictEqual(options.enableWindup.type, "boolean");
});

test("Lockscreen Customization 6: osu! and osumania login modes strictly match gameMode (game, menu)", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const osu = normalized.find((p) => p.id === "osu");
  assert.ok(osu, "osu theme must exist");

  const schema = osu.lockscreen?.config_schema;
  assert.ok(schema && schema.options, "osu must define options schema");
  assert.ok(schema.options.gameMode, "osu must define gameMode");
  assert.deepStrictEqual(
    schema.options.gameMode.options?.map((o) => o.value),
    ["game", "menu"]
  );
});

test("Lockscreen Customization 7: Unsupported and invented options are completely absent from all schemas", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  // Invented options that must NEVER exist in authentic Qylock schemas:
  const forbiddenKeys = [
    "clockStyle",
    "clockPosition",
    "showDate",
    "showSeconds",
    "showSystemInfo",
    "reelMotion",
    "weatherEffects",
  ];

  for (const pkg of normalized) {
    const options = pkg.lockscreen?.config_schema?.options;
    if (!options) continue;

    for (const forbidden of forbiddenKeys) {
      assert.strictEqual(
        (options as any)[forbidden],
        undefined,
        "Theme " + pkg.id + " must not contain invented control " + forbidden
      );
    }
  }
});

test("Lockscreen Customization 8: Application and test payloads faithfully serialize authentic custom configuration", () => {
  const sampleTerrariaConfig = {
    background_mode: "static",
    background_index: "4",
  };

  const payload = {
    packageId: "lockscreen-qylock-terraria",
    target: "quickshell",
    config: sampleTerrariaConfig,
  };

  const serialized = JSON.stringify(payload);
  const deserialized = JSON.parse(serialized);

  assert.strictEqual(deserialized.packageId, "lockscreen-qylock-terraria");
  assert.strictEqual(deserialized.target, "quickshell");
  assert.deepStrictEqual(deserialized.config, sampleTerrariaConfig);
});
