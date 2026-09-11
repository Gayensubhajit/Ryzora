/**
 * AppsView — Phase 24.1
 *
 * Full Arch Linux Repository Catalog & Application Storefront.
 * - Sourced from configured Pacman repositories (core, extra, multilib, garuda)
 * - Separate views for Applications (~1,400 desktop apps) vs All Packages (~15,500+ packages)
 * - Authoritative ALPM installed state from /var/lib/pacman/local/
 * - Backend-driven query for "Installed" ({ is_application_only: true, is_installed_only: true })
 * - Responsive debounced search (200ms)
 * - True infinite pagination (60 items/page) using VirtualizedAppGrid
 * - Isolated TransactionManager with fast non-blocking ALPM state refresh on completion
 */

import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
import {
  Search,
  CheckCircle2,
  RefreshCw,
  Layers,
  ArrowRight,
  Code2,
  Globe,
  Tv,
  ImageIcon,
  Gamepad2,
  Terminal,
  Sparkles,
  Package,
  Laptop,
} from "lucide-react";
import { pacmanAppProvider } from "../providers/index.ts";
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
import { TransactionDrawer } from "../components/apps/TransactionDrawer.tsx";
import { catalogService, CatalogStatus } from "../services/catalogService.ts";
import { VirtualizedAppGrid } from "../components/apps/VirtualizedAppGrid.tsx";
import { prefetchIconsForPage } from "../services/iconCache.ts";

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
  // Mode & Filter State
  const [viewMode, setViewMode] = useState<"applications" | "packages">("applications");
  const [selectedCategory, setSelectedCategory] = useState<CategoryFilter>("All");
  const [searchQuery, setSearchQuery] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [installedSourceFilter, setInstalledSourceFilter] = useState<"All" | "pacman" | "aur" | "flathub">("All");

  // Storefront & Catalogue Data
  const [storefrontPackages, setStorefrontPackages] = useState<PackageItem[]>([]);
  const [catalogueItems, setCatalogueItems] = useState<PackageItem[]>([]);
  const [totalCatalogueCount, setTotalCatalogueCount] = useState<number>(0);
  const [page, setPage] = useState<number>(0);
  const [loading, setLoading] = useState<boolean>(true);
  const [loadingMore, setLoadingMore] = useState<boolean>(false);

  // Status & Navigation State
  const [catalogStatus, setCatalogStatus] = useState<CatalogStatus | null>(null);
  const [refreshingCatalog, setRefreshingCatalog] = useState<boolean>(false);
  const [selectedApp, setSelectedApp] = useState<PackageItem | null>(null);
  const [appHistory, setAppHistory] = useState<PackageItem[]>([]);

  // Scroll preservation
  const gridScrollRef = useRef<number>(0);
  const gridRef = useRef<HTMLDivElement | null>(null);

  // Request epoch for cancellation — prevents stale responses overwriting latest view
  const requestEpoch = useRef<number>(0);

  // Debounce search input (200ms)
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery.trim());
    }, 200);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  // Initial catalog status & subscriber
  useEffect(() => {
    catalogService.getStatus().then(setCatalogStatus).catch(() => {});
    const unsub = catalogService.subscribe((status) => {
      setCatalogStatus(status);
    });
    return unsub;
  }, []);

  const isStorefrontHome = viewMode === "applications" && selectedCategory === "Popular" && !debouncedSearch;

  // Load storefront featured apps (Hero, Popular, Recommended)
  const loadStorefrontApps = useCallback(async () => {
    try {
      const [popRes, allRes] = await Promise.all([
        catalogService.getItems({
          is_application_only: true,
          category: "popular",
          page_size: 60,
        }),
        catalogService.getItems({
          is_application_only: true,
          page: 0,
          page_size: 60,
        }),
      ]);
      const combined = [...(popRes.items || []), ...(allRes.items || [])];
      const uniqueItems = Array.from(new Map(combined.map((i) => [i.id, i])).values());
      const mapped = uniqueItems.map((i) => catalogService.catalogItemToPackageItem(i));
      setStorefrontPackages(mapped);
      // Pre-fetch icons for storefront page
      prefetchIconsForPage(uniqueItems.map((i) => ({ id: i.id, icon_name: i.icon_name, icon_path: i.icon_path })));
    } catch (err) {
      console.warn("[AppsView] Failed to load storefront apps:", err);
    }
  }, []);

  // Load dedicated catalogue (Category, Search, Installed, or All Packages)
  const loadCatalogue = useCallback(async () => {
    // Bump epoch — any in-flight request with an older epoch will be discarded
    const epoch = ++requestEpoch.current;
    setLoading(true);
    setPage(0);
    if (gridRef.current) {
      gridRef.current.scrollTop = 0;
    }
    try {
      const isInstalledOnly = selectedCategory === "Installed";
      let categoryFilter: string | undefined = undefined;
      if (!isInstalledOnly && selectedCategory !== "All") {
        categoryFilter = selectedCategory === "Popular" ? "popular" : selectedCategory;
      }

      const res = await catalogService.getItems({
        is_application_only: viewMode === "applications",
        is_installed_only: isInstalledOnly,
        category: categoryFilter,
        search_query: debouncedSearch || undefined,
        page: 0,
        page_size: 60,
      });

      // Discard stale response (user changed category/search while this was in-flight)
      if (epoch !== requestEpoch.current) return;

      const mapped = (res.items || []).map((i) => catalogService.catalogItemToPackageItem(i));
      setCatalogueItems(mapped);
      setTotalCatalogueCount(res.total_count);
      // Pre-fetch icons for the loaded page in one batch IPC call
      prefetchIconsForPage(res.items.map((i) => ({ id: i.id, icon_name: i.icon_name, icon_path: i.icon_path })));
    } catch (err) {
      if (epoch !== requestEpoch.current) return;
      console.error("[AppsView] Failed to query catalog items:", err);
      setCatalogueItems([]);
      setTotalCatalogueCount(0);
    } finally {
      if (epoch === requestEpoch.current) setLoading(false);
    }
  }, [viewMode, selectedCategory, debouncedSearch]);

  // Main data loader effect
  useEffect(() => {
    if (isStorefrontHome) {
      setLoading(true);
      loadStorefrontApps().finally(() => setLoading(false));
    } else {
      loadCatalogue();
    }
  }, [isStorefrontHome, loadStorefrontApps, loadCatalogue]);

  // Load subsequent page when scrolling (infinite pagination)
  const loadMore = useCallback(async () => {
    if (loadingMore || loading) return;
    if (catalogueItems.length >= totalCatalogueCount) return;

    const nextPage = page + 1;
    setLoadingMore(true);
    try {
      const isInstalledOnly = selectedCategory === "Installed";
      let categoryFilter: string | undefined = undefined;
      if (!isInstalledOnly && selectedCategory !== "All") {
        categoryFilter = selectedCategory === "Popular" ? "popular" : selectedCategory;
      }

      const res = await catalogService.getItems({
        is_application_only: viewMode === "applications",
        is_installed_only: isInstalledOnly,
        category: categoryFilter,
        search_query: debouncedSearch || undefined,
        page: nextPage,
        page_size: 60,
      });

      if (res.items && res.items.length > 0) {
        const mapped = res.items.map((i) => catalogService.catalogItemToPackageItem(i));
        setCatalogueItems((prev) => [...prev, ...mapped]);
        setPage(nextPage);
        // Pre-fetch icons for new page in one batch IPC call
        prefetchIconsForPage(res.items.map((i) => ({ id: i.id, icon_name: i.icon_name, icon_path: i.icon_path })));
      }
    } catch (err) {
      console.error("[AppsView] Failed to load more catalog items:", err);
    } finally {
      setLoadingMore(false);
    }
  }, [
    loadingMore,
    loading,
    catalogueItems.length,
    totalCatalogueCount,
    page,
    viewMode,
    selectedCategory,
    debouncedSearch,
  ]);

  // Fast ALPM state refresh after transaction completes
  const handleRefreshInstalled = useCallback(async () => {
    try {
      const status = await catalogService.refreshInstalledState();
      setCatalogStatus(status);

      if (selectedCategory === "Installed") {
        // In Installed category: re-query current page window to reflect additions/removals
        const res = await catalogService.getItems({
          is_application_only: viewMode === "applications",
          is_installed_only: true,
          search_query: debouncedSearch || undefined,
          page: 0,
          page_size: Math.max(60, (page + 1) * 60),
        });
        const mapped = (res.items || []).map((i) => catalogService.catalogItemToPackageItem(i));
        setCatalogueItems(mapped);
        setTotalCatalogueCount(res.total_count);
      } else {
        // In other views: update is_installed in-place without rebuilding or layout shift
        const updateInPlace = (list: PackageItem[]) =>
          list.map((pkg) => {
            const meta = pacmanAppProvider.getMeta(pkg);
            if (meta) {
              (pkg as any).is_installed = meta.isInstalled;
              (pkg as any).installed_version = meta.installedVersion;
            }
            return pkg;
          });

        setCatalogueItems((prev) => updateInPlace(prev));
        setStorefrontPackages((prev) => updateInPlace(prev));
      }
    } catch (err) {
      console.warn("[AppsView] Failed to refresh installed status:", err);
    }
  }, [selectedCategory, viewMode, debouncedSearch, page]);

  // Full repository database refresh (manual button)
  const handleRefreshCatalog = async () => {
    setRefreshingCatalog(true);
    try {
      const status = await catalogService.refreshCatalog();
      setCatalogStatus(status);
      if (isStorefrontHome) {
        await loadStorefrontApps();
      } else {
        await loadCatalogue();
      }
    } catch (err) {
      console.error("Failed to refresh catalog:", err);
    } finally {
      setRefreshingCatalog(false);
    }
  };

  // Check URL for direct deep-link app target
  useEffect(() => {
    try {
      const targetAppId = new URLSearchParams(window.location.search).get("app");
      if (targetAppId) {
        const pool = [...storefrontPackages, ...catalogueItems];
        const found = pool.find(
          (p) =>
            p.id.toLowerCase() === targetAppId.toLowerCase() ||
            resolveCanonicalAppId(p.id) === targetAppId.toLowerCase()
        );
        if (found) {
          setSelectedApp(found);
        } else {
          catalogService.getItemDetails(targetAppId).then((item) => {
            if (item) {
              setSelectedApp(catalogService.catalogItemToPackageItem(item));
            }
          });
        }
      }
    } catch {}
  }, [storefrontPackages, catalogueItems]);

  const openAppDetail = useCallback((app: PackageItem) => {
    gridScrollRef.current = gridRef.current?.scrollTop ?? 0;
    setAppHistory([]);
    setSelectedApp(app);
    window.scrollTo({ top: 0, behavior: "smooth" });
  }, []);

  const handleSelectRelated = useCallback(
    (relatedId: string) => {
      const pool = [...storefrontPackages, ...catalogueItems];
      const found = pool.find(
        (p) =>
          p.id.toLowerCase() === relatedId.toLowerCase() ||
          resolveCanonicalAppId(p.id) === relatedId.toLowerCase()
      );
      if (found && selectedApp) {
        setAppHistory((prev) => [...prev, selectedApp]);
        setSelectedApp(found);
        window.scrollTo({ top: 0, behavior: "smooth" });
      } else {
        catalogService.getItemDetails(relatedId).then((item) => {
          if (item && selectedApp) {
            setAppHistory((prev) => [...prev, selectedApp]);
            setSelectedApp(catalogService.catalogItemToPackageItem(item));
            window.scrollTo({ top: 0, behavior: "smooth" });
          }
        });
      }
    },
    [storefrontPackages, catalogueItems, selectedApp]
  );

  const handleBack = useCallback(() => {
    if (appHistory.length > 0) {
      const prevApp = appHistory[appHistory.length - 1];
      setAppHistory((prev) => prev.slice(0, prev.length - 1));
      setSelectedApp(prevApp);
      window.scrollTo({ top: 0, behavior: "smooth" });
    } else {
      setSelectedApp(null);
      try {
        const url = new URL(window.location.href);
        url.searchParams.delete("app");
        window.history.replaceState({}, "", url.toString());
      } catch {}
      requestAnimationFrame(() => {
        if (gridRef.current) {
          gridRef.current.scrollTop = gridScrollRef.current;
        }
      });
    }
  }, [appHistory]);

  const checkInstalled = useCallback((pkg: PackageItem): boolean => {
    if ((pkg as any)?.is_installed !== undefined) return Boolean((pkg as any).is_installed);
    if (pkg.tags?.includes("installed")) return true;
    const meta = pacmanAppProvider.getMeta(pkg);
    return meta?.isInstalled ?? false;
  }, []);

  // Filter catalogue items based on installedSourceFilter
  const displayedCatalogueItems = useMemo(() => {
    if (selectedCategory !== "Installed" || installedSourceFilter === "All") {
      return catalogueItems;
    }
    return catalogueItems.filter((item) => {
      const repo = ((item as any).repository_id || (item as any).repository || "").toLowerCase();
      const metaSource = ((item as any).metadata_source || "").toLowerCase();

      if (installedSourceFilter === "flathub") {
        return repo === "flathub" || metaSource === "flatpak";
      }
      if (installedSourceFilter === "aur") {
        return repo === "aur" || metaSource === "aur";
      }
      if (installedSourceFilter === "pacman") {
        return repo !== "flathub" && repo !== "aur" && metaSource !== "flatpak" && metaSource !== "aur";
      }
      return true;
    });
  }, [catalogueItems, selectedCategory, installedSourceFilter]);

  // Storefront Home Curations
  const deduplicatedPackages = useMemo(() => {
    return deduplicateAppPackages(storefrontPackages);
  }, [storefrontPackages]);

  const featuredHeroApp = useMemo(() => {
    return (
      deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === "firefox") ||
      deduplicatedPackages[0]
    );
  }, [deduplicatedPackages]);

  const popularPicks = useMemo(() => {
    const popularMap = new Map<string, PackageItem>();
    for (const popId of POPULAR_CANONICAL_IDS) {
      const match = deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === popId);
      if (match && !popularMap.has(popId)) {
        popularMap.set(popId, match);
      }
    }
    if (popularMap.size < 8) {
      for (const p of deduplicatedPackages) {
        const cid = resolveCanonicalAppId(p.id);
        const meta = resolveAppMetadata(p.id, p.title);
        if (meta.isCuratedApp && !popularMap.has(cid) && !RECOMMENDED_CANONICAL_IDS.includes(cid)) {
          popularMap.set(cid, p);
          if (popularMap.size >= 8) break;
        }
      }
    }
    return Array.from(popularMap.values());
  }, [deduplicatedPackages]);

  const recommendedPicks = useMemo(() => {
    const popularCanonicalSet = new Set(popularPicks.map((p) => resolveCanonicalAppId(p.id)));
    if (featuredHeroApp) {
      popularCanonicalSet.add(resolveCanonicalAppId(featuredHeroApp.id));
    }
    const recommendedMap = new Map<string, PackageItem>();
    for (const recId of RECOMMENDED_CANONICAL_IDS) {
      const match = deduplicatedPackages.find((p) => resolveCanonicalAppId(p.id) === recId);
      if (match && !popularCanonicalSet.has(recId) && !recommendedMap.has(recId)) {
        recommendedMap.set(recId, match);
      }
    }
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

  // Render App Detail Page if selected
  if (selectedApp) {
    const parentApp = appHistory.length > 0 ? appHistory[appHistory.length - 1] : null;
    const parentMeta = parentApp ? resolveAppMetadata(parentApp.id, parentApp.title) : null;
    const backLabel = parentApp
      ? `Back to ${parentMeta?.displayName || parentApp.title || "Previous App"}`
      : "Back to Applications";

    return (
      <>
        <AppDetailPage
          app={selectedApp}
          backLabel={backLabel}
          onBack={handleBack}
          onStatusChanged={handleRefreshInstalled}
          onSelectRelated={handleSelectRelated}
        />
        <TransactionDrawer onRefreshApps={handleRefreshInstalled} />
      </>
    );
  }

  return (
    <>
      <div
        ref={gridRef}
        className="flex flex-col flex-1 h-full w-full overflow-y-auto px-8 py-6 space-y-8 bg-[var(--rz-bg)] text-[var(--rz-text)] animate-fadeIn scroll-smooth pb-20"
      >
        {/* ── Top Header & Integrated Search Bar ── */}
        <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 border-b border-[var(--rz-border-subtle)] pb-4">
          <div className="space-y-1.5">
            <div className="flex flex-wrap items-center gap-3">
              <h1 className="text-3xl font-extrabold tracking-tight text-[var(--rz-text)]">
                {viewMode === "applications" ? "Applications" : "Arch Repository Packages"}
              </h1>

              {/* Mode Switcher Pill */}
              <div className="flex items-center p-0.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-xs font-semibold">
                <button
                  type="button"
                  onClick={() => {
                    setViewMode("applications");
                    setSelectedCategory("All");
                  }}
                  className={`flex items-center gap-1.5 px-3 py-1 rounded-lg transition-all cursor-pointer ${
                    viewMode === "applications"
                      ? "bg-blue-600 text-white shadow-xs"
                      : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  <Laptop size={13} />
                  <span>
                    Applications (
                    {catalogStatus?.total_applications && catalogStatus.total_applications > 0
                      ? catalogStatus.total_applications.toLocaleString()
                      : "~1,400"}
                    )
                  </span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setViewMode("packages");
                    setSelectedCategory("All");
                  }}
                  className={`flex items-center gap-1.5 px-3 py-1 rounded-lg transition-all cursor-pointer ${
                    viewMode === "packages"
                      ? "bg-blue-600 text-white shadow-xs"
                      : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  <Package size={13} />
                  <span>
                    All Packages (
                    {catalogStatus?.total_packages && catalogStatus.total_packages > 0
                      ? catalogStatus.total_packages.toLocaleString()
                      : "15,539"}
                    )
                  </span>
                </button>
              </div>
            </div>

            {/* Repository Status & Refresh */}
            <div className="flex flex-wrap items-center gap-2.5 text-xs text-[var(--rz-text-muted)]">
              <span>
                Configured repositories:{" "}
                <strong className="text-[var(--rz-text-secondary)] font-medium">
                  {catalogStatus?.enabled_repositories?.join(", ") || "core, extra, multilib"}
                </strong>
              </span>
              <span>•</span>
              <span>
                Last refreshed:{" "}
                {catalogStatus?.last_refreshed
                  ? new Date(catalogStatus.last_refreshed * 1000).toLocaleTimeString([], {
                      hour: "2-digit",
                      minute: "2-digit",
                    })
                  : "Active"}
              </span>
              <span>•</span>
              <button
                type="button"
                onClick={handleRefreshCatalog}
                disabled={refreshingCatalog || catalogStatus?.is_refreshing}
                className="inline-flex items-center gap-1 text-blue-600 dark:text-blue-400 hover:underline cursor-pointer disabled:opacity-50"
              >
                <RefreshCw
                  size={11}
                  className={refreshingCatalog || catalogStatus?.is_refreshing ? "animate-spin" : ""}
                />
                <span>
                  {refreshingCatalog || catalogStatus?.is_refreshing ? "Refreshing..." : "Refresh Repositories"}
                </span>
              </button>
            </div>
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
            let label = cat as string;
            if (cat === "Installed" && catalogStatus?.total_installed_applications !== undefined && catalogStatus.total_installed_applications > 0) {
              label = `Installed (${catalogStatus.total_installed_applications})`;
            }
            return (
              <button
                key={cat}
                onClick={() => {
                  setSelectedCategory(cat);
                  setInstalledSourceFilter("All");
                }}
                className={`px-3.5 py-1.5 rounded-xl text-xs font-semibold transition-all shrink-0 cursor-pointer ${
                  isSelected
                    ? "bg-blue-600 text-white shadow-xs"
                    : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-transparent"
                }`}
              >
                {label}
              </button>
            );
          })}
        </div>

        {/* ── Installed Multi-Source Sub-Filter (Phase 25) ── */}
        {selectedCategory === "Installed" && (
          <div className="flex items-center gap-2 pt-0.5">
            <span className="text-xs text-[var(--rz-text-muted)] font-medium">Source filter:</span>
            <div className="flex items-center gap-1 p-0.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-xs font-medium">
              {(["All", "pacman", "aur", "flathub"] as const).map((source) => {
                const isCurrent = installedSourceFilter === source;
                return (
                  <button
                    key={source}
                    type="button"
                    onClick={() => setInstalledSourceFilter(source)}
                    className={`px-3 py-1 rounded-lg text-xs transition-all cursor-pointer ${
                      isCurrent
                        ? "bg-blue-600 text-white shadow-xs font-semibold"
                        : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)]"
                    }`}
                  >
                    {source === "All"
                      ? "All Sources"
                      : source === "pacman"
                      ? "Pacman"
                      : source === "aur"
                      ? "AUR"
                      : "Flathub"}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {isStorefrontHome ? (
          /* ═══════════════════════════════════════════════════════════════
             STOREFRONT HOME (Rendered on 'All' category when not searching)
             ═══════════════════════════════════════════════════════════════ */
          <div className="space-y-10">
            {/* ── Dynamic Hero Card ── */}
            {featuredHeroApp && (
              <div
                onClick={() => openAppDetail(featuredHeroApp)}
                className="relative overflow-hidden rounded-3xl bg-linear-to-r from-blue-600/10 via-[var(--rz-surface)] to-transparent border border-[var(--rz-border-subtle)] p-6 sm:p-8 flex flex-col md:flex-row items-center justify-between gap-6 cursor-pointer group hover:border-[var(--rz-border-strong)] transition-all duration-300 shadow-sm"
              >
                <div className="space-y-3 max-w-xl z-10">
                  <div className="flex items-center gap-2">
                    <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[11px] font-bold bg-blue-600 text-white tracking-wide uppercase">
                      <Sparkles size={11} />
                      Featured
                    </span>
                    <span className="text-xs text-[var(--rz-text-muted)] font-medium">
                      Arch Linux ({featuredHeroApp.repository_id || "extra"})
                    </span>
                  </div>
                  <h2 className="text-2xl sm:text-3xl font-extrabold tracking-tight text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
                    {featuredHeroApp.title}
                  </h2>
                  <p className="text-xs sm:text-sm text-[var(--rz-text-secondary)] line-clamp-2 leading-relaxed">
                    {featuredHeroApp.description}
                  </p>
                  <div className="pt-2">
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        openAppDetail(featuredHeroApp);
                      }}
                      className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-md hover:shadow-lg transition-all cursor-pointer"
                    >
                      <span>Explore</span>
                      <ArrowRight size={14} />
                    </button>
                  </div>
                </div>

                <div className="shrink-0 p-4 sm:p-5 rounded-3xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] group-hover:scale-105 transition-transform duration-300 shadow-sm z-10">
                  <AppIcon appId={featuredHeroApp.id} size="2xl" iconName={(featuredHeroApp as any).icon_name} iconPath={(featuredHeroApp as any).icon_path} />
                </div>
              </div>
            )}

            {/* ── Popular Applications Section ── */}
            {popularPicks.length > 0 && (
              <div className="space-y-4">
                <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-2">
                  <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Popular apps</h2>
                  <button
                    onClick={() => setSelectedCategory("All")}
                    className="text-xs text-blue-600 dark:text-blue-400 hover:underline cursor-pointer font-medium"
                  >
                    Browse all applications →
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

            {/* ── Recommended for You Section ── */}
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
             DEDICATED FILTERED CATALOGUE VIEW (Rendered for Category / Search / Installed / Packages)
             ═══════════════════════════════════════════════════════════════ */
          <div className="space-y-6">
            <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-3">
              <div>
                <h2 className="text-xl font-bold tracking-tight text-[var(--rz-text)]">
                  {debouncedSearch.trim()
                    ? `Search results for "${debouncedSearch.trim()}"`
                    : selectedCategory === "Installed"
                    ? viewMode === "applications"
                      ? "Installed applications"
                      : "Installed packages"
                    : selectedCategory === "All"
                    ? viewMode === "applications"
                      ? "All applications"
                      : "All packages"
                    : selectedCategory === "Popular"
                    ? "Popular applications"
                    : viewMode === "applications"
                    ? `${selectedCategory} applications`
                    : `${selectedCategory} packages`}
                </h2>
                <p className="text-xs text-[var(--rz-text-muted)] pt-0.5">
                  {totalCatalogueCount.toLocaleString()}{" "}
                  {viewMode === "applications"
                    ? totalCatalogueCount === 1
                      ? "application"
                      : "applications"
                    : totalCatalogueCount === 1
                    ? "package"
                    : "packages"}{" "}
                  available
                </p>
              </div>
            </div>

            {loading ? (
              /* Inline skeleton — only the grid area is blocked, tabs/search remain interactive */
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
                {Array.from({ length: 18 }).map((_, i) => (
                  <div
                    key={i}
                    className="flex flex-col items-center text-center p-4 rounded-2xl bg-[var(--rz-surface)]/30 border border-transparent animate-pulse"
                  >
                    <div className="w-[68px] h-[68px] rounded-2xl bg-[var(--rz-surface-elevated)] mb-3" />
                    <div className="h-3 w-16 rounded bg-[var(--rz-surface-elevated)] mb-1" />
                    <div className="h-2.5 w-12 rounded bg-[var(--rz-surface-elevated)]" />
                  </div>
                ))}
              </div>
            ) : catalogueItems.length === 0 ? (
              <div className="text-center py-20 space-y-3">
                <Layers size={36} className="mx-auto text-[var(--rz-text-muted)] opacity-60" />
                <h3 className="text-sm font-semibold text-[var(--rz-text)]">
                  {selectedCategory === "Installed" ? "No installed applications found" : "No packages found"}
                </h3>
                <p className="text-xs text-[var(--rz-text-muted)] max-w-sm mx-auto">
                  {debouncedSearch.trim()
                    ? `No results matched "${debouncedSearch}". Try adjusting your search query.`
                    : selectedCategory === "Installed"
                    ? "No installed applications matching the criteria were detected in /var/lib/pacman/local/."
                    : `No packages found in the ${selectedCategory} category.`}
                </p>
              </div>
            ) : (
              <VirtualizedAppGrid
                scrollContainerRef={gridRef}
                items={displayedCatalogueItems}
                hasMore={catalogueItems.length < totalCatalogueCount}
                isLoadingMore={loadingMore}
                onLoadMore={loadMore}
                renderItem={(app) => (
                  <AppCatalogueTile
                    key={app.id}
                    app={app}
                    isInstalled={checkInstalled(app)}
                    onClick={() => openAppDetail(app)}
                  />
                )}
              />
            )}
          </div>
        )}
      </div>
      <TransactionDrawer onRefreshApps={handleRefreshInstalled} />
    </>
  );
};

// ── Lightweight Catalogue App Tile ───────────────────────────────────────────
// Memoized tile with subtle source badges (Pacman / AUR / Flathub)
const AppCatalogueTile: React.FC<{
  app: PackageItem;
  isInstalled: boolean;
  onClick: () => void;
}> = React.memo(({ app, isInstalled, onClick }) => {
  const meta = resolveAppMetadata(app.id, app.title);
  const displayName = app.title || meta.displayName || app.id;
  const categoryName = (app as any).category || meta.category || "Application";
  const iconName: string | null = (app as any).icon_name ?? null;
  const iconPath: string | null = (app as any).icon_path ?? null;

  const repo = ((app as any).repository_id || (app as any).repository || "").toLowerCase();
  const metaSource = ((app as any).metadata_source || "").toLowerCase();
  const sourceLabel =
    repo === "flathub" || metaSource === "flatpak"
      ? "● Flathub"
      : repo === "aur" || metaSource === "aur"
      ? "● AUR"
      : `● Pacman · ${repo || "extra"}`;

  return (
    <div
      onClick={onClick}
      className="group flex flex-col items-center text-center p-4 rounded-2xl bg-[var(--rz-surface)]/30 hover:bg-[var(--rz-surface)] border border-transparent hover:border-[var(--rz-border-subtle)] transition-all duration-200 ease-out cursor-pointer hover:shadow-md hover:-translate-y-0.5"
    >
      <div className="mb-3 transition-transform group-hover:scale-105 duration-200 flex items-center justify-center">
        <AppIcon appId={app.id} size="lg" iconName={iconName} iconPath={iconPath} />
      </div>
      <span
        className="text-xs sm:text-[13px] font-bold text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate w-full"
        title={displayName}
      >
        {displayName}
      </span>
      <span className="text-[11px] text-[var(--rz-text-muted)] truncate w-full pt-0.5">
        {categoryName}
      </span>
      <span className="text-[10px] text-[var(--rz-text-muted)] truncate w-full pt-0.5 opacity-75 font-mono">
        {sourceLabel}
      </span>
      {isInstalled ? (
        <span className="mt-1.5 inline-flex items-center gap-1 text-[11px] font-medium text-emerald-600 dark:text-emerald-400">
          <CheckCircle2 size={12} />
          <span>Installed</span>
        </span>
      ) : (
        <span className="mt-1.5 h-4" />
      )}
    </div>
  );
}, (prev, next) =>
  prev.app.id === next.app.id &&
  prev.isInstalled === next.isInstalled &&
  (prev.app as any).icon_name === (next.app as any).icon_name &&
  (prev.app as any).icon_path === (next.app as any).icon_path &&
  (prev.app as any).repository === (next.app as any).repository
);
