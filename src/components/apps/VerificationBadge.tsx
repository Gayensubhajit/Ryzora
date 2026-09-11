import React, { useState, useRef, useEffect } from "react";
import { ShieldCheck, CheckCircle2, X } from "lucide-react";
import type { VerificationDetails } from "./appState.ts";

interface VerificationBadgeProps {
  details: VerificationDetails;
  packageName: string;
}

export const VerificationBadge: React.FC<VerificationBadgeProps> = ({ details, packageName }) => {
  const [popoverOpen, setPopoverOpen] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("openModal") === "verification";
    } catch {
      return false;
    }
  });
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setPopoverOpen(false);
      }
    };
    if (popoverOpen) {
      document.addEventListener("mousedown", handleOutsideClick);
    }
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
    };
  }, [popoverOpen]);

  const isOfficial = details.level === "official" || details.level === "verified";

  return (
    <div ref={containerRef} className="relative inline-block">
      <button
        type="button"
        onClick={() => setPopoverOpen((p) => !p)}
        className={[
          "inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold transition-all cursor-pointer",
          isOfficial
            ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border border-emerald-500/25 hover:bg-emerald-500/20"
            : "bg-zinc-500/10 text-[var(--rz-text-muted)] border border-zinc-500/20 hover:bg-zinc-500/20",
        ].join(" ")}
        title="Click for verification details"
      >
        <ShieldCheck size={13} className="shrink-0" />
        <span>{details.badgeLabel}</span>
      </button>

      {popoverOpen && (
        <div
          className={[
            "absolute left-0 top-full mt-2 w-80 rounded-2xl z-50 p-4",
            "bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)]",
            "shadow-2xl backdrop-blur-xl animate-fadeIn space-y-3",
          ].join(" ")}
          role="dialog"
          aria-label="Source Verification Details"
        >
          <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-2">
            <div className="flex items-center gap-2">
              <ShieldCheck size={16} className={isOfficial ? "text-emerald-500" : "text-[var(--rz-text-muted)]"} />
              <span className="text-xs font-bold text-[var(--rz-text)]">Source verification</span>
            </div>
            <button
              onClick={() => setPopoverOpen(false)}
              className="p-1 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] cursor-pointer"
            >
              <X size={13} />
            </button>
          </div>

          <div className="space-y-2 text-xs">
            <div>
              <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Source</div>
              <div className="font-semibold text-[var(--rz-text)]">{details.sourceType}</div>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div>
                <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Repository</div>
                <div className="font-mono text-[11px] text-[var(--rz-text)]">{details.repository}</div>
              </div>
              <div>
                <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Package</div>
                <div className="font-mono text-[11px] text-[var(--rz-text)]">{packageName}</div>
              </div>
            </div>

            <div>
              <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Package signature</div>
              <div className="text-[11px] text-[var(--rz-text-secondary)] flex items-center gap-1.5 pt-0.5">
                <CheckCircle2 size={12} className="text-emerald-500" />
                <span>{details.signatureStatus}</span>
              </div>
            </div>

            <div>
              <div className="text-[10px] uppercase font-bold text-[var(--rz-text-muted)]">Maintainer</div>
              <div className="text-[11px] text-[var(--rz-text-secondary)]">{details.maintainer}</div>
            </div>
          </div>

          <div className="p-2.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[11px] text-[var(--rz-text-muted)] leading-relaxed">
            {details.description}
          </div>
        </div>
      )}
    </div>
  );
};
