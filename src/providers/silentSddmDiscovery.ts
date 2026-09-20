/**
 * Upstream SilentSDDM Discovered Catalog & Constants (Phase S1/S2 Hardened)
 *
 * Full inventory of backgrounds/ directory at pinned commit:
 * uiriansan/SilentSDDM @ 73380d331fe0f8b8a3b17991bb917a0d438a4d34 (v1.5.0)
 *
 * Exact byte sizes and SHA-256 integrity hashes computed against raw upstream assets.
 * Licenses strictly distinguished between theme engine (GPL-3.0-or-later) and individual
 * third-party wallpaper assets (e.g. Pexels, DesktopHut, MoeWalls, or unknown).
 */
import type { SilentSddmWallpaper } from "../types/index.ts";

export const SILENTSDDM_UPSTREAM = {
  repo: "uiriansan/SilentSDDM",
  sha: "73380d331fe0f8b8a3b17991bb917a0d438a4d34",
  version: "1.5.0",
  themeLicense: "GPL-3.0-or-later",
  author: "uiriansan",
  authorAvatar: "https://avatars.githubusercontent.com/u/74550882?v=4",
  rawBase:
    "https://raw.githubusercontent.com/uiriansan/SilentSDDM/73380d331fe0f8b8a3b17991bb917a0d438a4d34",
};

export const SUPPORTED_VIDEO_EXTENSIONS = [
  ".avi",
  ".mp4",
  ".mov",
  ".mkv",
  ".m4v",
  ".webm",
] as const;

export const SUPPORTED_IMAGE_EXTENSIONS = [".jpg", ".jpeg", ".png"] as const;

export const REJECTED_EXTENSIONS = [".gif"] as const;

export const SILENTSDDM_WALLPAPERS: SilentSddmWallpaper[] = [
  {
    id: "silentsddm-default",
    name: "Default (Smoky)",
    filename: "default.jpg",
    path: "backgrounds/default.jpg",
    type: "image",
    sizeBytes: 244814,
    sha256: "27dfe124ff60c361c969a519ab8c42f64ed31917db192cd98ba18dcbf4316b04",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/default.jpg`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/default.jpg`,
    posterFilename: "default.jpg",
    license: {
      theme: "GPL-3.0-or-later",
      asset: "unknown",
      provenance: "upstream-repo",
      credit: "uiriansan",
    },
  },
  {
    id: "silentsddm-smoky",
    name: "Smoky",
    filename: "smoky.jpg",
    path: "backgrounds/smoky.jpg",
    type: "image",
    sizeBytes: 244814,
    sha256: "27dfe124ff60c361c969a519ab8c42f64ed31917db192cd98ba18dcbf4316b04",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/smoky.jpg`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/smoky.jpg`,
    posterFilename: "smoky.jpg",
    license: {
      theme: "GPL-3.0-or-later",
      asset: "unknown",
      provenance: "upstream-repo",
      credit: "uiriansan",
    },
  },
  {
    id: "silentsddm-mountain",
    name: "Mountain",
    filename: "mountain.jpg",
    path: "backgrounds/mountain.jpg",
    type: "image",
    sizeBytes: 235414,
    sha256: "c51cb3bee5a7012ada4e9f74622fd7ae780df45916c34e8d13c2aefbc302372c",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/mountain.jpg`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/mountain.jpg`,
    posterFilename: "mountain.jpg",
    license: {
      theme: "GPL-3.0-or-later",
      asset: "Pexels License (Free to use)",
      provenance: "upstream-acknowledgement",
      credit: "Joyston Judah",
      sourceUrl: "https://www.pexels.com/photo/white-and-black-mountain-wallpaper-933054/",
    },
  },
  {
    id: "silentsddm-ken",
    name: "Ken (Tokyo Ghoul:re)",
    filename: "ken.mp4",
    path: "backgrounds/ken.mp4",
    type: "video",
    sizeBytes: 6699629,
    sha256: "3ed4518c03c2b6f4a56af2fbb35b12dc954a646a8bc809043c5f2ab5f4b12ff0",
    posterFilename: "ken.png",
    posterSizeBytes: 563165,
    posterSha256: "7f023c301dd30607e9b14862a28c611e4d8e0fe95bbcf31167ca6fef36e47c71",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/ken.mp4`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/ken.png`,
    license: {
      theme: "GPL-3.0-or-later",
      asset: "Third-party (Free personal use)",
      provenance: "upstream-acknowledgement",
      credit: "MoeWalls",
      sourceUrl: "https://moewalls.com/anime/ken-kaneki-tokyo-ghoul-re-3-live-wallpaper/",
    },
  },
  {
    id: "silentsddm-rei",
    name: "Rei (Blue Light Anime Girl)",
    filename: "rei.mp4",
    path: "backgrounds/rei.mp4",
    type: "video",
    sizeBytes: 2757655,
    sha256: "af474934a850717f6cf025bd1a4af2796665caee4ba128127a8cc95e3864148a",
    posterFilename: "rei.png",
    posterSizeBytes: 629615,
    posterSha256: "b634deb145616c23bafb55119313ba7f7e13fcd338b0db2b5d97e23e30577bd3",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/rei.mp4`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/rei.png`,
    license: {
      theme: "GPL-3.0-or-later",
      asset: "Third-party (Free personal use)",
      provenance: "upstream-acknowledgement",
      credit: "DesktopHut",
      sourceUrl: "https://www.desktophut.com/blue-light-anime-girl-6794",
    },
  },
  {
    id: "silentsddm-silvia",
    name: "Silvia (Nissan Silvia S15)",
    filename: "silvia.mp4",
    path: "backgrounds/silvia.mp4",
    type: "video",
    sizeBytes: 2665781,
    sha256: "4187bedeb371d99f4fb132a2550d2dce215e25e6e3bc820b98c9f23b6dbe1362",
    posterFilename: "silvia.png",
    posterSizeBytes: 832911,
    posterSha256: "7f9cbd52f2e772778cd10b22b9a966cf1d86e824d4ef57823ea53f9cdd8c9baa",
    downloadUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/silvia.mp4`,
    posterUrl: `${SILENTSDDM_UPSTREAM.rawBase}/backgrounds/silvia.png`,
    license: {
      theme: "GPL-3.0-or-later",
      asset: "Third-party (Free personal use)",
      provenance: "upstream-acknowledgement",
      credit: "MoeWalls",
      sourceUrl: "https://moewalls.com/anime/anime-girl-nissan-silvia-live-wallpaper/",
    },
  },
];

export function getDiscoveredSilentSddmWallpapers(): SilentSddmWallpaper[] {
  return [...SILENTSDDM_WALLPAPERS];
}

export function isSupportedVideoExtension(ext: string): boolean {
  const normalized = ext.toLowerCase().startsWith(".") ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
  return (SUPPORTED_VIDEO_EXTENSIONS as readonly string[]).includes(normalized as any);
}

export function isSupportedImageExtension(ext: string): boolean {
  const normalized = ext.toLowerCase().startsWith(".") ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
  return (SUPPORTED_IMAGE_EXTENSIONS as readonly string[]).includes(normalized as any);
}

export function isRejectedExtension(ext: string): boolean {
  const normalized = ext.toLowerCase().startsWith(".") ? ext.toLowerCase() : `.${ext.toLowerCase()}`;
  return (REJECTED_EXTENSIONS as readonly string[]).includes(normalized as any);
}
