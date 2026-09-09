import React, { useState, useMemo } from "react";
import { SlidersHorizontal, X } from "lucide-react";
import { useApp } from "../context/AppContext";
import { HeroBanner } from "../components/HeroBanner";
import { StoreCard } from "../components/StoreCard";
import { ContentTabBar, STORE_CONTENT_TABS } from "../components/ContentTabBar";
import { CategoryId } from "../types";

type SortOption = "relevance" | "trending" | "rating" | "downloads" | "name" | "newest";
type ActiveTab = CategoryId | "all";

const PORTRAIT_TABS = new Set<ActiveTab>(["lockscreens", "wallpapers"]);

/** Per-tab sub-filter pills (mirrors CategoryView logic) */
const TAB_SUB_FILTERS: Partial<Record<ActiveTab, string[]>> = {
  lockscreens: ["All", "Hyprlock", "Quickshell", "Swaylock", "SDDM"],
  rices: ["All", "Hyprland", "Sway", "KDE", "GNOME"],
  themes: ["All", "GTK", "Qt", "Application"],
  wallpapers: ["All", "Abstract", "Nature", "Minimal", "Sci-Fi"],
  bars: ["All", "Waybar", "Eww", "Polybar"],
  terminal: ["All", "Kitty", "Alacritty", "Foot"],
};

export function compareSemver(v1: string, v2: string): number {
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

export const DiscoverView: React.FC = () => {
  const {
    packages,
    searchQuery,
    setSearchQuery,
    desktopFilter,
    systemInfo,
    installedPackages,
    repositories,
  } = useApp();

  const [activeTab, setActiveTab] = useState<ActiveTab>("all");
  const [subFilter, setSubFilter] = useState<string>("All");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [repoFilter, setRepoFilter] = useState<string>("all");
  const [sortBy, setSortBy] = useState<SortOption>("relevance");
  const [showFilters, setShowFilters] = useState(false);

  // Reset sub-filter when tab changes
  const handleTabChange = (tab: ActiveTab) => {
    setActiveTab(tab);
    setSubFilter("All");
  };

  const currentWm = systemInfo?.window_manager?.toLowerCase() || "";

  const filteredPackages = useMemo(() => {
    let list = packages.filter((pkg) => {
      // Tab / category filter
      const matchesTab =
        activeTab === "all" ||
        pkg.category === activeTab ||
        (activeTab === "lockscreens" && (pkg.package_type === "lockscreen" || pkg.category === "lockscreens")) ||
        (activeTab === "rices" && (pkg.package_type === "rice" || pkg.category === "rices")) ||
        (activeTab === "themes" && (pkg.package_type === "theme" || pkg.category === "themes")) ||
        (activeTab === "wallpapers" && (pkg.package_type === "wallpaper" || pkg.category === "wallpapers")) ||
        (activeTab === "bars" && (pkg.package_type === "waybar" || pkg.category === "bars")) ||
        (activeTab === "fastfetch" && (pkg.package_type === "fastfetch" || pkg.category === "fastfetch")) ||
        (activeTab === "terminal" && (pkg.package_type === "terminal" || pkg.category === "terminal")) ||
        (activeTab === "bundles" && pkg.category === "bundles");

      // Search
      const matchesSearch =
        !searchQuery ||
        pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.author.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase()));

      // Desktop
      const matchesDesktop =
        desktopFilter === "all" ||
        pkg.supported_desktops.includes("universal") ||
        pkg.supported_desktops.includes(desktopFilter as any);

      // Status filter
      const installedRecord = installedPackages.find((p) => p.package_id === pkg.id);
      const isInstalled = !!installedRecord;
      const matchesStatus =
        statusFilter === "all" ||
        (statusFilter === "official" && pkg.trust_tier === "official") ||
        (statusFilter === "verified" && pkg.trust_tier === "verified") ||
        (statusFilter === "community" && pkg.trust_tier === "community") ||
        (statusFilter === "featured" && pkg.featured) ||
        (statusFilter === "trending" && pkg.trending) ||
        (statusFilter === "installed" && isInstalled) ||
        (statusFilter === "stable" && pkg.release_channel === "stable") ||
        (statusFilter === "beta" && pkg.release_channel === "beta") ||
        (statusFilter === "nightly" && pkg.release_channel === "nightly");

      // Repository filter
      const matchesRepo = repoFilter === "all" || pkg.repository_id === repoFilter;

      // Sub-filter
      const matchesSubFilter =
        subFilter === "All" ||
        pkg.tags.some((t) => t.toLowerCase() === subFilter.toLowerCase()) ||
        pkg.supported_desktops.some((d) => d.toLowerCase() === subFilter.toLowerCase()) ||
        (pkg.components ?? []).some((c) =>
          c.component_type?.toLowerCase().includes(subFilter.toLowerCase())
        );

      return matchesTab && matchesSearch && matchesDesktop && matchesStatus && matchesRepo && matchesSubFilter;
    });

    // Sort
    list = [...list].sort((a, b) => {
      if (sortBy === "trending") return (b.trending ? 1 : 0) - (a.trending ? 1 : 0) || b.downloads - a.downloads;
      if (sortBy === "rating") return b.rating - a.rating;
      if (sortBy === "downloads") return b.downloads - a.downloads;
      if (sortBy === "name") return a.title.localeCompare(b.title);
      if (sortBy === "newest") return compareSemver(b.version, a.version);
      // relevance: featured first, then trending, then downloads
      const aScore = (a.featured ? 2 : 0) + (a.trending ? 1 : 0);
      const bScore = (b.featured ? 2 : 0) + (b.trending ? 1 : 0);
      return bScore - aScore || b.downloads - a.downloads;
    });

    return list;
  }, [packages, activeTab, searchQuery, desktopFilter, statusFilter, repoFilter, sortBy, installedPackages, subFilter]);

  const featuredRice = useMemo(() => packages.find((p) => p.featured), [packages]);

  const compatiblePackages = useMemo(
    () =>
      packages.filter((p) =>
        p.supported_desktops.some((d) => d === currentWm || currentWm.includes(d))
      ),
    [packages, currentWm]
  );

  const isAllTab = activeTab === "all";
  const isFiltering = !isAllTab || statusFilter !== "all" || repoFilter !== "all" || sortBy !== "relevance" || searchQuery !== "";
  const subFilters = TAB_SUB_FILTERS[activeTab];
  const usePortrait = PORTRAIT_TABS.has(activeTab);

  return (
    <div className="space-y-0 pb-10">
      {/* ── Content tab bar ── */}
      <div className="-mx-6 -mt-6 mb-4 sticky top-0 z-10">
        <ContentTabBar
          activeTab={activeTab}
          onTabChange={handleTabChange}
          tabs={STORE_CONTENT_TABS}
        />
      </div>

      {/* ── All tab: editorial home ── */}
      {isAllTab && !searchQuery && (
        <>
          {/* Hero */}
          {featuredRice && (
            <div className="mb-6">
              <HeroBanner featuredPackage={featuredRice} />
            </div>
          )}

          {/* Compatible with your setup */}
          {compatiblePackages.length > 0 && (
            <section className="mb-8">
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
                  Compatible with {systemInfo?.window_manager || "your desktop"}
                </h2>
                <span className="text-[11px] text-[var(--text-faint)] font-mono">{compatiblePackages.length} available</span>
              </div>
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
                {compatiblePackages.slice(0, 6).map((pkg) => (
                  <StoreCard key={pkg.id} packageItem={pkg} />
                ))}
              </div>
            </section>
          )}

          {/* Trending */}
          <section className="mb-8">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">Trending</h2>
            </div>
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {packages
                .filter((p) => p.trending)
                .slice(0, 12)
                .map((pkg) => (
                  <StoreCard key={pkg.id} packageItem={pkg} />
                ))}
            </div>
          </section>

          {/* New arrivals */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">New & Noteworthy</h2>
            </div>
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {[...packages]
                .sort((a, b) => compareSemver(b.version, a.version))
                .slice(0, 12)
                .map((pkg) => (
                  <StoreCard key={pkg.id} packageItem={pkg} />
                ))}
            </div>
          </section>
        </>
      )}

      {/* ── Category tab or search: dense catalogue ── */}
      {(!isAllTab || searchQuery) && (
        <>
          {/* Header row */}
          <div className="flex items-center justify-between mb-3 pt-1">
            <div className="flex items-center gap-2">
              <h1 className="text-sm font-bold text-[var(--text-primary)] capitalize">
                {searchQuery ? `Results for "${searchQuery}"` : (activeTab === "all" ? "All Packages" : activeTab)}
              </h1>
              <span className="text-[11px] text-[var(--text-faint)] font-mono">
                ({filteredPackages.length})
              </span>
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setShowFilters((v) => !v)}
                className={[
                  "flex items-center gap-1.5 px-2.5 py-1 rounded-md text-[11px] border transition-colors",
                  showFilters
                    ? "bg-[var(--accent-muted)] border-[var(--accent)] text-[var(--accent-text)]"
                    : "border-[var(--border-subtle)] text-[var(--text-muted)] hover:border-[var(--border-strong)]",
                ].join(" ")}
              >
                <SlidersHorizontal className="w-3 h-3" />
                Filters
              </button>
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as SortOption)}
                className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] outline-none text-[11px] cursor-pointer"
              >
                <option value="relevance">Relevance</option>
                <option value="trending">Trending</option>
                <option value="rating">Highest Rated</option>
                <option value="downloads">Most Downloaded</option>
                <option value="name">Name (A-Z)</option>
                <option value="newest">Newest</option>
              </select>
            </div>
          </div>

          {/* Expanded filter panel */}
          {showFilters && (
            <div className="mb-4 p-3 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-wrap gap-2 text-[11px]">
              <select
                value={statusFilter}
                onChange={(e) => setStatusFilter(e.target.value)}
                className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] outline-none"
              >
                <option value="all">All Tiers</option>
                <option value="official">Official</option>
                <option value="verified">Verified</option>
                <option value="community">Community</option>
                <option value="featured">Featured</option>
                <option value="trending">Trending</option>
                <option value="installed">Installed</option>
                <option value="stable">Stable</option>
                <option value="beta">Beta</option>
                <option value="nightly">Nightly</option>
              </select>
              {repositories.length > 1 && (
                <select
                  value={repoFilter}
                  onChange={(e) => setRepoFilter(e.target.value)}
                  className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] outline-none"
                >
                  <option value="all">All Repositories</option>
                  {repositories.map((r) => (
                    <option key={r.id} value={r.id}>{r.name}</option>
                  ))}
                </select>
              )}
              {isFiltering && (
                <button
                  onClick={() => {
                    setStatusFilter("all");
                    setRepoFilter("all");
                    setSortBy("relevance");
                    setSearchQuery("");
                  }}
                  className="flex items-center gap-1 px-2.5 py-1 rounded border border-[var(--border-subtle)] text-[var(--text-muted)] hover:text-white hover:border-[var(--border-strong)] transition-colors"
                >
                  <X className="w-3 h-3" />
                  Clear
                </button>
              )}
            </div>
          )}

          {/* Sub-filter pills (per-category) */}
          {subFilters && subFilters.length > 0 && (
            <div className="flex items-center gap-1.5 mb-4 overflow-x-auto scrollbar-none" style={{ scrollbarWidth: "none" }}>
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

          {/* Grid */}
          {filteredPackages.length === 0 ? (
            <div className="py-20 text-center text-xs text-[var(--text-muted)]">
              No packages match your search or filter criteria.
              {searchQuery && (
                <button
                  onClick={() => setSearchQuery("")}
                  className="block mx-auto mt-3 px-3 py-1 text-[11px] rounded border border-[var(--border-subtle)] text-[var(--accent-text)] hover:border-[var(--accent)] transition-colors"
                >
                  Clear search
                </button>
              )}
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
              {filteredPackages.map((pkg) => (
                <StoreCard key={pkg.id} packageItem={pkg} portrait={usePortrait} />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};
