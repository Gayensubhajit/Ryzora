import React from "react";
import { AlertTriangle, Check } from "lucide-react";

interface CompatibilityBadgeProps {
  label: string;
  isWarning?: boolean;
  className?: string;
}

export const CompatibilityBadge: React.FC<CompatibilityBadgeProps> = ({
  label,
  isWarning = false,
  className = "",
}) => {
  if (isWarning) {
    return (
      <span
        className={[
          "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-amber-500/20 text-amber-300 border border-amber-500/35 tracking-tight backdrop-blur-xs",
          className,
        ].join(" ")}
      >
        <AlertTriangle className="w-2.5 h-2.5 text-amber-400 shrink-0" />
        <span className="truncate">{label}</span>
      </span>
    );
  }

  return (
    <span
      className={[
        "inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-[var(--rz-surface-elevated)]/90 text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)] tracking-tight backdrop-blur-xs",
        className,
      ].join(" ")}
    >
      <Check className="w-2.5 h-2.5 text-emerald-400 shrink-0" />
      <span className="truncate">{label.replace(/^✓\s*/, "")}</span>
    </span>
  );
};
