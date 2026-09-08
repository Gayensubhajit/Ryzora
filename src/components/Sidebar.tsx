import React from "react";
import {
  Compass,
  LayoutGrid,
  Palette,
  Sliders,
  TerminalSquare,
  Lock,
  Image as ImageIcon,
  Terminal,
  DownloadCloud,
  History,
  Cpu,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { CategoryId } from "../types";

export const Sidebar: React.FC = () => {
  const { activeCategory, setActiveCategory, installedPackageIds, snapshots, systemInfo } = useApp();

  const mainCategories: { id: CategoryId; label: string; icon: React.ReactNode }[] = [
    { id: "discover", label: "Discover", icon: <Compass className="w-4 h-4" /> },
    { id: "rices", label: "Complete Rices", icon: <LayoutGrid className="w-4 h-4" /> },
    { id: "themes", label: "Themes & GTK", icon: <Palette className="w-4 h-4" /> },
    { id: "bars", label: "Status Bars", icon: <Sliders className="w-4 h-4" /> },
    { id: "fastfetch", label: "Fastfetch", icon: <TerminalSquare className="w-4 h-4" /> },
    { id: "lockscreens", label: "Lockscreens", icon: <Lock className="w-4 h-4" /> },
    { id: "wallpapers", label: "Wallpapers", icon: <ImageIcon className="w-4 h-4" /> },
    { id: "terminal", label: "Terminal", icon: <Terminal className="w-4 h-4" /> },
  ];

  const managementCategories: { id: CategoryId; label: string; icon: React.ReactNode; badge?: number }[] = [
    {
      id: "installed",
      label: "Installed",
      icon: <DownloadCloud className="w-4 h-4" />,
      badge: installedPackageIds.length,
    },
    {
      id: "backups",
      label: "Backups & Rollback",
      icon: <History className="w-4 h-4" />,
      badge: snapshots.length,
    },
    {
      id: "system",
      label: "System Probe",
      icon: <Cpu className="w-4 h-4" />,
    },
  ];

  return (
    <aside className="w-64 flex-shrink-0 flex flex-col justify-between h-screen border-r border-slate-800/70 bg-[#090d16]/95 backdrop-blur-xl select-none z-20">
      {/* Top Section: Brand */}
      <div className="p-5 pb-3">
        <div className="flex items-center gap-3">
          <div className="relative flex items-center justify-center w-10 h-10 rounded-xl bg-gradient-to-tr from-cyan-500 via-indigo-500 to-fuchsia-500 shadow-lg shadow-cyan-500/20 ring-1 ring-white/20">
            <Sparkles className="w-5 h-5 text-white" />
          </div>
          <div>
            <div className="flex items-center gap-1.5">
              <span className="font-extrabold text-xl tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-white via-slate-100 to-cyan-200">
                RYZORA
              </span>
              <span className="text-[10px] px-1.5 py-0.5 font-bold uppercase rounded bg-cyan-500/15 text-cyan-400 border border-cyan-500/30">
                v0.1
              </span>
            </div>
            <p className="text-[11px] text-slate-400 font-medium">Linux Customization Store</p>
          </div>
        </div>
      </div>

      {/* Navigation Links */}
      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-6">
        <div>
          <div className="px-3 mb-2 text-[10px] font-bold uppercase tracking-wider text-slate-400">
            Explore Marketplace
          </div>
          <nav className="space-y-1">
            {mainCategories.map((item) => {
              const isActive = activeCategory === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveCategory(item.id)}
                  className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-150 text-left ${
                    isActive
                      ? "bg-gradient-to-r from-cyan-500/15 to-indigo-500/10 text-cyan-300 border border-cyan-500/30 shadow-sm shadow-cyan-500/10"
                      : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40"
                  }`}
                >
                  <span className={`${isActive ? "text-cyan-400" : "text-slate-400"}`}>
                    {item.icon}
                  </span>
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        <div>
          <div className="px-3 mb-2 text-[10px] font-bold uppercase tracking-wider text-slate-400">
            Management & Safety
          </div>
          <nav className="space-y-1">
            {managementCategories.map((item) => {
              const isActive = activeCategory === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveCategory(item.id)}
                  className={`w-full flex items-center justify-between px-3 py-2 rounded-lg text-sm font-medium transition-all duration-150 text-left ${
                    isActive
                      ? "bg-gradient-to-r from-cyan-500/15 to-indigo-500/10 text-cyan-300 border border-cyan-500/30"
                      : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/40"
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <span className={`${isActive ? "text-cyan-400" : "text-slate-400"}`}>
                      {item.icon}
                    </span>
                    <span>{item.label}</span>
                  </div>
                  {item.badge !== undefined && item.badge > 0 && (
                    <span
                      className={`text-[11px] px-1.5 py-0.2 rounded-full font-semibold ${
                        isActive
                          ? "bg-cyan-500/30 text-cyan-200"
                          : "bg-slate-800 text-slate-400"
                      }`}
                    >
                      {item.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>

        {/* Safety Guarantee Banner */}
        <div className="mx-1 p-3 rounded-xl bg-gradient-to-b from-indigo-950/40 to-slate-900/60 border border-indigo-500/20">
          <div className="flex items-center gap-2 text-indigo-300 text-xs font-semibold mb-1">
            <ShieldCheck className="w-4 h-4 text-cyan-400" />
            <span>Safety First</span>
          </div>
          <p className="text-[11px] text-slate-400 leading-relaxed">
            All rices use declarative manifests. Automated snapshots are created prior to any modification.
          </p>
        </div>
      </div>

      {/* Bottom User Environment Widget */}
      <div className="p-3 border-t border-slate-800/60 bg-[#06080e]/60">
        <div
          onClick={() => setActiveCategory("system")}
          className="p-2.5 rounded-xl bg-slate-900/80 hover:bg-slate-800/70 border border-slate-800 transition-colors cursor-pointer group"
        >
          <div className="flex items-center justify-between mb-1.5">
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
              <span className="text-xs font-semibold text-slate-200 group-hover:text-cyan-300 transition-colors">
                {systemInfo?.distro_name || "Linux Host"}
              </span>
            </div>
            <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-slate-800 text-cyan-400 border border-slate-700">
              {systemInfo?.session_type || "wayland"}
            </span>
          </div>
          <div className="flex items-center justify-between text-[11px] text-slate-400">
            <span>DE / WM: <strong className="text-slate-300 font-medium">{systemInfo?.window_manager || "Hyprland"}</strong></span>
            <span className="text-[10px] text-slate-400 group-hover:text-slate-300">Probe →</span>
          </div>
        </div>
      </div>
    </aside>
  );
};
