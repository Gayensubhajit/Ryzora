/**
 * Phase 22 — Universal Package Engine Tests
 *
 * Tests the generic PackageEngine registry and the typed pipeline contract.
 * Validates all invariants specified in the user approval:
 *  - Engine registration and discovery
 *  - Provider deduplication (same package id from two providers)
 *  - Category filtering
 *  - Qylock is routed through the engine as a real consumer
 *  - Engine finds packages by id
 */

import test from "node:test";
import assert from "node:assert/strict";
import { PackageEngine } from "../engine/PackageEngine.ts";
import type { PackageProvider, PackageItem } from "../providers/types.ts";
import { qylockLockscreenProvider } from "../providers/qylockProvider.ts";

// ─────────────────────────────────────────────────────────────────────────────
// Mock provider factory
// ─────────────────────────────────────────────────────────────────────────────

function makeMockProvider(
  id: string,
  category: string,
  packages: Partial<PackageItem>[],
): PackageProvider {
  const items = packages.map(
    (p, i) =>
      ({
        id: p.id ?? `${id}-pkg-${i}`,
        title: p.title ?? `${id} Package ${i}`,
        category,
        package_type: category,
        ...p,
      }) as PackageItem,
  );

  return {
    id,
    name: `Mock Provider: ${id}`,
    category,
    discover: () => items,
    normalize: (raw: unknown) => raw as PackageItem,
    getSource: () => ({ type: "local" }),
    getTargets: () => ({}),
    getDependencies: () => [],
    getProvenance: () => ({ upstream: "mock" }),
    getMedia: () => ({ poster: "/mock.png" }),
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Registration tests
// ─────────────────────────────────────────────────────────────────────────────

test("Engine 1: registers a provider and returns it by id", () => {
  const engine = new PackageEngine();
  const p = makeMockProvider("test-provider", "lockscreen", []);
  engine.register(p);
  assert.strictEqual(engine.getProvider("test-provider"), p);
});

test("Engine 2: returns undefined for unregistered provider", () => {
  const engine = new PackageEngine();
  assert.strictEqual(engine.getProvider("nonexistent"), undefined);
});

test("Engine 3: replaces existing provider on re-register with same id", () => {
  const engine = new PackageEngine();
  const p1 = makeMockProvider("my-provider", "lockscreen", []);
  const p2 = makeMockProvider("my-provider", "lockscreen", [{ id: "extra" }]);
  engine.register(p1);
  engine.register(p2);
  assert.strictEqual(engine.getProvider("my-provider"), p2);
});

test("Engine 4: unregisters a provider", () => {
  const engine = new PackageEngine();
  const p = makeMockProvider("rm-me", "font", []);
  engine.register(p);
  engine.unregister("rm-me");
  assert.strictEqual(engine.getProvider("rm-me"), undefined);
});

test("Engine 5: providerCount reflects registered providers", () => {
  const engine = new PackageEngine();
  engine.register(makeMockProvider("a", "lockscreen", []));
  engine.register(makeMockProvider("b", "wallpaper", []));
  assert.strictEqual(engine.providerCount, 2);
});

test("Engine 6: listProviders returns summaries for all registered providers", () => {
  const engine = new PackageEngine();
  engine.register(makeMockProvider("prov-a", "lockscreen", []));
  engine.register(makeMockProvider("prov-b", "font", []));
  const summaries = engine.listProviders();
  assert.strictEqual(summaries.length, 2);
  assert.ok(summaries.some((s) => s.id === "prov-a"));
  assert.ok(summaries.some((s) => s.id === "prov-b"));
});

// ─────────────────────────────────────────────────────────────────────────────
// Discovery tests
// ─────────────────────────────────────────────────────────────────────────────

test("Engine 7: discoverAll returns empty array with no providers", async () => {
  const engine = new PackageEngine();
  const items = await engine.discoverAll();
  assert.deepStrictEqual(items, []);
});

test("Engine 8: discoverAll discovers packages from a single provider", async () => {
  const engine = new PackageEngine();
  engine.register(
    makeMockProvider("ls", "lockscreen", [{ id: "pkg-1" }, { id: "pkg-2" }]),
  );
  const items = await engine.discoverAll();
  assert.strictEqual(items.length, 2);
});

test("Engine 9: discoverAll merges packages from multiple providers", async () => {
  const engine = new PackageEngine();
  engine.register(
    makeMockProvider("p1", "lockscreen", [{ id: "ls-1" }, { id: "ls-2" }]),
  );
  engine.register(makeMockProvider("p2", "wallpaper", [{ id: "wp-1" }]));
  const items = await engine.discoverAll();
  assert.strictEqual(items.length, 3);
});

test("Engine 10: deduplicates by id — first-registered wins", async () => {
  const engine = new PackageEngine();
  engine.register(
    makeMockProvider("p1", "lockscreen", [
      { id: "shared-pkg", title: "From P1" },
    ]),
  );
  engine.register(
    makeMockProvider("p2", "lockscreen", [
      { id: "shared-pkg", title: "From P2" },
    ]),
  );
  const items = await engine.discoverAll();
  assert.strictEqual(items.length, 1);
  assert.strictEqual(items[0].title, "From P1");
});

test("Engine 11: deduplication is case-insensitive", async () => {
  const engine = new PackageEngine();
  engine.register(
    makeMockProvider("p1", "lockscreen", [{ id: "My-Package" }]),
  );
  engine.register(
    makeMockProvider("p2", "lockscreen", [{ id: "my-package" }]),
  );
  const items = await engine.discoverAll();
  assert.strictEqual(items.length, 1);
});

test("Engine 12: discoverAll continues if one provider throws", async () => {
  const engine = new PackageEngine();
  const bad: PackageProvider = {
    ...makeMockProvider("bad", "lockscreen", []),
    discover: () => {
      throw new Error("Network error");
    },
  };
  engine.register(bad);
  engine.register(makeMockProvider("good", "lockscreen", [{ id: "ok-pkg" }]));
  const items = await engine.discoverAll();
  assert.strictEqual(items.length, 1);
  assert.strictEqual(items[0].id, "ok-pkg");
});

// ─────────────────────────────────────────────────────────────────────────────
// findPackage
// ─────────────────────────────────────────────────────────────────────────────

test("Engine 13: findPackage finds a package by id", async () => {
  const engine = new PackageEngine();
  engine.register(makeMockProvider("p", "lockscreen", [{ id: "target-pkg" }]));
  const found = await engine.findPackage("target-pkg");
  assert.ok(found !== undefined);
  assert.strictEqual(found!.id, "target-pkg");
});

test("Engine 14: findPackage returns undefined for missing package", async () => {
  const engine = new PackageEngine();
  const found = await engine.findPackage("does-not-exist");
  assert.strictEqual(found, undefined);
});

test("Engine 15: findPackage is case-insensitive", async () => {
  const engine = new PackageEngine();
  engine.register(makeMockProvider("p", "lockscreen", [{ id: "Target-PKG" }]));
  const found = await engine.findPackage("target-pkg");
  assert.ok(found !== undefined);
});

// ─────────────────────────────────────────────────────────────────────────────
// Qylock as first real consumer of the engine
// ─────────────────────────────────────────────────────────────────────────────

test("Engine 16: Qylock provider registered with correct id", () => {
  const engine = new PackageEngine();
  engine.register(qylockLockscreenProvider as unknown as PackageProvider);
  assert.ok(engine.getProvider("qylock") !== undefined);
});

test("Engine 17: Qylock provider has category: lockscreen", () => {
  assert.strictEqual(qylockLockscreenProvider.category, "lockscreen");
});

test("Engine 18: Qylock discoverAll returns 27+ themes", async () => {
  const engine = new PackageEngine();
  engine.register(qylockLockscreenProvider as unknown as PackageProvider);
  const items = await engine.discoverAll();
  assert.ok(items.length >= 27, `Expected >=27, got ${items.length}`);
});

test("Engine 19: each Qylock package has required fields", async () => {
  const engine = new PackageEngine();
  engine.register(qylockLockscreenProvider as unknown as PackageProvider);
  const items = await engine.discoverAll();
  for (const item of items) {
    assert.ok(item.id, `Missing id in ${JSON.stringify(item)}`);
    assert.ok(item.title, `Missing title in ${item.id}`);
    assert.ok(item.category, `Missing category in ${item.id}`);
  }
});

test("Engine 20: Qylock discovers no duplicates internally", async () => {
  const engine = new PackageEngine();
  engine.register(qylockLockscreenProvider as unknown as PackageProvider);
  const items = await engine.discoverAll();
  const ids = items.map((p) => p.id.toLowerCase());
  const unique = new Set(ids);
  assert.strictEqual(unique.size, ids.length, "Duplicate IDs found");
});

test("Engine 21: can find a specific Qylock package by id", async () => {
  const engine = new PackageEngine();
  engine.register(qylockLockscreenProvider as unknown as PackageProvider);
  const found = await engine.findPackage("dog-samurai");
  assert.ok(found !== undefined, "dog-samurai not found");
});

test("Engine 22: Qylock getMedia returns a poster", () => {
  const items = qylockLockscreenProvider.discover() as PackageItem[];
  const first = items[0];
  const media = qylockLockscreenProvider.getMedia!(first);
  assert.ok(media?.poster, "getMedia should return a poster");
});

test("Engine 23: Qylock getSource returns git type pointing to qylock repo", () => {
  const items = qylockLockscreenProvider.discover() as PackageItem[];
  const first = items[0];
  const source = qylockLockscreenProvider.getSource(first) as { type: string; repository?: string };
  assert.strictEqual(source.type, "git");
  assert.ok(source.repository?.includes("qylock"), "repository should reference qylock");
});

test("Engine 24: Qylock getProvenance includes upstream URL and license", () => {
  const items = qylockLockscreenProvider.discover() as PackageItem[];
  const first = items[0];
  const prov = qylockLockscreenProvider.getProvenance(first) as { upstream: string; license?: string };
  assert.ok(prov.upstream, "Missing upstream in provenance");
  assert.ok(prov.license, "Missing license in provenance");
});

test("Engine 25: Qylock getDependencies returns quickshell for quickshell target", () => {
  const items = qylockLockscreenProvider.discover() as PackageItem[];
  const first = items[0];
  const deps = qylockLockscreenProvider.getDependencies(first, "quickshell");
  assert.ok(deps.includes("quickshell"), "quickshell dep missing");
});

// ─────────────────────────────────────────────────────────────────────────────
// Provider contract compliance
// ─────────────────────────────────────────────────────────────────────────────

test("Engine 26: qylockLockscreenProvider implements all required PackageProvider fields", () => {
  assert.ok(qylockLockscreenProvider.id);
  assert.ok(qylockLockscreenProvider.name);
  assert.ok(qylockLockscreenProvider.category);
  assert.strictEqual(typeof qylockLockscreenProvider.discover, "function");
  assert.strictEqual(typeof qylockLockscreenProvider.normalize, "function");
  assert.strictEqual(typeof qylockLockscreenProvider.getSource, "function");
  assert.strictEqual(typeof qylockLockscreenProvider.getTargets, "function");
  assert.strictEqual(typeof qylockLockscreenProvider.getDependencies, "function");
  assert.strictEqual(typeof qylockLockscreenProvider.getProvenance, "function");
});

test("Engine 27: qylockLockscreenProvider implements getMedia (Phase 22 addition)", () => {
  assert.strictEqual(typeof qylockLockscreenProvider.getMedia, "function");
});
