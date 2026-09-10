import React from "react";

interface PreviewGalleryProps {
  screenshots: string[];
  activeIndex: number;
  onSelect: (index: number) => void;
  title: string;
}

export const PreviewGallery: React.FC<PreviewGalleryProps> = ({
  screenshots,
  activeIndex,
  onSelect,
  title,
}) => {
  if (!screenshots || screenshots.length <= 1) return null;

  return (
    <div className="flex items-center gap-2 mt-3 overflow-x-auto scrollbar-none py-1">
      {screenshots.map((shot, idx) => {
        const isActive = activeIndex === idx;
        return (
          <button
            key={idx}
            type="button"
            onClick={() => onSelect(idx)}
            className={[
              "relative aspect-video w-24 sm:w-28 rounded-lg overflow-hidden border transition-all cursor-pointer shrink-0 select-none",
              isActive
                ? "border-[var(--rz-accent)] ring-2 ring-[var(--rz-accent)]/30 shadow-md"
                : "border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] opacity-70 hover:opacity-100",
            ].join(" ")}
            aria-label={`View screenshot ${idx + 1} of ${title}`}
          >
            <img
              src={shot}
              alt={`${title} thumbnail ${idx + 1}`}
              className="w-full h-full object-cover"
              loading="lazy"
            />
            {isActive && (
              <div className="absolute inset-0 bg-[var(--rz-accent)]/10 pointer-events-none" />
            )}
          </button>
        );
      })}
    </div>
  );
};
