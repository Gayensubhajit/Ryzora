export type CategoryId =
  | "hub"
  | "discover"
  | "updates"
  | "repositories"
  | "creators"
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
  | "system"
  | "author"
  | "settings"
  | "notifications"
  | "collections"
  | "integrity";

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
// ─────────────────────────────────────────────────────────────────────────────
// Dependency Intelligence & Resolver Types (Phase 9)
// ─────────────────────────────────────────────────────────────────────────────

export type DependencyKind =
  | "package"
  | "system_binary"
  | "desktop_capability"
  | "runtime_capability";

export type DependencyStatus =
  | "satisfied"
  | "missing"
  | "incompatible"
  | "conflict"
  | "unknown";

export interface DependencySpec {
  id: string;
  kind: DependencyKind;
  version_req?: string | null;
  required?: boolean;
  description?: string | null;
}

export interface ResolvedPackageNode {
  package_id: string;
  name: string;
  version: string;
  version_req?: string | null;
  required: boolean;
  effective_required?: boolean;
  repository_id?: string | null;
  status: DependencyStatus;
  status_message?: string | null;
  dependencies: DependencySpec[];
  transitive_packages: string[];
}

export interface ResolvedSystemDependency {
  binary: string;
  required: boolean;
  status: DependencyStatus;
  path?: string | null;
  description?: string | null;
}

export interface ResolvedCapabilityDependency {
  capability: string;
  kind: DependencyKind;
  required: boolean;
  status: DependencyStatus;
  current_value?: string | null;
  description?: string | null;
}

export interface DependencyResolutionReport {
  root_package_id: string;
  resolved: boolean;
  root_package?: ResolvedPackageNode | null;
  packages: ResolvedPackageNode[];
  system_dependencies: ResolvedSystemDependency[];
  capability_dependencies: ResolvedCapabilityDependency[];
  missing_required: string[];
  missing_optional: string[];
  conflicts: string[];
  cycles: string[][];
  install_order: string[];
}

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
  /** Typed package dependencies (Phase 9). */
  dependencies?: DependencySpec[];
  /** Target definitions for multi-target packages (e.g. quickshell, sddm). */
  targets?: Record<string, any>;
  /** Media definitions (poster, preview video, animated GIF). */
  media?: Record<string, any>;
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


// ─────────────────────────────────────────────────────────────────────────────
// Lock Screen / Upstream Target & Media types
// ─────────────────────────────────────────────────────────────────────────────

export type LockscreenTargetType = "quickshell" | "sddm" | "hyprlock" | "swaylock";

export interface LockscreenTargetSpec {
  id: LockscreenTargetType;
  name: string;
  scope: "user" | "system";
  supported: boolean;
  entrypoint: string;
  dependencies: string[];
  optional_dependencies?: string[];
  requires_session_lock?: boolean;
  requires_root?: boolean;
  install_destination: string;
  config_file?: string;
  shortcut_target?: string;
}

export interface LockscreenMediaSpec {
  poster: string;
  preview_video?: string;
  preview_animated?: string;
  upstream_video?: string;
  upstream_animated?: string;
  has_audio?: boolean;
  aspect_ratio?: "16:9" | "4:3";
}

export interface LockscreenRuntimeSpec {
  entrypoint: string;
  assets: string[];
  bg_video_path?: string;
  fonts?: { name: string; filename: string; bundled: boolean }[];
  requires_multimedia?: boolean;
}

export interface LockscreenProvenance {
  provider_name: "qylock" | "hyprlock-community" | "swaylock-community" | "ryzora-native" | string;
  upstream_repo?: string;
  upstream_revision?: string;
  upstream_path?: string;
  variant?: string;
  author: string;
  license: string;
}

export interface LockscreenVariant {
  id: string;
  name: string;
  description?: string;
  preview_image?: string;
  preview_video?: string;
  config_overrides?: Record<string, any>;
}

export type LockscreenOptionType = "select" | "boolean" | "number" | "color";

export interface LockscreenOptionChoice {
  value: string;
  label: string;
  description?: string;
}

export interface LockscreenOptionSpec {
  id: string;
  label: string;
  type: LockscreenOptionType;
  description?: string;
  default: string | boolean | number;
  options?: LockscreenOptionChoice[];
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
}

export interface LockscreenConfigSchema {
  variants?: LockscreenVariant[];
  options?: Record<string, LockscreenOptionSpec>;
}

export interface LockscreenManifest {
  provider: "qylock" | "hyprlock" | "swaylock" | "custom";
  targets: Partial<Record<LockscreenTargetType, LockscreenTargetSpec>>;
  media: LockscreenMediaSpec;
  runtime: LockscreenRuntimeSpec;
  provenance: LockscreenProvenance;
  config_schema?: LockscreenConfigSchema;
  selected_config?: Record<string, any>;
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
  release_channel?: ReleaseChannel;
  trust_tier?: TrustTier;
  moderation_status?: ModerationStatus;
  trending_score?: number;
  maintainer?: string;
  release_notes?: string;
  signature?: PackageSignatureMetadata;
  cryptographic_status?: CryptographicStatus;
  media_type?: "video" | "image" | "animated";
  preview_video?: string;
  preview_animated?: string;
  preview_video_url?: string;
  preview_poster_url?: string;
  supports_session_lock?: boolean;
  supports_login_screen?: boolean;
  targets?: string[];
  lockscreen?: LockscreenManifest;
  customizable?: boolean;
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
  requires_privilege?: boolean;
  selected_target?: string;
  dependency_report?: DependencyResolutionReport | null;
}

export interface InstallResult {
  success: boolean;
  package_id: string;
  version: string;
  snapshot_id: string;
  installed_files: string[];
  errors: string[];
  rolled_back: boolean;
  selected_target?: string;
}

export interface InstalledFileEntry {
  target: string;
  sha256: string;
  is_symlink?: boolean;
  symlink_target?: string | null;
}

export interface InstalledHistoryEntry {
  version: string;
  installed_at: number;
  snapshot_id: string;
  repository_id?: string;
  tree_hash?: string;
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
  history?: InstalledHistoryEntry[];
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

// ─────────────────────────────────────────────────────────────────────────────
// Package Authoring & Store Publishing types (Phase 10)
// ─────────────────────────────────────────────────────────────────────────────

export interface FileMappingDraft {
  source_path: string;
  target: string;
  package_rel_path?: string;
  description: string;
}

export interface PackageDraft {
  id: string;
  name: string;
  version: string;
  author: string;
  package_type: PackageType;
  description: string;
  tags: string[];
  color_palette: string[];
  compatibility: ManifestCompatibility;
  dependencies: DependencySpec[];
  files: FileMappingDraft[];
}

export interface AuthoringResult {
  success: boolean;
  package_dir: string;
  manifest_path: string;
  files_copied: number;
  total_bytes: number;
  sha256_checksum: string;
  warnings: string[];
}

export interface PublishResult {
  success: boolean;
  repository_path: string;
  package_id: string;
  package_version: string;
  archive_path?: string | null;
  archive_sha256?: string | null;
  message: string;
}

export interface ManifestValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Store & Distribution Infrastructure types (Phase 11)
// ─────────────────────────────────────────────────────────────────────────────

export type ReleaseChannel = "stable" | "beta" | "nightly";

export type TrustTier = "official" | "verified" | "community" | "untrusted";

export type ModerationStatus = "approved" | "pending_review" | "flagged" | "deprecated";

export interface StoreAuditReport {
  passed: boolean;
  binary_executables_found: string[];
  script_hooks_found: string[];
  path_traversal_errors: string[];
  size_limit_errors: string[];
  metadata_errors: string[];
  warnings: string[];
  total_files: number;
  total_bytes: number;
  tree_hash: string;
  score: number;
}

export interface DistributionRelease {
  schema_version: number;
  package_id: string;
  name: string;
  version: string;
  package_type: PackageType;
  channel: ReleaseChannel;
  author: {
    name: string;
    avatar: string;
    verified: boolean;
  };
  maintainer?: string;
  release_notes: string;
  published_at: string;
  tree_hash: string;
  min_ryzora_version: string;
  trust_tier: TrustTier;
  files_count: number;
  total_bytes: number;
}

export interface DistributionReleaseResult {
  success: boolean;
  bundle_dir: string;
  manifest_path: string;
  release_path: string;
  checksums_path: string;
  submission_md_path: string;
  submission_text: string;
  tree_hash: string;
  channel: ReleaseChannel;
  message: string;
}


// ─────────────────────────────────────────────────────────────────────────────
// Cryptographic Trust & Signature Architecture types (Phase 12)
// ─────────────────────────────────────────────────────────────────────────────

export interface SignerIdentity {
  author_id: string;
  name: string;
  handle?: string;
}

export interface PackageSignatureMetadata {
  schema_version: number;
  algorithm: string;
  key_id: string;
  public_key: string;
  signed_tree_hash: string;
  signature: string;
  signer_identity: SignerIdentity;
  signed_at: string;
}

export type CryptographicStatus =
  | "official_verified"
  | "author_verified"
  | "self_signed_unvetted"
  | "unsigned"
  | "invalid_signature"
  | "revoked_key"
  | "official_impersonation";

export interface CryptographicEvaluation {
  status: CryptographicStatus;
  key_id?: string;
  public_key?: string;
  signer_name?: string;
  is_valid: boolean;
  is_trusted: boolean;
  can_install: boolean;
  error_message?: string;
}

export interface AuthorKeyPairInfo {
  key_id: string;
  public_key_hex: string;
  author_name: string;
  private_key_path: string;
  public_key_path: string;
}

export interface TrustedKeyEntry {
  key_id: string;
  public_key: string;
  author_name: string;
  role: string;
  added_at: string;
  notes: string;
}

export interface RevokedKeyEntry {
  key_id: string;
  public_key: string;
  reason: string;
  revoked_at: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Community Ingestion & CI Automation Types (Phase 13)
// ─────────────────────────────────────────────────────────────────────────────

export interface IngestionCheckResult {
  check_id: string;
  name: string;
  passed: boolean;
  level: "info" | "warning" | "error" | string;
  message: string;
}

export interface IngestionReport {
  package_id: string;
  package_name: string;
  version: string;
  passed: boolean;
  audit_score: number;
  computed_trust_tier: TrustTier;
  moderation_status: ModerationStatus;
  checks: IngestionCheckResult[];
  errors: string[];
  warnings: string[];
  pr_comment_markdown: string;
  tree_hash: string;
  signature_status: CryptographicStatus;
  signer_key_id?: string;
}

export interface IngestionResult {
  success: boolean;
  package_id: string;
  version: string;
  repository_path: string;
  report: IngestionReport;
}

// ─────────────────────────────────────────────────────────────────────────────
// Ryzora Hub — Repository Sync, Categorized Updates & Creator Profiles (Phase 14)
// ─────────────────────────────────────────────────────────────────────────────

export interface RepositorySyncStatus {
  id: string;
  name: string;
  url: string;
  repo_type: "local" | "remote" | string;
  enabled: boolean;
  status: "online" | "offline" | "cached" | "refresh_failed" | string;
  package_count: number;
  last_synced?: string;
  error_message?: string;
  channel: string;
  available_channels: string[];
}

export interface RepositorySyncReport {
  repository_id: string;
  success: boolean;
  packages_discovered: number;
  timestamp: string;
  message: string;
}

export type UpdateCategory = "Security" | "Feature" | "Optional";

export interface PackageUpdateItem {
  package_id: string;
  name: string;
  installed_version: string;
  available_version: string;
  category: UpdateCategory;
  repository_id: string;
  release_channel: string;
  release_notes?: string;
  tree_hash?: string;
  cryptographic_status?: string;
  snapshot_id: string;
}

export interface UpdatesDashboardSummary {
  total_updates: number;
  security_updates_count: number;
  feature_updates_count: number;
  optional_updates_count: number;
  updates: PackageUpdateItem[];
}

export interface CreatorProfile {
  id: string;
  display_name: string;
  avatar: string;
  verified: boolean;
  trust_tier: "Official" | "Verified" | "Community" | "Untrusted";
  public_key_fingerprint?: string;
  total_packages: number;
  total_downloads: number;
  average_rating?: number;
  package_ids: string[];
  recent_packages: any[];
  release_channels: string[];
}

export interface HubOverview {
  total_installed: number;
  available_updates_count: number;
  security_updates_count: number;
  total_repositories: number;
  online_repositories_count: number;
  featured_creators: CreatorProfile[];
  recent_installs: InstalledPackageRecord[];
  latest_releases: any[];
  sync_summary: RepositorySyncStatus[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 15 — Settings, Notifications, Collections & Integrity
// ─────────────────────────────────────────────────────────────────────────────

// 15.1 — Settings
export interface RyzoraSettings {
  default_release_channel: "stable" | "beta" | "nightly";
  auto_refresh_enabled: boolean;
  auto_refresh_interval_minutes: number;
  notification_level: "all" | "security_only" | "none";
  update_notification_policy: "notify" | "silent";
  integrity_scan_on_startup: boolean;
  show_unverified_packages: boolean;
  show_nightly_packages: boolean;
  compact_ui: boolean;
  snapshot_policy: "never" | "ask" | "always";
  snapshot_retention_count: number;
  snapshot_auto_cleanup: boolean;
}

// 15.2 — Notifications & Activity
export type NotificationSeverity = "info" | "warning" | "error" | "critical";

export interface ActivityEntry {
  id: string;
  timestamp: string;
  severity: NotificationSeverity;
  category: string;
  title: string;
  message: string;
  read: boolean;
  related_package_id?: string;
  related_repository_id?: string;
}

// 15.4 — Collections
export interface CollectionPackage {
  id: string;
  version_req?: string;
  repository?: string;
}

export interface Collection {
  ryzora_collection: "1";
  id: string;
  name: string;
  description?: string;
  author?: string;
  created_at: string;
  updated_at: string;
  packages: CollectionPackage[];
}

export interface CollectionSummary {
  id: string;
  name: string;
  description?: string;
  package_count: number;
  created_at: string;
  updated_at: string;
}

export interface CollectionInstallPreview {
  collection_id: string;
  collection_name: string;
  total_packages: number;
  already_installed: string[];
  to_install: string[];
  unavailable: string[];
  warnings: string[];
}

export interface CollectionInstallResult {
  collection_id: string;
  total_packages: number;
  installed: string[];
  skipped: string[];
  failed: string[];
  rolled_back: boolean;
  errors: string[];
}

// 15.5 — Integrity Health
export type IntegrityStatus =
  | "healthy"
  | "modified"
  | "missing_files"
  | "unexpected_files"
  | "signature_failed"
  | "key_revoked"
  | "unable_to_verify";

export interface PackageIntegrityResult {
  package_id: string;
  name: string;
  version: string;
  trust_tier: string;
  status: IntegrityStatus;
  status_label: string;
  detail: string;
  signing_key_id?: string;
  scanned_at: string;
}

export interface IntegrityScanReport {
  total_checked: number;
  healthy: number;
  issues: PackageIntegrityResult[];
  all_results: PackageIntegrityResult[];
  scanned_at: string;
  duration_ms: number;
}

// CategoryId extension for Phase 15
export type Phase15CategoryId =
  | "settings"
  | "notifications"
  | "collections"
  | "integrity";

export interface ActiveLockscreenState {
  quickshell?: string | null;
  sddm?: string | null;
  quickshell_theme_path?: string | null;
  sddm_theme_path?: string | null;
  last_applied_at?: number | null;
  hypridle_override?: boolean;
  hypridle_source_hash?: string | null;
  hypridle_source_path?: string | null;
  hypridle_ryzora_config?: string | null;
  lock_wrapper_path?: string | null;
  sddm_previous_theme?: string | null;
  active_config?: Record<string, any> | null;
}

export interface LockscreenTargetCapability {
  adapter: string;
  name: string;
  category: 'session_lock' | 'login_screen';
  supported: boolean;
  reason: string;
  required_privilege: 'user' | 'administrator';
  runtime_binary: string;
  binary_installed: boolean;
  protocol: string;
}

export interface ActiveLockscreenInfo {
  session_lock_type: string;
  session_lock_name: string | null;
  session_lock_config: string | null;
  login_screen_type: string;
  login_screen_theme: string | null;
  login_screen_config: string | null;
  managed_by: string | null;
}

export interface SystemIntegrationReport {
  desktop: string;
  display_server: string;
  session_lock_provider: string;
  session_lock_entrypoint: string | null;
  idle_provider: string;
  idle_config: string | null;
  login_manager: string;
  login_theme: string | null;
  login_config: string | null;
  confidence: "high" | "medium" | "low" | string;
  evidence: string[];
  warnings: string[];
}

export interface HostCapabilities {
  os: string;
  distro_id: string;
  distro_name: string;
  desktop_environment: string;
  compositor: string;
  compositor_version?: string | null;
  session_type: string;
  session_lock_protocol: string;
  display_manager: string;
  display_manager_service?: string | null;
  display_manager_theme?: string | null;
  active_lockscreen: ActiveLockscreenInfo;
  installed_commands: Record<string, boolean>;
  supported_adapters: LockscreenTargetCapability[];
}

export interface SddmConfigEntrySummary {
  path: string;
  theme: string;
}

export interface SddmRuntimeStatus {
  available: boolean;
  helper_installed: boolean;
  installed: boolean;
  applied: boolean;
  active: boolean;
  ryzora_theme: string | null;
  effective_theme: string | null;
  effective_file: string | null;
  is_overridden: boolean;
  overridden_by: string | null;
  previous_theme: string | null;
  config_entries: SddmConfigEntrySummary[];
  error: string | null;
}

export interface LockscreenRuntimeStatus {
  target: string;
  adapter: string;
  available: boolean;
  installed: boolean;
  applied: boolean;
  active: boolean;
  entrypoint?: string | null;
  protocol: string;
  active_package?: string | null;
  launcher_path?: string | null;
  error?: string | null;
  hypridle_integration?: boolean;
  hypridle_config_drift?: boolean;
  lock_wrapper_exists?: boolean;
  privileged_helper_installed?: boolean;
  sddm_effective_theme?: string | null;
  sddm_effective_file?: string | null;
  sddm_is_overridden?: boolean;
  sddm_overridden_by?: string | null;
  sddm_previous_theme?: string | null;
  user_lock_hook_active?: boolean;
  user_lock_hook_path?: string | null;
}

export interface PrivilegedHelperStatus {
  installed: boolean;
  helper_path: string;
  helper_exists: boolean;
  helper_executable: boolean;
  helper_valid: boolean;
  policy_path: string;
  policy_exists: boolean;
  version?: string | null;
  sha256?: string | null;
  error?: string | null;
}

export interface LockscreenTestResult {
  success: boolean;
  target: string;
  test_runtime_dir: string;
  tested_config: Record<string, any>;
  message: string;
}
