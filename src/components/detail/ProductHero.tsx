import React, { useState } from "react";
import {
  DownloadCloud,
  Loader2,
  Check,
  AlertTriangle,
  Sparkles,
  ShieldCheck,
  Download,
  Star,
  Calendar,
  Lock,
  Monitor,
  Layers,
  Maximize2,
  Trash2,
} from "lucide-react";
import { PackageItem } from "../../types";
import { formatDownloads } from "../catalogue/catalogueUtils";
import { CreatorHeader } from "./CreatorHeader";
import { PreviewGallery } from "./PreviewGallery";
import { MediaPreview } from "../media/MediaPreview";

interface ProductHeroProps {
  packageItem: PackageItem;
  activeScreenshotIndex: number;
  onSelectScreenshot: (index: number) => void;
  onInstall: () => void;
  onOpenLightbox: () => void;
  isInstalling: boolean;
  isInstalled: boolean;
  isUpdateAvailable: boolean;
  isBlocked: boolean;
  missingDependencies: string[];
  selectedTarget?: "quickshell" | "sddm" | "both";
  onSelectTarget?: (target: "quickshell" | "sddm" | "both") => void;
  isSessionLockSupported?: boolean;
  isLoginScreenSupported?: boolean;
  isActive?: boolean;
  activeTargets?: { quickshell: boolean; sddm: boolean };
  onApply?: () => void;
  onDeactivate?: () => void;
  onTest?: () => void;
  onUninstall?: () => void;
  isApplying?: boolean;
  isOverridden?: boolean;
  overriddenBy?: string | null;
}

export const ProductHero: React.FC<ProductHeroProps> = ({
  packageItem,
  activeScreenshotIndex,
  onSelectScreenshot,
  onInstall,
  onOpenLightbox,
  isInstalling,
  isInstalled,
  isUpdateAvailable,
  isBlocked,
  missingDependencies,
  selectedTarget = "quickshell",
  onSelectTarget,
  isSessionLockSupported = true,
  isLoginScreenSupported = true,
  isActive = false,
  activeTargets = { quickshell: false, sddm: false },
  onApply,
  onDeactivate,
  onTest,
  onUninstall,
  isApplying = false,
  isOverridden = false,
  overriddenBy = null,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);

  const pkgSlug = packageItem.id
    .replace(/^lockscreen-qylock-/, "")
    .replace(/^lockscreen-/, "");

  const canTargetQs = Boolean(packageItem.supports_session_lock && isSessionLockSupported);
  const canTargetSddm = Boolean(packageItem.supports_login_screen && isLoginScreenSupported);
  const isDualTarget = canTargetQs && canTargetSddm;

  const subtype = packageItem.tags?.find((t) =>
    ["hyprlock", "quickshell", "swaylock", "sddm"].includes(t.toLowerCase())
  ) || "Lock Screen";

  const screenshots =
    packageItem.screenshots && packageItem.screenshots.length > 0
      ? packageItem.screenshots
      : [packageItem.hero_image];

  const activeImage = screenshots[activeScreenshotIndex] || packageItem.hero_image;

  const getActionLabel = () => {
    if (isInstalling) return "Installing...";
    if (isUpdateAvailable) return "Update";
    if (isInstalled) return "Installed";
    if (!canTargetQs && !canTargetSddm) return "Unsupported on Current Desktop";
    if (isDualTarget) {
      if (selectedTarget === "quickshell") return "Install Session Lock";
      if (selectedTarget === "sddm") return "Install Login Screen";
      return "Install Both";
    }
    return canTargetSddm ? "Install Login Screen" : "Install Session Lock";
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 p-4 sm:p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-sm">
      {/* ── Left Column: Media Hero (60-65%) ── */}
      <div className="lg:col-span-7 flex flex-col gap-3">
        <div className="relative rounded-xl overflow-hidden border border-[var(--rz-border-subtle)] group">
          {/* Main Media Viewer */}
          <MediaPreview
            poster={activeScreenshotIndex === 0 && (packageItem.preview_poster_url || packageItem.hero_image) ? (packageItem.preview_poster_url || packageItem.hero_image) : activeImage}
            videoSrc={activeScreenshotIndex === 0 ? (packageItem.preview_video_url || packageItem.preview_video || packageItem.lockscreen?.media.preview_video) : undefined}
            animatedSrc={activeScreenshotIndex === 0 ? (packageItem.preview_animated || packageItem.lockscreen?.media.preview_animated) : undefined}
            mediaType={activeScreenshotIndex === 0 ? (packageItem.media_type || (packageItem.preview_video_url || packageItem.preview_video ? "video" : (packageItem.preview_animated ? "animated" : "image"))) : "image"}
            alt={packageItem.title}
            mode="hero"
            aspectRatio="16/9"
            showBadge={true}
            className="w-full"
          />

          {/* Lightbox Trigger */}
          <button
            type="button"
            onClick={onOpenLightbox}
            className="absolute top-3 right-3 p-2 rounded-lg bg-black/60 hover:bg-black/80 text-white/90 border border-white/10 backdrop-blur-md transition-all shadow-md hover:scale-105 z-20 cursor-pointer"
            aria-label="View fullscreen image"
            title="Expand fullscreen preview"
          >
            <Maximize2 className="w-4 h-4" />
          </button>
        </div>

        {/* Thumbnail Selector Gallery */}
        <PreviewGallery
          screenshots={screenshots}
          activeIndex={activeScreenshotIndex}
          onSelect={onSelectScreenshot}
          title={packageItem.title}
        />
      </div>

      {/* ── Right Column: Package Identity, Target Selector & Actions (35-40%) ── */}
      <div className="lg:col-span-5 flex flex-col justify-between">
        <div>
          {/* Breadcrumb / Subtype */}
          <div className="flex items-center gap-2 mb-1.5">
            <span className="text-[11px] font-mono uppercase tracking-widest text-[var(--rz-text-secondary)] font-bold">
              Lock Screens · {subtype}
            </span>
            <span className="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-accent-text)]">
              v{packageItem.version}
            </span>
          </div>

          {/* Title */}
          <h1 className="text-xl sm:text-2xl font-bold tracking-tight text-[var(--rz-text)] leading-tight">
            {packageItem.title}
          </h1>

          {/* Trust badges */}
          <div className="flex items-center gap-1.5 mt-2">
            {packageItem.trust_tier === "official" && (
              <span className="px-2 py-0.5 rounded-md text-[10px] font-bold uppercase bg-amber-500/20 text-amber-300 border border-amber-400/30 flex items-center gap-1">
                <Sparkles className="w-3 h-3 text-amber-400" />
                Official
              </span>
            )}
            {packageItem.trust_tier === "verified" && (
              <span className="px-2 py-0.5 rounded-md text-[10px] font-bold uppercase bg-sky-500/20 text-sky-300 border border-sky-400/30 flex items-center gap-1">
                <ShieldCheck className="w-3 h-3 text-sky-400" />
                Verified
              </span>
            )}
            <span className="px-2 py-0.5 rounded-md text-[10px] font-semibold bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]">
              Qylock Upstream
            </span>
          </div>

          {/* Author Header */}
          <CreatorHeader
            name={packageItem.author.name}
            avatar={packageItem.author.avatar}
            verified={packageItem.author.verified}
            packageCount={12}
            totalDownloads={packageItem.downloads}
          />

          {/* Key Metrics */}
          <div className="grid grid-cols-3 gap-2 p-2.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] my-3 text-xs">
            <div>
              <span className="text-[10px] text-[var(--rz-text-muted)] block">Downloads</span>
              <span className="font-bold font-mono text-[var(--rz-text)] flex items-center gap-1 mt-0.5">
                <Download className="w-3 h-3 text-[var(--rz-text-muted)]" />
                {formatDownloads(packageItem.downloads)}
              </span>
            </div>
            <div>
              <span className="text-[10px] text-[var(--rz-text-muted)] block">Rating</span>
              <span className="font-bold font-mono text-[var(--rz-text)] flex items-center gap-1 mt-0.5">
                <Star className="w-3 h-3 text-amber-400 fill-amber-400" />
                {packageItem.rating > 0 ? packageItem.rating.toFixed(1) : "N/A"}
              </span>
            </div>
            <div>
              <span className="text-[10px] text-[var(--rz-text-muted)] block">License</span>
              <span className="font-bold font-mono text-[var(--rz-text)] flex items-center gap-1 mt-0.5 truncate text-[11px]">
                <Calendar className="w-3 h-3 text-[var(--rz-text-muted)] shrink-0" />
                GPL-3.0
              </span>
            </div>
          </div>

          {/* ── Capability-Driven Target Selector (When Dual-Target and System Supports Both) ── */}
          {isDualTarget && onSelectTarget && (
            <div className="p-3 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] mb-3">
              <span className="text-[10px] font-mono uppercase tracking-wider text-[var(--rz-text-muted)] font-bold block mb-2">
                Select Installation Target
              </span>
              <div className="grid grid-cols-1 gap-1.5">
                {/* Session Lock Option */}
                <label
                  className={`flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition-all ${
                    selectedTarget === "quickshell"
                      ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-text)] font-semibold"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:bg-[var(--rz-surface)]/80"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="radio"
                      name="lock_target"
                      value="quickshell"
                      checked={selectedTarget === "quickshell"}
                      onChange={() => onSelectTarget("quickshell")}
                      className="accent-[var(--rz-accent)]"
                    />
                    <Lock className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                    <div>
                      <span className="block text-[11px] font-medium">Session Lock (Quickshell)</span>
                      <span className="block text-[9px] text-[var(--rz-text-muted)]">User session · Wayland</span>
                    </div>
                  </div>
                  <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-emerald-500/20 text-emerald-800 dark:text-emerald-300">
                    User Privileges
                  </span>
                </label>

                {/* SDDM Login Screen Option */}
                <label
                  className={`flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition-all ${
                    selectedTarget === "sddm"
                      ? "bg-amber-500/15 border-amber-500 text-[var(--rz-text)] font-semibold"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:bg-[var(--rz-surface)]/80"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="radio"
                      name="lock_target"
                      value="sddm"
                      checked={selectedTarget === "sddm"}
                      onChange={() => onSelectTarget("sddm")}
                      className="accent-amber-500"
                    />
                    <Monitor className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                    <div>
                      <span className="block text-[11px] font-medium">Login Screen (SDDM)</span>
                      <span className="block text-[9px] text-[var(--rz-text-muted)]">Display manager · Root</span>
                    </div>
                  </div>
                  <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-amber-500/20 text-amber-800 dark:text-amber-300">
                    Polkit Auth
                  </span>
                </label>

                {/* Both Option */}
                <label
                  className={`flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition-all ${
                    selectedTarget === "both"
                      ? "bg-purple-500/15 border-purple-500 text-[var(--rz-text)] font-semibold"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:bg-[var(--rz-surface)]/80"
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="radio"
                      name="lock_target"
                      value="both"
                      checked={selectedTarget === "both"}
                      onChange={() => onSelectTarget("both")}
                      className="accent-purple-500"
                    />
                    <Layers className="w-3.5 h-3.5 text-purple-400 shrink-0" />
                    <div>
                      <span className="block text-[11px] font-medium">Both Targets</span>
                      <span className="block text-[9px] text-[var(--rz-text-muted)]">Unified Session + Login</span>
                    </div>
                  </div>
                  <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-purple-500/20 text-purple-800 dark:text-purple-300">
                    Recommended
                  </span>
                </label>
              </div>
            </div>
          )}
        </div>
        {/* ── Installation & Active Status Preview ── */}
        {isInstalled && (
          <div className="p-3.5 rounded-xl bg-[var(--rz-surface-elevated,#12161f)] border border-[var(--rz-border-subtle,#222a38)] mb-3 text-xs space-y-2.5">
            <div className="flex items-center justify-between text-[11px] pb-2 border-b border-[var(--rz-border-subtle,#222a38)]">
              <span className="font-mono uppercase tracking-wider text-[var(--rz-text-muted,#747d8f)] font-bold text-[10px] flex items-center gap-1.5">
                <span className="w-1.5 h-1.5 rounded-full bg-[var(--rz-accent,#4f8ff7)]" />
                Installation & Materialization
              </span>
              <span className="font-semibold text-xs flex items-center gap-1.5">
                {selectedTarget === "both" ? (
                  <span className="text-xs">
                    QS: {activeTargets?.quickshell ? <span className="text-emerald-400 font-bold">● Active</span> : <span className="text-zinc-400">○ Inactive</span>} · SDDM: {activeTargets?.sddm ? <span className="text-emerald-400 font-bold">● Active</span> : <span className="text-zinc-400">○ Inactive</span>}
                  </span>
                ) : isActive ? (
                  <span className="flex items-center gap-1.5 text-emerald-400 font-bold">
                    <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                    <span>● Active</span>
                  </span>
                ) : (
                  <span className="flex items-center gap-1.5 text-zinc-400 font-medium">
                    <span className="w-2 h-2 rounded-full bg-zinc-500" />
                    <span>○ Inactive (Installed)</span>
                  </span>
                )}
              </span>
            </div>

            <div className="space-y-2 text-[11px]">
              <div>
                <span className="text-[10px] uppercase font-mono tracking-wider text-[var(--rz-text-muted,#747d8f)] block">Target</span>
                <span className="font-semibold text-[var(--rz-text,#f5f5f7)]">
                  {selectedTarget === "both" ? "Session Lock (Quickshell) + Login Screen (SDDM)" : selectedTarget === "sddm" ? "Login Screen · SDDM" : "Session Lock · Quickshell"}
                </span>
              </div>

              <div>
                <span className="text-[10px] uppercase font-mono tracking-wider text-[var(--rz-text-muted,#747d8f)] block mb-0.5">Runtime Destination</span>
                {selectedTarget === "quickshell" && (
                  <div className="font-mono text-emerald-400 text-[10.5px] px-2.5 py-1.5 rounded-lg bg-[var(--rz-bg,#090b0e)] border border-[var(--rz-border-subtle,#222a38)] truncate" title={`~/.local/share/ryzora/lockscreens/qylock/${pkgSlug}/`}>
                    ~/.local/share/ryzora/lockscreens/qylock/{pkgSlug}/
                  </div>
                )}
                {selectedTarget === "sddm" && (
                  <div className="font-mono text-amber-400 text-[10.5px] px-2.5 py-1.5 rounded-lg bg-[var(--rz-bg,#090b0e)] border border-[var(--rz-border-subtle,#222a38)] truncate" title={`/usr/share/sddm/themes/ryzora-${pkgSlug}/`}>
                    /usr/share/sddm/themes/ryzora-{pkgSlug}/
                  </div>
                )}
                {selectedTarget === "both" && (
                  <div className="space-y-1 font-mono text-[10.5px]">
                    <div className="text-emerald-400 px-2.5 py-1.5 rounded-lg bg-[var(--rz-bg,#090b0e)] border border-[var(--rz-border-subtle,#222a38)] truncate">
                      User: ~/.local/share/ryzora/lockscreens/qylock/{pkgSlug}/
                    </div>
                    <div className="text-amber-400 px-2.5 py-1.5 rounded-lg bg-[var(--rz-bg,#090b0e)] border border-[var(--rz-border-subtle,#222a38)] truncate">
                      System: /usr/share/sddm/themes/ryzora-{pkgSlug}/
                    </div>
                  </div>
                )}
              </div>

              <div>
                <span className="text-[10px] uppercase font-mono tracking-wider text-[var(--rz-text-muted,#747d8f)] block mb-0.5">
                  {selectedTarget === "sddm" ? "Configuration" : "Integration"}
                </span>
                <div className="font-mono text-[10.5px] text-[var(--rz-text-secondary,#a6aab3)] px-2.5 py-1.5 rounded-lg bg-[var(--rz-bg,#090b0e)] border border-[var(--rz-border-subtle,#222a38)] truncate">
                  {selectedTarget === "sddm" ? "/etc/sddm.conf.d/ryzora-theme.conf" : "~/.local/bin/ryzora-lock"}
                </div>
              </div>
            </div>
          </div>
        )}

        {/* ── Primary Action Buttons (Install → Test → Apply → Deactivate/Uninstall Lifecycle) ── */}
        <div className="flex items-center gap-2 pt-2 border-t border-[var(--rz-border-subtle)]">
          {!isInstalled ? (
            /* 1. NOT INSTALLED: Show Install action */
            <button
              type="button"
              disabled={isBlocked || isInstalling || (!canTargetQs && !canTargetSddm)}
              onClick={onInstall}
              className={[
                "w-full py-2.5 rounded-xl text-xs font-bold transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-md",
                isBlocked
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                  : (selectedTarget === "sddm" || selectedTarget === "both")
                  ? "bg-amber-600 hover:bg-amber-500 text-white"
                  : "bg-[var(--rz-accent)] hover:bg-[var(--rz-accent)]/90 text-white",
              ].join(" ")}
            >
              {isInstalling ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>Installing...</span>
                </>
              ) : isUpdateAvailable ? (
                <>
                  <DownloadCloud className="w-4 h-4" />
                  <span>Update Package</span>
                </>
              ) : (
                <>
                  <DownloadCloud className="w-4 h-4" />
                  <span>{getActionLabel()}</span>
                </>
              )}
            </button>
          ) : !isActive ? (
            /* 2. INSTALLED (INACTIVE): [ Test ] [ Apply ] [ Uninstall ] */
            <>
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3.5 py-2 rounded-xl text-xs font-semibold border border-indigo-500/30 bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Launch isolated lockscreen test"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <button
                type="button"
                disabled={isApplying}
                onClick={onApply}
                className="flex-1 py-2 rounded-xl text-xs font-bold transition-all cursor-pointer select-none flex items-center justify-center gap-1.5 shadow-md bg-emerald-600 hover:bg-emerald-500 text-white"
              >
                {isApplying ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span>Activating...</span>
                  </>
                ) : (
                  <>
                    <Sparkles className="w-3.5 h-3.5" />
                    <span>{selectedTarget === "both" ? "Apply Both" : selectedTarget === "sddm" ? "Apply Login Screen" : "Apply Session Lock"}</span>
                  </>
                )}
              </button>

              {onUninstall && (
                <button
                  type="button"
                  onClick={() => setShowUninstallConfirm(true)}
                  className="px-3 py-2 rounded-xl text-xs font-medium border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text-muted)] hover:text-rose-400 hover:border-rose-500/30 hover:bg-rose-500/10 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Uninstall package"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  <span className="hidden sm:inline">Uninstall</span>
                </button>
              )}
            </>
          ) : isOverridden ? (
            /* 3. OVERRIDDEN: [ Test ] [ ⚠ Overridden ] [ Deactivate ] */
            <>
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3.5 py-2 rounded-xl text-xs font-semibold border border-indigo-500/30 bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Launch isolated lockscreen test"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <div
                className="flex-1 py-2 px-3 rounded-xl text-xs font-bold bg-amber-600/90 text-white shadow-md flex items-center justify-center gap-1.5 cursor-default select-none border border-amber-400/40"
                title={overriddenBy ? `Overridden by ${overriddenBy}` : "Configuration overridden"}
              >
                <AlertTriangle className="w-3.5 h-3.5 text-amber-200 stroke-[2.5]" />
                <span>⚠ Overridden</span>
              </div>

              {onDeactivate && (
                <button
                  type="button"
                  disabled={isApplying}
                  onClick={onDeactivate}
                  className="px-3 py-2 rounded-xl text-xs font-semibold bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-300 transition-all cursor-pointer select-none"
                >
                  Deactivate
                </button>
              )}
            </>
          ) : (
            /* 4. ACTIVE: [ Test ] [ ✓ Active ] [ Deactivate ] [ Uninstall ] */
            <>
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3.5 py-2 rounded-xl text-xs font-semibold border border-indigo-500/30 bg-indigo-500/10 hover:bg-indigo-500/20 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Launch isolated lockscreen test"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <div className="flex-1 py-2 px-3 rounded-xl text-xs font-bold bg-emerald-600 text-white shadow-md flex items-center justify-center gap-1.5 cursor-default select-none border border-emerald-400/30">
                <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
                <span>✓ Active</span>
              </div>

              {onDeactivate && (
                <button
                  type="button"
                  disabled={isApplying}
                  onClick={onDeactivate}
                  className="px-3 py-2 rounded-xl text-xs font-semibold bg-amber-500/10 hover:bg-amber-500/20 border border-amber-500/30 text-amber-300 transition-all cursor-pointer select-none"
                  title="Deactivate lockscreen"
                >
                  Deactivate
                </button>
              )}

              {onUninstall && (
                <button
                  type="button"
                  onClick={() => setShowUninstallConfirm(true)}
                  className="px-3 py-2 rounded-xl text-xs font-medium border border-rose-500/30 bg-rose-500/10 hover:bg-rose-500/20 text-rose-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Deactivate and cleanly uninstall"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  <span className="hidden sm:inline">Uninstall</span>
                </button>
              )}
            </>
          )}
        </div>

        {/* Warning if configuration is overridden */}
        {isOverridden && overriddenBy && (
          <div className="mt-2 flex items-center gap-1.5 text-[11px] text-amber-400 font-medium">
            <AlertTriangle className="w-3 h-3 shrink-0" />
            <span>Applied by Ryzora, but overridden by <code className="bg-black/30 px-1 py-0.5 rounded text-amber-200 font-mono text-[10px]">{overriddenBy}</code></span>
          </div>
        )}

        {/* Warning if installation is blocked */}
        {isBlocked && (
          <div className="mt-2 flex items-center gap-1.5 text-[11px] text-rose-400 font-medium">
            <AlertTriangle className="w-3 h-3 shrink-0" />
            <span>Installation blocked: missing {missingDependencies.join(", ")}</span>
          </div>
        )}
      </div>

      {/* ── Uninstall Confirmation Modal ── */}
      {showUninstallConfirm && onUninstall && (
        <div
          className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4 backdrop-blur-sm animate-in fade-in duration-150"
          onClick={() => setShowUninstallConfirm(false)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--rz-bg)] border border-rose-500/30 p-5 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2.5 text-rose-400 font-bold text-sm">
              <Trash2 className="w-4 h-4" />
              <span>{isActive ? "Deactivate & Uninstall Package?" : "Uninstall Package?"}</span>
            </div>

            <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
              {isActive ? (
                <>
                  <strong>{packageItem.title}</strong> is currently active. Uninstalling will first safely restore your previous configuration (e.g. Dusky/winter), restore transactional backups, and then remove the theme files.
                </>
              ) : (
                <>
                  Are you sure you want to uninstall <strong>{packageItem.title}</strong>? All downloaded theme assets and configurations will be removed.
                </>
              )}
            </p>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="px-3 py-1.5 rounded-lg border border-[var(--rz-border-subtle)] text-xs text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  setShowUninstallConfirm(false);
                  onUninstall();
                }}
                className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold flex items-center gap-1.5 cursor-pointer shadow-md"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Confirm Uninstall</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
