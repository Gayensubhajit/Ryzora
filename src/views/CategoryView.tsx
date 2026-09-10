import React, { useState, useMemo } from "react";
import { SlidersHorizontal, ExternalLink } from "lucide-react";
import { useApp } from "../context/AppContext";
import { StoreCard } from "../components/StoreCard";
import { PackageCard } from "../components/PackageCard";
import { CategoryId } from "../types";

// ── Category metadata ────────────────────────────────────────────────────────
const CATEGORY_META: Record<CategoryId, { title: string; subtitle: string }> = {
  hub: { title: "Ryzora Hub", subtitle: "Synchronized ecosystem, updates, and creators" },
  discover: { title: "Discover", subtitle: "Explore desktop configurations" },
  updates: { title: "Updates", subtitle: "Categorized updates with 1-click preview and rollback" },
  repositories: { title: "Repositories", subtitle: "Repository sync and release channel management" },
  creators: { title: "Creators", subtitle: "Creator profiles with Ed25519 keyring verification" },
  rices: { title: "Complete Rices", subtitle: "Full visual configurations: window manager, status bar, terminal" },
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
  "compatibility-lab": { title: "Compatibility Lab", subtitle: "Host capabilities and runtime adapter intersection visualizer" },
  apps: { title: "Applications", subtitle: "Linux native and universal software packages" },
};

/** Categories that use the new art-forward dense store grid */
const STORE_GRID_CATEGORIES = new Set<CategoryId>([
  "lockscreens", "rices", "themes", "wallpapers", "bars",
  "fastfetch", "terminal", "bundles",
]);

/** Per-category sub-filter pills */
const CATEGORY_SUB_FILTERS: Partial<Record<CategoryId, string[]>> = {
  lockscreens: ["All", "Hyprlock", "Quickshell", "Swaylock", "SDDM"],
  rices: ["All", "Hyprland", "Sway", "KDE", "GNOME"],
  themes: ["All", "GTK", "Qt", "Application"],
  wallpapers: ["All", "Abstract", "Nature", "Minimal", "Sci-Fi"],
  bars: ["All", "Waybar", "Eww", "Polybar"],
  terminal: ["All", "Kitty", "Alacritty", "Foot"],
};

/** Provider links for low-content CTA */
const CATEGORY_PROVIDERS: Partial<Record<CategoryId, { label: string; url: string }[]>> = {
  lockscreens: [
    { label: "Qylock", url: "https://github.com/Darkkal44/qylock" },
    { label: "Community", url: "https://github.com/Gayensubhajit/Ryzora" },
  ],
  rices: [
    { label: "GitHub", url: "https://github.com/topics/dotfiles" },
    { label: "Community", url: "https://github.com/Gayensubhajit/Ryzora" },
  ],
  wallpapers: [
    { label: "Community", url: "https://github.com/Gayensubhajit/Ryzora" },
  ],
};

function compareSemver(v1: string, v2: string): number {
  const clean = (v: string) => v.replace(/^v/, "").trim();
  const [base1, pre1] = clean(v1).split("-");
  const [base2, pre2] = clean(v2).split("-");
  const p1 = base1.split(".").map((n) => parseInt(n, 10) || 0);
  const p2 = base2.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(p1.length, p2.length); i++) {
    const d = (p1[i] || 0) - (p2[i] || 0);
    if (d !== 0) return d > 0 ? 1 : -1;
  }
  if (!pre1 && pre2) return 1;
  if (pre1 && !pre2) return -1;
  if (pre1 && pre2) return pre1.localeCompare(pre2);
  return 0;
}

// ── Low-content state ────────────────────────────────────────────────────────
const LowContentState: React.FC<{
  category: CategoryId;
  count: number;
  children: React.ReactNode;
}> = ({ category, count, children }) => {
  const providers = CATEGORY_PROVIDERS[category];
  const categoryMeta = CATEGORY_META[category];

  if (count === 0) {
    return (
      <div className="py-16 text-center space-y-3">
        <div className="text-3xl select-none opacity-30">◻</div>
        <div className="text-sm font-semibold text-[var(--text-muted)]">
          No {categoryMeta?.title ?? "packages"} yet
        </div>
        <p className="text-xs text-[var(--text-faint)] max-w-xs mx-auto leading-relaxed">
          This category is populated from connected repositories and content providers.
        </p>
        {providers && (
          <div className="pt-3 flex items-center justify-center gap-3">
            <span className="text-[11px] text-[var(--text-faint)]">Explore sources</span>
            {providers.map((p) => (
              <a
                key={p.label}
                href={p.url}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1 text-[11px] text-[var(--accent-text)] hover:underline"
                onClick={(e) => e.stopPropagation()}
              >
                {p.label}
                <ExternalLink className="w-2.5 h-2.5" />
              </a>
            ))}
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Cards */}
      {children}

      {/* "Looking for more?" strip — always shown for store categories */}
      {count < 6 && (
        <div className="mt-8 pt-6 border-t border-[var(--border-subtle)]">
          <div className="flex flex-col sm:flex-row sm:items-center gap-3 justify-between">
            <div>
              <div className="text-xs font-semibold text-[var(--text-muted)]">Looking for more?</div>
              <div className="text-[11px] text-[var(--text-faint)] mt-0.5">
                The catalog grows as providers publish new content.
              </div>
            </div>
            {providers && (
              <div className="flex items-center gap-3">
                {providers.map((p) => (
                  <a
                    key={p.label}
                    href={p.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-1 text-xs text-[var(--accent-text)] hover:underline"
                    onClick={(e) => e.stopPropagation()}
                  >
                    {p.label}
                    <ExternalLink className="w-3 h-3" />
                  </a>
                ))}
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

// ── Main view ────────────────────────────────────────────────────────────────
export const CategoryView: React.FC = () => {
  const { packages, activeCategory, searchQuery, desktopFilter } = useApp();
  const [sortBy, setSortBy] = useState<"rating" | "downloads" | "name" | "newest">("downloads");
  const [subFilter, setSubFilter] = useState<string>("All");

  const meta = CATEGORY_META[activeCategory] ?? { title: "Category", subtitle: "Browse packages" };
  const useStoreGrid = STORE_GRID_CATEGORIES.has(activeCategory);
  const subFilters = CATEGORY_SUB_FILTERS[activeCategory];

  React.useEffect(() => { setSubFilter("All"); }, [activeCategory]);

  const categoryPackages = useMemo(() => {
    let list = packages.filter((pkg) => {
      const matchesCategory =
        pkg.category === activeCategory ||
        (activeCategory === "fastfetch"    && (pkg.package_type === "fastfetch"  || pkg.category === "fastfetch")) ||
        (activeCategory === "wallpapers"   && (pkg.package_type === "wallpaper"  || pkg.category === "wallpapers")) ||
        (activeCategory === "rices"        && (pkg.package_type === "rice"       || pkg.category === "rices")) ||
        (activeCategory === "themes"       && (pkg.package_type === "theme"      || pkg.category === "themes")) ||
        (activeCategory === "bars"         && (pkg.package_type === "waybar"     || pkg.category === "bars")) ||
        (activeCategory === "lockscreens"  && (pkg.package_type === "lockscreen" || pkg.category === "lockscreens")) ||
        (activeCategory === "terminal"     && (pkg.package_type === "terminal"   || pkg.category === "terminal"));

      const matchesSearch =
        !searchQuery ||
        pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.author.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase()));

      const matchesDesktop =
        desktopFilter === "all" ||
        pkg.supported_desktops.includes("universal") ||
        pkg.supported_desktops.includes(desktopFilter as any);

      const matchesSubFilter =
        subFilter === "All" ||
        pkg.tags.some((t) => t.toLowerCase() === subFilter.toLowerCase()) ||
        pkg.supported_desktops.some((d) => d.toLowerCase() === subFilter.toLowerCase()) ||
        (pkg.components ?? []).some((c) => c.component_type?.toLowerCase().includes(subFilter.toLowerCase())) ||
        (subFilter.toLowerCase() === "sddm" && (pkg.supports_login_screen || pkg.lockscreen?.targets?.sddm != null || Boolean(pkg.manifest?.targets?.["sddm"]))) ||
        (subFilter.toLowerCase() === "quickshell" && (pkg.supports_session_lock || pkg.lockscreen?.targets?.quickshell != null || Boolean(pkg.manifest?.targets?.["quickshell"])));

      return matchesCategory && matchesSearch && matchesDesktop && matchesSubFilter;
    });

    return [...list].sort((a, b) => {
      if (sortBy === "rating")    return b.rating - a.rating;
      if (sortBy === "downloads") return b.downloads - a.downloads;
      if (sortBy === "name")      return a.title.localeCompare(b.title);
      if (sortBy === "newest")    return compareSemver(b.version, a.version);
      return 0;
    });
  }, [packages, activeCategory, searchQuery, desktopFilter, sortBy, subFilter]);

  // ── Store grid (art-forward, dense) ───────────────────────────────────────
  if (useStoreGrid) {
    return (
      <div className="pb-12">
        {/* Category heading */}
        <div className="flex items-baseline justify-between mb-4">
          <div>
            <h1 className="text-lg font-bold tracking-tight text-[var(--text-primary)] uppercase">
              {meta.title}
            </h1>
            <p className="text-[11px] text-[var(--text-faint)] mt-0.5">{meta.subtitle}</p>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="text-[11px] font-mono text-[var(--text-faint)]">
              {categoryPackages.length} {categoryPackages.length === 1 ? "item" : "items"}
            </span>
            <div className="flex items-center gap-1 ml-3">
              <SlidersHorizontal className="w-3 h-3 text-[var(--text-faint)]" />
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="ryz-select text-[11px]"
              >
                <option value="downloads">Popular</option>
                <option value="rating">Top Rated</option>
                <option value="name">A–Z</option>
                <option value="newest">Newest</option>
              </select>
            </div>
          </div>
        </div>

        {/* Sub-filter pills */}
        {subFilters && (
          <div className="flex items-center gap-1.5 mb-5 overflow-x-auto scrollbar-none" style={{ scrollbarWidth: "none" }}>
            {subFilters.map((f) => (
              <button
                key={f}
                onClick={() => setSubFilter(f)}
                className={[
                  "flex-shrink-0 px-3 py-1 rounded-full text-[11px] font-medium transition-all",
                  subFilter === f
                    ? "bg-[var(--accent)] text-white"
                    : "bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]",
                ].join(" ")}
              >
                {f}
              </button>
            ))}
          </div>
        )}

        {/* Low-content aware grid */}
        <LowContentState category={activeCategory} count={categoryPackages.length}>
          <div className="store-grid-fluid">
            {categoryPackages.map((pkg) => (
              <StoreCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        </LowContentState>
      </div>
    );
  }

  // ── Legacy grid (non-visual categories) ──────────────────────────────────
  return (
    <div className="space-y-6 pb-10">
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">{meta.title}</h1>
          <p className="text-xs text-[var(--text-muted)]">{meta.subtitle}</p>
        </div>
        <select
          value={sortBy}
          onChange={(e) => setSortBy(e.target.value as any)}
          className="ryz-select text-[11px] self-start sm:self-center"
        >
          <option value="downloads">Most Downloads</option>
          <option value="rating">Highest Rated</option>
          <option value="name">Name (A-Z)</option>
          <option value="newest">Newest Version</option>
        </select>
      </div>
      <div className="text-xs text-[var(--text-faint)] font-mono">{categoryPackages.length} packages</div>
      {categoryPackages.length === 0 ? (
        <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)]">
          No packages found matching your filter.
        </div>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-4">
          {categoryPackages.map((pkg) => <PackageCard key={pkg.id} packageItem={pkg} />)}
        </div>
      )}
    </div>
  );
};
