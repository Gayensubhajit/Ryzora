import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import { catalogService } from "../services/catalogService.ts";
import type { CatalogItem } from "../services/catalogService.ts";

describe("Phase 24 — Dynamic Arch Linux Repository Catalog Engine", () => {
  it("Catalog 1: Retrieves catalog status with fallback values in headless environment", async () => {
    const status = await catalogService.getStatus();
    assert.ok(status.total_packages > 0, "total_packages must be greater than zero");
    assert.ok(status.total_applications > 0, "total_applications must be greater than zero");
    assert.ok(status.enabled_repositories.length > 0, "must report configured repositories");
    assert.ok(status.enabled_repositories.includes("core") || status.enabled_repositories.includes("extra"), "must include standard Arch repos");
    assert.strictEqual(status.is_refreshing, false, "initial refreshing state is false");
  });

  it("Catalog 2: Queries applications catalog and respects is_application_only filter", async () => {
    const res = await catalogService.getItems({ is_application_only: true, page_size: 50 });
    assert.ok(res.items.length > 0, "must return applications");
    for (const item of res.items) {
      assert.strictEqual(item.is_application, true, "item must be a desktop application");
      assert.ok(item.id.length > 0, "item must have an id");
      assert.ok(item.display_name.length > 0, "item must have a display name");
    }
  });

  it("Catalog 3: Category filtering accurately isolates Graphics and Internet apps", async () => {
    const gfx = await catalogService.getItems({ category: "Graphics", page_size: 50 });
    assert.ok(gfx.items.length > 0, "must find graphics apps");
    assert.ok(
      gfx.items.some((i) => i.id === "blender" || i.id === "gimp"),
      "blender or gimp must be in Graphics category"
    );

    const net = await catalogService.getItems({ category: "Internet", page_size: 50 });
    assert.ok(net.items.length > 0, "must find internet apps");
    assert.ok(
      net.items.some((i) => i.id === "firefox"),
      "firefox must be in Internet category"
    );
  });

  it("Catalog 4: Search accurately finds packages by ID, display name, and summary", async () => {
    const blenderSearch = await catalogService.getItems({ search_query: "blender" });
    assert.ok(blenderSearch.items.length > 0, "search for blender must return results");
    assert.strictEqual(blenderSearch.items[0].id, "blender", "blender must be top match");

    const vlcSearch = await catalogService.getItems({ search_query: "media player" });
    assert.ok(vlcSearch.items.length > 0, "search for media player must return results");
    assert.ok(
      vlcSearch.items.some((i) => i.id === "vlc"),
      "vlc must be matched by media player summary"
    );
  });

  it("Catalog 5: Converts CatalogItem to canonical PackageItem with correct metadata", async () => {
    const item = await catalogService.getItemDetails("blender");
    assert.ok(item, "blender item details must be found");

    const pkg = catalogService.catalogItemToPackageItem(item!);
    assert.strictEqual(pkg.id, "blender");
    assert.strictEqual(pkg.category, "apps");
    assert.strictEqual(pkg.package_type, "app");
    assert.ok(pkg.tags.includes("pacman"), "tags must include pacman");
    assert.ok(pkg.tags.includes("application"), "tags must include application");
    assert.strictEqual(pkg.dependencies.packages.length, item!.dependencies.length);
  });

  it("Catalog 6: Separates download size from installed size honestly", async () => {
    const blender = await catalogService.getItemDetails("blender");
    assert.ok(blender);
    assert.ok(blender.download_size, "blender must have download size");
    assert.ok(blender.installed_size, "blender must have installed size");
    assert.notStrictEqual(blender.download_size, blender.installed_size, "download size and installed size must be distinct");
  });

  it("Catalog 7: Subscription receives updates on manual refresh", async () => {
    let notified = false;
    const unsub = catalogService.subscribe((status) => {
      if (status) notified = true;
    });

    const status = await catalogService.refreshCatalog();
    assert.ok(status);
    assert.strictEqual(notified, true, "subscriber must have been notified");
    unsub();
  });
});
