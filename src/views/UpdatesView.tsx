import React, { useState } from "react";
import {
  ArrowUpCircle,
  ShieldAlert,
  Sparkles,
  Layers,
  RefreshCw,
  CheckCircle2,
  AlertTriangle,
  Check,
  RotateCcw,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageUpdateItem, UpdatePlan } from "../types";

export const UpdatesView: React.FC = () => {
  const {
    updatesSummary,
    loadingUpdates,
    refreshUpdates,
    previewPackageUpdate,
    applyPackageUpdate,
    rollbackSnapshot,
    setToast,
  } = useApp();

  const [activePlan, setActivePlan] = useState<{
    item: PackageUpdateItem;
    plan: UpdatePlan;
  } | null>(null);
  const [isUpdating, setIsUpdating] = useState<boolean>(false);
  const [updateResult, setUpdateResult] = useState<{
    success: boolean;
    message: string;
    snapshotId?: string;
  } | null>(null);

  const updates = updatesSummary?.updates || [];
  const securityUpdates = updates.filter((u) => u.category === "Security");
  const featureUpdates = updates.filter((u) => u.category === "Feature");
  const optionalUpdates = updates.filter((u) => u.category === "Optional");

  const handlePreviewUpdate = async (item: PackageUpdateItem) => {
    try {
      const plan = await previewPackageUpdate(item.package_id);
      setActivePlan({ item, plan });
      setUpdateResult(null);
    } catch (e: any) {
      setToast({
        message: `Failed to preview update: ${e?.message || e}`,
        type: "warning",
      });
    }
  };

  const handleApplyUpdate = async () => {
    if (!activePlan) return;
    setIsUpdating(true);
    try {
      const res = await applyPackageUpdate(activePlan.item.package_id);
      if (res.success) {
        setUpdateResult({
          success: true,
          message: `Package '${activePlan.item.name}' updated successfully to v${activePlan.item.available_version}`,
          snapshotId: res.snapshot_id,
        });
        await refreshUpdates();
      } else {
        setUpdateResult({
          success: false,
          message: res.errors?.join("; ") || "Update failed and was rolled back automatically",
          snapshotId: res.snapshot_id,
        });
      }
    } catch (e: any) {
      setUpdateResult({
        success: false,
        message: `Update execution failed: ${e?.message || e}`,
      });
    } finally {
      setIsUpdating(false);
    }
  };

  const handleRollback = async (snapId: string) => {
    try {
      await rollbackSnapshot(snapId);
      setToast({ message: "Rolled back successfully", type: "success" });
      await refreshUpdates();
      setActivePlan(null);
      setUpdateResult(null);
    } catch (e: any) {
      setToast({ message: `Rollback failed: ${e?.message || e}`, type: "warning" });
    }
  };

  const renderUpdateCard = (item: PackageUpdateItem) => {
    const isSecurity = item.category === "Security";
    const isFeature = item.category === "Feature";

    return (
      <div
        key={item.package_id}
        className={`p-4 rounded-lg border transition-all ${
          isSecurity
            ? "border-amber-800/40 bg-amber-950/10 hover:border-amber-700/60"
            : isFeature
            ? "border-sky-800/40 bg-sky-950/10 hover:border-sky-700/60"
            : "border-[var(--border-subtle)] bg-[var(--bg-card)] hover:border-[var(--border-strong)]"
        }`}
      >
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
          <div>
            <div className="flex items-center space-x-2">
              <span className="text-sm font-bold text-[var(--text-primary)]">{item.name}</span>
              <span className="text-[10px] font-mono text-[var(--text-faint)]">({item.package_id})</span>
              <span
                className={`px-1.5 py-0.5 rounded text-[10px] font-mono uppercase font-semibold ${
                  isSecurity
                    ? "bg-amber-950/80 text-amber-400 border border-amber-800/40"
                    : isFeature
                    ? "bg-sky-950/80 text-sky-400 border border-sky-800/40"
                    : "bg-[var(--bg-surface-elevated)] text-[var(--text-muted)] border border-[var(--border-subtle)]"
                }`}
              >
                {item.category}
              </span>
              <span className="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-surface)] text-[var(--text-secondary)] border border-[var(--border-subtle)]">
                {item.release_channel}
              </span>
            </div>

            {/* Version Diff */}
            <div className="flex items-center space-x-2 text-xs font-mono mt-1.5 text-[var(--text-muted)]">
              <span>v{item.installed_version}</span>
              <span className="text-[var(--text-faint)]">→</span>
              <span className="font-bold text-[var(--text-primary)]">v{item.available_version}</span>
              <span className="text-[11px] text-[var(--text-faint)]">• Source: {item.repository_id}</span>
            </div>

            {item.release_notes && (
              <p className="text-xs text-[var(--text-secondary)] mt-2 line-clamp-2 bg-[var(--bg-surface)] p-2 rounded border border-[var(--border-subtle)]/60 font-sans">
                {item.release_notes}
              </p>
            )}
          </div>

          <div className="flex items-center space-x-2 shrink-0">
            <button
              onClick={() => handlePreviewUpdate(item)}
              className="px-3.5 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] transition-colors flex items-center space-x-1.5 shadow-sm"
            >
              <ArrowUpCircle className="w-3.5 h-3.5" />
              <span>Preview & Update</span>
            </button>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div className="space-y-6 pb-12">
      {/* View Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2 mb-1">
            <ArrowUpCircle className="w-5 h-5 text-[var(--accent)]" />
            <h1 className="text-lg font-bold text-[var(--text-primary)]">Package Updates</h1>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Categorized updates with automatic safety snapshots, conflict detection, and 1-click rollback.
          </p>
        </div>

        <button
          onClick={() => refreshUpdates()}
          disabled={loadingUpdates}
          className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1.5 disabled:opacity-50"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loadingUpdates ? "animate-spin" : ""}`} />
          <span>{loadingUpdates ? "Checking Updates..." : "Refresh Updates"}</span>
        </button>
      </div>

      {/* Category Summary Badges */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <div className="p-3 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-card)]">
          <div className="text-[11px] text-[var(--text-muted)] mb-1">Total Available</div>
          <div className="text-xl font-bold text-[var(--text-primary)]">{updates.length}</div>
        </div>
        <div className="p-3 rounded-lg border border-amber-800/30 bg-amber-950/10">
          <div className="text-[11px] text-amber-400 mb-1 flex items-center space-x-1">
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>Security</span>
          </div>
          <div className="text-xl font-bold text-amber-300">{securityUpdates.length}</div>
        </div>
        <div className="p-3 rounded-lg border border-sky-800/30 bg-sky-950/10">
          <div className="text-[11px] text-sky-400 mb-1 flex items-center space-x-1">
            <Sparkles className="w-3.5 h-3.5" />
            <span>Features</span>
          </div>
          <div className="text-xl font-bold text-sky-300">{featureUpdates.length}</div>
        </div>
        <div className="p-3 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-card)]">
          <div className="text-[11px] text-[var(--text-muted)] mb-1 flex items-center space-x-1">
            <Layers className="w-3.5 h-3.5" />
            <span>Optional / Patches</span>
          </div>
          <div className="text-xl font-bold text-[var(--text-secondary)]">{optionalUpdates.length}</div>
        </div>
      </div>

      {/* Empty State */}
      {updates.length === 0 ? (
        <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-3">
          <CheckCircle2 className="w-8 h-8 text-emerald-400 mx-auto" />
          <div className="text-sm font-semibold text-[var(--text-primary)]">
            All Installed Packages Are Up to Date
          </div>
          <p className="text-xs text-[var(--text-muted)] max-w-md mx-auto">
            Your system packages match the latest versions published in configured repositories.
          </p>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Security Updates Section */}
          {securityUpdates.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center space-x-2 text-xs font-bold uppercase tracking-wider text-amber-400">
                <ShieldAlert className="w-4 h-4" />
                <span>Security & Advisory Updates ({securityUpdates.length})</span>
              </div>
              <div className="space-y-2.5">
                {securityUpdates.map(renderUpdateCard)}
              </div>
            </div>
          )}

          {/* Feature Upgrades Section */}
          {featureUpdates.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center space-x-2 text-xs font-bold uppercase tracking-wider text-sky-400">
                <Sparkles className="w-4 h-4" />
                <span>Feature Upgrades ({featureUpdates.length})</span>
              </div>
              <div className="space-y-2.5">
                {featureUpdates.map(renderUpdateCard)}
              </div>
            </div>
          )}

          {/* Optional / Patch Updates Section */}
          {optionalUpdates.length > 0 && (
            <div className="space-y-3">
              <div className="flex items-center space-x-2 text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
                <Layers className="w-4 h-4" />
                <span>Optional & Patch Updates ({optionalUpdates.length})</span>
              </div>
              <div className="space-y-2.5">
                {optionalUpdates.map(renderUpdateCard)}
              </div>
            </div>
          )}
        </div>
      )}

      {/* Update Preview & Apply Modal */}
      {activePlan && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="w-full max-w-xl bg-[var(--bg-surface)] border border-[var(--border-strong)] rounded-lg shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150">
            <div className="p-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
              <div>
                <h3 className="text-sm font-bold text-[var(--text-primary)]">
                  Update Preview: {activePlan.item.name}
                </h3>
                <div className="text-[11px] font-mono text-[var(--text-muted)]">
                  v{activePlan.item.installed_version} → v{activePlan.item.available_version}
                </div>
              </div>
              <button
                onClick={() => setActivePlan(null)}
                className="text-[var(--text-muted)] hover:text-[var(--rz-text)] text-xs px-2 py-1"
              >
                ✕
              </button>
            </div>

            <div className="p-5 space-y-4 max-h-[60vh] overflow-y-auto">
              {updateResult ? (
                <div
                  className={`p-4 rounded border text-xs space-y-2 ${
                    updateResult.success
                      ? "bg-emerald-950/20 border-emerald-800/40 text-emerald-300"
                      : "bg-red-950/20 border-red-800/40 text-red-300"
                  }`}
                >
                  <div className="flex items-center space-x-2 font-semibold">
                    {updateResult.success ? (
                      <Check className="w-4 h-4 text-emerald-400" />
                    ) : (
                      <AlertTriangle className="w-4 h-4 text-red-400" />
                    )}
                    <span>{updateResult.message}</span>
                  </div>

                  {updateResult.snapshotId && (
                    <div className="text-[10px] font-mono text-[var(--text-muted)] pt-1 flex items-center justify-between">
                      <span>Safety Snapshot: {updateResult.snapshotId}</span>
                      <button
                        onClick={() => handleRollback(updateResult.snapshotId!)}
                        className="px-2 py-1 rounded bg-[var(--bg-card)] hover:bg-[var(--bg-surface-elevated)] text-[var(--text-secondary)] border border-[var(--border-subtle)] flex items-center space-x-1"
                      >
                        <RotateCcw className="w-3 h-3" />
                        <span>Rollback Now</span>
                      </button>
                    </div>
                  )}
                </div>
              ) : (
                <>
                  <div className="p-3 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs space-y-1.5">
                    <div className="text-[11px] font-semibold text-[var(--text-secondary)] uppercase">
                      Changes Summary
                    </div>
                    <div className="grid grid-cols-3 gap-2 text-center text-[11px] font-mono">
                      <div className="p-2 rounded bg-emerald-950/30 border border-emerald-800/30 text-emerald-300">
                        {activePlan.plan.creates.length} New
                      </div>
                      <div className="p-2 rounded bg-sky-950/30 border border-sky-800/30 text-sky-300">
                        {activePlan.plan.replaces.length} Update
                      </div>
                      <div className="p-2 rounded bg-red-950/30 border border-red-800/30 text-red-300">
                        {activePlan.plan.obsolete_removes.length} Remove
                      </div>
                    </div>
                  </div>

                  {activePlan.plan.conflicts.length > 0 && (
                    <div className="p-3 rounded bg-amber-950/20 border border-amber-800/30 text-xs text-amber-300 space-y-1">
                      <div className="font-semibold flex items-center space-x-1">
                        <AlertTriangle className="w-3.5 h-3.5" />
                        <span>Local Modifications Preserved</span>
                      </div>
                      <div className="text-[11px] text-amber-400/80">
                        Locally modified files will not be silently overwritten.
                      </div>
                    </div>
                  )}

                  <div className="space-y-1.5">
                    <div className="text-[11px] font-semibold text-[var(--text-secondary)] uppercase">
                      Files to Apply ({activePlan.plan.details.length})
                    </div>
                    <div className="space-y-1 max-h-40 overflow-y-auto">
                      {activePlan.plan.details.map((act, i) => (
                        <div
                          key={i}
                          className="p-1.5 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] text-[11px] font-mono flex items-center justify-between"
                        >
                          <span className="truncate max-w-[340px] text-[var(--text-secondary)]">
                            {act.target}
                          </span>
                          <span
                            className={`px-1 rounded text-[9px] uppercase font-semibold ${
                              act.action === "create"
                                ? "bg-emerald-950 text-emerald-400"
                                : act.action === "replace"
                                ? "bg-sky-950 text-sky-400"
                                : "bg-red-950 text-red-400"
                            }`}
                          >
                            {act.action}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                </>
              )}
            </div>

            <div className="p-4 border-t border-[var(--border-subtle)] bg-[var(--bg-card)] flex items-center justify-end space-x-2">
              <button
                onClick={() => setActivePlan(null)}
                className="px-3 py-1.5 rounded text-xs font-medium text-[var(--text-muted)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)]"
              >
                Close
              </button>

              {!updateResult && (
                <button
                  onClick={handleApplyUpdate}
                  disabled={isUpdating}
                  className="px-4 py-1.5 rounded text-xs font-semibold bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] transition-colors disabled:opacity-50 flex items-center space-x-1.5"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${isUpdating ? "animate-spin" : ""}`} />
                  <span>{isUpdating ? "Applying Update..." : "Apply Update Safely"}</span>
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
