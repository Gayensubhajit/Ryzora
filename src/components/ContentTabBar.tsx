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
    setCanScrollLeft(el.scrollLeft > 6);
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 6);
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
    const amount = direction === "left" ? -200 : 200;
    containerRef.current.scrollBy({ left: amount, behavior: "smooth" });
  };

  return (
    <div className="flex items-center gap-1.5 min-w-0 py-2 w-full">
      {canScrollLeft && (
        <button
          type="button"
          onClick={() => scroll("left")}
          className="p-1.5 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:text-white hover:bg-[var(--rz-surface-hover)] transition-colors shrink-0 cursor-pointer shadow-xs"
          aria-label="Scroll tabs left"
        >
          <ChevronLeft className="w-4 h-4" />
        </button>
      )}

      <div
        ref={containerRef}
        className="flex items-center overflow-x-auto gap-1.5 scrollbar-none w-full scroll-smooth py-0.5"
        style={{ scrollbarWidth: "none" }}
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
                "relative flex-shrink-0 px-3.5 py-1.5 rounded-lg text-xs sm:text-[13px] font-semibold transition-all whitespace-nowrap cursor-pointer",
                isActive
                  ? "bg-[var(--rz-accent)] text-white shadow-xs font-bold"
                  : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-transparent hover:border-[var(--rz-border-subtle)]",
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
          className="p-1.5 rounded-lg border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:text-white hover:bg-[var(--rz-surface-hover)] transition-colors shrink-0 cursor-pointer shadow-xs"
          aria-label="Scroll tabs right"
        >
          <ChevronRight className="w-4 h-4" />
        </button>
      )}
    </div>
  );
};
