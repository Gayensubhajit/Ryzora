import React from "react";
import { Sparkles, Star, Download, CheckCircle2, ArrowRight } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface HeroBannerProps {
  featuredPackage: PackageItem;
}

export const HeroBanner: React.FC<HeroBannerProps> = ({ featuredPackage }) => {
  const { setSelectedPackage, installedPackageIds, systemInfo } = useApp();

  const isInstalled = installedPackageIds.includes(featuredPackage.id);

  return (
    <div className="relative rounded-3xl overflow-hidden border border-cyan-500/30 bg-slate-950 shadow-2xl shadow-cyan-950/40 select-none group">
      {/* Background Graphic with gradient overlay */}
      <div className="absolute inset-0 z-0">
        <img
          src={featuredPackage.hero_image}
          alt={featuredPackage.title}
          className="w-full h-full object-cover object-center brightness-60 group-hover:scale-102 transition-transform duration-700 ease-out"
        />
        <div className="absolute inset-0 bg-gradient-to-r from-[#07090e] via-[#07090e]/85 to-transparent" />
        <div className="absolute inset-0 bg-gradient-to-t from-[#07090e] via-transparent to-transparent" />
      </div>

      {/* Hero Content */}
      <div className="relative z-10 p-8 md:p-10 max-w-2xl flex flex-col justify-between min-h-[360px]">
        <div>
          {/* Top Pill Badges */}
          <div className="flex flex-wrap items-center gap-2 mb-4">
            <span className="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-bold uppercase tracking-wider bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 shadow-sm shadow-cyan-500/20 backdrop-blur-md">
              <Sparkles className="w-3.5 h-3.5 text-cyan-400" />
              <span>Featured Rice of the Week</span>
            </span>

            <span className="flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium bg-slate-900/80 text-emerald-400 border border-emerald-500/30 backdrop-blur-md">
              <CheckCircle2 className="w-3.5 h-3.5" />
              <span>Compatible with {systemInfo?.window_manager || "Hyprland"}</span>
            </span>
          </div>

          <h1 className="text-3xl md:text-4xl font-extrabold text-white tracking-tight leading-tight mb-2 drop-shadow-md">
            {featuredPackage.title}
          </h1>

          <p className="text-sm md:text-base text-slate-300 leading-relaxed mb-6 line-clamp-2 max-w-xl drop-shadow">
            {featuredPackage.subtitle}. Fully modular configurations for Hyprland, Waybar, Kitty, Rofi, and Fastfetch.
          </p>

          {/* Color Swatch Previews */}
          <div className="flex items-center gap-2 mb-6">
            <span className="text-xs text-slate-400 font-medium">Palette:</span>
            <div className="flex items-center gap-1.5 p-1.5 rounded-xl bg-slate-900/60 backdrop-blur-md border border-white/10">
              {featuredPackage.color_palette.map((color, idx) => (
                <div
                  key={idx}
                  className="w-4 h-4 rounded-full ring-1 ring-black/50 shadow-sm"
                  style={{ backgroundColor: color }}
                  title={color}
                />
              ))}
            </div>
          </div>
        </div>

        {/* Buttons & Metadata */}
        <div className="flex flex-wrap items-center gap-4">
          <button
            onClick={() => setSelectedPackage(featuredPackage)}
            className="flex items-center gap-2 px-6 py-3 rounded-xl font-bold text-sm bg-gradient-to-r from-cyan-500 to-indigo-600 hover:from-cyan-400 hover:to-indigo-500 text-white shadow-lg shadow-cyan-500/25 hover:shadow-cyan-500/40 transition-all transform hover:-translate-y-0.5 active:translate-y-0"
          >
            <span>Inspect & Install Rice</span>
            <ArrowRight className="w-4 h-4" />
          </button>

          {isInstalled && (
            <span className="flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-semibold bg-emerald-950/70 text-emerald-300 border border-emerald-500/40 backdrop-blur-md">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <span>Installed on your system</span>
            </span>
          )}

          <div className="flex items-center gap-4 text-xs text-slate-300 pl-2">
            <div className="flex items-center gap-1 text-amber-300">
              <Star className="w-4 h-4 fill-amber-300" />
              <span className="font-bold text-white text-sm">{featuredPackage.rating.toFixed(1)}</span>
              <span className="text-slate-400">({featuredPackage.rating_count})</span>
            </div>

            <div className="flex items-center gap-1 text-slate-400">
              <Download className="w-4 h-4" />
              <span className="font-medium text-slate-300">{(featuredPackage.downloads / 1000).toFixed(1)}k installs</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
