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
      <div className="p-3.5 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-500 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-semibold text-[var(--rz-text)]">
            Missing Required Dependencies
          </div>
          <div className="mt-0.5 text-[11px] leading-relaxed text-[var(--rz-text-secondary)]">
            Required binaries for this configuration: <strong>{missingDependencies.join(", ")}</strong>. Please install them on your host.
          </div>
        </div>
      </div>
    );
  }

  // 2. GDM Display Manager Incompatibility Warning
  if (isGdmActive && (selectedTarget === "sddm" || selectedTarget === "both" || !isLoginScreenSupported)) {
    return (
      <div className="p-3.5 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-500 text-xs flex items-start gap-2.5">
        <Monitor className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <span className="font-semibold text-[var(--rz-text)]">
              Login Screen — Unsupported on GDM
            </span>
            <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500 border border-amber-500/30">
              Display Manager: GDM
            </span>
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-[var(--rz-text-secondary)]">
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
      <div className="p-3.5 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-500 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-semibold text-[var(--rz-text)]">
            Session Lock Protocol Unavailable
          </div>
          <p className="mt-1 text-[11px] leading-relaxed text-[var(--rz-text-secondary)]">
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
    <div className="rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] shadow-[0_1px_2px_rgba(0,0,0,0.02)] overflow-hidden transition-all">
      <div className="flex items-center justify-between py-2.5 px-4 text-xs">
        <div className="flex items-center gap-2 text-[var(--rz-text)]">
          <Check className="w-4 h-4 text-emerald-500 stroke-[2.5]" />
          <span className="font-medium">Compatible with your system</span>
          <span className="text-[var(--rz-text-muted)] hidden sm:inline font-normal">
            · {systemReport?.desktop || "Hyprland"} · {systemReport?.display_server || "Wayland"} · SDDM
          </span>
        </div>

        <button
          type="button"
          onClick={() => setShowDetails(!showDetails)}
          className="text-[var(--rz-accent)] hover:underline font-medium text-xs flex items-center gap-1 cursor-pointer select-none"
        >
          <span>{showDetails ? "Hide details" : "View details"}</span>
          <ChevronRight className={`w-3.5 h-3.5 transition-transform duration-150 ${showDetails ? "rotate-90" : ""}`} />
        </button>
      </div>

      {/* Collapsible Diagnostic Disclosure */}
      {showDetails && systemReport && (
        <div className="border-t border-[var(--rz-border-subtle)] p-4 bg-[var(--rz-surface-elevated)] text-xs space-y-3 animate-in fade-in duration-100">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted)] block">
                Session Lock Provider
              </span>
              <span className="font-medium text-[var(--rz-text)] mt-0.5 block">
                {systemReport.session_lock_provider}
              </span>
            </div>
            <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted)] block">
                Display Manager
              </span>
              <span className="font-medium text-[var(--rz-text)] mt-0.5 block">
                {systemReport.login_manager}
              </span>
            </div>
            <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <span className="text-[10px] uppercase font-semibold text-[var(--rz-text-muted)] block">
                Idle Provider
              </span>
              <span className="font-medium text-[var(--rz-text)] mt-0.5 block">
                {systemReport.idle_provider}
              </span>
            </div>
          </div>

          {systemReport.evidence && systemReport.evidence.length > 0 && (
            <div className="p-3 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] max-h-32 overflow-y-auto font-mono text-[10px] text-[var(--rz-text-muted)] space-y-1">
              <div className="font-sans font-semibold text-[10px] text-[var(--rz-text)] mb-1 uppercase tracking-wider">
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
