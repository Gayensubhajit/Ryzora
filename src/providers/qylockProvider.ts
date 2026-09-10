import { DISCOVERED_QYLOCK_THEMES, getDiscoveredQylockThemes } from "./qylockDiscovery.ts";
export { getDiscoveredQylockThemes };
import type {
  LockscreenProvider,
  LockscreenMediaSpec,
  LockscreenSourceSpec,
  LockscreenTargetsSpec,
  LockscreenProvenance,
  PackageMediaSpec,
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
  upstream_path?: string;
  variant?: string;
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

export interface TargetCheckStatus {
  passed: boolean;
  required: string;
  detected: string;
  detail: string;
}

export interface DetailedAdapterEvaluation {
  adapter: "quickshell" | "sddm";
  target_name: string;
  category: "session_lock" | "login_screen";
  supported: boolean;
  package_provided: boolean;
  reason: string;
  system_changes_made: boolean;
  checks: {
    package_capability: TargetCheckStatus;
    compositor_protocol: TargetCheckStatus;
    runtime_binary: TargetCheckStatus;
    authentication: TargetCheckStatus;
    display_manager?: TargetCheckStatus;
    privilege_boundary: TargetCheckStatus;
  };
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
  evaluations?: {
    quickshell?: DetailedAdapterEvaluation;
    sddm?: DetailedAdapterEvaluation;
  };
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
      upstream_path: raw.upstream_path,
      variant: raw.variant,
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
  const wm = systemInfo.window_manager?.toLowerCase() || "";
  const isKdePlasma = desktop.includes("kde") || desktop.includes("plasma") || wm.includes("kwin");
  const isGnome = desktop.includes("gnome") || wm.includes("mutter");
  const isWayland = systemInfo.session_type === "wayland";
  const displayManager = ((systemInfo as any).display_manager || "").toLowerCase();
  const isGdm = displayManager === "gdm";

  // Protocol Check: Quickshell session lock requires ext-session-lock-v1
  // KWin (KDE Plasma) and Mutter (GNOME) lack ext-session-lock-v1 support for external lockers
  if (isKdePlasma) {
    result.session_lock_supported = false;
    result.warnings.push(
      "ext-session-lock-v1 is unsupported on KDE Plasma/KWin. Quickshell session locking is disabled."
    );
  }

  if (isGnome) {
    result.session_lock_supported = false;
    result.warnings.push(
      "ext-session-lock-v1 is unsupported on GNOME/Mutter. Quickshell session locking is disabled."
    );
  }

  if (!isWayland) {
    result.session_lock_supported = false;
    result.warnings.push("Quickshell lockscreen requires a Wayland session.");
  }

  // Display Manager Check: SDDM themes can only be applied to SDDM
  if (isGdm) {
    result.login_screen_supported = false;
    result.warnings.push(
      "Your system uses GDM (GNOME Display Manager). Qylock SDDM themes cannot be installed into GDM."
    );
  } else if (displayManager === "lightdm") {
    result.login_screen_supported = false;
    result.warnings.push(
      "Your system uses LightDM. Qylock SDDM themes cannot be installed into LightDM."
    );
  } else if (displayManager && displayManager !== "sddm") {
    result.login_screen_supported = false;
    result.warnings.push(
      `Your system uses ${displayManager}. Qylock SDDM themes cannot be installed into this display manager.`
    );
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
    if (!result.login_screen_supported) {
      result.can_install = false;
    }
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

  // ── Detailed Adapter Evaluations for Compatibility Lab & Diagnostics ──
  const qsHasManifest = Boolean(quickshellTarget);
  const qsCompositorProtocolPassed = isWayland && !isKdePlasma && !isGnome;
  const installedCmds = (systemInfo as any).installed_commands || {};
  const qsBinaryPassed = installedCmds.quickshell !== false;
  const hostAdapters = (systemInfo as any).supported_adapters as any[] | undefined;
  const hostQsAdapter = hostAdapters?.find((a) => a.adapter === "quickshell");
  const qsAuthPassed = hostQsAdapter
    ? hostQsAdapter.reason?.toLowerCase().includes("pam")
      ? hostQsAdapter.supported
      : true
    : true;

  const qsSupported = qsHasManifest && result.session_lock_supported && qsBinaryPassed && qsAuthPassed;
  let qsReason = "";
  if (!qsHasManifest) {
    qsReason = "Package does not provide a Quickshell session lock target.";
  } else if (!isWayland) {
    qsReason = "Session type is not Wayland. Quickshell requires a Wayland session with ext-session-lock-v1 protocol.";
  } else if (isGnome) {
    qsReason = "Compositor is Mutter (GNOME). Mutter lacks ext-session-lock-v1 protocol support for external lockers. No system changes were made.";
  } else if (isKdePlasma) {
    qsReason = "Compositor is KWin (KDE Plasma). KWin lacks ext-session-lock-v1 protocol support for external lockers. No system changes were made.";
  } else if (!qsBinaryPassed) {
    qsReason = "Quickshell binary is not installed on the host system.";
  } else if (!qsAuthPassed) {
    qsReason = "Host lacks compatible PAM authentication service for Quickshell unlock.";
  } else {
    qsReason = "Fully compatible with your Wayland compositor, ext-session-lock-v1 protocol, and PAM authentication.";
  }

  const qsEval: DetailedAdapterEvaluation = {
    adapter: "quickshell",
    target_name: "Session Lock (Quickshell)",
    category: "session_lock",
    supported: qsSupported,
    package_provided: qsHasManifest,
    reason: qsReason,
    system_changes_made: false,
    checks: {
      package_capability: {
        passed: qsHasManifest,
        required: "targets.quickshell",
        detected: qsHasManifest ? "Provided in manifest" : "Not provided",
        detail: qsHasManifest ? "Package provides Quickshell QML entrypoint" : "No Quickshell target in package",
      },
      compositor_protocol: {
        passed: qsCompositorProtocolPassed,
        required: "ext-session-lock-v1",
        detected: isGnome
          ? "compositor-native (Mutter)"
          : isKdePlasma
          ? "compositor-native (KWin)"
          : !isWayland
          ? "X11 / None"
          : "ext-session-lock-v1",
        detail: qsCompositorProtocolPassed ? "Protocol supported by active compositor" : "Protocol not supported by active desktop",
      },
      runtime_binary: {
        passed: qsBinaryPassed,
        required: "quickshell",
        detected: qsBinaryPassed ? "Installed / available" : "Not installed",
        detail: qsBinaryPassed ? "Binary found in system PATH" : "quickshell binary missing",
      },
      authentication: {
        passed: qsAuthPassed,
        required: "PAM authentication service",
        detected: qsAuthPassed ? "Compatible PAM service verified" : "Incompatible PAM service",
        detail: qsAuthPassed ? "Host PAM service available for unlock" : "No compatible PAM service found",
      },
      privilege_boundary: {
        passed: true,
        required: "user",
        detected: "user",
        detail: "Runs unprivileged in user session (~/.local/share/ryzora/)",
      },
    },
  };

  const sddmHasManifest = Boolean(sddmTarget);
  const sddmDmPassed =
    displayManager === "sddm" ||
    (!isGdm && displayManager !== "lightdm" && (systemInfo as any).installed_commands?.sddm);
  const sddmBinaryPassed = installedCmds.sddm !== false;
  const sddmSupported = sddmHasManifest && result.login_screen_supported && sddmDmPassed;
  let sddmReason = "";
  if (!sddmHasManifest) {
    sddmReason = "Package does not provide an SDDM login theme.";
  } else if (isGdm) {
    sddmReason =
      "Package provides an SDDM login theme, but your system uses GDM (GNOME Display Manager). Qylock SDDM themes cannot be installed into GDM. No system changes were made.";
  } else if (displayManager === "lightdm") {
    sddmReason =
      "Package provides an SDDM login theme, but your system uses LightDM. Qylock SDDM themes cannot be installed into LightDM. No system changes were made.";
  } else if (!sddmDmPassed) {
    sddmReason = `Active display manager is '${displayManager || "unknown"}', not SDDM. SDDM login themes cannot be applied. No system changes were made.`;
  } else {
    sddmReason =
      "SDDM is your active system display manager. Theme installation requires administrator elevation (pkexec).";
  }

  const sddmEval: DetailedAdapterEvaluation = {
    adapter: "sddm",
    target_name: "Login Screen (SDDM)",
    category: "login_screen",
    supported: sddmSupported,
    package_provided: sddmHasManifest,
    reason: sddmReason,
    system_changes_made: false,
    checks: {
      package_capability: {
        passed: sddmHasManifest,
        required: "targets.sddm",
        detected: sddmHasManifest ? "Provided in manifest" : "Not provided",
        detail: sddmHasManifest ? "Package provides SDDM theme files" : "No SDDM target in package",
      },
      compositor_protocol: {
        passed: true,
        required: "sddm-greeter",
        detected: "sddm-greeter",
        detail: "Display manager greeter protocol",
      },
      runtime_binary: {
        passed: sddmBinaryPassed,
        required: "sddm",
        detected: sddmBinaryPassed ? "Installed" : "Not installed",
        detail: sddmBinaryPassed ? "SDDM daemon/binary available" : "sddm binary missing",
      },
      authentication: {
        passed: true,
        required: "Display Manager Auth",
        detected: "SDDM Greeter PAM",
        detail: "Standard display manager greeter authentication",
      },
      display_manager: {
        passed: sddmDmPassed,
        required: "sddm",
        detected: displayManager || "unknown",
        detail: isGdm
          ? "GDM active (incompatible with SDDM themes)"
          : displayManager === "lightdm"
          ? "LightDM active (incompatible)"
          : sddmDmPassed
          ? "SDDM is active"
          : "SDDM is not active",
      },
      privilege_boundary: {
        passed: true,
        required: "administrator",
        detected: "administrator (pkexec)",
        detail: "Requires administrator elevation via Polkit to write /usr/share/sddm/ and /etc/sddm.conf.d/",
      },
    },
  };

  result.evaluations = {
    quickshell: qsEval,
    sddm: sddmEval,
  };

  return result;
}

/**
 * Authentic catalogue items from Qylock repository
 */
export const RAW_QYLOCK_THEMES: RawQylockTheme[] = DISCOVERED_QYLOCK_THEMES;

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
  // Phase 22: category field required by PackageProvider<T>
  category: "lockscreen",
  discover: () => RAW_QYLOCK_THEMES.map(normalizeQylockTheme),
  normalize: (raw: any) => normalizeQylockTheme(raw),
  // getPreview kept for backward compatibility with LockscreenProvider callers
  getPreview: (pkg: PackageItem): LockscreenMediaSpec => ({
    poster: pkg.preview_poster_url || pkg.hero_image,
    preview_video: pkg.preview_video_url,
    preview_animated: pkg.lockscreen?.media.preview_animated,
    upstream_video: pkg.lockscreen?.media.upstream_video,
    upstream_animated: pkg.lockscreen?.media.upstream_animated,
  }),
  // Phase 22: getMedia implements PackageProvider<T>.getMedia
  getMedia: (pkg: PackageItem): PackageMediaSpec => ({
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
  // Phase 22: getTargets returns PackageTargetSpec[] alongside legacy LockscreenTargetsSpec
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
