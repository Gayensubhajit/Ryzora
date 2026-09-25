import React from "react";
import { Check, ArrowUp, AlertTriangle, Loader2 } from "lucide-react";

interface InstalledBadgeProps {
  isInstalled: boolean;
  isUpdateAvailable: boolean;
  isIncompatible?: boolean;
  isInstalling?: boolean;
  installMessage?: string;
}

export const InstalledBadge: React.FC<InstalledBadgeProps> = ({
  isInstalled,
  isUpdateAvailable,
  isIncompatible,
  isInstalling,
  installMessage,
}) => {
  if (isInstalling) {
    return (
      <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-[10px] font-semibold bg-[var(--rz-accent)]/20 text-[var(--rz-accent)] border border-[var(--rz-accent)]/40 backdrop-blur-xs">
        <Loader2 className="w-3 h-3 animate-spin" />
        {installMessage || "Installing…"}
      </span>
    );
  }

  if (isIncompatible) {
    return (
      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/30 backdrop-blur-xs">
        <AlertTriangle className="w-2.5 h-2.5" />
        Incompatible
      </span>
    );
  }

  if (isUpdateAvailable) {
    return (
      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/20 text-amber-300 border border-amber-400/40 backdrop-blur-xs">
        <ArrowUp className="w-2.5 h-2.5" />
        Update
      </span>
    );
  }

  if (isInstalled) {
    return (
      <span className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-400/40 backdrop-blur-xs">
        <Check className="w-2.5 h-2.5" />
        Installed
      </span>
    );
  }

  return null;
};
