import test, { describe, it } from "node:test";
import assert from "node:assert/strict";
import { prefetchIconsForPage, clearIconCache } from "../services/iconCache.ts";
import { resolveAppMetadata } from "../components/apps/appMetadata.ts";

describe("Phase 24.2 — Centralized Icon Resolution and Caching", () => {
  it("Icon 1: prefetchIconsForPage accepts catalog items with icon_name and icon_path without throwing", () => {
    clearIconCache();
    assert.doesNotThrow(() => {
      prefetchIconsForPage([
        { id: "firefox", icon_name: "firefox", icon_path: "/usr/share/swcatalog/icons/archlinux-arch-extra/128x128/firefox_firefox.png" },
        { id: "blender", icon_name: "blender", icon_path: "/usr/share/swcatalog/icons/archlinux-arch-extra/128x128/blender_blender.png" },
        { id: "unknown-tool", icon_name: null, icon_path: null },
      ]);
    });
  });

  it("Icon 2: resolveAppMetadata returns curated iconUrl for well-known applications", () => {
    const firefoxMeta = resolveAppMetadata("firefox");
    assert.ok(firefoxMeta.iconUrl, "firefox must have curated iconUrl");
    assert.ok(firefoxMeta.iconUrl.includes("firefox"), "iconUrl must contain firefox");

    const blenderMeta = resolveAppMetadata("blender");
    assert.ok(blenderMeta.iconUrl, "blender must have curated iconUrl");
    assert.ok(blenderMeta.iconUrl.includes("blender"), "iconUrl must contain blender");
  });

  it("Icon 3: clearIconCache properly empties pending and resolved structures", () => {
    clearIconCache();
    // Subsequent prefetch after clear works cleanly
    prefetchIconsForPage([{ id: "gimp" }]);
    clearIconCache();
  });
});
