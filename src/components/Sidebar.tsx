import React from "react";
import {
  Globe,
  ArrowUpCircle,
  Server,
  Users,
  Compass,
  Library,
  History,
  Cpu,
  PackagePlus,
  Bookmark,
  ShieldCheck,
  Bell,
  Settings,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { CategoryId } from "../types";

interface SidebarProps {
  onOpenAbout?: () => void;
}

type NavItem = { id: CategoryId; label: string; icon: React.ReactNode; badge?: number };

export const Sidebar: React.FC<SidebarProps> = ({ onOpenAbout }) => {
  const { activeCategory, setActiveCategory, installedPackageIds, snapshots, systemInfo, updatesSummary } = useApp();

  const primaryNav: NavItem[] = [
    { id: "hub", label: "Home", icon: <Globe className="w-4 h-4" /> },
    { id: "discover", label: "Discover", icon: <Compass className="w-4 h-4" /> },
    {
      id: "installed",
      label: "Library",
      icon: <Library className="w-4 h-4" />,
      badge: installedPackageIds.length,
    },
    {
      id: "updates",
      label: "Updates",
      icon: <ArrowUpCircle className="w-4 h-4" />,
      badge: updatesSummary && updatesSummary.total_updates > 0 ? updatesSummary.total_updates : undefined,
    },
    { id: "collections", label: "Collections", icon: <Bookmark className="w-4 h-4" /> },
  ];

  const communityNav: NavItem[] = [
    { id: "repositories", label: "Repositories", icon: <Server className="w-4 h-4" /> },
    { id: "creators", label: "Creators", icon: <Users className="w-4 h-4" /> },
  ];

  const toolsNav: NavItem[] = [
    { id: "backups", label: "Backups", icon: <History className="w-4 h-4" />, badge: snapshots.length },
    { id: "integrity", label: "Integrity", icon: <ShieldCheck className="w-4 h-4" /> },
    { id: "system", label: "System", icon: <Cpu className="w-4 h-4" /> },
    { id: "notifications", label: "Activity", icon: <Bell className="w-4 h-4" /> },
    { id: "author", label: "Package Creator", icon: <PackagePlus className="w-4 h-4" /> },
    { id: "settings", label: "Settings", icon: <Settings className="w-4 h-4" /> },
  ];

  const NavButton: React.FC<{ item: NavItem }> = ({ item }) => {
    const isActive = activeCategory === item.id;
    return (
      <button
        onClick={() => setActiveCategory(item.id)}
        className={[
          "w-full flex items-center justify-between px-2.5 py-1.5 rounded-md text-xs font-medium transition-all text-left border-l-2",
          isActive
            ? "bg-[var(--bg-surface-elevated)] text-[var(--text-primary)] border-[var(--accent)]"
            : "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] border-transparent",
        ].join(" ")}
      >
        <div className="flex items-center gap-2.5">
          <span className={isActive ? "text-[var(--accent-text)]" : "text-[var(--text-muted)]"}>
            {item.icon}
          </span>
          <span>{item.label}</span>
        </div>
        {item.badge !== undefined && item.badge > 0 && (
          <span className="text-[10px] px-1.5 py-0.5 rounded-full font-mono bg-[var(--bg-surface-elevated)] text-[var(--text-muted)] border border-[var(--border-subtle)] tabular-nums leading-none">
            {item.badge}
          </span>
        )}
      </button>
    );
  };

  const SectionLabel: React.FC<{ label: string }> = ({ label }) => (
    <div className="px-2.5 mb-1 text-[10px] font-semibold uppercase tracking-widest text-[var(--text-faint)] select-none">
      {label}
    </div>
  );

  return (
    <aside className="w-52 flex-shrink-0 flex flex-col h-screen border-r border-[var(--border-subtle)] bg-[var(--bg-sidebar)] select-none z-20">
      {/* Brand */}
      <div className="px-4 py-3.5 border-b border-[var(--border-subtle)]">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center justify-center w-6 h-6 rounded-md bg-[var(--accent)] text-white font-mono text-xs font-bold shadow-sm">
            R
          </div>
          <div>
            <div className="font-bold text-sm tracking-wide text-[var(--text-primary)]">RYZORA</div>
            <div className="text-[11px] text-[var(--text-muted)] font-normal">Linux customization</div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <div className="flex-1 overflow-y-auto px-2 py-3 space-y-4">
        <div className="space-y-0.5">
          <SectionLabel label="Store" />
          {primaryNav.map((item) => <NavButton key={item.id} item={item} />)}
        </div>
        <div className="space-y-0.5">
          <SectionLabel label="Community" />
          {communityNav.map((item) => <NavButton key={item.id} item={item} />)}
        </div>
        <div className="space-y-0.5">
          <SectionLabel label="Tools" />
          {toolsNav.map((item) => <NavButton key={item.id} item={item} />)}
        </div>
      </div>

      {/* System footer */}
      <div className="p-3 border-t border-[var(--border-subtle)] bg-[var(--bg-canvas)]/50">
        <button
          onClick={() => setActiveCategory("system")}
          className="w-full text-left p-2 rounded-md hover:bg-[var(--bg-surface-elevated)] transition-colors"
          title="System diagnostics"
        >
          <div className="text-xs font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.distro_name || "Linux Host"}
          </div>
          <div className="text-[11px] text-[var(--text-muted)] flex items-center gap-1.5 mt-0.5">
            <span>{systemInfo?.window_manager || "Hyprland"}</span>
            <span>·</span>
            <span className="capitalize">{systemInfo?.session_type || "wayland"}</span>
          </div>
        </button>
        {onOpenAbout && (
          <button
            onClick={onOpenAbout}
            className="mt-1 w-full text-left px-2 py-1 rounded-md text-[11px] text-[var(--text-faint)] hover:text-[var(--text-muted)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
          >
            About Ryzora
          </button>
        )}
      </div>
    </aside>
  );
};
