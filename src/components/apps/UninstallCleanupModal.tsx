/**
 * UninstallCleanupModal.tsx — Phase 26: Safe Uninstall, Cleanup & Storage Management
 *
 * Provides a 3-tier, provider-aware uninstall experience:
 *  1. Normal Uninstall (Default): Removes only the application package.
 *     Cache, user config, and personal files remain untouched.
 *  2. Unused Dependencies (+ Dependencies): Removes only dependencies confirmed unneeded by ALPM.
 *  3. Deep Cleanup (Advanced): Explicitly chosen verified targets (package cache, app cache, config).
 *
 * Strict Safety Guarantee:
 *  - Prominent "What will remain" section confirming personal files are never touched.
 *  - Real calculated byte sizes (no multipliers).
 */

import React, { useState, useEffect, useMemo } from "react";
import {
  Trash2,
  X,
  AlertTriangle,
  CheckCircle2,
  ShieldCheck,
  ChevronDown,
  ChevronUp,
  Loader2,
  Sliders,
  Archive,
  HardDrive,
} from "lucide-react";
import { AppIcon } from "./AppIconResolver.tsx";
import {
  inspectAppCleanup,
  executeAppCleanup,
  formatBytes,
  type AppCleanupPreview,
  type AppCleanupExecutionRequest,
} from "../../services/cleanupService.ts";

interface UninstallCleanupModalProps {
  packageId: string;
  displayName: string;
  providerId: string;
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (reclaimedBytes: number, message: string) => void;
}

export const UninstallCleanupModal: React.FC<UninstallCleanupModalProps> = ({
  packageId,
  displayName,
  providerId,
  isOpen,
  onClose,
  onSuccess,
}) => {
  const [loading, setLoading] = useState<boolean>(true);
  const [preview, setPreview] = useState<AppCleanupPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isAdvanced, setIsAdvanced] = useState<boolean>(false);
  const [executing, setExecuting] = useState<boolean>(false);

  // Advanced Selection State
  const [removePackage, setRemovePackage] = useState<boolean>(true);
  const [removeUnusedDeps, setRemoveUnusedDeps] = useState<boolean>(false);
  const [cleanPackageCache, setCleanPackageCache] = useState<boolean>(false);
  const [cleanAppCache, setCleanAppCache] = useState<boolean>(false);
  const [cleanAppConfig, setCleanAppConfig] = useState<boolean>(false);
  const [cleanAurBuild, setCleanAurBuild] = useState<boolean>(false);
  const [cleanFlatpakData, setCleanFlatpakData] = useState<boolean>(false);

  // Accordion Expand State
  const [expandDeps, setExpandDeps] = useState<boolean>(false);
  const [expandCache, setExpandCache] = useState<boolean>(false);

  useEffect(() => {
    if (!isOpen || !packageId) return;
    setLoading(true);
    setError(null);
    setIsAdvanced(false);

    inspectAppCleanup(packageId, providerId)
      .then((data) => {
        setPreview(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(typeof err === "string" ? err : "Failed to inspect cleanup data.");
        setLoading(false);
      });
  }, [isOpen, packageId, providerId]);

  // Calculate selected total bytes
  const totalSelectedBytes = useMemo(() => {
    if (!preview) return 0;
    let sum = 0;
    if (removePackage && preview.installed_package_bytes) {
      sum += preview.installed_package_bytes;
    }
    if (removeUnusedDeps) {
      sum += preview.total_unused_dependencies_bytes;
    }
    if (cleanPackageCache) {
      sum += preview.total_package_cache_bytes;
    }
    if (cleanAppCache && preview.app_cache_bytes) {
      sum += preview.app_cache_bytes;
    }
    if (cleanAppConfig && preview.app_config_bytes) {
      sum += preview.app_config_bytes;
    }
    if (cleanAurBuild) {
      sum += (preview.aur_build_dir_bytes || 0) + (preview.aur_cache_bytes || 0);
    }
    if (cleanFlatpakData && preview.flatpak_data_bytes) {
      sum += preview.flatpak_data_bytes;
    }
    return sum;
  }, [
    preview,
    removePackage,
    removeUnusedDeps,
    cleanPackageCache,
    cleanAppCache,
    cleanAppConfig,
    cleanAurBuild,
    cleanFlatpakData,
  ]);

  const hasOptionalSelected =
    removeUnusedDeps ||
    cleanPackageCache ||
    cleanAppCache ||
    cleanAppConfig ||
    cleanAurBuild ||
    cleanFlatpakData;

  const handleExecute = async () => {
    setExecuting(true);
    setError(null);

    const req: AppCleanupExecutionRequest = {
      package_id: packageId,
      provider_id: providerId,
      remove_package: removePackage,
      remove_unused_dependencies: isAdvanced ? removeUnusedDeps : false,
      clean_package_cache: isAdvanced ? cleanPackageCache : false,
      clean_app_cache: isAdvanced ? cleanAppCache : false,
      clean_app_config: isAdvanced ? cleanAppConfig : false,
      clean_aur_build: isAdvanced ? cleanAurBuild : false,
      clean_flatpak_data: isAdvanced ? cleanFlatpakData : false,
    };

    try {
      const res = await executeAppCleanup(req);
      if (res.success) {
        onSuccess(res.total_bytes_reclaimed || totalSelectedBytes, res.message);
        onClose();
      } else {
        setError(res.message || "Cleanup failed with errors.");
      }
    } catch (err: any) {
      setError(typeof err === "string" ? err : err?.message || "Execution failed.");
    } finally {
      setExecuting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 sm:p-6 bg-black/75 backdrop-blur-md animate-fadeIn"
      onClick={() => !executing && onClose()}
    >
      <div
        className="w-full max-w-lg rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-5 border-b border-[var(--rz-border-subtle)] flex items-center justify-between">
          <div className="flex items-center gap-3">
            <AppIcon packageId={packageId} size="md" className="rounded-xl shadow-xs" />
            <div>
              <h3 className="text-base font-bold text-[var(--rz-text)] flex items-center gap-2">
                Uninstall {displayName}
              </h3>
              <p className="text-xs text-[var(--rz-text-muted)] font-mono">
                {packageId} · <span className="uppercase">{providerId}</span>
              </p>
            </div>
          </div>
          <button
            type="button"
            disabled={executing}
            onClick={onClose}
            className="p-1.5 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer disabled:opacity-50"
          >
            <X size={18} />
          </button>
        </div>

        {/* Modal Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4 text-xs">
          {loading ? (
            <div className="py-12 flex flex-col items-center justify-center gap-2 text-[var(--rz-text-muted)]">
              <Loader2 size={24} className="animate-spin text-blue-500" />
              <span>Analyzing storage, dependencies, and package archives…</span>
            </div>
          ) : error ? (
            <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/20 text-red-500 flex items-start gap-2.5">
              <AlertTriangle size={18} className="shrink-0 mt-0.5" />
              <p>{error}</p>
            </div>
          ) : preview?.conflicts && preview.conflicts.length > 0 ? (
            /* Conflict Warning Banner */
            <div className="p-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 space-y-2">
              <div className="flex items-center gap-2 font-bold">
                <AlertTriangle size={16} />
                <span>Cannot Safely Uninstall</span>
              </div>
              <p className="text-[11px] leading-relaxed opacity-90">
                This package is required by other installed applications on your system. Removing it
                would break dependencies:
              </p>
              <ul className="list-disc pl-4 space-y-1 font-mono text-[11px]">
                {preview.conflicts.map((c, i) => (
                  <li key={i}>{c}</li>
                ))}
              </ul>
            </div>
          ) : !isAdvanced ? (
            /* ─────────────────────────────────────────────────────────────
               MODE 1: NORMAL UNINSTALL (DEFAULT)
               ───────────────────────────────────────────────────────────── */
            <div className="space-y-4">
              <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                The application will be removed from your system. Package archives, user
                configuration, and personal files will be preserved.
              </p>

              {/* Status Ledger */}
              <div className="rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] overflow-hidden divide-y divide-[var(--rz-border-subtle)]">
                <div className="flex items-center justify-between p-3">
                  <div className="flex items-center gap-2.5">
                    <Trash2 size={15} className="text-rose-500" />
                    <span className="font-semibold text-[var(--rz-text)]">Application files</span>
                  </div>
                  <span className="font-mono font-bold text-rose-500">
                    {formatBytes(preview?.installed_package_bytes)}
                  </span>
                </div>

                <div className="flex items-center justify-between p-3">
                  <div className="flex items-center gap-2.5">
                    <CheckCircle2 size={15} className="text-emerald-500" />
                    <span className="text-[var(--rz-text-secondary)]">Dependencies</span>
                  </div>
                  <span className="font-semibold text-emerald-600 dark:text-emerald-400">Kept</span>
                </div>

                <div className="flex items-center justify-between p-3">
                  <div className="flex items-center gap-2.5">
                    <Archive size={15} className="text-blue-500" />
                    <span className="text-[var(--rz-text-secondary)]">Package cache</span>
                  </div>
                  <span className="font-semibold text-blue-600 dark:text-blue-400">
                    Kept · {formatBytes(preview?.total_package_cache_bytes)}
                  </span>
                </div>

                <div className="flex items-center justify-between p-3">
                  <div className="flex items-center gap-2.5">
                    <ShieldCheck size={15} className="text-emerald-500" />
                    <span className="text-[var(--rz-text-secondary)]">User configuration & cache</span>
                  </div>
                  <span className="font-semibold text-emerald-600 dark:text-emerald-400">Kept</span>
                </div>

                <div className="flex items-center justify-between p-3 bg-emerald-500/5">
                  <div className="flex items-center gap-2.5">
                    <ShieldCheck size={15} className="text-emerald-500" />
                    <span className="font-medium text-emerald-700 dark:text-emerald-300">
                      Personal files & projects
                    </span>
                  </div>
                  <span className="font-bold text-emerald-600 dark:text-emerald-400">Kept</span>
                </div>
              </div>

              {/* Advanced Cleanup Trigger */}
              <div className="pt-1 flex items-center justify-between">
                <button
                  type="button"
                  onClick={() => setIsAdvanced(true)}
                  className="inline-flex items-center gap-1.5 text-xs text-blue-600 dark:text-blue-400 hover:underline font-semibold cursor-pointer"
                >
                  <Sliders size={13} />
                  <span>Advanced cleanup (dependencies, cache, config)…</span>
                </button>
              </div>
            </div>
          ) : (
            /* ─────────────────────────────────────────────────────────────
               MODE 2: ADVANCED / DEEP CLEANUP
               ───────────────────────────────────────────────────────────── */
            <div className="space-y-3.5">
              <div className="flex items-center justify-between">
                <span className="font-bold text-[var(--rz-text)]">Optional Cleanup Targets</span>
                <span className="text-[11px] text-[var(--rz-text-muted)]">
                  Only safe, verified items
                </span>
              </div>

              {/* Checklist */}
              <div className="space-y-2">
                {/* 1. Package itself */}
                <label className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] cursor-pointer">
                  <div className="flex items-center gap-2.5">
                    <input
                      type="checkbox"
                      checked={removePackage}
                      onChange={(e) => setRemovePackage(e.target.checked)}
                      className="rounded accent-rose-500 cursor-pointer"
                    />
                    <div>
                      <div className="font-semibold text-[var(--rz-text)]">Application package</div>
                      <div className="text-[11px] text-[var(--rz-text-muted)]">
                        Installed binary files and desktop launcher
                      </div>
                    </div>
                  </div>
                  <span className="font-mono font-bold text-[var(--rz-text)]">
                    {formatBytes(preview?.installed_package_bytes)}
                  </span>
                </label>

                {/* 2. Unused Dependencies */}
                {preview?.unused_dependencies && preview.unused_dependencies.length > 0 && (
                  <div className="rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] overflow-hidden">
                    <label className="flex items-center justify-between p-3 cursor-pointer">
                      <div className="flex items-center gap-2.5">
                        <input
                          type="checkbox"
                          checked={removeUnusedDeps}
                          onChange={(e) => setRemoveUnusedDeps(e.target.checked)}
                          className="rounded accent-rose-500 cursor-pointer"
                        />
                        <div>
                          <div className="font-semibold text-[var(--rz-text)]">
                            Remove unused dependencies
                          </div>
                          <div className="text-[11px] text-[var(--rz-text-muted)]">
                            {preview.unused_dependencies.length} package(s) no longer required elsewhere
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-[var(--rz-text)]">
                          {formatBytes(preview.total_unused_dependencies_bytes)}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.preventDefault();
                            setExpandDeps((p) => !p);
                          }}
                          className="p-1 rounded text-[var(--rz-text-muted)] hover:text-[var(--rz-text)]"
                        >
                          {expandDeps ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                        </button>
                      </div>
                    </label>

                    {expandDeps && (
                      <div className="p-3 border-t border-[var(--rz-border-subtle)] bg-[var(--rz-bg)]/40 space-y-1 font-mono text-[11px]">
                        {preview.unused_dependencies.map((d) => (
                          <div key={d.name} className="flex items-center justify-between text-[var(--rz-text-secondary)]">
                            <span>{d.name}</span>
                            <span>{formatBytes(d.size_bytes)}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* 3. Package Cache Archives */}
                {preview?.package_cache_items && preview.package_cache_items.length > 0 && (
                  <div className="rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] overflow-hidden">
                    <label className="flex items-center justify-between p-3 cursor-pointer">
                      <div className="flex items-center gap-2.5">
                        <input
                          type="checkbox"
                          checked={cleanPackageCache}
                          onChange={(e) => setCleanPackageCache(e.target.checked)}
                          className="rounded accent-rose-500 cursor-pointer"
                        />
                        <div>
                          <div className="font-semibold text-[var(--rz-text)]">
                            Delete package cache
                          </div>
                          <div className="text-[11px] text-[var(--rz-text-muted)]">
                            {preview.package_cache_items.length} downloaded archive(s) in /var/cache/pacman/pkg
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-[var(--rz-text)]">
                          {formatBytes(preview.total_package_cache_bytes)}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.preventDefault();
                            setExpandCache((p) => !p);
                          }}
                          className="p-1 rounded text-[var(--rz-text-muted)] hover:text-[var(--rz-text)]"
                        >
                          {expandCache ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                        </button>
                      </div>
                    </label>

                    {expandCache && (
                      <div className="p-3 border-t border-[var(--rz-border-subtle)] bg-[var(--rz-bg)]/40 space-y-1 font-mono text-[11px]">
                        {preview.package_cache_items.map((c) => (
                          <div key={c.filename} className="flex items-center justify-between text-[var(--rz-text-secondary)]">
                            <span className="truncate max-w-[280px]" title={c.filename}>
                              {c.filename}
                            </span>
                            <span>{formatBytes(c.size_bytes)}</span>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}

                {/* 4. Application Cache */}
                {preview?.app_cache_bytes !== undefined && preview.app_cache_bytes !== null && preview.app_cache_bytes > 0 && (
                  <label className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="checkbox"
                        checked={cleanAppCache}
                        onChange={(e) => setCleanAppCache(e.target.checked)}
                        className="rounded accent-rose-500 cursor-pointer"
                      />
                      <div>
                        <div className="font-semibold text-[var(--rz-text)]">Application cache</div>
                        <div className="text-[11px] text-[var(--rz-text-muted)]">
                          Temporary data in {preview.app_cache_path}
                        </div>
                      </div>
                    </div>
                    <span className="font-mono font-bold text-[var(--rz-text)]">
                      {formatBytes(preview.app_cache_bytes)}
                    </span>
                  </label>
                )}

                {/* 5. Application Configuration */}
                {preview?.app_config_bytes !== undefined && preview.app_config_bytes !== null && preview.app_config_bytes > 0 && (
                  <label className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="checkbox"
                        checked={cleanAppConfig}
                        onChange={(e) => setCleanAppConfig(e.target.checked)}
                        className="rounded accent-rose-500 cursor-pointer"
                      />
                      <div>
                        <div className="font-semibold text-[var(--rz-text)]">Application settings</div>
                        <div className="text-[11px] text-[var(--rz-text-muted)]">
                          Preferences and hotkeys in {preview.app_config_path}
                        </div>
                      </div>
                    </div>
                    <span className="font-mono font-bold text-[var(--rz-text)]">
                      {formatBytes(preview.app_config_bytes)}
                    </span>
                  </label>
                )}

                {/* 6. AUR Build Directory */}
                {providerId === "aur" && (preview?.aur_build_dir_bytes || preview?.aur_cache_bytes) && (
                  <label className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="checkbox"
                        checked={cleanAurBuild}
                        onChange={(e) => setCleanAurBuild(e.target.checked)}
                        className="rounded accent-rose-500 cursor-pointer"
                      />
                      <div>
                        <div className="font-semibold text-[var(--rz-text)]">AUR build files & cache</div>
                        <div className="text-[11px] text-[var(--rz-text-muted)]">
                          Unprivileged build repository and helper artifacts
                        </div>
                      </div>
                    </div>
                    <span className="font-mono font-bold text-[var(--rz-text)]">
                      {formatBytes((preview.aur_build_dir_bytes || 0) + (preview.aur_cache_bytes || 0))}
                    </span>
                  </label>
                )}

                {/* 7. Flatpak Data */}
                {providerId === "flatpak" && preview?.flatpak_data_bytes && (
                  <label className="flex items-center justify-between p-3 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <input
                        type="checkbox"
                        checked={cleanFlatpakData}
                        onChange={(e) => setCleanFlatpakData(e.target.checked)}
                        className="rounded accent-rose-500 cursor-pointer"
                      />
                      <div>
                        <div className="font-semibold text-[var(--rz-text)]">Flatpak application data</div>
                        <div className="text-[11px] text-[var(--rz-text-muted)]">
                          Sandbox storage in {preview.flatpak_data_path}
                        </div>
                      </div>
                    </div>
                    <span className="font-mono font-bold text-[var(--rz-text)]">
                      {formatBytes(preview.flatpak_data_bytes)}
                    </span>
                  </label>
                )}
              </div>

              {/* Flatpak Runtime Notice */}
              {preview?.flatpak_runtime_info && (
                <div className="p-3 rounded-xl bg-blue-500/10 border border-blue-500/20 text-blue-600 dark:text-blue-400 flex items-center gap-2">
                  <HardDrive size={15} />
                  <span>Flatpak Runtime: {preview.flatpak_runtime_info}</span>
                </div>
              )}

              {/* ── Reassurance Banner: What will remain ── */}
              <div className="p-3 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-700 dark:text-emerald-300 flex items-start gap-2.5">
                <ShieldCheck size={16} className="shrink-0 mt-0.5 text-emerald-500" />
                <div className="space-y-0.5">
                  <span className="font-bold">Will remain safe</span>
                  <p className="text-[11px] opacity-90 leading-relaxed">
                    Personal documents, projects, downloads, photos, videos, and files not explicitly selected will never be touched.
                  </p>
                </div>
              </div>

              {/* Potential Cleanup Counter */}
              {hasOptionalSelected && (
                <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] flex items-center justify-between">
                  <span className="font-semibold text-[var(--rz-text-muted)]">Potential cleanup</span>
                  <span className="font-mono font-bold text-base text-blue-600 dark:text-blue-400">
                    {formatBytes(totalSelectedBytes)}
                  </span>
                </div>
              )}

              {/* Return to standard mode */}
              <div className="pt-1">
                <button
                  type="button"
                  onClick={() => setIsAdvanced(false)}
                  className="text-xs text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] underline cursor-pointer"
                >
                  ← Back to standard uninstall
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer */}
        <div className="p-4 border-t border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] flex items-center justify-end gap-3">
          <button
            type="button"
            disabled={executing}
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-colors cursor-pointer disabled:opacity-50"
          >
            Cancel
          </button>

          <button
            type="button"
            disabled={
              loading ||
              executing ||
              Boolean(preview?.conflicts && preview.conflicts.length > 0) ||
              (!removePackage && !hasOptionalSelected)
            }
            onClick={handleExecute}
            className="inline-flex items-center gap-2 px-5 py-2 rounded-xl text-xs font-semibold bg-rose-600 hover:bg-rose-500 text-white transition-all shadow-md shadow-rose-600/20 cursor-pointer disabled:opacity-50"
          >
            {executing ? (
              <>
                <Loader2 size={14} className="animate-spin" />
                <span>Cleaning up…</span>
              </>
            ) : isAdvanced && hasOptionalSelected ? (
              <>
                <Trash2 size={14} />
                <span>Remove & Clean ({formatBytes(totalSelectedBytes)})</span>
              </>
            ) : (
              <>
                <Trash2 size={14} />
                <span>Uninstall</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
