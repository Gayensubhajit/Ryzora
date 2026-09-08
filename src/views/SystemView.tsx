import React from "react";
import { RefreshCw, Check, X } from "lucide-react";
import { useApp } from "../context/AppContext";

export const SystemView: React.FC = () => {
  const { systemInfo, loadingSystem, refreshSystem } = useApp();

  return (
    <div className="space-y-6 pb-10">
      {/* Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            System Diagnostics
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Detected hardware, display protocol, and desktop customization components.
          </p>
        </div>

        <button
          onClick={refreshSystem}
          disabled={loadingSystem}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-primary)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`w-3 h-3 ${loadingSystem ? "animate-spin" : ""}`} />
          <span>{loadingSystem ? "Probing..." : "Re-probe"}</span>
        </button>
      </div>

      {/* Grid of 4 Key Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Distribution</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.distro_name || "Garuda Linux"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.distro_version}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Window Manager</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.window_manager || "Hyprland"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5 capitalize">
            {systemInfo?.session_type || "wayland"}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Kernel</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.kernel_version || "6.18"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.distro_id}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Terminal & Shell</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.terminal || "Kitty"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.shell || "zsh"}
          </div>
        </div>
      </div>

      {/* Installed Components Table */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
          <span>Component Status ({systemInfo?.installed_components.filter((c) => c.installed).length || 0} installed)</span>
        </div>

        <div className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] divide-y divide-[var(--border-subtle)] overflow-hidden">
          {systemInfo?.installed_components.map((c, idx) => (
            <div
              key={idx}
              className="p-3 flex items-center justify-between gap-3 text-xs"
            >
              <div className="flex items-center gap-2.5">
                {c.installed ? (
                  <Check className="w-3.5 h-3.5 text-emerald-400 flex-shrink-0" />
                ) : (
                  <X className="w-3.5 h-3.5 text-[var(--text-faint)] flex-shrink-0" />
                )}
                <div>
                  <span className="font-medium text-[var(--text-primary)] mr-2">{c.name}</span>
                  <span className="text-[11px] text-[var(--text-faint)]">{c.category}</span>
                </div>
              </div>

              <div className="flex items-center gap-3 font-mono text-[11px]">
                {c.path ? (
                  <span className="text-[var(--text-muted)]">{c.path}</span>
                ) : (
                  <span className="text-[var(--text-faint)] italic">not found</span>
                )}
                <span
                  className={`text-[10px] px-1.5 py-0.2 rounded font-mono ${
                    c.installed
                      ? "text-emerald-400 bg-emerald-400/10 border border-emerald-400/20"
                      : "text-[var(--text-faint)] bg-[var(--bg-canvas)] border border-[var(--border-subtle)]"
                  }`}
                >
                  {c.installed ? "present" : "missing"}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
