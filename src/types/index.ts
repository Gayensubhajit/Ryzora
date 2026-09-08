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
  rating: "safe" | "verified" | "requires_review" | "unverified";
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
  repository_id?: string;
  content_hash?: string;
  package_size_bytes?: number;
  integrity_status?: "verified" | "unverified" | "corrupted" | "pending_download";
  is_cached?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Snapshot / Backup Engine types (Phase 3)
// ─────────────────────────────────────────────────────────────────────────────

export type FileEntryType = "Regular" | "Directory" | "Symlink" | "Absent";
export type SnapshotStatus = "Pending" | "Complete" | "Verified" | "Corrupted" | "Restored";

/** Metadata for one backed-up filesystem entry inside a snapshot. */
export interface SnapshotFileEntry {
  original_path: string;
  absolute_original: string;
  backup_relative: string;
  file_type: FileEntryType;
  symlink_target: string | null;
  size_bytes: number;
  sha256: string | null;
  existed: boolean;
}

/** The full snapshot record — mirrors Rust SnapshotMetadata. */
export interface SnapshotMetadata {
  id: string;
  label: string;
  created_at: number;
  formatted_date: string;
  ryzora_version: string;
  status: SnapshotStatus;
  entries: SnapshotFileEntry[];
  total_size_bytes: number;
  verified: boolean;
}

/** Result from the restore_snapshot Tauri command. */
export interface RestoreResult {
  restored_count: number;
  skipped_count: number;
  removed_absent_count: number;
  errors: string[];
  success: boolean;
}

/**
 * Legacy type alias — kept for any remaining code that uses the old
 * in-memory snapshot shape. Will be removed in a future cleanup.
 * @deprecated Use SnapshotMetadata instead.
 */
export interface SnapshotRecord {
  id: string;
  package_id: string;
  package_name: string;
  timestamp: number;
  formatted_date: string;
  backed_up_paths: string[];
  status: "active" | "restored";
}

// ─────────────────────────────────────────────────────────────────────────────
// Declarative Installer types (Phase 4)
// ─────────────────────────────────────────────────────────────────────────────

export interface InstallationPlan {
  package_id: string;
  package_name: string;
  package_version: string;
  files_to_create: string[];
  files_to_replace: string[];
  files_unchanged: string[];
  directories_to_create: string[];
  conflicts: string[];
  compatibility_status: "compatible" | "missing_dependencies" | "incompatible";
  required_dependencies: string[];
  missing_dependencies: string[];
  warnings: string[];
}

export interface InstallResult {
  success: boolean;
  package_id: string;
  version: string;
  snapshot_id: string;
  installed_files: string[];
  errors: string[];
  rolled_back: boolean;
}

export interface InstalledFileEntry {
  target: string;
  sha256: string;
  is_symlink?: boolean;
  symlink_target?: string | null;
}

export interface InstalledPackageRecord {
  package_id: string;
  name: string;
  version: string;
  package_type?: PackageType;
  repository_id?: string;
  installed_at: number;
  snapshot_id: string;
  installed_files: string[];
  files?: InstalledFileEntry[];
  package_source_path: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Safe Uninstall & Update types (Phase 7)
// ─────────────────────────────────────────────────────────────────────────────

export interface UninstallResult {
  package_id: string;
  success: boolean;
  removed_files: string[];
  already_missing_files: string[];
  conflict_files: string[];
  removed_directories: string[];
  retained_directories: string[];
  snapshot_id?: string | null;
  rolled_back: boolean;
  error?: string | null;
}

export type UpdateStatusKind =
  | "up_to_date"
  | "update_available"
  | "repository_unavailable"
  | "package_not_found"
  | "unable_to_determine";

export interface PackageUpdateStatus {
  package_id: string;
  installed_version: string;
  available_version?: string | null;
  repository_id?: string | null;
  status: UpdateStatusKind;
  message?: string | null;
}

export type FileAction =
  | "create"
  | "replace"
  | "unchanged"
  | "conflict"
  | "obsolete_remove"
  | "obsolete_retain";

export interface UpdateFileItem {
  target: string;
  action: FileAction;
  reason: string;
  current_sha256?: string | null;
  new_sha256?: string | null;
}

export interface UpdatePlan {
  package_id: string;
  from_version: string;
  to_version: string;
  repository_id?: string | null;
  creates: string[];
  replaces: string[];
  unchanged: string[];
  conflicts: string[];
  obsolete_removes: string[];
  obsolete_retains: string[];
  details: UpdateFileItem[];
  has_conflicts: boolean;
}

export interface UpdateResult {
  success: boolean;
  package_id: string;
  from_version: string;
  to_version: string;
  snapshot_id: string;
  updated_files: string[];
  obsolete_removed: string[];
  conflicts_retained: string[];
  rolled_back: boolean;
  errors: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Repository & Catalog System types (Phase 5)
// ─────────────────────────────────────────────────────────────────────────────

export interface RepositorySummary {
  id: string;
  name: string;
  package_count: number;
  path: string;
  repo_type?: string;
  enabled?: boolean;
  status?: string;
  last_refreshed?: string | null;
  last_error?: string | null;
}

export interface CacheStats {
  total_size_bytes: number;
  cached_packages_count: number;
  cached_repositories_count: number;
  cache_dir: string;
}

export interface RepositorySourceConfig {
  id: string;
  name: string;
  url: string;
  enabled: boolean;
  repo_type: "local" | "remote";
}
