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
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { CategoryId } from "../types";

export const Sidebar: React.FC = () => {
  const { activeCategory, setActiveCategory, installedPackageIds, snapshots, systemInfo } = useApp();

  const mainCategories: { id: CategoryId; label: string; icon: React.ReactNode }[] = [
    { id: "discover", label: "Discover", icon: <Compass className="w-4 h-4" /> },
    { id: "rices", label: "Complete Rices", icon: <LayoutGrid className="w-4 h-4" /> },
    { id: "themes", label: "Themes", icon: <Palette className="w-4 h-4" /> },
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
      label: "Backups",
      icon: <History className="w-4 h-4" />,
      badge: snapshots.length,
    },
    {
      id: "system",
      label: "System",
      icon: <Cpu className="w-4 h-4" />,
    },
  ];

  return (
    <aside className="w-56 flex-shrink-0 flex flex-col justify-between h-screen border-r border-[var(--border-subtle)] bg-[var(--bg-sidebar)] select-none z-20">
      {/* Brand Header */}
      <div className="px-4 py-3.5 border-b border-[var(--border-subtle)]">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center justify-center w-6 h-6 rounded-md bg-[var(--accent)] text-white font-mono text-xs font-bold">
            R
          </div>
          <div>
            <div className="font-bold text-sm tracking-wide text-[var(--text-primary)]">
              RYZORA
            </div>
            <div className="text-[11px] text-[var(--text-muted)] font-normal">
              Linux customization
            </div>
          </div>
        </div>
      </div>

      {/* Navigation Links */}
      <div className="flex-1 overflow-y-auto px-2 py-3 space-y-5">
        <div>
          <div className="px-2.5 mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--text-faint)]">
            Explore
          </div>
          <nav className="space-y-0.5">
            {mainCategories.map((item) => {
              const isActive = activeCategory === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveCategory(item.id)}
                  className={`w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors text-left ${
                    isActive
                      ? "bg-[var(--bg-surface-elevated)] text-white border-l-2 border-[var(--accent)]"
                      : "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
                  }`}
                >
                  <span className={isActive ? "text-[var(--accent-text)]" : "text-[var(--text-muted)]"}>
                    {item.icon}
                  </span>
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        <div>
          <div className="px-2.5 mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-[var(--text-faint)]">
            Manage
          </div>
          <nav className="space-y-0.5">
            {managementCategories.map((item) => {
              const isActive = activeCategory === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => setActiveCategory(item.id)}
                  className={`w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-xs font-medium transition-colors text-left ${
                    isActive
                      ? "bg-[var(--bg-surface-elevated)] text-white border-l-2 border-[var(--accent)]"
                      : "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)]"
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <span className={isActive ? "text-[var(--accent-text)]" : "text-[var(--text-muted)]"}>
                      {item.icon}
                    </span>
                    <span>{item.label}</span>
                  </div>
                  {item.badge !== undefined && item.badge > 0 && (
                    <span className="text-[10px] px-1.5 py-0.2 rounded font-mono bg-[var(--bg-surface-elevated)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
                      {item.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
        </div>
      </div>

      {/* Compact System Footer */}
      <div className="p-3 border-t border-[var(--border-subtle)] bg-[var(--bg-canvas)]/50">
        <button
          onClick={() => setActiveCategory("system")}
          className="w-full text-left p-2 rounded-md hover:bg-[var(--bg-surface-elevated)] transition-colors group"
        >
          <div className="text-xs font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.distro_name || "Linux Host"}
          </div>
          <div className="text-[11px] text-[var(--text-muted)] flex items-center gap-1.5">
            <span>{systemInfo?.window_manager || "Hyprland"}</span>
            <span>·</span>
            <span className="capitalize">{systemInfo?.session_type || "wayland"}</span>
          </div>
        </button>
      </div>
    </aside>
  );
};
