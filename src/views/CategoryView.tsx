import React from "react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";
import { CategoryId } from "../types";

const CATEGORY_META: Record<CategoryId, { title: string; subtitle: string }> = {
  discover: { title: "Discover", subtitle: "Explore featured rices and customization packages" },
  rices: {
    title: "Complete Desktop Rices",
    subtitle: "Turnkey visual overhauls bundling window manager, bar, terminal, and launcher configs",
  },
  themes: {
    title: "GTK & Qt Themes",
    subtitle: "Cohesive color schemes and icon packs for native Linux desktop applications",
  },
  bars: {
    title: "Status Bars",
    subtitle: "Modular Waybar, Polybar, and AGS widgets with custom styles and sensor monitors",
  },
  fastfetch: {
    title: "Fastfetch & System Fetch",
    subtitle: "High contrast ASCII art logos, hardware spec layouts, and terminal banners",
  },
  lockscreens: {
    title: "Lockscreens",
    subtitle: "Sleek security lockscreens with live clocks, weather, and media controls",
  },
  wallpapers: {
    title: "Wallpapers & Palettes",
    subtitle: "Curated 4K/8K wallpapers with auto-generated Pywal and Matugen color swatches",
  },
  terminal: {
    title: "Terminal & Shell",
    subtitle: "Kitty, Alacritty color profiles and lightning-fast Starship shell prompts",
  },
  icons: { title: "Icons", subtitle: "Vector icon themes" },
  cursors: { title: "Cursors", subtitle: "Cursor themes" },
  fonts: { title: "Fonts", subtitle: "Typography and coding fonts" },
  widgets: { title: "Widgets", subtitle: "Desktop screen widgets" },
  bundles: { title: "Bundles", subtitle: "Packaged collections" },
  installed: { title: "Installed", subtitle: "Manage installed configurations" },
  backups: { title: "Backups", subtitle: "System snapshot history" },
  system: { title: "System Probe", subtitle: "Environment diagnostics" },
};

export const CategoryView: React.FC = () => {
  const { packages, activeCategory, searchQuery, desktopFilter } = useApp();

  const meta = CATEGORY_META[activeCategory] || {
    title: "Customizations",
    subtitle: "Browse packages",
  };

  const categoryPackages = packages.filter((pkg) => {
    const matchesCategory = pkg.category === activeCategory;

    const matchesSearch =
      !searchQuery ||
      pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase()));

    const matchesDesktop =
      desktopFilter === "all" ||
      pkg.supported_desktops.includes("universal") ||
      pkg.supported_desktops.includes(desktopFilter);

    return matchesCategory && matchesSearch && matchesDesktop;
  });

  return (
    <div className="space-y-6 pb-12">
      {/* Category Header */}
      <div className="p-8 rounded-3xl bg-gradient-to-r from-slate-900/90 via-slate-900/60 to-transparent border border-slate-800">
        <div className="flex items-center gap-2 text-cyan-400 text-xs font-bold uppercase tracking-wider mb-2">
          <span>Marketplace Category</span>
        </div>
        <h1 className="text-3xl font-extrabold text-white tracking-tight mb-2">
          {meta.title}
        </h1>
        <p className="text-sm text-slate-400 max-w-2xl leading-relaxed">
          {meta.subtitle}
        </p>
      </div>

      {/* Grid of packages */}
      <div className="flex items-center justify-between">
        <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
          Showing {categoryPackages.length} package{categoryPackages.length === 1 ? "" : "s"}
        </span>
      </div>

      {categoryPackages.length === 0 ? (
        <div className="p-16 text-center rounded-3xl bg-slate-900/40 border border-slate-800 space-y-3">
          <div className="text-slate-400 text-sm">
            No packages currently found for this category with your active filters.
          </div>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
          {categoryPackages.map((pkg) => (
            <PackageCard key={pkg.id} packageItem={pkg} />
          ))}
        </div>
      )}
    </div>
  );
};
