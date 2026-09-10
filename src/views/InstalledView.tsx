import React, { useState } from "react";
import {
  RefreshCw,
  CheckCircle2,
  ArrowUpCircle,
  History,
  RotateCcw,
  Clock,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";
import { InstalledHistoryEntry, PackageUpdateStatus } from "../types";

export const InstalledView: React.FC = () => {
  const {
    packages,
    installedPackageIds,
    installedPackages: installedRecords,
    setActiveCategory,
    checkAllUpdates,
    getInstalledPackageHistory,
    rollbackSnapshot,
    setToast,
  } = useApp();

  const [checkingUpdates, setCheckingUpdates] = useState<boolean>(false);
  const [updateStatuses, setUpdateStatuses] = useState<PackageUpdateStatus[] | null>(null);
  const [historyModalOpen, setHistoryModalOpen] = useState<boolean>(false);
  const [selectedPkgHistory, setSelectedPkgHistory] = useState<{
    pkgId: string;
    pkgName: string;
    entries: InstalledHistoryEntry[];
  } | null>(null);
  const [rollingBackId, setRollingBackId] = useState<string | null>(null);

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

  const handleOpenHistory = async (pkgId: string, pkgName: string) => {
    try {
      const entries = await getInstalledPackageHistory(pkgId);
      setSelectedPkgHistory({ pkgId, pkgName, entries });
      setHistoryModalOpen(true);
    } catch (e: any) {
      // Fallback: look up in installedRecords
      const rec = installedRecords.find((r) => r.package_id === pkgId);
      if (rec && rec.history) {
        setSelectedPkgHistory({ pkgId, pkgName, entries: rec.history });
        setHistoryModalOpen(true);
      } else {
        setToast({ message: `History not found: ${e?.message || e}`, type: "warning" });
      }
    }
  };

  const handleRollbackSnapshot = async (snapshotId: string) => {
    setRollingBackId(snapshotId);
    try {
      await rollbackSnapshot(snapshotId);
      setToast({ message: "Rolled back to snapshot successfully", type: "success" });
      setHistoryModalOpen(false);
    } catch (e: any) {
      setToast({ message: `Rollback failed: ${e?.message || e}`, type: "warning" });
    } finally {
      setRollingBackId(null);
    }
  };

  const availableUpdates = updateStatuses?.filter((s) => s.status === "update_available") ?? [];

  return (
    <div className="space-y-6 pb-10">
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col sm:flex-row sm:items-center justify-between gap-4">
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
              className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1.5 disabled:opacity-50"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${checkingUpdates ? "animate-spin" : ""}`} />
              <span>{checkingUpdates ? "Checking..." : "Check for Updates"}</span>
            </button>
          )}

          <button
            onClick={() => setActiveCategory("updates")}
            className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
          >
            Updates
          </button>

          <button
            onClick={() => setActiveCategory("backups")}
            className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
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
              <strong>{availableUpdates.length}</strong> package update{availableUpdates.length === 1 ? "" : "s"} available. Click to review.
            </span>
          </div>
          <button
            onClick={() => setActiveCategory("updates")}
            className="px-2.5 py-1 rounded bg-sky-900/40 hover:bg-sky-900/60 border border-sky-700/50 text-sky-200 text-xs font-medium transition-colors"
          >
            Open Updates
          </button>
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
        <div className="space-y-4">
          <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
            <span>{installedPackages.length} active package{installedPackages.length === 1 ? "" : "s"}</span>
            <span className="text-[11px] text-[var(--text-muted)]">
              Click "Version History" on any package for historical snapshots & rollback.
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3.5">
            {installedPackages.map((pkg) => (
              <div key={pkg.id} className="flex flex-col">
                <PackageCard packageItem={pkg} />
                <div className="mt-1 flex items-center justify-between px-1">
                  <button
                    onClick={() => handleOpenHistory(pkg.id, pkg.title)}
                    className="text-[11px] text-[var(--text-muted)] hover:text-[var(--accent)] flex items-center space-x-1 py-1 transition-colors"
                  >
                    <History className="w-3 h-3" />
                    <span>Version History</span>
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Version History & Rollback Modal */}
      {historyModalOpen && selectedPkgHistory && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="w-full max-w-lg bg-[var(--bg-surface)] border border-[var(--border-strong)] rounded-lg shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150">
            <div className="p-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
              <div className="flex items-center space-x-2">
                <History className="w-4 h-4 text-[var(--accent)]" />
                <div>
                  <h3 className="text-sm font-bold text-[var(--text-primary)]">
                    Version History: {selectedPkgHistory.pkgName}
                  </h3>
                  <div className="text-[11px] font-mono text-[var(--text-muted)]">
                    {selectedPkgHistory.pkgId}
                  </div>
                </div>
              </div>
              <button
                onClick={() => setHistoryModalOpen(false)}
                className="text-[var(--text-muted)] hover:text-[var(--rz-text)] text-xs px-2 py-1"
              >
                ✕
              </button>
            </div>

            <div className="p-5 space-y-3 max-h-[60vh] overflow-y-auto">
              <div className="text-xs text-[var(--text-muted)]">
                Recorded historical installations and their corresponding pre-install safety snapshots.
              </div>

              {selectedPkgHistory.entries.length === 0 ? (
                <div className="p-6 text-center text-xs text-[var(--text-muted)] border border-[var(--border-subtle)] rounded bg-[var(--bg-card)]">
                  No historical version records available for this package.
                </div>
              ) : (
                <div className="space-y-2.5">
                  {selectedPkgHistory.entries.map((entry, idx) => {
                    const dateStr = new Date(entry.installed_at * 1000).toLocaleString();
                    const isLatest = idx === selectedPkgHistory.entries.length - 1;

                    return (
                      <div
                        key={idx}
                        className={`p-3 rounded border text-xs space-y-2 transition-all ${
                          isLatest
                            ? "border-[var(--accent)]/50 bg-[var(--accent)]/5"
                            : "border-[var(--border-subtle)] bg-[var(--bg-card)]"
                        }`}
                      >
                        <div className="flex items-center justify-between">
                          <div className="flex items-center space-x-2">
                            <span className="font-bold font-mono text-sm text-[var(--text-primary)]">
                              v{entry.version}
                            </span>
                            {isLatest && (
                              <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase font-semibold bg-[var(--accent)]/20 text-[var(--accent)]">
                                Current
                              </span>
                            )}
                          </div>

                          <div className="text-[11px] text-[var(--text-muted)] flex items-center space-x-1">
                            <Clock className="w-3 h-3" />
                            <span>{dateStr}</span>
                          </div>
                        </div>

                        <div className="text-[11px] font-mono text-[var(--text-muted)] space-y-0.5 pt-1 border-t border-[var(--border-subtle)]/60">
                          <div>Snapshot ID: <span className="text-[var(--text-secondary)]">{entry.snapshot_id || "None (direct install)"}</span></div>
                          {entry.tree_hash && (
                            <div className="truncate">Tree Hash: <span className="text-[var(--text-secondary)]">{entry.tree_hash.substring(0, 16)}...</span></div>
                          )}
                        </div>

                        {entry.snapshot_id && (
                          <div className="pt-1 flex items-center justify-end">
                            <button
                              onClick={() => handleRollbackSnapshot(entry.snapshot_id)}
                              disabled={rollingBackId === entry.snapshot_id}
                              className="px-2.5 py-1 rounded text-xs font-medium text-[var(--text-secondary)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1 disabled:opacity-50"
                            >
                              <RotateCcw className={`w-3 h-3 ${rollingBackId === entry.snapshot_id ? "animate-spin" : ""}`} />
                              <span>{rollingBackId === entry.snapshot_id ? "Rolling back..." : "Rollback to Snapshot"}</span>
                            </button>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>

            <div className="p-4 border-t border-[var(--border-subtle)] bg-[var(--bg-card)] flex items-center justify-end">
              <button
                onClick={() => setHistoryModalOpen(false)}
                className="px-3 py-1.5 rounded text-xs font-medium text-[var(--text-muted)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)]"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
