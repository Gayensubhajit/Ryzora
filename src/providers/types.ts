import type { PackageItem } from "../types/index.ts";
export type { PackageItem };

export interface LockscreenProvenance {
  upstream: string;
  revision: string;
  license?: string;
  path?: string;
}

export interface LockscreenSourceSpec {
  type: "git" | "archive" | "local";
  repository?: string;
  revision?: string;
  path?: string;
  url?: string;
  sha256?: string;
}

export interface LockscreenTargetsSpec {
  quickshell: boolean;
  sddm: boolean;
  hyprlock?: boolean;
  swaylock?: boolean;
}

export interface LockscreenMediaSpec {
  poster: string;
  preview_video?: string;
  preview_animated?: string;
  upstream_video?: string;
  upstream_animated?: string;
}

export interface LockscreenProvider {
  id: string;
  name: string;
  discover(): Promise<PackageItem[]> | PackageItem[];
  normalize(raw: any): PackageItem;
  getPreview(pkg: PackageItem): LockscreenMediaSpec;
  getSource(pkg: PackageItem): LockscreenSourceSpec;
  getTargets(pkg: PackageItem): LockscreenTargetsSpec;
  getDependencies(pkg: PackageItem, target: string): string[];
  getProvenance(pkg: PackageItem): LockscreenProvenance;
  install?(pkg: PackageItem, target: string, options?: any): Promise<any>;
  uninstall?(pkg: PackageItem, target: string): Promise<any>;
}
