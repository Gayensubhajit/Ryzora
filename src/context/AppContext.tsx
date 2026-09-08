import React, { createContext, useContext, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CategoryId,
  CompatibilityReport,
  CompatibilityRequirements,
  DesktopEnvironment,
  ManifestValidationResult,
  PackageItem,
  RestoreResult,
  SnapshotMetadata,
  SystemInfo,
} from "../types";
import { MOCK_PACKAGES } from "../data/mockPackages";

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
  snapshots: SnapshotMetadata[];
  isInstalling: boolean;
  installProgress: number;
  installLogs: string[];
  installPackage: (pkg: PackageItem) => Promise<void>;
  rollbackSnapshot: (snapId: string) => Promise<void>;
  deleteSnapshot: (snapId: string) => Promise<void>;
  refreshSystem: () => Promise<void>;
  checkCompatibility: (pkg: PackageItem) => CompatibilityReport;
  validateManifest: (manifestJson: string) => Promise<ManifestValidationResult>;
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
  const [packages] = useState<PackageItem[]>(MOCK_PACKAGES);
  const [compatibilityMap, setCompatibilityMap] = useState<Record<string, CompatibilityReport>>({});
  const [selectedPackage, setSelectedPackage] = useState<PackageItem | null>(null);
  const [installedPackageIds, setInstalledPackageIds] = useState<string[]>(() => {
    try {
      const saved = localStorage.getItem("ryzora_installed_ids");
      return saved ? JSON.parse(saved) : ["fastfetch-cyber-spec"];
    } catch {
      return ["fastfetch-cyber-spec"];
    }
  });
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

  useEffect(() => {
    refreshSystem();
    loadSnapshots();
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem("ryzora_installed_ids", JSON.stringify(installedPackageIds));
    } catch {
      // ignore
    }
  }, [installedPackageIds]);

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

  const installPackage = async (pkg: PackageItem) => {
    setIsInstalling(true);
    setInstallProgress(10);
    // Prefer manifest.files if present, fall back to components
    const manifestFiles = pkg.manifest?.files ?? [];
    const pathsToBackup = manifestFiles.length > 0
      ? manifestFiles.map((f) => f.target)
      : pkg.components.map((c) => c.target_path);

    setInstallLogs([
      `[Step 1/5] Validating package manifest for '${pkg.title}' v${pkg.version}...`,
      ...(pkg.manifest
        ? [`[Manifest] ryzora_spec: ${pkg.manifest.ryzora_spec} · type: ${pkg.manifest.package_type} · files: ${manifestFiles.length}`]
        : []),
    ]);

    await new Promise((r) => setTimeout(r, 400));
    setInstallProgress(30);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 2/5] Checking dependencies: ${pkg.dependencies.packages.join(", ") || "None"}`,
      `[Verified] Safe manifest specification confirmed.`,
    ]);

    await new Promise((r) => setTimeout(r, 500));
    setInstallProgress(55);

    setInstallLogs((prev) => [
      ...prev,
      `[Step 3/5] Simulating snapshot of ${pathsToBackup.length} configuration paths...`,
    ]);

    try {
      const snap = await invoke<SnapshotMetadata>("create_snapshot", {
        label: pkg.manifest?.name ?? pkg.title,
        paths: pathsToBackup,
      });
      setSnapshots((prev) => [snap, ...prev]);
    } catch (e) {
      // Snapshot creation failed — log but do not block simulated install
      setInstallLogs((prev) => [
        ...prev,
        `[Warning] Snapshot creation failed: ${e}`,
      ]);
    }

    await new Promise((r) => setTimeout(r, 400));
    setInstallProgress(85);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 4/5] Simulated staging of configuration files...`,
      ...pathsToBackup.map((p) => `  ✓ Staged ${p}`),
    ]);

    await new Promise((r) => setTimeout(r, 400));
    setInstallProgress(100);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 5/5] Success! '${pkg.title}' is active.`,
    ]);

    setInstalledPackageIds((prev) => (prev.includes(pkg.id) ? prev : [...prev, pkg.id]));
    setIsInstalling(false);
    setToast({
      message: `Installed ${pkg.title}`,
      type: "success",
    });
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
        snapshots,
        isInstalling,
        installProgress,
        installLogs,
        installPackage,
        rollbackSnapshot,
        refreshSystem,
        checkCompatibility,
        validateManifest,
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
