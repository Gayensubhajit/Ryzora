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
  color_palette: string[]; // hex codes for accent palette
  safety_audit: SafetyAudit;
  dependencies: {
    packages: string[];
    optional: string[];
  };
  components: PackageComponentSpec[];
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
