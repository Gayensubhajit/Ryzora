import React from "react";
import { Star, ShieldCheck, Sparkles } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

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
  const { setSelectedPackage, installedPackages, checkCompatibility, repositories } = useApp();

  const installedRecord = installedPackages.find((p) => p.package_id === packageItem.id);
  const isInstalled = !!installedRecord;
  const isUpdateAvailable =
    isInstalled && installedRecord
      ? compareSemver(packageItem.version, installedRecord.version) > 0
      : false;

  const repo = repositories.find((r) => r.id === packageItem.repository_id);
  const isOffline = repo && repo.status === "offline" && !packageItem.is_cached;

  const compat = checkCompatibility(packageItem);

  const componentNames = packageItem.components
    .map((c) => c.name.split(" ")[0])
    .slice(0, 4)
    .join(" · ");

  return (
    <div
      onClick={() => setSelectedPackage(packageItem)}
      className="group flex flex-col rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-strong)] hover:bg-[var(--bg-surface-elevated)] transition-colors cursor-pointer overflow-hidden select-none"
    >
      {/* 16:10 Thumbnail Preview */}
      <div className="relative aspect-[16/10] w-full bg-[var(--bg-canvas)] border-b border-[var(--border-subtle)] overflow-hidden">
        <img
          src={packageItem.hero_image}
          alt={packageItem.title}
          className="w-full h-full object-cover object-center group-hover:opacity-95 transition-opacity"
          loading="lazy"
        />

        {/* Top-Left: Integrity / Cache / Offline indicator */}
        <div className="absolute top-2 left-2 flex items-center gap-1">
          {packageItem.trust_tier === "official" && (
            <div
              className="px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase bg-amber-500/25 text-amber-300 border border-amber-500/40 flex items-center gap-1 shadow-sm"
              title="Official Ryzora Package"
            >
              <Sparkles className="w-3 h-3 text-amber-400" />
              <span>Official</span>
            </div>
          )}
          {packageItem.release_channel === "beta" && (
            <div
              className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-amber-500/20 text-amber-400 border border-amber-500/30"
              title="Beta Release Channel"
            >
              Beta
            </div>
          )}
          {packageItem.release_channel === "nightly" && (
            <div
              className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-purple-500/20 text-purple-300 border border-purple-500/30"
              title="Nightly Release Channel"
            >
              Nightly
            </div>
          )}
          {packageItem.integrity_status === "verified" ? (
            <div
              className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-emerald-400 border border-emerald-500/30 flex items-center gap-1"
              title="Cryptographically verified SHA-256 tree hash"
            >
              <ShieldCheck className="w-3 h-3 text-emerald-400" />
              <span>Verified</span>
            </div>
          ) : packageItem.integrity_status === "corrupted" ? (
            <div
              className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-rose-400 border border-rose-500/30 flex items-center gap-1"
              title="Cache integrity verification failed"
            >
              <ShieldCheck className="w-3 h-3 text-rose-400" />
              <span>Corrupted</span>
            </div>
          ) : isOffline ? (
            <div className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-amber-400 border border-amber-500/30">
              Offline
            </div>
          ) : packageItem.is_cached ? (
            <div className="px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-[var(--text-muted)] border border-[var(--border-subtle)]">
              Cached
            </div>
          ) : null}
        </div>

        {/* Top-Right: Installation State */}
        {isUpdateAvailable ? (
          <div className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-amber-400 border border-amber-500/30">
            Update v{packageItem.version}
          </div>
        ) : isInstalled ? (
          <div className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-emerald-400 border border-emerald-500/30">
            Installed
          </div>
        ) : null}
      </div>

      {/* Card Content */}
      <div className="p-3.5 flex-1 flex flex-col justify-between space-y-3">
        <div className="space-y-1">
          <div className="flex items-center justify-between gap-2">
            <h3 className="font-semibold text-xs text-[var(--text-primary)] group-hover:text-white truncate">
              {packageItem.title}
            </h3>
            <span className="text-[10px] font-mono uppercase text-[var(--text-faint)] flex-shrink-0">
              {packageItem.category}
            </span>
          </div>

          <p className="text-[11px] text-[var(--text-muted)] line-clamp-1">
            {packageItem.subtitle}
          </p>

          <div className="text-[11px] font-mono text-[var(--text-faint)] truncate pt-0.5">
            {componentNames || "Standalone"}
          </div>
        </div>

        {/* Quiet Compatibility & Stats Row */}
        <div className="pt-2 border-t border-[var(--border-subtle)] flex items-center justify-between text-[11px]">
          <div>
            {compat.level === "Compatible" ? (
              <span className="inline-flex items-center gap-1.5 text-emerald-400 font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Compatible
              </span>
            ) : compat.level === "MissingDependencies" ? (
              <span className="inline-flex items-center gap-1.5 text-amber-400 font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
                {compat.summary_label}
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 text-[var(--text-faint)]">
                {compat.summary_label}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2.5 text-[var(--text-faint)]">
            <div className="flex items-center gap-1">
              <Star className="w-3 h-3 fill-amber-400 text-amber-400" />
              <span className="text-[var(--text-muted)] font-medium">
                {packageItem.rating.toFixed(1)}
              </span>
            </div>

            <div className="flex items-center gap-0.5">
              <span>{(packageItem.downloads / 1000).toFixed(0)}k</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
