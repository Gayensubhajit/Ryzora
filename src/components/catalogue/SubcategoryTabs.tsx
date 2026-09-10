import React from "react";

interface SubcategoryTabsProps {
  subFilters: string[];
  activeSubFilter: string;
  onSelectSubFilter: (subFilter: string) => void;
  category?: string;
}

export const SubcategoryTabs: React.FC<SubcategoryTabsProps> = ({
  subFilters,
  activeSubFilter,
  onSelectSubFilter,
}) => {
  if (!subFilters || subFilters.length <= 1) return null;

  return (
    <div
      className="flex items-center gap-1.5 overflow-x-auto scrollbar-none py-0.5 mb-2.5"
      style={{ scrollbarWidth: "none" }}
    >
      {subFilters.map((filter) => {
        const isActive = activeSubFilter === filter;
        const isSddm = filter.toLowerCase() === "sddm";

        return (
          <button
            key={filter}
            type="button"
            onClick={() => onSelectSubFilter(filter)}
            className={[
              "flex-shrink-0 flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer select-none",
              isActive
                ? "bg-[var(--rz-accent)] text-white shadow-xs font-bold"
                : "bg-[var(--rz-surface)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] hover:text-[var(--rz-text)]",
            ].join(" ")}
          >
            {isSddm && (
              <span
                className={[
                  "w-1.5 h-1.5 rounded-full shrink-0",
                  isActive ? "bg-amber-300" : "bg-amber-400",
                ].join(" ")}
                title="System-level login screen"
              />
            )}
            <span>{filter}</span>
            {isSddm && (
              <span
                className={[
                  "text-[9px] uppercase font-mono px-1 py-0.2 rounded font-medium",
                  isActive ? "bg-black/20 text-white/90" : "bg-amber-500/10 text-amber-400",
                ].join(" ")}
              >
                Login
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};
