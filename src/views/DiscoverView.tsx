import React, { useState, useMemo } from "react";
import { ArrowUpDown } from "lucide-react";
import { useApp } from "../context/AppContext";
import { HeroBanner } from "../components/HeroBanner";
import { PackageCard } from "../components/PackageCard";
import { PackageType } from "../types";

type SortOption = "relevance" | "rating" | "downloads" | "name" | "newest";
type StatusFilterOption = "all" | "compatible" | "featured" | "trending" | "installed" | "update_available" | "verified";

function compareSemver(v1: string, v2: string): number {
  const parse = (v: string) =>
    v
      .replace(/^v/, "")
      .split("-")[0]
      .split(".")
      .map((n) => parseInt(n, 10) || 0);
  const p1 = parse(v1);
  const p2 = parse(v2);
  for (let i = 0; i < Math.max(p1.length, p2.length); i++) {
    const num1 = p1[i] || 0;
    const num2 = p2[i] || 0;
    if (num1 > num2) return 1;
    if (num1 < num2) return -1;
  }
  return 0;
}

export const DiscoverView: React.FC = () => {
  const {
    packages,
    searchQuery,
    desktopFilter,
    systemInfo,
    installedPackages,
    repositories,
    checkCompatibility,
  } = useApp();

  const [typeFilter, setTypeFilter] = useState<PackageType | "all">("all");
  const [statusFilter, setStatusFilter] = useState<StatusFilterOption>("all");
  const [repoFilter, setRepoFilter] = useState<string>("all");
  const [sortBy, setSortBy] = useState<SortOption>("relevance");

  const filteredAndSortedPackages = useMemo(() => {
    let result = packages.filter((pkg) => {
      // 1. Search Query
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const matches =
          pkg.title.toLowerCase().includes(q) ||
          pkg.subtitle.toLowerCase().includes(q) ||
          pkg.description.toLowerCase().includes(q) ||
          pkg.author.name.toLowerCase().includes(q) ||
          pkg.category.toLowerCase().includes(q) ||
          pkg.package_type.toLowerCase().includes(q) ||
          pkg.tags.some((t) => t.toLowerCase().includes(q));

        if (!matches) return false;
      }

      // 2. Desktop Filter
      if (desktopFilter !== "all") {
        const matchesDesktop =
          pkg.supported_desktops.includes("universal") ||
          pkg.supported_desktops.includes(desktopFilter);
        if (!matchesDesktop) return false;
      }

      // 3. Package Type Filter
      if (typeFilter !== "all" && pkg.package_type !== typeFilter) {
        return false;
      }

      // 4. Repository Filter
      if (repoFilter !== "all" && pkg.repository_id !== repoFilter) {
        return false;
      }

      // 5. Status Filter
      const installedRecord = installedPackages.find((p) => p.package_id === pkg.id);
      const isInstalled = !!installedRecord;
      const isUpdate =
        isInstalled && installedRecord
          ? compareSemver(pkg.version, installedRecord.version) > 0
          : false;
      const compat = checkCompatibility(pkg);

      if (statusFilter === "compatible" && compat.level !== "Compatible") {
        return false;
      }
      if (statusFilter === "featured" && !pkg.featured) {
        return false;
      }
      if (statusFilter === "trending" && !pkg.trending) {
        return false;
      }
      if (statusFilter === "installed" && !isInstalled) {
        return false;
      }
      if (statusFilter === "update_available" && !isUpdate) {
        return false;
      }
      if (
        statusFilter === "verified" &&
        pkg.safety_audit?.rating !== "verified" &&
        pkg.integrity_status !== "verified"
      ) {
        return false;
      }

      return true;
    });

    // Sort
    result = [...result].sort((a, b) => {
      if (sortBy === "rating") {
        return b.rating - a.rating;
      }
      if (sortBy === "downloads") {
        return b.downloads - a.downloads;
      }
      if (sortBy === "name") {
        return a.title.localeCompare(b.title);
      }
      if (sortBy === "newest") {
        return compareSemver(b.version, a.version);
      }
      // "relevance": if search query, sort by title match precedence
      if (searchQuery) {
        const q = searchQuery.toLowerCase();
        const aTitle = a.title.toLowerCase().includes(q) ? 1 : 0;
        const bTitle = b.title.toLowerCase().includes(q) ? 1 : 0;
        if (aTitle !== bTitle) return bTitle - aTitle;
      }
      return (b.downloads || 0) - (a.downloads || 0);
    });

    return result;
  }, [
    packages,
    searchQuery,
    desktopFilter,
    typeFilter,
    statusFilter,
    repoFilter,
    sortBy,
    installedPackages,
    checkCompatibility,
  ]);

  const featuredRice = packages.find((p) => p.featured) || packages[0];
  const currentWm = systemInfo?.window_manager.toLowerCase() || "hyprland";
  const compatiblePackages = packages.filter(
    (p) =>
      p.supported_desktops.includes("universal") ||
      p.supported_desktops.some((d) => d === currentWm || currentWm.includes(d))
  );

  const isFiltering =
    typeFilter !== "all" ||
    statusFilter !== "all" ||
    repoFilter !== "all" ||
    sortBy !== "relevance" ||
    searchQuery !== "";

  return (
    <div className="space-y-6 pb-10">
      {/* Featured Card (When not searching or active filter) */}
      {!isFiltering && desktopFilter === "all" && featuredRice && (
        <HeroBanner featuredPackage={featuredRice} />
      )}

      {/* Compatible with your setup (When not searching or active filter) */}
      {!isFiltering && desktopFilter === "all" && compatiblePackages.length > 0 && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <h2 className="text-sm font-bold text-[var(--text-primary)]">
              Compatible with your setup ({systemInfo?.window_manager || "Desktop"})
            </h2>
            <span className="text-xs text-[var(--text-faint)]">
              {compatiblePackages.length} available
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {compatiblePackages.slice(0, 3).map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        </div>
      )}

      {/* Main Catalog Header & Filter Bar */}
      <div className="space-y-3 pt-2">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-[var(--border-subtle)] pb-3">
          <div className="flex items-center gap-2">
            <h2 className="text-sm font-bold text-[var(--text-primary)]">
              {searchQuery ? `Results for "${searchQuery}"` : "Community Catalog"}
            </h2>
            <span className="text-xs text-[var(--text-faint)] font-mono">
              ({filteredAndSortedPackages.length} packages)
            </span>
          </div>

          {/* Filter & Sort Controls */}
          <div className="flex flex-wrap items-center gap-2 text-xs">
            {/* Type Selector */}
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value as any)}
              className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] focus:border-[var(--border-strong)] outline-none text-[11px]"
            >
              <option value="all">All Types</option>
              <option value="rice">Rices</option>
              <option value="theme">Themes</option>
              <option value="config">Configs</option>
              <option value="module">Modules</option>
              <option value="suite">Suites</option>
            </select>

            {/* Status Selector */}
            <select
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as any)}
              className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] focus:border-[var(--border-strong)] outline-none text-[11px]"
            >
              <option value="all">All Status</option>
              <option value="compatible">Compatible Only</option>
              <option value="verified">Verified Only</option>
              <option value="installed">Installed</option>
              <option value="update_available">Update Available</option>
              <option value="featured">Featured</option>
              <option value="trending">Trending</option>
            </select>

            {/* Repository Selector */}
            {repositories.length > 1 && (
              <select
                value={repoFilter}
                onChange={(e) => setRepoFilter(e.target.value)}
                className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] focus:border-[var(--border-strong)] outline-none text-[11px]"
              >
                <option value="all">All Repositories</option>
                {repositories.map((r) => (
                  <option key={r.id} value={r.id}>
                    {r.name}
                  </option>
                ))}
              </select>
            )}

            {/* Sort Selector */}
            <div className="flex items-center gap-1.5 pl-1 border-l border-[var(--border-subtle)]">
              <ArrowUpDown className="w-3.5 h-3.5 text-[var(--text-faint)]" />
              <select
                value={sortBy}
                onChange={(e) => setSortBy(e.target.value as any)}
                className="px-2 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--text-muted)] focus:text-[var(--text-primary)] focus:border-[var(--border-strong)] outline-none text-[11px]"
              >
                <option value="relevance">Relevance</option>
                <option value="rating">Highest Rated</option>
                <option value="downloads">Most Downloads</option>
                <option value="name">Name (A-Z)</option>
                <option value="newest">Newest Version</option>
              </select>
            </div>
          </div>
        </div>

        {/* Package Grid */}
        {filteredAndSortedPackages.length === 0 ? (
          <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)] space-y-2">
            <div>No packages match your search or filter criteria.</div>
            {isFiltering && (
              <button
                onClick={() => {
                  setTypeFilter("all");
                  setStatusFilter("all");
                  setRepoFilter("all");
                  setSortBy("relevance");
                }}
                className="px-3 py-1 text-[11px] rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--accent-text)] hover:border-[var(--accent)]"
              >
                Reset Filters
              </button>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredAndSortedPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
