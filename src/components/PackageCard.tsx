import React from "react";
import { Star, Download, ShieldCheck, CheckCircle2, AlertCircle, ArrowUpRight } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface PackageCardProps {
  packageItem: PackageItem;
}

export const PackageCard: React.FC<PackageCardProps> = ({ packageItem }) => {
  const { setSelectedPackage, systemInfo, installedPackageIds } = useApp();

  const isInstalled = installedPackageIds.includes(packageItem.id);

  // Check compatibility with user's detected Linux environment
  const currentWm = systemInfo?.window_manager.toLowerCase() || "hyprland";
  const currentDe = systemInfo?.desktop_environment.toLowerCase() || "hyprland";

  const isCompatible =
    packageItem.supported_desktops.includes("universal") ||
    packageItem.supported_desktops.some(
      (d) => d === currentWm || d === currentDe || currentWm.includes(d) || currentDe.includes(d)
    );

  return (
    <div
      onClick={() => setSelectedPackage(packageItem)}
      className="group relative flex flex-col rounded-2xl bg-gradient-to-b from-slate-900/90 to-slate-950/90 border border-slate-800/80 hover:border-cyan-500/40 hover:shadow-xl hover:shadow-cyan-500/10 transition-all duration-300 cursor-pointer overflow-hidden select-none"
    >
      {/* Thumbnail Image Header */}
      <div className="relative aspect-video w-full overflow-hidden bg-slate-950">
        <img
          src={packageItem.hero_image}
          alt={packageItem.title}
          className="w-full h-full object-cover object-center group-hover:scale-105 transition-transform duration-500 ease-out brightness-95 group-hover:brightness-105"
          loading="lazy"
        />
        <div className="absolute inset-0 bg-gradient-to-t from-slate-950 via-slate-950/20 to-transparent" />

        {/* Top Badges */}
        <div className="absolute top-2.5 left-2.5 right-2.5 flex items-center justify-between">
          <span className="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-slate-950/80 text-cyan-300 border border-cyan-500/30 backdrop-blur-md">
            {packageItem.category}
          </span>

          <div className="flex items-center gap-1.5">
            {isInstalled && (
              <span className="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-950/80 text-emerald-300 border border-emerald-500/40 backdrop-blur-md">
                <CheckCircle2 className="w-3 h-3" />
                <span>Installed</span>
              </span>
            )}
            <span className="flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-slate-950/80 text-slate-300 border border-slate-700/60 backdrop-blur-md">
              <ShieldCheck className="w-3 h-3 text-cyan-400" />
              <span>Safe</span>
            </span>
          </div>
        </div>

        {/* Color Palette Swatches */}
        <div className="absolute bottom-2.5 right-2.5 flex items-center gap-1 bg-slate-950/70 p-1 rounded-lg backdrop-blur-md border border-white/10">
          {packageItem.color_palette.slice(0, 4).map((color, idx) => (
            <span
              key={idx}
              className="w-2.5 h-2.5 rounded-full ring-1 ring-black/40"
              style={{ backgroundColor: color }}
            />
          ))}
        </div>
      </div>

      {/* Content Section */}
      <div className="p-4 flex-1 flex flex-col justify-between">
        <div>
          <div className="flex items-start justify-between gap-2 mb-1">
            <h3 className="font-bold text-base text-slate-100 group-hover:text-cyan-300 transition-colors line-clamp-1">
              {packageItem.title}
            </h3>
            <ArrowUpRight className="w-4 h-4 text-slate-400 group-hover:text-cyan-400 group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-all flex-shrink-0" />
          </div>

          <p className="text-xs text-slate-400 line-clamp-2 leading-relaxed mb-3">
            {packageItem.subtitle}
          </p>

          {/* Compatibility Pill */}
          <div className="mb-3">
            {isCompatible ? (
              <div className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                <CheckCircle2 className="w-3 h-3" />
                <span>Compatible with your {systemInfo?.window_manager || "desktop"}</span>
              </div>
            ) : (
              <div className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
                <AlertCircle className="w-3 h-3" />
                <span>Targeted for: {packageItem.supported_desktops.join(", ")}</span>
              </div>
            )}
          </div>
        </div>

        {/* Footer info: Author, Rating, Downloads */}
        <div className="pt-3 border-t border-slate-800/60 flex items-center justify-between text-xs text-slate-400">
          <div className="flex items-center gap-2">
            <img
              src={packageItem.author.avatar}
              alt={packageItem.author.name}
              className="w-5 h-5 rounded-full bg-slate-800 ring-1 ring-slate-700"
            />
            <span className="text-slate-300 font-medium truncate max-w-[90px]">
              {packageItem.author.name}
            </span>
          </div>

          <div className="flex items-center gap-3">
            <div className="flex items-center gap-1 text-amber-300">
              <Star className="w-3.5 h-3.5 fill-amber-300" />
              <span className="font-semibold text-slate-200">{packageItem.rating.toFixed(1)}</span>
            </div>

            <div className="flex items-center gap-1 text-slate-400">
              <Download className="w-3.5 h-3.5" />
              <span>{(packageItem.downloads / 1000).toFixed(1)}k</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
