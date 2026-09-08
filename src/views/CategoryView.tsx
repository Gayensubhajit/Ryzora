import React from "react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";
import { CategoryId } from "../types";

const CATEGORY_META: Record<CategoryId, { title: string; subtitle: string }> = {
  discover: { title: "Discover", subtitle: "Explore desktop configurations" },
  rices: {
    title: "Complete Rices",
    subtitle: "Full visual configurations bundling window manager, status bar, and terminal themes",
  },
  themes: {
    title: "Themes",
    subtitle: "GTK, Qt, and application color palettes",
  },
  bars: {
    title: "Status Bars",
    subtitle: "Waybar and status panel configurations",
  },
  fastfetch: {
    title: "Fastfetch",
    subtitle: "System fetch ASCII logos and layout profiles",
  },
  lockscreens: {
    title: "Lockscreens",
    subtitle: "Hyprlock and lockscreen configurations",
  },
  wallpapers: {
    title: "Wallpapers",
    subtitle: "High-resolution desktop wallpapers with embedded palettes",
  },
  terminal: {
    title: "Terminal",
    subtitle: "Kitty, Alacritty, and Starship shell prompt configs",
  },
  icons: { title: "Icons", subtitle: "Icon themes" },
  cursors: { title: "Cursors", subtitle: "Cursor sets" },
  fonts: { title: "Fonts", subtitle: "System and coding fonts" },
  widgets: { title: "Widgets", subtitle: "Desktop screen widgets" },
  bundles: { title: "Bundles", subtitle: "Package bundles" },
  installed: { title: "Installed", subtitle: "Installed configurations" },
  backups: { title: "Backups", subtitle: "Snapshot history" },
  system: { title: "System", subtitle: "System diagnostics" },
};

export const CategoryView: React.FC = () => {
  const { packages, activeCategory, searchQuery, desktopFilter } = useApp();

  const meta = CATEGORY_META[activeCategory] || {
    title: "Category",
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
    <div className="space-y-6 pb-10">
      {/* Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
        <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
          {meta.title}
        </h1>
        <p className="text-xs text-[var(--text-muted)]">
          {meta.subtitle}
        </p>
      </div>

      {/* Grid */}
      <div className="space-y-3">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
          <span>{categoryPackages.length} packages</span>
        </div>

        {categoryPackages.length === 0 ? (
          <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)]">
            No packages found in this category matching your filter.
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {categoryPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
