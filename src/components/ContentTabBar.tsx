import React, { useRef, useState, useEffect } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { CategoryId } from "../types";

export interface ContentTab {
  id: CategoryId | "all";
  label: string;
}

export const STORE_CONTENT_TABS: ContentTab[] = [
  { id: "all",         label: "All" },
  { id: "lockscreens", label: "Lock Screens" },
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
  const containerRef = useRef<HTMLDivElement>(null);
  const activeBtnRef = useRef<HTMLButtonElement | null>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);

  const checkScroll = () => {
    const el = containerRef.current;
    if (!el) return;
    setCanScrollLeft(el.scrollLeft > 8);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 8);
  };

  useEffect(() => {
    checkScroll();
    const el = containerRef.current;
    if (!el) return;
    el.addEventListener("scroll", checkScroll, { passive: true });
    window.addEventListener("resize", checkScroll);
    return () => {
      el.removeEventListener("scroll", checkScroll);
      window.removeEventListener("resize", checkScroll);
    };
  }, [tabs]);

  useEffect(() => {
    if (activeBtnRef.current) {
      activeBtnRef.current.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "nearest",
      });
    }
  }, [activeTab]);

  const scroll = (direction: "left" | "right") => {
    if (!containerRef.current) return;
    const amount = direction === "left" ? -240 : 240;
    containerRef.current.scrollBy({ left: amount, behavior: "smooth" });
  };

  return (
    <nav aria-label="Catalogue categories" className="relative w-full flex items-center group">
      {canScrollLeft && (
        <button
          type="button"
          onClick={() => scroll("left")}
          className="absolute left-0 z-10 p-1.5 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]/95 text-[var(--rz-text)] hover:text-white hover:bg-[var(--rz-surface-hover)] shadow-md backdrop-blur-sm transition-all cursor-pointer -translate-x-1"
          aria-label="Scroll categories left"
        >
          <ChevronLeft className="w-4 h-4" />
        </button>
      )}

      <div
        ref={containerRef}
        className="flex items-center gap-2 overflow-x-auto w-full py-1 scroll-smooth select-none"
        style={{ scrollbarWidth: "none", msOverflowStyle: "none" }}
      >
        {tabs.map((tab) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              ref={isActive ? activeBtnRef : null}
              onClick={() => onTabChange(tab.id)}
              className={[
                "relative flex-shrink-0 px-4 py-2 rounded-xl text-[13px] font-semibold transition-all duration-200 whitespace-nowrap cursor-pointer",
                isActive
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text)] border border-[var(--rz-accent)] shadow-sm ring-1 ring-[var(--rz-accent)]/20 font-bold"
                  : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)]/60 bg-[var(--rz-surface)]/40",
              ].join(" ")}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      {canScrollRight && (
        <button
          type="button"
          onClick={() => scroll("right")}
          className="absolute right-0 z-10 p-1.5 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]/95 text-[var(--rz-text)] hover:text-white hover:bg-[var(--rz-surface-hover)] shadow-md backdrop-blur-sm transition-all cursor-pointer translate-x-1"
          aria-label="Scroll categories right"
        >
          <ChevronRight className="w-4 h-4" />
        </button>
      )}
    </nav>
  );
};
