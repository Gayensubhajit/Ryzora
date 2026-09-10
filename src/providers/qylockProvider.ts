import type {
  LockscreenProvider,
  LockscreenMediaSpec,
  LockscreenSourceSpec,
  LockscreenTargetsSpec,
  LockscreenProvenance,
} from "./types.ts";
import type {
  PackageItem,
  LockscreenManifest,
  LockscreenTargetType,
  SystemInfo,
  LockscreenConfigSchema,
} from "../types/index.ts";

export interface RawQylockTheme {
  id: string;
  name: string;
  version: string;
  author: string;
  summary: string;
  description: string;
  tags: string[];
  accent: string;
  surface: string;
  poster: string;
  preview_video?: string;
  preview_animated?: string;
  upstream_video?: string;
  upstream_animated?: string;
  media_type: "video" | "animated" | "image";
  has_audio?: boolean;
  targets: LockscreenTargetType[];
  entrypoint: string;
  runtime_assets: string[];
  bg_video_path?: string;
  fonts?: { name: string; filename: string; bundled: boolean }[];
  requires_multimedia?: boolean;
  upstream_repo?: string;
  upstream_revision?: string;
  license?: string;
  config_schema?: LockscreenConfigSchema;
}

export interface TargetResolutionResult {
  can_install: boolean;
  requires_root: boolean;
  privilege_notice: string | null;
  target_files: { source: string; target: string; is_system: boolean }[];
  config_files: { path: string; content: string; is_system: boolean }[];
  missing_dependencies: string[];
  supported_dependencies: string[];
  warnings: string[];
  session_lock_supported: boolean;
  login_screen_supported: boolean;
}

/**
 * Normalizes an upstream Qylock catalogue definition into Ryzora's canonical PackageItem.
 * Strictly separates Preview Media from Runtime Assets.
 */
export function normalizeQylockTheme(raw: RawQylockTheme): PackageItem {
  const isVideo = raw.media_type === "video";

  const manifest: LockscreenManifest = {
    provider: "qylock",
    targets: {},
    media: {
      poster: raw.poster,
      preview_video: raw.preview_video,
      preview_animated: raw.preview_animated,
      upstream_video: raw.upstream_video,
      upstream_animated: raw.upstream_animated,
      has_audio: raw.has_audio || false,
      aspect_ratio: "16:9",
    },
    runtime: {
      entrypoint: raw.entrypoint,
      assets: raw.runtime_assets,
      bg_video_path: raw.bg_video_path,
      fonts: raw.fonts,
      requires_multimedia: raw.requires_multimedia ?? isVideo,
    },
    provenance: {
      provider_name: "qylock",
      upstream_repo: raw.upstream_repo || "https://github.com/Darkkal44/qylock",
      upstream_revision: raw.upstream_revision || "main",
      author: raw.author,
      license: raw.license || "GPL-3.0",
    },
    config_schema: raw.config_schema,
  };

  const multimediaDeps = (raw.requires_multimedia || isVideo)
    ? ["qt6-multimedia", "gst-plugins-good"]
    : [];

  // Quickshell Session Lock Target
  if (raw.targets.includes("quickshell")) {
    manifest.targets.quickshell = {
      id: "quickshell",
      name: "Session Lock (Quickshell)",
      scope: "user",
      supported: true,
      entrypoint: raw.entrypoint,
      dependencies: ["quickshell", "qt6-declarative", ...multimediaDeps],
      requires_session_lock: true,
      requires_root: false,
      install_destination: `~/.local/share/ryzora/lockscreens/qylock/${raw.id}`,
      shortcut_target: `~/.local/share/ryzora/integrations/quickshell/lock.sh`,
    };
  }

  // SDDM Login Greeter Target
  if (raw.targets.includes("sddm")) {
    manifest.targets.sddm = {
      id: "sddm",
      name: "Login Screen (SDDM)",
      scope: "system",
      supported: true,
      entrypoint: raw.entrypoint,
      dependencies: ["sddm", "qt6-declarative", "qt6-svg", ...multimediaDeps],
      requires_session_lock: false,
      requires_root: true,
      install_destination: `/usr/share/sddm/themes/ryzora-${raw.id}`,
      config_file: `/etc/sddm.conf.d/ryzora-theme.conf`,
    };
  }

  return {
    id: raw.id,
    title: raw.name,
    subtitle: `Qylock · ${raw.author}`,
    description: raw.description,
    version: raw.version,
    author: {
      name: raw.author,
      avatar: "https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=100&auto=format&fit=crop&q=80",
      verified: true,
    },
    category: "lockscreens",
    package_type: "lockscreen",
    tags: raw.tags,
    supported_desktops: ["hyprland", "sway", "cosmic", "universal"],
    supported_display: ["wayland"],
    rating: 4.9,
    rating_count: 84,
    downloads: 14200,
    hero_image: raw.poster,
    screenshots: [raw.poster],
    media_type: raw.media_type,
    preview_video_url: raw.preview_video,
    preview_poster_url: raw.poster,
    supports_session_lock: raw.targets.includes("quickshell"),
    supports_login_screen: raw.targets.includes("sddm"),
    lockscreen: manifest,
    customizable: Boolean(
      raw.config_schema &&
      ((raw.config_schema.variants && raw.config_schema.variants.length > 0) ||
       (raw.config_schema.options && Object.keys(raw.config_schema.options).length > 0))
    ),
    color_palette: [raw.accent, raw.surface, "#24283b", "#a9b1d6"],
    safety_audit: {
      rating: "verified",
      changes_system_files: raw.targets.includes("sddm"),
      requires_root: raw.targets.includes("sddm"),
      sandbox_compatible: true,
      files_modified_count: raw.runtime_assets.length + 3,
    },
    dependencies: {
      packages: raw.targets.includes("quickshell")
        ? ["quickshell", "qt6-declarative", ...multimediaDeps]
        : ["sddm", "qt6-declarative", "qt6-svg", ...multimediaDeps],
      optional: ["fzf"],
    },
    components: raw.runtime_assets.map((asset) => ({
      name: asset,
      component_type: "asset",
      target_path: asset,
      description: `Runtime theme asset: ${asset}`,
    })),
    compatibility: {
      supported_distros: ["all"],
      supported_desktops: ["hyprland", "sway", "cosmic", "universal"],
      supported_sessions: ["wayland"],
      required_binaries: raw.targets.includes("quickshell") ? ["quickshell"] : ["sddm"],
      optional_binaries: ["fzf"],
    },
  };
}

/**
 * Capability resolution engine:
 * Interrogates compositor, display manager, and system protocols before allowing target choices.
 */
export function resolveLockscreenCapabilities(
  systemInfo: SystemInfo,
  manifest?: LockscreenManifest,
  selectedTarget: "quickshell" | "sddm" | "both" = "quickshell"
): TargetResolutionResult {
  const result: TargetResolutionResult = {
    can_install: true,
    requires_root: false,
    privilege_notice: null,
    target_files: [],
    config_files: [],
    missing_dependencies: [],
    supported_dependencies: [],
    warnings: [],
    session_lock_supported: true,
    login_screen_supported: true,
  };

  if (!manifest) {
    return result;
  }

  const desktop = systemInfo.desktop_environment?.toLowerCase() || "";
  const isKdePlasma = desktop.includes("kde") || desktop.includes("plasma");
  const isWayland = systemInfo.session_type === "wayland";

  // Protocol Check: Quickshell session lock requires ext-session-lock-v1
  // KWin (KDE Plasma) currently lacks ext-session-lock-v1 support
  if (isKdePlasma) {
    result.session_lock_supported = false;
    result.warnings.push(
      "ext-session-lock-v1 is unsupported on KDE Plasma/KWin. Quickshell session locking is disabled."
    );
  }

  if (!isWayland) {
    result.session_lock_supported = false;
    result.warnings.push("Quickshell lockscreen requires a Wayland session.");
  }

  const quickshellTarget = manifest?.targets?.quickshell;
  const sddmTarget = manifest?.targets?.sddm;

  const includeQuickshell =
    (selectedTarget === "quickshell" || selectedTarget === "both") && Boolean(quickshellTarget);
  const includeSddm =
    (selectedTarget === "sddm" || selectedTarget === "both") && Boolean(sddmTarget);

  if (includeQuickshell && quickshellTarget) {
    if (!result.session_lock_supported) {
      result.can_install = false;
    }
    quickshellTarget.dependencies.forEach((dep) => {
      if (!result.supported_dependencies.includes(dep)) {
        result.supported_dependencies.push(dep);
      }
    });

    result.target_files.push(
      {
        source: manifest.runtime.entrypoint,
        target: `${quickshellTarget.install_destination}/${manifest.runtime.entrypoint}`,
        is_system: false,
      },
      ...manifest.runtime.assets.map((asset) => ({
        source: asset,
        target: `${quickshellTarget.install_destination}/${asset}`,
        is_system: false,
      }))
    );

    result.config_files.push({
      path: quickshellTarget.shortcut_target || "~/.local/share/ryzora/integrations/quickshell/lock.sh",
      content: `exec quickshell -p ~/.local/share/ryzora/integrations/quickshell/lock_shell.qml`,
      is_system: false,
    });
  }

  if (includeSddm && sddmTarget) {
    result.requires_root = true;
    result.privilege_notice =
      "Administrator permissions (pkexec) required to copy themes to /usr/share/sddm/ and update /etc/sddm.conf.d/";

    sddmTarget.dependencies.forEach((dep) => {
      if (!result.supported_dependencies.includes(dep)) {
        result.supported_dependencies.push(dep);
      }
    });

    result.target_files.push(
      {
        source: manifest.runtime.entrypoint,
        target: `${sddmTarget.install_destination}/${manifest.runtime.entrypoint}`,
        is_system: true,
      },
      ...manifest.runtime.assets.map((asset) => ({
        source: asset,
        target: `${sddmTarget.install_destination}/${asset}`,
        is_system: true,
      }))
    );

    result.config_files.push({
      path: sddmTarget.config_file || "/etc/sddm.conf.d/ryzora-theme.conf",
      content: `[Theme]\nCurrent=${sddmTarget.install_destination.split("/").pop()}\n`,
      is_system: true,
    });
  }

  return result;
}

/**
 * Authentic catalogue items from Qylock repository
 */
export const RAW_QYLOCK_THEMES: RawQylockTheme[] = [
  {
    "id": "dog-samurai",
    "name": "Dog Samurai",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "A painterly samurai hound under drifting red petals.",
    "description": "A cinematic video lock: a straw-hatted samurai dog with a sheathed katana as crimson petals fall, paired with a bold digital clock and minimal login field.",
    "tags": [
      "animated",
      "cinematic",
      "quickshell",
      "qylock",
      "samurai",
      "sddm",
      "video"
    ],
    "accent": "#945a46",
    "surface": "#141612",
    "poster": "/assets/lockscreens/dog-samurai-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "Orbitron-VariableFont_wght.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/dog-samurai-preview.mp4",
    "preview_animated": "/assets/lockscreens/dog-samurai.gif",
    "bg_video_path": "bg.mp4",
    "fonts": [
      {
        "name": "Orbitron",
        "filename": "Orbitron.ttf",
        "bundled": true
      }
    ]
  },
  {
    "id": "clockwork-tape",
    "name": "Tape (Clockwork)",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Warm magnetic cassette tape with mechanical clockwork drift.",
    "description": "A tactile retro lock: deep warm amber tones, rotating magnetic reel animations, and an authentic cassette tape aesthetic.",
    "tags": [
      "animated",
      "clockwork",
      "quickshell",
      "qylock",
      "retro",
      "sddm",
      "warm"
    ],
    "accent": "#d7a45f",
    "surface": "#101010",
    "poster": "/assets/lockscreens/clockwork-poster.jpg",
    "media_type": "animated",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "metadata.desktop",
      "theme.conf",
      "Outfit-Black.ttf"
    ],
    "requires_multimedia": false,
    "preview_video": "/assets/lockscreens/clockwork-preview.mp4",
    "preview_animated": "/assets/lockscreens/clockwork.gif",
    "config_schema": {
      "variants": [
        {
          "id": "amber-default",
          "name": "Amber Reel (Default)",
          "description": "Deep warm amber tones with mechanical spinning tape reels.",
          "preview_image": "/assets/lockscreens/clockwork-poster.jpg"
        },
        {
          "id": "midnight-chrome",
          "name": "Midnight Chrome",
          "description": "Cool monochrome aluminum cassette shell with neon blue indicator.",
          "preview_image": "/assets/lockscreens/clockwork-poster.jpg"
        },
        {
          "id": "sepia-vintage",
          "name": "Sepia 1984",
          "description": "Aged parchment tones with subtle analog tape flutter and CRT scanlines.",
          "preview_image": "/assets/lockscreens/clockwork-poster.jpg"
        }
      ],
      "options": {
        "clockType": {
          "id": "clockType",
          "label": "Clock Type",
          "type": "select",
          "description": "Choose between numeric flip counter or mechanical analog dial.",
          "default": "digital",
          "options": [
            { "value": "digital", "label": "Digital Flip" },
            { "value": "analog", "label": "Analog Dial" }
          ]
        },
        "clockPosition": {
          "id": "clockPosition",
          "label": "Clock Position",
          "type": "select",
          "description": "Vertical alignment of the cassette HUD.",
          "default": "center",
          "options": [
            { "value": "center", "label": "Center Reel" },
            { "value": "top", "label": "Top Deck" },
            { "value": "bottom", "label": "Bottom Tray" }
          ]
        },
        "showSeconds": {
          "id": "showSeconds",
          "label": "Show Seconds Counter",
          "type": "boolean",
          "description": "Display tape counter elapsed seconds below the hours.",
          "default": true
        },
        "reelMotion": {
          "id": "reelMotion",
          "label": "Cassette Reel Motion",
          "type": "select",
          "description": "Rotational speed of the magnetic tape reels.",
          "default": "normal",
          "options": [
            { "value": "normal", "label": "Normal (33 RPM)" },
            { "value": "slow", "label": "Slow Lofi Drift" },
            { "value": "static", "label": "Static (Idle)" }
          ]
        }
      }
    }
  },
  {
    "id": "nier-automata",
    "name": "NieR: Automata",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Minimalist YoRHa military interface with authentic tactical glitch styling.",
    "description": "Subtle scanlines, muted beige military UI, and exact YoRHa command styling. Operates seamlessly as both a session lock and system greeter.",
    "tags": [
      "minimal",
      "nier",
      "quickshell",
      "qylock",
      "sddm",
      "yorha"
    ],
    "accent": "#c8b99d",
    "surface": "#1a1a18",
    "poster": "/assets/lockscreens/nier-poster.jpg",
    "media_type": "animated",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "NierButton.qml",
      "bg.png",
      "metadata.desktop",
      "theme.conf"
    ],
    "requires_multimedia": false,
    "preview_video": "/assets/lockscreens/nier-preview.mp4",
    "preview_animated": "/assets/lockscreens/nier-automata.gif"
  },
  {
    "id": "forest",
    "name": "Forest",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Misty pine canopy with gentle ambient particle motion in full 1080p.",
    "description": "High altitude alpine forest with subtle floating mist, dynamic clock, and a calm, distraction-free authentication prompt.",
    "tags": [
      "ambient",
      "animated",
      "forest",
      "nature",
      "quickshell",
      "qylock",
      "sddm"
    ],
    "accent": "#4a7051",
    "surface": "#0f1610",
    "poster": "/assets/lockscreens/forest-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "Figtree-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/forest-preview.mp4",
    "preview_animated": "/assets/lockscreens/forest.gif",
    "bg_video_path": "bg.mp4"
  },
  {
    "id": "material-you",
    "name": "Material You",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Dynamic Material You adaptive styling with pastel pill clock.",
    "description": "Clean modern lock inspired by Android 14 design guidelines. Adapts seamlessly across light and dark system color schemes.",
    "tags": [
      "material-you",
      "minimal",
      "modern",
      "quickshell",
      "qylock",
      "sddm"
    ],
    "accent": "#7aa2f7",
    "surface": "#1a1b26",
    "poster": "/assets/lockscreens/material-you-poster.jpg",
    "media_type": "animated",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.png",
      "metadata.desktop",
      "theme.conf",
      "GoogleSans-VariableFont_GRAD,opsz,wght.ttf"
    ],
    "requires_multimedia": false,
    "preview_video": "/assets/lockscreens/material-you-preview.mp4",
    "preview_animated": "/assets/lockscreens/material-you.gif"
  },
  {
    "id": "enfield",
    "name": "Arknights: Endfield",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Talos-II planetary frontier tactical lockscreen with industrial telemetry and HUD overlays.",
    "description": "Talos-II planetary frontier tactical lockscreen with industrial telemetry and HUD overlays.",
    "tags": [
      "arknights",
      "enfield",
      "industrial",
      "quickshell",
      "qylock",
      "sddm",
      "tactical"
    ],
    "accent": "#f59e0b",
    "surface": "#18181b",
    "poster": "/assets/lockscreens/enfield-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "Orbitron-VariableFont_wght.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/enfield-preview.mp4",
    "preview_animated": "/assets/lockscreens/enfield.gif"
  },
  {
    "id": "field",
    "name": "Sunlit Field",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Golden sunlight streaming across an open grassland hill under summer cumulus clouds.",
    "description": "Golden sunlight streaming across an open grassland hill under summer cumulus clouds.",
    "tags": [
      "field",
      "minimal",
      "nature",
      "quickshell",
      "qylock",
      "sddm",
      "summer"
    ],
    "accent": "#eab308",
    "surface": "#1c1917",
    "poster": "/assets/lockscreens/field-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.png",
      "metadata.desktop",
      "theme.conf",
      "Orbitron-VariableFont_wght.ttf"
    ],
    "requires_multimedia": false
  },
  {
    "id": "genshin",
    "name": "Genshin Impact",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Dynamic time-of-day Teyvat lockscreen with dawn, day, dusk, and night animated cycles.",
    "description": "Dynamic time-of-day Teyvat lockscreen with dawn, day, dusk, and night animated cycles.",
    "tags": [
      "anime",
      "day-night",
      "dynamic",
      "genshin",
      "quickshell",
      "qylock",
      "sddm"
    ],
    "accent": "#d4af37",
    "surface": "#1e293b",
    "poster": "/assets/lockscreens/genshin-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "GenshinButton.qml",
      "GenshinFrame.qml",
      "Main.qml",
      "dawn.mp4",
      "day.mp4",
      "dusk.mp4",
      "logo.png",
      "metadata.desktop",
      "night.mp4",
      "theme.conf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/genshin-preview.mp4",
    "preview_animated": "/assets/lockscreens/genshin.gif"
  },
  {
    "id": "girl-coffee",
    "name": "Morning Coffee",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Peaceful early morning coffee break overlooking a sleepy sunlit city.",
    "description": "Peaceful early morning coffee break overlooking a sleepy sunlit city.",
    "tags": [
      "anime",
      "coffee",
      "morning",
      "peaceful",
      "quickshell",
      "qylock",
      "sddm"
    ],
    "accent": "#f97316",
    "surface": "#1c1917",
    "poster": "/assets/lockscreens/girl-coffee-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.png",
      "metadata.desktop",
      "theme.conf",
      "Itim-Regular.ttf"
    ],
    "requires_multimedia": false
  },
  {
    "id": "last-of-us",
    "name": "The Last of Us",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Post-apocalyptic overgrown city ruins with drifting spores and acoustic guitar motif.",
    "description": "Post-apocalyptic overgrown city ruins with drifting spores and acoustic guitar motif.",
    "tags": [
      "atmospheric",
      "cinematic",
      "last-of-us",
      "overgrown",
      "quickshell",
      "qylock",
      "sddm"
    ],
    "accent": "#84cc16",
    "surface": "#141a10",
    "poster": "/assets/lockscreens/last-of-us-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "Outfit-Black.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/last-of-us-preview.mp4",
    "preview_animated": "/assets/lockscreens/last-of-us.gif"
  },
  {
    "id": "minecraft",
    "name": "Minecraft Panoramas",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Slowly revolving classic Minecraft landscape with voxel skies and blocky typography.",
    "description": "Slowly revolving classic Minecraft landscape with voxel skies and blocky typography.",
    "tags": [
      "classic",
      "gaming",
      "minecraft",
      "quickshell",
      "qylock",
      "sddm",
      "voxel"
    ],
    "accent": "#65a30d",
    "surface": "#1c1917",
    "poster": "/assets/lockscreens/minecraft-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "background.png",
      "metadata.desktop",
      "theme.conf",
      "title.png"
    ],
    "requires_multimedia": false,
    "config_schema": {
      "variants": [
        {
          "id": "overworld-sunrise",
          "name": "Overworld Sunrise",
          "description": "Warm golden morning glow across lush voxel plains and distant hills.",
          "preview_image": "/assets/lockscreens/minecraft-poster.jpg"
        },
        {
          "id": "nether-fortress",
          "name": "Nether Fortress",
          "description": "Dark basalt pillars, glowing magma falls, and atmospheric soul fire particles.",
          "preview_image": "/assets/lockscreens/minecraft-poster.jpg"
        },
        {
          "id": "the-end",
          "name": "The End",
          "description": "Eerie purple obsidian sky with rotating void panorama and floating islands.",
          "preview_image": "/assets/lockscreens/minecraft-poster.jpg"
        }
      ],
      "options": {
        "hudStyle": {
          "id": "hudStyle",
          "label": "HUD Interface Style",
          "type": "select",
          "description": "Texture theme for the password prompt and status meters.",
          "default": "classic",
          "options": [
            { "value": "classic", "label": "Classic 16px GUI" },
            { "value": "hardcore", "label": "Hardcore Red Hearts" },
            { "value": "minimal", "label": "Clean Minimal" }
          ]
        },
        "showClock": {
          "id": "showClock",
          "label": "Minecraft Clock Dial",
          "type": "boolean",
          "description": "Render spinning pixel sun/moon clock in the upper corner.",
          "default": true
        },
        "panoramaSpeed": {
          "id": "panoramaSpeed",
          "label": "Panorama Rotation",
          "type": "select",
          "description": "Orbital panning speed of the panoramic backdrop.",
          "default": "normal",
          "options": [
            { "value": "normal", "label": "Normal 1x" },
            { "value": "cinematic", "label": "Cinematic Slow" },
            { "value": "fast", "label": "Dynamic 2x" }
          ]
        },
        "renderCrosshair": {
          "id": "renderCrosshair",
          "label": "Render Aim Crosshair",
          "type": "boolean",
          "description": "Show the iconic + crosshair at the screen center.",
          "default": false
        }
      }
    }
  },
  {
    "id": "ninja-gaiden",
    "name": "Ninja Gaiden Moon",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Dark silhouette of Ryu Hayabusa against an ominous blood-red full moon.",
    "description": "Dark silhouette of Ryu Hayabusa against an ominous blood-red full moon.",
    "tags": [
      "action",
      "blood-moon",
      "ninja",
      "quickshell",
      "qylock",
      "retro",
      "sddm"
    ],
    "accent": "#ef4444",
    "surface": "#18181b",
    "poster": "/assets/lockscreens/ninja-gaiden-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.png",
      "metadata.desktop",
      "theme.conf",
      "Tektur-VariableFont_wdth,wght.ttf"
    ],
    "requires_multimedia": false
  },
  {
    "id": "osu",
    "name": "osu! Beatmap Lock",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Dynamic rhythm game themed lockscreen with pink accent glows and combo counter aesthetic.",
    "description": "Dynamic rhythm game themed lockscreen with pink accent glows and combo counter aesthetic.",
    "tags": [
      "anime",
      "gaming",
      "osu",
      "quickshell",
      "qylock",
      "rhythm",
      "sddm"
    ],
    "accent": "#f43f5e",
    "surface": "#1e1b4b",
    "poster": "/assets/lockscreens/osu-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "metadata.desktop",
      "theme.conf",
      "achlys.png",
      "darkkal.png",
      "kaizky.png",
      "A Glow.jpg",
      "B Glow.jpg",
      "C Glow.jpg",
      "D Glow.jpg",
      "E Glow.jpg",
      "F Glow.jpg",
      "G Glow.jpg",
      "Torus Regular.otf"
    ],
    "requires_multimedia": false,
    "config_schema": {
      "variants": [
        {
          "id": "standard-pink",
          "name": "Standard Glow",
          "description": "Neon pink combo rings and smooth audio reactive pulses.",
          "preview_image": "/assets/lockscreens/osu-poster.jpg"
        },
        {
          "id": "mania-neon",
          "name": "Mania Cyber",
          "description": "Electric cyan speed indicators and key lane gradients.",
          "preview_image": "/assets/lockscreens/osu-poster.jpg"
        },
        {
          "id": "taiko-drum",
          "name": "Taiko Festival",
          "description": "Vibrant red rhythm markers and festive Japanese aesthetics.",
          "preview_image": "/assets/lockscreens/osu-poster.jpg"
        }
      ],
      "options": {
        "audioVisualizer": {
          "id": "audioVisualizer",
          "label": "Music Visualizer Pulse",
          "type": "boolean",
          "description": "Smooth audio reactive waveform pulses behind the lockscreen avatar.",
          "default": true
        },
        "showAccuracyMeter": {
          "id": "showAccuracyMeter",
          "label": "Show Accuracy Rating",
          "type": "boolean",
          "description": "Display SS/S/A grade indicator next to battery status.",
          "default": true
        },
        "glowIntensity": {
          "id": "glowIntensity",
          "label": "Glow Brightness",
          "type": "number",
          "description": "Brightness of the radial accent halo.",
          "default": 75,
          "min": 20,
          "max": 100,
          "step": 5,
          "unit": "%"
        }
      }
    }
  },
  {
    "id": "pixel-coffee",
    "name": "Pixel Coffee Shop",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Lo-fi pixel cafe window looking out at gentle autumn drizzle with steamy warm mugs.",
    "description": "Lo-fi pixel cafe window looking out at gentle autumn drizzle with steamy warm mugs.",
    "tags": [
      "coffee",
      "lofi",
      "pixel",
      "quickshell",
      "qylock",
      "relaxing",
      "sddm"
    ],
    "accent": "#b45309",
    "surface": "#29180c",
    "poster": "/assets/lockscreens/pixel-coffee-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-coffee-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-coffee.gif",
    "config_schema": {
      "variants": [
        {
          "id": "cozy-dusk",
          "name": "Cozy Dusk (Default)",
          "description": "Warm amber streetlights reflecting on wet pavement outside the glass.",
          "preview_image": "/assets/lockscreens/pixel-coffee-poster.jpg"
        },
        {
          "id": "midnight-rain",
          "name": "Midnight Rain",
          "description": "Deep indigo hues with tranquil rain streaks and dim vintage pendant lamps.",
          "preview_image": "/assets/lockscreens/pixel-coffee-poster.jpg"
        }
      ],
      "options": {
        "clockStyle": {
          "id": "clockStyle",
          "label": "Pixel Clock Font",
          "type": "select",
          "description": "Font family used for the time and date readout.",
          "default": "pixel-bold",
          "options": [
            { "value": "pixel-bold", "label": "Pixelify Sans" },
            { "value": "retro-lcd", "label": "Retro 7-Segment" },
            { "value": "minimal", "label": "Clean Sans" }
          ]
        },
        "clockPosition": {
          "id": "clockPosition",
          "label": "Clock Position",
          "type": "select",
          "description": "Screen alignment of the clock HUD.",
          "default": "top-left",
          "options": [
            { "value": "top-left", "label": "Top Left" },
            { "value": "center", "label": "Center Screen" },
            { "value": "top-right", "label": "Top Right" }
          ]
        },
        "showDate": {
          "id": "showDate",
          "label": "Display Date Readout",
          "type": "boolean",
          "description": "Show day of week and date string alongside the time.",
          "default": true
        },
        "weatherEffects": {
          "id": "weatherEffects",
          "label": "Window Rain Streaks",
          "type": "boolean",
          "description": "Simulate animated condensation and raindrops on the glass pane.",
          "default": true
        }
      }
    }
  },
  {
    "id": "pixel-cyberpunk",
    "name": "Pixel Cyberpunk",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Rain-slicked neon alley with holograms and retro 16-bit cyber aesthetics.",
    "description": "Rain-slicked neon alley with holograms and retro 16-bit cyber aesthetics.",
    "tags": [
      "cyberpunk",
      "neon",
      "pixel",
      "quickshell",
      "qylock",
      "retro",
      "sddm"
    ],
    "accent": "#ec4899",
    "surface": "#0f0f1a",
    "poster": "/assets/lockscreens/pixel-cyberpunk-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-cyberpunk-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-cyberpunk.gif"
  },
  {
    "id": "pixel-emerald",
    "name": "Pixel Emerald Valley",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Serene emerald mountain valley with flying airships and vibrant 16-bit pixel panorama.",
    "description": "Serene emerald mountain valley with flying airships and vibrant 16-bit pixel panorama.",
    "tags": [
      "emerald",
      "nature",
      "pixel",
      "quickshell",
      "qylock",
      "retro",
      "sddm"
    ],
    "accent": "#10b981",
    "surface": "#064e3b",
    "poster": "/assets/lockscreens/pixel-emerald-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-emerald-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-emerald.gif"
  },
  {
    "id": "pixel-rainyroom",
    "name": "Pixel Rainy Room",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Midnight room lit by neon monitors as soothing rain taps on the glass panes.",
    "description": "Midnight room lit by neon monitors as soothing rain taps on the glass panes.",
    "tags": [
      "cozy",
      "lofi",
      "pixel",
      "quickshell",
      "qylock",
      "rain",
      "sddm"
    ],
    "accent": "#6366f1",
    "surface": "#0b0f19",
    "poster": "/assets/lockscreens/pixel-rainyroom-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-rainyroom-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-rainyroom.gif"
  },
  {
    "id": "pixel-sakura",
    "name": "Pixel Sakura Blossom",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Gentle cherry blossom petals drifting over a quiet twilight village shrine.",
    "description": "Gentle cherry blossom petals drifting over a quiet twilight village shrine.",
    "tags": [
      "cherry-blossom",
      "japan",
      "pixel",
      "quickshell",
      "qylock",
      "sakura",
      "sddm"
    ],
    "accent": "#f472b6",
    "surface": "#1e1024",
    "poster": "/assets/lockscreens/pixel-sakura-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-sakura-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-sakura.gif"
  },
  {
    "id": "pixel-waterfall",
    "name": "Pixel Mountain Waterfall",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Towering cascade falling into crystal mountain pools with shimmering pixel water shaders.",
    "description": "Towering cascade falling into crystal mountain pools with shimmering pixel water shaders.",
    "tags": [
      "mountain",
      "pixel",
      "quickshell",
      "qylock",
      "relaxing",
      "sddm",
      "waterfall"
    ],
    "accent": "#0284c7",
    "surface": "#082f49",
    "poster": "/assets/lockscreens/pixel-waterfall-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "PixelifySans-Bold.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/pixel-waterfall-preview.mp4",
    "preview_animated": "/assets/lockscreens/pixel-waterfall.gif"
  },
  {
    "id": "star-rail",
    "name": "Honkai: Star Rail",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Astral Express cosmic voyage video lockscreen with futuristic rail clock and starry atmosphere.",
    "description": "Astral Express cosmic voyage video lockscreen with futuristic rail clock and starry atmosphere.",
    "tags": [
      "cosmic",
      "honkai",
      "quickshell",
      "qylock",
      "sddm",
      "space",
      "star-rail"
    ],
    "accent": "#818cf8",
    "surface": "#0f172a",
    "poster": "/assets/lockscreens/star-rail-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/star-rail-preview.mp4",
    "preview_animated": "/assets/lockscreens/star-rail.gif"
  },
  {
    "id": "sword",
    "name": "Sword in the Stone",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Atmospheric pixel fantasy lockscreen of a legendary blade resting in an enchanted glade.",
    "description": "Atmospheric pixel fantasy lockscreen of a legendary blade resting in an enchanted glade.",
    "tags": [
      "animated",
      "fantasy",
      "pixel",
      "quickshell",
      "qylock",
      "sddm",
      "sword"
    ],
    "accent": "#a855f7",
    "surface": "#1e1b4b",
    "poster": "/assets/lockscreens/sword-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "BackgroundVideo.qml",
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "The Last Shuriken.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/sword-preview.mp4",
    "preview_animated": "/assets/lockscreens/sword.gif"
  },
  {
    "id": "terraria",
    "name": "Terraria Overworld",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Iconic Terraria surface biome with floating day slime, floating islands, and retro pixel font.",
    "description": "Iconic Terraria surface biome with floating day slime, floating islands, and retro pixel font.",
    "tags": [
      "gaming",
      "pixel",
      "quickshell",
      "qylock",
      "sandbox",
      "sddm",
      "terraria"
    ],
    "accent": "#22c55e",
    "surface": "#14532d",
    "poster": "/assets/lockscreens/terraria-poster.jpg",
    "media_type": "animated",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "TerraButton.qml",
      "avatar.png",
      "metadata.desktop",
      "ter1.png",
      "ter2.png",
      "ter3.png",
      "ter4.png",
      "ter5.png",
      "terraria_logo.png",
      "theme.conf"
    ],
    "requires_multimedia": false,
    "preview_video": "/assets/lockscreens/terraria-preview.mp4",
    "preview_animated": "/assets/lockscreens/terraria.gif"
  },
  {
    "id": "windows-7",
    "name": "Windows 7 Aero Bliss",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Nostalgic translucent Aero Glass logon screen with authentic Orb button and blue aurora.",
    "description": "Nostalgic translucent Aero Glass logon screen with authentic Orb button and blue aurora.",
    "tags": [
      "aero",
      "glass",
      "quickshell",
      "qylock",
      "retro",
      "sddm",
      "windows-7"
    ],
    "accent": "#0284c7",
    "surface": "#0c4a6e",
    "poster": "/assets/lockscreens/windows-7-poster.jpg",
    "media_type": "image",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "background.png",
      "metadata.desktop",
      "pfp.png",
      "theme.conf"
    ],
    "requires_multimedia": false
  },
  {
    "id": "winter",
    "name": "Winter Snowfall",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Cozy silent winter snowfall with frosty pine silhouettes and ambient digital clock.",
    "description": "Cozy silent winter snowfall with frosty pine silhouettes and ambient digital clock.",
    "tags": [
      "ambient",
      "cozy",
      "quickshell",
      "qylock",
      "sddm",
      "snow",
      "winter"
    ],
    "accent": "#38bdf8",
    "surface": "#0f172a",
    "poster": "/assets/lockscreens/winter-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "metadata.desktop",
      "theme.conf",
      "Orbitron-VariableFont_wght.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/winter-preview.mp4",
    "preview_animated": "/assets/lockscreens/winter.gif"
  },
  {
    "id": "wuwa",
    "name": "Wuthering Waves",
    "version": "1.0.0",
    "author": "Darkkal44",
    "summary": "Rover cinematic lockscreen with animated resonance wave effects and dark sci-fi HUD.",
    "description": "Rover cinematic lockscreen with animated resonance wave effects and dark sci-fi HUD.",
    "tags": [
      "cinematic",
      "quickshell",
      "qylock",
      "sci-fi",
      "sddm",
      "wuthering-waves",
      "wuwa"
    ],
    "accent": "#06b6d4",
    "surface": "#0f172a",
    "poster": "/assets/lockscreens/wuwa-poster.jpg",
    "media_type": "video",
    "has_audio": false,
    "targets": [
      "quickshell",
      "sddm"
    ],
    "entrypoint": "Main.qml",
    "runtime_assets": [
      "Main.qml",
      "bg.mp4",
      "logo.png",
      "metadata.desktop",
      "theme.conf",
      "Orbitron-VariableFont_wght.ttf"
    ],
    "requires_multimedia": true,
    "preview_video": "/assets/lockscreens/wuwa-preview.mp4",
    "preview_animated": "/assets/lockscreens/wuwa.gif"
  }
];

export function getCatalogueLockScreens(): PackageItem[] {
  const normalizedQylock = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);

  // Native Hyprlock session lock
  const auroraHyprlock: PackageItem = {
    id: "aurora-hyprlock",
    title: "Aurora Glass Hyprlock",
    subtitle: "Hyprlock · WaylandEnthusiast",
    description: "Frosted translucent lockscreen config for Hyprlock with GPU shaders and pam auth.",
    version: "1.2.0",
    author: {
      name: "WaylandEnthusiast",
      avatar: "https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=100&auto=format&fit=crop&q=80",
      verified: true,
    },
    category: "lockscreens",
    package_type: "lockscreen",
    tags: ["hyprlock", "hyprland", "glass", "minimal"],
    supported_desktops: ["hyprland"],
    supported_display: ["wayland"],
    rating: 4.8,
    rating_count: 56,
    downloads: 8200,
    hero_image: "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=800&auto=format&fit=crop&q=80",
    screenshots: ["https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=800&auto=format&fit=crop&q=80"],
    media_type: "image",
    preview_poster_url: "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=800&auto=format&fit=crop&q=80",
    supports_session_lock: true,
    supports_login_screen: false,
    lockscreen: {
      provider: "hyprlock",
      targets: {
        hyprlock: {
          id: "hyprlock",
          name: "Session Lock (Hyprlock)",
          scope: "user",
          supported: true,
          entrypoint: "hyprlock.conf",
          dependencies: ["hyprlock", "hyprland"],
          requires_session_lock: true,
          requires_root: false,
          install_destination: "~/.config/hypr/hyprlock.conf",
        },
      },
      media: {
        poster: "https://images.unsplash.com/photo-1518709268805-4e9042af9f23?w=800&auto=format&fit=crop&q=80",
      },
      runtime: {
        entrypoint: "hyprlock.conf",
        assets: ["hyprlock.conf", "shaders/blur.frag"],
      },
      provenance: {
        provider_name: "hyprlock-community",
        author: "WaylandEnthusiast",
        license: "MIT",
      },
    },
    color_palette: ["#88c0d0", "#2e3440", "#4c566a"],
    safety_audit: {
      rating: "safe",
      changes_system_files: false,
      requires_root: false,
      sandbox_compatible: true,
      files_modified_count: 2,
    },
    dependencies: {
      packages: ["hyprlock"],
      optional: [],
    },
    components: [
      {
        name: "hyprlock.conf",
        component_type: "config",
        target_path: "~/.config/hypr/hyprlock.conf",
        description: "Hyprlock configuration",
      },
    ],
    compatibility: {
      supported_distros: ["all"],
      supported_desktops: ["hyprland"],
      supported_sessions: ["wayland"],
      required_binaries: ["hyprlock"],
      optional_binaries: [],
    },
  };

  // Native Swaylock session lock
  const swaylockBlur: PackageItem = {
    id: "swaylock-blur",
    title: "Swaylock Effects Blur",
    subtitle: "Swaylock · Mortie",
    description: "Compact, efficient swaylock configuration with screenshot blurring and ring indicator.",
    version: "1.0.0",
    author: {
      name: "Mortie",
      avatar: "https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=100&auto=format&fit=crop&q=80",
      verified: true,
    },
    category: "lockscreens",
    package_type: "lockscreen",
    tags: ["swaylock", "sway", "blur", "minimal"],
    supported_desktops: ["sway", "hyprland"],
    supported_display: ["wayland"],
    rating: 4.6,
    rating_count: 29,
    downloads: 4100,
    hero_image: "https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800&auto=format&fit=crop&q=80",
    screenshots: ["https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800&auto=format&fit=crop&q=80"],
    media_type: "image",
    preview_poster_url: "https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800&auto=format&fit=crop&q=80",
    supports_session_lock: true,
    supports_login_screen: false,
    lockscreen: {
      provider: "swaylock",
      targets: {
        swaylock: {
          id: "swaylock",
          name: "Session Lock (Swaylock)",
          scope: "user",
          supported: true,
          entrypoint: "config",
          dependencies: ["swaylock-effects"],
          requires_session_lock: true,
          requires_root: false,
          install_destination: "~/.config/swaylock/config",
        },
      },
      media: {
        poster: "https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800&auto=format&fit=crop&q=80",
      },
      runtime: {
        entrypoint: "config",
        assets: ["config"],
      },
      provenance: {
        provider_name: "swaylock-community",
        author: "Mortie",
        license: "MIT",
      },
    },
    color_palette: ["#bf616a", "#2e3440", "#d8dee9"],
    safety_audit: {
      rating: "safe",
      changes_system_files: false,
      requires_root: false,
      sandbox_compatible: true,
      files_modified_count: 1,
    },
    dependencies: {
      packages: ["swaylock-effects"],
      optional: [],
    },
    components: [
      {
        name: "config",
        component_type: "config",
        target_path: "~/.config/swaylock/config",
        description: "Swaylock config file",
      },
    ],
    compatibility: {
      supported_distros: ["all"],
      supported_desktops: ["sway", "hyprland"],
      supported_sessions: ["wayland"],
      required_binaries: ["swaylock-effects"],
      optional_binaries: [],
    },
  };

  return [...normalizedQylock, auroraHyprlock, swaylockBlur];
}


/**
 * Pluggable LockscreenProvider implementation for Qylock themes.
 */
export const qylockLockscreenProvider: LockscreenProvider = {
  id: "qylock",
  name: "Qylock Upstream Provider",
  discover: () => RAW_QYLOCK_THEMES.map(normalizeQylockTheme),
  normalize: (raw: any) => normalizeQylockTheme(raw),
  getPreview: (pkg: PackageItem): LockscreenMediaSpec => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
    preview_video: pkg.preview_video_url,
    preview_animated: pkg.lockscreen?.media.preview_animated,
    upstream_video: pkg.lockscreen?.media.upstream_video,
    upstream_animated: pkg.lockscreen?.media.upstream_animated,
  }),
  getSource: (pkg: PackageItem): LockscreenSourceSpec => ({
    type: "git",
    repository: "https://github.com/Darkkal44/qylock",
    revision: "main",
    path: "themes/" + pkg.id.replace("lockscreen-qylock-", ""),
  }),
  getTargets: (pkg: PackageItem): LockscreenTargetsSpec => ({
    quickshell: pkg.supports_session_lock ?? true,
    sddm: pkg.supports_login_screen ?? true,
  }),
  getDependencies: (_pkg: PackageItem, target: string): string[] => {
    if (target === "sddm") return ["sddm", "qt6-declarative", "qt6-svg"];
    if (target === "quickshell") return ["quickshell", "qt6-multimedia"];
    return ["quickshell", "sddm", "qt6-multimedia"];
  },
  getProvenance: (pkg: PackageItem): LockscreenProvenance => ({
    upstream: "https://github.com/Darkkal44/qylock",
    revision: "main",
    license: "GPL-3.0",
    path: "themes/" + pkg.id.replace("lockscreen-qylock-", ""),
  }),
};
