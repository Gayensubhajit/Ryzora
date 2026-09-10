import React, { useState, useMemo } from "react";
import { useApp } from "../context/AppContext";
import { HeroBanner } from "../components/HeroBanner";
import { StoreCard } from "../components/StoreCard";
import { ContentTabBar, STORE_CONTENT_TABS } from "../components/ContentTabBar";
import { CategoryId } from "../types";
import { CategoryHeader } from "../components/catalogue/CategoryHeader";
import { SubcategoryTabs } from "../components/catalogue/SubcategoryTabs";
import { FilterBar, SortMode } from "../components/catalogue/FilterBar";
import { FilterPanel } from "../components/catalogue/FilterPanel";
import {
  getPackageSubtype,
  isLoginScreen,
} from "../components/catalogue/catalogueUtils";

type ActiveTab = CategoryId | "all";

/** Per-tab sub-filter pills */
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
  const [showFilters, setShowFilters] = useState(false);
  const [sortBy, setSortBy] = useState<SortMode>("recommended");

  // Advanced / Panel filter states
  const [selectedTypes, setSelectedTypes] = useState<string[]>([]);
  const [selectedCompatibility, setSelectedCompatibility] = useState<string[]>([]);
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [verifiedOnly, setVerifiedOnly] = useState<boolean>(false);
  const [repoFilter, setRepoFilter] = useState<string>("all");

  // Reset category-specific sub-filters when tab changes
  const handleTabChange = (tab: ActiveTab) => {
    setActiveTab(tab);
    setSubFilter("All");
    setSelectedTypes([]);
  };

  const handleToggleType = (type: string) => {
    setSelectedTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    );
  };

  const handleToggleCompatibility = (comp: string) => {
    setSelectedCompatibility((prev) =>
      prev.includes(comp) ? prev.filter((c) => c !== comp) : [...prev, comp]
    );
  };

  const handleClearFilters = () => {
    setSubFilter("All");
    setSelectedTypes([]);
    setSelectedCompatibility([]);
    setStatusFilter("all");
    setVerifiedOnly(false);
    setRepoFilter("all");
    setSortBy("recommended");
    setSearchQuery("");
  };

  const activeFilterCount = useMemo(() => {
    let count = 0;
    if (subFilter !== "All") count++;
    if (selectedTypes.length > 0) count += selectedTypes.length;
    if (selectedCompatibility.length > 0) count += selectedCompatibility.length;
    if (statusFilter !== "all") count++;
    if (verifiedOnly) count++;
    if (repoFilter !== "all") count++;
    return count;
  }, [subFilter, selectedTypes, selectedCompatibility, statusFilter, verifiedOnly, repoFilter]);

  const hasActiveFilters = activeFilterCount > 0 || searchQuery.length > 0;

  const currentWm = systemInfo?.window_manager?.toLowerCase() || "";

  // Available filter options for the current tab
  const availableTypes = useMemo(() => {
    if (activeTab === "lockscreens") {
      return ["Hyprlock", "Quickshell", "Swaylock", "SDDM"];
    }
    return TAB_SUB_FILTERS[activeTab] ? TAB_SUB_FILTERS[activeTab]!.filter((f) => f !== "All") : [];
  }, [activeTab]);

  const availableCompatibility = useMemo(() => {
    return ["Hyprland", "KDE", "Sway", "Wayland"];
  }, []);

  const filteredPackages = useMemo(() => {
    let list = packages.filter((pkg) => {
      // 1. Tab / Category Filter
      const isLockscreenMatch =
        activeTab === "lockscreens" &&
        (pkg.package_type === "lockscreen" ||
          pkg.category === "lockscreens" ||
          pkg.tags.some((t) => t.toLowerCase().includes("lockscreen")));

      const isRiceMatch =
        activeTab === "rices" &&
        (pkg.package_type === "rice" || pkg.category === "rices");

      const isThemeMatch =
        activeTab === "themes" &&
        (pkg.package_type === "theme" || pkg.category === "themes");

      const isWallpaperMatch =
        activeTab === "wallpapers" &&
        (pkg.package_type === "wallpaper" || pkg.category === "wallpapers");

      const isBarMatch =
        activeTab === "bars" &&
        (pkg.package_type === "waybar" || pkg.category === "bars");

      const isFastfetchMatch =
        activeTab === "fastfetch" &&
        (pkg.package_type === "fastfetch" || pkg.category === "fastfetch");

      const isTerminalMatch =
        activeTab === "terminal" &&
        (pkg.package_type === "terminal" || pkg.category === "terminal");

      const isBundleMatch =
        activeTab === "bundles" && pkg.category === "bundles";

      const matchesTab =
        activeTab === "all" ||
        isLockscreenMatch ||
        isRiceMatch ||
        isThemeMatch ||
        isWallpaperMatch ||
        isBarMatch ||
        isFastfetchMatch ||
        isTerminalMatch ||
        isBundleMatch ||
        pkg.category === activeTab;

      if (!matchesTab) return false;

      // 2. Search Query
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const inTitle = pkg.title.toLowerCase().includes(q);
        const inSub = pkg.subtitle.toLowerCase().includes(q);
        const inDesc = pkg.description.toLowerCase().includes(q);
        const inAuthor = pkg.author.name.toLowerCase().includes(q);
        const inTags = pkg.tags.some((t) => t.toLowerCase().includes(q));
        if (!inTitle && !inSub && !inDesc && !inAuthor && !inTags) {
          return false;
        }
      }

      // 3. Desktop Filter from topbar
      if (desktopFilter !== "all") {
        const matchesGlobalDesktop =
          pkg.supported_desktops.includes("universal") ||
          pkg.supported_desktops.includes(desktopFilter as any);
        if (!matchesGlobalDesktop) return false;
      }

      // 4. Sub-filter pill (All, Hyprlock, Quickshell, Swaylock, SDDM)
      if (subFilter !== "All") {
        const sf = subFilter.toLowerCase();
        if (activeTab === "lockscreens") {
          if (sf === "sddm") {
            const hasSddm = isLoginScreen(pkg) || pkg.lockscreen?.targets?.sddm != null || pkg.tags.some((t) => t.toLowerCase() === "sddm");
            if (!hasSddm) return false;
          } else if (sf === "hyprlock") {
            const hasHyprlock =
              pkg.tags.some((t) => t.toLowerCase() === "hyprlock") ||
              pkg.title.toLowerCase().includes("hyprlock") ||
              pkg.lockscreen?.targets?.hyprlock != null;
            if (!hasHyprlock) return false;
          } else if (sf === "quickshell") {
            const hasQuickshell =
              pkg.tags.some((t) => t.toLowerCase() === "quickshell" || t.toLowerCase() === "qylock") ||
              pkg.title.toLowerCase().includes("quickshell") ||
              pkg.lockscreen?.targets?.quickshell != null;
            if (!hasQuickshell) return false;
          } else if (sf === "swaylock") {
            const hasSwaylock =
              pkg.tags.some((t) => t.toLowerCase() === "swaylock") ||
              pkg.title.toLowerCase().includes("swaylock") ||
              pkg.lockscreen?.targets?.swaylock != null;
            if (!hasSwaylock) return false;
          }
        } else {
          const inTags = pkg.tags.some((t) => t.toLowerCase() === sf);
          const inDesktops = pkg.supported_desktops.some((d) => d.toLowerCase() === sf);
          const inComps = (pkg.components ?? []).some((c) =>
            c.component_type?.toLowerCase().includes(sf)
          );
          if (!inTags && !inDesktops && !inComps) return false;
        }
      }

      // 5. FilterPanel Selected Types
      if (selectedTypes.length > 0) {
        const subtype = getPackageSubtype(pkg).toLowerCase();
        const matchesAnyType = selectedTypes.some((t) => {
          const tl = t.toLowerCase();
          return (
            subtype.includes(tl) ||
            pkg.tags.some((tag) => tag.toLowerCase() === tl) ||
            pkg.title.toLowerCase().includes(tl)
          );
        });
        if (!matchesAnyType) return false;
      }

      // 6. FilterPanel Desktop Compatibility
      if (selectedCompatibility.length > 0) {
        const matchesAnyComp = selectedCompatibility.some((comp) => {
          const cl = comp.toLowerCase();
          if (cl === "wayland") return pkg.supported_display.includes("wayland");
          return (
            pkg.supported_desktops.includes("universal") ||
            pkg.supported_desktops.some((d) => d.toLowerCase() === cl)
          );
        });
        if (!matchesAnyComp) return false;
      }

      // 7. Installation / Update Status
      const installedRecord = installedPackages.find((p) => p.package_id === pkg.id);
      const isInstalled = !!installedRecord;
      const isUpdate =
        isInstalled && installedRecord
          ? parseFloat(pkg.version) > parseFloat(installedRecord.version)
          : false;

      if (statusFilter === "installed" && !isInstalled) return false;
      if (statusFilter === "not_installed" && isInstalled) return false;
      if (statusFilter === "updates" && !isUpdate) return false;

      // 8. Advanced: Verified Only
      if (verifiedOnly) {
        const isVerified =
          pkg.trust_tier === "official" ||
          pkg.trust_tier === "verified" ||
          pkg.author.verified;
        if (!isVerified) return false;
      }

      // 9. Advanced: Repository Filter
      if (repoFilter !== "all" && pkg.repository_id !== repoFilter) {
        return false;
      }

      return true;
    });

    // Sort Options
    list = [...list].sort((a, b) => {
      if (sortBy === "downloads") return b.downloads - a.downloads;
      if (sortBy === "rating") return b.rating - a.rating;
      if (sortBy === "newest") return compareSemver(b.version, a.version);
      if (sortBy === "updated") return (b.recent ? 1 : 0) - (a.recent ? 1 : 0) || compareSemver(b.version, a.version);
      if (sortBy === "alpha") return a.title.localeCompare(b.title);

      // "recommended": Featured first, then trending, then downloads
      const aScore = (a.featured ? 2 : 0) + (a.trending ? 1 : 0);
      const bScore = (b.featured ? 2 : 0) + (b.trending ? 1 : 0);
      return bScore - aScore || b.downloads - a.downloads;
    });

    return list;
  }, [
    packages,
    activeTab,
    searchQuery,
    desktopFilter,
    subFilter,
    selectedTypes,
    selectedCompatibility,
    statusFilter,
    verifiedOnly,
    repoFilter,
    sortBy,
    installedPackages,
  ]);

  const featuredRice = useMemo(() => packages.find((p) => p.featured), [packages]);

  const compatiblePackages = useMemo(
    () =>
      packages.filter((p) =>
        p.supported_desktops.some((d) => d === currentWm || currentWm.includes(d))
      ),
    [packages, currentWm]
  );

  const isAllTab = activeTab === "all";
  const subFilters = TAB_SUB_FILTERS[activeTab];

  return (
    <div className="space-y-0 pb-12">
      {/* ── Top-level content tabs ── */}
      <div className="sticky top-0 z-20 bg-[var(--rz-bg)] -mx-4 sm:-mx-6 px-4 sm:px-6 mb-3 border-b border-[var(--rz-border-subtle)] shadow-xs">
        <ContentTabBar
          activeTab={activeTab}
          onTabChange={handleTabChange}
          tabs={STORE_CONTENT_TABS}
        />
      </div>

      {/* ── "All" Tab Editorial Home View ── */}
      {isAllTab && !searchQuery && (
        <>
          {/* Hero Banner */}
          {featuredRice && (
            <div className="mb-6">
              <HeroBanner featuredPackage={featuredRice} />
            </div>
          )}

          {/* Compatible with your setup */}
          {compatiblePackages.length > 0 && (
            <section className="mb-8">
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-secondary)]">
                  Compatible with {systemInfo?.window_manager || "your desktop"}
                </h2>
                <span className="text-[11px] text-[var(--rz-text-muted)] font-mono">
                  {compatiblePackages.length} available
                </span>
              </div>
              <div className="store-grid-fluid">
                {compatiblePackages.slice(0, 6).map((pkg) => (
                  <StoreCard key={pkg.id} packageItem={pkg} />
                ))}
              </div>
            </section>
          )}

          {/* Trending */}
          <section className="mb-8">
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-[11px] font-bold uppercase tracking-widest text-[var(--rz-text-muted)]">
                Trending
              </h2>
            </div>
            <div className="store-grid-fluid">
              {packages
                .filter((p) => p.trending)
                .slice(0, 12)
                .map((pkg) => (
                  <StoreCard key={pkg.id} packageItem={pkg} />
                ))}
            </div>
          </section>

          {/* New & Noteworthy */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <h2 className="text-[11px] font-bold uppercase tracking-widest text-[var(--rz-text-muted)]">
                New & Noteworthy
              </h2>
            </div>
            <div className="store-grid-fluid">
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

      {/* ── Category View (Lock Screens, Rices, etc.) or Active Search ── */}
      {(!isAllTab || searchQuery) && (
        <>
          {/* Unified Category Header */}
          <CategoryHeader
            category={activeTab}
            count={filteredPackages.length}
            subFilter={subFilter}
          />

          {/* Subcategory Filter Pills (e.g. All | Hyprlock | Quickshell | Swaylock | SDDM) */}
          {subFilters && (
            <SubcategoryTabs
              subFilters={subFilters}
              activeSubFilter={subFilter}
              onSelectSubFilter={setSubFilter}
              category={activeTab}
            />
          )}

          {/* Clean Filter & Sort Bar */}
          <FilterBar
            showFilters={showFilters}
            onToggleFilters={() => setShowFilters((v) => !v)}
            activeFilterCount={activeFilterCount}
            sortBy={sortBy}
            onChangeSort={setSortBy}
            onClearFilters={handleClearFilters}
            hasActiveFilters={hasActiveFilters}
          />

          {/* Expanded Filter Panel (Basic + Collapsible Advanced) */}
          {showFilters && (
            <FilterPanel
              availableTypes={availableTypes}
              selectedTypes={selectedTypes}
              onToggleType={handleToggleType}
              availableCompatibility={availableCompatibility}
              selectedCompatibility={selectedCompatibility}
              onToggleCompatibility={handleToggleCompatibility}
              status={statusFilter}
              onChangeStatus={setStatusFilter}
              verifiedOnly={verifiedOnly}
              onToggleVerified={setVerifiedOnly}
              repositories={repositories}
              selectedRepo={repoFilter}
              onChangeRepo={setRepoFilter}
            />
          )}

          {/* Visual Store Grid: 6 Columns down to 2 Columns */}
          {filteredPackages.length === 0 ? (
            <div className="py-16 text-center text-xs text-[var(--rz-text-secondary)] bg-[var(--rz-surface)] rounded-xl border border-[var(--rz-border-subtle)] my-4">
              <p className="font-semibold text-[var(--rz-text)]">
                No configurations match the selected criteria.
              </p>
              <p className="mt-1 text-[var(--rz-text-muted)] text-[11px]">
                Try adjusting your subcategory pills or clear active filters.
              </p>
              {hasActiveFilters && (
                <button
                  type="button"
                  onClick={handleClearFilters}
                  className="mt-3 px-3 py-1.5 text-xs font-semibold rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-accent-text)] hover:border-[var(--rz-accent)] transition-colors cursor-pointer"
                >
                  Reset all filters
                </button>
              )}
            </div>
          ) : (
            <div className="store-grid-fluid">
              {filteredPackages.map((pkg) => (
                <StoreCard key={pkg.id} packageItem={pkg} />
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};
