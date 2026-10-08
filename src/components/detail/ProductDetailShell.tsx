import React, { useEffect } from "react";
import { ArrowLeft, X, Minus, Square } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface ProductDetailShellProps {
  onClose: () => void;
  bgImage?: string;
  categoryLabel?: string;
  headerCenter?: React.ReactNode;
  headerRightActions?: React.ReactNode;
  children: React.ReactNode;
}

export const ProductDetailShell: React.FC<ProductDetailShellProps> = ({
  onClose,
  bgImage,
  categoryLabel = "Lock Screens",
  headerCenter,
  headerRightActions,
  children,
}) => {
  // Handle ESC key to close
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[var(--rz-bg)] text-[var(--rz-text)] antialiased overflow-hidden opacity-100 animate-in fade-in duration-200">
      {/* ── Ambient Blurred Artwork Background (Detail View only) ── */}
      {bgImage && (
        <div className="absolute inset-0 pointer-events-none overflow-hidden z-0">
          <div
            className="absolute inset-0 bg-cover bg-center opacity-10 dark:opacity-15 blur-3xl scale-125"
            style={{ backgroundImage: `url(${bgImage})` }}
          />
          <div className="absolute inset-0 bg-gradient-to-b from-[var(--rz-bg)]/85 via-[var(--rz-bg)]/95 to-[var(--rz-bg)]" />
        </div>
      )}

      {/* ── Top Fixed Navigation Bar (Full Width) ── */}
      <header className="relative z-10 flex items-center justify-between px-6 sm:px-8 lg:px-10 py-2.5 border-b border-[var(--rz-border-subtle)] bg-[var(--rz-bg)]/95 backdrop-blur-md shrink-0">
        <div className="flex items-center gap-4 shrink-0">
          <button
            type="button"
            data-tauri-drag-region="false"
            onClick={onClose}
            className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs sm:text-[13px] font-semibold text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] transition-all cursor-pointer select-none"
            aria-label={`Back to ${categoryLabel}`}
          >
            <ArrowLeft className="w-4 h-4" />
            <span>Back to {categoryLabel}</span>
          </button>

          {headerCenter && (
            <div className="hidden sm:flex items-center gap-2 pl-2 border-l border-[var(--rz-border-subtle)]">
              {headerCenter}
            </div>
          )}
        </div>

        {/* Dedicated draggable spacer — window drag & double-click toggle maximize */}
        <div
          className="flex-1 h-full min-w-[20px] cursor-default self-stretch"
          data-tauri-drag-region
          onDoubleClick={() => invoke("window_toggle_maximize")}
        />

        {/* Client-Side Window Controls & Header Actions */}
        <div className="flex items-center space-x-2 shrink-0">
          {headerRightActions && (
            <div className="flex items-center mr-2">{headerRightActions}</div>
          )}

          <div className="flex items-center space-x-1 pl-2 border-l border-[var(--rz-border-subtle)]">
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
              className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--rz-text-secondary)] hover:text-red-400 hover:bg-red-500/10 transition-colors cursor-pointer"
              title="Close"
              aria-label="Close Window"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      </header>

      {/* ── Scrollable Body (Full Width Viewport, no artificial gutters) ── */}
      <div className="relative z-10 flex-1 overflow-y-auto min-w-0">
        <div className="w-full px-6 sm:px-8 lg:px-10 py-6 space-y-6">
          {children}
        </div>
      </div>
    </div>
  );
};
