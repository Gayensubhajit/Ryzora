import React, { useState } from "react";
import { X, Eye, FileCode, FolderArchive, Loader2, DownloadCloud } from "lucide-react";
import { useApp } from "../context/AppContext";
import { InstallationPlan } from "../types";
import { ProductDetailShell } from "../components/detail/ProductDetailShell";
import { ProductHero } from "../components/detail/ProductHero";
import { CompatibilityPanel } from "../components/detail/CompatibilityPanel";
import { DetailTabs } from "../components/detail/DetailTabs";
import { resolveLockscreenCapabilities } from "../providers/qylockProvider";
import { StickyActionBar } from "../components/detail/StickyActionBar";

export const LockScreenDetailView: React.FC = () => {
  const {
    selectedPackage,
    setSelectedPackage,
    systemInfo,
    installedPackages,
    installPackage,
    previewInstallation,
    isInstalling,
    installProgress,
    setToast,
    activeLockscreen,
    applyLockscreen,
    deactivateLockscreen,
    hostCapabilities,
  } = useApp();

  const [activeScreenshotIndex, setActiveScreenshotIndex] = useState(0);
  const [showFullscreen, setShowFullscreen] = useState(false);
  const [showDryRun, setShowDryRun] = useState(false);
  const [dryRunPlan, setDryRunPlan] = useState<InstallationPlan | null>(null);
  const [loadingPlan, setLoadingPlan] = useState(false);
  const [createSnapshot, setCreateSnapshot] = useState(true);

  if (!selectedPackage) return null;

  const isLockscreen =
    selectedPackage.category === "lockscreens" ||
    selectedPackage.package_type === "lockscreen";

  if (!isLockscreen) return null;

  // Protocol & Desktop capability detection
  const isKde =
    systemInfo?.desktop_environment?.toLowerCase().includes("kde") ||
    systemInfo?.desktop_environment?.toLowerCase().includes("plasma");
  const isWayland = systemInfo?.session_type?.toLowerCase() === "wayland";
  const qsAdapter = hostCapabilities?.supported_adapters?.find(a => a.adapter === "quickshell");
  const isSessionLockSupported = qsAdapter ? qsAdapter.supported : (isWayland && !isKde);

  const [selectedTarget, setSelectedTarget] = useState<"quickshell" | "sddm" | "both">(() => {
    if (!isSessionLockSupported && selectedPackage.supports_login_screen) return "sddm";
    if (selectedPackage.supports_session_lock) return "quickshell";
    if (selectedPackage.supports_login_screen) return "sddm";
    return "quickshell";
  });

  const installedRecord = installedPackages.find(
    (p) => p.package_id === selectedPackage.id
  );
  const isInstalled = !!installedRecord;
  const isUpdateAvailable =
    isInstalled && installedRecord
      ? parseFloat(selectedPackage.version) > parseFloat(installedRecord.version)
      : false;

  const [isApplying, setIsApplying] = useState(false);

  const pkgSlug = selectedPackage.id
    .replace(/^lockscreen-qylock-/, "")
    .replace(/^lockscreen-/, "");

  const isQuickshellActive =
    Boolean(activeLockscreen?.quickshell) &&
    (activeLockscreen.quickshell === selectedPackage.id ||
      activeLockscreen.quickshell === `lockscreen-qylock-${pkgSlug}` ||
      activeLockscreen.quickshell === pkgSlug);

  const isSddmActive =
    Boolean(activeLockscreen?.sddm) &&
    (activeLockscreen.sddm === selectedPackage.id ||
      activeLockscreen.sddm === `lockscreen-qylock-${pkgSlug}` ||
      activeLockscreen.sddm === pkgSlug ||
      activeLockscreen.sddm === `ryzora-${pkgSlug}`);

  const isActiveForTarget =
    selectedTarget === "quickshell"
      ? isQuickshellActive
      : selectedTarget === "sddm"
      ? isSddmActive
      : isQuickshellActive && isSddmActive;

  const handleApply = async () => {
    setIsApplying(true);
    try {
      await applyLockscreen(selectedPackage.id, selectedTarget);
    } catch {
      // toast is handled in AppContext
    } finally {
      setIsApplying(false);
    }
  };

  const handleDeactivate = async () => {
    setIsApplying(true);
    try {
      await deactivateLockscreen(selectedTarget);
    } catch {
      // toast is handled in AppContext
    } finally {
      setIsApplying(false);
    }
  };

  const handleClose = () => {
    setSelectedPackage(null);
  };

  const targetCapabilities = selectedPackage.lockscreen && systemInfo
    ? resolveLockscreenCapabilities(systemInfo, selectedPackage.lockscreen, selectedTarget)
    : null;
  const missingDependencies = targetCapabilities?.missing_dependencies || [];

  // Dry run preview handler
  const handleOpenPreview = async () => {
    setLoadingPlan(true);
    setShowDryRun(true);
    try {
      const plan = await previewInstallation(selectedPackage.id, selectedTarget);
      setDryRunPlan(plan);
    } catch (e: any) {
      const msg = e?.message || String(e);
      setDryRunPlan(null);
      setToast({
        message: `Dry-run preview failed: ${msg}`,
        type: "warning",
      });
    } finally {
      setLoadingPlan(false);
    }
  };

  // Install handler
  const handleInstall = async () => {
    try {
      const res = await installPackage(selectedPackage, createSnapshot, selectedTarget);
      if (res && res.success) {
        setToast({
          message: `${selectedPackage.title} installed successfully`,
          type: "success",
        });
      }
    } catch (e: any) {
      const msg = e?.message || String(e);
      setToast({
        message: `Installation failed: ${msg}`,
        type: "warning",
      });
    }
  };

  const screenshots =
    selectedPackage.screenshots && selectedPackage.screenshots.length > 0
      ? selectedPackage.screenshots
      : selectedPackage.hero_image
      ? [selectedPackage.hero_image]
      : [];

  const activeImage = screenshots[activeScreenshotIndex] || selectedPackage.hero_image;

  return (
    <ProductDetailShell
      onClose={handleClose}
      bgImage={selectedPackage.hero_image}
      categoryLabel="Lock Screens"
    >
      {/* ── Two-Column Hero (60-65% Preview / 35-40% Identity) ── */}
      <ProductHero
        packageItem={selectedPackage}
        activeScreenshotIndex={activeScreenshotIndex}
        onSelectScreenshot={setActiveScreenshotIndex}
        onPreview={handleOpenPreview}
        onInstall={handleInstall}
        onOpenLightbox={() => setShowFullscreen(true)}
        isInstalling={isInstalling}
        isInstalled={isInstalled}
        isUpdateAvailable={isUpdateAvailable}
        isBlocked={false}
        missingDependencies={missingDependencies}
        selectedTarget={selectedTarget}
        onSelectTarget={setSelectedTarget}
        isSessionLockSupported={isSessionLockSupported}
        isActive={isActiveForTarget}
        activeTargets={{ quickshell: isQuickshellActive, sddm: isSddmActive }}
        onApply={handleApply}
        onDeactivate={handleDeactivate}
        isApplying={isApplying}
      />

      {/* ── Prominent Compatibility / Safety Model Banner ── */}
      <CompatibilityPanel
        packageItem={selectedPackage}
        systemInfo={systemInfo}
        missingDependencies={missingDependencies}
        selectedTarget={selectedTarget}
        isSessionLockSupported={isSessionLockSupported}
      />

      {/* ── Technical Detail Tabs ── */}
      <DetailTabs
        packageItem={selectedPackage}
        systemInfo={systemInfo}
        selectedTarget={selectedTarget}
      />

      {/* ── Sticky Bottom Action Bar ── */}
      <StickyActionBar
        packageItem={selectedPackage}
        isInstalled={isInstalled}
        isUpdateAvailable={isUpdateAvailable}
        isInstalling={isInstalling}
        installProgress={installProgress}
        missingDependencies={missingDependencies}
        selectedTarget={selectedTarget}
        onInstall={handleInstall}
        onPreview={handleOpenPreview}
        isActive={isActiveForTarget}
        onApply={handleApply}
        onDeactivate={handleDeactivate}
        isApplying={isApplying}
      />

      {/* ── Fullscreen Lightbox Image Viewer ── */}
      {showFullscreen && activeImage && (
        <div
          className="fixed inset-0 z-50 bg-black/95 flex flex-col items-center justify-center p-4 backdrop-blur-md animate-in fade-in duration-150"
          onClick={() => setShowFullscreen(false)}
        >
          <button
            type="button"
            onClick={() => setShowFullscreen(false)}
            className="absolute top-4 right-4 p-2 rounded-xl bg-white/10 hover:bg-white/20 text-white border border-white/20 cursor-pointer select-none"
            aria-label="Close fullscreen image"
          >
            <X className="w-5 h-5" />
          </button>

          <img
            src={activeImage}
            alt={selectedPackage.title}
            className="max-h-[90vh] max-w-[92vw] object-contain rounded-xl shadow-2xl border border-white/10"
            onClick={(e) => e.stopPropagation()}
          />

          <div className="absolute bottom-4 inset-x-0 text-center text-xs text-white/70 pointer-events-none">
            {selectedPackage.title} · Screenshot {activeScreenshotIndex + 1} of {screenshots.length} · Press Esc to close
          </div>
        </div>
      )}

      {/* ── Dry-Run Installation Preview Dialog ── */}
      {showDryRun && (
        <div
          className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4 backdrop-blur-sm animate-in fade-in duration-150"
          onClick={() => setShowDryRun(false)}
        >
          <div
            className="w-full max-w-xl rounded-2xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] p-5 shadow-2xl text-xs space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-3">
              <div className="flex items-center gap-2 font-bold text-sm text-[var(--rz-text)]">
                <Eye className="w-4 h-4 text-[var(--rz-accent)]" />
                <span>Dry-Run Installation Preview ({selectedTarget.toUpperCase()})</span>
              </div>
              <button
                type="button"
                onClick={() => setShowDryRun(false)}
                className="p-1 rounded-md text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {loadingPlan ? (
              <div className="py-8 flex flex-col items-center justify-center gap-2 text-[var(--rz-text-secondary)]">
                <Loader2 className="w-5 h-5 animate-spin text-[var(--rz-accent)]" />
                <span>Analyzing package manifest and filesystem targets...</span>
              </div>
            ) : dryRunPlan ? (
              <div className="space-y-3">
                <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] grid grid-cols-2 gap-2 text-[11px]">
                  <div>
                    <span className="text-[var(--rz-text-muted)] block">Target Package:</span>
                    <span className="font-semibold text-[var(--rz-text)]">{dryRunPlan.package_id}</span>
                  </div>
                  <div>
                    <span className="text-[var(--rz-text-muted)] block">Version:</span>
                    <span className="font-semibold text-[var(--rz-text)]">v{dryRunPlan.package_version}</span>
                  </div>
                  <div>
                    <span className="text-[var(--rz-text-muted)] block">Files to install:</span>
                    <span className="font-semibold text-emerald-400">
                      {dryRunPlan.files_to_create.length + dryRunPlan.files_to_replace.length} files
                    </span>
                  </div>
                  <div>
                    <span className="text-[var(--rz-text-muted)] block">Pre-install Backup:</span>
                    <span className="font-semibold text-emerald-400">Enabled</span>
                  </div>
                </div>

                {(dryRunPlan.files_to_create.length > 0 || dryRunPlan.files_to_replace.length > 0) && (
                  <div>
                    <span className="text-[10px] font-bold uppercase tracking-wider text-[var(--rz-text-secondary)] block mb-1">
                      Target Files
                    </span>
                    <div className="max-h-36 overflow-y-auto rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] p-2 font-mono text-[10.5px] space-y-1">
                      {dryRunPlan.files_to_create.map((targetPath, i) => (
                        <div key={`create-${i}`} className="flex items-center gap-1.5 text-[var(--rz-text-secondary)] truncate">
                          <FileCode className="w-3 h-3 text-emerald-400 shrink-0" />
                          <span className="truncate">{targetPath}</span>
                          <span className="ml-auto text-[9px] text-emerald-400 font-sans">new</span>
                        </div>
                      ))}
                      {dryRunPlan.files_to_replace.map((targetPath, i) => (
                        <div key={`replace-${i}`} className="flex items-center gap-1.5 text-[var(--rz-text-secondary)] truncate">
                          <FileCode className="w-3 h-3 text-amber-400 shrink-0" />
                          <span className="truncate">{targetPath}</span>
                          <span className="ml-auto text-[9px] text-amber-400 font-sans">modify</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)]">
                Declarative dry-run simulation verified zero conflicts. Safe to install.
              </div>
            )}

            {/* Snapshot checkbox */}
            <label className="flex items-center gap-2 text-[11px] text-[var(--rz-text)] cursor-pointer select-none pt-1">
              <input
                type="checkbox"
                checked={createSnapshot}
                onChange={(e) => setCreateSnapshot(e.target.checked)}
                className="rounded border-[var(--rz-border-subtle)] text-[var(--rz-accent)] focus:ring-[var(--rz-accent)]"
              />
              <span className="flex items-center gap-1">
                <FolderArchive className="w-3 h-3 text-emerald-400" />
                Create atomic pre-install backup snapshot
              </span>
            </label>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setShowDryRun(false)}
                className="px-3 py-1.5 rounded-lg border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  setShowDryRun(false);
                  handleInstall();
                }}
                className="px-4 py-1.5 rounded-lg bg-[var(--rz-accent)] text-white font-bold hover:bg-[var(--rz-accent)]/90 flex items-center gap-1.5 cursor-pointer"
              >
                <DownloadCloud className="w-3.5 h-3.5" />
                <span>Confirm & Install</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </ProductDetailShell>
  );
};
