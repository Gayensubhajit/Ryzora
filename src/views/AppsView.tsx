/**
 * AppsView — Phase 23C.1
 *
 * Modern software storefront and catalogue for Ryzora.
 * Key Behaviors:
 *   - Storefront Home ("All"): Featured hero, distinct Popular essentials, distinct Recommended picks, category grid.
 *   - Dedicated Filtered Catalogue: Clean filtered grid for Popular, Installed, or specific categories with no duplicate sections.
 *   - Strict Canonical Deduplication: Normalizes package/desktop/metadata IDs so no app is repeated across or within sections.
 *   - Catalogue-like Tiles: Lightweight low-contrast surfaces with 68px authentic artwork, subtle elevation, and minimal installed indicator.
 *   - Preserved scroll & category state across detail navigation.
 */

import React, { useState, useEffect, useMemo, useRef, useCallback } from "react";
import {
  Search,
  CheckCircle2,
  RefreshCw,
  ArrowRight,
  Layers,
  Code2,
  Globe,
  Tv,
  ImageIcon,
  Gamepad2,
  Terminal,
  Sparkles,
} from "lucide-react";
import { packageEngine, pacmanAppProvider } from "../providers/index.ts";
import type { PackageItem } from "../providers/types.ts";
import {
  AppIcon,
  resolveAppMetadata,
  resolveCanonicalAppId,
  deduplicateAppPackages,
  POPULAR_CANONICAL_IDS,
  RECOMMENDED_CANONICAL_IDS,
} from "../components/apps/AppIconResolver.tsx";
import { AppDetailPage } from "../components/apps/AppDetailPage.tsx";

const CATEGORIES = [
  "All",
  "Popular",
  "Internet",
  "Development",
  "Multimedia",
  "Graphics",
  "Games",
  "Utilities",
  "Installed",
] as const;

type CategoryFilter = typeof CATEGORIES[number];

export const AppsView: React.FC = () => {
  const [packages, setPackages] = useState<PackageItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>("All");
  const [selectedApp, setSelectedApp] = useState<PackageItem | null>(null);

  // Load all app packages
  const loadApps = async () => {
    setLoading(true);
    try {
      const items = await packageEngine.discoverByCategory("apps");
      setPackages(items);
    } catch (err) {
      console.error("Failed to discover apps:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadApps();
  }, []);

  useEffect(() => {
    try {
      const targetAppId = new URLSearchParams(window.location.search).get("app");
      if (targetAppId && packages.length > 0) {
        const found = packages.find(
          (p) =>
            p.id.toLowerCase() === targetAppId.toLowerCase() ||
            resolveCanonicalAppId(p.id) === targetAppId.toLowerCase()
        );
        if (found) {
          setSelectedApp(found);
        }
      }
    } catch {}
  }, [packages]);

  // Scroll position preservation
  const gridScrollRef = useRef<number>(0);
  const gridRef = useRef<HTMLDivElement | null>(null);

  // Save scroll position when entering detail view
  const openAppDetail = useCallback((app: PackageItem) => {
    gridScrollRef.current = gridRef.current?.scrollTop ?? 0;
    setSelectedApp(app);
  }, []);

  // Restore scroll position when returning
  const handleBack = useCallback(() => {
    setSelectedApp(null);
    requestAnimationFrame(() => {
      if (gridRef.current) {
        gridRef.current.scrollTop = gridScrollRef.current;
      }
    });
  }, []);

  const checkInstalled = useCallback((pkg: PackageItem): boolean => {
    const meta = pacmanAppProvider.getMeta(pkg);
    return meta?.isInstalled ?? false;
  }, []);

  // Step 1: Deduplicate entire package pool by canonical application ID
  const deduplicatedPackages = useMemo(() => {
    return deduplicateAppPackages(packages);
  }, [packages]);

  // Step 2: Featured hero app (Firefox or first curated package)
  const featuredHeroApp = useMemo(() => {
    return (
      deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === "firefox") ||
      deduplicatedPackages[0]
    );
  }, [deduplicatedPackages]);

  // Step 3: Curated Popular essentials (strictly up to 8 packages matching POPULAR_CANONICAL_IDS)
  const popularPicks = useMemo(() => {
    const popularMap = new Map<string, PackageItem>();

    // Match ordered POPULAR_CANONICAL_IDS
    for (const popId of POPULAR_CANONICAL_IDS) {
      const match = deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === popId);
      if (match && !popularMap.has(popId)) {
        popularMap.set(popId, match);
      }
    }

    // If fewer than 8, fill with other curated apps that are not recommended
    if (popularMap.size < 8) {
      for (const p of deduplicatedPackages) {
        const cid = resolveCanonicalAppId(p.id);
        const meta = resolveAppMetadata(p.id, p.title);
        if (
          meta.isCuratedApp &&
          !popularMap.has(cid) &&
          !RECOMMENDED_CANONICAL_IDS.includes(cid)
        ) {
          popularMap.set(cid, p);
          if (popularMap.size >= 8) break;
        }
      }
    }

    return Array.from(popularMap.values());
  }, [deduplicatedPackages]);

  // Step 4: Curated Recommended picks (strictly distinct from Popular and Hero!)
  const recommendedPicks = useMemo(() => {
    const popularCanonicalSet = new Set(popularPicks.map((p) => resolveCanonicalAppId(p.id)));
    if (featuredHeroApp) {
      popularCanonicalSet.add(resolveCanonicalAppId(featuredHeroApp.id));
    }

    const recommendedMap = new Map<string, PackageItem>();

    // Match ordered RECOMMENDED_CANONICAL_IDS
    for (const recId of RECOMMENDED_CANONICAL_IDS) {
      const match = deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === recId);
      if (match && !popularCanonicalSet.has(recId) && !recommendedMap.has(recId)) {
        recommendedMap.set(recId, match);
      }
    }

    // If fewer than 8, fill with remaining non-popular apps
    if (recommendedMap.size < 8) {
      for (const p of deduplicatedPackages) {
        const cid = resolveCanonicalAppId(p.id);
        if (!popularCanonicalSet.has(cid) && !recommendedMap.has(cid)) {
          recommendedMap.set(cid, p);
          if (recommendedMap.size >= 8) break;
        }
      }
    }

    return Array.from(recommendedMap.values());
  }, [deduplicatedPackages, popularPicks, featuredHeroApp]);

  // Step 5: Filtered catalogue for category or search views
  const filteredPackages = useMemo(() => {
    return deduplicatedPackages.filter((pkg) => {
      const meta = resolveAppMetadata(pkg.id, pkg.title);
      const isInstalled = checkInstalled(pkg);
      const cid = resolveCanonicalAppId(pkg.id);

      // Category filter
      if (selectedCategory === "Installed" && !isInstalled) {
        return false;
      }
      if (selectedCategory === "Popular") {
        const isPopular = POPULAR_CANONICAL_IDS.includes(cid) || meta.isCuratedApp;
        if (!isPopular) return false;
      }
      if (
        selectedCategory !== "All" &&
        selectedCategory !== "Installed" &&
        selectedCategory !== "Popular" &&
        meta.category !== selectedCategory
      ) {
        return false;
      }

      // Search query filter
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        const matchesName = pkg.id.toLowerCase().includes(q) || pkg.title.toLowerCase().includes(q);
        const matchesDesc = (pkg.description || "").toLowerCase().includes(q);
        const matchesMeta =
          meta.displayName.toLowerCase().includes(q) ||
          meta.publisher.toLowerCase().includes(q) ||
          meta.summary.toLowerCase().includes(q);
        return matchesName || matchesDesc || matchesMeta;
      }

      return true;
    });
  }, [deduplicatedPackages, selectedCategory, searchQuery, checkInstalled]);

  // If an app is selected, render the full-screen AppDetailPage!
  if (selectedApp) {
    return (
      <AppDetailPage
        app={selectedApp}
        onBack={handleBack}
        onStatusChanged={() => {
          loadApps();
        }}
        onSelectRelated={(relatedId) => {
          const found = packages.find(
            (p) =>
              p.id.toLowerCase() === relatedId.toLowerCase() ||
              resolveCanonicalAppId(p.id) === relatedId.toLowerCase()
          );
          if (found) {
            setSelectedApp(found);
          }
        }}
      />
    );
  }

  const isStorefrontHome = selectedCategory === "All" && !searchQuery.trim();

  return (
    <div
      ref={gridRef}
      className="flex flex-col flex-1 h-full w-full overflow-y-auto px-8 py-6 space-y-8 bg-[var(--rz-bg)] text-[var(--rz-text)] animate-fadeIn scroll-smooth"
    >
      {/* ── Top Header & Integrated Search Bar ── */}
      <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 border-b border-[var(--rz-border-subtle)] pb-4">
        <div>
          <h1 className="text-3xl font-extrabold tracking-tight text-[var(--rz-text)]">Applications</h1>
          <p className="text-xs text-[var(--rz-text-muted)] pt-1">
            Discover, preview, install and manage software for your system
          </p>
        </div>

        {/* Search Field */}
        <div className="relative w-full md:w-96">
          <Search
            size={16}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--rz-text-muted)] pointer-events-none"
          />
          <input
            type="text"
            placeholder="Search apps, games, tools..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] focus:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] focus:border-blue-500/50 rounded-xl pl-10 pr-9 py-2 text-xs placeholder:text-[var(--rz-text-muted)] text-[var(--rz-text)] focus:outline-none focus:ring-2 focus:ring-blue-500/20 transition-all shadow-xs"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery("")}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] cursor-pointer"
            >
              ×
            </button>
          )}
        </div>
      </div>

      {/* ── Lightweight Segmented Category Navigation ── */}
      <div className="flex items-center gap-1.5 overflow-x-auto pb-1 scrollbar-none">
        {CATEGORIES.map((cat) => {
          const isSelected = selectedCategory === cat;
          return (
            <button
              key={cat}
              onClick={() => setSelectedCategory(cat)}
              className={`px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all shrink-0 cursor-pointer ${
                isSelected
                  ? "bg-blue-600 text-white shadow-xs"
                  : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-transparent"
              }`}
            >
              {cat}
            </button>
          );
        })}
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-24 text-[var(--rz-text-muted)] gap-2">
          <RefreshCw size={18} className="animate-spin text-blue-600 dark:text-blue-400" />
          <span className="text-xs">Loading application catalog...</span>
        </div>
      ) : isStorefrontHome ? (
        /* ═══════════════════════════════════════════════════════════════
           STOREFRONT HOME VIEW (Only rendered when selectedCategory === "All" && !searchQuery)
           ═══════════════════════════════════════════════════════════════ */
        <div className="space-y-10">
          {/* ── Cinematic Featured Hero Showcase ── */}
          {featuredHeroApp && (
            <div
              onClick={() => openAppDetail(featuredHeroApp)}
              className="relative w-full rounded-2xl md:rounded-3xl p-8 md:p-10 flex flex-col md:flex-row items-center justify-between gap-8 bg-[var(--rz-surface)] border border-[var(--rz-border)] shadow-md hover:shadow-lg hover:border-[var(--rz-border-strong)] transition-all duration-300 cursor-pointer group overflow-hidden"
            >
              {/* Very subtle oversized blurred watermark icon behind the right side (replaces giant gradient) */}
              <div className="absolute -right-12 -top-12 w-96 h-96 pointer-events-none select-none opacity-5 dark:opacity-10 blur-xl scale-125 overflow-hidden flex items-center justify-center">
                <AppIcon appId={featuredHeroApp.id} size="2xl" />
              </div>

              <div className="space-y-4 z-10 max-w-xl">
                <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-[11px] font-bold tracking-wider uppercase bg-blue-600/10 text-blue-600 dark:text-blue-400 border border-blue-500/20">
                  <Sparkles size={12} />
                  <span>Featured application</span>
                </div>

                <div className="space-y-1">
                  <h2 className="text-3xl sm:text-4xl md:text-5xl font-extrabold tracking-tight text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                    {resolveAppMetadata(featuredHeroApp.id).displayName}
                  </h2>
                  <div className="text-xs sm:text-sm font-semibold text-[var(--rz-text-muted)] flex items-center gap-1.5 pt-0.5">
                    <span>{resolveAppMetadata(featuredHeroApp.id).publisher}</span>
                    <CheckCircle2 size={13} className="text-blue-600 dark:text-blue-400" />
                  </div>
                </div>

                <p className="text-sm sm:text-base text-[var(--rz-text-secondary)] leading-relaxed max-w-lg">
                  {resolveAppMetadata(featuredHeroApp.id).summary || featuredHeroApp.description}
                </p>

                <div className="pt-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      openAppDetail(featuredHeroApp);
                    }}
                    className="inline-flex items-center gap-2 px-6 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs sm:text-sm shadow-md transition-all cursor-pointer group-hover:gap-3"
                  >
                    <span>View application</span>
                    <ArrowRight size={15} />
                  </button>
                </div>
              </div>

              {/* Large Authentic Artwork Container */}
              <div className="shrink-0 p-4 sm:p-5 rounded-3xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] group-hover:scale-105 transition-transform duration-300 shadow-sm z-10">
                <AppIcon appId={featuredHeroApp.id} size="2xl" />
              </div>
            </div>
          )}

          {/* ── Popular Applications Section ── */}
          {popularPicks.length > 0 && (
            <div className="space-y-4">
              <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-2">
                <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Popular apps</h2>
                <button
                  onClick={() => setSelectedCategory("Popular")}
                  className="text-xs text-blue-600 dark:text-blue-400 hover:underline cursor-pointer font-medium"
                >
                  See all →
                </button>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-8 gap-4">
                {popularPicks.map((app) => (
                  <AppCatalogueTile
                    key={app.id}
                    app={app}
                    isInstalled={checkInstalled(app)}
                    onClick={() => openAppDetail(app)}
                  />
                ))}
              </div>
            </div>
          )}

          {/* ── Recommended for You Section (Strictly distinct from Popular) ── */}
          {recommendedPicks.length > 0 && (
            <div className="space-y-4">
              <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-2">
                <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Recommended for you</h2>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-8 gap-4">
                {recommendedPicks.map((app) => (
                  <AppCatalogueTile
                    key={app.id}
                    app={app}
                    isInstalled={checkInstalled(app)}
                    onClick={() => openAppDetail(app)}
                  />
                ))}
              </div>
            </div>
          )}

          {/* ── Browse by Category Matrix ── */}
          <div className="space-y-4 pt-4 border-t border-[var(--rz-border-subtle)]">
            <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Browse by category</h2>
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3">
              {[
                { name: "Internet", icon: Globe },
                { name: "Development", icon: Code2 },
                { name: "Multimedia", icon: Tv },
                { name: "Graphics", icon: ImageIcon },
                { name: "Games", icon: Gamepad2 },
                { name: "Utilities", icon: Terminal },
              ].map(({ name, icon: CatIcon }) => (
                <button
                  key={name}
                  onClick={() => setSelectedCategory(name as CategoryFilter)}
                  className="flex items-center gap-3 p-3.5 rounded-xl bg-[var(--rz-surface)]/60 hover:bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] text-left transition-all cursor-pointer group shadow-xs hover:shadow-sm"
                >
                  <div className="p-2 rounded-lg bg-blue-600/10 text-blue-600 dark:text-blue-400 group-hover:scale-110 transition-transform">
                    <CatIcon size={16} />
                  </div>
                  <span className="text-xs font-semibold text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                    {name}
                  </span>
                </button>
              ))}
            </div>
          </div>
        </div>
      ) : (
        /* ═══════════════════════════════════════════════════════════════
           DEDICATED FILTERED CATALOGUE VIEW (Rendered when category selected or searching)
           ═══════════════════════════════════════════════════════════════ */
        <div className="space-y-6">
          <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-3">
            <div>
              <h2 className="text-xl font-bold tracking-tight text-[var(--rz-text)]">
                {searchQuery.trim()
                  ? `Search results for "${searchQuery.trim()}"`
                  : selectedCategory === "Popular"
                  ? "Popular applications"
                  : selectedCategory === "Installed"
                  ? "Installed applications"
                  : `${selectedCategory} applications`}
              </h2>
              <p className="text-xs text-[var(--rz-text-muted)] pt-0.5">
                {filteredPackages.length} {filteredPackages.length === 1 ? "application" : "applications"} available
              </p>
            </div>
          </div>

          {filteredPackages.length === 0 ? (
            <div className="text-center py-20 space-y-3">
              <Layers size={36} className="mx-auto text-[var(--rz-text-muted)] opacity-60" />
              <h3 className="text-sm font-semibold text-[var(--rz-text)]">No applications found</h3>
              <p className="text-xs text-[var(--rz-text-muted)] max-w-sm mx-auto">
                {searchQuery.trim()
                  ? `No applications matched "${searchQuery}". Try adjusting your search query.`
                  : `No applications found in the ${selectedCategory} category.`}
              </p>
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
              {filteredPackages.map((app) => (
                <AppCatalogueTile
                  key={app.id}
                  app={app}
                  isInstalled={checkInstalled(app)}
                  onClick={() => openAppDetail(app)}
                />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

// ── Lightweight Catalogue-Like App Tile ───────────────────────────────────────
const AppCatalogueTile: React.FC<{
  app: PackageItem;
  isInstalled: boolean;
  onClick: () => void;
}> = ({ app, isInstalled, onClick }) => {
  const meta = resolveAppMetadata(app.id, app.title);

  return (
    <div
      onClick={onClick}
      className="group flex flex-col items-center text-center p-4 rounded-2xl bg-[var(--rz-surface)]/30 hover:bg-[var(--rz-surface)] border border-transparent hover:border-[var(--rz-border-subtle)] transition-all duration-200 ease-out cursor-pointer hover:shadow-md hover:-translate-y-0.5"
    >
      <div className="mb-3 transition-transform group-hover:scale-105 duration-200 flex items-center justify-center">
        <AppIcon appId={app.id} size="lg" />
      </div>
      <span className="text-xs sm:text-[13px] font-bold text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate w-full">
        {meta.displayName}
      </span>
      <span className="text-[11px] text-[var(--rz-text-muted)] truncate w-full pt-0.5">
        {meta.category}
      </span>
      {isInstalled ? (
        <span className="mt-2 inline-flex items-center gap-1 text-[11px] font-medium text-emerald-600 dark:text-emerald-400">
          <CheckCircle2 size={12} />
          <span>Installed</span>
        </span>
      ) : (
        <span className="mt-2 h-4" />
      )}
    </div>
  );
};
