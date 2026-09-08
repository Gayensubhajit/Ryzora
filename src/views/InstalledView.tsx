import React from "react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";

export const InstalledView: React.FC = () => {
  const { packages, installedPackageIds, setActiveCategory } = useApp();

  const installedPackages = packages.filter((pkg) => installedPackageIds.includes(pkg.id));

  return (
    <div className="space-y-6 pb-10">
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            Installed Packages
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Configurations currently applied on your Linux environment.
          </p>
        </div>

        <button
          onClick={() => setActiveCategory("backups")}
          className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
        >
          View Backups
        </button>
      </div>

      {installedPackages.length === 0 ? (
        <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-3">
          <div className="text-xs text-[var(--text-muted)]">No packages currently installed.</div>
          <button
            onClick={() => setActiveCategory("discover")}
            className="px-3 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] transition-colors"
          >
            Browse Marketplace
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          <div className="text-xs text-[var(--text-faint)]">
            {installedPackages.length} active package{installedPackages.length === 1 ? "" : "s"}
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {installedPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
