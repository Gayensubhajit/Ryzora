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
  Maximize2,
  Trash2,
  MoreHorizontal,
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
  selectedTarget = "both",
  onSelectTarget,
  isSessionLockSupported = true,
  isLoginScreenSupported = true,
  onTest,
  onUninstall,
  isOverridden = false,
  overriddenBy = null,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [showOverflow, setShowOverflow] = useState(false);

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

  // Target checkbox state
  const isQsChecked = selectedTarget === "both" || selectedTarget === "quickshell";
  const isSddmChecked = selectedTarget === "both" || selectedTarget === "sddm";

  const handleToggleQs = () => {
    if (!onSelectTarget || !isDualTarget) return;
    if (isQsChecked && isSddmChecked) {
      onSelectTarget("sddm");
    } else if (!isQsChecked) {
      onSelectTarget(isSddmChecked ? "both" : "quickshell");
    }
  };

  const handleToggleSddm = () => {
    if (!onSelectTarget || !isDualTarget) return;
    if (isSddmChecked && isQsChecked) {
      onSelectTarget("quickshell");
    } else if (!isSddmChecked) {
      onSelectTarget(isQsChecked ? "both" : "sddm");
    }
  };

  const handleConfirmUninstall = () => {
    setShowUninstallConfirm(false);
    if (onUninstall) {
      onUninstall();
    }
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1.55fr)_minmax(380px,0.85fr)] gap-6 lg:gap-8 p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-sm">
      {/* ── Left Column: Media Hero (Dominant Preview) ── */}
      <div className="flex flex-col gap-3 min-w-0">
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

      {/* ── Right Column: Package Identity, Target Selector & Actions ── */}
      <div className="flex flex-col gap-4 justify-start min-w-0">
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
          <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-[var(--rz-text)] leading-tight">
            {packageItem.title}
          </h1>

          {/* Trust badges */}
          <div className="flex flex-wrap items-center gap-1.5 mt-2.5">
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
            <span className="px-2 py-0.5 rounded-md text-[10px] font-semibold bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 flex items-center gap-1">
              <ShieldCheck className="w-3 h-3 text-emerald-400" />
              Safe to install · Reversible
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
          <div className="grid grid-cols-3 gap-2 p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] my-3 text-xs">
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

          {/* ── Installation Targets (Streamlined Two-Option UI) ── */}
          <div className="p-3.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] mb-3">
            <span className="text-[10px] font-mono uppercase tracking-wider text-[var(--rz-text-muted)] font-bold block mb-2.5">
              Installation
            </span>
            <div className="space-y-1.5">
              {/* Session Lock Option */}
              {canTargetQs && (
                <div
                  onClick={handleToggleQs}
                  className={`flex items-center justify-between p-2.5 rounded-lg border text-xs transition-all ${
                    isDualTarget ? "cursor-pointer" : "cursor-default"
                  } ${
                    isQsChecked
                      ? "bg-[var(--rz-accent)]/10 border-[var(--rz-accent)]/50 text-[var(--rz-text)]"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] opacity-70"
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <div className={`w-4 h-4 rounded flex items-center justify-center border transition-all ${
                      isQsChecked ? "bg-[var(--rz-accent)] border-[var(--rz-accent)] text-white" : "border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]"
                    }`}>
                      {isQsChecked && <Check className="w-3 h-3 stroke-[3]" />}
                    </div>
                    <div>
                      <span className="block text-xs font-semibold">Session Lock</span>
                      <span className="block text-[10px] text-[var(--rz-text-muted)]">Hyprland · Wayland</span>
                    </div>
                  </div>
                  <Lock className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                </div>
              )}

              {/* Login Screen Option */}
              {canTargetSddm && (
                <div
                  onClick={handleToggleSddm}
                  className={`flex items-center justify-between p-2.5 rounded-lg border text-xs transition-all ${
                    isDualTarget ? "cursor-pointer" : "cursor-default"
                  } ${
                    isSddmChecked
                      ? "bg-[var(--rz-accent)]/10 border-[var(--rz-accent)]/50 text-[var(--rz-text)]"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] opacity-70"
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <div className={`w-4 h-4 rounded flex items-center justify-center border transition-all ${
                      isSddmChecked ? "bg-[var(--rz-accent)] border-[var(--rz-accent)] text-white" : "border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]"
                    }`}>
                      {isSddmChecked && <Check className="w-3 h-3 stroke-[3]" />}
                    </div>
                    <div>
                      <span className="block text-xs font-semibold">Login Screen</span>
                      <span className="block text-[10px] text-[var(--rz-text-muted)]">SDDM Greeter</span>
                    </div>
                  </div>
                  <Monitor className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Primary Action Area (Install before, Test/Overflow after) ── */}
        <div className="pt-2 border-t border-[var(--rz-border-subtle)]">
          {!isInstalled ? (
            /* 1. NOT INSTALLED: Clear Single Install Action */
            <button
              type="button"
              disabled={isBlocked || isInstalling || (!canTargetQs && !canTargetSddm)}
              onClick={onInstall}
              className={[
                "w-full py-3 px-5 rounded-xl text-xs font-bold transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-md",
                isBlocked
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                  : "bg-[var(--rz-accent,#4f8ff7)] hover:bg-[var(--rz-accent,#4f8ff7)]/90 text-white",
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
                  <span>Update</span>
                </>
              ) : (
                <>
                  <DownloadCloud className="w-4 h-4" />
                  <span>Install</span>
                </>
              )}
            </button>
          ) : (
            /* 2. INSTALLED: [ ✓ Installed ] + [ Test ] + [ ⋯ ] */
            <div className="space-y-2.5">
              <div className="flex items-center justify-between p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 text-xs font-semibold">
                <span className="flex items-center gap-2">
                  <Check className="w-4 h-4 text-emerald-400 stroke-[3]" />
                  <span>
                    {selectedTarget === "both"
                      ? "Installed for Session + Login"
                      : selectedTarget === "sddm"
                      ? "Installed for Login Screen"
                      : "Installed for Session Lock"}
                  </span>
                </span>
              </div>

              <div className="flex items-center gap-2">
                {onTest && (
                  <button
                    type="button"
                    onClick={onTest}
                    className="flex-1 py-2.5 rounded-xl text-xs font-semibold bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 text-indigo-300 transition-all cursor-pointer select-none flex items-center justify-center gap-1.5 shadow-sm"
                    title="Launch isolated lockscreen test"
                  >
                    <Lock className="w-3.5 h-3.5" />
                    <span>Test</span>
                  </button>
                )}

                {onUninstall && (
                  <div className="relative">
                    <button
                      type="button"
                      onClick={() => setShowOverflow(!showOverflow)}
                      className="p-2.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] transition-all cursor-pointer select-none"
                      aria-label="More options"
                      title="More options"
                    >
                      <MoreHorizontal className="w-4 h-4" />
                    </button>

                    {showOverflow && (
                      <div
                        className="absolute right-0 bottom-full mb-2 w-36 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] shadow-xl p-1 z-40 animate-in fade-in zoom-in-95 duration-100"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            setShowOverflow(false);
                            setShowUninstallConfirm(true);
                          }}
                          className="w-full px-3 py-2 rounded-lg text-xs font-medium text-rose-400 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                          <span>Uninstall</span>
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Warning if configuration is overridden */}
        {isOverridden && overriddenBy && (
          <div className="flex items-center gap-1.5 text-[11px] text-amber-400 font-medium">
            <AlertTriangle className="w-3 h-3 shrink-0" />
            <span>Applied by Ryzora, but overridden by <code className="bg-black/30 px-1 py-0.5 rounded text-amber-200 font-mono text-[10px]">{overriddenBy}</code></span>
          </div>
        )}

        {/* Warning if installation is blocked */}
        {isBlocked && (
          <div className="flex items-center gap-1.5 text-[11px] text-rose-400 font-medium">
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
              <span>Uninstall {packageItem.title}?</span>
            </div>

            <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
              Are you sure you want to uninstall <strong>{packageItem.title}</strong>? All downloaded theme assets and configurations will be cleanly removed.
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
                onClick={handleConfirmUninstall}
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
