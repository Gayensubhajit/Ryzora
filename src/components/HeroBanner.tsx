import React from "react";
import { Star, Download } from "lucide-react";
import { PackageItem } from "../types";
import { useApp } from "../context/AppContext";

interface HeroBannerProps {
  featuredPackage: PackageItem;
}

export const HeroBanner: React.FC<HeroBannerProps> = ({ featuredPackage }) => {
  const { setSelectedPackage } = useApp();

  const componentNames = featuredPackage.components.map((c) => c.name.split(" ")[0]).join(" · ");

  return (
    <div className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-hidden">
      <div className="grid grid-cols-1 md:grid-cols-12 gap-0">
        {/* Left Info Column */}
        <div className="p-6 md:col-span-7 flex flex-col justify-between space-y-4">
          <div>
            <div className="flex items-center justify-between text-[11px] font-mono uppercase text-[var(--text-faint)] mb-2">
              <span>Featured</span>
              <span className="font-semibold text-[var(--text-muted)] tracking-wider">
                {featuredPackage.category.toUpperCase()}
              </span>
            </div>

            <h2 className="text-xl font-bold text-[var(--text-primary)] mb-1">
              {featuredPackage.title}
            </h2>

            <p className="text-xs text-[var(--text-muted)] leading-relaxed mb-3">
              {featuredPackage.subtitle}
            </p>

            {/* Component list */}
            <div className="text-[11px] font-mono text-[var(--text-faint)] truncate mb-4">
              {componentNames}
            </div>
          </div>

          {/* Actions and Stats */}
          <div className="pt-3 border-t border-[var(--border-subtle)] flex items-center justify-between">
            <div className="flex items-center gap-4 text-xs text-[var(--text-muted)]">
              <div className="flex items-center gap-1">
                <Star className="w-3.5 h-3.5 fill-amber-400 text-amber-400" />
                <span className="font-semibold text-[var(--text-primary)]">
                  {featuredPackage.rating.toFixed(1)}
                </span>
                <span className="text-[var(--text-faint)]">({featuredPackage.rating_count})</span>
              </div>

              <div className="flex items-center gap-1 text-[var(--text-faint)]">
                <Download className="w-3.5 h-3.5" />
                <span>{(featuredPackage.downloads / 1000).toFixed(1)}k installs</span>
              </div>
            </div>

            <button
              onClick={() => setSelectedPackage(featuredPackage)}
              className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
            >
              View Details
            </button>
          </div>
        </div>

        {/* Right Preview Image */}
        <div className="relative md:col-span-5 h-48 md:h-auto bg-[var(--bg-canvas)] border-t md:border-t-0 md:border-l border-[var(--border-subtle)] overflow-hidden">
          <img
            src={featuredPackage.hero_image}
            alt={featuredPackage.title}
            className="w-full h-full object-cover object-center"
          />
        </div>
      </div>
    </div>
  );
};
