import React from "react";
import { Search, X, Filter, History, Cpu } from "lucide-react";
import { useApp } from "../context/AppContext";
import { DesktopEnvironment } from "../types";

export const TopBar: React.FC = () => {
  const {
    searchQuery,
    setSearchQuery,
    desktopFilter,
    setDesktopFilter,
    snapshots,
    systemInfo,
    setActiveCategory,
  } = useApp();

  const desktops: { id: DesktopEnvironment | "all"; label: string }[] = [
    { id: "all", label: "All Desktops" },
    { id: "hyprland", label: "Hyprland" },
    { id: "sway", label: "Sway" },
    { id: "kde", label: "KDE Plasma" },
    { id: "gnome", label: "GNOME" },
    { id: "xfce", label: "XFCE" },
    { id: "universal", label: "Universal" },
  ];

  return (
    <header className="h-12 px-5 border-b border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between gap-4 sticky top-0 z-10 select-none">
      {/* Search Input */}
      <div className="relative flex-1 max-w-md">
        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-[var(--text-faint)]">
          <Search className="w-3.5 h-3.5" />
        </div>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search rices, themes, bars..."
          className="w-full pl-8 pr-7 py-1.5 bg-[var(--bg-canvas)] border border-[var(--border-subtle)] focus:border-[var(--accent)] rounded-md text-xs text-[var(--text-primary)] placeholder-[var(--text-faint)] focus:outline-none transition-colors"
        />
        {searchQuery && (
          <button
            onClick={() => setSearchQuery("")}
            className="absolute inset-y-0 right-0 pr-2.5 flex items-center text-[var(--text-muted)] hover:text-white"
          >
            <X className="w-3 h-3" />
          </button>
        )}
      </div>

      {/* Filter and Status Controls */}
      <div className="flex items-center gap-2">
        {/* Desktop Filter */}
        <div className="flex items-center gap-1 px-2 py-1 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-xs text-[var(--text-muted)]">
          <Filter className="w-3 h-3 text-[var(--text-faint)]" />
          <select
            value={desktopFilter}
            onChange={(e) => setDesktopFilter(e.target.value as DesktopEnvironment | "all")}
            className="bg-transparent text-[var(--text-primary)] font-medium text-xs focus:outline-none cursor-pointer pr-1"
          >
            {desktops.map((d) => (
              <option key={d.id} value={d.id} className="bg-[#14171d] text-slate-200">
                {d.label}
              </option>
            ))}
          </select>
        </div>

        {/* Snapshots Button */}
        <button
          onClick={() => setActiveCategory("backups")}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-elevated)] border border-[var(--border-subtle)] transition-colors"
          title="View configuration snapshots"
        >
          <History className="w-3.5 h-3.5 text-[var(--text-faint)]" />
          <span>Snapshots</span>
          <span className="font-mono text-[10px] text-[var(--text-faint)]">
            {snapshots.length}
          </span>
        </button>

        {/* System Diagnostics Trigger */}
        <button
          onClick={() => setActiveCategory("system")}
          className="flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-elevated)] border border-[var(--border-subtle)] transition-colors"
          title="System environment"
        >
          <Cpu className="w-3.5 h-3.5 text-[var(--text-faint)]" />
          <span className="font-mono text-[11px] text-[var(--text-muted)]">
            {systemInfo?.window_manager || "Hyprland"}
          </span>
        </button>
      </div>
    </header>
  );
};
