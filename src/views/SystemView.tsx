import React from "react";
import { Cpu, RefreshCw, CheckCircle2, XCircle, Monitor, Terminal, HardDrive } from "lucide-react";
import { useApp } from "../context/AppContext";

export const SystemView: React.FC = () => {
  const { systemInfo, loadingSystem, refreshSystem } = useApp();

  return (
    <div className="space-y-6 pb-12">
      {/* Header */}
      <div className="p-8 rounded-3xl bg-gradient-to-r from-cyan-950/40 via-slate-900/60 to-transparent border border-cyan-500/30 flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-cyan-400 text-xs font-bold uppercase tracking-wider mb-2">
            <Cpu className="w-4 h-4" />
            <span>Hardware & Environment Probe</span>
          </div>
          <h1 className="text-3xl font-extrabold text-white tracking-tight mb-2">
            System Diagnostics
          </h1>
          <p className="text-sm text-slate-400 max-w-xl leading-relaxed">
            Ryzora probes your Linux distribution and desktop environment to recommend compatible customization packages and verify prerequisites.
          </p>
        </div>

        <button
          onClick={refreshSystem}
          disabled={loadingSystem}
          className="flex items-center gap-2 px-4 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-cyan-300 border border-cyan-500/30 transition-all disabled:opacity-50"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loadingSystem ? "animate-spin" : ""}`} />
          <span>{loadingSystem ? "Probing..." : "Re-probe System"}</span>
        </button>
      </div>

      {/* Primary Environment Specs */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="p-5 rounded-2xl bg-slate-900/70 border border-slate-800 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <Monitor className="w-3.5 h-3.5 text-cyan-400" />
            <span>Linux Distribution</span>
          </div>
          <div className="text-lg font-bold text-white truncate">
            {systemInfo?.distro_name || "Garuda Linux"}
          </div>
          <div className="text-[11px] text-slate-500 font-mono">
            ID: {systemInfo?.distro_id} • {systemInfo?.distro_version}
          </div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/70 border border-slate-800 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <HardDrive className="w-3.5 h-3.5 text-indigo-400" />
            <span>Window Manager</span>
          </div>
          <div className="text-lg font-bold text-white truncate">
            {systemInfo?.window_manager || "Hyprland"}
          </div>
          <div className="text-[11px] text-slate-500 font-mono">
            Session: {systemInfo?.session_type}
          </div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/70 border border-slate-800 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <Cpu className="w-3.5 h-3.5 text-emerald-400" />
            <span>Linux Kernel</span>
          </div>
          <div className="text-lg font-bold text-white truncate">
            {systemInfo?.kernel_version || "6.18 LTS"}
          </div>
          <div className="text-[11px] text-emerald-400 font-medium">
            Arch Linux Base
          </div>
        </div>

        <div className="p-5 rounded-2xl bg-slate-900/70 border border-slate-800 space-y-1">
          <div className="text-xs text-slate-400 flex items-center gap-1.5">
            <Terminal className="w-3.5 h-3.5 text-amber-400" />
            <span>Shell & Terminal</span>
          </div>
          <div className="text-lg font-bold text-white truncate">
            {systemInfo?.terminal || "Kitty"}
          </div>
          <div className="text-[11px] text-slate-500 font-mono">
            Shell: {systemInfo?.shell || "fish"}
          </div>
        </div>
      </div>

      {/* Component Probe Table */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
            Detected Desktop & Customization Components
          </span>
          <span className="text-xs text-slate-500">
            {systemInfo?.installed_components.filter((c) => c.installed).length || 0} installed
          </span>
        </div>

        <div className="rounded-2xl bg-slate-900/60 border border-slate-800 overflow-hidden divide-y divide-slate-800/80">
          {systemInfo?.installed_components.map((c, idx) => (
            <div
              key={idx}
              className="p-4 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs"
            >
              <div className="flex items-center gap-3">
                {c.installed ? (
                  <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0" />
                ) : (
                  <XCircle className="w-4 h-4 text-slate-600 flex-shrink-0" />
                )}
                <div>
                  <div className="font-bold text-slate-200">{c.name}</div>
                  <div className="text-slate-500">{c.category}</div>
                </div>
              </div>

              <div className="flex items-center gap-3 font-mono text-[11px]">
                {c.path ? (
                  <span className="text-slate-400 bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                    {c.path}
                  </span>
                ) : (
                  <span className="text-slate-600 italic">Not found in PATH</span>
                )}

                <span
                  className={`px-2 py-0.5 rounded-full font-semibold text-[10px] uppercase ${
                    c.installed
                      ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                      : "bg-slate-800 text-slate-500 border border-slate-700"
                  }`}
                >
                  {c.installed ? "Installed" : "Available to Install"}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
