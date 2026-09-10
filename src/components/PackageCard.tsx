import React from "react";
import { Star, ShieldCheck, Sparkles } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";
import { MediaPreview } from "./media/MediaPreview";

interface PackageCardProps {
  packageItem: PackageItem;
}

function compareSemver(v1: string, v2: string): number {
  const parse = (v: string) =>
    v
      .replace(/^v/, "")
      .split("-")[0]
      .split(".")
      .map((n) => parseInt(n, 10) || 0);
  const p1 = parse(v1);
  const p2 = parse(v2);
  for (let i = 0; i < Math.max(p1.length, p2.length); i++) {
    const num1 = p1[i] || 0;
    const num2 = p2[i] || 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }
  return 0;
}

export const PackageCard: React.FC<PackageCardProps> = ({ packageItem }) => {
  const {
    setSelectedPackage,
    installedPackages,
    checkCompatibility,
    repositories,
    activeLockscreen,
    systemInfo,
  } = useApp();

  const installedRecord = installedPackages.find((p) => p.package_id === packageItem.id);
  const isInstalled = !!installedRecord;
  const isUpdateAvailable =
    isInstalled && installedRecord
      ? compareSemver(packageItem.version, installedRecord.version) > 0
      : false;

  const isActive = Boolean(
    (activeLockscreen?.quickshell && (activeLockscreen.quickshell === packageItem.id || activeLockscreen.quickshell === packageItem.id.replace(/^lockscreen-(qylock-)?/, ""))) ||
    (activeLockscreen?.sddm && (activeLockscreen.sddm === packageItem.id || activeLockscreen.sddm === packageItem.id.replace(/^lockscreen-(qylock-)?/, "")))
  );

  const repo = repositories.find((r) => r.id === packageItem.repository_id);
  // offline status
  const isOffline = repo && repo.status === "offline" && !packageItem.is_cached;
  void isOffline;

  const compat = checkCompatibility(packageItem);

  // If installed or active or works via SDDM, do not show false "Missing: quickshell"
  const isWorkingThroughTarget =
    isInstalled ||
    isActive ||
    (packageItem.supports_login_screen && systemInfo?.installed_components?.some(c => c.binary.toLowerCase() === "sddm" && c.installed));

  const componentNames = packageItem.components
    .map((c) => c.name.split(" ")[0])
    .slice(0, 3)
    .join(" · ");

  const poster =
    packageItem.preview_poster_url ||
    packageItem.hero_image ||
    packageItem.lockscreen?.media.poster;
  const videoSrc =
    packageItem.preview_video_url ||
    packageItem.preview_video ||
    packageItem.lockscreen?.media.preview_video;
  const animatedSrc =
    packageItem.preview_animated ||
    packageItem.lockscreen?.media.preview_animated;
  const mediaType =
    packageItem.media_type ||
    (videoSrc ? "video" : animatedSrc ? "animated" : "image");

  return (
    <div
      onClick={() => setSelectedPackage(packageItem)}
      className="group flex flex-col rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-surface-elevated)] transition-all duration-200 cursor-pointer overflow-hidden select-none shadow-xs hover:shadow-md"
    >
      {/* Aspect-16:9 Media Preview */}
      <div className="relative aspect-video w-full bg-[var(--bg-canvas)] border-b border-[var(--border-subtle)] overflow-hidden">
        <MediaPreview
          poster={poster || ""}
          videoSrc={videoSrc}
          animatedSrc={animatedSrc}
          mediaType={mediaType}
          alt={packageItem.title}
          mode="card"
          aspectRatio="16/9"
          showBadge={false}
          className="w-full h-full object-cover transition-transform duration-500 ease-out group-hover:scale-105"
        />

        {/* Top-Left: Trust / Provider indicator */}
        <div className="absolute top-2 left-2 flex items-center gap-1 z-10">
          {packageItem.trust_tier === "official" && (
            <div
              className="px-1.5 py-0.5 rounded text-[9px] font-semibold uppercase bg-amber-500/25 text-amber-300 border border-amber-500/40 flex items-center gap-1 shadow-sm backdrop-blur-xs"
              title="Official Ryzora Package"
            >
              <Sparkles className="w-2.5 h-2.5 text-amber-400" />
              <span>Official</span>
            </div>
          )}
          {packageItem.trust_tier === "verified" && (
            <div
              className="px-1.5 py-0.5 rounded text-[9px] font-semibold uppercase bg-blue-500/25 text-blue-300 border border-blue-500/40 flex items-center gap-1 shadow-sm backdrop-blur-xs"
              title="Verified Author"
            >
              <ShieldCheck className="w-2.5 h-2.5 text-blue-400" />
              <span>Verified</span>
            </div>
          )}
          {mediaType === "video" && (
            <span className="px-1.5 py-0.5 rounded-full text-[9px] font-medium uppercase backdrop-blur-md bg-black/60 text-white/95 border border-white/10 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
              Video
            </span>
          )}
        </div>

        {/* Top-Right: Installation / Active State Badge */}
        <div className="absolute top-2 right-2 z-10">
          {isActive ? (
            <div className="px-2 py-0.5 rounded-full text-[9px] font-mono font-bold uppercase bg-emerald-500 text-slate-950 shadow-sm flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-slate-950 animate-pulse" />
              <span>Active</span>
            </div>
          ) : isUpdateAvailable ? (
            <div className="px-2 py-0.5 rounded-full text-[9px] font-mono uppercase bg-amber-500/90 text-slate-950 font-bold shadow-sm">
              Update
            </div>
          ) : isInstalled ? (
            <div className="px-2 py-0.5 rounded-full text-[9px] font-mono uppercase bg-[var(--bg-surface)]/90 text-emerald-400 border border-emerald-500/30 backdrop-blur-xs font-semibold">
              Installed
            </div>
          ) : packageItem.is_cached ? (
            <div className="px-2 py-0.5 rounded-full text-[9px] font-mono uppercase bg-[var(--bg-surface)]/90 text-[var(--text-muted)] border border-[var(--border-subtle)] backdrop-blur-xs">
              Cached
            </div>
          ) : null}
        </div>
      </div>

      {/* Card Content - Compact & Clean */}
      <div className="p-2.5 sm:p-3 flex-1 flex flex-col justify-between space-y-2">
        <div className="space-y-0.5">
          <div className="flex items-center justify-between gap-1.5">
            <h3 className="font-bold text-xs text-[var(--text-primary)] group-hover:text-white truncate">
              {packageItem.title}
            </h3>
            <span className="text-[9px] font-mono uppercase text-[var(--text-faint)] flex-shrink-0">
              {packageItem.category}
            </span>
          </div>

          <p className="text-[11px] text-[var(--text-muted)] line-clamp-1">
            {packageItem.subtitle || packageItem.description}
          </p>

          <div className="text-[10px] font-mono text-[var(--text-faint)] truncate">
            {packageItem.author?.name || "Community"}
            {componentNames ? " · " + componentNames : ""}
          </div>
        </div>

        {/* Quiet Compatibility & Rating Row */}
        <div className="pt-1.5 border-t border-[var(--border-subtle)] flex items-center justify-between text-[11px]">
          <div>
            {isActive ? (
              <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[10.5px]">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Active Theme
              </span>
            ) : isInstalled && isWorkingThroughTarget ? (
              <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[10.5px]">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Installed
              </span>
            ) : compat.level === "Compatible" ? (
              <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[10.5px]">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Compatible
              </span>
            ) : compat.level === "MissingDependencies" && !isWorkingThroughTarget ? (
              <span className="inline-flex items-center gap-1 text-amber-400 font-medium text-[10.5px] truncate max-w-[120px]" title={compat.summary_label}>
                <span className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0" />
                {compat.summary_label}
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[10.5px]">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Ready
              </span>
            )}
          </div>

          <div className="flex items-center gap-2 text-[var(--text-faint)] text-[10.5px]">
            <div className="flex items-center gap-0.5">
              <Star className="w-3 h-3 fill-amber-400 text-amber-400" />
              <span className="text-[var(--text-muted)] font-medium">
                {packageItem.rating ? packageItem.rating.toFixed(1) : "4.9"}
              </span>
            </div>

            <div className="flex items-center gap-0.5">
              <span>{packageItem.downloads ? (packageItem.downloads / 1000).toFixed(0) : "14"}k</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
