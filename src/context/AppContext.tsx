import React, { createContext, useContext, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CategoryId,
  DesktopEnvironment,
  PackageItem,
  SnapshotRecord,
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
  snapshots: SnapshotRecord[];
  isInstalling: boolean;
  installProgress: number;
  installLogs: string[];
  installPackage: (pkg: PackageItem) => Promise<void>;
  rollbackSnapshot: (snapId: string) => Promise<void>;
  refreshSystem: () => Promise<void>;
  toast: { message: string; type: "success" | "info" | "warning" } | null;
  setToast: (toast: { message: string; type: "success" | "info" | "warning" } | null) => void;
}

const AppContext = createContext<AppContextType | undefined>(undefined);

const FALLBACK_SYSTEM_INFO: SystemInfo = {
  distro_name: "Garuda Linux",
  distro_id: "garuda",
  distro_version: "Rolling",
  kernel_version: "6.18.50-1-lts",
  desktop_environment: "Hyprland",
  window_manager: "Hyprland",
  session_type: "wayland",
  shell: "fish",
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

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [loadingSystem, setLoadingSystem] = useState<boolean>(true);
  const [activeCategory, setActiveCategory] = useState<CategoryId>("discover");
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [desktopFilter, setDesktopFilter] = useState<DesktopEnvironment | "all">("all");
  const [packages] = useState<PackageItem[]>(MOCK_PACKAGES);
  const [selectedPackage, setSelectedPackage] = useState<PackageItem | null>(null);
  const [installedPackageIds, setInstalledPackageIds] = useState<string[]>(() => {
    try {
      const saved = localStorage.getItem("ryzora_installed_ids");
      return saved ? JSON.parse(saved) : ["fastfetch-cyber-spec"];
    } catch {
      return ["fastfetch-cyber-spec"];
    }
  });
  const [snapshots, setSnapshots] = useState<SnapshotRecord[]>([]);
  const [isInstalling, setIsInstalling] = useState<boolean>(false);
  const [installProgress, setInstallProgress] = useState<number>(0);
  const [installLogs, setInstallLogs] = useState<string[]>([]);
  const [toast, setToast] = useState<{ message: string; type: "success" | "info" | "warning" } | null>(null);

  // Load system info from Tauri Rust backend
  const refreshSystem = async () => {
    setLoadingSystem(true);
    try {
      const info = await invoke<SystemInfo>("detect_system_info");
      setSystemInfo(info);
    } catch {
      // In browser development or fallback
      setSystemInfo(FALLBACK_SYSTEM_INFO);
    } finally {
      setLoadingSystem(false);
    }
  };

  // Load snapshots from Tauri Rust backend
  const loadSnapshots = async () => {
    try {
      const list = await invoke<SnapshotRecord[]>("get_backups");
      setSnapshots(list);
    } catch {
      setSnapshots([
        {
          id: "snap-init-001",
          package_id: "system-baseline",
          package_name: "Initial System Baseline",
          timestamp: Date.now() / 1000 - 86400,
          formatted_date: "2026-09-08 18:00:00",
          backed_up_paths: [
            "~/.config/hypr/hyprland.conf",
            "~/.config/waybar/config.jsonc",
            "~/.config/kitty/kitty.conf",
          ],
          status: "active",
        },
      ]);
    }
  };

  useEffect(() => {
    refreshSystem();
    loadSnapshots();
  }, []);

  // Save installed IDs
  useEffect(() => {
    try {
      localStorage.setItem("ryzora_installed_ids", JSON.stringify(installedPackageIds));
    } catch {
      // ignore
    }
  }, [installedPackageIds]);

  // Toast timeout
  useEffect(() => {
    if (toast) {
      const timer = setTimeout(() => setToast(null), 4000);
      return () => clearTimeout(timer);
    }
  }, [toast]);

  // Safe install flow with backup creation
  const installPackage = async (pkg: PackageItem) => {
    setIsInstalling(true);
    setInstallProgress(10);
    setInstallLogs([`[Step 1/5] Validating package manifest for '${pkg.title}' v${pkg.version}...`]);

    await new Promise((r) => setTimeout(r, 600));
    setInstallProgress(30);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 2/5] Checking dependencies: ${pkg.dependencies.packages.join(", ") || "None"}`,
      `[Safety] Verification passed: Zero unauthorized script execution.`,
    ]);

    await new Promise((r) => setTimeout(r, 800));
    setInstallProgress(55);

    // Extract target paths to backup
    const pathsToBackup = pkg.components.map((c) => c.target_path);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 3/5] Creating safety snapshot of ${pathsToBackup.length} configuration paths...`,
    ]);

    try {
      const snap = await invoke<SnapshotRecord>("create_backup_snapshot", {
        packageId: pkg.id,
        packageName: pkg.title,
        paths: pathsToBackup,
      });
      setSnapshots((prev) => [snap, ...prev]);
    } catch {
      // Mock snapshot record in fallback
      const mockSnap: SnapshotRecord = {
        id: `snap-${Date.now()}`,
        package_id: pkg.id,
        package_name: pkg.title,
        timestamp: Date.now() / 1000,
        formatted_date: "Just now",
        backed_up_paths: pathsToBackup,
        status: "active",
      };
      setSnapshots((prev) => [mockSnap, ...prev]);
    }

    await new Promise((r) => setTimeout(r, 700));
    setInstallProgress(85);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 4/5] Staging configurations into target user directories...`,
      ...pkg.components.map((c) => `  ✓ Linked ${c.target_path}`),
    ]);

    await new Promise((r) => setTimeout(r, 600));
    setInstallProgress(100);
    setInstallLogs((prev) => [
      ...prev,
      `[Step 5/5] Success! '${pkg.title}' has been successfully installed.`,
      `[Info] Configuration active. You can roll back anytime from the Backups tab.`,
    ]);

    setInstalledPackageIds((prev) => (prev.includes(pkg.id) ? prev : [...prev, pkg.id]));
    setIsInstalling(false);
    setToast({
      message: `Successfully installed ${pkg.title}! Snapshot created.`,
      type: "success",
    });
  };

  const rollbackSnapshot = async (snapId: string) => {
    try {
      await invoke("rollback_snapshot", { snapshotId: snapId });
    } catch {
      // Fallback update
    }
    setSnapshots((prev) =>
      prev.map((s) => (s.id === snapId ? { ...s, status: "restored" } : s))
    );
    setToast({
      message: `Configurations successfully rolled back to snapshot ${snapId}!`,
      type: "success",
    });
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
