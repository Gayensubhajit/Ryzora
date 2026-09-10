import React from "react";
import { Check, DownloadCloud, Eye, AlertTriangle, Loader2 } from "lucide-react";
import { PackageItem } from "../../types";

interface StickyActionBarProps {
  packageItem: PackageItem;
  isInstalled: boolean;
  isUpdateAvailable: boolean;
  isInstalling: boolean;
  installProgress: number;
  missingDependencies?: string[];
  selectedTarget?: "quickshell" | "sddm" | "both";
  onInstall: () => void;
  onPreview: () => void;
  isActive?: boolean;
  onApply?: () => void;
  onDeactivate?: () => void;
  isApplying?: boolean;
}

export const StickyActionBar: React.FC<StickyActionBarProps> = ({
  packageItem,
  isInstalled,
  isUpdateAvailable,
  isInstalling,
  missingDependencies = [],
  selectedTarget = "quickshell",
  onInstall,
  onPreview,
  isActive = false,
  onApply,
  onDeactivate,
  isApplying = false,
}) => {
  const isBlocked = missingDependencies.length > 0;
  const isSddm = selectedTarget === "sddm" || (!packageItem.supports_session_lock && packageItem.supports_login_screen);
  const isBoth = selectedTarget === "both";

  const getButtonLabel = () => {
    if (isInstalling) return "Installing...";
    if (isUpdateAvailable) return `Update to v${packageItem.version}`;
    if (isInstalled) return "Installed";
    if (isBoth) return "Install Both";
    if (isSddm) return "Install Login Screen";
    return "Install Session Lock";
  };

  return (
    <div className="sticky bottom-0 inset-x-0 z-30 p-3 sm:px-6 bg-[var(--rz-bg)]/90 backdrop-blur-md border-t border-[var(--rz-border-subtle)] shadow-lg flex items-center justify-between gap-3 -mx-4 sm:-mx-6 mt-8">
      {/* Left: Compatibility / Status indicator */}
      <div className="flex items-center gap-2 min-w-0">
        {isBlocked ? (
          <div className="flex items-center gap-1.5 text-xs text-rose-400 font-medium truncate">
            <AlertTriangle className="w-4 h-4 shrink-0" />
            <span className="truncate">Missing dependency: {missingDependencies.join(", ")}</span>
          </div>
        ) : isBoth ? (
          <div className="flex items-center gap-1.5 text-xs text-sky-400 font-medium truncate">
            <Check className="w-4 h-4 shrink-0" />
            <span className="truncate">Dual: User Session Lock + System Greeter</span>
          </div>
        ) : isSddm ? (
          <div className="flex items-center gap-1.5 text-xs text-amber-400 font-medium truncate">
            <AlertTriangle className="w-4 h-4 shrink-0" />
            <span className="truncate">System login screen · Requires admin approval (pkexec)</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
            <Check className="w-4 h-4 shrink-0" />
            <span className="truncate">Compatible with your system · User space</span>
          </div>
        )}
      </div>

      {/* Right: Actions */}
      <div className="flex items-center gap-2 shrink-0">
        <button
          type="button"
          onClick={onPreview}
          className="px-3.5 py-1.5 rounded-lg text-xs font-semibold border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer select-none flex items-center gap-1.5"
        >
          <Eye className="w-3.5 h-3.5" />
          <span>Dry Run</span>
        </button>

        {!isInstalled ? (
          <button
            type="button"
            disabled={isBlocked || isInstalling}
            onClick={onInstall}
            className={[
              "px-4 py-1.5 rounded-lg text-xs font-bold transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-sm",
              isBlocked
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                : isSddm || isBoth
                ? "bg-amber-600 hover:bg-amber-500 text-white"
                : "bg-[var(--rz-accent)] hover:bg-[var(--rz-accent)]/90 text-white",
            ].join(" ")}
          >
            {isInstalling ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Installing...</span>
              </>
            ) : isUpdateAvailable ? (
              <>
                <DownloadCloud className="w-3.5 h-3.5" />
                <span>{getButtonLabel()}</span>
              </>
            ) : (
              <>
                <DownloadCloud className="w-3.5 h-3.5" />
                <span>{getButtonLabel()}</span>
              </>
            )}
          </button>
        ) : !isActive ? (
          <button
            type="button"
            disabled={isApplying}
            onClick={onApply}
            className="px-4 py-1.5 rounded-lg text-xs font-bold transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-sm bg-emerald-600 hover:bg-emerald-500 text-white"
          >
            {isApplying ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Activating...</span>
              </>
            ) : (
              <>
                <Check className="w-3.5 h-3.5" />
                <span>{isBoth ? "Apply Both" : isSddm ? "Apply Login Screen" : "Apply Session Lock"}</span>
              </>
            )}
          </button>
        ) : (
          <div className="flex items-center gap-1.5">
            <div className="px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 text-white shadow-sm flex items-center gap-1 cursor-default select-none border border-emerald-400/30">
              <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
              <span>✓ Active</span>
            </div>
            {onDeactivate && (
              <button
                type="button"
                disabled={isApplying}
                onClick={onDeactivate}
                className="px-2.5 py-1.5 rounded-lg text-xs font-semibold bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-300 transition-all cursor-pointer select-none"
              >
                Deactivate
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
