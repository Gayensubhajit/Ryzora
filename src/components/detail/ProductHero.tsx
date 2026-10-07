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
  installStageText?: string;
  isInstalled: boolean;
  isUpdateAvailable: boolean;
  isBlocked: boolean;
  missingDependencies: string[];
  selectedTarget?: "sddm";
  onSelectTarget?: (target: "sddm") => void;
  isSessionLockSupported?: boolean;
  isLoginScreenSupported?: boolean;
  isActive?: boolean;
  activeTargets?: { sddm_login?: boolean; sddm?: boolean };
  installedTargets?: { sddm?: boolean };
  onApply?: (target?: "sddm") => void;
  onDeactivate?: (target?: "sddm") => void;
  onTest?: (target?: "sddm") => void;
  onUninstall?: () => void;
  isApplying?: boolean;
  isOverridden?: boolean;
  overriddenBy?: string | null;
  privilegedHelperInstalled?: boolean;
  onSetupPrivilegedHelper?: () => void;
  isSettingUpHelper?: boolean;
}

export const ProductHero: React.FC<ProductHeroProps> = ({
  packageItem,
  activeScreenshotIndex,
  onSelectScreenshot,
  onInstall,
  onOpenLightbox,
  isInstalling,
  installStageText,
  isInstalled,
  isUpdateAvailable,
  isBlocked,
  missingDependencies,
  selectedTarget: _selectedTarget = "sddm",
  onSelectTarget: _onSelectTarget,
  isSessionLockSupported: _isSessionLockSupported,
  isLoginScreenSupported = true,
  isActive = false,
  activeTargets = { sddm_login: false, sddm: false },
  installedTargets: _installedTargets,
  onApply,
  onDeactivate,
  onTest,
  onUninstall,
  isApplying = false,
  isOverridden = false,
  overriddenBy = null,
  privilegedHelperInstalled = true,
  onSetupPrivilegedHelper,
  isSettingUpHelper = false,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [showOverflow, setShowOverflow] = useState(false);

  const canTargetSddm = Boolean(packageItem.supports_login_screen && isLoginScreenSupported);
  const isSddmActive = Boolean(isActive || activeTargets.sddm || activeTargets.sddm_login);

  const screenshots =
    packageItem.screenshots && packageItem.screenshots.length > 0
      ? packageItem.screenshots
      : [packageItem.hero_image];

  const activeImage = screenshots[activeScreenshotIndex] || packageItem.hero_image;

  const handleConfirmUninstall = () => {
    setShowUninstallConfirm(false);
    if (onUninstall) {
      onUninstall();
    }
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[minmax(0,1.55fr)_minmax(360px,0.85fr)] gap-6 lg:gap-10 p-6 sm:p-8 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-[0_1px_3px_rgba(0,0,0,0.04),0_8px_24px_rgba(0,0,0,0.03)]">
      {/* ── Left Column: Media Hero (Large Crisp Preview) ── */}
      <div className="flex flex-col gap-3 min-w-0">
        <div className="relative rounded-xl overflow-hidden border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] aspect-video group">
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
                LOGIN SCREEN · SDDM
              </span>
              <span className="text-[11px] text-[var(--rz-text-muted)]">·</span>
              <span className="text-[11px] font-mono text-[var(--rz-text-muted)]">
                v{packageItem.version || "1.0.0"}
              </span>
            </div>

            <h1 className="text-2xl font-bold tracking-tight text-[var(--rz-text)] truncate">
              {packageItem.title}
            </h1>

            <p className="text-xs text-[var(--rz-text-secondary)] mt-1">
              By <span className="text-[var(--rz-text)] font-medium">{packageItem.author.name}</span>
            </p>
          </div>

          <p className="text-xs leading-relaxed text-[var(--rz-text-secondary)] line-clamp-3">
            {packageItem.description}
          </p>

          {/* Social Proof & Safety Signals */}
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs text-[var(--rz-text-secondary)] pt-1">
            {packageItem.rating && (
              <>
                <span className="flex items-center gap-1 font-semibold text-[var(--rz-text)]">
                  <Star className="w-3.5 h-3.5 text-amber-400 fill-amber-400" />
                  <span>{packageItem.rating.toFixed(1)}</span>
                </span>
                <span>·</span>
              </>
            )}
            <span>{formatDownloads(packageItem.downloads)} downloads</span>
            {packageItem.lockscreen?.provenance?.license && (
              <>
                <span>·</span>
                <span>Asset license: {packageItem.lockscreen.provenance.license}</span>
              </>
            )}
            <span>·</span>
            <span className="flex items-center gap-1 text-[var(--rz-text-muted)]">
              <ShieldCheck className="w-3.5 h-3.5 text-emerald-500" />
              <span>Safe to install · Reversible</span>
            </span>
          </div>

          {/* ── Targets Section: Available on this system (Visually Calm & Informative) ── */}
          <div className="space-y-2 pt-2">
            <span className="text-xs font-semibold text-[var(--rz-text)] block">
              Available on this system
            </span>

            <div className="space-y-1.5">
              {!canTargetSddm ? (
                <div className="p-3 rounded-xl border border-amber-500/30 bg-amber-500/5 text-xs space-y-1.5">
                  <div className="flex items-center gap-2 text-amber-500 font-semibold">
                    <AlertTriangle className="w-4 h-4 shrink-0" />
                    <span>Incompatible with current environment</span>
                  </div>
                  <p className="text-[11px] text-[var(--rz-text-secondary)] leading-relaxed">
                    This theme provides SDDM (Login Screen), but SDDM is not your active display manager.
                  </p>
                  <div className="pt-1.5 mt-1 border-t border-amber-500/20 text-[11px] space-y-1">
                    <span className="font-semibold text-[var(--rz-text)] block">To enable SDDM login screens:</span>
                    <code className="block font-mono text-[10px] text-[var(--rz-accent)] bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] p-1.5 rounded select-all">
                      sudo pacman -S sddm && sudo systemctl enable --now sddm
                    </code>
                  </div>
                </div>
              ) : (
                <div className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] text-xs">
                  <div className="flex items-center gap-3">
                    <Check className="w-4 h-4 text-emerald-500 stroke-[2.5] shrink-0" />
                    <div>
                      <span className="block text-xs font-semibold text-[var(--rz-text)]">Login Screen</span>
                      <span className="block text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                        SDDM
                      </span>
                    </div>
                  </div>

                  {!privilegedHelperInstalled && onSetupPrivilegedHelper && !isInstalled ? (
                    <button
                      type="button"
                      disabled={isSettingUpHelper}
                      onClick={(e) => {
                        e.stopPropagation();
                        onSetupPrivilegedHelper();
                      }}
                      className="px-2.5 py-1 rounded-lg text-[11px] font-medium bg-amber-500/15 text-amber-500 hover:bg-amber-500/25 border border-amber-500/30 transition-all cursor-pointer flex items-center gap-1 shrink-0"
                    >
                      <ShieldCheck className="w-3 h-3" />
                      <span>{isSettingUpHelper ? "Setting Up…" : "Set Up System Integration"}</span>
                    </button>
                  ) : null}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Bottom Action Bar: Prominent, state-aware single-row action area ── */}
        <div className="pt-3 border-t border-[var(--rz-border-subtle)]">
          {!isInstalled ? (
            <button
              type="button"
              disabled={isBlocked || isInstalling || !canTargetSddm}
              onClick={onInstall}
              className={[
                "w-full py-2.5 px-5 rounded-xl text-xs font-semibold transition-all select-none flex items-center justify-center gap-2 shadow-xs",
                isBlocked || !canTargetSddm
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                  : "bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white cursor-pointer",
              ].join(" ")}
            >
              {isInstalling ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>{installStageText || "Installing…"}</span>
                </>
              ) : !canTargetSddm ? (
                <span>Incompatible with System</span>
              ) : isUpdateAvailable ? (
                <>
                  <DownloadCloud className="w-4 h-4" />
                  <span>Update</span>
                </>
              ) : (
                <span>Install</span>
              )}
            </button>
          ) : isSddmActive ? (
            /* Active State: Status indicator on left, actions on right */
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-2 text-xs font-semibold text-emerald-400 shrink-0">
                <span className="w-2 h-2 rounded-full bg-emerald-500 shadow-xs" />
                <span>Active · Login Screen</span>
              </div>

              <div className="flex items-center gap-2">
                {onTest && (
                  <button
                    type="button"
                    onClick={() => onTest("sddm")}
                    className="px-3.5 py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
                    title="Test on login screen"
                  >
                    <Play className="w-3.5 h-3.5 text-[var(--rz-accent)] fill-[var(--rz-accent)]/20" />
                    <span>Test</span>
                  </button>
                )}

                {onDeactivate && (
                  <button
                    type="button"
                    disabled={isApplying}
                    onClick={() => onDeactivate("sddm")}
                    className="py-2 px-4 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-amber-500/40 text-amber-500 hover:text-amber-400 transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-xs disabled:opacity-50"
                    title="Deactivate Login Screen"
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Deactivate</span>
                  </button>
                )}

                {onUninstall && (
                  <div className="relative">
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        setShowOverflow(!showOverflow);
                      }}
                      className="p-2 rounded-xl border border-[var(--rz-border-strong)] bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer select-none shadow-xs"
                      aria-label="More options"
                      title="More options"
                    >
                      <MoreHorizontal className="w-4 h-4" />
                    </button>

                    {showOverflow && (
                      <div
                        className="absolute right-0 bottom-full mb-2 w-48 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] shadow-xl p-1.5 z-40 animate-in fade-in zoom-in-95 duration-100"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            setShowOverflow(false);
                            setShowUninstallConfirm(true);
                          }}
                          className="w-full px-2.5 py-1.5 rounded-lg text-xs font-medium text-rose-500 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                        >
                          <Trash2 className="w-3.5 h-3.5 text-rose-500" />
                          <span>Uninstall</span>
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          ) : (
            /* Installed (Inactive) State: Status indicator on left, actions on right */
            <div className="flex items-center justify-between gap-3">
              <div className="flex items-center gap-1.5 text-xs font-medium text-emerald-500 shrink-0">
                <Check className="w-3.5 h-3.5 stroke-[2.5]" />
                <span>Installed</span>
              </div>

              <div className="flex items-center gap-2">
                {onTest && (
                  <button
                    type="button"
                    onClick={() => onTest("sddm")}
                    className="px-3.5 py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
                    title="Test on login screen"
                  >
                    <Play className="w-3.5 h-3.5 text-[var(--rz-accent)] fill-[var(--rz-accent)]/20" />
                    <span>Test</span>
                  </button>
                )}

                <button
                  type="button"
                  disabled={isApplying}
                  onClick={() => onApply?.("sddm")}
                  className="py-2 px-5 rounded-xl text-xs font-semibold bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-xs disabled:opacity-60"
                >
                  {isApplying ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>Applying...</span>
                    </>
                  ) : (
                    <span>Apply to Login Screen</span>
                  )}
                </button>

                {onUninstall && (
                  <div className="relative">
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        setShowOverflow(!showOverflow);
                      }}
                      className="p-2 rounded-xl border border-[var(--rz-border-strong)] bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer select-none shadow-xs"
                      aria-label="More options"
                      title="More options"
                    >
                      <MoreHorizontal className="w-4 h-4" />
                    </button>

                    {showOverflow && (
                      <div
                        className="absolute right-0 bottom-full mb-2 w-48 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] shadow-xl p-1.5 z-40 animate-in fade-in zoom-in-95 duration-100"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          onClick={() => {
                            setShowOverflow(false);
                            setShowUninstallConfirm(true);
                          }}
                          className="w-full px-2.5 py-1.5 rounded-lg text-xs font-medium text-rose-500 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                        >
                          <Trash2 className="w-3.5 h-3.5 text-rose-500" />
                          <span>Uninstall</span>
                        </button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Overridden status indicator */}
          {isOverridden && overriddenBy && (
            <div className="flex items-center gap-1.5 text-[11px] text-amber-500 font-medium pt-2">
              <AlertTriangle className="w-3 h-3 shrink-0" />
              <span>
                Applied, but overridden by <code className="px-1 py-0.5 rounded bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[10px] font-mono">{overriddenBy}</code>
              </span>
            </div>
          )}

          {/* Blocked indicator */}
          {(isBlocked || !canTargetSddm) && (
            <div className="flex items-center gap-1.5 text-[11px] text-amber-500 font-medium pt-2">
              <AlertTriangle className="w-3 h-3 shrink-0" />
              <span>
                {!canTargetSddm
                  ? "Installation unavailable: SDDM login runtime not detected or supported on this host"
                  : missingDependencies && missingDependencies.length > 0
                  ? `Installation blocked: missing ${missingDependencies.join(", ")}`
                  : "Installation unavailable on current system configuration"}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* ── Safe Active / Clean Uninstall Confirmation Dialog ── */}
      {showUninstallConfirm && onUninstall && (
        <div
          className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-4 backdrop-blur-xs animate-in fade-in duration-150"
          onClick={() => setShowUninstallConfirm(false)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] p-6 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2 text-rose-500 font-semibold text-sm">
              <Trash2 className="w-4 h-4" />
              <span>Uninstall {packageItem.title}?</span>
            </div>

            {isSddmActive ? (
              <>
                <div className="space-y-2">
                  <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                    This lock screen is currently active for:
                  </p>

                  <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
                    <div className="flex items-center gap-2 text-xs font-medium text-[var(--rz-text)]">
                      <Check className="w-4 h-4 text-emerald-500 stroke-[2.5]" />
                      <span>Login Screen</span>
                      <span className="text-[11px] text-[var(--rz-text-muted)] font-mono ml-auto">SDDM</span>
                    </div>
                  </div>

                  <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                    Before uninstalling, Ryzora will deactivate the active target and restore the previous system login configuration.
                  </p>
                </div>

                <div className="flex items-center justify-end gap-2 pt-3 border-t border-[var(--rz-border-subtle)]">
                  <button
                    type="button"
                    disabled={isApplying}
                    onClick={() => setShowUninstallConfirm(false)}
                    className="px-3.5 py-1.5 rounded-lg border border-[var(--rz-border-strong)] text-xs text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] cursor-pointer disabled:opacity-50"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    disabled={isApplying}
                    onClick={handleConfirmUninstall}
                    className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold cursor-pointer shadow-xs flex items-center gap-1.5 disabled:opacity-50"
                  >
                    {isApplying ? (
                      <>
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        <span>Deactivating & Uninstalling...</span>
                      </>
                    ) : (
                      <span>Deactivate & Uninstall</span>
                    )}
                  </button>
                </div>
              </>
            ) : (
              <>
                <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                  Theme files and configuration for <strong>{packageItem.title}</strong> will be cleanly removed from your system.
                </p>

                <div className="flex items-center justify-end gap-2 pt-3 border-t border-[var(--rz-border-subtle)]">
                  <button
                    type="button"
                    disabled={isApplying}
                    onClick={() => setShowUninstallConfirm(false)}
                    className="px-3.5 py-1.5 rounded-lg border border-[var(--rz-border-strong)] text-xs text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] cursor-pointer disabled:opacity-50"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    disabled={isApplying}
                    onClick={handleConfirmUninstall}
                    className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold cursor-pointer shadow-xs flex items-center gap-1.5 disabled:opacity-50"
                  >
                    {isApplying ? (
                      <>
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        <span>Uninstalling...</span>
                      </>
                    ) : (
                      <span>Uninstall</span>
                    )}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
