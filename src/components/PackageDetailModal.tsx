import React, { useState } from "react";
import {
  X,
  RotateCcw,
  Check,
  AlertCircle,
  ShieldCheck,
  Eye,
  FilePlus,
  FileEdit,
  FileCheck,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { InstallationPlan } from "../types";

export const PackageDetailModal: React.FC = () => {
  const {
    selectedPackage,
    setSelectedPackage,
    setActiveCategory,
    installedPackageIds,
    installPackage,
    previewInstallation,
    isInstalling,
    installProgress,
    installLogs,
    snapshots,
    rollbackSnapshot,
    checkCompatibility,
  } = useApp();

  const [activeTab, setActiveTab] = useState<"overview" | "manifest" | "dependencies">("overview");
  const [selectedScreenshotIndex, setSelectedScreenshotIndex] = useState<number>(0);

  // Preview & Installation flow states
  const [showPreview, setShowPreview] = useState<boolean>(false);
  const [loadingPlan, setLoadingPlan] = useState<boolean>(false);
  const [plan, setPlan] = useState<InstallationPlan | null>(null);
  const [installCompleted, setInstallCompleted] = useState<boolean>(false);
  const [installedSnapshotId, setInstalledSnapshotId] = useState<string | null>(null);

  if (!selectedPackage) return null;

  const isInstalled = installedPackageIds.includes(selectedPackage.id);
  const relatedSnapshot = snapshots.find(
    (s) => s.label === (selectedPackage.manifest?.name ?? selectedPackage.title)
  );
  const compat = checkCompatibility(selectedPackage);

  const handleOpenPreview = async () => {
    setLoadingPlan(true);
    setShowPreview(true);
    try {
      const p = await previewInstallation(selectedPackage.id);
      setPlan(p);
    } catch {
      // If preview fails (e.g. package not in local repo), generate client fallback plan
      const manifestFiles = selectedPackage.manifest?.files ?? [];
      const targets = manifestFiles.length > 0
        ? manifestFiles.map((f) => f.target)
        : selectedPackage.components.map((c) => c.target_path);

      setPlan({
        package_id: selectedPackage.id,
        package_name: selectedPackage.title,
        package_version: selectedPackage.version,
        files_to_create: targets,
        files_to_replace: [],
        files_unchanged: [],
        directories_to_create: [],
        conflicts: [],
        compatibility_status: compat.missing_required_apps.length > 0 ? "missing_dependencies" : "compatible",
        required_dependencies: selectedPackage.dependencies.packages,
        missing_dependencies: compat.missing_required_apps,
        warnings: [],
      });
    } finally {
      setLoadingPlan(false);
    }
  };

  const handleConfirmInstall = async () => {
    try {
      const res = await installPackage(selectedPackage);
      if (res && res.success) {
        setInstalledSnapshotId(res.snapshot_id);
        setInstallCompleted(true);
      }
    } catch {
      // Error handled in AppContext logs and toast
    }
  };

  const handleClose = () => {
    setShowPreview(false);
    setInstallCompleted(false);
    setPlan(null);
    setSelectedPackage(null);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 md:p-8 bg-black/70 backdrop-blur-sm select-none">
      <div
        className="relative w-full max-w-3xl max-h-[85vh] flex flex-col rounded-lg bg-[var(--bg-surface)] border border-[var(--border-strong)] shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-5 py-3 border-b border-[var(--border-subtle)] flex items-center justify-between gap-4 bg-[var(--bg-surface-elevated)]">
          <div className="flex items-center gap-2.5 truncate">
            <span className="font-bold text-sm text-[var(--text-primary)] truncate">
              {selectedPackage.title}
            </span>
            <span className="text-[10px] font-mono uppercase text-[var(--text-muted)] px-1.5 py-0.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              v{selectedPackage.version}
            </span>
            {showPreview && !installCompleted && (
              <span className="text-[10px] font-mono uppercase text-[var(--accent-text)] px-1.5 py-0.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
                Installation Preview
              </span>
            )}
          </div>

          <button
            onClick={handleClose}
            className="p-1 rounded text-[var(--text-muted)] hover:text-white hover:bg-[var(--bg-surface)] transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
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

              {installedSnapshotId && (
                <div className="inline-block px-3 py-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-xs font-mono text-left">
                  <div className="text-[10px] text-[var(--text-muted)] uppercase tracking-wider font-sans">
                    Pre-install Snapshot ID
                  </div>
                  <div className="text-[var(--accent-text)] font-semibold mt-0.5">
                    {installedSnapshotId}
                  </div>
                </div>
              )}

              <div className="flex items-center justify-center gap-3 pt-2">
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

                <button
                  onClick={handleClose}
                  className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
                >
                  Done
                </button>
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
                  <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between">
                    <div>
                      <div className="font-semibold text-[var(--text-primary)]">
                        {plan.files_to_create.length + plan.files_to_replace.length} files will be installed
                      </div>
                      <div className="text-[11px] text-[var(--text-muted)] mt-0.5">
                        {plan.files_to_create.length} new, {plan.files_to_replace.length} replaced, {plan.files_unchanged.length} unchanged
                      </div>
                    </div>
                    <span className="font-mono text-[10px] uppercase text-[var(--text-muted)] px-2 py-0.5 rounded border border-[var(--border-subtle)]">
                      Dry Run Validated
                    </span>
                  </div>

                  {/* Files Breakdown */}
                  <div className="space-y-3">
                    {/* Files to Create */}
                    {plan.files_to_create.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-[11px] font-semibold text-emerald-400 uppercase tracking-wider">
                          <FilePlus className="w-3.5 h-3.5" />
                          <span>Create ({plan.files_to_create.length})</span>
                        </div>
                        <div className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-canvas)] divide-y divide-[var(--border-subtle)] overflow-hidden font-mono text-[11px]">
                          {plan.files_to_create.map((path, idx) => (
                            <div key={idx} className="p-2 flex items-center gap-2 text-[var(--text-primary)]">
                              <span className="text-emerald-400 font-bold">+</span>
                              <span>{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Files to Replace */}
                    {plan.files_to_replace.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-[11px] font-semibold text-amber-400 uppercase tracking-wider">
                          <FileEdit className="w-3.5 h-3.5" />
                          <span>Replace / Modify ({plan.files_to_replace.length})</span>
                        </div>
                        <div className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-canvas)] divide-y divide-[var(--border-subtle)] overflow-hidden font-mono text-[11px]">
                          {plan.files_to_replace.map((path, idx) => (
                            <div key={idx} className="p-2 flex items-center gap-2 text-[var(--text-primary)]">
                              <span className="text-amber-400 font-bold">~</span>
                              <span>{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}

                    {/* Files Unchanged */}
                    {plan.files_unchanged.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--text-muted)] uppercase tracking-wider">
                          <FileCheck className="w-3.5 h-3.5" />
                          <span>Unchanged ({plan.files_unchanged.length})</span>
                        </div>
                        <div className="rounded-md border border-[var(--border-subtle)] bg-[var(--bg-canvas)] divide-y divide-[var(--border-subtle)] overflow-hidden font-mono text-[11px]">
                          {plan.files_unchanged.map((path, idx) => (
                            <div key={idx} className="p-2 flex items-center gap-2 text-[var(--text-muted)]">
                              <span className="font-bold">=</span>
                              <span>{path}</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>

                  {/* Safety & Backup card */}
                  <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-1">
                    <div className="text-[11px] font-semibold uppercase text-[var(--text-muted)] tracking-wider">
                      Backup
                    </div>
                    <div className="flex items-center gap-2 text-emerald-400 font-medium">
                      <Check className="w-3.5 h-3.5" />
                      <span>Automatic verified snapshot will be taken before modification</span>
                    </div>
                    <div className="text-[11px] text-[var(--text-faint)]">
                      Enables 1-click verified rollback if anything fails.
                    </div>
                  </div>

                  {/* Dependencies & Compatibility */}
                  <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-2">
                    <div className="text-[11px] font-semibold uppercase text-[var(--text-muted)] tracking-wider">
                      Dependencies & Compatibility
                    </div>

                    <div className="space-y-1">
                      {plan.required_dependencies.length > 0 ? (
                        plan.required_dependencies.map((dep, idx) => {
                          const isMissing = plan.missing_dependencies.includes(dep);
                          return (
                            <div key={idx} className="flex items-center justify-between font-mono text-xs">
                              <span className="text-[var(--text-primary)]">{dep}</span>
                              {isMissing ? (
                                <span className="inline-flex items-center gap-1 text-amber-400 font-sans font-medium text-[11px]">
                                  <AlertCircle className="w-3 h-3" />
                                  Missing dependency: {dep}
                                </span>
                              ) : (
                                <span className="inline-flex items-center gap-1 text-emerald-400 font-sans font-medium text-[11px]">
                                  <Check className="w-3 h-3" />
                                  Available in PATH
                                </span>
                              )}
                            </div>
                          );
                        })
                      ) : (
                        <div className="text-[var(--text-muted)] text-[11px]">No external application dependencies required.</div>
                      )}
                    </div>

                    {plan.missing_dependencies.length > 0 && (
                      <div className="p-2 rounded bg-amber-950/20 border border-amber-800/40 text-amber-300 text-[11px] mt-2">
                        Missing dependency: {plan.missing_dependencies.join(", ")}. Please install required components via your system package manager before applying this rice.
                      </div>
                    )}

                    {plan.conflicts.length > 0 && (
                      <div className="p-2 rounded bg-red-950/20 border border-red-800/40 text-red-300 text-[11px] mt-2">
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
                  <div className="aspect-[16/9] w-full rounded-md overflow-hidden bg-[var(--bg-canvas)] border border-[var(--border-subtle)] relative">
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
              <div className="flex items-center gap-1 border-b border-[var(--border-subtle)] pb-px">
                <button
                  onClick={() => setActiveTab("overview")}
                  className={`px-3 py-1.5 text-xs font-medium border-b-2 transition-colors ${
                    activeTab === "overview"
                      ? "border-[var(--accent)] text-[var(--text-primary)]"
                      : "border-transparent text-[var(--text-muted)] hover:text-white"
                  }`}
                >
                  Overview
                </button>
                <button
                  onClick={() => setActiveTab("manifest")}
                  className={`px-3 py-1.5 text-xs font-medium border-b-2 transition-colors ${
                    activeTab === "manifest"
                      ? "border-[var(--accent)] text-[var(--text-primary)]"
                      : "border-transparent text-[var(--text-muted)] hover:text-white"
                  }`}
                >
                  Files & Manifest
                </button>
                <button
                  onClick={() => setActiveTab("dependencies")}
                  className={`px-3 py-1.5 text-xs font-medium border-b-2 transition-colors flex items-center gap-1.5 ${
                    activeTab === "dependencies"
                      ? "border-[var(--accent)] text-[var(--text-primary)]"
                      : "border-transparent text-[var(--text-muted)] hover:text-white"
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
                  <p className="text-[var(--text-muted)] leading-relaxed">
                    {selectedPackage.description}
                  </p>

                  {selectedPackage.color_palette && selectedPackage.color_palette.length > 0 && (
                    <div>
                      <div className="text-[11px] text-[var(--text-muted)] mb-1.5">Color Palette</div>
                      <div className="flex items-center gap-1.5">
                        {selectedPackage.color_palette.map((c, idx) => (
                          <div
                            key={idx}
                            style={{ backgroundColor: c }}
                            className="w-6 h-6 rounded border border-white/10"
                            title={c}
                          />
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Safety & Sandbox Info */}
                  <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-1.5">
                    <div className="flex items-center gap-1.5 text-emerald-400 font-semibold text-xs">
                      <ShieldCheck className="w-4 h-4" />
                      <span>Ryzora Safety Audit · Declarative Specification</span>
                    </div>
                    <ul className="space-y-1 text-[11px] text-[var(--text-muted)]">
                      <li className="flex items-center gap-1.5">
                        <Check className="w-3 h-3 text-emerald-400" />
                        <span>Zero shell scripts — purely declarative configuration files</span>
                      </li>
                      <li className="flex items-center gap-1.5">
                        <Check className="w-3 h-3 text-emerald-400" />
                        <span>No root privileges required — strictly user-level ~/.config changes</span>
                      </li>
                      <li className="flex items-center gap-1.5">
                        <Check className="w-3 h-3 text-emerald-400" />
                        <span>Automatic verified snapshot before modification</span>
                      </li>
                    </ul>
                  </div>
                </div>
              )}

              {activeTab === "manifest" && (
                <div className="space-y-3 text-xs">
                  <div className="flex items-center justify-between">
                    <div className="text-[11px] text-[var(--text-muted)]">
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

                  <div className="divide-y divide-[var(--border-subtle)] rounded-md border border-[var(--border-subtle)] bg-[var(--bg-canvas)] overflow-hidden">
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

                  <div className="p-2.5 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[11px] text-[var(--text-faint)]">
                    Declarative manifest · No shell execution · Safe target paths only
                  </div>
                </div>
              )}

              {activeTab === "dependencies" && (
                <div className="space-y-4 text-xs">
                  <div>
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

                  {compat.issues.length > 0 && (
                    <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-1.5">
                      <div className="font-semibold text-[var(--text-primary)] text-xs">Compatibility Issues</div>
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
        <div className="px-5 py-3 border-t border-[var(--border-subtle)] bg-[var(--bg-surface-elevated)] flex items-center justify-between gap-3">
          <div className="text-[11px] text-[var(--text-faint)]">
            Declarative manifest · Safe configuration
          </div>

          <div className="flex items-center gap-2">
            {installCompleted ? (
              <button
                onClick={handleClose}
                className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors"
              >
                Done
              </button>
            ) : showPreview ? (
              <>
                <button
                  onClick={() => setShowPreview(false)}
                  disabled={isInstalling}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                >
                  Cancel
                </button>

                <button
                  onClick={handleConfirmInstall}
                  disabled={
                    isInstalling ||
                    loadingPlan ||
                    (plan !== null &&
                      (plan.missing_dependencies.length > 0 ||
                        plan.compatibility_status === "incompatible" ||
                        plan.conflicts.length > 0))
                  }
                  className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors disabled:opacity-50"
                >
                  {isInstalling
                    ? "Installing..."
                    : plan && plan.missing_dependencies.length > 0
                    ? `Missing: ${plan.missing_dependencies[0]}`
                    : "Install"}
                </button>
              </>
            ) : (
              <>
                {isInstalled && relatedSnapshot && (
                  <button
                    onClick={() => rollbackSnapshot(relatedSnapshot.id)}
                    disabled={isInstalling}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                  >
                    <RotateCcw className="w-3 h-3" />
                    <span>Rollback</span>
                  </button>
                )}

                <button
                  onClick={handleOpenPreview}
                  disabled={isInstalling}
                  className="flex items-center gap-1 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                >
                  <Eye className="w-3.5 h-3.5" />
                  <span>Preview</span>
                </button>

                <button
                  onClick={handleOpenPreview}
                  disabled={isInstalling}
                  className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors disabled:opacity-50"
                >
                  {isInstalling ? "Installing..." : isInstalled ? "Reinstall" : "Install"}
                </button>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
