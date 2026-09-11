import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";

interface VirtualizedAppGridProps<T> {
  items: T[];
  renderItem: (item: T, index: number) => React.ReactNode;
  itemHeight?: number;
  gap?: number;
  emptyMessage?: React.ReactNode;
  className?: string;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  onLoadMore?: () => void;
  /** Pass the scrolling ancestor element so the grid tracks the right scroll container.
   *  Defaults to window if not provided. */
  scrollContainerRef?: React.RefObject<HTMLElement | null>;
}

export function VirtualizedAppGrid<T>({
  items,
  renderItem,
  itemHeight = 156,
  gap = 16,
  emptyMessage,
  className = "",
  hasMore = false,
  isLoadingMore = false,
  onLoadMore,
  scrollContainerRef,
}: VirtualizedAppGridProps<T>): React.ReactElement {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [columns, setColumns] = useState<number>(6);
  const [scrollTop, setScrollTop] = useState<number>(0);
  const [containerHeight, setContainerHeight] = useState<number>(800);

  // Responsive column calculation based on container width
  const updateDimensions = useCallback(() => {
    if (!containerRef.current) return;
    const width = containerRef.current.clientWidth;
    let cols = 6;
    if (width < 480) cols = 2;
    else if (width < 680) cols = 3;
    else if (width < 960) cols = 4;
    else if (width < 1200) cols = 5;
    else cols = 6;
    setColumns(cols);
    const scrollEl = scrollContainerRef?.current;
    setContainerHeight(scrollEl ? scrollEl.clientHeight : window.innerHeight || 800);
  }, [scrollContainerRef]);

  useEffect(() => {
    updateDimensions();
    window.addEventListener("resize", updateDimensions);
    return () => window.removeEventListener("resize", updateDimensions);
  }, [updateDimensions]);

  // Track scroll on the correct container (AppsView overflow-y-auto div or window)
  useEffect(() => {
    const scrollEl: HTMLElement | Window = scrollContainerRef?.current ?? window;

    const handleScroll = () => {
      if (scrollContainerRef?.current) {
        const containerEl = scrollContainerRef.current;
        const gridEl = containerRef.current;
        if (!gridEl) return;
        const gridTop = gridEl.offsetTop;
        const relativeScroll = Math.max(0, containerEl.scrollTop - gridTop);
        setScrollTop(relativeScroll);
      } else {
        if (!containerRef.current) return;
        const rect = containerRef.current.getBoundingClientRect();
        setScrollTop(Math.max(0, -rect.top));
      }
    };

    scrollEl.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll(); // initial
    return () => scrollEl.removeEventListener("scroll", handleScroll);
  }, [scrollContainerRef]);

  const totalItems = items.length;
  const effectiveRowHeight = itemHeight + gap;
  const totalRows = Math.ceil(totalItems / columns);

  const overscan = 2;
  const startRow = Math.max(0, Math.floor(scrollTop / effectiveRowHeight) - overscan);
  const endRow = Math.min(
    totalRows,
    Math.ceil((scrollTop + containerHeight) / effectiveRowHeight) + overscan
  );

  // Auto-trigger loadMore when near end of rendered items
  useEffect(() => {
    if (hasMore && !isLoadingMore && onLoadMore && totalRows > 0) {
      if (endRow >= totalRows - 2) {
        onLoadMore();
      }
    }
  }, [endRow, totalRows, hasMore, isLoadingMore, onLoadMore]);

  const visibleItems = useMemo(() => {
    const result: { item: T; index: number; row: number; col: number }[] = [];
    const startIndex = startRow * columns;
    const endIndex = Math.min(totalItems, endRow * columns);
    for (let i = startIndex; i < endIndex; i++) {
      const row = Math.floor(i / columns);
      const col = i % columns;
      result.push({ item: items[i], index: i, row, col });
    }
    return result;
  }, [items, startRow, endRow, columns, totalItems]);

  if (totalItems === 0) {
    return <>{emptyMessage ?? null}</>;
  }

  const topPadding = startRow * effectiveRowHeight;
  const bottomPadding = Math.max(0, (totalRows - endRow) * effectiveRowHeight);

  return (
    <div ref={containerRef} className={`relative w-full ${className}`}>
      {topPadding > 0 && <div style={{ height: `${topPadding}px` }} aria-hidden="true" />}

      <div
        className="grid gap-4"
        style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
      >
        {visibleItems.map(({ item, index }) => (
          <React.Fragment key={index}>{renderItem(item, index)}</React.Fragment>
        ))}
      </div>

      {bottomPadding > 0 && <div style={{ height: `${bottomPadding}px` }} aria-hidden="true" />}

      {hasMore && (
        <div className="flex justify-center py-6">
          <button
            type="button"
            onClick={onLoadMore}
            disabled={isLoadingMore}
            className="flex items-center gap-2 px-6 py-2.5 rounded-xl bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] text-xs font-semibold text-[var(--rz-text)] transition-all cursor-pointer shadow-xs disabled:opacity-50"
          >
            {isLoadingMore ? (
              <>
                <span className="w-3.5 h-3.5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" />
                <span>Loading more...</span>
              </>
            ) : (
              <span>Load More ({totalItems} loaded)</span>
            )}
          </button>
        </div>
      )}
    </div>
  );
}
