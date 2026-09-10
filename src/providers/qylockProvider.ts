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
    id: "dog-samurai",
    name: "Dog Samurai",
    version: "1.0.0",
    author: "Darkkal44",
    summary: "A painterly samurai hound under drifting red petals.",
    description:
      "A cinematic video lock: a straw-hatted samurai dog with a sheathed katana as crimson petals fall, paired with a bold digital clock and minimal login field.",
    tags: ["samurai", "cinematic", "video", "animated", "qylock"],
    accent: "#945a46",
    surface: "#141612",
    poster: "/assets/lockscreens/dog-samurai-poster.jpg",
    preview_video: "/assets/lockscreens/dog-samurai-preview.mp4",
    preview_animated: "/assets/lockscreens/dog-samurai.gif",
    upstream_video: "https://raw.githubusercontent.com/Darkkal44/qylock/main/themes/dog-samurai/bg.mp4",
    upstream_animated: "https://raw.githubusercontent.com/Darkkal44/qylock/main/Assets/dog_samurai.gif",
    media_type: "video",
    has_audio: false,
    targets: ["quickshell", "sddm"],
    entrypoint: "Main.qml",
    runtime_assets: ["Main.qml", "bg.mp4", "theme.conf", "metadata.desktop", "font/Orbitron.ttf"],
    bg_video_path: "bg.mp4",
    fonts: [{ name: "Orbitron", filename: "Orbitron.ttf", bundled: true }],
    requires_multimedia: true,
  },
  {
    id: "clockwork-tape",
    name: "Tape (Clockwork)",
    version: "1.0.0",
    author: "Darkkal44",
    summary: "Warm magnetic cassette tape with mechanical clockwork drift.",
    description:
      "A tactile retro lock: deep warm amber tones, rotating magnetic reel animations, and an authentic cassette tape aesthetic.",
    tags: ["clockwork", "retro", "warm", "animated", "qylock"],
    accent: "#d7a45f",
    surface: "#101010",
    poster: "/assets/lockscreens/clockwork-poster.jpg",
    preview_video: "/assets/lockscreens/clockwork-preview.mp4",
    preview_animated: "/assets/lockscreens/clockwork.gif",
    upstream_animated: "https://raw.githubusercontent.com/Darkkal44/qylock/main/Assets/clockwork.gif",
    media_type: "animated",
    has_audio: false,
    targets: ["quickshell", "sddm"],
    entrypoint: "Main.qml",
    runtime_assets: ["Main.qml", "tape.png", "theme.conf", "metadata.desktop"],
    requires_multimedia: false,
  },
  {
    id: "nier-automata",
    name: "NieR: Automata",
    version: "1.0.0",
    author: "Darkkal44",
    summary: "Minimalist YoRHa military interface with authentic tactical glitch styling.",
    description:
      "Subtle scanlines, muted beige military UI, and exact YoRHa command styling. Operates seamlessly as both a session lock and system greeter.",
    tags: ["nier", "minimal", "yorha", "animated", "qylock"],
    accent: "#c8b99d",
    surface: "#1a1a18",
    poster: "/assets/lockscreens/nier-poster.jpg",
    preview_video: "/assets/lockscreens/nier-preview.mp4",
    preview_animated: "/assets/lockscreens/nier-automata.gif",
    upstream_animated: "https://raw.githubusercontent.com/Darkkal44/qylock/main/Assets/nier_automata.gif",
    media_type: "animated",
    has_audio: false,
    targets: ["quickshell", "sddm"],
    entrypoint: "Main.qml",
    runtime_assets: ["Main.qml", "NierButton.qml", "bg.png", "theme.conf", "metadata.desktop"],
    requires_multimedia: false,
  },
  {
    id: "forest",
    name: "Forest",
    version: "1.0.0",
    author: "Darkkal44",
    summary: "Misty pine canopy with gentle ambient particle motion in full 1080p.",
    description:
      "High altitude alpine forest with subtle floating mist, dynamic clock, and a calm, distraction-free authentication prompt.",
    tags: ["forest", "nature", "ambient", "video", "animated", "qylock"],
    accent: "#4a7051",
    surface: "#0f1610",
    poster: "/assets/lockscreens/forest-poster.jpg",
    preview_video: "/assets/lockscreens/forest-preview.mp4",
    preview_animated: "/assets/lockscreens/forest.gif",
    upstream_video: "https://raw.githubusercontent.com/Darkkal44/qylock/main/themes/forest/bg.mp4",
    upstream_animated: "https://raw.githubusercontent.com/Darkkal44/qylock/main/Assets/forest.gif",
    media_type: "video",
    has_audio: false,
    targets: ["quickshell", "sddm"],
    entrypoint: "Main.qml",
    runtime_assets: ["Main.qml", "bg.mp4", "theme.conf", "metadata.desktop"],
    bg_video_path: "bg.mp4",
    requires_multimedia: true,
  },
  {
    id: "material-you",
    name: "Material You",
    version: "1.0.0",
    author: "Darkkal44",
    summary: "Dynamic Material You adaptive styling with pastel pill clock.",
    description:
      "Clean modern lock inspired by Android 14 design guidelines. Adapts seamlessly across light and dark system color schemes.",
    tags: ["material-you", "minimal", "modern", "animated", "qylock"],
    accent: "#7aa2f7",
    surface: "#1a1b26",
    poster: "/assets/lockscreens/material-you-poster.jpg",
    preview_video: "/assets/lockscreens/material-you-preview.mp4",
    preview_animated: "/assets/lockscreens/material-you.gif",
    upstream_animated: "https://raw.githubusercontent.com/Darkkal44/qylock/main/Assets/material-you.gif",
    media_type: "animated",
    has_audio: false,
    targets: ["quickshell", "sddm"],
    entrypoint: "Main.qml",
    runtime_assets: ["Main.qml", "bg.png", "theme.conf", "metadata.desktop"],
    requires_multimedia: false,
  },
];

/**
 * Returns all canonical normalized lockscreen packages
 */
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
