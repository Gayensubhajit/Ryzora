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
  PanelLeftClose,
  PanelLeft,
  FlaskConical,
  AppWindow,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { CategoryId } from "../types";

interface SidebarProps {
  onOpenAbout?: () => void;
}

type NavItem = { id: CategoryId; label: string; icon: React.ReactNode; badge?: number | string };

export const Sidebar: React.FC<SidebarProps> = ({ onOpenAbout }) => {
  const {
    activeCategory,
    setActiveCategory,
    installedPackageIds,
    snapshots,
    systemInfo,
    updatesSummary,
    sidebarCollapsed,
    toggleSidebar,
  } = useApp();

  const primaryNav: NavItem[] = [
    { id: "hub",         label: "Home",        icon: <Globe className="w-4 h-4" /> },
    { id: "discover",    label: "Discover",    icon: <Compass className="w-4 h-4" /> },
    { id: "apps",        label: "Apps",        icon: <AppWindow className="w-4 h-4" /> },
    {
      id: "installed",   label: "Library",     icon: <Library className="w-4 h-4" />,
      badge: installedPackageIds.length,
    },
    {
      id: "updates",     label: "Updates",     icon: <ArrowUpCircle className="w-4 h-4" />,
      badge: updatesSummary && updatesSummary.total_updates > 0 ? updatesSummary.total_updates : undefined,
    },
    { id: "collections", label: "Collections", icon: <Bookmark className="w-4 h-4" /> },
  ];

  const communityNav: NavItem[] = [
    { id: "repositories", label: "Repositories", icon: <Server className="w-4 h-4" /> },
    { id: "creators",     label: "Creators",     icon: <Users className="w-4 h-4" /> },
  ];

  const toolsNav: NavItem[] = [
    { id: "backups",       label: "Backups",        icon: <History className="w-4 h-4" />, badge: snapshots.length },
    { id: "integrity",     label: "Integrity",      icon: <ShieldCheck className="w-4 h-4" /> },
    { id: "system",        label: "System",         icon: <Cpu className="w-4 h-4" /> },
    { id: "notifications", label: "Activity",       icon: <Bell className="w-4 h-4" /> },
    { id: "author",        label: "Pkg Creator",    icon: <PackagePlus className="w-4 h-4" /> },
    { id: "compatibility-lab", label: "Compat Lab", icon: <FlaskConical className="w-4 h-4 text-purple-400" />, badge: "DEV" },
    { id: "settings",      label: "Settings",       icon: <Settings className="w-4 h-4" /> },
  ];

  const NavButton: React.FC<{ item: NavItem }> = ({ item }) => {
    const isActive = activeCategory === item.id;
    return (
      <button
        type="button"
        onClick={() => setActiveCategory(item.id)}
        title={sidebarCollapsed ? `${item.label}${item.badge ? ` (${item.badge})` : ""}` : undefined}
        className={[
          "relative w-full flex items-center rounded-lg font-medium transition-all text-left cursor-pointer",
          sidebarCollapsed ? "justify-center py-2.5 px-1" : "justify-between px-3 py-2 text-[13px]",
          isActive
            ? "text-[var(--rz-nav-item-active-text)] bg-[var(--rz-nav-item-active-bg)] font-semibold shadow-xs"
            : "text-[var(--rz-nav-item)] hover:text-[var(--rz-nav-item-hover)] hover:bg-[var(--rz-surface-hover)]",
        ].join(" ")}
      >
        <div className="flex items-center gap-2.5">
          <span className={isActive ? "text-[var(--rz-accent)]" : "text-[var(--rz-text-muted)]"}>
            {item.icon}
          </span>
          {!sidebarCollapsed && <span className="truncate">{item.label}</span>}
        </div>
        {item.badge !== undefined && (
          sidebarCollapsed ? (
            <span className="absolute top-1 right-1 w-2.5 h-2.5 rounded-full bg-[var(--rz-accent)] ring-2 ring-[var(--rz-sidebar-bg)]" />
          ) : (
            <span className={[
              "px-1.5 py-0.5 rounded font-mono leading-none",
              typeof item.badge === "string"
                ? "text-[9px] font-bold bg-amber-500/20 text-amber-300 border border-amber-500/30 uppercase"
                : "text-[10px] rounded-full bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)] tabular-nums"
            ].join(" ")}>
              {item.badge}
            </span>
          )
        )}
      </button>
    );
  };

  const SectionLabel: React.FC<{ label: string }> = ({ label }) =>
    sidebarCollapsed ? (
      <div className="my-2 border-t border-[var(--border-subtle)]/60 mx-2" />
    ) : (
      <div className="px-3 mb-1 mt-3 text-[10px] font-bold uppercase tracking-wider text-[var(--rz-nav-section-label)] select-none truncate">
        {label}
      </div>
    );

  return (
    <aside
      className={[
        sidebarCollapsed ? "w-14" : "w-48 sm:w-52",
        "flex-shrink-0 flex flex-col h-screen border-r border-[var(--border-subtle)] bg-[var(--rz-sidebar-bg)] select-none z-20 transition-all duration-200 ease-in-out",
      ].join(" ")}
    >
      {/* Brand & Collapse Header */}
      <div className="h-12 px-3 sm:px-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div className="flex items-center gap-2.5 overflow-hidden">
          <div className="flex items-center justify-center w-7 h-7 rounded-lg bg-[var(--accent)] text-white font-mono text-xs font-bold flex-shrink-0 shadow-xs">
            R
          </div>
          {!sidebarCollapsed && (
            <span className="font-bold text-xs tracking-widest text-[var(--text-primary)] uppercase truncate">
              Ryzora
            </span>
          )}
        </div>
        {!sidebarCollapsed && (
          <button
            type="button"
            onClick={toggleSidebar}
            className="p-1.5 rounded-md text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
            title="Collapse Sidebar (Ctrl+B)"
            aria-label="Collapse Sidebar"
          >
            <PanelLeftClose className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Navigation */}
      <div className="flex-1 overflow-y-auto px-2 py-3 space-y-4 scrollbar-none" style={{ scrollbarWidth: "none" }}>
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

      {/* System footer — responsive */}
      <div className="px-2 py-2.5 border-t border-[var(--border-subtle)]">
        {!sidebarCollapsed ? (
          <>
            <button
              type="button"
              onClick={() => setActiveCategory("system")}
              className="w-full text-left px-2.5 py-1.5 rounded-lg hover:bg-[var(--bg-surface-elevated)] transition-colors cursor-pointer"
              title="System diagnostics"
            >
              <div className="text-xs font-semibold text-[var(--rz-text)] truncate">
                {systemInfo?.window_manager || "Hyprland"}
              </div>
              <div className="text-[10px] text-[var(--rz-text-muted)] capitalize mt-0.5 truncate">
                {systemInfo?.session_type || "wayland"} · Connected
              </div>
            </button>
            {onOpenAbout && (
              <button
                type="button"
                onClick={onOpenAbout}
                className="mt-1 w-full text-left px-2.5 py-1 rounded text-[11px] font-medium text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--bg-surface-elevated)] transition-colors cursor-pointer"
              >
                About Ryzora
              </button>
            )}
          </>
        ) : (
          <div className="flex flex-col items-center gap-1.5">
            <button
              type="button"
              onClick={() => setActiveCategory("system")}
              className="w-9 h-9 flex items-center justify-center rounded-lg hover:bg-[var(--bg-surface-elevated)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer"
              title={`System: ${systemInfo?.window_manager || "Hyprland"} (${systemInfo?.session_type || "wayland"})`}
            >
              <Cpu className="w-4 h-4" />
            </button>
            <button
              type="button"
              onClick={toggleSidebar}
              className="w-9 h-9 flex items-center justify-center rounded-lg hover:bg-[var(--bg-surface-elevated)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer"
              title="Expand Sidebar"
              aria-label="Expand Sidebar"
            >
              <PanelLeft className="w-4 h-4" />
            </button>
          </div>
        )}
      </div>
    </aside>
  );
};
