import React, { useState } from "react";
import {
  DownloadCloud,
  Loader2,
  Check,
  AlertTriangle,
  Eye,
  Sparkles,
  ShieldCheck,
  Download,
  Star,
  Calendar,
  Lock,
  Monitor,
  Layers,
  Maximize2,
  ShieldAlert,
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
  onPreview: () => void;
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
}

export const ProductHero: React.FC<ProductHeroProps> = ({
  packageItem,
  activeScreenshotIndex,
  onSelectScreenshot,
  onPreview,
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
}) => {
  const [showAllTags, setShowAllTags] = useState(false);

  const isDualTarget =
    Boolean(packageItem.supports_session_lock && packageItem.supports_login_screen);

  const subtype = packageItem.tags?.find((t) =>
    ["hyprlock", "quickshell", "swaylock", "sddm"].includes(t.toLowerCase())
  ) || "Lock Screen";

  const screenshots =
    packageItem.screenshots && packageItem.screenshots.length > 0
      ? packageItem.screenshots
      : [packageItem.hero_image];

  const activeImage = screenshots[activeScreenshotIndex] || packageItem.hero_image;

  const visibleTags = showAllTags ? packageItem.tags : packageItem.tags?.slice(0, 4) || [];
  const hiddenTagsCount = (packageItem.tags?.length || 0) - visibleTags.length;

  const getActionLabel = () => {
    if (isInstalling) return "Installing...";
    if (isUpdateAvailable) return "Update";
    if (isInstalled) return "Installed";
    if (isDualTarget) {
      if (selectedTarget === "quickshell") return "Install Session Lock";
      if (selectedTarget === "sddm") return "Install Login Screen";
      return "Install Both";
    }
    return packageItem.supports_login_screen ? "Install Login Screen" : "Install Session Lock";
  };

  return (
    <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 p-4 sm:p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-sm">
      {/* ── Left Column: Media Hero (60-65%) ── */}
      <div className="lg:col-span-7 flex flex-col gap-3">
        <div className="relative rounded-xl overflow-hidden border border-[var(--rz-border-subtle)] group">
          {/* Main Media Viewer */}
          <MediaPreview
            poster={activeScreenshotIndex === 0 && packageItem.preview_poster_url ? packageItem.preview_poster_url : activeImage}
            videoSrc={activeScreenshotIndex === 0 ? packageItem.preview_video_url : undefined}
            animatedSrc={activeScreenshotIndex === 0 ? packageItem.lockscreen?.media.preview_animated : undefined}
            mediaType={activeScreenshotIndex === 0 ? (packageItem.media_type || "image") : "image"}
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

          {/* ── Capability-Driven Target Selector (When Dual-Target) ── */}
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
                  } ${!isSessionLockSupported ? "opacity-50 cursor-not-allowed" : ""}`}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="radio"
                      name="lock_target"
                      value="quickshell"
                      checked={selectedTarget === "quickshell"}
                      disabled={!isSessionLockSupported}
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
                    User Space
                  </span>
                </label>

                {/* Login Screen (SDDM) Option */}
                <label
                  className={`flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition-all ${
                    selectedTarget === "sddm"
                      ? "bg-amber-500/15 border-amber-500/50 text-[var(--rz-text)] font-semibold"
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
                      <span className="block text-[9px] text-[var(--rz-text-muted)]">Display Manager Greeter</span>
                    </div>
                  </div>
                  <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-amber-500/20 text-amber-800 dark:text-amber-300">
                    Admin / Root
                  </span>
                </label>

                {/* Both Dual Option */}
                <label
                  className={`flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition-all ${
                    selectedTarget === "both"
                      ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-text)] font-semibold"
                      : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:bg-[var(--rz-surface)]/80"
                  } ${!isSessionLockSupported ? "opacity-50 cursor-not-allowed" : ""}`}
                >
                  <div className="flex items-center gap-2">
                    <input
                      type="radio"
                      name="lock_target"
                      value="both"
                      checked={selectedTarget === "both"}
                      disabled={!isSessionLockSupported}
                      onChange={() => onSelectTarget("both")}
                      className="accent-[var(--rz-accent)]"
                    />
                    <Layers className="w-3.5 h-3.5 text-sky-400 shrink-0" />
                    <div>
                      <span className="block text-[11px] font-medium">Both (Session Lock + Login Screen)</span>
                      <span className="block text-[9px] text-[var(--rz-text-muted)]">Unified appearance at boot & lock</span>
                    </div>
                  </div>
                  <span className="px-1.5 py-0.5 rounded text-[9px] font-mono bg-purple-500/20 text-purple-800 dark:text-purple-300">
                    Dual
                  </span>
                </label>
              </div>

              {/* Privilege boundary notice if SDDM or Both is active */}
              {(selectedTarget === "sddm" || selectedTarget === "both") && (
                <div className="mt-2 flex items-center gap-1.5 text-[10px] text-amber-400 font-medium">
                  <ShieldAlert className="w-3 h-3 shrink-0" />
                  <span>Requires elevated administrator privileges (pkexec)</span>
                </div>
              )}
            </div>
          )}

          {/* Concise Subtitle / Description */}
          <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed mt-2 mb-3">
            {packageItem.subtitle || packageItem.description}
          </p>

          {/* Expandable Tags */}
          {packageItem.tags && packageItem.tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5 mb-4">
              {visibleTags.map((tag) => (
                <span
                  key={tag}
                  className="px-2 py-0.5 rounded-md text-[10px] font-medium bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]"
                >
                  {tag}
                </span>
              ))}
              {!showAllTags && hiddenTagsCount > 0 && (
                <button
                  type="button"
                  onClick={() => setShowAllTags(true)}
                  className="px-2 py-0.5 rounded-md text-[10px] font-semibold text-[var(--rz-accent-text)] hover:underline cursor-pointer"
                >
                  +{hiddenTagsCount} more
                </button>
              )}
            </div>
          )}
        </div>

        {/* ── Primary Action Buttons ── */}
        <div className="flex items-center gap-2.5 pt-2 border-t border-[var(--rz-border-subtle)]">
          <button
            type="button"
            onClick={onPreview}
            className="flex-1 py-2 rounded-xl text-xs font-semibold border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer select-none flex items-center justify-center gap-1.5"
          >
            <Eye className="w-3.5 h-3.5" />
            <span>Dry Run Preview</span>
          </button>

          <button
            type="button"
            disabled={isBlocked || isInstalling}
            onClick={onInstall}
            className={[
              "flex-1 py-2 rounded-xl text-xs font-bold transition-all cursor-pointer select-none flex items-center justify-center gap-1.5 shadow-md",
              isBlocked
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                : isInstalled && !isUpdateAvailable
                ? "bg-emerald-600 hover:bg-emerald-500 text-white"
                : (selectedTarget === "sddm" || selectedTarget === "both")
                ? "bg-amber-600 hover:bg-amber-500 text-white"
                : "bg-[var(--rz-accent)] hover:bg-[var(--rz-accent)]/90 text-white",
            ].join(" ")}
          >
            {isInstalling ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Installing...</span>
              </>
            ) : isUpdateAvailable ? (
              <>
                <DownloadCloud className="w-3.5 h-3.5" />
                <span>Update</span>
              </>
            ) : isInstalled ? (
              <>
                <Check className="w-3.5 h-3.5" />
                <span>Installed</span>
              </>
            ) : (
              <>
                <DownloadCloud className="w-3.5 h-3.5" />
                <span>{getActionLabel()}</span>
              </>
            )}
          </button>
        </div>

        {/* Warning if installation is blocked */}
        {isBlocked && (
          <div className="mt-2 flex items-center gap-1.5 text-[11px] text-rose-400 font-medium">
            <AlertTriangle className="w-3 h-3 shrink-0" />
            <span>Installation blocked: missing {missingDependencies.join(", ")}</span>
          </div>
        )}
      </div>
    </div>
  );
};
