import type { LockscreenTestResult } from "../types/index.ts";
import React, { useState, useEffect } from "react";
import { X, Check } from "lucide-react";
import { useApp } from "../context/AppContext";
import { ProductDetailShell } from "../components/detail/ProductDetailShell";
import { ProductHero } from "../components/detail/ProductHero";
import { CompatibilityPanel } from "../components/detail/CompatibilityPanel";
import { DetailTabs } from "../components/detail/DetailTabs";
import { resolveLockscreenCapabilities } from "../providers/qylockProvider";
import { LockScreenCustomizer } from "../components/detail/LockScreenCustomizer";

export const LockScreenDetailView: React.FC = () => {
  const {
    selectedPackage,
    setSelectedPackage,
    systemInfo,
    systemIntegrationReport,
    settings,
    installedPackages,
    installPackage,
    isInstalling,
    setToast,
    activeLockscreen,
    applyLockscreen,
    deactivateLockscreen,
    refreshActiveLockscreen,
    deactivateAndUninstallLockscreen,
    testLockscreen,
    hostCapabilities,
    sddmRuntimeStatus,
    loadSddmRuntimeStatus,
  } = useApp();

  const [activeScreenshotIndex, setActiveScreenshotIndex] = useState(0);

  const schema = selectedPackage?.lockscreen?.config_schema;
  const storageKey = selectedPackage ? `ryzora_lockscreen_config_${selectedPackage.id}` : "";

  const [selectedVariantId, setSelectedVariantId] = useState<string>(() => {
    if (!selectedPackage || !storageKey) return "";
    try {
      const saved = localStorage.getItem(`${storageKey}_variant`);
      if (saved) return saved;
    } catch {}
    return schema?.variants?.[0]?.id || "";
  });

  const [customConfig, setCustomConfig] = useState<Record<string, any>>(() => {
    const defaults: Record<string, any> = {};
    if (schema?.options) {
      for (const [key, spec] of Object.entries(schema.options)) {
        defaults[key] = spec.default;
      }
    }
    if (storageKey) {
      try {
        const saved = localStorage.getItem(storageKey);
        if (saved) {
          return { ...defaults, ...JSON.parse(saved) };
        }
      } catch {}
    }
    return defaults;
  });

  const handleConfigChange = (newCfg: Record<string, any>) => {
    setCustomConfig(newCfg);
    if (storageKey) {
      try {
        localStorage.setItem(storageKey, JSON.stringify(newCfg));
      } catch {}
    }
  };

  const handleSelectVariant = (variantId: string) => {
    setSelectedVariantId(variantId);
    if (storageKey) {
      try {
        localStorage.setItem(`${storageKey}_variant`, variantId);
      } catch {}
    }
    const v = schema?.variants?.find((item) => item.id === variantId);
    if (v?.config_overrides) {
      handleConfigChange({ ...customConfig, ...v.config_overrides });
    }
  };
  const [showFullscreen, setShowFullscreen] = useState(false);
  const [lastTestResult, setLastTestResult] = useState<LockscreenTestResult | null>(null);

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

  const isGnome =
    (systemIntegrationReport?.desktop?.toLowerCase().includes("gnome") ?? false) ||
    (systemInfo?.desktop_environment?.toLowerCase().includes("gnome") ?? false);

  const isGdmActive =
    systemIntegrationReport?.login_manager?.toLowerCase() === "gdm" ||
    hostCapabilities?.display_manager?.toLowerCase() === "gdm" ||
    ((systemInfo as any)?.display_manager?.toLowerCase() === "gdm");

  const isSddmActiveHost =
    (systemIntegrationReport?.login_manager?.toLowerCase() === "sddm" ||
     hostCapabilities?.display_manager?.toLowerCase() === "sddm" ||
     ((systemInfo as any)?.display_manager?.toLowerCase() === "sddm")) &&
    !isGdmActive;

  const isSessionLockSupported =
    (hostCapabilities
      ? (hostCapabilities.supported_adapters?.find((a) => a.adapter === "quickshell")?.supported ?? false)
      : (isWayland && !isKde && !isGnome));

  const isLoginScreenSupported =
    (hostCapabilities
      ? (hostCapabilities.supported_adapters?.find((a) => a.adapter === "sddm")?.supported ?? false)
      : isSddmActiveHost);

  const canQs = Boolean(selectedPackage.supports_session_lock && isSessionLockSupported);
  const canSddm = Boolean(selectedPackage.supports_login_screen && isLoginScreenSupported && !isGdmActive);

  const [selectedTarget, setSelectedTarget] = useState<"quickshell" | "sddm" | "both">(() => {
    if (canQs && canSddm) return "both";
    if (canSddm) return "sddm";
    return "quickshell";
  });

  useEffect(() => {
    if (selectedPackage) {
      loadSddmRuntimeStatus(selectedPackage.id);
    }
  }, [selectedPackage, selectedTarget]);

  const installedRecord = installedPackages.find(
    (p) => p.package_id === selectedPackage.id
  );

  // Target-aware installation verification
  const hasUserFiles = Boolean(
    installedRecord?.files?.some((f) => f.target.startsWith("~/")) ||
    installedRecord?.installed_files?.some((f) => f.startsWith("~/") || f.includes(".local/share/ryzora"))
  );
  const hasSddmFiles = Boolean(
    sddmRuntimeStatus?.available ||
    installedRecord?.files?.some((f) => f.target.includes("/usr/share/sddm")) ||
    installedRecord?.installed_files?.some((f) => f.includes("/usr/share/sddm"))
  );

  const isInstalledForTarget =
    !installedRecord
      ? false
      : selectedTarget === "quickshell"
      ? hasUserFiles
      : selectedTarget === "sddm"
      ? hasSddmFiles
      : hasUserFiles && hasSddmFiles;

  const isInstalled = isInstalledForTarget;
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

  const isSddmApplied =
    Boolean(activeLockscreen?.sddm) &&
    (activeLockscreen.sddm === selectedPackage.id ||
      activeLockscreen.sddm === `lockscreen-qylock-${pkgSlug}` ||
      activeLockscreen.sddm === pkgSlug ||
      activeLockscreen.sddm === `ryzora-${pkgSlug}`);

  // Only consider active if the effective SDDM theme resolved on the machine actually matches!
  const isSddmActive = isSddmApplied && sddmRuntimeStatus?.active === true;
  const isSddmOverridden = isSddmApplied && sddmRuntimeStatus?.is_overridden === true;



  const isTargetOverridden =
    (selectedTarget === "sddm" || selectedTarget === "both") && isSddmOverridden;

  const handleApply = async (targetOverride?: "quickshell" | "sddm" | "both") => {
    setIsApplying(true);
    const targetToApply = targetOverride || selectedTarget;
    try {
      const fullConfig = {
        ...customConfig,
        variant: selectedVariantId || (schema?.variants?.[0]?.id || "default"),
      };
      await applyLockscreen(selectedPackage.id, targetToApply, fullConfig);
    } catch {
      // toast is handled in AppContext
    } finally {
      setIsApplying(false);
    }
  };

  const handleDeactivate = async (targetOverride?: "quickshell" | "sddm" | "both") => {
    setIsApplying(true);
    const targetToDeactivate =
      targetOverride ||
      (isQuickshellActive && isSddmActive
        ? "both"
        : isQuickshellActive
        ? "quickshell"
        : "sddm");
    try {
      await deactivateLockscreen(targetToDeactivate);
      await refreshActiveLockscreen();
    } catch {
      // toast is handled in AppContext
    } finally {
      setIsApplying(false);
    }
  };

  const handleTest = async (targetOverride?: "quickshell" | "sddm") => {
    try {
      const fullConfig = {
        ...customConfig,
        variant: selectedVariantId || (schema?.variants?.[0]?.id || "default"),
      };
      const targetToTest =
        targetOverride ||
        (isQuickshellActive && !isSddmActive
          ? "quickshell"
          : !isQuickshellActive && isSddmActive
          ? "sddm"
          : selectedTarget === "sddm"
          ? "sddm"
          : "quickshell");
      const res = await testLockscreen(selectedPackage.id, targetToTest, fullConfig);
      if (res) {
        setLastTestResult(res);
      }
    } catch (e: any) {
      // toast is handled in AppContext or error returned
    }
  };

  const handleUninstall = async () => {
    setIsApplying(true);
    try {
      const activeTarget =
        isQuickshellActive && isSddmActive
          ? "both"
          : isQuickshellActive
          ? "quickshell"
          : isSddmActive
          ? "sddm"
          : undefined;

      const success = await deactivateAndUninstallLockscreen(selectedPackage.id, activeTarget);
      if (success) {
        await refreshActiveLockscreen();
      }
    } catch {
      // toast is handled in AppContext
    } finally {
      setIsApplying(false);
    }
  };

  const handleClose = () => {
    setSelectedPackage(null);
  };

  const targetCapabilities =
    selectedPackage.lockscreen && systemInfo
      ? resolveLockscreenCapabilities(systemInfo, selectedPackage.lockscreen, selectedTarget)
      : null;
  const missingDependencies = targetCapabilities?.missing_dependencies || [];

  // Install handler: Never disrupt user with snapshot prompts.
  // Full snapshots only taken if user explicitly configured "always" in Settings.
  // Ryzora uses automatic lightweight transactional backups on apply.
  const handleInstall = async () => {
    const shouldSnapshot = settings?.snapshot_policy === "always";

    try {
      const res = await installPackage(selectedPackage, shouldSnapshot, selectedTarget);
      if (res && res.success) {
        setToast({
          message: `${selectedPackage.title} installed successfully.`,
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
        isLoginScreenSupported={isLoginScreenSupported}
        isActive={isQuickshellActive || isSddmActive}
        activeTargets={{ quickshell: isQuickshellActive, sddm: isSddmActive }}
        installedTargets={{ quickshell: hasUserFiles, sddm: hasSddmFiles }}
        onApply={handleApply}
        onDeactivate={handleDeactivate}
        onTest={isInstalled ? handleTest : undefined}
        onUninstall={isInstalled ? handleUninstall : undefined}
        isApplying={isApplying}
        isOverridden={isTargetOverridden}
        overriddenBy={sddmRuntimeStatus?.overridden_by}
      />

      {/* ── Compatibility Summary Strip (Directly Below Hero) ── */}
      <CompatibilityPanel
        packageItem={selectedPackage}
        systemInfo={systemInfo}
        missingDependencies={missingDependencies}
        selectedTarget={selectedTarget}
        isSessionLockSupported={isSessionLockSupported}
        isLoginScreenSupported={isLoginScreenSupported}
        isGdmActive={isGdmActive}
      />

      {/* ── Interactive Theme Customization & Variants ── */}
      {schema && (
        <LockScreenCustomizer
          packageItem={selectedPackage}
          configSchema={schema}
          currentConfig={customConfig}
          selectedVariantId={selectedVariantId}
          onChangeConfig={handleConfigChange}
          onSelectVariant={handleSelectVariant}
        />
      )}

      {/* ── Technical Detail Tabs (About, Screenshots, Specs) ── */}
      <DetailTabs
        packageItem={selectedPackage}
        systemInfo={systemInfo}
        selectedTarget={selectedTarget}
      />

      {/* ── Isolated Test Environment Dedicated Modal ── */}
      {lastTestResult && (
        <div
          className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4 animate-in fade-in duration-150"
          onClick={() => setLastTestResult(null)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] p-6 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between pb-3 border-b border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-2 font-semibold text-sm text-[var(--rz-text)]">
                <Check className="w-4 h-4 text-emerald-500 stroke-[2.5]" />
                <span>Isolated Test Environment</span>
              </div>
              <button
                type="button"
                onClick={() => setLastTestResult(null)}
                className="text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-3 text-xs">
              <div className="flex items-center justify-between p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <span className="text-[var(--rz-text-secondary)]">Target Mode</span>
                <span className="font-semibold text-[var(--rz-text)] uppercase">{lastTestResult.target}</span>
              </div>

              <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1">
                <span className="text-[var(--rz-text-muted)] text-[10px] uppercase font-semibold block">Runtime Sandbox</span>
                <code className="text-[11px] font-mono text-[var(--rz-text)] break-all">{lastTestResult.test_runtime_dir}</code>
              </div>

              {Object.keys(lastTestResult.tested_config).length > 0 && (
                <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1">
                  <span className="text-[var(--rz-text-muted)] text-[10px] uppercase font-semibold block">Materialized Configuration</span>
                  <div className="text-[11px] font-mono text-[var(--rz-text)] space-y-0.5 max-h-28 overflow-y-auto">
                    {Object.entries(lastTestResult.tested_config).map(([k, v]) => (
                      <div key={k}>{k} = {String(v)}</div>
                    ))}
                  </div>
                </div>
              )}
            </div>

            <div className="flex justify-end pt-3 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setLastTestResult(null)}
                className="px-4 py-2 rounded-xl bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white text-xs font-semibold cursor-pointer shadow-xs"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}

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
    </ProductDetailShell>
  );
};
