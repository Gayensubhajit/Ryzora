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
  components: PackageComponentSpec[];
  compatibility: CompatibilityRequirements;
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
