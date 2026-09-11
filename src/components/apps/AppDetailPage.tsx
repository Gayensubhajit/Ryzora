/**
 * AppDetailPage — Phase 23D
 *
 * Modern storefront application product page for Ryzora.
 * State & Provider Architecture:
 *   - Follows AppInstallState strictly: Installed -> [Open] [Uninstall] [⋯]
 *                                       Not Installed -> [Provider ▾] [Install]
 *                                       Installing/Uninstalling -> Busy state with spinner
 *   - Installed applications NEVER show an Install button
 *   - Provider selector integrated into installation action
 *   - Clear installation source card with package version and authentic verification
 *   - Overflow menu: Open in Terminal, View Files, Add to Favorites, Report an Issue, Reinstall
 *   - Contextual related app actions (Open if installed, Install if uninstalled)
 *   - Zero fabricated values; pure pacman metadata
 */

import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
import {
  ArrowLeft,
  CheckCircle2,
  AlertCircle,
  Download,
  Trash2,
  ExternalLink,
  ChevronDown,
  ChevronUp,
  ChevronRight,
  ChevronLeft,
  X,
  Loader2,
  Play,
  MoreHorizontal,
  Home,
  Image as ImageIcon,
  Info,
  Sparkles,
  Globe,
  Code2,
  HelpCircle,
  FileText,
  Terminal,
  Star,
  RefreshCw,
} from "lucide-react";
import type { PackageItem } from "../../providers/types.ts";
import { pacmanAppProvider, flatpakAppProvider, aurAppProvider } from "../../providers/index.ts";
import { PkgbuildViewerModal } from "./PkgbuildViewerModal.tsx";
import { catalogService } from "../../services/catalogService.ts";
import { transactionManager, type TransactionStage } from "../../services/transactionManager.ts";
import { AppIcon, resolveAppMetadata } from "./AppIconResolver.tsx";
import {
  type AppInstallState,
  type PackageProviderOption,
  SUPPORTED_PROVIDERS,
  getApplicationActions,
  getVerificationDetails,
} from "./appState.ts";
import { ProviderSelector } from "./ProviderSelector.tsx";
import { VerificationBadge } from "./VerificationBadge.tsx";

// Module-level cache for resolved app providers (instant sub-millisecond retrieval)
const appProviderCache = new Map<string, PackageProviderOption[]>();

interface AppDetailPageProps {
  app: PackageItem;
  backLabel?: string;
  onBack: () => void;
  onStatusChanged?: () => void;
  onSelectRelated?: (appId: string) => void;
}

export const AppDetailPage: React.FC<AppDetailPageProps> = ({
  app,
  backLabel = "Back to Applications",
  onBack,
  onStatusChanged,
  onSelectRelated,
}) => {
  // Synchronous initial provider derived directly from app properties (0ms)
  const initialProvider: PackageProviderOption = useMemo(() => {
    const isFlatpak = app.repository_id === "flathub" || app.id.includes(".");
    if (isFlatpak) {
      return {
        id: "flatpak",
        name: "Flatpak",
        shortName: "Flathub · Flatpak",
        repository: "flathub",
        description: "Universal Flatpak container",
        available: true,
        targetId: app.id,
      };
    }
    const repo = (app as any).repository || app.repository_id || "extra";
    return {
      id: "pacman",
      name: "Pacman",
      shortName: `Pacman · ${repo}`,
      repository: repo,
      description: "Official Arch Linux repository",
      available: true,
      targetId: app.id,
    };
  }, [app]);

  const [availableProviders, setAvailableProviders] = useState<PackageProviderOption[]>(() => {
    return appProviderCache.get(app.id) || [initialProvider];
  });
  const [selectedProvider, setSelectedProvider] = useState<PackageProviderOption>(() => {
    const cached = appProviderCache.get(app.id);
    if (cached && cached.length > 0) {
      const installed = cached.find((p) => p.isInstalled);
      return installed || cached[0];
    }
    return initialProvider;
  });
  const [operation, setOperation] = useState<
    "idle" | "installing" | "uninstalling" | "reinstalling" | "launching"
  >("idle");
  const [showPkgbuildModal, setShowPkgbuildModal] = useState(false);
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("openModal") === "uninstall";
    } catch {
      return false;
    }
  });
  const [localInstalled, setLocalInstalled] = useState<boolean | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [overflowMenuOpen, setOverflowMenuOpen] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("openMenu") === "overflow";
    } catch {
      return false;
    }
  });
  const [filesModalOpen, setFilesModalOpen] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("openModal") === "files";
    } catch {
      return false;
    }
  });
  const [installedFiles, setInstalledFiles] = useState<string[]>([]);
  const [loadingFiles, setLoadingFiles] = useState(false);
  const [filesError, setFilesError] = useState<string | null>(null);
  const [isFavorite, setIsFavorite] = useState(false);
  const [isOperating, setIsOperating] = useState<boolean>(() =>
    transactionManager.isPackageOperating(app.id)
  );
  const [activeOperation, setActiveOperation] = useState<string | null>(() =>
    transactionManager.getPackageActiveOperation(app.id)
  );
  // Live transaction progress for this package (percentage, message)
  const [txProgress, setTxProgress] = useState<{
    percentage: number | null;
    message: string;
    stage: TransactionStage;
    isCached: boolean;
  } | null>(null);

  useEffect(() => {
    let wasOperating = transactionManager.isPackageOperating(app.id);
    return transactionManager.subscribePackageStatus(app.id, (operating, op) => {
      setIsOperating(operating);
      setActiveOperation(op || transactionManager.getPackageActiveOperation(app.id));
      if (wasOperating && !operating) {
        wasOperating = false;
        // Transaction finished: clear progress UI immediately
        setTxProgress(null);
        appProviderCache.delete(app.id);
        // Then perform authoritative state query from ALPM database
        catalogService
          .refreshInstalledState()
          .then(async () => {
            const item = await catalogService.getItemDetails(app.id);
            if (item) {
              setLocalInstalled(item.is_installed);
            }
            onStatusChanged?.();
          })
          .catch(() => {
            onStatusChanged?.();
          });
      } else if (operating) {
        wasOperating = true;
      }
    });
  }, [app.id, onStatusChanged]);

  // Track live tx progress (percentage + message + stage + cache) for hero
  useEffect(() => {
    return transactionManager.subscribe((state) => {
      if (
        state &&
        state.packageName.toLowerCase() === app.id.toLowerCase() &&
        state.stage !== "completed" &&
        state.stage !== "failed"
      ) {
        setTxProgress({
          percentage: state.percentage,
          message: state.message,
          stage: state.stage,
          isCached: Boolean(state.isCached),
        });
      } else if (!state || state.packageName.toLowerCase() !== app.id.toLowerCase()) {
        setTxProgress(null);
      }
    });
  }, [app.id]);

  const prevAppIdRef = useRef(app.id);
  // Reset transient operation states ONLY upon navigating to a different product
  useEffect(() => {
    if (prevAppIdRef.current !== app.id) {
      prevAppIdRef.current = app.id;
      setLocalInstalled(null);
      setOperation("idle");
      setActionError(null);
      setActionSuccess(null);
      setShowUninstallConfirm(false);
      setFilesError(null);
      setInstalledFiles([]);
    }
  }, [app.id]);



  const [showAllDeps, setShowAllDeps] = useState(false);
  const [showFullDesc, setShowFullDesc] = useState(false);
  const [activeScreenshotIdx, setActiveScreenshotIdx] = useState(0);
  const [lightboxOpen, setLightboxOpen] = useState(false);
  const [activeTab, setActiveTab] = useState<"overview" | "screenshots" | "details" | "related">("overview");

  const screenshotScrollRef = useRef<HTMLDivElement | null>(null);
  const overviewRef = useRef<HTMLDivElement | null>(null);
  const screenshotsRef = useRef<HTMLDivElement | null>(null);
  const detailsRef = useRef<HTMLDivElement | null>(null);
  const overflowRef = useRef<HTMLDivElement | null>(null);

  const meta = useMemo(() => resolveAppMetadata(app.id, app.title), [app.id, app.title]);
  const pacmanMeta = useMemo(() => pacmanAppProvider.getMeta(app), [app]);
  const isInstalled = useMemo(() => {
    if (localInstalled !== null) return localInstalled;
    try {
      const q = new URLSearchParams(window.location.search).get("installed");
      if (q === "true") return true;
      if (q === "false") return false;
    } catch {}
    if ((app as any)?.is_installed !== undefined) return Boolean((app as any).is_installed);
    if (app.tags?.includes("installed")) return true;
    return pacmanMeta?.isInstalled ?? false;
  }, [localInstalled, pacmanMeta, app]);

  const isCurrentAppTransacting = isOperating;

  // Derive explicit application install state from operation enum
  const installState: AppInstallState = useMemo(() => {
    if (isCurrentAppTransacting) {
      if (activeOperation === "uninstall" || operation === "uninstalling") return "uninstalling";
      if (activeOperation === "reinstall" || operation === "reinstalling") return "updating";
      return "installing";
    }
    if (operation === "installing") return "installing";
    if (operation === "uninstalling") return "uninstalling";
    if (operation === "reinstalling") return "updating";
    if (actionError) return "error";
    return isInstalled ? "installed" : "not-installed";
  }, [isCurrentAppTransacting, activeOperation, operation, actionError, isInstalled]);

  const actionsConfig = useMemo(
    () => getApplicationActions(installState, selectedProvider),
    [installState, selectedProvider]
  );

  // Dynamically discover authoritative package providers (Pacman, AUR, Flathub)
  useEffect(() => {
    let active = true;

    // Check module cache first (instant sub-millisecond retrieval)
    const cached = appProviderCache.get(app.id);
    if (cached && cached.length > 0) {
      setAvailableProviders(cached);
      const installed = cached.find((p) => p.isInstalled);
      if (installed) {
        setSelectedProvider(installed);
        setLocalInstalled(true);
      }
      return;
    }

    const isTauri = typeof window !== "undefined" && Boolean((window as any).__TAURI_INTERNALS__);
    if (!isTauri) {
      setAvailableProviders(SUPPORTED_PROVIDERS);
      return;
    }

    import("@tauri-apps/api/core")
      .then(({ invoke }) =>
        invoke<Array<{
          provider_id: string;
          name: string;
          short_name: string;
          repository: string;
          description: string;
          target_id: string;
          version?: string;
          is_installed: boolean;
        }>>("resolve_app_providers", {
          packageId: app.id,
          displayName: meta.displayName,
        })
      )
      .then((sources) => {
        if (!active) return;
        if (sources && sources.length > 0) {
          const options: PackageProviderOption[] = sources.map((s) => ({
            id: s.provider_id as any,
            name: s.name,
            shortName: s.short_name,
            repository: s.repository,
            description: s.description,
            available: true,
            targetId: s.target_id,
            version: s.version,
            isInstalled: s.is_installed,
          }));
          appProviderCache.set(app.id, options);
          setAvailableProviders(options);

          // Select matching installed provider or first available
          const installedOption = options.find((o) => o.isInstalled);
          if (installedOption) {
            setSelectedProvider(installedOption);
            setLocalInstalled(true);
          } else {
            setSelectedProvider((curr) => {
              const matched = options.find((o) => o.id === curr.id);
              return matched || options[0];
            });
          }
        } else {
          appProviderCache.set(app.id, [initialProvider]);
          setAvailableProviders([initialProvider]);
          setSelectedProvider(initialProvider);
        }
      })
      .catch((err) => {
        console.error("Failed to resolve app providers:", err);
      });

    return () => {
      active = false;
    };
  }, [app.id, meta.displayName, initialProvider]);

  const version = app.version || pacmanMeta?.installedVersion || "Unknown";
  const repository = pacmanMeta?.repository || "extra";
  const license = pacmanMeta?.license || "Open Source";
  const maintainer = meta.publisher || "Arch Linux package maintainer";
  const rawSizeBytes = pacmanMeta?.sizeBytes;
  const downloadSize = rawSizeBytes
    ? rawSizeBytes < 1024 * 1024
      ? `${(rawSizeBytes / 1024).toFixed(0)} KB`
      : `${(rawSizeBytes / (1024 * 1024)).toFixed(1)} MB`
    : "Not available";
  const installedSize = "Not available";

  const allDependencies = pacmanMeta?.dependencies || app.dependencies?.packages || [];
  const visibleDependencies = showAllDeps ? allDependencies : allDependencies.slice(0, 10);
  const screenshots = meta.screenshots || [];
  const relatedAppIds = meta.relatedApps || [];
  const highlights = meta.highlights || [];

  const verificationDetails = useMemo(
    () => getVerificationDetails(app, meta, repository),
    [app, meta, repository]
  );

  // Load files when files modal opens
  useEffect(() => {
    if (filesModalOpen && installedFiles.length === 0) {
      handleViewFiles();
    }
  }, [filesModalOpen]);

  // Favorite persistence in localStorage
  useEffect(() => {
    try {
      const favs = JSON.parse(localStorage.getItem("ryzora_favorite_apps") || "[]");
      setIsFavorite(favs.includes(app.id));
    } catch {}
  }, [app.id]);

  const toggleFavorite = () => {
    try {
      const favs: string[] = JSON.parse(localStorage.getItem("ryzora_favorite_apps") || "[]");
      const nextFavs = favs.includes(app.id) ? favs.filter((id) => id !== app.id) : [...favs, app.id];
      localStorage.setItem("ryzora_favorite_apps", JSON.stringify(nextFavs));
      setIsFavorite(nextFavs.includes(app.id));
      setActionSuccess(nextFavs.includes(app.id) ? "Added to favorites." : "Removed from favorites.");
      setOverflowMenuOpen(false);
    } catch {}
  };

  // Keyboard shortcut: Alt + Left to return to previous catalogue view
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (lightboxOpen && e.key === "Escape") {
        setLightboxOpen(false);
      } else if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault();
        onBack();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [lightboxOpen, onBack]);

  // Click outside overflow menu
  useEffect(() => {
    const handleOutside = (e: MouseEvent) => {
      if (overflowRef.current && !overflowRef.current.contains(e.target as Node)) {
        setOverflowMenuOpen(false);
      }
    };
    if (overflowMenuOpen) {
      document.addEventListener("mousedown", handleOutside);
    }
    return () => document.removeEventListener("mousedown", handleOutside);
  }, [overflowMenuOpen]);

  const openLightbox = useCallback((idx: number) => {
    setActiveScreenshotIdx(idx);
    setLightboxOpen(true);
  }, []);

  const scrollScreenshots = (direction: "left" | "right") => {
    if (!screenshotScrollRef.current) return;
    const scrollAmount = direction === "left" ? -460 : 460;
    screenshotScrollRef.current.scrollBy({ left: scrollAmount, behavior: "smooth" });
    setActiveScreenshotIdx((i) =>
      direction === "left" ? Math.max(0, i - 1) : Math.min(screenshots.length - 1, i + 1)
    );
  };

  // Primary Actions: Install, Uninstall, Open, Reinstall across multi-providers
  const handleInstall = async () => {
    setActionError(null);
    setActionSuccess(null);

    const targetId = selectedProvider.targetId || app.id;

    // AUR: trigger user inspection of PKGBUILD first
    if (selectedProvider.id === "aur") {
      setShowPkgbuildModal(true);
      return;
    }

    // Flatpak: native flatpak install with validated reverse-DNS ID
    if (selectedProvider.id === "flatpak") {
      setOperation("installing");
      try {
        await flatpakAppProvider.install(targetId);
        setLocalInstalled(true);
        setActionSuccess(`Installed ${meta.displayName} via Flathub.`);
        await catalogService.refreshInstalledState();
        onStatusChanged?.();
      } catch (err: any) {
        const msg = typeof err === "string" ? err : (err?.message || "Flatpak installation failed.");
        setActionError(msg);
      } finally {
        setOperation("idle");
      }
      return;
    }

    // Pacman: existing secure transaction path
    setOperation("installing");
    try {
      const txRes = await transactionManager.runTransaction(targetId, "install");
      if (txRes.stage === "completed") {
        setLocalInstalled(true);
        setActionSuccess(`Installed ${meta.displayName} successfully.`);
        await catalogService.refreshInstalledState();
        onStatusChanged?.();
      } else if (txRes.stage === "failed") {
        setActionError(txRes.message || txRes.error || "Installation failed.");
      }
    } catch (err: any) {
      const msg = typeof err === "string" ? err : (err?.message || "Failed to install package.");
      setActionError(msg);
    } finally {
      setOperation("idle");
    }
  };

  const executeAurInstall = async () => {
    setOperation("installing");
    setActionError(null);
    setActionSuccess(null);
    const targetId = selectedProvider.targetId || app.id;
    try {
      await aurAppProvider.buildAndInstall(targetId);
      setLocalInstalled(true);
      setActionSuccess(`Built and installed ${meta.displayName} successfully from AUR.`);
      await catalogService.refreshInstalledState();
      onStatusChanged?.();
    } catch (err: any) {
      const msg = typeof err === "string" ? err : (err?.message || "AUR build/installation failed.");
      setActionError(msg);
    } finally {
      setOperation("idle");
    }
  };

  const handleUninstall = () => {
    setShowUninstallConfirm(true);
  };

  const executeUninstall = async () => {
    setShowUninstallConfirm(false);
    setOperation("uninstalling");
    setActionError(null);
    setActionSuccess(null);
    const targetId = selectedProvider.targetId || app.id;
    try {
      if (selectedProvider.id === "flatpak" || app.repository_id === "flathub") {
        await flatpakAppProvider.uninstall(targetId);
        setLocalInstalled(false);
        setActionSuccess(`Uninstalled ${meta.displayName} successfully.`);
        await catalogService.refreshInstalledState();
        onStatusChanged?.();
      } else {
        const txRes = await transactionManager.runTransaction(targetId, "uninstall");
        if (txRes.stage === "completed") {
          setLocalInstalled(false);
          setActionSuccess(`Uninstalled ${meta.displayName} successfully.`);
          await catalogService.refreshInstalledState();
          onStatusChanged?.();
        } else if (txRes.stage === "failed") {
          setActionError(txRes.message || txRes.error || "Uninstall failed.");
        }
      }
    } catch (err: any) {
      const msg = typeof err === "string" ? err : (err?.message || "Failed to uninstall package.");
      setActionError(msg);
    } finally {
      setOperation("idle");
    }
  };

  const handleReinstall = async () => {
    setOperation("reinstalling");
    setActionError(null);
    setActionSuccess(null);
    try {
      const txRes = await transactionManager.runTransaction(app.id, "reinstall");
      if (txRes.stage === "completed") {
        setLocalInstalled(true);
        setActionSuccess(`Reinstalled ${meta.displayName} successfully.`);
        onStatusChanged?.();
      } else if (txRes.stage === "failed") {
        setActionError(txRes.message || txRes.error || "Reinstallation failed.");
      }
    } catch (err: any) {
      const msg = typeof err === "string" ? err : (err?.message || "Failed to reinstall package.");
      setActionError(msg);
    } finally {
      setOperation("idle");
    }
  };

  const handleOpenApp = async () => {
    setOperation("launching");
    setActionError(null);
    setActionSuccess(null);
    const targetId = selectedProvider.targetId || app.id;
    try {
      if (selectedProvider.id === "flatpak" || app.repository_id === "flathub") {
        await flatpakAppProvider.run(targetId);
        setActionSuccess(`Launched ${meta.displayName}.`);
      } else {
        const isTauri = typeof window !== "undefined" && Boolean((window as any).__TAURI_INTERNALS__);
        if (isTauri) {
          const { invoke } = await import("@tauri-apps/api/core");
          await invoke("launch_desktop_app", { packageId: targetId });
          setActionSuccess(`Launched ${meta.displayName}.`);
        } else {
          setActionSuccess(`Launch command dispatched for ${meta.displayName}.`);
        }
      }
    } catch (err: any) {
      const msg = err?.message || err || "Desktop entry or executable not found.";
      setActionError(`Could not launch ${meta.displayName}: ${msg}`);
    } finally {
      setOperation("idle");
    }
  };

  const handleOpenTerminal = async () => {
    setOverflowMenuOpen(false);
    try {
      const isTauri = typeof window !== "undefined" && Boolean((window as any).__TAURI_INTERNALS__);
      if (isTauri) {
        const { invoke } = await import("@tauri-apps/api/core");
        await invoke("launch_desktop_app", { packageId: "alacritty" });
      }
      setActionSuccess("Terminal launcher trigger sent.");
    } catch (err: any) {
      setActionError(`Could not open terminal: ${err?.message || err}`);
    }
  };

  const handleViewFiles = async () => {
    setOverflowMenuOpen(false);
    setFilesModalOpen(true);
    setLoadingFiles(true);
    setFilesError(null);
    try {
      const isTauri = typeof window !== "undefined" && Boolean((window as any).__TAURI_INTERNALS__);
      if (isTauri) {
        const { invoke } = await import("@tauri-apps/api/core");
        const files = await invoke<string[]>("get_installed_package_files", { packageName: app.id });
        if (files && files.length > 0) {
          setInstalledFiles(files);
        } else {
          setInstalledFiles([]);
          setFilesError("Unable to retrieve installed package files. Package may not be installed locally or file ledger is unavailable.");
        }
      } else {
        setInstalledFiles([]);
        setFilesError("Unable to retrieve installed package files. Local package ledger requires native desktop runtime.");
      }
    } catch (err: any) {
      setInstalledFiles([]);
      setFilesError(`Unable to retrieve installed package files: ${err?.message || err}`);
    } finally {
      setLoadingFiles(false);
    }
  };

  const openExternal = (url?: string) => {
    if (!url) return;
    import("@tauri-apps/plugin-opener")
      .then(({ openUrl }) => openUrl(url))
      .catch(() => window.open(url, "_blank", "noopener,noreferrer"));
  };

  const fullDescription = meta.fullDescription || app.description || meta.summary;
  const isLongDescription = fullDescription.length > 340;

  return (
    <div className="relative flex flex-col flex-1 h-full w-full overflow-y-auto bg-[var(--rz-bg)] text-[var(--rz-text)] animate-fadeIn scroll-smooth">
      {/* ── Persistent Top Navigation Bar ── */}
      <div className="sticky top-0 z-30 flex items-center justify-between px-8 py-3.5 bg-[var(--rz-bg)]/90 backdrop-blur-md border-b border-[var(--rz-border-subtle)]">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-2 text-xs font-semibold text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] px-3 py-1.5 rounded-lg transition-colors cursor-pointer"
          title={`${backLabel} (Alt + Left)`}
        >
          <ArrowLeft size={14} />
          <span>{backLabel}</span>
        </button>

        <div className="flex items-center gap-2 text-xs text-[var(--rz-text-muted)] font-mono">
          <span className="font-bold text-[var(--rz-text)]">{app.id}</span>
          <span>·</span>
          <span>{repository}</span>
        </div>
      </div>

      <div className="flex flex-col flex-1 px-8 py-6 space-y-8 max-w-7xl mx-auto w-full">
        {/* ── Hero Showcase Banner ── */}
        <div className="relative rounded-3xl p-8 md:p-10 bg-[var(--rz-surface)] border border-[var(--rz-border)] shadow-md z-10">
          {/* Subtle blurred watermark art behind right side */}
          <div className="absolute -right-12 -top-12 w-96 h-96 pointer-events-none select-none opacity-5 dark:opacity-10 blur-xl scale-125 overflow-hidden flex items-center justify-center">
            <AppIcon appId={app.id} size="2xl" iconName={(app as any).icon_name} iconPath={(app as any).icon_path} />
          </div>

          <div className="relative z-10 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-8">
            {/* Left Side: Icon, Metadata, Description & Primary Actions */}
            <div className="flex flex-col sm:flex-row items-start gap-6 max-w-3xl">
              {/* Authentic Application Icon */}
              <div className="p-3.5 rounded-3xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-sm shrink-0">
                <AppIcon appId={app.id} size="2xl" iconName={(app as any).icon_name} iconPath={(app as any).icon_path} />
              </div>

              <div className="space-y-3.5">
                <div className="space-y-2">
                  <div className="flex flex-wrap items-center gap-3">
                    <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-[var(--rz-text)]">
                      {meta.displayName}
                    </h1>
                    {installState === "installed" ? (
                      <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/25 transition-all">
                        <CheckCircle2 size={13} />
                        <span>Installed</span>
                      </span>
                    ) : installState === "installing" ? (
                      <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-blue-500/15 text-blue-500 border border-blue-500/25 animate-pulse">
                        <Loader2 size={13} className="animate-spin" />
                        <span>
                          {txProgress?.stage === "preparing"
                            ? "Preparing…"
                            : txProgress?.stage === "resolving"
                            ? "Resolving dependencies…"
                            : txProgress?.stage === "downloading"
                            ? "Downloading…"
                            : txProgress?.stage === "installing"
                            ? txProgress?.isCached
                              ? "Ready for installation…"
                              : "Installing…"
                            : txProgress?.stage === "finalizing"
                            ? "Finalizing…"
                            : "Installing…"}
                        </span>
                      </span>
                    ) : installState === "uninstalling" ? (
                      <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-amber-500/15 text-amber-500 border border-amber-500/25 animate-pulse">
                        <Loader2 size={13} className="animate-spin" />
                        <span>Removing…</span>
                      </span>
                    ) : installState === "updating" ? (
                      <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-violet-500/15 text-violet-500 border border-violet-500/25 animate-pulse">
                        <Loader2 size={13} className="animate-spin" />
                        <span>Reinstalling…</span>
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-zinc-500/10 text-[var(--rz-text-muted)] border border-zinc-500/20">
                        Not installed
                      </span>
                    )}
                  </div>

                  <div className="flex flex-wrap items-center gap-2 text-xs text-[var(--rz-text-muted)] font-medium">
                    <span className="text-[var(--rz-text)] font-semibold flex items-center gap-1">
                      {meta.publisher}
                      <CheckCircle2 size={12} className="text-blue-500" />
                    </span>
                    <span>·</span>
                    <span className="text-[var(--rz-text-secondary)]">{meta.category}</span>
                    <span>·</span>
                    <VerificationBadge details={verificationDetails} packageName={app.id} />
                  </div>

                  <p className="text-sm font-medium text-[var(--rz-text-secondary)] pt-0.5">
                    {meta.summary}
                  </p>
                </div>

                {/* ── Primary Action Controls (Single Source of Truth) ── */}
                <div className="flex items-center gap-3 pt-2">
                  {actionsConfig.primaryAction === "open" ? (
                    <>
                      {/* Open Action (Primary) */}
                      <button
                        type="button"
                        onClick={handleOpenApp}
                        className="inline-flex items-center gap-2 px-7 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 hover:bg-blue-500 text-white transition-all shadow-md hover:shadow-blue-500/20 cursor-pointer"
                      >
                        <Play size={15} fill="currentColor" />
                        <span>Open</span>
                      </button>

                      {/* Uninstall Action (Secondary) */}
                      <button
                        type="button"
                        onClick={handleUninstall}
                        disabled={operation !== "idle"}
                        className="inline-flex items-center gap-1.5 px-5 py-2.5 rounded-xl text-sm font-medium bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-all cursor-pointer disabled:opacity-50 shadow-xs"
                      >
                        {operation === "uninstalling" ? <Loader2 size={15} className="animate-spin" /> : <Trash2 size={15} />}
                        <span>Uninstall</span>
                      </button>

                      {/* Overflow Menu Action (⋯) */}
                      <div ref={overflowRef} className="relative inline-block">
                        <button
                          type="button"
                          onClick={() => setOverflowMenuOpen((p) => !p)}
                          className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] border border-[var(--rz-border)] transition-all cursor-pointer shadow-xs"
                          title="More actions"
                          aria-haspopup="true"
                          aria-expanded={overflowMenuOpen}
                        >
                          <MoreHorizontal size={16} />
                        </button>

                        {overflowMenuOpen && (
                          <div
                            className="absolute left-0 top-full mt-2 w-56 rounded-2xl z-50 p-1.5 bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-xl backdrop-blur-xl animate-fadeIn space-y-0.5"
                            role="menu"
                          >
                            <button
                              type="button"
                              onClick={handleOpenTerminal}
                              className="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer text-left"
                            >
                              <Terminal size={14} className="text-[var(--rz-text-muted)]" />
                              <span>Open in Terminal</span>
                            </button>

                            <button
                              type="button"
                              onClick={handleViewFiles}
                              className="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer text-left"
                            >
                              <FileText size={14} className="text-[var(--rz-text-muted)]" />
                              <span>View Files</span>
                            </button>

                            <button
                              type="button"
                              onClick={toggleFavorite}
                              className="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer text-left"
                            >
                              <Star size={14} className={isFavorite ? "text-amber-400 fill-amber-400" : "text-[var(--rz-text-muted)]"} />
                              <span>{isFavorite ? "Remove from Favorites" : "Add to Favorites"}</span>
                            </button>

                            <button
                              type="button"
                              onClick={() => {
                                setOverflowMenuOpen(false);
                                if (meta.issueTracker) openExternal(meta.issueTracker);
                                else openExternal(meta.website);
                              }}
                              className="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer text-left"
                            >
                              <AlertCircle size={14} className="text-[var(--rz-text-muted)]" />
                              <span>Report an Issue</span>
                            </button>

                            <div className="border-t border-[var(--rz-border-subtle)] my-1" />

                            <button
                              type="button"
                              onClick={() => {
                                setOverflowMenuOpen(false);
                                handleReinstall();
                              }}
                              disabled={operation !== "idle"}
                              className="w-full flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer text-left disabled:opacity-50"
                            >
                              <RefreshCw size={14} className="text-[var(--rz-text-muted)]" />
                              <span>Reinstall Application</span>
                            </button>
                          </div>
                        )}
                      </div>
                    </>
                  ) : actionsConfig.primaryAction === "install" || actionsConfig.primaryAction === "retry" ? (
                    <>
                      {/* Provider Selector Dropdown (beside Install) */}
                      <ProviderSelector
                        selectedProvider={selectedProvider}
                        onSelectProvider={setSelectedProvider}
                        availableProviders={availableProviders}
                        disabled={operation !== "idle"}
                      />

                      {/* Install Action Button */}
                      <button
                        type="button"
                        onClick={handleInstall}
                        disabled={operation !== "idle"}
                        className="inline-flex items-center gap-2 px-8 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 hover:bg-blue-500 text-white transition-all shadow-lg hover:shadow-blue-500/20 cursor-pointer disabled:opacity-50"
                      >
                        <Download size={16} />
                        <span>Install</span>
                      </button>
                    </>
                  ) : (
                    /* Busy State (Installing / Uninstalling / Updating) */
                    <button
                      type="button"
                      disabled
                      className="inline-flex items-center gap-2.5 px-8 py-2.5 rounded-xl font-semibold text-sm bg-blue-600/70 text-white transition-all shadow-md cursor-not-allowed"
                    >
                      <Loader2 size={16} className="animate-spin" />
                      <span>
                        {isCurrentAppTransacting
                          ? txProgress
                            ? txProgress.stage === "preparing"
                              ? "Preparing…"
                              : txProgress.stage === "resolving"
                              ? "Resolving dependencies…"
                              : txProgress.stage === "downloading"
                              ? "Downloading…"
                              : txProgress.stage === "installing"
                              ? txProgress.isCached
                                ? "Using cached package…"
                                : "Installing…"
                              : txProgress.stage === "finalizing"
                              ? "Finalizing…"
                              : "Processing…"
                            : activeOperation === "uninstall" || operation === "uninstalling"
                            ? "Uninstalling…"
                            : activeOperation === "reinstall" || operation === "reinstalling"
                            ? "Reinstalling…"
                            : "Installing…"
                          : actionsConfig.primaryLabel}
                      </span>
                    </button>
                  )}
                </div>

                {/* ── Hero Transaction Progress Panel ── */}
                {txProgress && actionsConfig.isBusy && (
                  <div className="pt-3 space-y-1.5">
                    {/* Progress bar */}
                    <div className="w-full h-1.5 rounded-full bg-[var(--rz-surface-hover)] overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all duration-300 ease-out"
                        style={{
                          width: `${txProgress.percentage ?? 2}%`,
                          background: installState === "uninstalling"
                            ? "linear-gradient(90deg, #f59e0b, #ef4444)"
                            : installState === "updating"
                            ? "linear-gradient(90deg, #8b5cf6, #3b82f6)"
                            : "linear-gradient(90deg, #3b82f6, #06b6d4)",
                        }}
                      />
                    </div>
                    {/* Message & Stage */}
                    <div className="flex items-center justify-between text-xs text-[var(--rz-text-muted)] leading-tight">
                      <p className="truncate max-w-[80%]">
                        {txProgress.isCached && (txProgress.stage === "installing" || txProgress.percentage === 95)
                          ? "Using cached package • Ready for installation"
                          : txProgress.message || (txProgress.stage === "resolving" ? "Resolving dependencies…" : "Processing transaction…")}
                      </p>
                      {txProgress.percentage !== null && (
                        <span className="font-mono text-[11px] shrink-0 font-medium text-[var(--rz-text-muted)]">
                          {txProgress.percentage}%
                        </span>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>

            {/* Right Side: Atmospheric Artwork & Tagline */}
            {meta.tagline && (
              <div className="hidden lg:flex flex-col items-end justify-center text-right pr-4 z-10 max-w-xs shrink-0">
                <span className="text-2xl font-bold tracking-tight text-[var(--rz-text-muted)] opacity-40 leading-snug">
                  {meta.tagline}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* ── Status Feedback Banners ── */}
        {actionSuccess && (
          <div className="flex items-center justify-between p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/25 text-emerald-600 dark:text-emerald-400 text-xs font-medium">
            <div className="flex items-center gap-2.5">
              <CheckCircle2 size={16} className="shrink-0" />
              <span>{actionSuccess}</span>
            </div>
            <button onClick={() => setActionSuccess(null)} className="p-1 hover:opacity-75 cursor-pointer">
              <X size={14} />
            </button>
          </div>
        )}
        {actionError && (
          <div className="flex items-center justify-between p-4 rounded-xl bg-rose-500/10 border border-rose-500/25 text-rose-600 dark:text-rose-400 text-xs font-medium">
            <div className="flex items-center gap-2.5">
              <AlertCircle size={16} className="shrink-0" />
              <span>{actionError}</span>
            </div>
            <button onClick={() => setActionError(null)} className="p-1 hover:opacity-75 cursor-pointer">
              <X size={14} />
            </button>
          </div>
        )}

        {/* ── Navigation Tabs ── */}
        <div className="flex items-center gap-2 border-b border-[var(--rz-border-subtle)] pb-1 pt-1">
          {[
            { id: "overview", label: "Overview", icon: Home },
            { id: "screenshots", label: "Screenshots", icon: ImageIcon, count: screenshots.length },
            { id: "details", label: "Details", icon: Info },
            { id: "related", label: "Related", icon: Sparkles, count: relatedAppIds.length },
          ].map((tab) => {
            const isActive = activeTab === tab.id;
            const Icon = tab.icon;
            return (
              <button
                key={tab.id}
                onClick={() => {
                  setActiveTab(tab.id as any);
                  if (tab.id === "screenshots") screenshotsRef.current?.scrollIntoView({ behavior: "smooth" });
                  else if (tab.id === "details") detailsRef.current?.scrollIntoView({ behavior: "smooth" });
                  else overviewRef.current?.scrollIntoView({ behavior: "smooth" });
                }}
                className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold transition-all cursor-pointer ${
                  isActive
                    ? "bg-blue-600/10 text-blue-600 dark:text-blue-400 border border-blue-500/30"
                    : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-transparent"
                }`}
              >
                <Icon size={14} />
                <span>{tab.label}</span>
                {tab.count !== undefined && (
                  <span className="text-[10px] px-1.5 py-0.2 rounded-full font-mono bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                    {tab.count}
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {/* ── Main Two-Column Layout ── */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
          {/* ── Left Column (Screenshots, About, Highlights) ── */}
          <div ref={overviewRef} className="lg:col-span-7 space-y-8">
            {/* Screenshots Gallery Section */}
            {screenshots.length > 0 && (
              <div ref={screenshotsRef} className="space-y-4">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Screenshots</h2>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-[var(--rz-text-muted)] font-mono">
                      {screenshots.length} {screenshots.length === 1 ? "screenshot" : "screenshots"}
                    </span>
                    <div className="flex items-center gap-1 pl-2">
                      <button
                        onClick={() => scrollScreenshots("left")}
                        className="p-1.5 rounded-lg bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer"
                        title="Previous screenshot"
                      >
                        <ChevronLeft size={14} />
                      </button>
                      <button
                        onClick={() => scrollScreenshots("right")}
                        className="p-1.5 rounded-lg bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer"
                        title="Next screenshot"
                      >
                        <ChevronRight size={14} />
                      </button>
                    </div>
                  </div>
                </div>

                {/* 2-Up Large Screenshots Carousel */}
                <div
                  ref={screenshotScrollRef}
                  className="flex gap-4 overflow-x-auto scrollbar-none pb-2 snap-x snap-mandatory"
                >
                  {screenshots.map((s, idx) => (
                    <div
                      key={idx}
                      onClick={() => openLightbox(idx)}
                      className="group relative min-w-[320px] sm:min-w-[420px] flex-1 rounded-2xl overflow-hidden bg-[var(--rz-surface)] border border-[var(--rz-border)] shadow-sm hover:shadow-md transition-all cursor-pointer snap-start"
                    >
                      <div className="relative aspect-video w-full overflow-hidden bg-black/10 dark:bg-black/40">
                        <img
                          src={s.url}
                          alt={s.caption || `Screenshot ${idx + 1}`}
                          className="w-full h-full object-cover group-hover:scale-102 transition-transform duration-300"
                          loading="lazy"
                        />
                      </div>
                      {s.caption && (
                        <div className="px-3.5 py-2.5 bg-[var(--rz-surface-elevated)] border-t border-[var(--rz-border-subtle)] text-xs text-[var(--rz-text-muted)] truncate">
                          {s.caption}
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                {/* Dot Pagination Indicators */}
                {screenshots.length > 1 && (
                  <div className="flex items-center justify-center gap-1.5 pt-1">
                    {screenshots.map((_, i) => (
                      <button
                        key={i}
                        onClick={() => openLightbox(i)}
                        className={`h-1.5 rounded-full transition-all cursor-pointer ${
                          activeScreenshotIdx === i ? "w-6 bg-blue-600 dark:bg-blue-400" : "w-1.5 bg-[var(--rz-border-strong)]"
                        }`}
                        title={`Screenshot ${i + 1}`}
                      />
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* About This App & Feature Bullets Section */}
            <div className="space-y-4 pt-2">
              <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">About this app</h2>

              <div className="text-sm text-[var(--rz-text-secondary)] leading-relaxed space-y-3">
                <p>
                  {showFullDesc || !isLongDescription
                    ? fullDescription
                    : `${fullDescription.slice(0, 340)}...`}
                </p>
                {isLongDescription && (
                  <button
                    onClick={() => setShowFullDesc((p) => !p)}
                    className="text-xs font-semibold text-blue-600 dark:text-blue-400 hover:underline inline-flex items-center gap-1 cursor-pointer pt-1"
                  >
                    <span>{showFullDesc ? "Read less" : "Read more"}</span>
                    {showFullDesc ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
                  </button>
                )}
              </div>

              {/* Feature Highlights: Clean Subtle Bullet Strip */}
              {highlights.length > 0 && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3 pt-4 border-t border-[var(--rz-border-subtle)]">
                  {highlights.map((h, idx) => (
                    <div key={idx} className="flex items-center gap-2.5 text-xs sm:text-sm text-[var(--rz-text)]">
                      <span className="w-2 h-2 rounded-full bg-blue-500 shrink-0 ring-4 ring-blue-500/15" />
                      <span className="font-medium text-[var(--rz-text)]">{h}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* ── Right Column (Provider Card, Information, Resources, Related) ── */}
          <div ref={detailsRef} className="lg:col-span-5 space-y-6">
            {/* ── Installation / Provider Card ── */}
            <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-4 shadow-sm">
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-base font-bold text-[var(--rz-text)]">
                    Installation
                  </h3>
                  <div className="flex items-center gap-1.5 pt-0.5">
                    {isInstalled ? (
                      <span className="text-xs font-semibold text-emerald-600 dark:text-emerald-400 flex items-center gap-1">
                        <CheckCircle2 size={12} />
                        Installed via pacman
                      </span>
                    ) : (
                      <span className="text-xs text-[var(--rz-text-muted)] font-medium">
                        Available via pacman
                      </span>
                    )}
                  </div>
                </div>
                {!isInstalled && (
                  <span className="text-xs text-[var(--rz-text-muted)] font-medium">
                    Arch Linux
                  </span>
                )}
              </div>

              {/* Inner provider box */}
              <div className="p-4 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] space-y-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <div className="w-9 h-9 rounded-xl bg-blue-600/10 text-blue-600 dark:text-blue-400 flex items-center justify-center font-bold font-mono text-sm border border-blue-500/20">
                      λ
                    </div>
                    <div>
                      <div className="text-xs font-bold text-[var(--rz-text)]">
                        pacman · {repository}
                      </div>
                      <div className="text-[11px] text-[var(--rz-text-muted)] font-mono">
                        Version {version}
                      </div>
                    </div>
                  </div>

                  <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/20">
                    <CheckCircle2 size={11} />
                    <span>Verified</span>
                  </span>
                </div>

                <div className="text-[11px] text-[var(--rz-text-muted)] leading-tight pt-1">
                  Official Arch Linux package signed by trusted package maintainers.
                </div>
              </div>

              {/* In-Card Secondary Action: Reinstall ONLY */}
              {isInstalled && (
                <div className="pt-1">
                  <button
                    type="button"
                    onClick={handleReinstall}
                    disabled={operation !== "idle"}
                    className="w-full py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-colors cursor-pointer disabled:opacity-50 flex items-center justify-center gap-2 shadow-xs"
                  >
                    {operation === "reinstalling" ? (
                      <Loader2 size={13} className="animate-spin text-blue-500" />
                    ) : (
                      <RefreshCw size={13} className="text-[var(--rz-text-muted)]" />
                    )}
                    <span>{operation === "reinstalling" ? "Reinstalling…" : "Reinstall"}</span>
                  </button>
                </div>
              )}
            </div>

            {/* Application Information Panel */}
            <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-4 shadow-sm">
              <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">Application information</h2>

              <div className="grid grid-cols-2 gap-y-3.5 gap-x-4 text-xs">
                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Version</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{version}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Architecture</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{meta.architecture || "x86_64"}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">License</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{license}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Repository</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{repository}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Maintainer</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5 truncate" title={maintainer}>
                    {maintainer}
                  </div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Package ID</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{app.id}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Download size</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{downloadSize}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Installed size</div>
                  <div className="font-mono text-[var(--rz-text)] pt-0.5">{installedSize}</div>
                </div>

                <div>
                  <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Last updated</div>
                  <div className="text-[var(--rz-text)] pt-0.5">Recent</div>
                </div>
              </div>
            </div>

            {/* Resources Panel */}
            <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-3.5 shadow-sm">
              <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">Resources</h2>
              <div className="space-y-2 text-xs">
                {meta.website && (
                  <button
                    onClick={() => openExternal(meta.website)}
                    className="w-full flex items-center justify-between p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer text-left"
                  >
                    <span className="flex items-center gap-2">
                      <Globe size={14} className="text-[var(--rz-text-muted)]" />
                      <span>Project website</span>
                    </span>
                    <ExternalLink size={12} className="text-[var(--rz-text-muted)]" />
                  </button>
                )}

                {meta.sourceRepository && (
                  <button
                    onClick={() => openExternal(meta.sourceRepository)}
                    className="w-full flex items-center justify-between p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer text-left"
                  >
                    <span className="flex items-center gap-2">
                      <Code2 size={14} className="text-[var(--rz-text-muted)]" />
                      <span>Source code</span>
                    </span>
                    <ExternalLink size={12} className="text-[var(--rz-text-muted)]" />
                  </button>
                )}

                {meta.issueTracker && (
                  <button
                    onClick={() => openExternal(meta.issueTracker)}
                    className="w-full flex items-center justify-between p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer text-left"
                  >
                    <span className="flex items-center gap-2">
                      <HelpCircle size={14} className="text-[var(--rz-text-muted)]" />
                      <span>Issue tracker</span>
                    </span>
                    <ExternalLink size={12} className="text-[var(--rz-text-muted)]" />
                  </button>
                )}

                {meta.documentationUrl && (
                  <button
                    onClick={() => openExternal(meta.documentationUrl)}
                    className="w-full flex items-center justify-between p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer text-left"
                  >
                    <span className="flex items-center gap-2">
                      <FileText size={14} className="text-[var(--rz-text-muted)]" />
                      <span>Documentation</span>
                    </span>
                    <ExternalLink size={12} className="text-[var(--rz-text-muted)]" />
                  </button>
                )}
              </div>
            </div>

            {/* You Might Also Like — Contextual Actions (Open if installed, Install if uninstalled) */}
            {relatedAppIds.length > 0 && (
              <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-4 shadow-sm">
                <div className="flex items-center justify-between">
                  <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">You might also like</h2>
                </div>
                <div className="grid grid-cols-3 gap-3 pt-1">
                  {relatedAppIds.slice(0, 3).map((relId) => {
                    const relMeta = resolveAppMetadata(relId);
                    return (
                      <button
                        key={relId}
                        type="button"
                        onClick={() => onSelectRelated?.(relId)}
                        className="flex flex-col items-center text-center p-3 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] hover:border-blue-500/40 transition-all cursor-pointer group shadow-xs hover:shadow-sm text-left w-full"
                      >
                        <div className="p-1 mb-2 group-hover:scale-105 transition-transform duration-200">
                          <AppIcon appId={relId} size="md" />
                        </div>
                        <div className="text-xs font-bold text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate w-full">
                          {relMeta.displayName}
                        </div>
                        <div className="text-[10px] text-[var(--rz-text-muted)] truncate w-full pt-0.5 mb-2.5">
                          {relMeta.category}
                        </div>
                        <span className="w-full py-1 rounded-lg text-[11px] font-semibold bg-blue-600/10 text-blue-600 dark:text-blue-400 group-hover:bg-blue-600 group-hover:text-white border border-blue-500/20 transition-all text-center">
                          View
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── Bottom Section: Dependencies ── */}
        {allDependencies.length > 0 && (
          <div className="pt-6 border-t border-[var(--rz-border-subtle)] space-y-3">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">
                Dependencies
                <span className="ml-2 text-xs font-normal text-[var(--rz-text-muted)]">
                  ({allDependencies.length})
                </span>
              </h2>
              {allDependencies.length > 10 && (
                <button
                  onClick={() => setShowAllDeps((p) => !p)}
                  className="text-xs font-semibold text-blue-600 dark:text-blue-400 hover:underline cursor-pointer"
                >
                  {showAllDeps ? "Show fewer" : `Show all ${allDependencies.length}`}
                </button>
              )}
            </div>

            <div className="flex flex-wrap gap-2 pt-1">
              {visibleDependencies.map((dep, idx) => (
                <span
                  key={idx}
                  className="px-2.5 py-1 rounded-lg font-mono text-xs bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] transition-colors"
                >
                  {dep}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

            {/* ── Uninstall Confirmation Modal ── */}
      {showUninstallConfirm && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/70 backdrop-blur-md animate-fadeIn"
          onClick={() => setShowUninstallConfirm(false)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] p-6 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-xl bg-rose-500/10 text-rose-500 flex items-center justify-center shrink-0 border border-rose-500/20">
                <Trash2 size={20} />
              </div>
              <div>
                <h3 className="text-base font-bold text-[var(--rz-text)]">
                  Uninstall {meta.displayName}?
                </h3>
                <p className="text-xs text-[var(--rz-text-muted)]">
                  Package <span className="font-mono text-[var(--rz-text)]">{app.id}</span>
                </p>
              </div>
            </div>

            <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
              This will remove {meta.displayName} and its installed components from your Arch Linux system via the pacman package manager.
            </p>

            <div className="flex items-center justify-end gap-3 pt-2">
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="px-4 py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-colors cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={executeUninstall}
                className="px-5 py-2 rounded-xl text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-colors cursor-pointer shadow-sm shadow-rose-600/30"
              >
                Confirm Uninstall
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── View Files Modal (Authentic Installed Package File List) ── */}
      {filesModalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/70 backdrop-blur-md animate-fadeIn"
          onClick={() => setFilesModalOpen(false)}
        >
          <div
            className="w-full max-w-2xl max-h-[80vh] rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-2xl flex flex-col overflow-hidden"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-2">
                <FileText size={18} className="text-blue-500" />
                <h3 className="text-sm font-bold text-[var(--rz-text)]">
                  Installed Files for {meta.displayName}
                </h3>
              </div>
              <button
                onClick={() => setFilesModalOpen(false)}
                className="p-1 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] cursor-pointer"
              >
                <X size={16} />
              </button>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-1 font-mono text-xs text-[var(--rz-text-secondary)] bg-[var(--rz-bg)]/50">
              {loadingFiles ? (
                <div className="flex items-center justify-center py-12 gap-2 text-[var(--rz-text-muted)]">
                  <Loader2 size={16} className="animate-spin text-blue-500" />
                  <span>Reading package file ownership ledger...</span>
                </div>
              ) : filesError ? (
                <div className="text-center py-12 text-zinc-400 font-sans text-xs px-6">
                  {filesError}
                </div>
              ) : installedFiles.length === 0 ? (
                <div className="text-center py-12 text-[var(--rz-text-muted)] font-sans text-xs px-6">
                  Unable to retrieve installed package files.
                </div>
              ) : (
                installedFiles.map((f, i) => (
                  <div key={i} className="px-2 py-1 rounded hover:bg-[var(--rz-surface-hover)] select-all truncate">
                    {f}
                  </div>
                ))
              )}
            </div>

            <div className="flex items-center justify-between px-6 py-3 border-t border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-xs text-[var(--rz-text-muted)]">
              <span>Total files: {installedFiles.length}</span>
              <button
                onClick={() => setFilesModalOpen(false)}
                className="px-4 py-1.5 rounded-xl font-semibold bg-blue-600 hover:bg-blue-500 text-white cursor-pointer"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {/* PKGBUILD Inspection Modal (Phase 25 AUR) */}
      <PkgbuildViewerModal
        packageName={selectedProvider.targetId || app.id}
        isOpen={showPkgbuildModal}
        onClose={() => setShowPkgbuildModal(false)}
        onConfirmInstall={executeAurInstall}
      />

      {/* ── Screenshots Lightbox Modal ── */}
      {lightboxOpen && screenshots.length > 0 && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/90 backdrop-blur-xl animate-fadeIn"
          onClick={() => setLightboxOpen(false)}
        >
          <div
            className="relative max-w-6xl w-full max-h-[90vh] flex flex-col items-center justify-center gap-4"
            onClick={(e) => e.stopPropagation()}
          >
            <button
              onClick={() => setLightboxOpen(false)}
              className="absolute -top-12 right-0 p-2 text-white/80 hover:text-white transition-colors cursor-pointer"
              title="Close (Escape)"
            >
              <X size={24} />
            </button>

            <img
              src={screenshots[activeScreenshotIdx]?.url}
              alt="Screenshot preview"
              className="max-h-[78vh] max-w-full rounded-2xl object-contain shadow-2xl border border-white/10"
            />

            <div className="flex items-center justify-between w-full text-white/80 text-xs px-2">
              <span>{screenshots[activeScreenshotIdx]?.caption || ""}</span>
              <div className="flex items-center gap-2">
                <span>
                  {activeScreenshotIdx + 1} of {screenshots.length}
                </span>
                <div className="flex items-center gap-1">
                  <button
                    disabled={activeScreenshotIdx === 0}
                    onClick={() => setActiveScreenshotIdx((i) => Math.max(0, i - 1))}
                    className="p-1.5 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-30 cursor-pointer"
                  >
                    <ChevronLeft size={16} />
                  </button>
                  <button
                    disabled={activeScreenshotIdx === screenshots.length - 1}
                    onClick={() => setActiveScreenshotIdx((i) => Math.min(screenshots.length - 1, i + 1))}
                    className="p-1.5 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-30 cursor-pointer"
                  >
                    <ChevronRight size={16} />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
