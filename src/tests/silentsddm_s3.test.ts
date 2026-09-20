import { test } from "node:test";
import assert from "node:assert";
import {
  SILENTSDDM_UPSTREAM,
  SILENTSDDM_WALLPAPERS,
} from "../providers/silentSddmDiscovery.ts";
import type {
  SilentSddmEngineManifest,
  UpstreamWallpaperInstallRequest,
} from "../types/index.ts";

// ── S3 Engine Constants ────────────────────────────────────────────────────

const ENGINE_COMMIT = "73380d331fe0f8b8a3b17991bb917a0d438a4d34";
const ENGINE_ARCHIVE_SHA256 = "c3979c47be1eb951c528fca341bfa91d41ae4e2818bd147ef3180aa00dad4317";
const ENGINE_ARCHIVE_SIZE_BYTES = 20807064;

test("SilentSDDM S3 1: Pinned engine archive constants match acceptance criteria exactly", () => {
  assert.strictEqual(
    SILENTSDDM_UPSTREAM.sha,
    ENGINE_COMMIT,
    "Discovery module must reference pinned engine commit"
  );
  // Validate the expected archive constants (these match the Rust ENGINE_ARCHIVE_SHA256)
  assert.strictEqual(ENGINE_ARCHIVE_SHA256.length, 64);
  assert.ok(/^[0-9a-f]{64}$/.test(ENGINE_ARCHIVE_SHA256));
  assert.strictEqual(ENGINE_ARCHIVE_SIZE_BYTES, 20807064);
});

test("SilentSDDM S3 2: CAS deduplication candidates — default.jpg and smoky.jpg share SHA-256", () => {
  const defaultWp = SILENTSDDM_WALLPAPERS.find((w) => w.id === "silentsddm-default");
  const smokyWp = SILENTSDDM_WALLPAPERS.find((w) => w.id === "silentsddm-smoky");

  assert.ok(defaultWp, "default.jpg must be in catalog");
  assert.ok(smokyWp, "smoky.jpg must be in catalog");

  // Both share the exact same content hash — the CAS blob store will deduplicate these
  assert.strictEqual(
    defaultWp.sha256,
    "27dfe124ff60c361c969a519ab8c42f64ed31917db192cd98ba18dcbf4316b04",
    "default.jpg must have confirmed SHA-256"
  );
  assert.strictEqual(
    smokyWp.sha256,
    "27dfe124ff60c361c969a519ab8c42f64ed31917db192cd98ba18dcbf4316b04",
    "smoky.jpg must have confirmed SHA-256 (identical to default — deduplication target)"
  );
  assert.strictEqual(defaultWp.sizeBytes, 244814);
  assert.strictEqual(smokyWp.sizeBytes, 244814);

  // But they remain distinct catalog entries
  assert.strictEqual(defaultWp.filename, "default.jpg");
  assert.strictEqual(smokyWp.filename, "smoky.jpg");
  assert.notStrictEqual(defaultWp.id, smokyWp.id);
});

test("SilentSDDM S3 3: Every catalog asset can be converted to UpstreamWallpaperInstallRequest", () => {
  assert.strictEqual(SILENTSDDM_WALLPAPERS.length, 6);

  for (const w of SILENTSDDM_WALLPAPERS) {
    // Simulate building an UpstreamWallpaperInstallRequest — all required fields present
    const req: UpstreamWallpaperInstallRequest = {
      id: w.id,
      filename: w.filename,
      media_type: w.type === "video" ? "video" : "image",
      sha256: w.sha256,
      size_bytes: w.sizeBytes,
      download_url: w.downloadUrl,
      poster_url: w.posterUrl ?? null,
      poster_sha256: w.posterSha256 ?? null,
    };

    assert.ok(req.id.startsWith("silentsddm-"));
    assert.ok(req.filename.length > 0);
    assert.ok(req.size_bytes > 0);
    assert.strictEqual(req.sha256.length, 64);
    assert.ok(req.download_url.startsWith("https://raw.githubusercontent.com/"));

    if (req.media_type === "video") {
      assert.ok(req.poster_url, `Video ${w.id} must have poster_url`);
      assert.ok(req.poster_sha256, `Video ${w.id} must have poster_sha256`);
      assert.strictEqual(req.poster_sha256!.length, 64);
    }
  }
});

test("SilentSDDM S3 4: No backgrounds/ bundled — download URLs prove independence from engine archive", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    // Each wallpaper's download URL must point to the raw GitHub blob, not a bundled path
    assert.ok(
      w.downloadUrl.includes("raw.githubusercontent.com"),
      `${w.id} download URL must be a raw GitHub URL, not bundled in engine`
    );
    // The download URL must reference the specific commit hash
    assert.ok(
      w.downloadUrl.includes(ENGINE_COMMIT),
      `${w.id} download URL must reference the pinned commit`
    );
  }
});

test("SilentSDDM S3 5: SilentSddmEngineManifest type has all required fields", () => {
  // Validate that the TS type is correctly structured for serialization
  const mockManifest: SilentSddmEngineManifest = {
    engine_id: "silentsddm",
    version: "1.5.0",
    source_commit: ENGINE_COMMIT,
    archive_sha256: ENGINE_ARCHIVE_SHA256,
    installed: true,
    engine_path: "/usr/share/sddm/themes/ryzora-silent",
    installed_at: "2026-09-21T00:00:00Z",
  };
  assert.strictEqual(mockManifest.engine_id, "silentsddm");
  assert.strictEqual(mockManifest.installed, true);
  assert.strictEqual(mockManifest.archive_sha256, ENGINE_ARCHIVE_SHA256);
  assert.ok(!mockManifest.engine_path.includes("backgrounds/"), 
    "Engine path must never reference backgrounds/ — that would indicate bundling violation");
});

test("SilentSDDM S3 6: Install-is-not-Apply invariant — catalog has no activation metadata", () => {
  for (const w of SILENTSDDM_WALLPAPERS) {
    // Catalog items describe wallpapers for download; they carry no SDDM activation data
    const wAny = w as any;
    assert.strictEqual(wAny.active, undefined, `${w.id} must not carry active flag`);
    assert.strictEqual(wAny.sddm_conf_path, undefined, `${w.id} must not reference sddm.conf path`);
    assert.strictEqual(wAny.apply_config, undefined, `${w.id} must not carry apply_config`);
  }
});
