import React, { useState, useMemo } from "react";
import { ArrowUpDown, SlidersHorizontal } from "lucide-react";
import { useApp } from "../context/AppContext";
import { StoreCard } from "../components/StoreCard";
import { PackageCard } from "../components/PackageCard";
import { CategoryId } from "../types";

const CATEGORY_META: Record<CategoryId, { title: string; subtitle: string }> = {
  hub: { title: "Ryzora Hub", subtitle: "Synchronized ecosystem, updates, and creators" },
  discover: { title: "Discover", subtitle: "Explore desktop configurations" },
  updates: { title: "Updates", subtitle: "Categorized updates with 1-click preview and rollback" },
  repositories: { title: "Repositories", subtitle: "Repository sync and release channel management" },
  creators: { title: "Creators", subtitle: "Creator profiles with Ed25519 keyring verification" },
  rices: { title: "Complete Rices", subtitle: "Full visual configurations bundling window manager, status bar, and terminal themes" },
  themes: { title: "Themes", subtitle: "GTK, Qt, and application color palettes" },
  bars: { title: "Status Bars", subtitle: "Waybar and status panel configurations" },
  fastfetch: { title: "Fastfetch", subtitle: "System fetch ASCII logos and layout profiles" },
  lockscreens: { title: "Lockscreens", subtitle: "Hyprlock, Quickshell, Swaylock and SDDM lockscreen configurations" },
  wallpapers: { title: "Wallpapers", subtitle: "High-resolution desktop wallpapers with embedded palettes" },
  terminal: { title: "Terminal", subtitle: "Kitty, Alacritty, and Starship shell prompt configs" },
  icons: { title: "Icons", subtitle: "Icon themes" },
  cursors: { title: "Cursors", subtitle: "Cursor sets" },
  fonts: { title: "Fonts", subtitle: "System and coding fonts" },
  widgets: { title: "Widgets", subtitle: "Desktop screen widgets" },
  bundles: { title: "Bundles", subtitle: "Package bundles" },
  installed: { title: "Installed", subtitle: "Installed configurations" },
  backups: { title: "Backups", subtitle: "Snapshot history" },
  system: { title: "System", subtitle: "System diagnostics" },
  author: { title: "Package Creator", subtitle: "Author and publish Ryzora packages" },
  settings: { title: "Settings", subtitle: "Preferences and configuration" },
  notifications: { title: "Activity", subtitle: "Persistent log of all Ryzora events" },
  collections: { title: "Collections", subtitle: "Saved package lists in .ryzlist format" },
  integrity: { title: "Integrity Health", subtitle: "Read-only verification of installed packages" },
};

/** Categories that use the new art-forward dense store grid */
const STORE_GRID_CATEGORIES = new Set<CategoryId>([
  "lockscreens",
  "rices",
  "themes",
  "wallpapers",
  "bars",
  "fastfetch",
  "terminal",
  "bundles",
]);

/** Categories where portrait aspect ratio cards look better */
const PORTRAIT_CATEGORIES = new Set<CategoryId>(["lockscreens", "wallpapers"]);

/** Per-category sub-filter pills */
const CATEGORY_SUB_FILTERS: Partial<Record<CategoryId, string[]>> = {
  lockscreens: ["All", "Hyprlock", "Quickshell", "Swaylock", "SDDM"],
  rices: ["All", "Hyprland", "Sway", "KDE", "GNOME"],
  themes: ["All", "GTK", "Qt", "Application"],
  wallpapers: ["All", "Abstract", "Nature", "Minimal", "Sci-Fi"],
  bars: ["All", "Waybar", "Eww", "Polybar"],
  terminal: ["All", "Kitty", "Alacritty", "Foot"],
};

function compareSemver(v1: string, v2: string): number {
  const clean = (v: string) => v.replace(/^v/, "").trim();
  const [base1, pre1] = clean(v1).split("-");
  const [base2, pre2] = clean(v2).split("-");

  const p1 = base1.split(".").map((n) => parseInt(n, 10) || 0);
  const p2 = base2.split(".").map((n) => parseInt(n, 10) || 0);

  for (let i = 0; i < Math.max(p1.length, p2.length); i++) {
    const num1 = p1[i] || 0;
    const num2 = p2[i] || 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }

  if (!pre1 && pre2) return 1;
  if (pre1 && !pre2) return -1;
  if (pre1 && pre2) return pre1.localeCompare(pre2);

  return 0;
}

export const CategoryView: React.FC = () => {
  const { packages, activeCategory, searchQuery, desktopFilter } = useApp();
  const [sortBy, setSortBy] = useState<"rating" | "downloads" | "name" | "newest">("downloads");
  const [subFilter, setSubFilter] = useState<string>("All");

  const meta = CATEGORY_META[activeCategory] || { title: "Category", subtitle: "Browse packages" };
  const useStoreGrid = STORE_GRID_CATEGORIES.has(activeCategory);
  const usePortrait = PORTRAIT_CATEGORIES.has(activeCategory);
  const subFilters = CATEGORY_SUB_FILTERS[activeCategory];

  // Reset sub-filter when category changes
  React.useEffect(() => {
    setSubFilter("All");
  }, [activeCategory]);

  const categoryPackages = useMemo(() => {
    let list = packages.filter((pkg) => {
      const matchesCategory =
        pkg.category === activeCategory ||
        (activeCategory === "fastfetch" && (pkg.package_type === "fastfetch" || pkg.category === "fastfetch")) ||
        (activeCategory === "wallpapers" && (pkg.package_type === "wallpaper" || pkg.category === "wallpapers")) ||
        (activeCategory === "rices" && (pkg.package_type === "rice" || pkg.category === "rices")) ||
        (activeCategory === "themes" && (pkg.package_type === "theme" || pkg.category === "themes")) ||
        (activeCategory === "icons" && (pkg.package_type === "icon" || pkg.category === "icons")) ||
        (activeCategory === "bars" && (pkg.package_type === "waybar" || pkg.category === "bars")) ||
        (activeCategory === "lockscreens" && (pkg.package_type === "lockscreen" || pkg.category === "lockscreens")) ||
        (activeCategory === "terminal" && (pkg.package_type === "terminal" || pkg.category === "terminal"));

      const matchesSearch =
        !searchQuery ||
        pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.author.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase()));

      const matchesDesktop =
        desktopFilter === "all" ||
        pkg.supported_desktops.includes("universal") ||
        pkg.supported_desktops.includes(desktopFilter as any);

      // Sub-filter (tag / desktop matching by label)
      const matchesSubFilter =
        subFilter === "All" ||
        pkg.tags.some((t) => t.toLowerCase() === subFilter.toLowerCase()) ||
        pkg.supported_desktops.some((d) => d.toLowerCase() === subFilter.toLowerCase()) ||
        (pkg.components ?? []).some((c) =>
          c.component_type?.toLowerCase().includes(subFilter.toLowerCase())
        );

      return matchesCategory && matchesSearch && matchesDesktop && matchesSubFilter;
    });

    list = [...list].sort((a, b) => {
      if (sortBy === "rating") return b.rating - a.rating;
      if (sortBy === "downloads") return b.downloads - a.downloads;
      if (sortBy === "name") return a.title.localeCompare(b.title);
      if (sortBy === "newest") return compareSemver(b.version, a.version);
      return 0;
    });

    return list;
  }, [packages, activeCategory, searchQuery, desktopFilter, sortBy, subFilter]);

  // ─── Store-grid layout (art-forward, dense) ────────────────────────────
  if (useStoreGrid) {
    return (
      <div className="space-y-0 pb-10">
        {/* Category header */}
        <div className="flex items-center justify-between py-4 px-0">
          <div>
            <h1 className="text-base font-bold text-[var(--text-primary)] tracking-tight">{meta.title}</h1>
            <p className="text-[11px] text-[var(--text-muted)] mt-0.5">{meta.subtitle}</p>
          </div>
          <div className="flex items-center gap-1.5 text-[11px] text-[var(--text-faint)]">
            <span className="font-mono">{categoryPackages.length}</span>
            <span>items</span>
          </div>
        </div>

        {/* Sub-filter pills */}
        {subFilters && subFilters.length > 0 && (
          <div className="flex items-center gap-1.5 pb-4 overflow-x-auto scrollbar-none" style={{ scrollbarWidth: "none" }}>
            {subFilters.map((f) => (
              <button
                key={f}
                onClick={() => setSubFilter(f)}
                className={[
                  "flex-shrink-0 px-3 py-1 rounded-full text-[11px] font-medium transition-all",
                  subFilter === f
                    ? "bg-[var(--accent)] text-white shadow-sm shadow-[var(--accent)]/30"
                    : "bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]",
                ].join(" ")}
              >
                {f}
              </button>
            ))}
          </div>
        )}

        {/* Sort control */}
        <div className="flex items-center justify-end gap-1.5 pb-3">
          <SlidersHorizontal className="w-3.5 h-3.5 text-[var(--text-faint)]" />
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
            className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] outline-none text-[11px] cursor-pointer"
          >
            <option value="downloads">Most Downloaded</option>
            <option value="rating">Highest Rated</option>
            <option value="name">Name (A-Z)</option>
            <option value="newest">Newest</option>
          </select>
        </div>

        {/* Dense store grid */}
        {categoryPackages.length === 0 ? (
          <div className="py-20 text-center text-xs text-[var(--text-muted)]">
            No packages match your filter.
          </div>
        ) : (
          <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
            {categoryPackages.map((pkg) => (
              <StoreCard key={pkg.id} packageItem={pkg} portrait={usePortrait} />
            ))}
          </div>
        )}
      </div>
    );
  }

  // ─── Legacy grid layout (for non-visual categories) ────────────────────
  return (
    <div className="space-y-6 pb-10">
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">{meta.title}</h1>
          <p className="text-xs text-[var(--text-muted)]">{meta.subtitle}</p>
        </div>
        <div className="flex items-center gap-1.5 self-start sm:self-center">
          <ArrowUpDown className="w-3.5 h-3.5 text-[var(--text-faint)]" />
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
            className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] outline-none text-[11px]"
          >
            <option value="downloads">Most Downloads</option>
            <option value="rating">Highest Rated</option>
            <option value="name">Name (A-Z)</option>
            <option value="newest">Newest Version</option>
          </select>
        </div>
      </div>

      <div className="space-y-3">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)] font-mono">
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
