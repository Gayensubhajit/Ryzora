import React from "react";
import { SlidersHorizontal, X, ArrowDownUp } from "lucide-react";

export type SortMode =
  | "recommended"
  | "downloads"
  | "newest"
  | "updated"
  | "rating"
  | "alpha";

interface FilterBarProps {
  showFilters: boolean;
  onToggleFilters: () => void;
  activeFilterCount: number;
  sortBy: SortMode;
  onChangeSort: (sort: SortMode) => void;
  onClearFilters?: () => void;
  hasActiveFilters?: boolean;
}

export const FilterBar: React.FC<FilterBarProps> = ({
  showFilters,
  onToggleFilters,
  activeFilterCount,
  sortBy,
  onChangeSort,
  onClearFilters,
  hasActiveFilters = false,
}) => {
  return (
    <div className="flex items-center justify-between gap-2.5 mb-3 flex-wrap">
      <div className="flex items-center gap-2">
        {/* Toggle Filter Panel */}
        <button
          type="button"
          onClick={onToggleFilters}
          className={[
            "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border transition-all cursor-pointer select-none",
            showFilters
              ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-accent-text)]"
              : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)]",
          ].join(" ")}
        >
          <SlidersHorizontal className="w-3.5 h-3.5" />
          <span>Filters</span>
          {activeFilterCount > 0 && (
            <span className="w-4 h-4 rounded-full text-[10px] font-bold bg-[var(--rz-accent)] text-white flex items-center justify-center ml-0.5">
              {activeFilterCount}
            </span>
          )}
        </button>

        {/* Clear filters if active */}
        {hasActiveFilters && onClearFilters && (
          <button
            type="button"
            onClick={onClearFilters}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)] transition-colors cursor-pointer"
          >
            <X className="w-3 h-3" />
            <span>Reset</span>
          </button>
        )}
      </div>

      {/* Simplified 6-Option Sort Dropdown */}
      <div className="flex items-center gap-1.5 ml-auto">
        <ArrowDownUp className="w-3.5 h-3.5 text-[var(--rz-text-muted)] shrink-0 hidden sm:block" />
        <span className="text-xs text-[var(--rz-text-secondary)] hidden sm:inline font-medium">
          Sort:
        </span>
        <select
          value={sortBy}
          onChange={(e) => onChangeSort(e.target.value as SortMode)}
          className="ryz-select text-xs font-medium py-1.5 px-2.5"
          aria-label="Sort packages by"
        >
          <option value="recommended">Recommended</option>
          <option value="downloads">Most Downloaded</option>
          <option value="newest">Newest</option>
          <option value="updated">Recently Updated</option>
          <option value="rating">Highest Rated</option>
          <option value="alpha">Alphabetical</option>
        </select>
      </div>
    </div>
  );
};
