import React from "react";
import { useApp } from "../context/AppContext";
import { HeroBanner } from "../components/HeroBanner";
import { PackageCard } from "../components/PackageCard";

export const DiscoverView: React.FC = () => {
  const { packages, searchQuery, desktopFilter, systemInfo } = useApp();

  const filteredPackages = packages.filter((pkg) => {
    const matchesSearch =
      !searchQuery ||
      pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase())) ||
      pkg.category.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesDesktop =
      desktopFilter === "all" ||
      pkg.supported_desktops.includes("universal") ||
      pkg.supported_desktops.includes(desktopFilter);

    return matchesSearch && matchesDesktop;
  });

  const featuredRice = packages.find((p) => p.featured) || packages[0];

  const currentWm = systemInfo?.window_manager.toLowerCase() || "hyprland";
  const compatiblePackages = filteredPackages.filter(
    (p) =>
      p.supported_desktops.includes("universal") ||
      p.supported_desktops.some((d) => d === currentWm || currentWm.includes(d))
  );

  return (
    <div className="space-y-8 pb-10">
      {/* Featured Card */}
      {!searchQuery && desktopFilter === "all" && featuredRice && (
        <HeroBanner featuredPackage={featuredRice} />
      )}

      {/* Compatible with your setup */}
      {!searchQuery && compatiblePackages.length > 0 && (
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

      {/* Main Grid */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-bold text-[var(--text-primary)]">
            {searchQuery ? `Results for "${searchQuery}"` : "All Packages"}
          </h2>
          <span className="text-xs text-[var(--text-faint)] font-mono">
            {filteredPackages.length} packages
          </span>
        </div>

        {filteredPackages.length === 0 ? (
          <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)]">
            No packages match your search or desktop filter.
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
