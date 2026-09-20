import React, { useState } from "react";
import {
  DownloadCloud,
  Loader2,
  Check,
  AlertTriangle,
  ShieldCheck,
  Star,
  Maximize2,
  Trash2,
  MoreHorizontal,
  Play,
  RotateCcw,
} from "lucide-react";
import { PackageItem } from "../../types";
import { formatDownloads } from "../catalogue/catalogueUtils";
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
  installedTargets?: { quickshell: boolean; sddm: boolean };
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
  isActive = false,
  activeTargets = { quickshell: false, sddm: false },
  installedTargets = { quickshell: false, sddm: false },
  onApply,
  onDeactivate,
  onTest,
  onUninstall,
  isApplying = false,
  isOverridden = false,
  overriddenBy = null,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [showOverflow, setShowOverflow] = useState(false);

  const canTargetQs = Boolean(packageItem.supports_session_lock && isSessionLockSupported);
  const canTargetSddm = Boolean(packageItem.supports_login_screen && isLoginScreenSupported);
  const isDualTarget = canTargetQs && canTargetSddm;

  const subtype =
    packageItem.tags?.find((t) =>
      ["hyprlock", "quickshell", "swaylock", "sddm"].includes(t.toLowerCase())
    ) || "Lock Screen";

  const screenshots =
    packageItem.screenshots && packageItem.screenshots.length > 0
      ? packageItem.screenshots
      : [packageItem.hero_image];

  const activeImage = screenshots[activeScreenshotIndex] || packageItem.hero_image;

  // Pre-install selection state
  const isQsChecked = selectedTarget === "both" || selectedTarget === "quickshell";
  const isSddmChecked = selectedTarget === "both" || selectedTarget === "sddm";

  const handleToggleQs = () => {
    if (!onSelectTarget || !isDualTarget || isInstalled) return;
    if (isQsChecked && isSddmChecked) {
      onSelectTarget("sddm");
    } else if (!isQsChecked) {
      onSelectTarget(isSddmChecked ? "both" : "quickshell");
    }
  };

  const handleToggleSddm = () => {
    if (!onSelectTarget || !isDualTarget || isInstalled) return;
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

  const targetSummary =
    selectedTarget === "both"
      ? "Session + Login"
      : selectedTarget === "sddm"
      ? "Login Screen"
      : "Session Lock";

  const applyLabel =
    selectedTarget === "both"
      ? "Apply"
      : selectedTarget === "sddm"
      ? "Apply to Login Screen"
      : "Apply to Session";

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1.55fr)_minmax(360px,0.85fr)] gap-6 lg:gap-10 p-6 sm:p-8 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-[0_1px_3px_rgba(0,0,0,0.04),0_8px_24px_rgba(0,0,0,0.03)]">
      {/* ── Left Column: Media Hero (Large Crisp Preview) ── */}
      <div className="flex flex-col gap-3 min-w-0">
        <div className="relative rounded-xl overflow-hidden border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] aspect-video group">
          {/* Main Media Viewer */}
          <MediaPreview
            poster={
              activeScreenshotIndex === 0 && (packageItem.preview_poster_url || packageItem.hero_image)
                ? packageItem.preview_poster_url || packageItem.hero_image
                : activeImage
            }
            videoSrc={
              activeScreenshotIndex === 0
                ? packageItem.preview_video_url || packageItem.preview_video || packageItem.lockscreen?.media?.preview_video
                : undefined
            }
            animatedSrc={
              activeScreenshotIndex === 0
                ? packageItem.preview_animated || packageItem.lockscreen?.media?.preview_animated
                : undefined
            }
            mediaType={
              activeScreenshotIndex === 0
                ? packageItem.media_type ||
                  (packageItem.preview_video_url || packageItem.preview_video
                    ? "video"
                    : packageItem.preview_animated
                    ? "animated"
                    : "image")
                : "image"
            }
            alt={packageItem.title}
            mode="hero"
            aspectRatio="16/9"
            showBadge={false}
            className="w-full h-full object-cover"
          />

          {/* Lightbox Trigger */}
          <button
            type="button"
            onClick={onOpenLightbox}
            className="absolute top-3 right-3 p-2 rounded-lg bg-black/50 hover:bg-black/75 text-white/90 border border-white/15 backdrop-blur-md transition-all shadow-sm z-20 cursor-pointer"
            aria-label="View fullscreen image"
            title="Expand fullscreen preview"
          >
            <Maximize2 className="w-4 h-4" />
          </button>
        </div>

        {/* Thumbnail Selector Gallery */}
        {screenshots.length > 1 && (
          <PreviewGallery
            screenshots={screenshots}
            activeIndex={activeScreenshotIndex}
            onSelect={onSelectScreenshot}
            title={packageItem.title}
          />
        )}
      </div>

      {/* ── Right Column: Apple-Inspired Product Identity & Lifecycle ── */}
      <div className="flex flex-col justify-between min-w-0 py-1 space-y-5">
        <div className="space-y-4">
          {/* Metadata Header */}
          <div>
            <div className="flex items-center gap-2 mb-1.5">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-[var(--rz-text-muted)]">
                Lock Screens · {subtype}
              </span>
              <span className="text-[11px] text-[var(--rz-text-muted)]">·</span>
              <span className="text-[11px] font-mono text-[var(--rz-text-muted)]">
                v{packageItem.version}
              </span>
            </div>

            {/* Title */}
            <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-[var(--rz-text)] leading-tight">
              {packageItem.title}
            </h1>

            {/* Description */}
            <p className="text-xs sm:text-sm text-[var(--rz-text-secondary)] leading-relaxed mt-2 line-clamp-3">
              {packageItem.description || "A beautifully crafted lock screen experience for your desktop."}
            </p>
          </div>

          {/* Author & Rating Single-Line Header */}
          <div className="flex flex-wrap items-center gap-3 text-xs text-[var(--rz-text-secondary)] pt-1 border-t border-[var(--rz-border-subtle)]">
            <span className="font-medium text-[var(--rz-text)]">{packageItem.author.name}</span>
            <span>·</span>
            <span className="flex items-center gap-1 font-medium">
              <Star className="w-3.5 h-3.5 text-amber-500 fill-amber-500" />
              <span>{packageItem.rating > 0 ? packageItem.rating.toFixed(1) : "4.9"}</span>
            </span>
            <span>·</span>
            <span>{formatDownloads(packageItem.downloads)} downloads</span>
            <span>·</span>
            <span className="flex items-center gap-1 text-[var(--rz-text-muted)]">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-500" />
              <span>Safe to install · Reversible</span>
            </span>
          </div>

          {/* ── Targets Section: Mode-Aware (Checkboxes BEFORE install, Status Badges AFTER install) ── */}
          <div className="space-y-2 pt-2">
            <span className="text-xs font-semibold text-[var(--rz-text)] block">
              {!isInstalled ? "Installation" : isActive ? "Targets" : "Installed on"}
            </span>

            <div className="space-y-1.5">
              {/* Session Lock Row */}
              {canTargetQs && (!isInstalled || isQsChecked) && (
                <div
                  onClick={handleToggleQs}
                  className={`flex items-center justify-between p-3 rounded-xl border text-xs transition-all ${
                    !isInstalled && isDualTarget ? "cursor-pointer" : "cursor-default"
                  } ${
                    !isInstalled
                      ? isQsChecked
                        ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] text-[var(--rz-text)] shadow-xs"
                        : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)] opacity-70"
                      : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text)]"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    {!isInstalled ? (
                      <div
                        className={`w-4 h-4 rounded flex items-center justify-center border transition-all ${
                          isQsChecked
                            ? "bg-[var(--rz-accent)] border-[var(--rz-accent)] text-white"
                            : "border-[var(--rz-border-strong)] bg-[var(--rz-surface)]"
                        }`}
                      >
                        {isQsChecked && <Check className="w-3 h-3 stroke-[3]" />}
                      </div>
                    ) : activeTargets.quickshell ? (
                      <span className="w-2 h-2 rounded-full bg-emerald-500 shrink-0" />
                    ) : (
                      <Check className="w-3.5 h-3.5 text-emerald-500 stroke-[2.5] shrink-0" />
                    )}
                    <div>
                      <span className="block text-xs font-medium">Session Lock</span>
                      <span className="block text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                        Hyprland · Wayland
                      </span>
                    </div>
                  </div>

                  {isInstalled && (
                    <span
                      className={`text-[11px] font-medium px-2 py-0.5 rounded-md border ${
                        activeTargets.quickshell
                          ? "bg-emerald-500/10 text-emerald-500 border-emerald-500/30 font-semibold"
                          : "bg-[var(--rz-surface)] text-[var(--rz-text-muted)] border-[var(--rz-border-subtle)]"
                      }`}
                    >
                      {activeTargets.quickshell ? "Active" : installedTargets.quickshell ? "Installed" : "Ready"}
                    </span>
                  )}
                </div>
              )}

              {/* Login Screen Row */}
              {canTargetSddm && (!isInstalled || isSddmChecked) && (
                <div
                  onClick={handleToggleSddm}
                  className={`flex items-center justify-between p-3 rounded-xl border text-xs transition-all ${
                    !isInstalled && isDualTarget ? "cursor-pointer" : "cursor-default"
                  } ${
                    !isInstalled
                      ? isSddmChecked
                        ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] text-[var(--rz-text)] shadow-xs"
                        : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)] opacity-70"
                      : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text)]"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    {!isInstalled ? (
                      <div
                        className={`w-4 h-4 rounded flex items-center justify-center border transition-all ${
                          isSddmChecked
                            ? "bg-[var(--rz-accent)] border-[var(--rz-accent)] text-white"
                            : "border-[var(--rz-border-strong)] bg-[var(--rz-surface)]"
                        }`}
                      >
                        {isSddmChecked && <Check className="w-3 h-3 stroke-[3]" />}
                      </div>
                    ) : activeTargets.sddm ? (
                      <span className="w-2 h-2 rounded-full bg-emerald-500 shrink-0" />
                    ) : (
                      <Check className="w-3.5 h-3.5 text-emerald-500 stroke-[2.5] shrink-0" />
                    )}
                    <div>
                      <span className="block text-xs font-medium">Login Screen</span>
                      <span className="block text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                        SDDM
                      </span>
                    </div>
                  </div>

                  {isInstalled && (
                    <span
                      className={`text-[11px] font-medium px-2 py-0.5 rounded-md border ${
                        activeTargets.sddm
                          ? "bg-emerald-500/10 text-emerald-500 border-emerald-500/30 font-semibold"
                          : "bg-[var(--rz-surface)] text-[var(--rz-text-muted)] border-[var(--rz-border-subtle)]"
                      }`}
                    >
                      {activeTargets.sddm ? "Active" : installedTargets.sddm ? "Installed" : "Ready"}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Action Section: 3-State Lifecycle (Available -> Installed -> Active) ── */}
        <div className="pt-3 border-t border-[var(--rz-border-subtle)] space-y-2.5">
          {!isInstalled ? (
            /* STATE 1: AVAILABLE (Not installed) -> [ Install ] */
            <button
              type="button"
              disabled={isBlocked || isInstalling || (!canTargetQs && !canTargetSddm)}
              onClick={onInstall}
              className={[
                "w-full py-2.5 px-5 rounded-xl text-xs font-semibold transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-xs",
                isBlocked
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                  : "bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white",
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
                <span>Install</span>
              )}
            </button>
          ) : !isActive ? (
            /* STATE 2: INSTALLED BUT NOT ACTIVE -> [ Apply ] + [ ⋯ ] */
            <div className="space-y-2.5">
              <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-500 py-0.5">
                <Check className="w-3.5 h-3.5 stroke-[2.5]" />
                <span>Installed</span>
              </div>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  disabled={isApplying}
                  onClick={onApply}
                  className="flex-1 py-2.5 px-5 rounded-xl text-xs font-semibold bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-xs"
                >
                  {isApplying ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>Applying...</span>
                    </>
                  ) : (
                    <span>{applyLabel}</span>
                  )}
                </button>

                {onUninstall && (
                  <div className="relative">
                    <button
                      type="button"
                      onClick={() => setShowOverflow(!showOverflow)}
                      className="p-2.5 rounded-xl border border-[var(--rz-border-strong)] bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer select-none shadow-xs"
                      aria-label="More options"
                      title="More options"
                    >
                      <MoreHorizontal className="w-4 h-4" />
                    </button>

                    {showOverflow && (
                      <div
                        className="absolute right-0 bottom-full mb-2 w-36 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] shadow-xl p-1 z-40 animate-in fade-in zoom-in-95 duration-100"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            setShowOverflow(false);
                            setShowUninstallConfirm(true);
                          }}
                          className="w-full px-3 py-2 rounded-lg text-xs font-medium text-rose-500 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
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
          ) : (
            /* STATE 3: INSTALLED + ACTIVE -> [ Test ] + [ ⋯ ] (with Deactivate & Uninstall) */
            <div className="space-y-2.5">
              <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-500 py-0.5">
                <Check className="w-3.5 h-3.5 stroke-[2.5]" />
                <span>Active for {targetSummary}</span>
              </div>

              <div className="flex items-center gap-2">
                {onTest && (
                  <button
                    type="button"
                    onClick={onTest}
                    className="flex-1 py-2.5 px-4 rounded-xl text-xs font-medium bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-colors cursor-pointer select-none flex items-center justify-center gap-1.5 shadow-xs"
                    title="Launch isolated lockscreen test"
                  >
                    <span>Test</span>
                  </button>
                )}

                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setShowOverflow(!showOverflow)}
                    className="p-2.5 rounded-xl border border-[var(--rz-border-strong)] bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer select-none shadow-xs"
                    aria-label="More options"
                    title="More options"
                  >
                    <MoreHorizontal className="w-4 h-4" />
                  </button>

                  {showOverflow && (
                    <div
                      className="absolute right-0 bottom-full mb-2 w-40 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] shadow-xl p-1 z-40 animate-in fade-in zoom-in-95 duration-100"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {onTest && (
                        <button
                          type="button"
                          onClick={() => {
                            setShowOverflow(false);
                            onTest();
                          }}
                          className="w-full px-3 py-2 rounded-lg text-xs font-medium text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] flex items-center gap-2 transition-colors cursor-pointer text-left"
                        >
                          <Play className="w-3.5 h-3.5 text-[var(--rz-text-muted)]" />
                          <span>Test</span>
                        </button>
                      )}

                      {onDeactivate && (
                        <button
                          type="button"
                          disabled={isApplying}
                          onClick={() => {
                            setShowOverflow(false);
                            onDeactivate();
                          }}
                          className="w-full px-3 py-2 rounded-lg text-xs font-medium text-amber-500 hover:bg-amber-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                        >
                          <RotateCcw className="w-3.5 h-3.5 text-amber-500" />
                          <span>Deactivate</span>
                        </button>
                      )}

                      {onUninstall && (
                        <>
                          <div className="my-1 border-t border-[var(--rz-border-subtle)]" />
                          <button
                            type="button"
                            onClick={() => {
                              setShowOverflow(false);
                              setShowUninstallConfirm(true);
                            }}
                            className="w-full px-3 py-2 rounded-lg text-xs font-medium text-rose-500 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                          >
                            <Trash2 className="w-3.5 h-3.5 text-rose-500" />
                            <span>Uninstall</span>
                          </button>
                        </>
                      )}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Overridden status indicator */}
          {isOverridden && overriddenBy && (
            <div className="flex items-center gap-1.5 text-[11px] text-amber-500 font-medium pt-1">
              <AlertTriangle className="w-3 h-3 shrink-0" />
              <span>
                Applied, but overridden by <code className="px-1 py-0.5 rounded bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[10px] font-mono">{overriddenBy}</code>
              </span>
            </div>
          )}

          {/* Blocked indicator */}
          {isBlocked && (
            <div className="flex items-center gap-1.5 text-[11px] text-rose-500 font-medium pt-1">
              <AlertTriangle className="w-3 h-3 shrink-0" />
              <span>Installation blocked: missing {missingDependencies.join(", ")}</span>
            </div>
          )}
        </div>
      </div>

      {/* ── Clean Uninstall Confirmation Dialog ── */}
      {showUninstallConfirm && onUninstall && (
        <div
          className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4 backdrop-blur-xs animate-in fade-in duration-150"
          onClick={() => setShowUninstallConfirm(false)}
        >
          <div
            className="w-full max-w-sm rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] p-6 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 text-rose-500 font-semibold text-sm">
              <Trash2 className="w-4 h-4" />
              <span>Uninstall {packageItem.title}?</span>
            </div>

            <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
              Theme files and configuration for <strong>{packageItem.title}</strong> will be cleanly removed from your system.
            </p>

            <div className="flex items-center justify-end gap-2 pt-3 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="px-3.5 py-1.5 rounded-lg border border-[var(--rz-border-strong)] text-xs text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleConfirmUninstall}
                className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold cursor-pointer shadow-xs"
              >
                Uninstall
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
