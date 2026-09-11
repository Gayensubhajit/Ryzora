/**
 * AppsView — Phase 23C
 *
 * Modern software-store landing page for Ryzora.
 * Design Philosophy:
 *   - Dark neutral canvas (#090b0f)
 *   - Restrained Ryzora blue (#3B82F6) interactive accents
 *   - Cinematic application hero showcase with subtle atmospheric glow
 *   - Lightweight segmented category navigation
 *   - Popular & Recommended app tiles with 68px authentic icons and 150ms hover elevation
 *   - Preserved search query & scroll state when navigating to/from AppDetailPage
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
} from "lucide-react";
import { packageEngine, pacmanAppProvider } from "../providers/index.ts";
import type { PackageItem } from "../providers/types.ts";
import { AppIcon, resolveAppMetadata } from "../components/apps/AppIconResolver.tsx";
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

  const checkInstalled = (pkg: PackageItem): boolean => {
    const meta = pacmanAppProvider.getMeta(pkg);
    return meta?.isInstalled ?? false;
  };

  // Filter and rank items
  const filteredPackages = useMemo(() => {
    return packages.filter((pkg) => {
      const meta = resolveAppMetadata(pkg.id, pkg.title);
      const isInstalled = checkInstalled(pkg);

      // Category filter
      if (selectedCategory === "Installed" && !isInstalled) {
        return false;
      }
      if (selectedCategory === "Popular" && !meta.isCuratedApp) {
        return false;
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
  }, [packages, selectedCategory, searchQuery]);

  // Featured hero app (Firefox or first curated)
  const featuredHeroApp = useMemo(() => {
    return packages.find((p) => p.id === "firefox") || packages[0];
  }, [packages]);

  // Popular essentials
  const popularPicks = useMemo(() => {
    return packages
      .filter((p) => {
        const meta = resolveAppMetadata(p.id);
        return meta.isCuratedApp;
      })
      .slice(0, 8);
  }, [packages]);

  // Recommended for you (non-curated or extended catalog)
  const recommendedPicks = useMemo(() => {
    const curatedIds = new Set(popularPicks.map((p) => p.id));
    return packages.filter((p) => !curatedIds.has(p.id)).slice(0, 8);
  }, [packages, popularPicks]);

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
          const found = packages.find((p) => p.id === relatedId);
          if (found) {
            setSelectedApp(found);
          }
        }}
      />
    );
  }

  return (
    <div
      ref={gridRef}
      className="flex flex-col flex-1 h-full w-full overflow-y-auto px-8 py-6 space-y-8 bg-[#090b0f] text-foreground animate-fadeIn scroll-smooth"
    >
      {/* ── Top Header & Integrated Search Bar ── */}
      <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 border-b border-white/[0.06] pb-4">
        <div>
          <h1 className="text-3xl font-extrabold tracking-tight text-white">Applications</h1>
          <p className="text-xs text-foreground-muted pt-1">
            Discover, preview, install and manage software for your system
          </p>
        </div>

        {/* Search Field */}
        <div className="relative w-full md:w-96">
          <Search
            size={16}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-foreground-muted pointer-events-none"
          />
          <input
            type="text"
            placeholder="Search apps, games, tools..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-white/[0.04] hover:bg-white/[0.07] focus:bg-white/[0.08] border border-white/[0.08] focus:border-blue-500/50 rounded-xl pl-10 pr-9 py-2 text-xs placeholder:text-foreground-muted/60 text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 transition-all"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery("")}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-foreground-muted hover:text-white cursor-pointer"
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
                  ? "bg-blue-600/20 text-blue-400 border border-blue-500/30 shadow-sm"
                  : "text-foreground-muted hover:text-white hover:bg-white/[0.03] border border-transparent"
              }`}
            >
              {cat}
            </button>
          );
        })}
      </div>

      {/* ── Main Apps Content Area ── */}
      {loading ? (
        <div className="flex flex-col items-center justify-center py-24 space-y-3">
          <RefreshCw size={24} className="animate-spin text-blue-500" />
          <p className="text-xs text-foreground-muted font-medium">Loading applications catalog...</p>
        </div>
      ) : filteredPackages.length === 0 ? (
        <div className="text-center py-24 space-y-3">
          <Layers size={36} className="mx-auto text-foreground-muted/30" />
          <h3 className="text-sm font-semibold text-white">No applications found</h3>
          <p className="text-xs text-foreground-muted max-w-sm mx-auto">
            Try adjusting your search query or switching to another category.
          </p>
        </div>
      ) : (
        <div className="space-y-10">
          {/* ── Cinematic Featured Hero Showcase (Shown when browsing All) ── */}
          {!searchQuery && selectedCategory === "All" && featuredHeroApp && (
            <div
              onClick={() => openAppDetail(featuredHeroApp)}
              className="relative w-full rounded-2xl md:rounded-3xl p-8 flex flex-col md:flex-row items-center justify-between gap-8 bg-surface-elevated/40 border border-white/[0.08] shadow-2xl hover:border-white/[0.16] transition-all duration-300 cursor-pointer group overflow-hidden"
            >
              {/* Very subtle atmospheric radial glow behind artwork */}
              <div
                className="absolute right-0 top-0 w-96 h-96 rounded-full blur-3xl pointer-events-none opacity-10"
                style={{ background: "radial-gradient(circle, #3B82F6 20%, #6366F1 50%, transparent 70%)" }}
              />

              <div className="space-y-3 z-10 max-w-xl">
                <span className="px-2.5 py-0.5 rounded-full text-[11px] font-bold tracking-wider uppercase bg-blue-600/20 text-blue-400 border border-blue-500/30">
                  Featured application
                </span>
                <h2 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-white group-hover:text-blue-300 transition-colors">
                  Firefox
                </h2>
                <div className="text-xs font-semibold text-foreground-muted flex items-center gap-1.5">
                  <span>Mozilla</span>
                  <CheckCircle2 size={12} className="text-blue-400" />
                </div>
                <p className="text-sm text-foreground/80 leading-relaxed max-w-lg">
                  Fast, private, and extensible web browser. Explore the web with unmatched privacy protections and open standards.
                </p>
                <div className="pt-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      openAppDetail(featuredHeroApp);
                    }}
                    className="inline-flex items-center gap-2 px-6 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white font-semibold text-xs shadow-md transition-all cursor-pointer"
                  >
                    <span>View application</span>
                    <ArrowRight size={14} />
                  </button>
                </div>
              </div>

              {/* 112px Authentic Firefox Icon Container */}
              <div className="shrink-0 p-3 rounded-3xl bg-white/[0.02] border border-white/[0.06] group-hover:scale-105 transition-transform duration-300">
                <AppIcon appId="firefox" size="2xl" />
              </div>
            </div>
          )}

          {/* ── Popular Applications Section ── */}
          {(!searchQuery && (selectedCategory === "All" || selectedCategory === "Popular")) && popularPicks.length > 0 && (
            <div className="space-y-4">
              <div className="flex items-center justify-between border-b border-white/[0.04] pb-2">
                <h2 className="text-lg font-bold tracking-tight text-white">Popular apps</h2>
                <span className="text-xs text-blue-400 hover:underline cursor-pointer">See all →</span>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-8 gap-4">
                {popularPicks.map((app) => {
                  const meta = resolveAppMetadata(app.id, app.title);
                  const isInstalled = checkInstalled(app);
                  return (
                    <div
                      key={app.id}
                      onClick={() => openAppDetail(app)}
                      className="group flex flex-col items-center text-center p-4 rounded-2xl bg-white/[0.02] hover:bg-white/[0.06] border border-transparent hover:border-white/[0.08] transition-all duration-150 ease-out cursor-pointer"
                    >
                      <div className="p-1 mb-3 rounded-2xl transition-transform group-hover:scale-105 duration-200">
                        <AppIcon appId={app.id} size="lg" />
                      </div>
                      <span className="text-xs font-bold text-white group-hover:text-blue-400 transition-colors truncate w-full">
                        {meta.displayName}
                      </span>
                      <span className="text-[11px] text-foreground-muted truncate w-full pt-0.5">
                        {meta.category}
                      </span>
                      {isInstalled && (
                        <span className="mt-2 text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25 font-semibold">
                          Installed
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* ── Recommended for You / Filtered Catalog Grid ── */}
          <div className="space-y-4">
            <div className="flex items-center justify-between border-b border-white/[0.04] pb-2">
              <h2 className="text-lg font-bold tracking-tight text-white">
                {searchQuery
                  ? `Search results (${filteredPackages.length})`
                  : selectedCategory === "All"
                  ? "Recommended for you"
                  : `${selectedCategory} (${filteredPackages.length})`}
              </h2>
            </div>

            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
              {(!searchQuery && selectedCategory === "All" && recommendedPicks.length > 0
                ? recommendedPicks
                : filteredPackages
              ).map((app) => {
                const meta = resolveAppMetadata(app.id, app.title);
                const isInstalled = checkInstalled(app);

                return (
                  <div
                    key={app.id}
                    onClick={() => openAppDetail(app)}
                    className="group flex flex-col items-center text-center p-4 rounded-2xl bg-white/[0.02] hover:bg-white/[0.06] border border-transparent hover:border-white/[0.08] transition-all duration-150 ease-out cursor-pointer"
                  >
                    <div className="p-1 mb-3 rounded-2xl transition-transform group-hover:scale-105 duration-200">
                      <AppIcon appId={app.id} size="lg" />
                    </div>
                    <span className="text-xs font-bold text-white group-hover:text-blue-400 transition-colors truncate w-full">
                      {meta.displayName}
                    </span>
                    <span className="text-[11px] text-foreground-muted truncate w-full pt-0.5">
                      {meta.category}
                    </span>
                    {isInstalled && (
                      <span className="mt-2 text-[10px] px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/25 font-semibold">
                        Installed
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* ── Browse by Category Matrix (Shown when on All view) ── */}
          {!searchQuery && selectedCategory === "All" && (
            <div className="space-y-4 pt-4 border-t border-white/[0.06]">
              <h2 className="text-lg font-bold tracking-tight text-white">Browse by category</h2>
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
                    className="flex items-center gap-3 p-3.5 rounded-xl bg-white/[0.02] hover:bg-white/[0.06] border border-white/[0.06] hover:border-blue-500/40 text-left transition-all cursor-pointer group"
                  >
                    <div className="p-2 rounded-lg bg-blue-600/10 text-blue-400 group-hover:scale-110 transition-transform">
                      <CatIcon size={16} />
                    </div>
                    <span className="text-xs font-semibold text-white group-hover:text-blue-400 transition-colors">
                      {name}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
