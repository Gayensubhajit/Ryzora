import { getCatalogueLockScreens } from "../providers/qylockProvider";
import React, { createContext, useContext, useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { MOCK_PACKAGES } from "../data/mockPackages";
import {
  DependencyResolutionReport,
  UninstallResult,
  PackageUpdateStatus,
  UpdatePlan,
  UpdateResult,
  CategoryId,
  CompatibilityReport,
  CompatibilityRequirements,
  DesktopEnvironment,
  InstallationPlan,
  InstallResult,
  InstalledPackageRecord,
  ManifestValidationResult,
  PackageItem,
  RepositorySourceConfig,
  RepositorySummary,
  RestoreResult,
  SnapshotMetadata,
  SystemInfo,
  CacheStats,
  PackageDraft,
  AuthoringResult,
  PublishResult,
  StoreAuditReport,
  DistributionReleaseResult,
  AuthorKeyPairInfo,
  TrustedKeyEntry,
  CryptographicEvaluation,
  ReleaseChannel,
  IngestionReport,
  PrivilegedHelperStatus,
  SddmRuntimeStatus,
  IngestionResult,
  HubOverview,
  UpdatesDashboardSummary,
  CreatorProfile,
  RepositorySyncStatus,
  RepositorySyncReport,
  InstalledHistoryEntry,
  ActiveLockscreenState,
  HostCapabilities,
  LockscreenRuntimeStatus,
  SystemIntegrationReport,
  RyzoraSettings,
} from "../types";

interface AppContextType {
  systemInfo: SystemInfo | null;
  loadingSystem: boolean;
  activeCategory: CategoryId;
  setActiveCategory: (cat: CategoryId) => void;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  desktopFilter: DesktopEnvironment | "all";
  setDesktopFilter: (filter: DesktopEnvironment | "all") => void;
  packages: PackageItem[];
  selectedPackage: PackageItem | null;
  setSelectedPackage: (pkg: PackageItem | null) => void;
  installedPackageIds: string[];
  installedPackages: InstalledPackageRecord[];
  snapshots: SnapshotMetadata[];
  isInstalling: boolean;
  installProgress: number;
  installLogs: string[];
  previewInstallation: (packageId: string, target?: string) => Promise<InstallationPlan>;
  resolvePackageDependencies: (packageId: string) => Promise<DependencyResolutionReport>;
  installPackage: (pkg: PackageItem, createSnapshot?: boolean, target?: string) => Promise<InstallResult>;
  uninstallPackage: (packageId: string) => Promise<UninstallResult>;
  checkPackageUpdate: (packageId: string) => Promise<PackageUpdateStatus>;
  checkAllUpdates: () => Promise<PackageUpdateStatus[]>;
  previewPackageUpdate: (packageId: string) => Promise<UpdatePlan>;
  applyPackageUpdate: (packageId: string) => Promise<UpdateResult>;
  rollbackSnapshot: (snapId: string) => Promise<void>;
  deleteSnapshot: (snapId: string) => Promise<void>;
  refreshSystem: () => Promise<void>;
  repositories: RepositorySummary[];
  repositorySources: RepositorySourceConfig[];
  refreshCatalog: () => Promise<void>;
  refreshRepositories: () => Promise<void>;
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  addRepositorySource: (config: RepositorySourceConfig) => Promise<void>;
  removeRepositorySource: (id: string) => Promise<void>;
  getCacheStats: () => Promise<CacheStats>;
  clearPackageCache: (packageId?: string) => Promise<number>;
  clearAllCache: () => Promise<number>;
  checkCompatibility: (pkg: PackageItem) => CompatibilityReport;
  validateManifest: (manifestJson: string) => Promise<ManifestValidationResult>;
  validatePackageDraft: (draft: PackageDraft) => Promise<ManifestValidationResult>;
  createPackage: (draft: PackageDraft, destinationDir: string) => Promise<AuthoringResult>;
  publishPackageToRepository: (packageDir: string, repositoryPath: string) => Promise<PublishResult>;
  auditStoreSubmission: (packageDir: string) => Promise<StoreAuditReport>;
  buildDistributionRelease: (
    packageDir: string,
    outputDir: string,
    channel: ReleaseChannel,
    releaseNotes?: string,
    maintainer?: string
  ) => Promise<DistributionReleaseResult>;
  getAuthorKeypair: (authorName?: string) => Promise<AuthorKeyPairInfo>;
  listTrustedKeys: () => Promise<TrustedKeyEntry[]>;
  verifyPackageCryptography: (packageDir: string) => Promise<CryptographicEvaluation>;
  runCiSubmissionAudit: (submissionDir: string, repoDir?: string) => Promise<IngestionReport>;
  ingestCommunitySubmission: (submissionDir: string, repoDir: string) => Promise<IngestionResult>;
  toast: { message: string; type: "success" | "info" | "warning" } | null;
  setToast: (toast: { message: string; type: "success" | "info" | "warning" } | null) => void;
  // Phase 14: Ryzora Hub, Updates, Creators & Sync
  hubOverview: HubOverview | null;
  loadingHub: boolean;
  refreshHub: () => Promise<void>;
  updatesSummary: UpdatesDashboardSummary | null;
  loadingUpdates: boolean;
  refreshUpdates: () => Promise<void>;
  creatorProfiles: CreatorProfile[];
  loadingCreators: boolean;
  refreshCreators: () => Promise<void>;
  repositorySyncStatuses: RepositorySyncStatus[];
  loadingRepoSync: boolean;
  refreshRepositorySync: (repoId: string) => Promise<RepositorySyncReport>;
  refreshAllRepositoriesSync: () => Promise<RepositorySyncReport[]>;
  switchRepositoryChannel: (repoId: string, channel: string) => Promise<RepositorySyncStatus>;
  getInstalledPackageHistory: (packageId: string) => Promise<InstalledHistoryEntry[]>;
  activeLockscreen: ActiveLockscreenState;
  applyLockscreen: (packageId: string, target: "quickshell" | "sddm" | "both", config?: Record<string, any>) => Promise<ActiveLockscreenState>;
  deactivateLockscreen: (target: "quickshell" | "sddm" | "both") => Promise<ActiveLockscreenState>;
  refreshActiveLockscreen: () => Promise<ActiveLockscreenState>;
  testLockscreen: (packageId?: string, target?: string) => Promise<void>;
  checkConfigDrift: () => Promise<string | null>;
  privilegedHelperStatus: PrivilegedHelperStatus | null;
  isSettingUpHelper: boolean;
  checkPrivilegedHelper: () => Promise<PrivilegedHelperStatus>;
  setupPrivilegedHelper: () => Promise<PrivilegedHelperStatus>;
  hostCapabilities: HostCapabilities | null;
  runtimeStatus: LockscreenRuntimeStatus | null;
  sddmRuntimeStatus: SddmRuntimeStatus | null;
  systemIntegrationReport: SystemIntegrationReport | null;
  settings: RyzoraSettings | null;
  loadSettings: () => Promise<RyzoraSettings>;
  loadHostCapabilities: () => Promise<HostCapabilities>;
  loadSystemIntegrationReport: () => Promise<SystemIntegrationReport>;
  deactivateAndUninstallLockscreen: (packageId: string, target?: string) => Promise<boolean>;
  loadRuntimeStatus: () => Promise<LockscreenRuntimeStatus>;
  loadSddmRuntimeStatus: (packageId?: string) => Promise<SddmRuntimeStatus>;
}

const AppContext = createContext<AppContextType | undefined>(undefined);

const FALLBACK_SYSTEM_INFO: SystemInfo = {
  distro_name: "Garuda Linux",
  distro_id: "garuda",
  distro_family: "arch",
  distro_version: "Rolling",
  kernel_version: "6.18.50-1-lts",
  desktop_environment: "Hyprland",
  window_manager: "Hyprland",
  session_type: "wayland",
  shell: "zsh",
  terminal: "kitty",
  installed_components: [
    { name: "Hyprland", binary: "hyprland", installed: true, path: "/usr/bin/hyprland", category: "Window Manager" },
    { name: "Waybar", binary: "waybar", installed: true, path: "/usr/bin/waybar", category: "Status Bar" },
    { name: "Kitty", binary: "kitty", installed: true, path: "/usr/bin/kitty", category: "Terminal" },
    { name: "Fastfetch", binary: "fastfetch", installed: true, path: "/usr/bin/fastfetch", category: "System Fetch" },
    { name: "Rofi", binary: "rofi", installed: true, path: "/usr/bin/rofi", category: "Launcher" },
    { name: "Hyprlock", binary: "hyprlock", installed: true, path: "/usr/bin/hyprlock", category: "Lockscreen" },
    { name: "Starship", binary: "starship", installed: true, path: "/usr/bin/starship", category: "Shell Prompt" },
    { name: "Sway", binary: "sway", installed: false, path: null, category: "Window Manager" },
  ],
};

function evaluateClientCompatibility(
  sys: SystemInfo | null,
  reqs: CompatibilityRequirements
): CompatibilityReport {
  if (!sys) {
    return {
      level: "Compatible",
      score: 100,
      summary_label: "Compatible",
      session_compatible: true,
      desktop_compatible: true,
      distro_compatible: true,
      satisfied_apps: [],
      missing_required_apps: [],
      missing_optional_apps: [],
      issues: [],
    };
  }

  const issues: CompatibilityReport["issues"] = [];
  const satisfied_apps: string[] = [];
  const missing_required_apps: string[] = [];
  const missing_optional_apps: string[] = [];

  const distroId = sys.distro_id.toLowerCase();
  const distroFamily = (sys.distro_family || "").toLowerCase();
  const distro_compatible =
    reqs.supported_distros.length === 0 ||
    reqs.supported_distros.some((d) => {
      const dl = d.toLowerCase();
      return dl === "all" || dl === "universal" || dl === distroId || dl === distroFamily;
    });

  if (!distro_compatible) {
    issues.push({
      severity: "error",
      code: "DISTRO_MISMATCH",
      message: `Package requires distros [${reqs.supported_distros.join(", ")}]`,
      target: sys.distro_id,
    });
  }

  const sessionType = sys.session_type.toLowerCase();
  const session_compatible =
    reqs.supported_sessions.length === 0 ||
    reqs.supported_sessions.some((s) => {
      const sl = s.toLowerCase();
      return sl === "any" || sl === "all" || sl === sessionType;
    });

  if (!session_compatible) {
    issues.push({
      severity: "error",
      code: "SESSION_MISMATCH",
      message: `Package requires session [${reqs.supported_sessions.join(", ")}]`,
      target: sys.session_type,
    });
  }

  const wm = sys.window_manager.toLowerCase();
  const de = sys.desktop_environment.toLowerCase();
  const desktop_compatible =
    reqs.supported_desktops.length === 0 ||
    reqs.supported_desktops.some((d) => {
      const dl = d.toLowerCase();
      return (
        dl === "universal" ||
        dl === "all" ||
        dl === wm ||
        dl === de ||
        wm.includes(dl) ||
        de.includes(dl)
      );
    });

  if (!desktop_compatible) {
    issues.push({
      severity: "error",
      code: "DESKTOP_MISMATCH",
      message: `Package designed for [${reqs.supported_desktops.join(", ")}]`,
      target: sys.window_manager,
    });
  }

  const installedSet = new Set(
    sys.installed_components.filter((c) => c.installed).map((c) => c.binary.toLowerCase())
  );

  for (const bin of reqs.required_binaries) {
    const bl = bin.toLowerCase();
    const found =
      installedSet.has(bl) ||
      (bl === "rofi-wayland" && installedSet.has("rofi")) ||
      (bl === "rofi" && installedSet.has("rofi-wayland"));

    if (found) {
      satisfied_apps.push(bin);
    } else {
      missing_required_apps.push(bin);
      issues.push({
        severity: "warning",
        code: "MISSING_REQUIRED_BIN",
        message: `Required tool '${bin}' is not detected in PATH`,
        target: bin,
      });
    }
  }

  for (const bin of reqs.optional_binaries) {
    const bl = bin.toLowerCase();
    if (installedSet.has(bl)) {
      satisfied_apps.push(bin);
    } else {
      missing_optional_apps.push(bin);
      issues.push({
        severity: "info",
        code: "MISSING_OPTIONAL_BIN",
        message: `Optional tool '${bin}' is not detected in PATH`,
        target: bin,
      });
    }
  }

  let level: CompatibilityReport["level"] = "Compatible";
  let score = 100;
  let summary_label = "Compatible";

  if (!desktop_compatible) {
    level = "IncompatibleDesktop";
    score = 20;
    const target = reqs.supported_desktops[0] || "desktop";
    summary_label = `Requires ${target.charAt(0).toUpperCase() + target.slice(1)}`;
  } else if (!session_compatible) {
    level = "IncompatibleSession";
    score = 30;
    const target = reqs.supported_sessions[0] || "Wayland";
    summary_label = `Requires ${target.charAt(0).toUpperCase() + target.slice(1)}`;
  } else if (!distro_compatible) {
    level = "IncompatibleDistro";
    score = 40;
    summary_label = "Distro mismatch";
  } else if (missing_required_apps.length > 0) {
    level = "MissingDependencies";
    score = 75;
    summary_label = `Missing: ${missing_required_apps[0]}`;
  }

  return {
    level,
    score,
    summary_label,
    session_compatible,
    desktop_compatible,
    distro_compatible,
    satisfied_apps,
    missing_required_apps,
    missing_optional_apps,
    issues,
  };
}

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [loadingSystem, setLoadingSystem] = useState<boolean>(true);
  const [activeCategory, setActiveCategory] = useState<CategoryId>("discover");
  const [sidebarCollapsed, setSidebarCollapsed] = useState<boolean>(() => {
    const saved = localStorage.getItem("ryzora_sidebar_collapsed");
    return saved !== null ? saved === "true" : false;
  });

  const toggleSidebar = () => {
    setSidebarCollapsed((prev) => {
      const next = !prev;
      localStorage.setItem("ryzora_sidebar_collapsed", String(next));
      return next;
    });
  };
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [desktopFilter, setDesktopFilter] = useState<DesktopEnvironment | "all">("all");
  const [packages, setPackages] = useState<PackageItem[]>([]);
  const [repositories, setRepositories] = useState<RepositorySummary[]>([]);
  const [repositorySources, setRepositorySources] = useState<RepositorySourceConfig[]>([]);
  const [compatibilityMap, setCompatibilityMap] = useState<Record<string, CompatibilityReport>>({});
  const [selectedPackage, setSelectedPackage] = useState<PackageItem | null>(null);
  const [installedPackages, setInstalledPackages] = useState<InstalledPackageRecord[]>([]);
  const [installedPackageIds, setInstalledPackageIds] = useState<string[]>([]);
  const [snapshots, setSnapshots] = useState<SnapshotMetadata[]>([]);
  const [isInstalling, setIsInstalling] = useState<boolean>(false);
  const [installProgress, setInstallProgress] = useState<number>(0);
  const [installLogs, setInstallLogs] = useState<string[]>([]);
  const [toast, setToast] = useState<{ message: string; type: "success" | "info" | "warning" } | null>(null);
  const [hubOverview, setHubOverview] = useState<HubOverview | null>(null);
  const [loadingHub, setLoadingHub] = useState<boolean>(false);
  const [updatesSummary, setUpdatesSummary] = useState<UpdatesDashboardSummary | null>(null);
  const [loadingUpdates, setLoadingUpdates] = useState<boolean>(false);
  const [creatorProfiles, setCreatorProfiles] = useState<CreatorProfile[]>([]);
  const [loadingCreators, setLoadingCreators] = useState<boolean>(false);
  const [repositorySyncStatuses, setRepositorySyncStatuses] = useState<RepositorySyncStatus[]>([]);
  const [loadingRepoSync, setLoadingRepoSync] = useState<boolean>(false);
  const [hostCapabilities, setHostCapabilities] = useState<HostCapabilities | null>(null);
  const [runtimeStatus, setRuntimeStatus] = useState<LockscreenRuntimeStatus | null>(null);
  const [sddmRuntimeStatus, setSddmRuntimeStatus] = useState<SddmRuntimeStatus | null>(null);
  const [systemIntegrationReport, setSystemIntegrationReport] = useState<SystemIntegrationReport | null>(null);
  const [settings, setSettings] = useState<RyzoraSettings | null>(null);

  const loadSettings = async (): Promise<RyzoraSettings> => {
    try {
      const s = await invoke<RyzoraSettings>("get_settings");
      setSettings(s);
      return s;
    } catch (e) {
      console.warn("Failed to load settings:", e);
      const fallback: RyzoraSettings = {
        default_release_channel: "stable",
        auto_refresh_enabled: true,
        auto_refresh_interval_minutes: 60,
        notification_level: "all",
        update_notification_policy: "notify",
        integrity_scan_on_startup: true,
        show_unverified_packages: false,
        show_nightly_packages: false,
        compact_ui: false,
        snapshot_policy: "ask",
        snapshot_retention_count: 5,
        snapshot_auto_cleanup: true,
      };
      setSettings(fallback);
      return fallback;
    }
  };

  const loadSddmRuntimeStatus = async (packageId?: string): Promise<SddmRuntimeStatus> => {
    try {
      const st = await invoke<SddmRuntimeStatus>("get_sddm_runtime_status", { packageId });
      setSddmRuntimeStatus(st);
      return st;
    } catch (e: any) {
      console.warn("Failed to load SDDM runtime status:", e);
      const fallback: SddmRuntimeStatus = {
        available: true,
        helper_installed: false,
        installed: false,
        applied: false,
        active: false,
        ryzora_theme: null,
        effective_theme: null,
        effective_file: null,
        is_overridden: false,
        overridden_by: null,
        previous_theme: null,
        config_entries: [],
        error: e?.message || String(e),
      };
      setSddmRuntimeStatus(fallback);
      return fallback;
    }
  };


  const loadHostCapabilities = useCallback(async (): Promise<HostCapabilities> => {
    try {
      const caps = await invoke<HostCapabilities>("get_host_capabilities");
      setHostCapabilities(caps);
      return caps;
    } catch (e) {
      console.warn("Failed to load host capabilities:", e);
      const fallback: HostCapabilities = {
        os: "Arch Linux",
        distro_id: "arch",
        distro_name: "Arch Linux",
        desktop_environment: "Hyprland",
        compositor: "Hyprland",
        compositor_version: "0.55.x",
        session_type: "wayland",
        session_lock_protocol: "ext-session-lock-v1",
        display_manager: "sddm",
        display_manager_service: "sddm.service",
        display_manager_theme: "winter",
        active_lockscreen: {
          session_lock_type: "hyprlock",
          session_lock_name: "~/.config/hypr/hyprlock_themes/006_stacked_clock/hyprlock.conf",
          session_lock_config: "/home/silentbyte/.config/hypr/hypridle.conf",
          login_screen_type: "sddm",
          login_screen_theme: "winter",
          login_screen_config: "sddm.service",
          managed_by: "Dusky",
        },
        installed_commands: {
          quickshell: true,
          hyprlock: true,
          swaylock: false,
          sddm: true,
          pkexec: true,
        },
        supported_adapters: [
          {
            adapter: "quickshell",
            name: "Quickshell Session Lock",
            category: "session_lock",
            supported: true,
            reason: "Fully compatible with your Wayland compositor and ext-session-lock-v1",
            required_privilege: "user",
            runtime_binary: "quickshell",
            binary_installed: true,
            protocol: "ext-session-lock-v1",
          },
          {
            adapter: "sddm",
            name: "SDDM Login Screen",
            category: "login_screen",
            supported: true,
            reason: "SDDM is your active system display manager",
            required_privilege: "administrator",
            runtime_binary: "sddm",
            binary_installed: true,
            protocol: "sddm-greeter",
          }
        ],
      };
      setHostCapabilities(fallback);
      return fallback;
    }
  }, []);

  const loadRuntimeStatus = useCallback(async (): Promise<LockscreenRuntimeStatus> => {
    try {
      const st = await invoke<LockscreenRuntimeStatus>("get_lock_screen_runtime_status");
      setRuntimeStatus(st);
      return st;
    } catch (e) {
      console.warn("Failed to load runtime status:", e);
      const fallback: LockscreenRuntimeStatus = {
        target: "quickshell",
        adapter: "quickshell",
        available: true,
        installed: true,
        applied: true,
        active: true,
        protocol: "ext-session-lock-v1",
      };
      setRuntimeStatus(fallback);
      return fallback;
    }
  }, []);

  const [activeLockscreen, setActiveLockscreen] = useState<ActiveLockscreenState>({
    quickshell: null,
    sddm: null,
  });

  const refreshActiveLockscreen = async (): Promise<ActiveLockscreenState> => {
    try {
      const state = await invoke<ActiveLockscreenState>("get_active_lockscreen");
      setActiveLockscreen(state || { quickshell: null, sddm: null });
      return state || { quickshell: null, sddm: null };
    } catch {
      return { quickshell: null, sddm: null };
    }
  };

  const applyLockscreen = async (
    packageId: string,
    target: "quickshell" | "sddm" | "both",
    config?: Record<string, any>
  ): Promise<ActiveLockscreenState> => {
    try {
      const state = await invoke<ActiveLockscreenState>("apply_lockscreen", {
        packageId,
        target,
        config: config || null,
      });
      setActiveLockscreen(state);
      setToast({
        message: `Activated ${packageId} for ${
          target === "both" ? "Session Lock & SDDM Login Screen" : target === "quickshell" ? "Session Lock" : "SDDM Login Screen"
        }!`,
        type: "success",
      });
      return state;
    } catch (e: any) {
      setToast({
        message: `Failed to activate lockscreen: ${e?.message || e}`,
        type: "warning",
      });
      throw e;
    }
  };

  const deactivateLockscreen = async (
    target: "quickshell" | "sddm" | "both"
  ): Promise<ActiveLockscreenState> => {
    try {
      const state = await invoke<ActiveLockscreenState>("deactivate_lockscreen", {
        target,
      });
      setActiveLockscreen(state);
      setToast({
        message: `Deactivated ${
          target === "both" ? "Session Lock & SDDM" : target === "quickshell" ? "Session Lock" : "SDDM Login Screen"
        }.`,
        type: "info",
      });
      return state;
    } catch (e: any) {
      setToast({
        message: `Failed to deactivate lockscreen: ${e?.message || e}`,
        type: "warning",
      });
      throw e;
    }
  };

  const loadSystemIntegrationReport = async (): Promise<SystemIntegrationReport> => {
    try {
      const rep = await invoke<SystemIntegrationReport>("get_system_integration_report");
      setSystemIntegrationReport(rep);
      return rep;
    } catch (e) {
      console.error("Failed to load system integration report:", e);
      const fallback: SystemIntegrationReport = {
        desktop: "Hyprland",
        display_server: "Wayland",
        session_lock_provider: "Quickshell",
        session_lock_entrypoint: null,
        idle_provider: "hypridle",
        idle_config: "~/.config/hypr/hypridle.conf",
        login_manager: "SDDM",
        login_theme: "winter",
        login_config: "/etc/sddm.conf.d/theme.conf",
        confidence: "high",
        evidence: ["Default environment fallback"],
        warnings: [],
      };
      setSystemIntegrationReport(fallback);
      return fallback;
    }
  };

  const deactivateAndUninstallLockscreen = async (packageId: string, target?: string): Promise<boolean> => {
    try {
      await invoke("deactivate_and_uninstall_lockscreen", { packageId, target });
      await loadInstalledPackages();
      await refreshActiveLockscreen();
      setToast({
        message: "Lockscreen cleanly deactivated and uninstalled.",
        type: "success",
      });
      return true;
    } catch (e: any) {
      setToast({
        message: `Failed to uninstall lockscreen: ${e}`,
        type: "warning",
      });
      return false;
    }
  };

  const testLockscreen = async (packageId?: string, target?: string): Promise<void> => {
    try {
      await invoke("launch_lockscreen_test", { packageId, target });
      setToast({
        message: "Lockscreen test launched.",
        type: "success",
      });
    } catch (e: any) {
      setToast({
        message: `Failed to launch lockscreen test: ${e?.message || e}`,
        type: "warning",
      });
      throw e;
    }
  };

  const checkConfigDrift = async (): Promise<string | null> => {
    try {
      return await invoke<string | null>("check_lockscreen_config_drift");
    } catch (e) {
      console.warn("Failed to check lockscreen config drift:", e);
      return null;
    }
  };

  const [privilegedHelperStatus, setPrivilegedHelperStatus] = useState<PrivilegedHelperStatus | null>(null);
  const [isSettingUpHelper, setIsSettingUpHelper] = useState(false);

  const checkPrivilegedHelper = async (): Promise<PrivilegedHelperStatus> => {
    try {
      const status = await invoke<PrivilegedHelperStatus>("get_privileged_helper_status");
      setPrivilegedHelperStatus(status);
      return status;
    } catch (e: any) {
      console.warn("Failed to check privileged helper status:", e);
      const fallback: PrivilegedHelperStatus = {
        installed: false,
        helper_path: "/usr/lib/ryzora/ryzora-sddm-helper",
        helper_exists: false,
        helper_executable: false,
        helper_valid: false,
        policy_path: "/usr/share/polkit-1/actions/io.ryzora.sddm.policy",
        policy_exists: false,
        error: e?.message || String(e),
      };
      setPrivilegedHelperStatus(fallback);
      return fallback;
    }
  };

  const setupPrivilegedHelper = async (): Promise<PrivilegedHelperStatus> => {
    setIsSettingUpHelper(true);
    try {
      const status = await invoke<PrivilegedHelperStatus>("setup_privileged_helper");
      setPrivilegedHelperStatus(status);
      setToast({
        message: "Ryzora privileged system integration configured successfully.",
        type: "success",
      });
      return status;
    } catch (e: any) {
      const msg = e?.message || String(e);
      setToast({
        message: `System integration setup failed: ${msg}`,
        type: "warning",
      });
      throw e;
    } finally {
      setIsSettingUpHelper(false);
    }
  };

  const refreshSystem = async () => {
    setLoadingSystem(true);
    let currentSystem: SystemInfo = FALLBACK_SYSTEM_INFO;
    try {
      const info = await invoke<SystemInfo>("detect_system_info");
      setSystemInfo(info);
      currentSystem = info;
    } catch {
      setSystemInfo(FALLBACK_SYSTEM_INFO);
    } finally {
      setLoadingSystem(false);
    }

    // Evaluate compatibility through Rust backend
    try {
      const batchInputs = packages.map((p) => ({
        id: p.id,
        requirements: p.compatibility,
      }));
      const reports = await invoke<Record<string, CompatibilityReport>>(
        "evaluate_batch_compatibility",
        { packages: batchInputs }
      );
      setCompatibilityMap(reports);
    } catch {
      // Synchronous fallback evaluation
      const fallbackMap: Record<string, CompatibilityReport> = {};
      for (const p of packages) {
        fallbackMap[p.id] = evaluateClientCompatibility(currentSystem, p.compatibility);
      }
      setCompatibilityMap(fallbackMap);
    }
  };

  const loadSnapshots = async () => {
    try {
      const list = await invoke<SnapshotMetadata[]>("list_snapshots");
      setSnapshots(list);
    } catch {
      // No fallback data — real snapshots only
      setSnapshots([]);
    }
  };

  const loadInstalledPackages = async () => {
    try {
      const list = await invoke<InstalledPackageRecord[]>("list_installed_packages");
      setInstalledPackages(list);
      setInstalledPackageIds(list.map((p) => p.package_id));
    } catch {
      // In browser preview / mock mode without Tauri runtime, provide installed fallback
      const mockInstalled: InstalledPackageRecord[] = [
        {
          package_id: "dog-samurai",
          name: "Dog Samurai",
          version: "1.0.0",
          package_type: "lockscreen",
          installed_at: Date.now() - 86400000 * 2,
          snapshot_id: "snap-dog-samurai-01",
          installed_files: ["/usr/share/sddm/themes/dog-samurai"],
          package_source_path: "qylock/dog-samurai",
        },
        {
          package_id: "clockwork-tape",
          name: "Tape (Clockwork)",
          version: "1.0.0",
          package_type: "lockscreen",
          installed_at: Date.now() - 86400000,
          snapshot_id: "snap-clockwork-tape-01",
          installed_files: ["/usr/share/sddm/themes/tape"],
          package_source_path: "qylock/clockwork-tape",
        },
        {
          package_id: "rice-cyberpunk-neon",
          name: "Cyberpunk Neon 2077",
          version: "2.4.0",
          package_type: "rice",
          installed_at: Date.now() - 86400000 * 5,
          snapshot_id: "snap-cyberpunk-01",
          installed_files: ["~/.config/hypr/hyprland.conf", "~/.config/waybar/config"],
          package_source_path: "rices/rice-cyberpunk-neon",
        },
      ];
      setInstalledPackages(mockInstalled);
      setInstalledPackageIds(mockInstalled.map((p) => p.package_id));
    }
  };

  const mergeCanonicalLockScreens = (basePackages: PackageItem[]): PackageItem[] => {
    const lockscreens = getCatalogueLockScreens();
    const merged: PackageItem[] = basePackages.map((pkg) => {
      const hasQs = pkg.supports_session_lock ?? Boolean(
        pkg.manifest?.targets?.["quickshell"] ||
        pkg.lockscreen?.targets?.quickshell ||
        pkg.targets?.includes("quickshell") ||
        pkg.tags?.some((t) => t.toLowerCase() === "quickshell")
      );
      const hasSddm = pkg.supports_login_screen ?? Boolean(
        pkg.manifest?.targets?.["sddm"] ||
        pkg.lockscreen?.targets?.sddm ||
        pkg.targets?.includes("sddm") ||
        pkg.tags?.some((t) => t.toLowerCase() === "sddm")
      );
      return {
        ...pkg,
        supports_session_lock: hasQs,
        supports_login_screen: hasSddm,
      };
    });

    for (const ls of lockscreens) {
      const existingIdx = merged.findIndex(
        (p) =>
          p.id === ls.id ||
          p.id === `lockscreen-qylock-${ls.id}` ||
          p.id === `lockscreen-${ls.id}` ||
          (ls.id === "aurora-hyprlock" && p.id === "lockscreen-hyprlock-aurora") ||
          (ls.id === "swaylock-blur" && p.id === "lockscreen-swaylock-blur")
      );
      if (existingIdx >= 0) {
        merged[existingIdx] = {
          ...merged[existingIdx],
          ...ls,
          id: merged[existingIdx].id,
          manifest: merged[existingIdx].manifest || ls.manifest,
        };
      } else {
        merged.push(ls);
      }
    }

    const normalizeMediaUrl = (url?: string): string | undefined => {
      if (!url || typeof url !== "string") return undefined;
      const trimmed = url.trim();
      if (!trimmed) return undefined;
      if (
        trimmed.startsWith("http://") ||
        trimmed.startsWith("https://") ||
        trimmed.startsWith("/") ||
        trimmed.startsWith("data:") ||
        trimmed.startsWith("blob:")
      ) {
        return trimmed;
      }
      return "/" + trimmed;
    };

    return merged.map((pkg) => {
      const vid = pkg.preview_video_url || pkg.preview_video || pkg.manifest?.media?.preview_video || pkg.lockscreen?.media?.preview_video;
      const poster = pkg.preview_poster_url || pkg.hero_image || pkg.manifest?.media?.poster || pkg.lockscreen?.media?.poster;
      const anim = pkg.preview_animated || pkg.manifest?.media?.preview_animated || pkg.lockscreen?.media?.preview_animated;

      const normVid = normalizeMediaUrl(vid);
      const normPoster = normalizeMediaUrl(poster);
      const normAnim = normalizeMediaUrl(anim);

      const hasVideo = Boolean(normVid);
      const hasAnim = Boolean(normAnim);

      let mediaType: "video" | "animated" | "image" = "image";
      if (pkg.media_type === "video" || pkg.media_type === "animated" || pkg.media_type === "image") {
        mediaType = pkg.media_type;
      } else if ((pkg.manifest as any)?.media_type) {
        mediaType = (pkg.manifest as any).media_type;
      } else if (hasVideo) {
        mediaType = "video";
      } else if (hasAnim) {
        mediaType = "animated";
      }

      return {
        ...pkg,
        hero_image: normPoster || pkg.hero_image,
        preview_video_url: normVid,
        preview_video: normVid,
        preview_poster_url: normPoster,
        preview_animated: normAnim,
        media_type: mediaType,
      };
    });
  };

  const loadCatalogPackages = async () => {
    try {
      const catalog = await invoke<PackageItem[]>("get_catalog_packages");
      setPackages(mergeCanonicalLockScreens(catalog || []));
      const repos = await invoke<RepositorySummary[]>("get_repository_info");
      setRepositories(repos);
    } catch (e) {
      console.warn("Failed to load catalog from RepositoryManager, using fallback in dev/browser", e);
      setPackages(mergeCanonicalLockScreens(MOCK_PACKAGES));
    }
  };

  const loadRepositorySources = async () => {
    try {
      const sources = await invoke<RepositorySourceConfig[]>("list_repository_sources");
      setRepositorySources(sources);
    } catch {
      setRepositorySources([]);
    }
  };

  const refreshCatalog = async () => {
    try {
      const catalog = await invoke<PackageItem[]>("refresh_catalog");
      setPackages(mergeCanonicalLockScreens(catalog || []));
      const repos = await invoke<RepositorySummary[]>("get_repository_info");
      setRepositories(repos);
    } catch (e) {
      console.warn("Failed to refresh catalog", e);
    }
  };

  const refreshRepositories = async () => {
    try {
      const catalog = await invoke<PackageItem[]>("refresh_catalog");
      setPackages(mergeCanonicalLockScreens(catalog || []));
      const repos = await invoke<RepositorySummary[]>("get_repository_info");
      setRepositories(repos);
      await loadRepositorySources();
      setToast({ message: "Repositories refreshed", type: "info" });
    } catch (e: any) {
      setToast({ message: `Refresh failed: ${e?.message || e}`, type: "warning" });
    }
  };

  const addRepositorySource = async (config: RepositorySourceConfig) => {
    try {
      await invoke("add_repository_source", { config });
      await loadRepositorySources();
      await refreshRepositories();
      setToast({ message: `Added repository '${config.name}'`, type: "success" });
    } catch (e: any) {
      setToast({ message: `Failed to add repository: ${e?.message || e}`, type: "warning" });
    }
  };

  const removeRepositorySource = async (id: string) => {
    try {
      await invoke("remove_repository_source", { id });
      await loadRepositorySources();
      await refreshRepositories();
      setToast({ message: "Repository removed", type: "info" });
    } catch (e: any) {
      setToast({ message: `Failed to remove repository: ${e?.message || e}`, type: "warning" });
    }
  };

  const getCacheStats = async (): Promise<CacheStats> => {
    try {
      return await invoke<CacheStats>("get_cache_stats");
    } catch {
      return { total_size_bytes: 0, cached_packages_count: 0, cached_repositories_count: 0, cache_dir: "" };
    }
  };

  const clearPackageCache = async (packageId?: string): Promise<number> => {
    try {
      const bytes = await invoke<number>("clear_package_cache", { packageId: packageId ?? null });
      await refreshCatalog();
      setToast({ message: `Package cache cleaned (${Math.round(bytes / 1024)} KB freed)`, type: "info" });
      return bytes;
    } catch (e: any) {
      setToast({ message: `Failed to clean cache: ${e?.message || e}`, type: "warning" });
      return 0;
    }
  };

  const clearAllCache = async (): Promise<number> => {
    try {
      const bytes = await invoke<number>("clear_all_cache");
      await refreshCatalog();
      setToast({ message: `All cache cleaned (${Math.round(bytes / 1024)} KB freed)`, type: "info" });
      return bytes;
    } catch (e: any) {
      setToast({ message: `Failed to clean cache: ${e?.message || e}`, type: "warning" });
      return 0;
    }
  };

  useEffect(() => {
    refreshSystem();
    loadSnapshots();
    loadInstalledPackages();
    loadCatalogPackages();
    loadRepositorySources();
    refreshHub();
    refreshUpdates();
    refreshCreators();
    refreshRepoSyncStatuses();
    refreshActiveLockscreen();
    checkPrivilegedHelper();
    loadHostCapabilities();
    loadSystemIntegrationReport();
    loadSettings();
    loadRuntimeStatus();
  }, []);

  useEffect(() => {
    if (toast) {
      const timer = setTimeout(() => setToast(null), 4000);
      return () => clearTimeout(timer);
    }
  }, [toast]);

  const checkCompatibility = (pkg: PackageItem): CompatibilityReport => {
    if (compatibilityMap[pkg.id]) {
      return compatibilityMap[pkg.id];
    }
    return evaluateClientCompatibility(systemInfo, pkg.compatibility);
  };

  const validateManifest = async (manifestJson: string): Promise<ManifestValidationResult> => {
    try {
      return await invoke<ManifestValidationResult>("validate_manifest", { manifestJson });
    } catch {
      return { valid: false, errors: ["Validation service unavailable"], warnings: [] };
    }
  };

  const previewInstallation = async (packageId: string, target?: string): Promise<InstallationPlan> => {
    try {
      await invoke("prepare_provider_package", { packageId });
    } catch {
      // Non-provider package or already present locally
    }
    return await invoke<InstallationPlan>("preview_installation", { packageId, target });
  };

  const resolvePackageDependencies = async (packageId: string): Promise<DependencyResolutionReport> => {
    return await invoke<DependencyResolutionReport>("resolve_package_dependencies", { packageId });
  };

  const installPackage = async (
    pkg: PackageItem,
    createSnapshot: boolean = true,
    target?: string
  ): Promise<InstallResult> => {
    setIsInstalling(true);
    setInstallProgress(10);
    setInstallLogs([
      `[Step 1/5] Inspecting package payload & validating manifest for '${pkg.title}'...`,
    ]);

    try {
      // Step 0: Ensure provider package payload is prepared & synthesized
      if (pkg.repository_id?.startsWith("provider:")) {
        setInstallLogs((prev) => [
          ...prev,
          `[Provider] Validating declarative payload from ${pkg.repository_id}...`,
        ]);
        try {
          await invoke("prepare_provider_package", { packageId: pkg.id });
        } catch (prepErr) {
          console.warn("Provider package prepare warning:", prepErr);
        }
      }

      // Step 1: Pre-flight preview/plan
      const plan = await previewInstallation(pkg.id, target);
      setInstallProgress(25);
      setInstallLogs((prev) => [
        ...prev,
        `[Plan] To create: ${plan.files_to_create.length} · To replace: ${plan.files_to_replace.length} · Unchanged: ${plan.files_unchanged.length}`,
      ]);

      if (plan.conflicts.length > 0) {
        throw new Error(`Conflicts detected: ${plan.conflicts.join("; ")}`);
      }
      if (plan.missing_dependencies.length > 0) {
        throw new Error(`Missing required dependencies: ${plan.missing_dependencies.join(", ")}`);
      }

      setInstallProgress(40);
      if (createSnapshot) {
        setInstallLogs((prev) => [
          ...prev,
          `[Step 2/5] Creating & verifying pre-install snapshot...`,
        ]);
      } else {
        setInstallLogs((prev) => [
          ...prev,
          `[Step 2/5] Skipping pre-install snapshot per user request (fast install)...`,
        ]);
      }

      setInstallProgress(65);
      setInstallLogs((prev) => [
        ...prev,
        `[Step 3/5] Staging files in isolated sandbox & validating checksums...`,
      ]);

      setInstallProgress(80);
      setInstallLogs((prev) => [
        ...prev,
        `[Step 4/5] Safely applying configuration to target paths...`,
      ]);

      const result = await invoke<InstallResult>("install_package", {
        packageId: pkg.id,
        createSnapshot,
        target,
      });

      if (result.success) {
        setInstallProgress(100);
        setInstallLogs((prev) => [
          ...prev,
          result.snapshot_id
            ? `[Step 5/5] Verified on disk! Snapshot created: ${result.snapshot_id}`
            : `[Step 5/5] Verified on disk! Direct install completed without snapshot.`,
          `✓ '${pkg.title}' installed successfully (${result.installed_files.length} files).`,
        ]);

        await loadInstalledPackages();
        if (createSnapshot) {
          await loadSnapshots();
        }

        setToast({
          message: `Installed ${pkg.title}`,
          type: "success",
        });

        setIsInstalling(false);
        return result;
      } else {
        const errorMsg = result.errors.join("; ") || "Unknown error";
        setInstallLogs((prev) => [
          ...prev,
          `[Failed] ${errorMsg}`,
          result.rolled_back
            ? result.snapshot_id
              ? `[Rollback] Restored original configuration from snapshot ${result.snapshot_id}.`
              : `[Rollback] Cleaned up partially written target files.`
            : ``,
        ]);
        setToast({
          message: `Installation failed: ${errorMsg}`,
          type: "warning",
        });
        setIsInstalling(false);
        return result;
      }
    } catch (e: any) {
      const errStr = e?.message || String(e);
      setInstallLogs((prev) => [
        ...prev,
        `[Error] ${errStr}`,
      ]);
      setToast({
        message: `Install failed: ${errStr}`,
        type: "warning",
      });
      setIsInstalling(false);
      throw e;
    }
  };

  const rollbackSnapshot = async (snapId: string) => {
    try {
      const result = await invoke<RestoreResult>("restore_snapshot", { id: snapId });
      if (result.success) {
        setToast({
          message: `Restored ${result.restored_count} file(s) from snapshot`,
          type: "success",
        });
      } else {
        setToast({
          message: `Restore completed with ${result.errors.length} issue(s)`,
          type: "warning",
        });
      }
    } catch (e) {
      setToast({
        message: `Restore failed: ${e}`,
        type: "warning",
      });
    }
    // Refresh snapshots to get updated status
    await loadSnapshots();
  };

  const uninstallPackage = async (packageId: string): Promise<UninstallResult> => {
    try {
      const res = await invoke<UninstallResult>("uninstall_package", { packageId });
      await loadInstalledPackages();
      await loadSnapshots();
      return res;
    } catch (e: any) {
      console.error("Failed to uninstall package:", e);
      throw e;
    }
  };

  const checkPackageUpdate = async (packageId: string): Promise<PackageUpdateStatus> => {
    return await invoke<PackageUpdateStatus>("check_package_update", { packageId });
  };

  const checkAllUpdates = async (): Promise<PackageUpdateStatus[]> => {
    return await invoke<PackageUpdateStatus[]>("check_all_updates");
  };

  const previewPackageUpdate = async (packageId: string): Promise<UpdatePlan> => {
    return await invoke<UpdatePlan>("preview_package_update", { packageId });
  };

  const applyPackageUpdate = async (packageId: string): Promise<UpdateResult> => {
    try {
      const res = await invoke<UpdateResult>("apply_package_update", { packageId });
      await loadInstalledPackages();
      await loadSnapshots();
      return res;
    } catch (e: any) {
      console.error("Failed to apply update:", e);
      throw e;
    }
  };

  const validatePackageDraft = async (draft: PackageDraft): Promise<ManifestValidationResult> => {
    return await invoke<ManifestValidationResult>("validate_package_draft", { draft });
  };

  const createPackage = async (draft: PackageDraft, destinationDir: string): Promise<AuthoringResult> => {
    return await invoke<AuthoringResult>("create_package", { draft, destinationDir });
  };

  const publishPackageToRepository = async (
    packageDir: string,
    repositoryPath: string
  ): Promise<PublishResult> => {
    const res = await invoke<PublishResult>("publish_package_to_repository", {
      packageDir,
      repositoryPath,
    });
    await refreshCatalog();
    await refreshRepositories();
    return res;
  };


  const auditStoreSubmission = async (packageDir: string): Promise<StoreAuditReport> => {
    return await invoke<StoreAuditReport>("audit_store_submission", { packageDir });
  };

  const buildDistributionRelease = async (
    packageDir: string,
    outputDir: string,
    channel: ReleaseChannel,
    releaseNotes?: string,
    maintainer?: string
  ): Promise<DistributionReleaseResult> => {
    return await invoke<DistributionReleaseResult>("build_distribution_release", {
      packageDir,
      outputDir,
      channel,
      releaseNotes: releaseNotes || null,
      maintainer: maintainer || null,
    });
  };

  const getAuthorKeypair = async (authorName?: string): Promise<AuthorKeyPairInfo> => {
    return await invoke<AuthorKeyPairInfo>("get_author_keypair", { authorName: authorName || null });
  };

  const listTrustedKeys = async (): Promise<TrustedKeyEntry[]> => {
    return await invoke<TrustedKeyEntry[]>("list_trusted_keys");
  };

  const verifyPackageCryptography = async (packageDir: string): Promise<CryptographicEvaluation> => {
    return await invoke<CryptographicEvaluation>("verify_package_cryptography", { packageDir });
  };

  const runCiSubmissionAudit = async (
    submissionDir: string,
    repoDir?: string
  ): Promise<IngestionReport> => {
    return await invoke<IngestionReport>("run_ci_submission_audit", {
      submissionDir,
      repoDir: repoDir || null,
    });
  };

  const ingestCommunitySubmission = async (
    submissionDir: string,
    repoDir: string
  ): Promise<IngestionResult> => {
    const res = await invoke<IngestionResult>("ingest_community_submission", {
      submissionDir,
      repoDir,
    });
    await refreshCatalog();
    await refreshRepositories();
    return res;
  };

  const deleteSnapshot = async (snapId: string) => {
    try {
      await invoke("delete_snapshot", { id: snapId });
      setSnapshots((prev) => prev.filter((s) => s.id !== snapId));
      setToast({ message: "Snapshot deleted", type: "info" });
    } catch (e) {
      setToast({ message: `Delete failed: ${e}`, type: "warning" });
    }
  };

  const refreshHub = async () => {
    setLoadingHub(true);
    try {
      const data = await invoke<HubOverview>("get_hub_overview");
      setHubOverview(data);
    } catch (e) {
      console.error("Failed to load hub overview", e);
    } finally {
      setLoadingHub(false);
    }
  };

  const refreshUpdates = async () => {
    setLoadingUpdates(true);
    try {
      const data = await invoke<UpdatesDashboardSummary>("get_updates_dashboard");
      setUpdatesSummary(data);
    } catch (e) {
      console.error("Failed to load updates dashboard", e);
    } finally {
      setLoadingUpdates(false);
    }
  };

  const refreshCreators = async () => {
    setLoadingCreators(true);
    try {
      const data = await invoke<CreatorProfile[]>("list_creator_profiles");
      setCreatorProfiles(data || []);
    } catch (e) {
      console.error("Failed to load creator profiles", e);
    } finally {
      setLoadingCreators(false);
    }
  };

  const refreshRepoSyncStatuses = async () => {
    setLoadingRepoSync(true);
    try {
      const data = await invoke<RepositorySyncStatus[]>("get_repository_sync_status");
      setRepositorySyncStatuses(data || []);
    } catch (e) {
      console.error("Failed to load repository sync statuses", e);
    } finally {
      setLoadingRepoSync(false);
    }
  };

  const refreshRepositorySync = async (repositoryId: string): Promise<RepositorySyncReport> => {
    setLoadingRepoSync(true);
    try {
      const report = await invoke<RepositorySyncReport>("refresh_repository_sync", { repositoryId });
      await refreshRepoSyncStatuses();
      await refreshCatalog();
      await refreshUpdates();
      await refreshHub();
      setToast({
        message: report.message,
        type: report.success ? "success" : "warning",
      });
      return report;
    } finally {
      setLoadingRepoSync(false);
    }
  };

  const refreshAllRepositoriesSync = async (): Promise<RepositorySyncReport[]> => {
    setLoadingRepoSync(true);
    try {
      const reports = await invoke<RepositorySyncReport[]>("refresh_all_repositories_sync");
      await refreshRepoSyncStatuses();
      await refreshCatalog();
      await refreshUpdates();
      await refreshHub();
      const allOk = reports.every((r) => r.success);
      setToast({
        message: `All repositories synchronized (${reports.length} sources)`,
        type: allOk ? "success" : "info",
      });
      return reports;
    } finally {
      setLoadingRepoSync(false);
    }
  };

  const switchRepositoryChannel = async (repositoryId: string, channel: string): Promise<RepositorySyncStatus> => {
    const status = await invoke<RepositorySyncStatus>("switch_repository_channel", { repositoryId, channel });
    await refreshRepoSyncStatuses();
    await refreshCatalog();
    await refreshUpdates();
    await refreshHub();
    setToast({
      message: `Repository '${status.name}' channel switched to ${channel}`,
      type: "success",
    });
    return status;
  };

  const getInstalledPackageHistory = async (packageId: string): Promise<InstalledHistoryEntry[]> => {
    return await invoke<InstalledHistoryEntry[]>("get_installed_package_history", { packageId });
  };

  return (
    <AppContext.Provider
      value={{
        systemInfo,
        loadingSystem,
        activeCategory,
        setActiveCategory,
        searchQuery,
        setSearchQuery,
        desktopFilter,
        setDesktopFilter,
        packages,
        selectedPackage,
        setSelectedPackage,
        installedPackageIds,
        installedPackages,
        snapshots,
        isInstalling,
        installProgress,
        installLogs,
        previewInstallation,
        resolvePackageDependencies,
        installPackage,
        uninstallPackage,
        checkPackageUpdate,
        checkAllUpdates,
        previewPackageUpdate,
        applyPackageUpdate,
        rollbackSnapshot,
        refreshSystem,
        repositories,
        repositorySources,
        refreshCatalog,
        refreshRepositories,
        sidebarCollapsed,
        toggleSidebar,
        setSidebarCollapsed,
        addRepositorySource,
        removeRepositorySource,
        getCacheStats,
        clearPackageCache,
        clearAllCache,
        checkCompatibility,
        validateManifest,
        validatePackageDraft,
        createPackage,
        publishPackageToRepository,
        auditStoreSubmission,
        buildDistributionRelease,
        getAuthorKeypair,
        listTrustedKeys,
        verifyPackageCryptography,
        runCiSubmissionAudit,
        ingestCommunitySubmission,
        deleteSnapshot,
        toast,
        setToast,
        hubOverview,
        loadingHub,
        refreshHub,
        updatesSummary,
        loadingUpdates,
        refreshUpdates,
        creatorProfiles,
        loadingCreators,
        refreshCreators,
        repositorySyncStatuses,
        loadingRepoSync,
        refreshRepositorySync,
        refreshAllRepositoriesSync,
        switchRepositoryChannel,
        getInstalledPackageHistory,
        activeLockscreen,
        applyLockscreen,
        deactivateLockscreen,
        refreshActiveLockscreen,
        testLockscreen,
        checkConfigDrift,
        privilegedHelperStatus,
        isSettingUpHelper,
        checkPrivilegedHelper,
        setupPrivilegedHelper,
        hostCapabilities,
        runtimeStatus,
        sddmRuntimeStatus,
        systemIntegrationReport,
        loadHostCapabilities,
        loadSystemIntegrationReport,
        settings,
        loadSettings,
        deactivateAndUninstallLockscreen,
        loadRuntimeStatus,
        loadSddmRuntimeStatus,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export const useApp = () => {
  const context = useContext(AppContext);
  if (!context) {
    throw new Error("useApp must be used within an AppProvider");
  }
  return context;
};
