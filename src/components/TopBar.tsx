import React, { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Search,
  X,
  DownloadCloud,
  Sun,
  Moon,
  Monitor,
  Check,
  Minus,
  Square,
  PanelLeft,
  PanelLeftClose,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { useTheme, AppearanceMode, AccentColor, ACCENT_SWATCHES } from "../theme";

interface TopBarProps {
  onOpenAbout?: () => void;
}

export const TopBar: React.FC<TopBarProps> = () => {
  const {
    searchQuery,
    setSearchQuery,
    setActiveCategory,
    sidebarCollapsed,
    toggleSidebar,
  } = useApp();

  const { mode, accent, resolvedTheme, setMode, setAccent } = useTheme();
  const [popoverOpen, setPopoverOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);

  // Close popover on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setPopoverOpen(false);
      }
    };
    if (popoverOpen) {
      document.addEventListener("mousedown", handleClickOutside);
      return () => document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [popoverOpen]);

  // Keyboard shortcut: Ctrl+B / Cmd+B to toggle sidebar
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        toggleSidebar();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [toggleSidebar]);

  const appearanceOptions: { id: AppearanceMode; label: string; icon: React.ReactNode }[] = [
    { id: "light", label: "Light", icon: <Sun className="w-4 h-4" /> },
    { id: "dark", label: "Dark", icon: <Moon className="w-4 h-4" /> },
    { id: "system", label: "System", icon: <Monitor className="w-4 h-4" /> },
  ];

  const accents: AccentColor[] = ["blue", "purple", "green", "orange"];

  return (
    <header
      className="h-12 px-3 sm:px-4 border-b border-[var(--rz-border-subtle)] bg-[var(--rz-bg)] flex items-center gap-2 sticky top-0 z-30 select-none transition-colors min-w-0"
    >
      {/* Sidebar toggle button (Dolphin / Finder style) */}
      <button
        type="button"
        data-tauri-drag-region="false"
        onClick={toggleSidebar}
        className="flex items-center justify-center w-8 h-8 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:text-white hover:border-[var(--rz-border-strong)] hover:bg-[var(--rz-surface-hover)] transition-all flex-shrink-0 cursor-pointer shadow-xs"
        title={sidebarCollapsed ? "Expand Sidebar (Ctrl+B)" : "Collapse Sidebar (Ctrl+B)"}
        aria-label="Toggle Navigation Sidebar"
      >
        {sidebarCollapsed ? <PanelLeft className="w-4 h-4" /> : <PanelLeftClose className="w-4 h-4" />}
      </button>

      {/* Search — flexible width */}
      <div className="relative flex-1 max-w-xs sm:max-w-sm min-w-[110px]" data-tauri-drag-region="false">
        <div className="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none text-[var(--rz-text-muted)]">
          <Search className="w-4 h-4" />
        </div>
        <input
          type="text"
          data-tauri-drag-region="false"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search rices, themes, lockscreens…"
          className="w-full pl-8 pr-7 py-1.5 rounded-lg text-xs sm:text-[13px] font-medium text-[var(--rz-text)] placeholder-[var(--rz-search-placeholder)] bg-[var(--rz-search-bg)] border border-[var(--rz-border)] focus:border-[var(--rz-accent)] focus:ring-1 focus:ring-[var(--rz-accent)] focus:outline-none transition-all shadow-xs"
        />
        {searchQuery && (
          <button
            type="button"
            data-tauri-drag-region="false"
            onClick={() => setSearchQuery("")}
            className="absolute inset-y-0 right-0 pr-2.5 flex items-center text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}
      </div>

      {/* Dedicated draggable spacer — only this area triggers window drag */}
      <div
        className="flex-1 h-full min-w-[20px] cursor-default"
        data-tauri-drag-region
        onDoubleClick={() => invoke("window_toggle_maximize")}
      />

      {/* Library link — responsive label */}
      <button
        type="button"
        data-tauri-drag-region="false"
        onClick={() => setActiveCategory("installed")}
        className="flex items-center gap-1.5 text-xs font-semibold text-[var(--rz-text)] hover:text-white transition-colors flex-shrink-0 px-2.5 py-1.5 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] cursor-pointer shadow-xs"
        title="Library — installed packages"
      >
        <DownloadCloud className="w-4 h-4 text-[var(--rz-accent)]" />
        <span className="hidden sm:inline">Library</span>
      </button>

      {/* Appearance Quick Switch Popover */}
      <div className="relative flex-shrink-0" ref={popoverRef} data-tauri-drag-region="false">
        <button
          type="button"
          data-tauri-drag-region="false"
          onClick={() => setPopoverOpen(!popoverOpen)}
          className="flex items-center justify-center w-8 h-8 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:text-white hover:border-[var(--rz-border-strong)] hover:bg-[var(--rz-surface-hover)] transition-all cursor-pointer shadow-xs"
          title={`Appearance: ${mode} (${resolvedTheme})`}
          aria-label="Change Appearance"
        >
          {mode === "light" ? (
            <Sun className="w-4 h-4 text-amber-500" />
          ) : mode === "dark" ? (
            <Moon className="w-4 h-4 text-[var(--rz-accent)]" />
          ) : (
            <Monitor className="w-4 h-4 text-[var(--rz-text-secondary)]" />
          )}
        </button>

        {popoverOpen && (
          <div
            className="absolute right-0 mt-2 w-52 rounded-xl p-2 z-50 text-xs shadow-2xl transition-all animate-in fade-in zoom-in-95 duration-100"
            style={{
              background: "var(--rz-glass-popover-bg)",
              border: "var(--rz-glass-popover-border)",
              backdropFilter: "var(--rz-glass-backdrop)",
              boxShadow: "var(--rz-shadow-lg)",
            }}
          >
            <div className="px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider text-[var(--rz-text-faint)]">
              Appearance
            </div>

            <div className="space-y-0.5">
              {appearanceOptions.map((opt) => {
                const isActive = mode === opt.id;
                return (
                  <button
                    key={opt.id}
                    type="button"
                    onClick={() => {
                      setMode(opt.id);
                    }}
                    className={[
                      "w-full flex items-center justify-between px-2.5 py-2 rounded-lg text-xs transition-colors cursor-pointer",
                      isActive
                        ? "bg-[var(--rz-surface-hover)] text-[var(--rz-text)] font-semibold"
                        : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)]",
                    ].join(" ")}
                  >
                    <div className="flex items-center gap-2">
                      <span className={isActive ? "text-[var(--rz-accent)]" : "text-[var(--rz-text-muted)]"}>
                        {opt.icon}
                      </span>
                      <span>{opt.label}</span>
                    </div>
                    {isActive && <Check className="w-4 h-4 text-[var(--rz-accent)]" />}
                  </button>
                );
              })}
            </div>

            <div className="my-1.5 border-t border-[var(--rz-border-subtle)]" />

            <div className="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-[var(--rz-text-faint)]">
              Accent
            </div>
            <div className="flex items-center justify-between px-2 py-1">
              {accents.map((ac) => {
                const isSelected = accent === ac;
                return (
                  <button
                    key={ac}
                    type="button"
                    onClick={() => setAccent(ac)}
                    className={[
                      "w-5 h-5 rounded-full flex items-center justify-center transition-all cursor-pointer",
                      ACCENT_SWATCHES[ac].bgClass,
                      isSelected ? "ring-2 ring-offset-2 ring-[var(--rz-accent)] scale-110" : "opacity-80 hover:opacity-100",
                    ].join(" ")}
                    title={ACCENT_SWATCHES[ac].name}
                  >
                    {isSelected && <Check className="w-3 h-3 text-white" />}
                  </button>
                );
              })}
            </div>
          </div>
        )}
      </div>

      {/* Client-Side Window Controls */}
      <div className="flex items-center ml-1 border-l border-[var(--rz-border-subtle)] pl-2 space-x-1 flex-shrink-0">
        <button
          type="button"
          data-tauri-drag-region="false"
          onClick={() => invoke("window_minimize")}
          className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
          title="Minimize"
          aria-label="Minimize Window"
        >
          <Minus className="w-4 h-4" />
        </button>
        <button
          type="button"
          data-tauri-drag-region="false"
          onClick={() => invoke("window_toggle_maximize")}
          className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
          title="Maximize"
          aria-label="Maximize Window"
        >
          <Square className="w-3.5 h-3.5" />
        </button>
        <button
          type="button"
          data-tauri-drag-region="false"
          onClick={() => invoke("window_close")}
          className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--rz-text-secondary)] hover:text-white hover:bg-rose-500 transition-colors cursor-pointer"
          title="Close"
          aria-label="Close Window"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
