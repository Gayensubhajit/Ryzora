import React, { useEffect } from "react";
import { ArrowLeft, X } from "lucide-react";

interface ProductDetailShellProps {
  onClose: () => void;
  bgImage?: string;
  categoryLabel?: string;
  children: React.ReactNode;
}

export const ProductDetailShell: React.FC<ProductDetailShellProps> = ({
  onClose,
  bgImage,
  categoryLabel = "Lock Screens",
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

      {/* ── Top Fixed Navigation Bar ── */}
      <header className="relative z-10 flex items-center justify-between px-4 sm:px-8 py-3.5 border-b border-[var(--rz-border-subtle)] bg-[var(--rz-bg)]/95 backdrop-blur-md shrink-0">
        <button
          type="button"
          onClick={onClose}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs sm:text-[13px] font-semibold text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] transition-all cursor-pointer select-none"
          aria-label={`Back to ${categoryLabel}`}
        >
          <ArrowLeft className="w-4 h-4" />
          <span>Back to {categoryLabel}</span>
        </button>

        <button
          type="button"
          onClick={onClose}
          className="p-1.5 rounded-lg border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] transition-colors cursor-pointer select-none"
          aria-label="Close detail view"
        >
          <X className="w-4 h-4" />
        </button>
      </header>

      {/* ── Scrollable Body ── */}
      <div className="relative z-10 flex-1 overflow-y-auto min-w-0">
        <div className="max-w-6xl mx-auto px-4 sm:px-8 py-6">
          {children}
        </div>
      </div>
    </div>
  );
};
