import React from "react";
import { Lock, Sparkles } from "lucide-react";
import { CategoryId } from "../../types";

interface CategoryHeaderProps {
  category: CategoryId | "all";
  count: number;
  subFilter?: string;
}

export const CategoryHeader: React.FC<CategoryHeaderProps> = ({
  category,
  count,
  subFilter,
}) => {
  if (category === "all") return null;

  const isLockScreens = category === "lockscreens";

  const getTitle = () => {
    switch (category) {
      case "lockscreens":
        return "Lock Screens";
      case "rices":
        return "Complete Rices";
      case "themes":
        return "Themes";
      case "wallpapers":
        return "Wallpapers";
      case "bars":
        return "Status Bars";
      case "fastfetch":
        return "Fastfetch Profiles";
      case "terminal":
        return "Terminals & Shells";
      case "bundles":
        return "Curated Bundles";
      default:
        return String(category).toUpperCase();
    }
  };

  const getSubtitle = () => {
    switch (category) {
      case "lockscreens":
        return "Session locks & display manager login screens for modern Wayland & X11 desktops.";
      case "rices":
        return "Cohesive visual desktop configurations including window manager, bar, and terminal.";
      case "themes":
        return "Coordinated color schemes, GTK/Qt themes, and desktop styling.";
      case "wallpapers":
        return "High-resolution desktop art and embedded color palettes.";
      case "bars":
        return "Modular Waybar, Eww, and status panel configurations.";
      case "fastfetch":
        return "ASCII logo art, system telemetry presets, and neofetch replacements.";
      case "terminal":
        return "Color palettes and configuration profiles for Kitty, Alacritty, Foot, and Starship.";
      case "bundles":
        return "Ecosystem-wide packages bundling themes, lockscreens, bars, and rices.";
      default:
        return "Explore community and verified configurations.";
    }
  };

  return (
    <div className="mb-2.5">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <div className="p-1 rounded-md bg-[var(--rz-accent)]/10 text-[var(--rz-accent-text)] border border-[var(--rz-accent)]/20">
            {isLockScreens ? <Lock className="w-3.5 h-3.5" /> : <Sparkles className="w-3.5 h-3.5" />}
          </div>
          <h1 className="text-base font-bold tracking-tight text-[var(--rz-text)]">
            {getTitle()}
          </h1>
          <span className="text-[10px] font-mono px-2 py-0.5 rounded-full bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)] font-medium">
            {count} {count === 1 ? "item" : "items"}
            {subFilter && subFilter !== "All" ? ` · ${subFilter}` : ""}
          </span>
        </div>
      </div>

      <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 max-w-2xl leading-normal">
        {getSubtitle()}
      </p>

      {/* Compressed & Quieter Session Lock / Login Screen Banner */}
      {isLockScreens && (
        <div className="mt-2 flex items-center gap-2 flex-wrap text-[11px]">
          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)]">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 shrink-0" />
            <span className="font-semibold text-[var(--rz-text)]">Session Lock:</span>
            <span className="text-[var(--rz-text-muted)] text-[10.5px]">Hyprlock · Quickshell · Swaylock (User session)</span>
          </div>

          <div className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-[var(--rz-surface)] border border-amber-500/25 text-[var(--rz-text-secondary)]">
            <span className="w-1.5 h-1.5 rounded-full bg-amber-400 shrink-0" />
            <span className="font-semibold text-[var(--rz-text)]">Login Screen:</span>
            <span className="text-[var(--rz-text-muted)] text-[10.5px]">SDDM (System display manager)</span>
          </div>
        </div>
      )}
    </div>
  );
};
