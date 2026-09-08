import React, { useState } from "react";
import {
  X,
  Star,
  Download,
  ShieldCheck,
  CheckCircle2,
  Layers,
  History,
  RotateCcw,
} from "lucide-react";
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

  const [activeTab, setActiveTab] = useState<"overview" | "manifest" | "safety">("overview");
  const [selectedScreenshotIndex, setSelectedScreenshotIndex] = useState<number>(0);

  if (!selectedPackage) return null;

  const isInstalled = installedPackageIds.includes(selectedPackage.id);

  // Find most recent snapshot for this package if installed
  const relatedSnapshot = snapshots.find((s) => s.package_id === selectedPackage.id);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 md:p-8 bg-black/80 backdrop-blur-md animate-in fade-in duration-200">
      <div
        className="relative w-full max-w-4xl max-h-[90vh] flex flex-col rounded-3xl bg-[#0b0f19] border border-slate-800 shadow-2xl overflow-hidden"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Top Header */}
        <div className="p-6 pb-4 border-b border-slate-800/80 flex items-center justify-between gap-4 bg-slate-900/50">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <span className="px-2.5 py-0.5 rounded-full text-[11px] font-bold uppercase tracking-wider bg-cyan-500/15 text-cyan-300 border border-cyan-500/30">
                {selectedPackage.category}
              </span>
              <span className="px-2 py-0.5 rounded-full text-[11px] font-mono bg-slate-800 text-slate-300 border border-slate-700">
                v{selectedPackage.version}
              </span>
            </div>
            <h2 className="text-xl font-bold text-white truncate max-w-md">
              {selectedPackage.title}
            </h2>
          </div>

          <button
            onClick={() => setSelectedPackage(null)}
            className="p-2 rounded-xl text-slate-400 hover:text-white hover:bg-slate-800 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Modal Body (Scrollable) */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Screenshot Showcase */}
          <div className="space-y-3">
            <div className="relative aspect-video w-full rounded-2xl overflow-hidden bg-slate-950 border border-slate-800">
              <img
                src={selectedPackage.screenshots[selectedScreenshotIndex] || selectedPackage.hero_image}
                alt={selectedPackage.title}
                className="w-full h-full object-cover"
              />
            </div>

            {selectedPackage.screenshots.length > 1 && (
              <div className="flex items-center gap-3 overflow-x-auto pb-1">
                {selectedPackage.screenshots.map((shot, idx) => (
                  <button
                    key={idx}
                    onClick={() => setSelectedScreenshotIndex(idx)}
                    className={`relative w-24 h-14 rounded-xl overflow-hidden border-2 transition-all flex-shrink-0 ${
                      selectedScreenshotIndex === idx
                        ? "border-cyan-400 scale-102 ring-2 ring-cyan-500/30"
                        : "border-slate-800 opacity-60 hover:opacity-100"
                    }`}
                  >
                    <img src={shot} alt="preview" className="w-full h-full object-cover" />
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Navigation Tabs */}
          <div className="flex items-center gap-2 border-b border-slate-800/80 pb-1 text-sm font-semibold">
            <button
              onClick={() => setActiveTab("overview")}
              className={`px-4 py-2 rounded-xl transition-all ${
                activeTab === "overview"
                  ? "bg-slate-800 text-cyan-300 border border-slate-700"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              Overview & Details
            </button>
            <button
              onClick={() => setActiveTab("manifest")}
              className={`px-4 py-2 rounded-xl transition-all flex items-center gap-1.5 ${
                activeTab === "manifest"
                  ? "bg-slate-800 text-cyan-300 border border-slate-700"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              <Layers className="w-4 h-4" />
              <span>Manifest & Files ({selectedPackage.components.length})</span>
            </button>
            <button
              onClick={() => setActiveTab("safety")}
              className={`px-4 py-2 rounded-xl transition-all flex items-center gap-1.5 ${
                activeTab === "safety"
                  ? "bg-slate-800 text-cyan-300 border border-slate-700"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              <ShieldCheck className="w-4 h-4 text-cyan-400" />
              <span>Safety & Backups</span>
            </button>
          </div>

          {/* Tab 1: Overview */}
          {activeTab === "overview" && (
            <div className="space-y-5">
              <div>
                <h4 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-1">
                  Description
                </h4>
                <p className="text-sm text-slate-300 leading-relaxed">
                  {selectedPackage.description}
                </p>
              </div>

              {/* Author & Stats Grid */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 p-4 rounded-2xl bg-slate-900/60 border border-slate-800">
                <div>
                  <div className="text-[11px] text-slate-400 mb-1">Author</div>
                  <div className="flex items-center gap-2">
                    <img
                      src={selectedPackage.author.avatar}
                      alt={selectedPackage.author.name}
                      className="w-5 h-5 rounded-full bg-slate-800"
                    />
                    <span className="text-xs font-semibold text-slate-200">
                      {selectedPackage.author.name}
                    </span>
                  </div>
                </div>

                <div>
                  <div className="text-[11px] text-slate-400 mb-1">Rating</div>
                  <div className="flex items-center gap-1 text-amber-300 text-xs font-bold">
                    <Star className="w-3.5 h-3.5 fill-amber-300" />
                    <span>{selectedPackage.rating.toFixed(2)}</span>
                    <span className="text-slate-500 font-normal">({selectedPackage.rating_count})</span>
                  </div>
                </div>

                <div>
                  <div className="text-[11px] text-slate-400 mb-1">Downloads</div>
                  <div className="flex items-center gap-1 text-slate-200 text-xs font-semibold">
                    <Download className="w-3.5 h-3.5 text-cyan-400" />
                    <span>{selectedPackage.downloads.toLocaleString()}</span>
                  </div>
                </div>

                <div>
                  <div className="text-[11px] text-slate-400 mb-1">Desktops</div>
                  <div className="text-xs font-semibold text-slate-300 uppercase">
                    {selectedPackage.supported_desktops.join(", ")}
                  </div>
                </div>
              </div>

              {/* Tags & Palette */}
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div className="flex flex-wrap items-center gap-1.5">
                  {selectedPackage.tags.map((tag, idx) => (
                    <span
                      key={idx}
                      className="px-2.5 py-1 rounded-lg text-xs bg-slate-900 text-slate-300 border border-slate-800"
                    >
                      #{tag}
                    </span>
                  ))}
                </div>

                <div className="flex items-center gap-2">
                  <span className="text-xs text-slate-400 font-medium">Color Palette:</span>
                  <div className="flex items-center gap-1.5 p-1 rounded-lg bg-slate-900 border border-slate-800">
                    {selectedPackage.color_palette.map((color, idx) => (
                      <span
                        key={idx}
                        className="w-3.5 h-3.5 rounded-full ring-1 ring-black/40"
                        style={{ backgroundColor: color }}
                        title={color}
                      />
                    ))}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Tab 2: Manifest Inspector */}
          {activeTab === "manifest" && (
            <div className="space-y-4">
              <div className="p-3.5 rounded-xl bg-cyan-950/20 border border-cyan-500/30 text-xs text-cyan-200">
                <strong>Declarative Package Manifest</strong>: Ryzora explicitly maps every configuration file to its designated destination. No root privileges or arbitrary bash execution required.
              </div>

              <div className="space-y-2">
                <h4 className="text-xs font-bold uppercase tracking-wider text-slate-400">
                  Target Files & Directories ({selectedPackage.components.length})
                </h4>

                <div className="divide-y divide-slate-800/80 rounded-2xl bg-slate-900/60 border border-slate-800 overflow-hidden">
                  {selectedPackage.components.map((c, idx) => (
                    <div key={idx} className="p-3.5 flex items-start justify-between gap-4 text-xs">
                      <div>
                        <div className="font-bold text-slate-200 mb-0.5">{c.name}</div>
                        <div className="text-slate-400">{c.description}</div>
                      </div>
                      <code className="px-2.5 py-1 rounded-md bg-slate-950 text-cyan-400 font-mono text-[11px] border border-slate-800 flex-shrink-0">
                        {c.target_path}
                      </code>
                    </div>
                  ))}
                </div>
              </div>

              {/* Dependencies Check */}
              <div>
                <h4 className="text-xs font-bold uppercase tracking-wider text-slate-400 mb-2">
                  Required Dependencies
                </h4>
                <div className="flex flex-wrap gap-2">
                  {selectedPackage.dependencies.packages.map((dep, idx) => {
                    const isInstalledOnSystem = systemInfo?.installed_components.some(
                      (ic) => ic.binary === dep && ic.installed
                    );

                    return (
                      <span
                        key={idx}
                        className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-xl text-xs font-mono border ${
                          isInstalledOnSystem
                            ? "bg-emerald-950/40 text-emerald-300 border-emerald-500/40"
                            : "bg-slate-900 text-slate-300 border-slate-800"
                        }`}
                      >
                        {isInstalledOnSystem ? (
                          <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                        ) : (
                          <span className="w-1.5 h-1.5 rounded-full bg-slate-500" />
                        )}
                        <span>{dep}</span>
                        {isInstalledOnSystem && <span className="text-[10px] text-emerald-400">(ready)</span>}
                      </span>
                    );
                  })}
                </div>
              </div>
            </div>
          )}

          {/* Tab 3: Safety & Backups */}
          {activeTab === "safety" && (
            <div className="space-y-4">
              <div className="p-4 rounded-2xl bg-emerald-950/20 border border-emerald-500/30 space-y-2">
                <div className="flex items-center gap-2 text-emerald-400 font-bold text-sm">
                  <ShieldCheck className="w-5 h-5" />
                  <span>Audit Rating: {selectedPackage.safety_audit.rating.toUpperCase()}</span>
                </div>
                <p className="text-xs text-slate-300 leading-relaxed">
                  This package complies with Ryzora's strict safety standards. It does not request root privileges, does not modify kernel/system binaries, and can be cleanly rolled back with one click.
                </p>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
                <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800">
                  <div className="text-slate-400 mb-1">Pre-Install Snapshot</div>
                  <div className="font-semibold text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>Automatic (~/.config)</span>
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800">
                  <div className="text-slate-400 mb-1">Root Privileges</div>
                  <div className="font-semibold text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>Not Required (User-space)</span>
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-900/60 border border-slate-800">
                  <div className="text-slate-400 mb-1">Rollback Ready</div>
                  <div className="font-semibold text-emerald-400 flex items-center gap-1">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>1-Click Restore</span>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Installation Progress & Terminal Console Logs */}
          {isInstalling && (
            <div className="p-4 rounded-2xl bg-slate-950 border border-cyan-500/40 space-y-3 shadow-xl">
              <div className="flex items-center justify-between text-xs font-semibold text-cyan-300">
                <div className="flex items-center gap-2">
                  <span className="w-2 h-2 rounded-full bg-cyan-400 animate-ping" />
                  <span>Safely Installing {selectedPackage.title}...</span>
                </div>
                <span className="font-mono">{installProgress}%</span>
              </div>

              {/* Progress Track */}
              <div className="w-full h-2 rounded-full bg-slate-900 overflow-hidden">
                <div
                  className="h-full bg-gradient-to-r from-cyan-500 to-indigo-500 transition-all duration-300"
                  style={{ width: `${installProgress}%` }}
                />
              </div>

              {/* Mini Terminal Logs */}
              <div className="p-3 rounded-xl bg-black/80 font-mono text-[11px] text-slate-300 space-y-1 max-h-32 overflow-y-auto">
                {installLogs.map((line, idx) => (
                  <div key={idx} className="leading-tight">
                    {line}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer Actions */}
        <div className="p-5 border-t border-slate-800/80 bg-slate-900/50 flex flex-wrap items-center justify-between gap-4">
          <div className="text-xs text-slate-400 flex items-center gap-2">
            <History className="w-4 h-4 text-indigo-400" />
            <span>Automatic snapshot is generated before changes</span>
          </div>

          <div className="flex items-center gap-3">
            {isInstalled && relatedSnapshot && (
              <button
                onClick={() => rollbackSnapshot(relatedSnapshot.id)}
                disabled={isInstalling}
                className="flex items-center gap-1.5 px-4 py-2.5 rounded-xl text-xs font-bold bg-slate-800 hover:bg-slate-700 text-amber-300 border border-amber-500/30 transition-all"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>Rollback Changes</span>
              </button>
            )}

            <button
              onClick={() => installPackage(selectedPackage)}
              disabled={isInstalling}
              className={`flex items-center gap-2 px-6 py-2.5 rounded-xl text-sm font-bold text-white shadow-lg transition-all ${
                isInstalled
                  ? "bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700"
                  : "bg-gradient-to-r from-cyan-500 via-indigo-600 to-fuchsia-600 hover:from-cyan-400 hover:to-indigo-500 shadow-cyan-500/20"
              }`}
            >
              {isInstalling ? (
                <>
                  <span className="w-4 h-4 rounded-full border-2 border-white/30 border-t-white animate-spin" />
                  <span>Installing...</span>
                </>
              ) : isInstalled ? (
                <>
                  <CheckCircle2 className="w-4 h-4 text-emerald-400" />
                  <span>Reinstall / Re-apply</span>
                </>
              ) : (
                <>
                  <Download className="w-4 h-4" />
                  <span>Install {selectedPackage.category === "rices" ? "Rice" : "Package"}</span>
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
