import { convertFileSrc } from "@tauri-apps/api/core";
/**
 * Custom Media Provider for SilentSDDM / Ryzora Lock Screens
 *
 * Normalizes user-imported custom media assets (photos and videos)
 * into first-class PackageItem representations with pure SDDM targets.
 */

import type { PackageItem, SilentSddmCachedAsset, LockscreenManifest } from "../types/index.ts";

/**
 * Resolves a local filesystem path to an asset URL safe for webview rendering.
 * Expands leading ~/ to the user home directory so convertFileSrc receives an
 * absolute path (tilde paths are NOT expanded by the browser or Tauri).
 */
export function resolveLocalAssetSrc(filePath: string): string {
  if (!filePath) return filePath;
  // Expand ~ to home only in a Tauri context where we can read the env
  let absPath = filePath;
  if (absPath.startsWith("~/") || absPath === "~") {
    const home =
      (typeof window !== "undefined" ? (window as any).__RYZORA_HOME__ : undefined) ||
      (typeof (globalThis as any).process !== "undefined" ? (globalThis as any).process.env?.HOME : undefined);
    if (home) {
      absPath = home + absPath.slice(1);
    } else {
      return absPath; // no home yet — caller will retry
    }
  }

  // Use the local HTTP media server for absolute paths.
  // WebKitGTK on Linux does NOT forward Range headers to custom URI scheme handlers
  // (asset://), so video streaming of large files requires a real HTTP server.
  if (absPath.startsWith("/") && typeof window !== "undefined") {
    const port = (window as any).__RYZORA_MEDIA_PORT__;
    if (port) {
      return `http://127.0.0.1:${port}/media?path=${encodeURIComponent(absPath)}`;
    }
    // Fallback: asset:// (works for images; large videos may fail on Linux)
    return convertFileSrc(absPath);
  }

  return absPath;
}

export function normalizeCustomMediaAsset(asset: SilentSddmCachedAsset): PackageItem {
  const isVideo = asset.media_type === "video";
  const displayName = asset.display_name?.trim() || asset.filename.replace(/\.[^.]+$/, "");
  const title = `Custom · ${displayName}`;

  // Use the absolute original_path for media (no-copy import — file stays at user's location)
  // Fall back to legacy custom storage path if original_path is absent (old imports)
  const rawMediaPath = asset.original_path || `~/.local/share/ryzora/lockscreens/silentsddm/custom/${asset.filename}`;
  const basePath = resolveLocalAssetSrc(rawMediaPath);

  // Poster lives in Ryzora posters dir (absolute path derived from home)
  // We rely on the poster path being discoverable via the asset filename key
  const rawPosterPath = `~/.local/share/ryzora/lockscreens/silentsddm/posters/${asset.filename}.poster.jpg`;
  const posterPath = isVideo ? resolveLocalAssetSrc(rawPosterPath) : basePath;

  const manifest: LockscreenManifest = {
    provider: "silentsddm",
    targets: {
      sddm: {
        id: "sddm",
        name: "SDDM Login & Lock Screen",
        scope: "system",
        supported: true,
        entrypoint: "Main.qml",
        dependencies: [
          "sddm",
          "qt6-declarative",
          "qt6-svg",
          ...(isVideo ? ["qt6-multimedia", "gst-plugins-good"] : []),
        ],
        requires_session_lock: false,
        requires_root: true,
        install_destination: "/usr/share/sddm/themes/ryzora-silent",
        config_file: "/etc/sddm.conf.d/zz-ryzora-theme.conf",
      },
    },
    media: {
      poster: posterPath,
      preview_video: isVideo ? basePath : undefined,
      has_audio: false,
      aspect_ratio: "16:9",
    },
    runtime: {
      entrypoint: "Main.qml",
      assets: [asset.filename, `${asset.filename}.poster.jpg`],
      bg_video_path: isVideo ? asset.filename : undefined,
      requires_multimedia: isVideo,
    },
    provenance: {
      provider_name: "silentsddm",
      upstream_repo: "local",
      upstream_revision: asset.sha256,
      upstream_path: asset.original_path || asset.filename,
      author: "You",
      license: "Custom user media",
    },
  };

  return {
    id: asset.id,
    title,
    subtitle: "Custom Background · Local file",
    description: `Custom ${asset.media_type} background '${displayName}' · Local file`,
    version: "1.0.0",
    author: {
      name: "You",
      avatar: "",
      verified: true,
    },
    category: "lockscreens",
    package_type: "lockscreen",
    tags: ["silentsddm", "sddm", "custom", "my media", asset.media_type],
    supported_desktops: ["universal", "hyprland", "sway", "kde", "gnome"],
    supported_display: ["wayland", "x11"],
    rating: 0,
    rating_count: 0,
    downloads: 0,
    hero_image: posterPath,
    screenshots: [posterPath],
    media_type: isVideo ? "video" : "image",
    preview_video_url: isVideo ? basePath : undefined,
    preview_poster_url: posterPath,
    // Store absolute original path for Apply-time hash verification
    original_path: asset.original_path,
    supports_session_lock: false,
    supports_login_screen: true,
    lockscreen: manifest,
    customizable: false,
    color_palette: ["#1a1b26", "#bb9af7", "#7aa2f7"],
    safety_audit: {
      rating: "verified",
      changes_system_files: true,
      requires_root: true,
      sandbox_compatible: true,
      files_modified_count: 2,
    },
    dependencies: {
      packages: isVideo
        ? ["sddm", "qt6-declarative", "qt6-svg", "qt6-multimedia", "gst-plugins-good"]
        : ["sddm", "qt6-declarative", "qt6-svg"],
      optional: [],
    },
    components: [
      {
        name: asset.filename,
        component_type: "custom_media",
        target_path: basePath,
        description: `Custom media file: ${asset.filename} (${asset.size_bytes} bytes)`,
      },
    ],
    compatibility: {
      supported_distros: ["all"],
      supported_desktops: ["universal", "hyprland", "sway", "kde", "gnome"],
      supported_sessions: ["wayland", "x11"],
      required_binaries: ["sddm"],
      optional_binaries: [],
    },
    content_hash: asset.sha256,
    package_size_bytes: asset.size_bytes,
  };
}
