import React, { useState } from "react";
import { X, Star, RotateCcw, Check } from "lucide-react";
import { useApp } from "../context/AppContext";

export const PackageDetailModal: React.FC = () => {
  const {
    selectedPackage,
    setSelectedPackage,
    installedPackageIds,
    installPackage,
    isInstalling,
    installProgress,
    installLogs,
    systemInfo,
    snapshots,
    rollbackSnapshot,
  } = useApp();

  const [activeTab, setActiveTab] = useState<"overview" | "manifest" | "dependencies">("overview");
  const [selectedScreenshotIndex, setSelectedScreenshotIndex] = useState<number>(0);

  if (!selectedPackage) return null;

  const isInstalled = installedPackageIds.includes(selectedPackage.id);
  const relatedSnapshot = snapshots.find((s) => s.package_id === selectedPackage.id);

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
          </div>

          <button
            onClick={() => setSelectedPackage(null)}
            className="p-1 rounded text-[var(--text-muted)] hover:text-white hover:bg-[var(--bg-surface)] transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {/* Main Screenshot Preview */}
          <div className="space-y-2">
            <div className="relative aspect-video w-full rounded-md overflow-hidden bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              <img
                src={selectedPackage.screenshots[selectedScreenshotIndex] || selectedPackage.hero_image}
                alt={selectedPackage.title}
                className="w-full h-full object-cover"
              />
            </div>

            {selectedPackage.screenshots.length > 1 && (
              <div className="flex items-center gap-2 overflow-x-auto pb-1">
                {selectedPackage.screenshots.map((shot, idx) => (
                  <button
                    key={idx}
                    onClick={() => setSelectedScreenshotIndex(idx)}
                    className={`relative w-20 h-12 rounded overflow-hidden border transition-all flex-shrink-0 ${
                      selectedScreenshotIndex === idx
                        ? "border-[var(--accent)] opacity-100"
                        : "border-[var(--border-subtle)] opacity-60 hover:opacity-100"
                    }`}
                  >
                    <img src={shot} alt="preview" className="w-full h-full object-cover" />
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Clean Segmented Tab Navigation */}
          <div className="flex items-center gap-1 border-b border-[var(--border-subtle)] pb-2 text-xs">
            <button
              onClick={() => setActiveTab("overview")}
              className={`px-3 py-1 rounded-md transition-colors ${
                activeTab === "overview"
                  ? "bg-[var(--bg-surface-elevated)] text-white font-medium"
                  : "text-[var(--text-muted)] hover:text-white"
              }`}
            >
              Overview
            </button>
            <button
              onClick={() => setActiveTab("manifest")}
              className={`px-3 py-1 rounded-md transition-colors ${
                activeTab === "manifest"
                  ? "bg-[var(--bg-surface-elevated)] text-white font-medium"
                  : "text-[var(--text-muted)] hover:text-white"
              }`}
            >
              Components & Files ({selectedPackage.components.length})
            </button>
            <button
              onClick={() => setActiveTab("dependencies")}
              className={`px-3 py-1 rounded-md transition-colors ${
                activeTab === "dependencies"
                  ? "bg-[var(--bg-surface-elevated)] text-white font-medium"
                  : "text-[var(--text-muted)] hover:text-white"
              }`}
            >
              Dependencies ({selectedPackage.dependencies.packages.length})
            </button>
          </div>

          {/* Tab Content */}
          {activeTab === "overview" && (
            <div className="space-y-4 text-xs">
              <p className="text-[var(--text-muted)] leading-relaxed">
                {selectedPackage.description}
              </p>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
                <div>
                  <div className="text-[10px] text-[var(--text-faint)] uppercase mb-0.5">Author</div>
                  <div className="text-[var(--text-primary)] font-medium truncate">
                    {selectedPackage.author.name}
                  </div>
                </div>

                <div>
                  <div className="text-[10px] text-[var(--text-faint)] uppercase mb-0.5">Rating</div>
                  <div className="text-[var(--text-primary)] font-medium flex items-center gap-1">
                    <Star className="w-3 h-3 fill-amber-400 text-amber-400" />
                    <span>{selectedPackage.rating.toFixed(1)}</span>
                    <span className="text-[var(--text-faint)]">({selectedPackage.rating_count})</span>
                  </div>
                </div>

                <div>
                  <div className="text-[10px] text-[var(--text-faint)] uppercase mb-0.5">Installs</div>
                  <div className="text-[var(--text-primary)] font-medium">
                    {selectedPackage.downloads.toLocaleString()}
                  </div>
                </div>

                <div>
                  <div className="text-[10px] text-[var(--text-faint)] uppercase mb-0.5">Target Desktops</div>
                  <div className="text-[var(--text-primary)] font-medium uppercase truncate">
                    {selectedPackage.supported_desktops.join(", ")}
                  </div>
                </div>
              </div>

              {/* Safety notice */}
              <div className="p-3 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[11px] text-[var(--text-muted)] space-y-1">
                <div className="font-semibold text-[var(--text-primary)]">Pre-install Snapshot</div>
                <div>
                  Installing this package will automatically snapshot modified configuration files in <code className="text-slate-300">~/.config/</code> so you can roll back at any time.
                </div>
              </div>
            </div>
          )}

          {activeTab === "manifest" && (
            <div className="space-y-3 text-xs">
              <div className="text-[11px] text-[var(--text-muted)]">
                Files installed by this package:
              </div>

              <div className="divide-y divide-[var(--border-subtle)] rounded-md border border-[var(--border-subtle)] bg-[var(--bg-canvas)] overflow-hidden">
                {selectedPackage.components.map((c, idx) => (
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
            </div>
          )}

          {activeTab === "dependencies" && (
            <div className="space-y-3 text-xs">
              <div className="text-[11px] text-[var(--text-muted)]">
                Required packages for this configuration:
              </div>

              <div className="space-y-1.5">
                {selectedPackage.dependencies.packages.map((dep, idx) => {
                  const isPresent = systemInfo?.installed_components.some(
                    (ic) => ic.binary === dep && ic.installed
                  );

                  return (
                    <div
                      key={idx}
                      className="p-2 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-between text-xs"
                    >
                      <span className="font-mono text-[var(--text-primary)]">{dep}</span>
                      {isPresent ? (
                        <span className="inline-flex items-center gap-1 text-emerald-400 font-medium text-[11px]">
                          <Check className="w-3 h-3" />
                          Installed
                        </span>
                      ) : (
                        <span className="text-[var(--text-faint)] text-[11px]">
                          Not detected in PATH
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* Installation Progress & Logs */}
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

              <div className="p-2 rounded bg-black/60 font-mono text-[10px] text-[var(--text-muted)] space-y-0.5 max-h-24 overflow-y-auto">
                {installLogs.map((line, idx) => (
                  <div key={idx}>{line}</div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-[var(--border-subtle)] bg-[var(--bg-surface-elevated)] flex items-center justify-between gap-3">
          <div className="text-[11px] text-[var(--text-faint)]">
            Declarative manifest · Safe configuration
          </div>

          <div className="flex items-center gap-2">
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
              onClick={() => installPackage(selectedPackage)}
              disabled={isInstalling}
              className="px-4 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white transition-colors disabled:opacity-50"
            >
              {isInstalling ? "Installing..." : isInstalled ? "Reinstall" : "Install"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
