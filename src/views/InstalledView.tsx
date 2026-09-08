import React, { useState } from "react";
import { RefreshCw, CheckCircle2, ArrowUpCircle } from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";
import { PackageUpdateStatus } from "../types";

export const InstalledView: React.FC = () => {
  const { packages, installedPackageIds, setActiveCategory, checkAllUpdates } = useApp();
  const [checkingUpdates, setCheckingUpdates] = useState<boolean>(false);
  const [updateStatuses, setUpdateStatuses] = useState<PackageUpdateStatus[] | null>(null);

  const installedPackages = packages.filter((pkg) => installedPackageIds.includes(pkg.id));

  const handleCheckAllUpdates = async () => {
    setCheckingUpdates(true);
    try {
      const statuses = await checkAllUpdates();
      setUpdateStatuses(statuses);
    } catch (e) {
      console.error("Failed to check updates:", e);
    } finally {
      setCheckingUpdates(false);
    }
  };

  const availableUpdates = updateStatuses?.filter((s) => s.status === "update_available") ?? [];

  return (
    <div className="space-y-6 pb-10">
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            Installed Packages
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Configurations currently applied on your Linux environment ({installedPackages.length} active).
          </p>
        </div>

        <div className="flex items-center space-x-2">
          {installedPackages.length > 0 && (
            <button
              onClick={handleCheckAllUpdates}
              disabled={checkingUpdates}
              className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1.5 disabled:opacity-50"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${checkingUpdates ? "animate-spin" : ""}`} />
              <span>{checkingUpdates ? "Checking..." : "Check for Updates"}</span>
            </button>
          )}

          <button
            onClick={() => setActiveCategory("backups")}
            className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
          >
            View Backups
          </button>
        </div>
      </div>

      {availableUpdates.length > 0 && (
        <div className="p-3.5 rounded-lg border border-sky-800/40 bg-sky-950/20 flex items-center justify-between text-xs text-sky-300">
          <div className="flex items-center space-x-2">
            <ArrowUpCircle className="w-4 h-4 text-sky-400 shrink-0" />
            <span>
              <strong>{availableUpdates.length}</strong> package update{availableUpdates.length === 1 ? "" : "s"} available. Click a package card to preview and safely apply changes.
            </span>
          </div>
        </div>
      )}

      {updateStatuses !== null && availableUpdates.length === 0 && (
        <div className="p-3 rounded-lg border border-emerald-800/30 bg-emerald-950/20 flex items-center space-x-2 text-xs text-emerald-300">
          <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
          <span>All installed packages are up to date.</span>
        </div>
      )}

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
