import React, { useState, useEffect } from "react";
import {
  X,
  RotateCcw,
  Check,
  Sparkles,
  AlertCircle,
  ShieldCheck,
  Shield,
  Key,
  Eye,
  FilePlus,
  FileEdit,
  FileCheck,
  Trash2,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
  Terminal,
  Cpu,
  Package as PackageIcon,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import {
  DependencyResolutionReport,
  InstallationPlan,
  UninstallResult,
  PackageUpdateStatus,
  UpdatePlan,
  UpdateResult,
} from "../types";

export const PackageDetailModal: React.FC = () => {
  const {
    selectedPackage,
    setSelectedPackage,
    setActiveCategory,
    installedPackageIds,
    installPackage,
    previewInstallation,
    resolvePackageDependencies,
    isInstalling,
    installProgress,
    installLogs,
    snapshots,
    rollbackSnapshot,
    checkCompatibility,
    installedPackages,
    uninstallPackage,
    checkPackageUpdate,
    previewPackageUpdate,
    applyPackageUpdate,
    setToast,
  } = useApp();

  const [activeTab, setActiveTab] = useState<"overview" | "manifest" | "dependencies">("overview");
  const [selectedScreenshotIndex, setSelectedScreenshotIndex] = useState<number>(0);

  // Dependency Resolution (Phase 9)
  const [depReport, setDepReport] = useState<DependencyResolutionReport | null>(null);
  const [loadingDeps, setLoadingDeps] = useState<boolean>(false);
  const [depError, setDepError] = useState<string | null>(null);

  useEffect(() => {
    if (selectedPackage) {
      setDepReport(null);
      setDepError(null);
    }
  }, [selectedPackage?.id]);

  useEffect(() => {
    if (selectedPackage && activeTab === "dependencies" && !depReport && !loadingDeps && !depError) {
      setLoadingDeps(true);
      setDepError(null);
      resolvePackageDependencies(selectedPackage.id)
        .then((rep) => setDepReport(rep))
        .catch((err) => {
          console.error("Dependency resolution failed:", err);
          const msg = typeof err === "string" ? err : (err && typeof err === "object" && "message" in err ? String(err.message) : "Failed to resolve dependency graph");
          setDepError(msg);
        })
        .finally(() => setLoadingDeps(false));
    }
  }, [selectedPackage, activeTab, depReport, loadingDeps, depError, resolvePackageDependencies]);

  // Preview & Installation flow states
  const [showPreview, setShowPreview] = useState<boolean>(false);
  const [loadingPlan, setLoadingPlan] = useState<boolean>(false);
  const [plan, setPlan] = useState<InstallationPlan | null>(null);
  const [installCompleted, setInstallCompleted] = useState<boolean>(false);
  const [installedSnapshotId, setInstalledSnapshotId] = useState<string | null>(null);
  const [createSnapshot, setCreateSnapshot] = useState<boolean>(() => {
    const pref = localStorage.getItem("ryzora_snapshot_on_install");
    return pref !== null ? pref === "true" : false;
  });

  // Uninstall flow states
  const [showUninstall, setShowUninstall] = useState<boolean>(false);
  const [isUninstalling, setIsUninstalling] = useState<boolean>(false);
  const [uninstallResult, setUninstallResult] = useState<UninstallResult | null>(null);
  const [uninstallError, setUninstallError] = useState<string | null>(null);

  // Update flow states
  const [checkingUpdate, setCheckingUpdate] = useState<boolean>(false);
  const [updateStatus, setUpdateStatus] = useState<PackageUpdateStatus | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState<boolean>(false);
  const [loadingUpdatePlan, setLoadingUpdatePlan] = useState<boolean>(false);
  const [updatePlan, setUpdatePlan] = useState<UpdatePlan | null>(null);
  const [isUpdating, setIsUpdating] = useState<boolean>(false);
  const [updateResult, setUpdateResult] = useState<UpdateResult | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);

  if (!selectedPackage) return null;

  const isInstalled = installedPackageIds.includes(selectedPackage.id);
  const relatedSnapshot = snapshots.find(
    (s) => s.label === (selectedPackage.manifest?.name ?? selectedPackage.title)
  );
  const compat = checkCompatibility(selectedPackage);
  const installedRecord = installedPackages.find((p) => p.package_id === selectedPackage.id);

  const hasMissingRequiredDeps = Boolean(
    (plan && (
      (plan.dependency_report && plan.dependency_report.missing_required.length > 0) ||
      plan.missing_dependencies.length > 0
    )) ||
    compat.missing_required_apps.length > 0
  );

  const firstMissingDep =
    plan?.dependency_report?.missing_required?.[0] ||
    plan?.missing_dependencies?.[0] ||
    compat.missing_required_apps?.[0] ||
    null;
  const isLegacyRecord = installedRecord && (!installedRecord.files || installedRecord.files.length === 0);

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);
    try {
      const status = await checkPackageUpdate(selectedPackage.id);
      setUpdateStatus(status);
      if (status.status === "update_available") {
        handleOpenUpdateModal();
      }
    } catch (e: any) {
      console.error("Update check failed:", e);
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handleOpenUpdateModal = async () => {
    setShowUpdateModal(true);
    setLoadingUpdatePlan(true);
    setUpdateError(null);
    setUpdateResult(null);
    try {
      const p = await previewPackageUpdate(selectedPackage.id);
      setUpdatePlan(p);
    } catch (e: any) {
      setUpdateError(e?.message ?? String(e));
    } finally {
      setLoadingUpdatePlan(false);
    }
  };

  const handleConfirmUpdate = async () => {
    setIsUpdating(true);
    setUpdateError(null);
    try {
      const res = await applyPackageUpdate(selectedPackage.id);
      setUpdateResult(res);
      if (res.success) {
        setUpdateStatus(null);
      }
    } catch (e: any) {
      setUpdateError(e?.message ?? String(e));
    } finally {
      setIsUpdating(false);
    }
  };

  const handleOpenUninstall = () => {
    setShowUninstall(true);
    setUninstallError(null);
    setUninstallResult(null);
  };

  const handleConfirmUninstall = async () => {
    setIsUninstalling(true);
    setUninstallError(null);
    try {
      const res = await uninstallPackage(selectedPackage.id);
      setUninstallResult(res);
    } catch (e: any) {
      setUninstallError(e?.message ?? String(e));
    } finally {
      setIsUninstalling(false);
    }
  };

  const handleOpenPreview = async () => {
    setLoadingPlan(true);
    setShowPreview(true);
    try {
      const p = await previewInstallation(selectedPackage.id);
      setPlan(p);
    } catch (e: any) {
      const msg = e?.message || String(e);
      setPlan(null);
      setShowPreview(false);
      setToast({
        message: `Preview failed: ${msg}`,
        type: "warning",
      });
    } finally {
      setLoadingPlan(false);
    }
  };

  const handleConfirmInstall = async () => {
    try {
      const res = await installPackage(selectedPackage, createSnapshot);
      if (res && res.success) {
        setInstalledSnapshotId(res.snapshot_id || null);
        setInstallCompleted(true);
      }
    } catch {
      // Error handled in AppContext logs and toast
    }
  };

  const handleClose = () => {
    setShowPreview(false);
    setShowUninstall(false);
    setShowUpdateModal(false);
    setPlan(null);
    setUpdatePlan(null);
    setUninstallResult(null);
    setUpdateResult(null);
    setInstallCompleted(false);
    setSelectedPackage(null);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 md:p-8 select-none" style={{ background: "var(--rz-modal-backdrop)" }} onClick={handleClose}>
      {/* Blurred artwork backdrop */}
      {selectedPackage.hero_image && (
        <div
          className="absolute inset-0 -z-0 bg-cover bg-center opacity-25 pointer-events-none transition-opacity duration-700"
          style={{ backgroundImage: `url(${selectedPackage.hero_image})`, filter: "blur(64px) saturate(140%)" }}
        />
      )}
      <div className="absolute inset-0 -z-0 bg-black/50 pointer-events-none" />
      <div
        className="relative z-10 w-full max-w-3xl max-h-[86vh] flex flex-col rounded-[16px] overflow-hidden backdrop-blur-[28px]"
        style={{
          background: "var(--rz-glass-modal-bg)",
          border: "var(--rz-glass-modal-border)",
          boxShadow: "var(--rz-glass-modal-shadow)",
          backdropFilter: "var(--rz-glass-backdrop)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Subtle glass top highlight */}
        <div
          className="absolute inset-x-0 top-0 h-[1px] pointer-events-none z-20"
          style={{
            background: "linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.18), transparent)",
          }}
        />
        {/* Header */}
        <div
          className="px-5 py-3.5 border-b border-[var(--rz-border-subtle)] flex items-center justify-between gap-4"
          style={{
            background: "var(--rz-glass-header-bg)",
            backdropFilter: "var(--rz-glass-backdrop)",
          }}
        >
          <div className="flex items-center gap-2.5 truncate">
            <span className="font-semibold text-sm text-[var(--rz-text)] truncate">
              {selectedPackage.title}
            </span>
            <span className="text-[10px] font-mono text-[var(--rz-text-secondary)] px-1.5 py-0.5 rounded-[6px] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
              v{selectedPackage.version}
            </span>
            {selectedPackage.trust_tier === "official" && (
              <span className="text-[10px] font-semibold text-[var(--rz-badge-warning-text)] px-2 py-0.5 rounded-[6px] bg-[var(--rz-badge-warning-bg)] border border-[var(--rz-badge-warning-border)] flex items-center gap-1">
                <Sparkles className="w-3 h-3 text-[var(--rz-badge-warning-text)]" />
                Official
              </span>
            )}
            {selectedPackage.trust_tier === "verified" && (
              <span className="text-[10px] font-semibold text-[var(--rz-badge-info-text)] px-2 py-0.5 rounded-[6px] bg-[var(--rz-badge-info-bg)] border border-[var(--rz-badge-info-border)] flex items-center gap-1">
                <ShieldCheck className="w-3 h-3 text-[var(--rz-badge-info-text)]" />
                Verified
              </span>
            )}
            {selectedPackage.moderation_status && (
              <span className={`text-[10px] font-semibold px-2 py-0.5 rounded-[6px] border flex items-center gap-1 ${
                selectedPackage.moderation_status === "approved"
                  ? "bg-[var(--rz-badge-success-bg)] text-[var(--rz-badge-success-text)] border-[var(--rz-badge-success-border)]"
                  : selectedPackage.moderation_status === "pending_review"
                  ? "bg-[var(--rz-badge-warning-bg)] text-[var(--rz-badge-warning-text)] border-[var(--rz-badge-warning-border)]"
                  : selectedPackage.moderation_status === "flagged"
                  ? "bg-[var(--rz-badge-danger-bg)] text-[var(--rz-badge-danger-text)] border-[var(--rz-badge-danger-border)]"
                  : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border-[var(--rz-border-subtle)]"
              }`}>
                {selectedPackage.moderation_status === "approved" && "Approved"}
                {selectedPackage.moderation_status === "pending_review" && "Pending Review"}
                {selectedPackage.moderation_status === "flagged" && "Flagged"}
                {selectedPackage.moderation_status === "deprecated" && "Deprecated"}
              </span>
            )}
            {selectedPackage.release_channel && (
              <span className="text-[10px] font-mono capitalize px-1.5 py-0.5 rounded-[6px] bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]">
                {selectedPackage.release_channel}
              </span>
            )}
            {showPreview && !installCompleted && (
              <span className="text-[10px] font-mono font-semibold text-[var(--rz-badge-info-text)] px-2 py-0.5 rounded-[6px] bg-[var(--rz-badge-info-bg)] border border-[var(--rz-badge-info-border)]">
                Installation Preview
              </span>
            )}
          </div>

          <button
            onClick={handleClose}
            className="p-1.5 rounded-[8px] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-transparent hover:border-[var(--rz-border-subtle)] transition-colors"
            title="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Modal Body */}
        <div
          className="flex-1 overflow-y-auto p-5 space-y-4"
          style={{ background: "var(--rz-glass-body-bg)" }}
        >
          {/* ─────────────────────────────────────────────────────────────
              COMPLETED VIEW
          ───────────────────────────────────────────────────────────── */}
          {installCompleted ? (
            <div className="py-8 px-4 text-center space-y-4">
              <div className="inline-flex p-3 rounded-full bg-emerald-950/40 text-emerald-400 border border-emerald-800/60">
                <Check className="w-8 h-8" />
              </div>

              <div>
                <h3 className="text-base font-bold text-[var(--text-primary)]">
                  Installed Successfully
                </h3>
                <p className="text-xs text-[var(--text-muted)] mt-1 max-w-md mx-auto">
                  All configuration files were safely staged, applied, and verified with SHA-256 checksums.
                </p>
              </div>

              {installedSnapshotId ? (
                <div className="inline-block px-3 py-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-xs font-mono text-left">
                  <div className="text-[10px] text-[var(--text-muted)] uppercase tracking-wider font-sans">
                    Pre-install Snapshot ID
                  </div>
                  <div className="text-[var(--accent-text)] font-semibold mt-0.5">
                    {installedSnapshotId}
                  </div>
                </div>
              ) : (
                <div className="inline-block px-3 py-1.5 rounded-md bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-xs text-left">
                  <div className="text-[10px] text-[var(--rz-text-muted)] uppercase tracking-wider font-semibold">
                    Snapshot Status
                  </div>
                  <div className="text-[var(--rz-text-secondary)] mt-0.5 font-medium">
                    Installed directly without backup snapshot
                  </div>
                </div>
              )}

              <div className="flex items-center justify-center gap-3 pt-2">
                {installedSnapshotId && (
                  <button
                    onClick={() => {
                      handleClose();
                      setActiveCategory("backups");
                    }}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors"
                  >
                    <RotateCcw className="w-3.5 h-3.5" />
                    <span>Open Backups</span>
                  </button>
                )}

                <button
                  onClick={() => {
                    handleClose();
                    setActiveCategory("installed");
                  }}
                  className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] border border-[var(--rz-border-subtle)] hover:bg-[var(--rz-surface-elevated)] transition-colors"
                >
                  <span>View in Installed</span>
                </button>

                <button
                  onClick={handleClose}
                  className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
                >
                  Done
                </button>
              </div>
            </div>
          ) : showUninstall ? (
            <div className="space-y-4">
              <div className="p-4 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-3">
                <div className="flex items-start justify-between">
                  <div>
                    <h2 className="text-sm font-semibold text-[var(--text-primary)]">
                      Safe Package Uninstall
                    </h2>
                    <p className="text-xs text-[var(--text-muted)] mt-0.5">
                      Review tracked files and safety guarantees before removal.
                    </p>
                  </div>
                  <span className="px-2 py-0.5 rounded text-[11px] font-mono bg-[var(--bg-surface-elevated)] text-[var(--text-secondary)] border border-[var(--border-subtle)]">
                    v{installedRecord?.version ?? selectedPackage.version}
                  </span>
                </div>

                {isLegacyRecord ? (
                  <div className="p-3 rounded bg-amber-950/30 border border-amber-800/40 text-amber-300 text-xs space-y-2">
                    <div className="flex items-center gap-1.5 font-semibold">
                      <AlertTriangle className="w-4 h-4 text-amber-400" />
                      <span>Legacy Metadata Detected</span>
                    </div>
                    <p className="text-[11px] leading-relaxed text-amber-200/90">
                      This package was installed before cryptographic checksum verification was available. Automated uninstall cannot verify ownership of these files. To prevent accidental data loss, automated deletion is refused. Please remove your configuration files manually or reinstall this package through the current installer.
                    </p>
                  </div>
                ) : (
                  <>
                    <div className="space-y-1.5">
                      <div className="text-xs font-medium text-[var(--text-secondary)]">
                        Tracked files ({installedRecord?.files?.length ?? 0}):
                      </div>
                      <div className="max-h-36 overflow-y-auto rounded bg-black/40 border border-[var(--border-subtle)] p-2 font-mono text-[11px] text-[var(--text-muted)] space-y-1">
                        {installedRecord?.files && installedRecord.files.length > 0 ? (
                          installedRecord.files.map((f, idx) => (
                            <div key={idx} className="flex items-center justify-between">
                              <span className="text-zinc-300 truncate">{f.target}</span>
                              <span className="text-[10px] text-zinc-500 font-mono">
                                {f.sha256.slice(0, 8)}...
                              </span>
                            </div>
                          ))
                        ) : (
                          <div className="text-zinc-500">No tracked files listed</div>
                        )}
                      </div>
                    </div>

                    <div className="p-2.5 rounded bg-blue-950/20 border border-blue-800/30 text-blue-300 text-[11px] space-y-1">
                      <div className="flex items-center gap-1 font-semibold text-blue-200">
                        <ShieldCheck className="w-3.5 h-3.5" />
                        <span>Zero-Loss Protection Guarantee</span>
                      </div>
                      <p className="text-blue-300/90 leading-relaxed">
                        If you have modified any of these files on your system after installation, Ryzora's checksum engine will detect the modification, preserve the file on disk, and retain it as a conflict. It will never be deleted.
                      </p>
                    </div>

                    <div className="p-2.5 rounded bg-emerald-950/20 border border-emerald-800/30 text-emerald-300 text-[11px] space-y-1">
                      <div className="flex items-center gap-1 font-semibold text-emerald-200">
                        <Check className="w-3.5 h-3.5" />
                        <span>Verified Pre-Uninstall Snapshot</span>
                      </div>
                      <p className="text-emerald-300/90 leading-relaxed">
                        A full verified backup snapshot of all files will be recorded before any deletions occur. In the unlikely event of an error, rollback will restore all files automatically.
                      </p>
                    </div>
                  </>
                )}

                {uninstallError && (
                  <div className="p-2.5 rounded bg-red-950/30 border border-red-800/40 text-red-300 text-xs flex items-center gap-2">
                    <AlertCircle className="w-4 h-4 shrink-0 text-red-400" />
                    <span>{uninstallError}</span>
                  </div>
                )}

                {uninstallResult && (
                  <div className="p-3 rounded bg-zinc-900 border border-emerald-500/40 space-y-2">
                    <div className="flex items-center gap-1.5 text-xs font-semibold text-emerald-400">
                      <CheckCircle2 className="w-4 h-4" />
                      <span>Uninstall Completed Successfully</span>
                    </div>
                    <div className="text-[11px] text-[var(--text-muted)] space-y-0.5 font-mono">
                      <div>Removed files: {uninstallResult.removed_files.length}</div>
                      {uninstallResult.conflict_files.length > 0 && (
                        <div className="text-amber-400 font-semibold">
                          Retained modified user files: {uninstallResult.conflict_files.join(", ")}
                        </div>
                      )}
                      {uninstallResult.already_missing_files.length > 0 && (
                        <div>Already missing files: {uninstallResult.already_missing_files.length}</div>
                      )}
                      {uninstallResult.snapshot_id && (
                        <div>Pre-uninstall snapshot: #{uninstallResult.snapshot_id}</div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            </div>
          ) : showUpdateModal ? (
            <div className="space-y-4">
              <div className="p-4 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-3">
                <div className="flex items-start justify-between">
                  <div>
                    <h2 className="text-sm font-semibold text-[var(--text-primary)]">
                      Safe Package Update
                    </h2>
                    <p className="text-xs text-[var(--text-muted)] mt-0.5">
                      Preview changes between your current installation and the updated package.
                    </p>
                  </div>
                  {updatePlan && (
                    <span className="px-2 py-0.5 rounded text-[11px] font-mono bg-sky-950/40 text-sky-300 border border-sky-800/40">
                      v{updatePlan.from_version} → v{updatePlan.to_version}
                    </span>
                  )}
                </div>

                {loadingUpdatePlan ? (
                  <div className="py-8 text-center text-xs text-[var(--text-muted)] flex items-center justify-center gap-2">
                    <RefreshCw className="w-4 h-4 animate-spin text-[var(--accent)]" />
                    <span>Evaluating package update preview...</span>
                  </div>
                ) : updatePlan ? (
                  <>
                    <div className="grid grid-cols-3 gap-2 text-center text-xs">
                      <div className="p-2 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                        <div className="text-emerald-400 font-mono font-bold text-sm">
                          {updatePlan.creates.length}
                        </div>
                        <div className="text-[10px] text-[var(--text-muted)]">To Create</div>
                      </div>
                      <div className="p-2 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                        <div className="text-sky-400 font-mono font-bold text-sm">
                          {updatePlan.replaces.length}
                        </div>
                        <div className="text-[10px] text-[var(--text-muted)]">To Replace</div>
                      </div>
                      <div className="p-2 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                        <div className="text-zinc-400 font-mono font-bold text-sm">
                          {updatePlan.unchanged.length}
                        </div>
                        <div className="text-[10px] text-[var(--text-muted)]">Unchanged</div>
                      </div>
                    </div>

                    <div className="space-y-1.5">
                      <div className="text-xs font-medium text-[var(--text-secondary)]">
                        File Actions:
                      </div>
                      <div className="max-h-40 overflow-y-auto rounded bg-black/40 border border-[var(--border-subtle)] p-2 font-mono text-[11px] space-y-1.5">
                        {updatePlan.details.map((d, idx) => (
                          <div key={idx} className="flex items-center justify-between gap-2">
                            <span className="text-zinc-300 truncate">{d.target}</span>
                            <span className={`px-1.5 py-0.5 rounded text-[9px] uppercase font-bold shrink-0 ${
                              d.action === "create" ? "bg-emerald-950/60 text-emerald-400 border border-emerald-800/40" :
                              d.action === "replace" ? "bg-sky-950/60 text-sky-400 border border-sky-800/40" :
                              d.action === "unchanged" ? "bg-zinc-800/60 text-zinc-400 border border-zinc-700/40" :
                              d.action === "conflict" || d.action === "obsolete_retain" ? "bg-amber-950/60 text-amber-400 border border-amber-800/40" :
                              "bg-rose-950/60 text-rose-400 border border-rose-800/40"
                            }`}>
                              {d.action.replace("_", " ")}
                            </span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {updatePlan.has_conflicts && (
                      <div className="p-2.5 rounded bg-amber-950/30 border border-amber-800/40 text-amber-300 text-[11px] space-y-1">
                        <div className="flex items-center gap-1 font-semibold text-amber-200">
                          <AlertTriangle className="w-3.5 h-3.5" />
                          <span>Conflicting User Modifications Detected ({updatePlan.conflicts.length})</span>
                        </div>
                        <p className="text-amber-200/90 leading-relaxed">
                          The update engine identified user modifications in: {updatePlan.conflicts.join(", ")}. These modified files will NOT be overwritten during update application and are safely preserved.
                        </p>
                      </div>
                    )}

                    <div className="p-2.5 rounded bg-blue-950/20 border border-blue-800/30 text-blue-300 text-[11px] space-y-1">
                      <div className="flex items-center gap-1 font-semibold text-blue-200">
                        <ShieldCheck className="w-3.5 h-3.5" />
                        <span>Pre-Update Snapshot & Rollback Guarantee</span>
                      </div>
                      <p className="text-blue-300/90 leading-relaxed">
                        A verified pre-update snapshot will be created before any changes. If anything fails during staging or application, Ryzora will automatically roll back to your current installation.
                      </p>
                    </div>
                  </>
                ) : null}

                {updateError && (
                  <div className="p-2.5 rounded bg-red-950/30 border border-red-800/40 text-red-300 text-xs flex items-center gap-2">
                    <AlertCircle className="w-4 h-4 shrink-0 text-red-400" />
                    <span>{updateError}</span>
                  </div>
                )}

                {updateResult && (
                  <div className="p-3 rounded bg-zinc-900 border border-emerald-500/40 space-y-2">
                    <div className="flex items-center gap-1.5 text-xs font-semibold text-emerald-400">
                      <CheckCircle2 className="w-4 h-4" />
                      <span>Update Applied Successfully!</span>
                    </div>
                    <div className="text-[11px] text-[var(--text-muted)] space-y-0.5 font-mono">
                      <div>Updated to version: v{updateResult.to_version}</div>
                      <div>Updated files: {updateResult.updated_files.length}</div>
                      {updateResult.obsolete_removed.length > 0 && (
                        <div>Obsolete files removed: {updateResult.obsolete_removed.length}</div>
                      )}
                      {updateResult.conflicts_retained.length > 0 && (
                        <div className="text-amber-400 font-semibold">
                          Modified files retained: {updateResult.conflicts_retained.join(", ")}
                        </div>
                      )}
                      <div>Backup snapshot: #{updateResult.snapshot_id}</div>
                    </div>
                  </div>
                )}
              </div>
            </div>
          ) : showPreview ? (
            /* ─────────────────────────────────────────────────────────────
                PREVIEW / DRY-RUN VIEW
            ───────────────────────────────────────────────────────────── */
            <div className="space-y-4">
              {loadingPlan ? (
                <div className="py-12 text-center text-xs text-[var(--text-muted)] space-y-2">
                  <div className="font-mono text-xs animate-pulse">Inspecting package & calculating filesystem diff...</div>
                  <div className="text-[11px] text-[var(--text-faint)]">Read-only dry run · No files modified</div>
                </div>
              ) : plan ? (
                <div className="space-y-4 text-xs">
                  {/* Summary Banner */}
                  <div className="flex items-center justify-between pb-3.5 border-b border-[var(--rz-border-subtle)]">
                    <div>
                      <div className="font-semibold text-sm text-[var(--rz-text)]">
                        {plan.files_to_create.length + plan.files_to_replace.length === 1
                          ? "1 file will be installed"
                          : `${plan.files_to_create.length + plan.files_to_replace.length} files will be installed`}
                      </div>
                      <div className="text-xs text-[var(--rz-text-secondary)] mt-0.5">
                        {plan.files_to_create.length} new · {plan.files_to_replace.length} replaced · {plan.files_unchanged.length} unchanged
                      </div>
                    </div>
                    <span className="text-[10px] font-mono text-[var(--rz-text-secondary)] px-2 py-0.5 rounded-[6px] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                      Dry Run Validated
                    </span>
                  </div>

                  {/* Files Breakdown */}
                  <div className="space-y-3">
                    {/* Files to Create */}
                    {plan.files_to_create.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text)]">
                          <FilePlus className="w-3.5 h-3.5 text-[#6fd6a5]" />
                          <span>Create ({plan.files_to_create.length})</span>
                        </div>
                        <div className="rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] divide-y divide-[var(--rz-border-subtle)] overflow-hidden font-mono text-xs shadow-xs">
                          {plan.files_to_create.map((path, idx) => (
                            <div key={idx} className="p-2.5 flex items-center gap-2 text-[var(--rz-text)]">
                              <span className="text-[#6fd6a5] font-bold">+</span>
                              <span className="truncate">{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Files to Replace */}
                    {plan.files_to_replace.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text)]">
                          <FileEdit className="w-3.5 h-3.5 text-[#e5b85c]" />
                          <span>Replace / Modify ({plan.files_to_replace.length})</span>
                        </div>
                        <div className="rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] divide-y divide-[var(--rz-border-subtle)] overflow-hidden font-mono text-xs shadow-xs">
                          {plan.files_to_replace.map((path, idx) => (
                            <div key={idx} className="p-2.5 flex items-center gap-2 text-[var(--rz-text)]">
                              <span className="text-[#e5b85c] font-bold">~</span>
                              <span className="truncate">{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Files Unchanged */}
                    {plan.files_unchanged.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text-secondary)]">
                          <FileCheck className="w-3.5 h-3.5 text-[var(--rz-text-muted)]" />
                          <span>Unchanged ({plan.files_unchanged.length})</span>
                        </div>
                        <div className="rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] divide-y divide-[var(--rz-border-subtle)] overflow-hidden font-mono text-xs shadow-xs">
                          {plan.files_unchanged.map((path, idx) => (
                            <div key={idx} className="p-2.5 flex items-center gap-2 text-[var(--rz-text-secondary)]">
                              <span className="font-bold text-[var(--rz-text-muted)]">=</span>
                              <span className="truncate">{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Safety & Backup Option */}
                  <div className="pt-3.5 border-t border-[var(--rz-border-subtle)] space-y-2">
                    <div className="text-[11px] font-semibold text-[var(--rz-text-muted)] flex items-center justify-between">
                      <span>Safety & Backup Option</span>
                      <span className={`text-[10px] font-mono px-2 py-0.5 rounded-[5px] border ${
                        createSnapshot
                          ? "bg-[rgba(111,214,165,0.10)] text-[#6fd6a5] border-emerald-400/20"
                          : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border-[var(--rz-border-subtle)]"
                      }`}>
                        {createSnapshot ? "Snapshot: Enabled" : "Snapshot: Skipped"}
                      </span>
                    </div>

                    <label className="flex items-start gap-2.5 p-2.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] cursor-pointer transition-colors group">
                      <input
                        type="checkbox"
                        checked={createSnapshot}
                        onChange={(e) => {
                          setCreateSnapshot(e.target.checked);
                          localStorage.setItem("ryzora_snapshot_on_install", String(e.target.checked));
                        }}
                        className="mt-0.5 w-4 h-4 rounded border-[var(--rz-border)] text-[var(--rz-accent)] focus:ring-0 bg-transparent cursor-pointer shrink-0 accent-[var(--rz-accent)]"
                      />
                      <div className="space-y-0.5 flex-1 select-none">
                        <div className="text-xs font-semibold text-[var(--rz-text)] flex items-center gap-1.5">
                          <Shield className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                          <span>Create safety backup snapshot before installing</span>
                        </div>
                        <div className="text-[11px] text-[var(--rz-text-secondary)] leading-relaxed">
                          {createSnapshot
                            ? "Takes an atomic verified snapshot of existing config files before modification. Enables 1-click rollback from Backups."
                            : "Installs directly without taking a backup snapshot. Faster installation and uses less disk space."}
                        </div>
                      </div>
                    </label>
                  </div>

                  {/* Dependencies & Compatibility */}
                  <div className="pt-3.5 border-t border-[var(--rz-border-subtle)] space-y-2.5">
                    <div className="flex items-center justify-between">
                      <div className="text-xs font-semibold text-[var(--rz-text)] flex items-center gap-1.5">
                        <Cpu className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                        <span>Dependencies & Compatibility</span>
                      </div>
                      {plan.dependency_report && (
                        <span className={`text-[10px] font-mono px-2 py-0.5 rounded-[6px] border ${
                          plan.dependency_report.resolved
                            ? "bg-[rgba(111,214,165,0.10)] text-[#6fd6a5] border-emerald-400/20"
                            : plan.dependency_report.missing_required.length === 0
                            ? "bg-[rgba(230,184,92,0.10)] text-[#e5b85c] border-amber-400/20"
                            : "bg-rose-500/10 text-[#ef7474] border-rose-500/20"
                        }`}>
                          {plan.dependency_report.resolved
                            ? "DAG Resolved"
                            : plan.dependency_report.missing_required.length === 0
                            ? "Optional Missing"
                            : "Required Missing"}
                        </span>
                      )}
                    </div>

                    {plan.dependency_report?.install_order && plan.dependency_report.install_order.length > 1 && (
                      <div className="text-xs text-[var(--rz-text-secondary)] font-mono bg-[var(--rz-surface-elevated)] px-2.5 py-1.5 rounded-[6px] border border-[var(--rz-border-subtle)]">
                        Install Sequence: {plan.dependency_report.install_order.join(" → ")}
                      </div>
                    )}

                    <div className="space-y-1.5 pt-0.5">
                      {plan.required_dependencies.length > 0 ? (
                        plan.required_dependencies.map((dep, idx) => {
                          const isMissing = plan.missing_dependencies.includes(dep);
                          return (
                            <div key={idx} className="flex items-center justify-between font-mono text-xs">
                              <span className="text-[var(--rz-text)] font-semibold">{dep}</span>
                              {isMissing ? (
                                <span className="inline-flex items-center gap-1 text-[var(--rz-status-error)] font-sans font-medium text-xs">
                                  <AlertCircle className="w-3.5 h-3.5" />
                                  Missing in PATH (Required)
                                </span>
                              ) : (
                                <span className="inline-flex items-center gap-1 text-[var(--rz-status-success)] font-sans font-medium text-xs">
                                  <Check className="w-3.5 h-3.5" />
                                  Available in PATH
                                </span>
                              )}
                            </div>
                          );
                        })
                      ) : (
                        <div className="text-xs text-[var(--rz-text-secondary)]">No external application dependencies required.</div>
                      )}
                    </div>

                    {plan.dependency_report?.missing_required && plan.dependency_report.missing_required.length > 0 && (
                      <div
                        className="p-3.5 rounded-xl text-xs space-y-1 mt-2"
                        style={{
                          background: "var(--rz-status-error-bg)",
                          border: "1px solid var(--rz-status-error-border)",
                          color: "var(--rz-status-error)",
                        }}
                      >
                        <div className="flex items-center gap-1.5 font-semibold text-[var(--rz-status-error)]">
                          <AlertCircle className="w-3.5 h-3.5" />
                          <span>Missing required dependency</span>
                        </div>
                        <p className="leading-relaxed text-[11px] text-[var(--rz-text)] pl-5">
                          <span className="font-mono font-bold text-[var(--rz-status-error)]">{plan.dependency_report.missing_required.join(", ")}</span> isn't available on your system.
                          Install it with your system package manager before continuing.
                        </p>
                      </div>
                    )}

                    {plan.dependency_report?.missing_optional && plan.dependency_report.missing_optional.length > 0 && plan.dependency_report.missing_required.length === 0 && (
                      <div className="p-2.5 rounded-[8px] bg-amber-400/10 border border-amber-400/20 text-[#e5b85c] text-xs">
                        Optional component(s) not installed: <span className="font-mono">{plan.dependency_report.missing_optional.join(", ")}</span>. You can continue safely with available components.
                      </div>
                    )}

                    {plan.conflicts.length > 0 && (
                      <div className="p-2.5 rounded-[8px] bg-rose-500/10 border border-rose-500/20 text-[#ef7474] text-xs font-mono">
                        Conflicts: {plan.conflicts.join("; ")}
                      </div>
                    )}
                  </div>
                </div>
              ) : null}

              {/* Real Installation Progress */}
              {isInstalling && (
                <div className="p-3.5 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-strong)] space-y-2">
                  <div className="flex items-center justify-between text-xs font-semibold text-[var(--text-primary)]">
                    <span>Installing {selectedPackage.title}...</span>
                    <span className="font-mono text-[var(--text-muted)]">{installProgress}%</span>
                  </div>

                  <div className="w-full h-1.5 rounded-full bg-[var(--bg-surface)] overflow-hidden">
                    <div
                      className="h-full bg-[var(--accent)] transition-all duration-300"
                      style={{ width: `${installProgress}%` }}
                    />
                  </div>

                  <div className="p-2 rounded bg-black/60 font-mono text-[10px] text-[var(--text-muted)] space-y-0.5 max-h-28 overflow-y-auto">
                    {installLogs.map((line, idx) => (
                      <div key={idx}>{line}</div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          ) : (
            /* ─────────────────────────────────────────────────────────────
                DETAILS TABS VIEW (Overview / Manifest / Dependencies)
            ───────────────────────────────────────────────────────────── */
            <div className="space-y-4">
              {/* Screenshots Carousel */}
              {selectedPackage.screenshots && selectedPackage.screenshots.length > 0 && (
                <div className="space-y-2">
                  <div className="aspect-[16/9] w-full rounded-lg overflow-hidden bg-[var(--bg-canvas)] relative shadow-xl">
                    <img
                      src={selectedPackage.screenshots[selectedScreenshotIndex]}
                      alt={selectedPackage.title}
                      className="w-full h-full object-cover"
                    />
                  </div>

                  {selectedPackage.screenshots.length > 1 && (
                    <div className="flex gap-2 overflow-x-auto pb-1">
                      {selectedPackage.screenshots.map((s, idx) => (
                        <button
                          key={idx}
                          onClick={() => setSelectedScreenshotIndex(idx)}
                          className={`w-16 h-10 rounded border overflow-hidden transition-all flex-shrink-0 ${
                            selectedScreenshotIndex === idx
                              ? "border-[var(--accent)]"
                              : "border-[var(--border-subtle)] opacity-60 hover:opacity-100"
                          }`}
                        >
                          <img src={s} alt="" className="w-full h-full object-cover" />
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}

              {/* Navigation Tabs */}
              <div className="flex items-center gap-1 border-b border-[var(--rz-border-subtle)] pb-px">
                <button
                  onClick={() => setActiveTab("overview")}
                  className={`px-3 py-1.5 text-xs font-semibold border-b-2 transition-colors ${
                    activeTab === "overview"
                      ? "border-[var(--rz-accent)] text-[var(--rz-text)]"
                      : "border-transparent text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  Overview
                </button>
                <button
                  onClick={() => setActiveTab("manifest")}
                  className={`px-3 py-1.5 text-xs font-semibold border-b-2 transition-colors ${
                    activeTab === "manifest"
                      ? "border-[var(--rz-accent)] text-[var(--rz-text)]"
                      : "border-transparent text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  Files & Manifest
                </button>
                <button
                  onClick={() => setActiveTab("dependencies")}
                  className={`px-3 py-1.5 text-xs font-semibold border-b-2 transition-colors flex items-center gap-1.5 ${
                    activeTab === "dependencies"
                      ? "border-[var(--rz-accent)] text-[var(--rz-text)]"
                      : "border-transparent text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  <span>Dependencies</span>
                  {compat.missing_required_apps.length > 0 && (
                    <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
                  )}
                </button>
              </div>

              {/* Tab Contents */}
              {activeTab === "overview" && (
                <div className="space-y-4 text-xs">
                  <p className="text-[var(--rz-text)] leading-relaxed text-xs">
                    {selectedPackage.description}
                  </p>

                  {selectedPackage.color_palette && selectedPackage.color_palette.length > 0 && (
                    <div>
                      <div className="text-[11px] font-medium text-[var(--rz-text-secondary)] mb-1.5">Color Palette</div>
                      <div className="flex items-center gap-1.5">
                        {selectedPackage.color_palette.map((c, idx) => (
                          <div
                            key={idx}
                            style={{ backgroundColor: c }}
                            className="w-6 h-6 rounded border border-[var(--rz-border)] shadow-xs"
                            title={c}
                          />
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Provider Provenance */}
                  {selectedPackage.repository_id?.startsWith("provider:") && (
                    <div className="pt-3 border-t border-[var(--rz-border-subtle)] space-y-1.5">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-1.5 font-semibold text-[var(--rz-text)] text-xs">
                          <PackageIcon className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                          <span>Provider Origin: {selectedPackage.repository_id.replace("provider:", "")}</span>
                        </div>
                        <span className="text-[10px] font-mono text-[var(--rz-text-secondary)] px-1.5 py-0.5 rounded-[6px] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                          Declarative Ecosystem
                        </span>
                      </div>
                      <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                        Sourced via native {selectedPackage.repository_id.replace("provider:", "")} integration. Staged lazily on-demand only upon installation, validated against component-aware directory boundaries, and protected by Ryzora's pre-install snapshot and rollback guarantees.
                      </p>
                      {selectedPackage.maintainer && (
                        <div className="text-[11px] text-[var(--rz-text-secondary)] font-mono">
                          Original Author: <span className="text-[var(--rz-text)] font-semibold">{selectedPackage.maintainer}</span>
                        </div>
                      )}
                    </div>
                  )}

                  {/* Repository & Integrity Metadata */}
                  <div className="grid grid-cols-2 sm:grid-cols-5 gap-3 pt-3 border-t border-[var(--rz-border-subtle)] text-[11px]">
                    <div className="space-y-0.5">
                      <div className="text-[11px] font-medium text-[var(--rz-text-muted)]">Source repository</div>
                      <div className="text-[13px] font-semibold text-[var(--rz-text)] truncate">
                        {selectedPackage.repository_id || "Local Community"}
                      </div>
                    </div>

                    <div className="space-y-0.5">
                      <div className="text-[11px] font-medium text-[var(--rz-text-muted)]">Payload status</div>
                      <div className="text-[13px] font-semibold text-[var(--rz-text)] truncate">
                        {selectedPackage.is_cached
                          ? "Cached locally"
                          : "Remote (HTTPS on-demand)"}
                      </div>
                    </div>

                    <div className="space-y-0.5">
                      <div className="text-[11px] font-medium text-[var(--rz-text-muted)]">Release channel</div>
                      <div className="text-[13px] font-semibold text-[var(--rz-text)] capitalize truncate">
                        {selectedPackage.release_channel || "stable"}
                      </div>
                    </div>

                    <div className="space-y-0.5">
                      <div className="text-[11px] font-medium text-[var(--rz-text-muted)]">Store trust tier</div>
                      <div className="text-[13px] font-semibold text-[var(--rz-text)] truncate flex items-center gap-1.5">
                        {selectedPackage.trust_tier === "official" ? (
                          <>
                            <Sparkles className="w-3.5 h-3.5 text-[#e5b85c]" />
                            <span className="text-[#e5b85c]">Official</span>
                          </>
                        ) : selectedPackage.trust_tier === "verified" ? (
                          <>
                            <ShieldCheck className="w-3.5 h-3.5 text-[#7aa2ff]" />
                            <span className="text-[#7aa2ff]">Verified</span>
                          </>
                        ) : selectedPackage.trust_tier === "untrusted" ? (
                          <span className="text-[#ef7474]">Untrusted</span>
                        ) : (
                          <span className="text-[var(--rz-text-secondary)]">Community</span>
                        )}
                      </div>
                    </div>

                    <div className="space-y-0.5">
                      <div className="text-[11px] font-medium text-[var(--rz-text-muted)]">CI moderation</div>
                      <div className="text-[13px] font-semibold text-[var(--rz-text)] truncate">
                        {selectedPackage.moderation_status === "approved" ? (
                          <span className="text-[#6fd6a5]">Approved</span>
                        ) : selectedPackage.moderation_status === "flagged" ? (
                          <span className="text-[#ef7474]">Flagged</span>
                        ) : selectedPackage.moderation_status === "deprecated" ? (
                          <span className="text-[var(--rz-text-muted)]">Deprecated</span>
                        ) : (
                          <span className="text-[#e5b85c]">Pending Review</span>
                        )}
                      </div>
                    </div>
                  </div>

                  {/* Safety & Sandbox Info */}
                  <div className="pt-3 border-t border-[var(--rz-border-subtle)] space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text)]">
                        <ShieldCheck className="w-4 h-4 text-[var(--rz-accent)]" />
                        <span>Declarative Package</span>
                      </div>
                      <span
                        className={`text-[10px] font-mono px-2 py-0.5 rounded-[6px] border ${
                          selectedPackage.integrity_status === "verified"
                            ? "text-[#6fd6a5] border-emerald-400/20 bg-[rgba(111,214,165,0.10)]"
                            : selectedPackage.integrity_status === "corrupted"
                            ? "text-[#ef7474] border-rose-400/20 bg-rose-500/10"
                            : selectedPackage.integrity_status === "pending_download"
                            ? "text-[#7aa2ff] border-sky-400/20 bg-[rgba(88,132,235,0.10)]"
                            : "text-[var(--rz-text-secondary)] border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)]"
                        }`}
                      >
                        {selectedPackage.integrity_status === "verified"
                          ? "SHA-256 Checksum Verified"
                          : selectedPackage.integrity_status === "corrupted"
                          ? "Corrupted Cache"
                          : selectedPackage.integrity_status === "pending_download"
                          ? "SHA-256 Attested"
                          : "No Content Hash"}
                      </span>
                    </div>

                    {selectedPackage.content_hash && (
                      <div className="font-mono text-[10px] text-[var(--rz-text)] truncate bg-[var(--rz-surface-elevated)] px-2.5 py-1.5 rounded-[6px] border border-[var(--rz-border-subtle)]">
                        tree: {selectedPackage.content_hash}
                      </div>
                    )}

                    <ul className="space-y-1.5 text-xs text-[var(--rz-text-secondary)] pt-1">
                      <li className="flex items-center gap-2">
                        <Check className="w-3.5 h-3.5 text-[#6fd6a5] shrink-0" />
                        <span className="text-[var(--rz-text)]">Zero shell scripts — purely declarative configuration files</span>
                      </li>
                      <li className="flex items-center gap-2">
                        <Check className="w-3.5 h-3.5 text-[#6fd6a5] shrink-0" />
                        <span className="text-[var(--rz-text)]">No root privileges required — strictly user-level ~/.config changes</span>
                      </li>
                      <li className="flex items-center gap-2">
                        <Check className="w-3.5 h-3.5 text-[#6fd6a5] shrink-0" />
                        <span className="text-[var(--rz-text)]">
                          {createSnapshot
                            ? "Automatic verified snapshot before file modification"
                            : "Direct safe installation (backup snapshot optional)"}
                        </span>
                      </li>
                    </ul>
                  </div>

                  {/* Cryptographic Trust & Signature */}
                  <div className="pt-3 border-t border-[var(--rz-border-subtle)] space-y-2">
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text)]">
                        <Key className="w-4 h-4 text-[var(--rz-accent)]" />
                        <span>Cryptographic Authenticity & Provenance</span>
                      </div>
                      <span
                        className={`text-[10px] font-mono px-2 py-0.5 rounded-[6px] border ${
                          selectedPackage.cryptographic_status === "official_verified"
                            ? "text-[#e5b85c] border-amber-400/20 bg-[rgba(230,184,92,0.10)]"
                            : selectedPackage.cryptographic_status === "author_verified"
                            ? "text-[#7aa2ff] border-sky-400/20 bg-[rgba(88,132,235,0.10)]"
                            : selectedPackage.cryptographic_status === "self_signed_unvetted"
                            ? "text-purple-300 border-purple-500/20 bg-purple-500/10"
                            : selectedPackage.cryptographic_status === "invalid_signature" ||
                              selectedPackage.cryptographic_status === "revoked_key" ||
                              selectedPackage.cryptographic_status === "official_impersonation"
                            ? "text-[#ef7474] border-rose-500/20 bg-rose-500/10"
                            : "text-[var(--rz-text-secondary)] border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)]"
                        }`}
                      >
                        {selectedPackage.cryptographic_status === "official_verified"
                          ? "Official Core Signed"
                          : selectedPackage.cryptographic_status === "author_verified"
                          ? "Verified Author Signed"
                          : selectedPackage.cryptographic_status === "self_signed_unvetted"
                          ? "Self-Signed (Unvetted)"
                          : selectedPackage.cryptographic_status === "invalid_signature"
                          ? "Invalid Signature"
                          : selectedPackage.cryptographic_status === "revoked_key"
                          ? "Key Revoked"
                          : selectedPackage.cryptographic_status === "official_impersonation"
                          ? "Official Impersonation"
                          : "Unsigned"}
                      </span>
                    </div>

                    {selectedPackage.signature ? (
                      <div className="space-y-1.5 pt-1 text-[11px] font-mono">
                        <div className="flex items-center justify-between text-[var(--rz-text-muted)]">
                          <span>Signer:</span>
                          <span className="text-[var(--rz-text)] font-semibold">
                            {selectedPackage.signature.signer_identity.name}
                            {selectedPackage.signature.signer_identity.handle
                              ? ` (${selectedPackage.signature.signer_identity.handle})`
                              : ""}
                          </span>
                        </div>
                        <div className="flex items-center justify-between text-[var(--rz-text-muted)]">
                          <span>Algorithm / Key ID:</span>
                          <span className="text-[var(--rz-accent-text)] truncate max-w-[200px] font-semibold">
                            {selectedPackage.signature.algorithm.toUpperCase()} · {selectedPackage.signature.key_id}
                          </span>
                        </div>
                        <div className="p-1.5 rounded-[6px] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[10px] text-[var(--rz-text-secondary)] truncate select-all">
                          pubkey: {selectedPackage.signature.public_key}
                        </div>
                        <p className="text-[11px] font-sans text-[var(--rz-text-secondary)] pt-0.5">
                          {selectedPackage.cryptographic_status === "official_verified"
                            ? "Verified against Ryzora Core Root Authority. Guaranteed authentic release."
                            : selectedPackage.cryptographic_status === "author_verified"
                            ? "Verified against Ryzora Community Trust Keyring. Author identity recognized."
                            : "Mathematically authentic Ed25519 signature by author. Key is not yet vetted in the Ryzora Core keyring."}
                        </p>
                      </div>
                    ) : (
                      <p className="text-xs text-[var(--rz-text-secondary)] pt-0.5">
                        This package is unsigned. Permitted as a Community contribution under declarative sandbox rules.
                      </p>
                    )}
                  </div>
                </div>
              )}

              {activeTab === "manifest" && (
                <div className="space-y-3 text-xs">
                  <div className="flex items-center justify-between">
                    <div className="text-[11px] font-medium text-[var(--rz-text-secondary)]">
                      {selectedPackage.manifest
                        ? `Files declared by this package (ryzora_spec v${selectedPackage.manifest.ryzora_spec}):`
                        : "Files installed by this package:"}
                    </div>
                    {selectedPackage.manifest && (
                      <span className="font-mono text-[10px] uppercase text-[var(--accent-text)] bg-[var(--bg-canvas)] border border-[var(--border-subtle)] px-1.5 py-0.5 rounded">
                        {selectedPackage.manifest.package_type}
                      </span>
                    )}
                  </div>

                  <div className="divide-y divide-[var(--rz-border-subtle)] rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] overflow-hidden shadow-xs">
                    {selectedPackage.manifest && selectedPackage.manifest.files.length > 0
                      ? selectedPackage.manifest.files.map((f, idx) => (
                          <div key={idx} className="p-2.5 flex items-start justify-between gap-3 font-mono text-[11px]">
                            <div className="min-w-0">
                              <code className="text-[var(--text-faint)] text-[10px] block truncate">{f.source}</code>
                              <div className="text-[var(--text-muted)] font-sans text-[11px] mt-0.5">{f.description}</div>
                            </div>
                            <div className="flex-shrink-0 flex items-center gap-1.5">
                              <span className="text-[var(--text-faint)] text-[10px]">→</span>
                              <code className="text-[var(--accent-text)] bg-[var(--bg-surface)] px-2 py-0.5 rounded border border-[var(--border-subtle)]">
                                {f.target}
                              </code>
                            </div>
                          </div>
                        ))
                      : selectedPackage.components.map((c, idx) => (
                          <div key={idx} className="p-2.5 flex items-start justify-between gap-3 font-mono text-[11px]">
                            <div>
                              <div className="font-semibold text-[var(--text-primary)] font-sans text-xs">{c.name}</div>
                              <div className="text-[var(--text-faint)] font-sans text-[11px]">{c.description}</div>
                            </div>
                            <code className="text-[var(--accent-text)] bg-[var(--bg-surface)] px-2 py-0.5 rounded border border-[var(--border-subtle)] flex-shrink-0">
                              {c.target_path}
                            </code>
                          </div>
                        ))}
                  </div>

                  <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[11px] text-[var(--rz-text-secondary)]">
                    Declarative manifest · No shell execution · Safe target paths only
                  </div>
                </div>
              )}

              {activeTab === "dependencies" && (
                <div className="space-y-4 text-xs">
                  {loadingDeps ? (
                    <div className="py-8 text-center text-xs text-[var(--text-muted)] space-y-2">
                      <div className="font-mono text-xs animate-pulse">Resolving dependency DAG & probing system capabilities...</div>
                      <div className="text-[11px] text-[var(--text-faint)]">Read-only capability detection · Zero external commands</div>
                    </div>
                  ) : depReport ? (
                    <div className="space-y-4">
                      {/* Resolution Overview Banner */}
                      {depReport.resolved ? (
                        <div className="p-3 rounded-xl bg-[var(--rz-status-success-bg)] border border-[var(--rz-status-success-border)] space-y-1">
                          <div className="flex items-center gap-1.5 text-[var(--rz-status-success)] font-semibold text-xs">
                            <CheckCircle2 className="w-4 h-4 text-[var(--rz-status-success)]" />
                            <span>All Dependencies Satisfied</span>
                          </div>
                          {depReport.install_order.length > 0 && (
                            <div className="text-[11px] text-[var(--rz-text)] font-mono">
                              Topological Install Order: {depReport.install_order.join(" → ")}
                            </div>
                          )}
                        </div>
                      ) : (
                        <div className="space-y-2">
                          {depReport.missing_required.length > 0 && (
                            <div className="p-3 rounded-xl bg-[var(--rz-status-error-bg)] border border-[var(--rz-status-error-border)] space-y-1">
                              <div className="flex items-center gap-1.5 text-[var(--rz-status-error)] font-semibold text-xs">
                                <AlertCircle className="w-4 h-4 text-[var(--rz-status-error)]" />
                                <span>Missing {depReport.missing_required.length} Required Dependency(s)</span>
                              </div>
                              <div className="text-[11px] text-[var(--rz-text)] leading-relaxed">
                                Ryzora strictly avoids executing system package managers. Please install missing tools (
                                <span className="font-mono font-bold text-[var(--rz-status-error)]">{depReport.missing_required.join(", ")}</span>
                                ) via your distribution package manager (pacman, apt, dnf, etc.).
                              </div>
                            </div>
                          )}

                          {depReport.conflicts.length > 0 && (
                            <div className="p-3 rounded-md bg-amber-950/20 border border-amber-800/40 space-y-1">
                              <div className="flex items-center gap-1.5 text-amber-400 font-semibold text-xs">
                                <AlertTriangle className="w-4 h-4 text-amber-400" />
                                <span>Conflicting Requirements</span>
                              </div>
                              <ul className="text-[11px] text-amber-300/90 space-y-0.5 list-disc pl-4 font-mono">
                                {depReport.conflicts.map((c, idx) => (
                                  <li key={idx}>{c}</li>
                                ))}
                              </ul>
                            </div>
                          )}

                          {depReport.cycles.length > 0 && (
                            <div className="p-3 rounded-md bg-rose-950/30 border border-rose-700/50 space-y-1">
                              <div className="flex items-center gap-1.5 text-rose-400 font-semibold text-xs">
                                <RotateCcw className="w-4 h-4 text-rose-400" />
                                <span>Circular Dependency Detected</span>
                              </div>
                              <ul className="text-[11px] text-rose-300 font-mono space-y-0.5">
                                {depReport.cycles.map((cy, idx) => (
                                  <li key={idx}>{cy.join(" → ")}</li>
                                ))}
                              </ul>
                            </div>
                          )}
                        </div>
                      )}

                      {/* Missing Optional Banner */}
                      {depReport.missing_optional.length > 0 && depReport.missing_required.length === 0 && (
                        <div className="p-2.5 rounded-md bg-amber-950/20 border border-amber-800/40 text-amber-300 text-[11px]">
                          Optional tool(s) not found: <span className="font-mono font-medium">{depReport.missing_optional.join(", ")}</span>. The package will install safely with available components.
                        </div>
                      )}

                      {/* Tier 1: Ryzora Packages DAG */}
                      {depReport.packages.length > 0 && (
                        <div className="space-y-1.5">
                          <div className="text-[11px] font-semibold uppercase text-[var(--text-muted)] tracking-wider flex items-center gap-1.5">
                            <PackageIcon className="w-3.5 h-3.5 text-[var(--accent)]" />
                            <span>Package Dependency Graph ({depReport.packages.length})</span>
                          </div>
                          <div className="space-y-1">
                            {depReport.packages.map((pkgNode, idx) => (
                              <div
                                key={idx}
                                className="p-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between"
                              >
                                <div className="space-y-0.5">
                                  <div className="flex items-center gap-2">
                                    <span className="font-mono font-medium text-[var(--text-primary)]">
                                      {pkgNode.name}
                                    </span>
                                    <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-[var(--bg-surface)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
                                      v{pkgNode.version}
                                    </span>
                                    {pkgNode.version_req && (
                                      <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-cyan-950/40 text-cyan-300 border border-cyan-800/30">
                                        req: {pkgNode.version_req}
                                      </span>
                                    )}
                                    {pkgNode.repository_id && (
                                      <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-[var(--bg-surface)] text-[var(--text-faint)]">
                                        repo: {pkgNode.repository_id}
                                      </span>
                                    )}
                                    {pkgNode.effective_required === false && (
                                      <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-amber-950/40 text-amber-300 border border-amber-800/30">
                                        optional
                                      </span>
                                    )}
                                  </div>
                                  {pkgNode.transitive_packages.length > 0 && (
                                    <div className="text-[10px] text-[var(--text-muted)] font-mono">
                                      ↳ depends on: {pkgNode.transitive_packages.join(", ")}
                                    </div>
                                  )}
                                  {pkgNode.status_message && (
                                    <div className="text-[10px] text-rose-300/90 font-mono bg-rose-950/30 px-2 py-0.5 rounded border border-rose-900/40 mt-1">
                                      {pkgNode.status_message}
                                    </div>
                                  )}
                                </div>
                                <div>
                                  {pkgNode.status === "satisfied" ? (
                                    <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[11px]">
                                      <Check className="w-3 h-3" />
                                      Satisfied
                                    </span>
                                  ) : pkgNode.status === "missing" ? (
                                    <span className="inline-flex items-center gap-1 text-rose-400 font-medium text-[11px]">
                                      <AlertCircle className="w-3 h-3" />
                                      Missing
                                    </span>
                                  ) : (
                                    <span className="inline-flex items-center gap-1 text-amber-400 font-medium text-[11px]">
                                      <AlertTriangle className="w-3 h-3" />
                                      Incompatible
                                    </span>
                                  )}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {/* Tier 2: System Binaries & Tools */}
                      {depReport.system_dependencies.length > 0 && (
                        <div className="space-y-1.5">
                          <div className="text-[11px] font-semibold uppercase text-[var(--text-muted)] tracking-wider flex items-center gap-1.5">
                            <Terminal className="w-3.5 h-3.5 text-cyan-400" />
                            <span>System Binaries ({depReport.system_dependencies.length})</span>
                          </div>
                          <div className="space-y-1">
                            {depReport.system_dependencies.map((sysDep, idx) => (
                              <div
                                key={idx}
                                className="p-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between"
                              >
                                <div className="space-y-0.5">
                                  <div className="flex items-center gap-2">
                                    <span className="font-mono text-[var(--text-primary)]">{sysDep.binary}</span>
                                    <span
                                      className={`text-[9px] uppercase px-1 rounded font-semibold ${
                                        sysDep.required
                                          ? "bg-rose-950/40 text-rose-300 border border-rose-800/40"
                                          : "bg-slate-800/60 text-slate-300"
                                      }`}
                                    >
                                      {sysDep.required ? "Required" : "Optional"}
                                    </span>
                                  </div>
                                  {sysDep.path && (
                                    <div className="text-[10px] text-[var(--text-faint)] font-mono">{sysDep.path}</div>
                                  )}
                                  {sysDep.description && (
                                    <div className="text-[10px] text-[var(--text-muted)]">{sysDep.description}</div>
                                  )}
                                </div>
                                <div>
                                  {sysDep.status === "satisfied" ? (
                                    <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[11px]">
                                      <Check className="w-3 h-3" />
                                      Found in PATH
                                    </span>
                                  ) : sysDep.required ? (
                                    <span className="inline-flex items-center gap-1 text-rose-400 font-medium text-[11px]">
                                      <AlertCircle className="w-3 h-3" />
                                      Missing in PATH
                                    </span>
                                  ) : (
                                    <span className="inline-flex items-center gap-1 text-amber-400 text-[11px]">
                                      <AlertCircle className="w-3 h-3" />
                                      Optional Not Found
                                    </span>
                                  )}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}

                      {/* Tier 3: Desktop & Runtime Capabilities */}
                      {depReport.capability_dependencies.length > 0 && (
                        <div className="space-y-1.5">
                          <div className="text-[11px] font-semibold uppercase text-[var(--text-muted)] tracking-wider flex items-center gap-1.5">
                            <Cpu className="w-3.5 h-3.5 text-purple-400" />
                            <span>System Capabilities ({depReport.capability_dependencies.length})</span>
                          </div>
                          <div className="space-y-1">
                            {depReport.capability_dependencies.map((capDep, idx) => (
                              <div
                                key={idx}
                                className="p-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between"
                              >
                                <div className="space-y-0.5">
                                  <div className="flex items-center gap-2">
                                    <span className="font-mono text-[var(--text-primary)]">{capDep.capability}</span>
                                    <span className="text-[9px] uppercase px-1 rounded bg-[var(--bg-surface)] text-[var(--text-faint)]">
                                      {capDep.kind === "desktop_capability" ? "Desktop/Session" : "Runtime"}
                                    </span>
                                  </div>
                                  {capDep.current_value && (
                                    <div className="text-[10px] text-[var(--text-muted)]">{capDep.current_value}</div>
                                  )}
                                </div>
                                <div>
                                  {capDep.status === "satisfied" ? (
                                    <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[11px]">
                                      <Check className="w-3 h-3" />
                                      Satisfied
                                    </span>
                                  ) : (
                                    <span className="inline-flex items-center gap-1 text-rose-400 font-medium text-[11px]">
                                      <AlertCircle className="w-3 h-3" />
                                      Incompatible
                                    </span>
                                  )}
                                </div>
                              </div>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                  ) : (
                    /* Fallback to package manifest dependencies */
                    <div>
                      {depError && (
                        <div className="p-3 mb-3 rounded-md bg-rose-950/30 border border-rose-800/40 space-y-1">
                          <div className="flex items-center gap-1.5 text-rose-400 font-semibold text-xs">
                            <AlertCircle className="w-4 h-4 text-rose-400" />
                            <span>Dependency Resolution Error</span>
                          </div>
                          <div className="text-[11px] text-rose-300/90 font-mono leading-relaxed">
                            {depError}
                          </div>
                          <div className="text-[10px] text-[var(--text-muted)]">
                            One or more configured repositories may be offline, unreachable, or returning invalid metadata.
                          </div>
                        </div>
                      )}
                      <div className="text-[11px] text-[var(--text-muted)] mb-2">
                        Application dependencies required in PATH:
                      </div>
                      <div className="space-y-1.5">
                        {selectedPackage.dependencies.packages.map((dep, idx) => {
                          const isSatisfied = compat.satisfied_apps.includes(dep);
                          return (
                            <div
                              key={idx}
                              className="p-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between text-xs"
                            >
                              <span className="font-mono text-[var(--text-primary)]">{dep}</span>
                              {isSatisfied ? (
                                <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[11px]">
                                  <Check className="w-3 h-3" />
                                  Installed
                                </span>
                              ) : (
                                <span className="inline-flex items-center gap-1 text-amber-400 text-[11px]">
                                  <AlertCircle className="w-3 h-3" />
                                  Missing in PATH
                                </span>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  )}

                  {compat.issues.length > 0 && (
                    <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-1.5">
                      <div className="font-semibold text-[var(--text-primary)] text-xs">Compatibility Warnings</div>
                      <ul className="space-y-1 text-[11px] text-[var(--text-muted)]">
                        {compat.issues.map((iss, idx) => (
                          <li key={idx} className="flex items-start gap-1.5">
                            <span className="text-amber-400 mt-0.5">·</span>
                            <span>{iss.message}</span>
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div
          className="px-5 py-3.5 border-t border-[var(--rz-border-subtle)] flex items-center justify-between gap-4"
          style={{ background: "var(--rz-glass-footer-bg)" }}
        >
          {/* Left: Compatibility Status */}
          <div className="flex items-center gap-2 text-xs">
            {hasMissingRequiredDeps ? (
              <span className="inline-flex items-center gap-1.5 text-[#e98a91] font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-[#ef7474]" />
                Missing required dependency{firstMissingDep ? `: ${firstMissingDep}` : ""}
              </span>
            ) : compat.level === "Compatible" ? (
              <span className="inline-flex items-center gap-1.5 text-[#6fd6a5] font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-[#6fd6a5]" />
                Compatible with your system
              </span>
            ) : compat.level === "MissingDependencies" ? (
              <span className="inline-flex items-center gap-1.5 text-[#e5b85c] font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-[#e5b85c]" />
                {compat.summary_label}
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 text-[var(--rz-text-secondary)] font-medium">
                <span className="w-1.5 h-1.5 rounded-full bg-[var(--rz-text-muted)]" />
                {compat.summary_label}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2.5">
            {installCompleted ? (
              <button
                onClick={handleClose}
                style={{
                  background: "var(--rz-accent-glass)",
                  border: "var(--rz-accent-glass-border)",
                  boxShadow: "var(--rz-accent-glass-shadow)",
                }}
                className="px-4 py-1.5 rounded-[9px] text-xs font-semibold text-white transition-all hover:brightness-110"
              >
                Done
              </button>
            ) : showUninstall ? (
              <>
                {uninstallResult ? (
                  <button
                    onClick={handleClose}
                    className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
                  >
                    Done
                  </button>
                ) : (
                  <>
                    <button
                      onClick={() => setShowUninstall(false)}
                      disabled={isUninstalling}
                      className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                    >
                      Cancel
                    </button>
                    {!isLegacyRecord && (
                      <button
                        onClick={handleConfirmUninstall}
                        disabled={isUninstalling}
                        className="px-4 py-1.5 rounded-md text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-colors disabled:opacity-50 flex items-center gap-1.5"
                      >
                        {isUninstalling && <RefreshCw className="w-3.5 h-3.5 animate-spin" />}
                        <span>{isUninstalling ? "Uninstalling..." : "Confirm Uninstall"}</span>
                      </button>
                    )}
                  </>
                )}
              </>
            ) : showUpdateModal ? (
              <>
                {updateResult ? (
                  <button
                    onClick={handleClose}
                    className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
                  >
                    Done
                  </button>
                ) : (
                  <>
                    <button
                      onClick={() => setShowUpdateModal(false)}
                      disabled={isUpdating}
                      className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                    >
                      Cancel
                    </button>
                    <button
                      onClick={handleConfirmUpdate}
                      disabled={isUpdating || !updatePlan || loadingUpdatePlan}
                      className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors disabled:opacity-50 flex items-center gap-1.5"
                    >
                      {isUpdating && <RefreshCw className="w-3.5 h-3.5 animate-spin" />}
                      <span>{isUpdating ? "Updating..." : "Apply Update"}</span>
                    </button>
                  </>
                )}
              </>
            ) : showPreview ? (
              <>
                <button
                  onClick={() => setShowPreview(false)}
                  disabled={isInstalling}
                  className="px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] transition-colors disabled:opacity-40 shadow-xs"
                >
                  Cancel
                </button>

                {plan && plan.dependency_report && plan.dependency_report.missing_required.length === 0 && plan.dependency_report.missing_optional.length > 0 ? (
                  <button
                    onClick={handleConfirmInstall}
                    disabled={isInstalling || loadingPlan}
                    className="px-4 py-1.5 rounded-[9px] text-xs font-semibold text-white transition-all disabled:opacity-40 shadow-[0_4px_18px_rgba(70,110,220,0.20),inset_0_1px_0_rgba(255,255,255,0.18)] bg-[rgba(88,132,235,0.85)] hover:bg-[rgba(105,145,245,0.95)] border border-[rgba(150,180,255,0.45)] flex items-center gap-1.5"
                  >
                    {isInstalling ? "Installing..." : "Continue with available components"}
                  </button>
                ) : (
                  <button
                    onClick={handleConfirmInstall}
                    disabled={
                      Boolean(
                        isInstalling ||
                          loadingPlan ||
                          hasMissingRequiredDeps ||
                          (plan !== null &&
                            (plan.compatibility_status === "incompatible" ||
                              plan.conflicts.length > 0 ||
                              Boolean(plan.dependency_report && !plan.dependency_report.resolved)))
                      )
                    }
                    style={{
                      backgroundColor: hasMissingRequiredDeps ? "var(--rz-surface-hover)" : "var(--rz-accent)",
                      color: hasMissingRequiredDeps ? "var(--rz-text-muted)" : "#ffffff",
                      border: hasMissingRequiredDeps ? "1px solid var(--rz-border-subtle)" : "1px solid transparent",
                    }}
                    className="px-4 py-1.5 rounded-[9px] text-xs font-semibold transition-all disabled:opacity-40 disabled:cursor-not-allowed shadow-xs"
                    onMouseEnter={(e) => {
                      if (!hasMissingRequiredDeps && !isInstalling) {
                        (e.currentTarget as HTMLElement).style.backgroundColor = "var(--rz-accent-hover)";
                      }
                    }}
                    onMouseLeave={(e) => {
                      if (!hasMissingRequiredDeps) {
                        (e.currentTarget as HTMLElement).style.backgroundColor = "var(--rz-accent)";
                      }
                    }}
                  >
                    {isInstalling
                      ? "Installing..."
                      : hasMissingRequiredDeps
                      ? `Missing: ${firstMissingDep}`
                      : plan?.compatibility_status === "incompatible"
                      ? "Incompatible"
                      : createSnapshot
                      ? "Install with Snapshot"
                      : "Install"}
                  </button>
                )}
              </>
            ) : (
              <>
                {isInstalled ? (
                  <>
                    <button
                      onClick={handleCheckUpdate}
                      disabled={checkingUpdate || isUpdating || isUninstalling}
                      className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] transition-colors disabled:opacity-40 shadow-xs"
                    >
                      <RefreshCw className={`w-3.5 h-3.5 ${checkingUpdate ? "animate-spin" : ""}`} />
                      <span>
                        {updateStatus?.status === "update_available"
                          ? `Update (v${updateStatus.available_version})`
                          : updateStatus?.status === "up_to_date"
                          ? "Up to date"
                          : "Check for Update"}
                      </span>
                    </button>

                    <button
                      onClick={handleOpenUninstall}
                      disabled={isInstalling || isUpdating || isUninstalling}
                      className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-status-error)] hover:text-rose-700 bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/20 transition-colors disabled:opacity-40"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      <span>Uninstall</span>
                    </button>

                    {relatedSnapshot && (
                      <button
                        onClick={() => rollbackSnapshot(relatedSnapshot.id)}
                        disabled={isInstalling}
                        className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] transition-colors disabled:opacity-40 shadow-xs"
                      >
                        <RotateCcw className="w-3 h-3" />
                        <span>Rollback</span>
                      </button>
                    )}

                    <button
                      onClick={handleOpenPreview}
                      disabled={isInstalling || isUpdating || isUninstalling}
                      className="px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] transition-colors disabled:opacity-40 shadow-xs"
                    >
                      Reinstall
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      onClick={handleOpenPreview}
                      disabled={isInstalling}
                      className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-[9px] text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] transition-colors disabled:opacity-40 shadow-xs"
                    >
                      <Eye className="w-3.5 h-3.5 text-[var(--rz-text-secondary)]" />
                      <span>Preview</span>
                    </button>

                    <button
                      onClick={handleOpenPreview}
                      disabled={isInstalling || hasMissingRequiredDeps}
                      style={{
                        backgroundColor: hasMissingRequiredDeps ? "var(--rz-surface-hover)" : "var(--rz-accent)",
                        color: hasMissingRequiredDeps ? "var(--rz-text-muted)" : "#ffffff",
                        border: hasMissingRequiredDeps ? "1px solid var(--rz-border-subtle)" : "1px solid transparent",
                      }}
                      className="px-4 py-1.5 rounded-[9px] text-xs font-semibold transition-all disabled:opacity-40 disabled:cursor-not-allowed shadow-xs"
                      onMouseEnter={(e) => {
                        if (!hasMissingRequiredDeps && !isInstalling) {
                          (e.currentTarget as HTMLElement).style.backgroundColor = "var(--rz-accent-hover)";
                        }
                      }}
                      onMouseLeave={(e) => {
                        if (!hasMissingRequiredDeps) {
                          (e.currentTarget as HTMLElement).style.backgroundColor = "var(--rz-accent)";
                        }
                      }}
                    >
                      {hasMissingRequiredDeps ? `Missing: ${firstMissingDep}` : "Install"}
                    </button>
                  </>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
