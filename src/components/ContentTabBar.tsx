import React, { useRef } from "react";
import { CategoryId } from "../types";

export interface ContentTab {
  id: CategoryId | "all";
  label: string;
  emoji?: string;
}

export const STORE_CONTENT_TABS: ContentTab[] = [
  { id: "all",         label: "All" },
  { id: "lockscreens", label: "Lockscreens" },
  { id: "rices",       label: "Rices" },
  { id: "themes",      label: "Themes" },
  { id: "wallpapers",  label: "Wallpapers" },
  { id: "bars",        label: "Bars" },
  { id: "fastfetch",   label: "Fastfetch" },
  { id: "terminal",    label: "Terminals" },
  { id: "bundles",     label: "Bundles" },
];

interface ContentTabBarProps {
  activeTab: CategoryId | "all";
  onTabChange: (tab: CategoryId | "all") => void;
  tabs?: ContentTab[];
}

export const ContentTabBar: React.FC<ContentTabBarProps> = ({
  activeTab,
  onTabChange,
  tabs = STORE_CONTENT_TABS,
}) => {
  const scrollRef = useRef<HTMLDivElement>(null);

  return (
    <div className="relative border-b border-[var(--border-subtle)] bg-[var(--bg-sidebar)]">
      <div
        ref={scrollRef}
        className="flex items-center gap-0.5 overflow-x-auto px-4 py-0 scrollbar-none"
        style={{ scrollbarWidth: "none", msOverflowStyle: "none" }}
      >
        {tabs.map((tab) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => onTabChange(tab.id)}
              className={[
                "relative flex-shrink-0 px-3.5 py-2.5 text-xs font-medium transition-all whitespace-nowrap",
                "border-b-2 -mb-px focus:outline-none",
                isActive
                  ? "text-[var(--text-primary)] border-[var(--accent)]"
                  : "text-[var(--text-faint)] border-transparent hover:text-[var(--text-muted)] hover:border-[var(--border-strong)]",
              ].join(" ")}
            >
              {tab.label}
            </button>
          );
        })}
      </div>
    </div>
  );
};
