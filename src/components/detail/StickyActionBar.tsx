import React, { useState } from "react";
import { Check, DownloadCloud, AlertTriangle, Loader2, Lock, Trash2, X } from "lucide-react";
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
  selectedTarget = "quickshell",
  onInstall,
  isActive = false,
  onApply,
  onDeactivate,
  onTest,
  onUninstall,
  isApplying = false,
  isOverridden = false,
  overriddenBy = null,
}) => {
  const [showUninstallConfirm, setShowUninstallConfirm] = useState(false);
  const isBlocked = missingDependencies.length > 0;
  const isSddm = selectedTarget === "sddm" || (!packageItem.supports_session_lock && packageItem.supports_login_screen);
  const isBoth = selectedTarget === "both";

  const getButtonLabel = () => {
    if (isInstalling) return "Installing...";
    if (isUpdateAvailable) return `Update to v${packageItem.version}`;
    if (isBoth) return "Install Both";
    if (isSddm) return "Install Login Screen";
    return "Install Session Lock";
  };

  const getApplyLabel = () => {
    if (isApplying) return "Activating...";
    if (isBoth) return "Apply Both";
    if (isSddm) return "Apply Login Screen";
    return "Apply Session Lock";
  };

  const handleConfirmUninstall = () => {
    setShowUninstallConfirm(false);
    if (onUninstall) {
      onUninstall();
    }
  };

  return (
    <>
      <div className="sticky bottom-0 inset-x-0 z-30 p-3 sm:px-6 bg-[var(--rz-bg)]/90 backdrop-blur-md border-t border-[var(--rz-border-subtle)] shadow-lg flex items-center justify-between gap-3 -mx-4 sm:-mx-6 mt-8">
        {/* Left: Status indicator */}
        <div className="flex items-center gap-2 min-w-0">
          {isBlocked ? (
            <div className="flex items-center gap-1.5 text-xs text-rose-400 font-medium truncate">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span className="truncate">Missing dependency: {missingDependencies.join(", ")}</span>
            </div>
          ) : isBoth ? (
            <div className="flex items-center gap-1.5 text-xs text-sky-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0" />
              <span className="truncate">Dual Target: User Session Lock + System Login Screen</span>
            </div>
          ) : isSddm ? (
            <div className="flex items-center gap-1.5 text-xs text-amber-400 font-medium truncate">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span className="truncate">System Login Screen · Requires admin elevation</span>
            </div>
          ) : (
            <div className="flex items-center gap-1.5 text-xs text-emerald-400 font-medium truncate">
              <Check className="w-4 h-4 shrink-0" />
              <span className="truncate">User Session Lock · Non-privileged</span>
            </div>
          )}
        </div>

        {/* Right: Lifecycle Actions */}
        <div className="flex items-center gap-2 shrink-0">
          {!isInstalled ? (
            // 1. NOT_INSTALLED STATE
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
            // 2. INSTALLED (INACTIVE) STATE: [ Test ] [ Apply ] [ Uninstall ]
            <div className="flex items-center gap-2">
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Test lockscreen reversibly on screen"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <button
                type="button"
                disabled={isApplying}
                onClick={onApply}
                className="px-4 py-1.5 rounded-lg text-xs font-bold transition-all cursor-pointer select-none flex items-center gap-1.5 shadow-sm bg-emerald-600 hover:bg-emerald-500 text-white"
              >
                {isApplying ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                    <span>{getApplyLabel()}</span>
                  </>
                ) : (
                  <>
                    <Check className="w-3.5 h-3.5" />
                    <span>{getApplyLabel()}</span>
                  </>
                )}
              </button>

              {onUninstall && (
                <button
                  type="button"
                  onClick={() => setShowUninstallConfirm(true)}
                  className="p-1.5 rounded-lg text-xs font-semibold bg-[var(--rz-surface)] hover:bg-rose-500/20 border border-[var(--rz-border-subtle)] hover:border-rose-500/40 text-[var(--rz-text-muted)] hover:text-rose-300 transition-all cursor-pointer select-none flex items-center"
                  title="Uninstall package files"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              )}
            </div>
          ) : isOverridden ? (
            // 3. OVERRIDDEN STATE: [ Test ] [ ⚠ Overridden ] [ Deactivate ]
            <div className="flex items-center gap-2">
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <div
                className="px-3 py-1.5 rounded-lg text-xs font-bold bg-amber-600/90 text-white shadow-sm flex items-center gap-1 cursor-default select-none border border-amber-400/40"
                title={overriddenBy ? `Overridden by ${overriddenBy}` : "Configuration overridden"}
              >
                <AlertTriangle className="w-3.5 h-3.5 text-amber-200 stroke-[2.5]" />
                <span>⚠ Overridden</span>
              </div>

              {onDeactivate && (
                <button
                  type="button"
                  disabled={isApplying}
                  onClick={onDeactivate}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-300 transition-all cursor-pointer select-none"
                >
                  Deactivate
                </button>
              )}
            </div>
          ) : (
            // 4. ACTIVE STATE: [ Test ] [ ✓ Active ] [ Deactivate ] [ Uninstall ]
            <div className="flex items-center gap-2">
              {onTest && (
                <button
                  type="button"
                  onClick={onTest}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-500/15 hover:bg-indigo-500/25 border border-indigo-500/30 text-indigo-300 transition-all cursor-pointer select-none flex items-center gap-1.5"
                  title="Test active lockscreen on screen"
                >
                  <Lock className="w-3.5 h-3.5" />
                  <span>Test</span>
                </button>
              )}

              <div className="px-3 py-1.5 rounded-lg text-xs font-bold bg-emerald-600 text-white shadow-sm flex items-center gap-1.5 cursor-default select-none border border-emerald-400/30">
                <Check className="w-3.5 h-3.5 text-white stroke-[3]" />
                <span>✓ Active</span>
              </div>

              {onDeactivate && (
                <button
                  type="button"
                  disabled={isApplying}
                  onClick={onDeactivate}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/30 text-rose-300 transition-all cursor-pointer select-none"
                >
                  Deactivate
                </button>
              )}

              {onUninstall && (
                <button
                  type="button"
                  onClick={() => setShowUninstallConfirm(true)}
                  className="p-1.5 rounded-lg text-xs font-semibold bg-[var(--rz-surface)] hover:bg-rose-500/20 border border-[var(--rz-border-subtle)] hover:border-rose-500/40 text-[var(--rz-text-muted)] hover:text-rose-300 transition-all cursor-pointer select-none flex items-center"
                  title="Uninstall package files"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              )}
            </div>
          )}
        </div>
      </div>

      {/* Uninstall Confirmation Modal */}
      {showUninstallConfirm && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-[var(--rz-surface)] border border-[var(--rz-border-strong)] rounded-xl max-w-md w-full p-5 shadow-2xl space-y-4">
            <div className="flex items-center justify-between pb-3 border-b border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-2 text-rose-400 font-bold text-sm">
                <Trash2 className="w-4 h-4" />
                <span>Uninstall {packageItem.title}?</span>
              </div>
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="text-[var(--rz-text-muted)] hover:text-[var(--rz-text-strong)] cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="text-xs text-[var(--rz-text-muted)] space-y-2">
              {isActive ? (
                <div className="p-2.5 rounded-lg bg-amber-500/15 border border-amber-500/30 text-amber-200">
                  <p className="font-bold flex items-center gap-1.5">
                    <AlertTriangle className="w-3.5 h-3.5 text-amber-300 shrink-0" />
                    This theme is currently active.
                  </p>
                  <p className="mt-1 text-[11px] text-amber-300/90 leading-relaxed">
                    Uninstalling will automatically <strong>deactivate</strong> it first and restore your original system configuration (e.g. Dusky lockscreen and SDDM theme).
                  </p>
                </div>
              ) : (
                <p>
                  This will remove the installed theme files from Ryzora storage. Your system's original configuration will remain completely untouched.
                </p>
              )}

              <div className="p-2.5 rounded-lg bg-black/20 font-mono text-[10.5px] space-y-1">
                <div className="text-[var(--rz-text-strong)] font-semibold">Targets affected:</div>
                <div className="text-neutral-400">• ~/.local/share/ryzora/lockscreens/...</div>
                {packageItem.supports_login_screen && (
                  <div className="text-neutral-400">• /usr/share/sddm/themes/ryzora-... (if installed)</div>
                )}
              </div>
            </div>

            <div className="flex items-center justify-end gap-2.5 pt-2 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => setShowUninstallConfirm(false)}
                className="px-3.5 py-1.5 rounded-lg text-xs font-semibold border border-[var(--rz-border-subtle)] hover:bg-[var(--rz-surface-elevated)] cursor-pointer"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleConfirmUninstall}
                className="px-4 py-1.5 rounded-lg text-xs font-bold bg-rose-600 hover:bg-rose-500 text-white shadow-sm cursor-pointer"
              >
                {isActive ? "Deactivate & Uninstall" : "Uninstall"}
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
