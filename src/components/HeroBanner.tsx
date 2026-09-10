import React from "react";
import { Star, Download, Sparkles } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface HeroBannerProps {
  featuredPackage: PackageItem;
}

export const HeroBanner: React.FC<HeroBannerProps> = ({ featuredPackage }) => {
  const { setSelectedPackage } = useApp();

  const componentNames = featuredPackage.components.map((c) => c.name.split(" ")[0]).join(" · ");

  return (
    <div
      className="relative w-full h-56 sm:h-64 md:h-72 rounded-2xl overflow-hidden cursor-pointer group border border-[var(--rz-border)] shadow-xl"
      onClick={() => setSelectedPackage(featuredPackage)}
    >
      {/* Full-bleed artwork */}
      <img
        src={featuredPackage.hero_image}
        alt={featuredPackage.title}
        className="absolute inset-0 w-full h-full object-cover object-center transition-transform duration-700 ease-out group-hover:scale-105"
      />

      {/* Layered gradients: left fade + bottom fade */}
      <div className="absolute inset-0 bg-gradient-to-r from-black/90 via-black/40 to-transparent" />
      <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />

      {/* FEATURED badge */}
      <div className="absolute top-4 left-5">
        <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-[var(--accent)]/90 text-white text-[11px] font-semibold uppercase tracking-widest shadow-lg">
          <Sparkles className="w-3 h-3" />
          Featured
        </span>
      </div>

      {/* Content overlay — sits on top of gradient */}
      <div className="absolute inset-0 flex flex-col justify-end p-5 md:p-6 pointer-events-none">
        <div className="max-w-md">
          {/* Category chip */}
          <div className="text-[10px] font-mono uppercase tracking-widest text-white/50 mb-1.5">
            {featuredPackage.category}
          </div>

          {/* Title */}
          <h2 className="text-xl md:text-2xl font-bold text-white leading-tight mb-1 drop-shadow-md">
            {featuredPackage.title}
          </h2>

          {/* Subtitle */}
          <p className="text-xs text-white/70 leading-relaxed line-clamp-2 mb-3">
            {featuredPackage.subtitle}
          </p>

          {/* Colour palette */}
          {featuredPackage.color_palette && featuredPackage.color_palette.length > 0 && (
            <div className="flex items-center gap-1.5 mb-4">
              {featuredPackage.color_palette.slice(0, 5).map((color, i) => (
                <span
                  key={i}
                  className="w-3.5 h-3.5 rounded-full border border-white/20 shadow-sm flex-shrink-0"
                  style={{ backgroundColor: color }}
                  title={color}
                />
              ))}
            </div>
          )}

          {/* Stats + CTA */}
          <div className="flex items-center justify-between pointer-events-auto">
            <div className="flex items-center gap-4 text-xs text-white/60">
              <span className="flex items-center gap-1">
                <Star className="w-3.5 h-3.5 fill-amber-400 text-amber-400" />
                <span className="font-semibold text-white">{featuredPackage.rating.toFixed(1)}</span>
                <span className="text-[10px]">({featuredPackage.rating_count})</span>
              </span>
              <span className="flex items-center gap-1">
                <Download className="w-3.5 h-3.5 text-white/50" />
                {(featuredPackage.downloads / 1000).toFixed(1)}k
              </span>
              {componentNames && (
                <span className="hidden md:block font-mono text-[10px] text-white/40 truncate max-w-[180px]">
                  {componentNames}
                </span>
              )}
            </div>

            <button
              className="px-4 py-1.5 rounded-lg text-xs font-semibold bg-white text-black hover:bg-white/90 transition-colors shadow-md"
              onClick={(e) => { e.stopPropagation(); setSelectedPackage(featuredPackage); }}
            >
              View Details
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
