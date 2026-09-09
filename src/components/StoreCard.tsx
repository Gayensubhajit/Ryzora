import React from "react";
import { Sparkles, ShieldCheck, Download } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface StoreCardProps {
  packageItem: PackageItem;
  /** When true shows a taller portrait aspect ratio (e.g. lockscreens) */
  portrait?: boolean;
}

export const StoreCard: React.FC<StoreCardProps> = ({ packageItem, portrait = false }) => {
  const { setSelectedPackage, installedPackages } = useApp();

  const installedRecord = installedPackages.find((p) => p.package_id === packageItem.id);
  const isInstalled = !!installedRecord;
  const isUpdateAvailable =
    isInstalled && installedRecord
      ? parseFloat(packageItem.version) > parseFloat(installedRecord.version)
      : false;

  const handleClick = () => setSelectedPackage(packageItem);

  const aspectClass = portrait ? "aspect-[3/4]" : "aspect-[16/10]";

  return (
    <div
      onClick={handleClick}
      className="store-card group relative overflow-hidden rounded-lg cursor-pointer select-none"
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === "Enter" && handleClick()}
      aria-label={`${packageItem.title} – ${packageItem.category}`}
    >
      {/* Art fill */}
      <div className={`${aspectClass} w-full relative overflow-hidden bg-[var(--bg-canvas)]`}>
        <img
          src={packageItem.hero_image}
          alt={packageItem.title}
          className="w-full h-full object-cover object-center transition-transform duration-500 ease-out group-hover:scale-105"
          loading="lazy"
          decoding="async"
        />

        {/* Permanent dark gradient at bottom for legibility */}
        <div className="absolute inset-0 bg-gradient-to-t from-black/70 via-black/10 to-transparent pointer-events-none" />

        {/* ── Static badges (top-left) ── */}
        <div className="absolute top-2 left-2 flex items-center gap-1 flex-wrap">
          {packageItem.trust_tier === "official" && (
            <span
              className="px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase bg-amber-500/25 text-amber-300 border border-amber-500/40 flex items-center gap-1 shadow-sm backdrop-blur-sm"
              title="Official Ryzora Package"
            >
              <Sparkles className="w-2.5 h-2.5" />
              Official
            </span>
          )}
          {packageItem.trust_tier === "verified" && (
            <span
              className="px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase bg-blue-500/25 text-blue-300 border border-blue-500/40 flex items-center gap-1 shadow-sm backdrop-blur-sm"
              title="Verified Author"
            >
              <ShieldCheck className="w-2.5 h-2.5" />
              Verified
            </span>
          )}
        </div>

        {/* ── Install state badge (top-right) ── */}
        {isUpdateAvailable ? (
          <span className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-amber-500/30 text-amber-300 border border-amber-500/40 backdrop-blur-sm">
            Update
          </span>
        ) : isInstalled ? (
          <span className="absolute top-2 right-2 px-1.5 py-0.5 rounded text-[10px] font-mono uppercase bg-emerald-500/30 text-emerald-300 border border-emerald-500/40 backdrop-blur-sm">
            Installed
          </span>
        ) : null}

        {/* ── Static name strip (always visible at bottom) ── */}
        <div className="absolute bottom-0 inset-x-0 p-2.5 pointer-events-none">
          <div className="text-white text-xs font-semibold leading-tight truncate drop-shadow-sm">
            {packageItem.title}
          </div>
          <div className="text-white/50 text-[10px] font-mono uppercase tracking-wide mt-0.5">
            {packageItem.category}
          </div>
        </div>

        {/* ── Hover overlay: slides up, reveals author + quick-install ── */}
        <div className="absolute inset-0 bg-black/60 backdrop-blur-[2px] opacity-0 group-hover:opacity-100 transition-opacity duration-200 flex flex-col justify-end p-3 pointer-events-none group-hover:pointer-events-auto">
          <div className="text-white font-semibold text-xs truncate">{packageItem.title}</div>
          <div className="text-white/60 text-[11px] truncate mt-0.5">
            by {packageItem.author.name}
            {packageItem.repository_id?.startsWith("provider:") &&
              ` · ${packageItem.repository_id.replace("provider:", "")}`}
          </div>

          {/* Colour palette dots */}
          {packageItem.color_palette && packageItem.color_palette.length > 0 && (
            <div className="flex items-center gap-1 mt-2">
              {packageItem.color_palette.slice(0, 5).map((color, i) => (
                <span
                  key={i}
                  className="w-3 h-3 rounded-full border border-white/20 flex-shrink-0 shadow-sm"
                  style={{ backgroundColor: color }}
                  title={color}
                />
              ))}
            </div>
          )}

          <div className="flex items-center justify-between mt-2.5">
            <span className="text-white/50 text-[10px] flex items-center gap-1">
              <Download className="w-3 h-3" />
              {(packageItem.downloads / 1000).toFixed(0)}k
            </span>
            <span className="text-[10px] text-white/90 font-medium px-2.5 py-1 rounded bg-[var(--accent)] hover:bg-[var(--accent-hover)] transition-colors cursor-pointer pointer-events-auto" onClick={handleClick}>
              View Details
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
