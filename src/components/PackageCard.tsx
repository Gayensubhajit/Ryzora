import React from "react";
import { Star } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface PackageCardProps {
  packageItem: PackageItem;
}

export const PackageCard: React.FC<PackageCardProps> = ({ packageItem }) => {
  const { setSelectedPackage, systemInfo, installedPackageIds } = useApp();

  const isInstalled = installedPackageIds.includes(packageItem.id);

  const currentWm = systemInfo?.window_manager.toLowerCase() || "hyprland";
  const currentDe = systemInfo?.desktop_environment.toLowerCase() || "hyprland";

  const isCompatible =
    packageItem.supported_desktops.includes("universal") ||
    packageItem.supported_desktops.some(
      (d) => d === currentWm || d === currentDe || currentWm.includes(d) || currentDe.includes(d)
    );

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
        {isInstalled && (
          <div className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--bg-surface)]/90 text-emerald-400 border border-emerald-500/30">
            Installed
          </div>
        )}
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
            {isCompatible ? (
              <span className="inline-flex items-center gap-1.5 text-emerald-400 font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                Compatible
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 text-[var(--text-faint)]">
                Requires {packageItem.supported_desktops[0]}
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
