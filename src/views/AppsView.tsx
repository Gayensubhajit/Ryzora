import React, { useState, useEffect, useMemo, useRef, useCallback } from "react";
import {
  Search,
  CheckCircle2,
  RefreshCw,
  ArrowRight,
  Flame,
  Layers,
  Code2,
  Globe,
  Tv,
  ImageIcon,
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
    // Restore scroll after re-render
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

  // Featured hero app (e.g. Firefox)
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

  // Recommended for you

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
    <div ref={gridRef} className="flex flex-col flex-1 h-full w-full overflow-y-auto px-8 py-6 space-y-8 bg-background text-foreground animate-fadeIn">
      {/* Top Header & Wide Search Bar */}
      <div className="flex flex-col md:flex-row items-center justify-between gap-4 border-b border-border/40 pb-4">
        <h1 className="text-2xl font-bold tracking-tight">Applications</h1>

        {/* Search Field */}
        <div className="relative w-full md:w-96">
          <Search
            size={17}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-foreground-muted pointer-events-none"
          />
          <input
            type="text"
            placeholder="Search apps, games, tools..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-surface-elevated/70 hover:bg-surface-elevated focus:bg-surface-elevated border border-border/70 focus:border-accent-primary/60 rounded-full pl-10 pr-9 py-2 text-xs placeholder:text-foreground-muted/60 focus:outline-none focus:ring-2 focus:ring-accent-primary/20 transition-all"
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery("")}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-foreground-muted hover:text-foreground cursor-pointer"
            >
              ×
            </button>
          )}
        </div>
      </div>

      {/* Featured Wide Banner Showcase (Shown when not actively searching) */}
      {!searchQuery && selectedCategory === "All" && featuredHeroApp && (
        <div
          onClick={() => openAppDetail(featuredHeroApp)}
          className="relative w-full rounded-3xl overflow-hidden p-8 flex flex-col md:flex-row items-center justify-between gap-6 bg-gradient-to-r from-orange-600/30 via-red-600/20 to-surface-elevated/60 border border-orange-500/30 shadow-lg hover:shadow-xl transition-all duration-300 hover:border-orange-500/50 cursor-pointer group"
        >
          <div className="space-y-3 z-10 max-w-xl">
            <span className="px-3 py-1 rounded-full text-[11px] font-bold tracking-wider uppercase bg-orange-500/20 text-orange-400 border border-orange-500/30">
              Featured Application
            </span>
            <h2 className="text-3xl font-extrabold tracking-tight text-white group-hover:text-orange-300 transition-colors">
              Firefox
            </h2>
            <p className="text-sm text-white/80 leading-relaxed">
              Fast, Private, and extensible web browser. Explore the web with unmatched privacy protections and open standards.
            </p>
            <div className="pt-2">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  openAppDetail(featuredHeroApp);
                }}
                className="inline-flex items-center gap-2 px-6 py-2.5 rounded-2xl bg-orange-500 text-white font-semibold text-xs shadow-md hover:bg-orange-600 transition-all cursor-pointer"
              >
                <span>View Application</span>
                <ArrowRight size={14} />
              </button>
            </div>
          </div>

          <div className="shrink-0 p-4 rounded-3xl bg-black/20 backdrop-blur-sm border border-white/10 group-hover:scale-105 transition-transform duration-300">
            <AppIcon appId="firefox" size="2xl" />
          </div>
        </div>
      )}

      {/* Minimal Category Navigation */}
      <div className="flex items-center gap-2 overflow-x-auto pb-1 scrollbar-none">
        {CATEGORIES.map((cat) => (
          <button
            key={cat}
            onClick={() => setSelectedCategory(cat)}
            className={`px-4 py-1.5 rounded-full text-xs font-medium transition-all shrink-0 cursor-pointer ${
              selectedCategory === cat
                ? "bg-foreground text-background shadow-sm"
                : "bg-surface-elevated/50 text-foreground-muted hover:text-foreground hover:bg-surface-elevated"
            }`}
          >
            {cat}
          </button>
        ))}
      </div>

      {/* Main Apps Content Area */}
      {loading ? (
        <div className="flex flex-col items-center justify-center py-20 space-y-3">
          <RefreshCw size={24} className="animate-spin text-accent-primary" />
          <p className="text-xs text-foreground-muted">Loading applications...</p>
        </div>
      ) : filteredPackages.length === 0 ? (
        <div className="text-center py-20 space-y-2">
          <Layers size={36} className="mx-auto text-foreground-muted/40" />
          <h3 className="text-sm font-semibold">No applications found</h3>
          <p className="text-xs text-foreground-muted max-w-xs mx-auto">
            {searchQuery
              ? `No results matching "${searchQuery}".`
              : "No applications available in this category."}
          </p>
        </div>
      ) : (
        <div className="space-y-10">
          {/* Section 1: Popular Apps Icon Row (Full Viewport Spread) */}
          {!searchQuery && selectedCategory === "All" && popularPicks.length > 0 && (
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-sm font-bold text-foreground">
                  <Flame size={16} className="text-amber-500" />
                  <span>Popular Apps</span>
                </div>
                <button
                  onClick={() => setSelectedCategory("Popular")}
                  className="text-xs text-accent-primary hover:underline font-medium cursor-pointer"
                >
                  See All →
                </button>
              </div>

              <div className="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 gap-4">
                {popularPicks.map((app) => {
                  const meta = resolveAppMetadata(app.id, app.title);
                  const isInstalled = checkInstalled(app);
                  return (
                    <button
                      key={`pop-${app.id}`}
                      onClick={() => openAppDetail(app)}
                      className="group flex flex-col items-center text-center p-3 rounded-2xl hover:bg-surface-elevated/60 transition-all duration-200 hover:-translate-y-1.5 cursor-pointer"
                    >
                      <div className="relative mb-2.5 flex items-center justify-center">
                        <AppIcon appId={app.id} size="lg" className="group-hover:scale-108 transition-transform" />
                        {isInstalled && (
                          <div className="absolute -top-1 -right-1 bg-emerald-500 text-white rounded-full p-0.5 border-2 border-background shadow-xs">
                            <CheckCircle2 size={11} />
                          </div>
                        )}
                      </div>
                      <span className="text-xs font-semibold text-foreground group-hover:text-accent-primary truncate max-w-full">
                        {meta.displayName}
                      </span>
                      <span className="text-[10px] text-foreground-muted truncate max-w-full mt-0.5">
                        {meta.category}
                      </span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Section 2: Recommended / All Grid (Spacious Full Width) */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-sm font-bold text-foreground">
                {searchQuery
                  ? `Search Results (${filteredPackages.length})`
                  : selectedCategory === "All"
                  ? "Recommended for You"
                  : `${selectedCategory} Applications (${filteredPackages.length})`}
              </h2>
            </div>

            <div className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-y-7 gap-x-4">
              {filteredPackages.map((app) => {
                const meta = resolveAppMetadata(app.id, app.title);
                const isInstalled = checkInstalled(app);

                return (
                  <button
                    key={app.id}
                    onClick={() => openAppDetail(app)}
                    className="group flex flex-col items-center text-center p-3 rounded-2xl hover:bg-surface-elevated/50 transition-all duration-200 hover:-translate-y-1.5 cursor-pointer focus:outline-none focus:ring-2 focus:ring-accent-primary/30"
                  >
                    {/* Undistorted Aspect-Square Dominant Icon */}
                    <div className="relative mb-2.5 flex items-center justify-center">
                      <AppIcon
                        appId={app.id}
                        size="lg"
                        className="group-hover:scale-108 transition-transform duration-200"
                      />
                      {isInstalled && (
                        <div className="absolute -top-1 -right-1 bg-emerald-500 text-white rounded-full p-0.5 border-2 border-background shadow-xs">
                          <CheckCircle2 size={11} />
                        </div>
                      )}
                    </div>

                    {/* App Name */}
                    <span className="text-xs font-medium text-foreground group-hover:text-accent-primary truncate max-w-full transition-colors leading-tight">
                      {meta.displayName}
                    </span>

                    {/* Subtle Category Pill */}
                    <span className="text-[10px] text-foreground-muted truncate max-w-full mt-0.5">
                      {meta.category}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Section 3: Categories Matrix */}
          {!searchQuery && selectedCategory === "All" && (
            <div className="space-y-4 pt-4 border-t border-border/40">
              <h2 className="text-sm font-bold text-foreground">Categories</h2>
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
                {[
                  { name: "Internet", icon: Globe, count: "Web browsers, chat, email" },
                  { name: "Development", icon: Code2, count: "IDEs, editors, git" },
                  { name: "Multimedia", icon: Tv, count: "Audio, video players, OBS" },
                  { name: "Graphics", icon: ImageIcon, count: "Design, 3D, image editors" },
                ].map((cat) => {
                  const Icon = cat.icon;
                  return (
                    <button
                      key={cat.name}
                      onClick={() => setSelectedCategory(cat.name as CategoryFilter)}
                      className="p-5 rounded-2xl bg-surface-elevated/40 hover:bg-surface-elevated/80 border border-border/60 hover:border-accent-primary/50 text-left transition-all duration-200 hover:-translate-y-1 cursor-pointer flex items-center gap-4"
                    >
                      <div className="p-3 rounded-xl bg-accent-primary/10 text-accent-primary">
                        <Icon size={22} />
                      </div>
                      <div className="min-w-0">
                        <h3 className="text-sm font-bold text-foreground truncate">{cat.name}</h3>
                        <p className="text-[11px] text-foreground-muted truncate">{cat.count}</p>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
