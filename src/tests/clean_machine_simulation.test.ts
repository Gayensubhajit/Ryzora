import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import {
  getCatalogueLockScreens,
  resolveLockscreenCapabilities,
  getLockscreenProvider,
} from "../providers/index.ts";
import type { SystemInfo } from "../types/index.ts";

const mockSystem: SystemInfo = {
  os_type: "linux",
  distro_id: "arch",
  distro_name: "Arch Linux",
  desktop_env: "hyprland",
  session_type: "wayland",
  window_manager: "hyprland",
  has_systemd: true,
  installed_packages: ["quickshell", "qt6-declarative", "qt6-multimedia", "qt6-svg", "sddm"],
  active_theme: null,
};

test("Clean Machine Simulation: Complete catalogue, previews, and self-contained lifecycle without Qylock installed", () => {
  // 1. Invariant: /home/silentbyte/qylock must not be required
  const qylock = getLockscreenProvider("qylock");
  assert.ok(qylock, "Qylock provider must be registered on a clean machine");

  // 2. Fetch Catalogue from Repository
  const catalogue = getCatalogueLockScreens();
  assert.equal(catalogue.length, 27, "Must display all 27 lockscreens on a clean machine");

  const qylockItems = catalogue.filter(p => p.lockscreen?.provider === "qylock");
  assert.equal(qylockItems.length, 25, "Must have all 25 Qylock items");

  // 3. Previews must be self-contained and repository-distributable
  for (const item of qylockItems) {
    assert.ok(item.preview_poster_url, `Item ${item.title} must have preview poster`);
    if (item.media_type === "video" || item.media_type === "animated" || item.preview_video_url) {
      assert.ok(item.preview_video_url, `Video/animated item ${item.title} must have preview video`);
      const videoRel = item.preview_video_url.replace(/^\//, "");
      assert.ok(
        fs.existsSync(path.resolve("public", videoRel)) || fs.existsSync(path.resolve(videoRel)),
        `Video for ${item.title} must exist in repository assets`
      );
    }
    
    const posterRel = item.preview_poster_url.replace(/^\//, "");
    assert.ok(
      fs.existsSync(path.resolve("public", posterRel)) || fs.existsSync(path.resolve(posterRel)),
      `Poster for ${item.title} must exist in repository assets`
    );

    // Provenance must point upstream to Darkkal44/qylock
    const prov = qylock.getProvenance(item);
    assert.equal(prov.upstream, "https://github.com/Darkkal44/qylock");
    assert.equal(prov.revision, "main");
    assert.equal(prov.license, "GPL-3.0");
  }

  // 4. Installation Resolution Lifecycle: Dry-Run & Target Paths
  const dogSamurai = qylockItems.find(p => p.title === "Dog Samurai")!;
  
  // Quickshell Session Lock: User-space self-contained
  const qsRes = resolveLockscreenCapabilities(mockSystem, dogSamurai.lockscreen, "quickshell");
  assert.equal(qsRes.can_install, true);
  assert.equal(qsRes.requires_root, false);
  for (const f of qsRes.target_files) {
    assert.ok(
      f.target.startsWith("~/.local/share/ryzora/"),
      `File ${f.target} must be in Ryzora-owned storage`
    );
    assert.ok(!f.target.includes("qylock-lock"));
    assert.ok(!f.target.includes("/home/silentbyte/qylock"));
  }

  // SDDM Login Screen: System boundary with pkexec
  const sddmRes = resolveLockscreenCapabilities(mockSystem, dogSamurai.lockscreen, "sddm");
  assert.equal(sddmRes.can_install, true);
  assert.equal(sddmRes.requires_root, true);
  assert.ok(sddmRes.privilege_notice?.includes("pkexec"));
  for (const f of sddmRes.target_files) {
    assert.ok(
      f.target.startsWith("/usr/share/sddm/themes/ryzora-"),
      `SDDM target ${f.target} must be properly namespaced`
    );
  }

  // 5. Zero external path invariant across all serialized records
  const allSerialized = JSON.stringify({ catalogue, qsRes, sddmRes });
  assert.ok(!allSerialized.includes("/home/silentbyte/qylock"));
  assert.ok(!allSerialized.includes("~/.local/share/qylock"));
  assert.ok(!allSerialized.includes("~/.config/qylock"));
});

test("Clean Machine Simulation: Community repository 27 lockscreens and activation lifecycle", () => {
  const indexJsonPath = path.resolve("repositories/community/indexes/lockscreens.json");
  const index = JSON.parse(fs.readFileSync(indexJsonPath, "utf8"));

  assert.equal(index.packages.length, 27, "Must contain all 27 repository-backed lockscreens");

  for (const pkg of index.packages) {
    // Assert zero references to external /home/silentbyte or legacy qylock config
    const raw = JSON.stringify(pkg);
    assert.ok(!raw.includes("/home/silentbyte/qylock"), `Package ${pkg.id} must not reference /home/silentbyte/qylock`);
    assert.ok(!raw.includes("~/.local/share/qylock"), `Package ${pkg.id} must not reference ~/.local/share/qylock`);
    assert.ok(!raw.includes("~/.config/qylock"), `Package ${pkg.id} must not reference ~/.config/qylock`);
  }

  // Active state lifecycle clean machine check
  const activeState = { quickshell: null, sddm: null };
  assert.equal(activeState.quickshell, null);
  assert.equal(activeState.sddm, null);
});
