import React, { useState } from "react";
import { Check, DownloadCloud, AlertTriangle, Loader2, Lock, Trash2, MoreHorizontal } from "lucide-react";
import { PackageItem } from "../../types";

interface StickyActionBarProps {
  packageItem: PackageItem;
  isInstalled: boolean;
  isUpdateAvailable: boolean;
  isInstalling: boolean;
  installProgress?: number;
  missingDependencies?: string[];
  selectedTarget?: "quickshell" | "sddm" | "both";
  onInstall: () => void;
  onPreview?: () => void;
  isActive?: boolean;
  onApply?: () => void;
  onDeactivate?: () => void;
  onTest?: () => void;
  onUninstall?: () => void;
  isApplying?: boolean;
  isOverridden?: boolean;
  overriddenBy?: string | null;
}

export const StickyActionBar: React.FC<StickyActionBarProps> = ({
  packageItem,
  isInstalled,
  isUpdateAvailable,
  isInstalling,
  missingDependencies = [],
  selectedTarget = "both",
  onInstall,
  onTest,
  onUninstall,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const [showOverflow, setShowOverflow] = useState(false);
  const isBlocked = missingDependencies.length > 0;
  const isBoth = selectedTarget === "both";
  const isSddm = selectedTarget === "sddm";

  const handleConfirmUninstall = () => {
    setShowUninstallConfirm(false);
    if (onUninstall) {
      onUninstall();
    }
  };

  return (
    <>
      <div className="sticky bottom-0 inset-x-0 z-30 py-3.5 px-6 sm:px-8 lg:px-10 bg-[var(--rz-bg)]/95 backdrop-blur-md border-t border-[var(--rz-border-subtle)] shadow-lg flex items-center justify-between gap-4 -mx-6 sm:-mx-8 lg:-mx-10 mt-8">
        {/* Left: Status indicator */}
        <div className="flex items-center gap-2 min-w-0">
          {isBlocked ? (
            <div className="flex items-center gap-1.5 text-xs text-rose-400 font-medium truncate">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span className="truncate">Missing dependency: {missingDependencies.join(", ")}</span>
            </div>
          ) : !isInstalled ? (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0" />
              <span className="truncate">Ready to install</span>
            </div>
          ) : isBoth ? (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0 stroke-[3]" />
              <span className="truncate">Installed for Session + Login</span>
            </div>
          ) : isSddm ? (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0 stroke-[3]" />
              <span className="truncate">Installed for Login Screen</span>
            </div>
          ) : (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0 stroke-[3]" />
              <span className="truncate">Installed for Session Lock</span>
            </div>
          )}
        </div>

        {/* Right: Lifecycle Actions */}
        <div className="flex items-center gap-2 shrink-0">
          {!isInstalled ? (
            <button
              type="button"
              disabled={isBlocked || isInstalling}
              onClick={onInstall}
              className={[
                "px-5 py-2 rounded-xl text-xs font-bold transition-all cursor-pointer select-none flex items-center gap-2 shadow-sm",
                isBlocked
                  ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)] cursor-not-allowed opacity-60"
                  : "bg-[var(--rz-accent,#4f8ff7)] hover:bg-[var(--rz-accent,#4f8ff7)]/90 text-white",
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
                  <span>Update</span>
                </>
              ) : (
                <>
                  <DownloadCloud className="w-3.5 h-3.5" />
                  <span>Install</span>
                </>
              )}
            </button>
          ) : (
            <div className="flex items-center gap-2">
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-4 py-2 rounded-xl text-xs font-semibold bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Test lockscreen reversibly on screen"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              {onUninstall && (
                <div className="relative">
                  <button
                    type="button"
                    onClick={() => setShowOverflow(!showOverflow)}
                    className="p-2 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] transition-all cursor-pointer"
                    aria-label="More options"
                    title="More options"
                  >
                    <MoreHorizontal className="w-4 h-4" />
                  </button>

                  {showOverflow && (
                    <div
                      className="absolute right-0 bottom-full mb-2 w-36 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] shadow-xl p-1 z-40 animate-in fade-in zoom-in-95 duration-100"
                      onClick={(e) => e.stopPropagation()}
                    >
                      <button
                        type="button"
                        onClick={() => {
                          setShowOverflow(false);
                          setShowUninstallConfirm(true);
                        }}
                        className="w-full px-3 py-2 rounded-lg text-xs font-medium text-rose-400 hover:bg-rose-500/10 flex items-center gap-2 transition-colors cursor-pointer text-left"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                        <span>Uninstall</span>
                      </button>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Uninstall Confirmation Modal */}
      {showUninstallConfirm && onUninstall && (
        <div
          className="fixed inset-0 z-50 bg-black/75 flex items-center justify-center p-4 backdrop-blur-sm animate-in fade-in duration-150"
          onClick={() => setShowUninstallConfirm(false)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-[var(--rz-bg)] border border-rose-500/30 p-5 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center gap-2.5 text-rose-400 font-bold text-sm">
              <Trash2 className="w-4 h-4" />
              <span>Uninstall {packageItem.title}?</span>
            </div>

            <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
              Are you sure you want to uninstall <strong>{packageItem.title}</strong>? All downloaded theme assets and configurations will be cleanly removed.
            </p>

            <div className="flex items-center justify-end gap-2 pt-2 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="px-3 py-1.5 rounded-lg border border-[var(--rz-border-subtle)] text-xs text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleConfirmUninstall}
                className="px-4 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-bold flex items-center gap-1.5 cursor-pointer shadow-md"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>Confirm Uninstall</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
