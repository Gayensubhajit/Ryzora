import React, { useState } from "react";
import {
  Check,
  AlertTriangle,
  ChevronRight,
  Monitor,
} from "lucide-react";
import { PackageItem, SystemInfo } from "../../types";
import { useApp } from "../../context/AppContext";

interface CompatibilityPanelProps {
  packageItem: PackageItem;
  systemInfo: SystemInfo | null;
  missingDependencies?: string[];
  selectedTarget?: "quickshell" | "sddm" | "both";
  isSessionLockSupported?: boolean;
  isLoginScreenSupported?: boolean;
  isGdmActive?: boolean;
}

export const CompatibilityPanel: React.FC<CompatibilityPanelProps> = ({
  packageItem: _packageItem,
  systemInfo,
  missingDependencies = [],
  selectedTarget = "quickshell",
  isSessionLockSupported = true,
  isLoginScreenSupported = true,
  isGdmActive = false,
}) => {
  const [showDetails, setShowDetails] = useState(false);
  const currentWm = systemInfo?.window_manager?.toLowerCase() || "";
  const { systemIntegrationReport: systemReport } = useApp();
  const isMissingDeps = missingDependencies.length > 0;

  // 1. Missing Dependencies Warning
  if (isMissingDeps) {
    return (
      <div className="p-3.5 rounded-xl border border-rose-200 dark:border-rose-900/40 bg-rose-50 dark:bg-rose-950/20 text-rose-800 dark:text-rose-300 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-semibold text-rose-900 dark:text-rose-200">
            Missing Required Dependencies
          </div>
          <div className="mt-0.5 text-[11px] leading-relaxed opacity-90">
            Required binaries for this configuration: <strong>{missingDependencies.join(", ")}</strong>. Please install them on your host.
          </div>
        </div>
      </div>
    );
  }

  // 2. GDM Display Manager Incompatibility Warning
  if (isGdmActive && (selectedTarget === "sddm" || selectedTarget === "both" || !isLoginScreenSupported)) {
    return (
      <div className="p-3.5 rounded-xl border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20 text-amber-800 dark:text-amber-300 text-xs flex items-start gap-2.5">
        <Monitor className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <span className="font-semibold text-amber-900 dark:text-amber-200">
              Login Screen — Unsupported on GDM
            </span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/40 text-amber-800 dark:text-amber-300">
              Display Manager: GDM
            </span>
          </div>
          <p className="mt-1 text-[11px] leading-relaxed opacity-90">
            This theme provides an SDDM login greeter. Your system runs GDM (GNOME Display Manager). Ryzora protects your display manager by not installing SDDM themes into GDM.
          </p>
        </div>
      </div>
    );
  }

  // 3. Protocol Failure Warning
  if (!isSessionLockSupported && (selectedTarget === "quickshell" || selectedTarget === "both")) {
    const isGnome = (systemInfo?.desktop_environment || "").toLowerCase().includes("gnome") || currentWm.includes("mutter");
    return (
      <div className="p-3.5 rounded-xl border border-amber-200 dark:border-amber-900/40 bg-amber-50 dark:bg-amber-950/20 text-amber-800 dark:text-amber-300 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-semibold text-amber-900 dark:text-amber-200">
            Session Lock Protocol Unavailable
          </div>
          <p className="mt-1 text-[11px] leading-relaxed opacity-90">
            {isGnome
              ? "GNOME / Mutter uses a built-in lockscreen and does not implement ext-session-lock-v1."
              : `Your window manager (${currentWm || "KWin"}) does not implement ext-session-lock-v1 required for session lock.`}
          </p>
        </div>
      </div>
    );
  }

  // 4. Default: Quiet, Clean Single-Row Compatibility Summary
  return (
    <div className="rounded-xl border border-[var(--rz-border-subtle,#d2d2d7)] bg-[var(--rz-surface,#ffffff)] shadow-[0_1px_2px_rgba(0,0,0,0.02)] overflow-hidden transition-all">
      <div className="flex items-center justify-between py-2.5 px-4 text-xs">
        <div className="flex items-center gap-2 text-[var(--rz-text,#1d1d1f)]">
          <Check className="w-4 h-4 text-emerald-600 dark:text-emerald-400 stroke-[2.5]" />
          <span className="font-medium">Compatible with your system</span>
          <span className="text-[var(--rz-text-muted,#86868b)] hidden sm:inline font-normal">
            · {systemReport?.desktop || "Hyprland"} · {systemReport?.display_server || "Wayland"} · SDDM
          </span>
        </div>

        <button
          type="button"
          onClick={() => setShowDetails(!showDetails)}
          className="text-[var(--rz-accent,#0071e3)] hover:underline font-medium text-xs flex items-center gap-1 cursor-pointer select-none"
        >
          <span>{showDetails ? "Hide details" : "View details"}</span>
          <ChevronRight className={`w-3.5 h-3.5 transition-transform duration-150 ${showDetails ? "rotate-90" : ""}`} />
        </button>
      </div>

      {/* Collapsible Diagnostic Disclosure */}
      {showDetails && systemReport && (
        <div className="border-t border-[var(--rz-border-subtle,#e5e5e7)] p-4 bg-[var(--rz-surface-elevated,#f5f5f7)] text-xs space-y-3 animate-in fade-in duration-100">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="p-2.5 rounded-lg bg-white dark:bg-[var(--rz-surface)] border border-[var(--rz-border-subtle,#d2d2d7)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted,#86868b)] block">
                Session Lock Provider
              </span>
              <span className="font-medium text-[var(--rz-text,#1d1d1f)] mt-0.5 block">
                {systemReport.session_lock_provider}
              </span>
            </div>
            <div className="p-2.5 rounded-lg bg-white dark:bg-[var(--rz-surface)] border border-[var(--rz-border-subtle,#d2d2d7)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted,#86868b)] block">
                Display Manager
              </span>
              <span className="font-medium text-[var(--rz-text,#1d1d1f)] mt-0.5 block">
                {systemReport.login_manager}
              </span>
            </div>
            <div className="p-2.5 rounded-lg bg-white dark:bg-[var(--rz-surface)] border border-[var(--rz-border-subtle,#d2d2d7)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted,#86868b)] block">
                Idle Provider
              </span>
              <span className="font-medium text-[var(--rz-text,#1d1d1f)] mt-0.5 block">
                {systemReport.idle_provider}
              </span>
            </div>
          </div>

          {systemReport.evidence && systemReport.evidence.length > 0 && (
            <div className="p-3 rounded-lg bg-white dark:bg-[var(--rz-surface)] border border-[var(--rz-border-subtle,#d2d2d7)] max-h-32 overflow-y-auto font-mono text-[10px] text-[var(--rz-text-muted,#86868b)] space-y-1">
              <div className="font-sans font-semibold text-[10px] text-[var(--rz-text,#1d1d1f)] mb-1 uppercase tracking-wider">
                Signals Detected:
              </div>
              {systemReport.evidence.map((sig, idx) => (
                <div key={idx} className="truncate">
                  • {sig}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
