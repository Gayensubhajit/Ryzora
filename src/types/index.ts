export type CategoryId =
  | "discover"
  | "rices"
  | "themes"
  | "bars"
  | "lockscreens"
  | "fastfetch"
  | "wallpapers"
  | "terminal"
  | "icons"
  | "cursors"
  | "fonts"
  | "widgets"
  | "bundles"
  | "installed"
  | "backups"
  | "system";

export type DesktopEnvironment =
  | "hyprland"
  | "sway"
  | "kde"
  | "gnome"
  | "xfce"
  | "cinnamon"
  | "cosmic"
  | "i3"
  | "universal";

// ─────────────────────────────────────────────────────────────────────────────
// Ryzora Package Manifest — spec v1
// ─────────────────────────────────────────────────────────────────────────────

/** All canonical Ryzora package types. */
export type PackageType =
  | "rice"
  | "theme"
  | "waybar"
  | "fastfetch"
  | "lockscreen"
  | "wallpaper"
  | "terminal"
  | "icon"
  | "cursor"
  | "font"
  | "widget"
  | "bundle";

/** Compatibility block inside a manifest — uses author-facing field names. */
export interface ManifestCompatibility {
  /** Desktop environments / window managers supported. Empty = universal. */
  desktops: string[];
  /** Required session protocol ("wayland" | "x11"). Empty = any. */
  sessions: string[];
  /** Supported distro IDs / families. Empty or ["all"] = universal. */
  distros: string[];
  /** Binaries that MUST be in PATH. */
  required: string[];
  /** Binaries that improve the experience but are not mandatory. */
  optional: string[];
}

/** A single file mapping within a Ryzora package. */
export interface ManifestFile {
  /** Path relative to the package root, e.g. "files/hypr/hyprland.conf". */
  source: string;
  /** Destination on the user system — must start with "~/". */
  target: string;
  /** Human-readable description of what this file does. */
  description: string;
}

/** The authoritative Ryzora Package Manifest (spec v1). */
export interface RyzoraManifest {
  /** Unique package slug, e.g. "cyberpunk-neon-2077". */
  id: string;
  /** Human-readable name. */
  name: string;
  /** Semver version string (X.Y.Z). */
  version: string;
  /** Manifest specification version. Must be "1". */
  ryzora_spec: string;
  /** Author name or handle. */
  author: string;
  /** Canonical package type. */
  package_type: PackageType;
  /** Long description. */
  description: string;
  /** Searchable tags. */
  tags: string[];
  /** Hex color palette for UI previews. */
  color_palette: string[];
  /** Compatibility requirements. */
  compatibility: ManifestCompatibility;
  /** Files this package installs. */
  files: ManifestFile[];
}

/** Result returned by the Rust validate_manifest command. */
export interface ManifestValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// System Probe types
// ─────────────────────────────────────────────────────────────────────────────

export interface InstalledComponent {
  name: string;
  binary: string;
  installed: boolean;
  path: string | null;
  category: string;
}

export interface SystemInfo {
  distro_name: string;
  distro_id: string;
  distro_family: string;
  distro_version: string;
  kernel_version: string;
  desktop_environment: string;
  window_manager: string;
  session_type: string; // "wayland" | "x11"
  shell: string;
  terminal: string;
  installed_components: InstalledComponent[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Compatibility Engine types
// ─────────────────────────────────────────────────────────────────────────────

export type CompatibilityLevel =
  | "Compatible"
  | "MissingDependencies"
  | "IncompatibleSession"
  | "IncompatibleDesktop"
  | "IncompatibleDistro";

export interface CompatibilityIssue {
  severity: "error" | "warning" | "info";
  code: string;
  message: string;
  target?: string;
}

/**
 * Internal compatibility requirements shape used by the Rust engine.
 * Keys use underscore-snake names to match Rust serde output.
 */
export interface CompatibilityRequirements {
  supported_distros: string[];
  supported_desktops: string[];
  supported_sessions: string[];
  required_binaries: string[];
  optional_binaries: string[];
}

export interface CompatibilityReport {
  level: CompatibilityLevel;
  score: number;
  summary_label: string;
  session_compatible: boolean;
  desktop_compatible: boolean;
  distro_compatible: boolean;
  satisfied_apps: string[];
  missing_required_apps: string[];
  missing_optional_apps: string[];
  issues: CompatibilityIssue[];
}

// ─────────────────────────────────────────────────────────────────────────────
// UI / Marketplace types
// ─────────────────────────────────────────────────────────────────────────────

export interface PackageComponentSpec {
  name: string;
  component_type: string;
  target_path: string;
  description: string;
  icon?: string;
}

export interface SafetyAudit {
  rating: "safe" | "verified" | "requires_review";
  changes_system_files: boolean;
  requires_root: boolean;
  sandbox_compatible: boolean;
  files_modified_count: number;
}

export interface PackageItem {
  id: string;
  title: string;
  subtitle: string;
  description: string;
  version: string;
  author: {
    name: string;
    avatar: string;
    verified: boolean;
  };
  category: CategoryId;
  /** Canonical package type — maps to ManifestCompatibility. */
  package_type: PackageType;
  tags: string[];
  supported_desktops: DesktopEnvironment[];
  supported_display: ("wayland" | "x11")[];
  rating: number;
  rating_count: number;
  downloads: number;
  hero_image: string;
  screenshots: string[];
  featured?: boolean;
  trending?: boolean;
  recent?: boolean;
  color_palette: string[];
  safety_audit: SafetyAudit;
  dependencies: {
    packages: string[];
    optional: string[];
  };
  /** Legacy component list — kept for backward compatibility. */
  components: PackageComponentSpec[];
  /** Internal requirements used by the Rust compatibility engine. */
  compatibility: CompatibilityRequirements;
  /**
   * The parsed Ryzora manifest for this package.
   * Present for manifest-backed packages; undefined for legacy mock entries
   * that have not yet been migrated.
   */
  manifest?: RyzoraManifest;
}

export interface SnapshotRecord {
  id: string;
  package_id: string;
  package_name: string;
  timestamp: number;
  formatted_date: string;
  backed_up_paths: string[];
  status: "active" | "restored";
}
