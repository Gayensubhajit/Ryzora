import test from "node:test";
import assert from "node:assert";
import { RAW_QYLOCK_THEMES, normalizeQylockTheme } from "../providers/qylockProvider.ts";
import type { LockscreenConfigSchema, PackageItem } from "../types/index.ts";

test("Lockscreen Customization 1: Configurable themes expose valid config_schema and customizable=true", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  const clockwork = normalized.find((p) => p.id === "clockwork-tape");
  assert.ok(clockwork, "clockwork-tape theme must exist");
  assert.strictEqual(clockwork.customizable, true, "clockwork-tape must have customizable=true");
  assert.ok(clockwork.lockscreen?.config_schema, "clockwork-tape must define config_schema");

  const schema = clockwork.lockscreen!.config_schema!;
  assert.ok(schema.variants && schema.variants.length >= 3, "clockwork-tape must define at least 3 variants");
  assert.ok(schema.options && Object.keys(schema.options).length >= 4, "clockwork-tape must define at least 4 options");
});

test("Lockscreen Customization 2: Non-configurable themes do NOT expose config_schema or customizable=true", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  const nonConfigurableIds = [
    "dog-samurai",
    "nier-automata",
    "forest",
    "material-you",
  ];

  for (const id of nonConfigurableIds) {
    const pkg = normalized.find((p) => p.id === id);
    assert.ok(pkg, `${id} theme must exist`);
    assert.strictEqual(pkg.customizable, false, `${id} must not be flagged customizable`);
    assert.strictEqual(pkg.lockscreen?.config_schema, undefined, `${id} must not have config_schema`);
  }
});

test("Lockscreen Customization 3: Variants structure contains id, name, description, and preview", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const minecraft = normalized.find((p) => p.id === "minecraft");
  assert.ok(minecraft, "minecraft theme must exist");
  assert.strictEqual(minecraft.customizable, true);

  const variants = minecraft.lockscreen?.config_schema?.variants;
  assert.ok(variants && variants.length === 3);

  const [v1, v2, v3] = variants;
  assert.strictEqual(v1.id, "overworld-sunrise");
  assert.strictEqual(v2.id, "nether-fortress");
  assert.strictEqual(v3.id, "the-end");

  for (const v of variants) {
    assert.ok(v.name && v.name.length > 0, "Variant must have a display name");
    assert.ok(v.description && v.description.length > 0, "Variant must have a description");
    assert.ok(v.preview_image && v.preview_image.startsWith("/"), "Variant must have a normalized preview image");
  }
});

test("Lockscreen Customization 4: Option specs strictly define type, default, and valid choices", () => {
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const coffee = normalized.find((p) => p.id === "pixel-coffee");
  assert.ok(coffee, "pixel-coffee theme must exist");
  assert.strictEqual(coffee.customizable, true);

  const options = coffee.lockscreen?.config_schema?.options;
  assert.ok(options);

  // Check select option
  assert.ok(options.clockPosition);
  assert.strictEqual(options.clockPosition.type, "select");
  assert.strictEqual(options.clockPosition.default, "top-left");
  assert.ok(options.clockPosition.options && options.clockPosition.options.length === 3);

  // Check boolean option
  assert.ok(options.weatherEffects);
  assert.strictEqual(options.weatherEffects.type, "boolean");
  assert.strictEqual(options.weatherEffects.default, true);
});

test("Lockscreen Customization 5: Control type mapping logic properly categorizes options", () => {
  // Segmented control (<= 3 choices) vs Dropdown (4+ choices)
  const normalized = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const clockwork = normalized.find((p) => p.id === "clockwork-tape");
  const options = clockwork!.lockscreen!.config_schema!.options!;

  const clockType = options.clockType;
  assert.strictEqual(clockType.type, "select");
  assert.ok(clockType.options!.length <= 3, "2 choices should map to segmented control");

  const reelMotion = options.reelMotion;
  assert.strictEqual(reelMotion.type, "select");
  assert.ok(reelMotion.options!.length <= 3, "3 choices should map to segmented control");
});

test("Lockscreen Customization 6: Application payload can include custom configuration dictionary", () => {
  const sampleConfig = {
    clockType: "analog",
    clockPosition: "top",
    showSeconds: false,
    reelMotion: "slow",
  };

  const payload = {
    packageId: "clockwork-tape",
    target: "quickshell",
    config: sampleConfig,
  };

  const serialized = JSON.stringify(payload);
  const deserialized = JSON.parse(serialized);

  assert.strictEqual(deserialized.packageId, "clockwork-tape");
  assert.strictEqual(deserialized.target, "quickshell");
  assert.deepStrictEqual(deserialized.config, sampleConfig);
});
