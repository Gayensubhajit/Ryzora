import React, { useState, useEffect } from "react";
import { X } from "lucide-react";
import { useApp } from "../context/AppContext";
import { ProductDetailShell } from "../components/detail/ProductDetailShell";
import { ProductHero } from "../components/detail/ProductHero";
import { CompatibilityPanel } from "../components/detail/CompatibilityPanel";
import { DetailTabs } from "../components/detail/DetailTabs";
import { resolveLockscreenCapabilities } from "../providers/qylockProvider";
import { StickyActionBar } from "../components/detail/StickyActionBar";
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
    installProgress,
    setToast,
    activeLockscreen,
    applyLockscreen,
    deactivateLockscreen,
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

  const isSessionLockSupported =
    (systemIntegrationReport
      ? systemIntegrationReport.display_server.toLowerCase() === "wayland" &&
        !systemIntegrationReport.desktop.toLowerCase().includes("kde")
      : (isWayland && !isKde)) ||
    (hostCapabilities?.supported_adapters?.find((a) => a.adapter === "quickshell")?.supported ?? false);

  const isLoginScreenSupported = systemIntegrationReport
    ? systemIntegrationReport.login_manager !== "Unknown"
    : true;

  const canQs = Boolean(selectedPackage.supports_session_lock && isSessionLockSupported);
  const canSddm = Boolean(selectedPackage.supports_login_screen && isLoginScreenSupported);

  const [selectedTarget, setSelectedTarget] = useState<"quickshell" | "sddm" | "both">(() => {
    if (canQs && canSddm) return "quickshell";
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

  const isActiveForTarget =
    selectedTarget === "quickshell"
      ? isQuickshellActive
      : selectedTarget === "sddm"
      ? isSddmActive
      : isQuickshellActive && isSddmActive;

  const isTargetOverridden =
    (selectedTarget === "sddm" || selectedTarget === "both") && isSddmOverridden;

  const handleApply = async () => {
    setIsApplying(true);
    try {
      const fullConfig = {
        ...customConfig,
        variant: selectedVariantId || (schema?.variants?.[0]?.id || "default"),
      };
      await applyLockscreen(selectedPackage.id, selectedTarget, fullConfig);
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

  const handleTest = async () => {
    if (!isInstalled) {
      setToast({
        message: `Please install ${selectedPackage.title} before running a test.`,
        type: "warning",
      });
      return;
    }
    try {
      await testLockscreen(selectedPackage.id, selectedTarget);
    } catch (e: any) {
      // toast is handled in AppContext or error returned
    }
  };

  const handleUninstall = async () => {
    try {
      await deactivateAndUninstallLockscreen(selectedPackage.id, selectedTarget);
    } catch {
      // toast is handled in AppContext
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
          message: `${selectedPackage.title} installed successfully. You can now Test or Apply it.`,
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
        isActive={isActiveForTarget}
        activeTargets={{ quickshell: isQuickshellActive, sddm: isSddmActive }}
        onApply={handleApply}
        onDeactivate={handleDeactivate}
        onTest={isInstalled ? handleTest : undefined}
        onUninstall={isInstalled ? handleUninstall : undefined}
        isApplying={isApplying}
        isOverridden={isTargetOverridden}
        overriddenBy={sddmRuntimeStatus?.overridden_by}
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
        isActive={isActiveForTarget}
        onApply={handleApply}
        onDeactivate={handleDeactivate}
        onTest={isInstalled ? handleTest : undefined}
        onUninstall={isInstalled ? handleUninstall : undefined}
        isApplying={isApplying}
        isOverridden={isTargetOverridden}
        overriddenBy={sddmRuntimeStatus?.overridden_by}
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
    </ProductDetailShell>
  );
};
