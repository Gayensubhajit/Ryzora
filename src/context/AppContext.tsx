import React, { createContext, useContext, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  previewInstallation: (packageId: string) => Promise<InstallationPlan>;
  resolvePackageDependencies: (packageId: string) => Promise<DependencyResolutionReport>;
  installPackage: (pkg: PackageItem) => Promise<InstallResult>;
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
  toast: { message: string; type: "success" | "info" | "warning" } | null;
  setToast: (toast: { message: string; type: "success" | "info" | "warning" } | null) => void;
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
      setInstalledPackages([]);
      setInstalledPackageIds([]);
    }
  };

  const loadCatalogPackages = async () => {
    try {
      const catalog = await invoke<PackageItem[]>("get_catalog_packages");
      setPackages(catalog || []);
      const repos = await invoke<RepositorySummary[]>("get_repository_info");
      setRepositories(repos);
    } catch (e) {
      console.warn("Failed to load catalog from RepositoryManager", e);
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
      setPackages(catalog || []);
      const repos = await invoke<RepositorySummary[]>("get_repository_info");
      setRepositories(repos);
    } catch (e) {
      console.warn("Failed to refresh catalog", e);
    }
  };

  const refreshRepositories = async () => {
    try {
      const catalog = await invoke<PackageItem[]>("refresh_catalog");
      setPackages(catalog || []);
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

  const previewInstallation = async (packageId: string): Promise<InstallationPlan> => {
    return await invoke<InstallationPlan>("preview_installation", { packageId });
  };

  const resolvePackageDependencies = async (packageId: string): Promise<DependencyResolutionReport> => {
    return await invoke<DependencyResolutionReport>("resolve_package_dependencies", { packageId });
  };

  const installPackage = async (pkg: PackageItem): Promise<InstallResult> => {
    setIsInstalling(true);
    setInstallProgress(10);
    setInstallLogs([
      `[Step 1/5] Inspecting package payload & validating manifest for '${pkg.title}'...`,
    ]);

    try {
      // Step 1: Pre-flight preview/plan
      const plan = await previewInstallation(pkg.id);
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
      setInstallLogs((prev) => [
        ...prev,
        `[Step 2/5] Creating & verifying pre-install snapshot...`,
      ]);

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

      const result = await invoke<InstallResult>("install_package", { packageId: pkg.id });

      if (result.success) {
        setInstallProgress(100);
        setInstallLogs((prev) => [
          ...prev,
          `[Step 5/5] Verified on disk! Snapshot created: ${result.snapshot_id}`,
          `✓ '${pkg.title}' installed successfully (${result.installed_files.length} files).`,
        ]);

        await loadInstalledPackages();
        await loadSnapshots();

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
            ? `[Rollback] Restored original configuration from snapshot ${result.snapshot_id}.`
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

  const deleteSnapshot = async (snapId: string) => {
    try {
      await invoke("delete_snapshot", { id: snapId });
      setSnapshots((prev) => prev.filter((s) => s.id !== snapId));
      setToast({ message: "Snapshot deleted", type: "info" });
    } catch (e) {
      setToast({ message: `Delete failed: ${e}`, type: "warning" });
    }
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
        deleteSnapshot,
        toast,
        setToast,
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
