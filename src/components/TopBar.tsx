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
    systemInfo,
    snapshots,
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
    <header className="h-16 px-6 border-b border-slate-800/70 bg-[#090d16]/80 backdrop-blur-xl flex items-center justify-between gap-4 sticky top-0 z-10 select-none">
      {/* Search Bar */}
      <div className="relative flex-1 max-w-lg">
        <div className="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-slate-400">
          <Search className="w-4 h-4" />
        </div>
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search rices, themes, waybar modules, fastfetch..."
          className="w-full pl-9 pr-8 py-2 bg-slate-900/90 hover:bg-slate-900 border border-slate-800 focus:border-cyan-500/50 rounded-xl text-sm text-slate-200 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-cyan-500/20 transition-all shadow-inner"
        />
        {searchQuery && (
          <button
            onClick={() => setSearchQuery("")}
            className="absolute inset-y-0 right-0 pr-3 flex items-center text-slate-400 hover:text-slate-200"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      {/* Desktop Filter Dropdown & Actions */}
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-1.5 bg-slate-900/80 border border-slate-800 px-2.5 py-1.5 rounded-xl text-xs text-slate-300">
          <Filter className="w-3.5 h-3.5 text-cyan-400" />
          <span className="text-slate-400 font-medium">Desktop:</span>
          <select
            value={desktopFilter}
            onChange={(e) => setDesktopFilter(e.target.value as DesktopEnvironment | "all")}
            className="bg-transparent text-cyan-300 font-medium focus:outline-none cursor-pointer pr-1"
          >
            {desktops.map((d) => (
              <option key={d.id} value={d.id} className="bg-slate-900 text-slate-200">
                {d.label}
              </option>
            ))}
          </select>
        </div>

        {/* Backups Button */}
        <button
          onClick={() => setActiveCategory("backups")}
          className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 hover:border-slate-700 text-xs font-medium text-slate-300 transition-colors"
          title="Manage system snapshots & rollbacks"
        >
          <History className="w-3.5 h-3.5 text-indigo-400" />
          <span>Snapshots</span>
          <span className="px-1.5 py-0.2 rounded-md bg-indigo-500/20 text-indigo-300 font-mono text-[10px]">
            {snapshots.length}
          </span>
        </button>

        {/* System Diagnostics Chip */}
        <button
          onClick={() => setActiveCategory("system")}
          className="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 hover:border-cyan-500/30 text-xs font-medium text-slate-300 transition-colors"
        >
          <Cpu className="w-3.5 h-3.5 text-emerald-400" />
          <span className="hidden md:inline font-mono text-[11px] text-slate-300">
            {systemInfo?.window_manager || "Hyprland"}
          </span>
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
        </button>
      </div>
    </header>
  );
};
