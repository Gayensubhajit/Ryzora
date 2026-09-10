import React from "react";
import { CheckCircle2, AlertTriangle, ShieldAlert, Monitor, Cpu, Layers, ShieldCheck, Loader2, ChevronDown, ChevronUp, Terminal } from "lucide-react";
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
  const [showEvidence, setShowEvidence] = React.useState(false);
  const currentWm = systemInfo?.window_manager?.toLowerCase() || "";
  const isWayland = systemInfo?.session_type?.toLowerCase() === "wayland";
  const { privilegedHelperStatus, setupPrivilegedHelper, isSettingUpHelper, sddmRuntimeStatus } = useApp();
  const isMissingDeps = missingDependencies.length > 0;

  const systemReport = useApp().systemIntegrationReport;

  const renderSystemIntegrationCard = () => {
    if (!systemReport) return null;
    return (
      <div className="p-3.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]/80 text-[var(--rz-text)] my-3 text-xs shadow-sm">
        <div className="flex items-center justify-between gap-2 pb-2 border-b border-[var(--rz-border-subtle)]">
          <div className="flex items-center gap-1.5 font-bold text-[var(--rz-text-strong)]">
            <Monitor className="w-4 h-4 text-[var(--rz-accent)]" />
            <span>Detected System Lock Environment</span>
          </div>
          <span className="px-2 py-0.5 rounded text-[10px] font-mono uppercase bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)]">
            {systemReport.desktop} · {systemReport.display_server}
          </span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5 pt-2.5 text-[11px]">
          <div className="p-2 rounded-lg bg-black/15 border border-white/5">
            <div className="text-[10px] font-semibold text-[var(--rz-text-muted)] uppercase tracking-wider">Session Lock</div>
            <div className="font-bold text-[var(--rz-text-strong)] mt-0.5 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
              {systemReport.session_lock_provider}
            </div>
            <div className="text-[10px] text-[var(--rz-text-muted)] truncate mt-0.5">
              Trigger: {systemReport.idle_provider}
            </div>
          </div>

          <div className="p-2 rounded-lg bg-black/15 border border-white/5">
            <div className="text-[10px] font-semibold text-[var(--rz-text-muted)] uppercase tracking-wider">Login Screen</div>
            <div className="font-bold text-[var(--rz-text-strong)] mt-0.5 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-sky-400" />
              {systemReport.login_manager}
            </div>
            <div className="text-[10px] text-[var(--rz-text-muted)] truncate mt-0.5">
              Theme: {systemReport.login_theme || "System Default"}
            </div>
          </div>

          <div className="p-2 rounded-lg bg-black/15 border border-white/5">
            <div className="text-[10px] font-semibold text-[var(--rz-text-muted)] uppercase tracking-wider">Idle Trigger</div>
            <div className="font-bold text-[var(--rz-text-strong)] mt-0.5 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-indigo-400" />
              {systemReport.idle_provider}
            </div>
            <div className="text-[10px] text-[var(--rz-text-muted)] truncate mt-0.5" title={systemReport.idle_config || ""}>
              {systemReport.idle_config ? "Configured" : "Default"}
            </div>
          </div>
        </div>

        {systemReport.warnings && systemReport.warnings.length > 0 && (
          <div className="mt-2.5 p-2 rounded-lg bg-amber-500/10 border border-amber-500/20 text-amber-300 text-[11px] space-y-1">
            {systemReport.warnings.map((w, idx) => (
              <div key={idx} className="flex items-start gap-1.5">
                <AlertTriangle className="w-3.5 h-3.5 text-amber-400 shrink-0 mt-0.5" />
                <span>{w}</span>
              </div>
            ))}
          </div>
        )}

        <div className="mt-2.5 pt-2 border-t border-[var(--rz-border-subtle)] flex flex-col gap-1.5">
          <button
            type="button"
            onClick={() => setShowEvidence(!showEvidence)}
            className="flex items-center justify-between w-full text-[10.5px] text-[var(--rz-text-muted)] hover:text-[var(--rz-text-strong)] transition-colors py-0.5 cursor-pointer"
          >
            <span className="flex items-center gap-1">
              <Terminal className="w-3 h-3 text-[var(--rz-accent)]" />
              View Detection Evidence & Details ({systemReport.evidence.length} signals)
            </span>
            {showEvidence ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
          </button>

          {showEvidence && (
            <div className="mt-1 p-2 rounded-lg bg-black/30 font-mono text-[10px] text-[var(--rz-text-muted)] space-y-1 max-h-36 overflow-y-auto border border-white/5">
              {systemReport.evidence.map((ev, i) => (
                <div key={i} className="truncate hover:text-white transition-colors" title={ev}>
                  <span className="text-[var(--rz-accent)]">• </span>{ev}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    );
  };

  const renderTargetPanel = () => {
    if (isMissingDeps) {
    return (
      <div className="p-3.5 rounded-xl border border-rose-500/30 bg-rose-500/10 text-rose-300 my-3 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-bold text-rose-200">
            Missing Required Dependencies
          </div>
          <div className="mt-0.5 text-rose-300/90 text-[11px] leading-relaxed">
            The following binaries are required for your selected target:{" "}
            <strong>{missingDependencies.join(", ")}</strong>. Please install them on your system before proceeding.
          </div>
        </div>
      </div>
    );
  }

  // GDM Display Manager Incompatibility Warning
  if (isGdmActive && (selectedTarget === "sddm" || selectedTarget === "both" || !isLoginScreenSupported)) {
    return (
      <div className="p-3.5 rounded-xl border border-sky-500/30 bg-sky-500/10 text-sky-900 dark:text-sky-200 my-3 text-xs flex items-start gap-2.5">
        <Monitor className="w-4 h-4 text-sky-400 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <span className="font-bold text-sky-950 dark:text-sky-100">
              Login Screen — Unsupported on GDM
            </span>
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-sky-500/20 text-sky-300 border border-sky-400/30 shrink-0">
              Display Manager: GDM
            </span>
          </div>
          <p className="mt-1 text-sky-900 dark:text-sky-900/90 dark:text-sky-200/90 text-[11px] leading-relaxed">
            This package provides an SDDM login theme, but your system uses <strong>GDM (GNOME Display Manager)</strong>. GDM uses GNOME Shell instead of Qt/QML greeters. Ryzora protects your display manager by not installing SDDM themes into GDM.
          </p>
        </div>
      </div>
    );
  }

  // KDE / GNOME protocol failure warning for Session Lock
  if (!isSessionLockSupported && (selectedTarget === "quickshell" || selectedTarget === "both")) {
    const isGnome = (systemInfo?.desktop_environment || "").toLowerCase().includes("gnome") || currentWm.includes("mutter");
    return (
      <div className="p-3.5 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-200 my-3 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-bold text-amber-950 dark:text-amber-100">
            Session Lock Unavailable
          </div>
          <p className="mt-1 text-amber-900 dark:text-amber-900/90 dark:text-amber-200/90 text-[11px] leading-relaxed">
            {isGnome
              ? "GNOME / Mutter uses a built-in lockscreen and does not support ext-session-lock-v1 for external lockers. Quickshell session lock is unavailable."
              : `Your current window manager (${currentWm || "KWin"}) does not implement the Wayland ext-session-lock-v1 protocol required by Quickshell.`}
          </p>
        </div>
      </div>
    );
  }

  // Both: Dual Installation Panel
  if (selectedTarget === "both") {
    return (
      <div className="p-3.5 rounded-xl border border-sky-500/35 bg-sky-500/10 text-sky-900 dark:text-sky-200 my-3 text-xs flex items-start gap-2.5">
        <Layers className="w-4 h-4 text-sky-400 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between gap-2">
            <span className="font-bold text-sky-950 dark:text-sky-100">Dual Target Installation</span>
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-sky-500/20 text-sky-300 border border-sky-400/30">
              Session Lock + Login Screen
            </span>
          </div>
          <p className="mt-1 text-sky-900 dark:text-sky-900/90 dark:text-sky-200/90 text-[11px] leading-relaxed">
            Installs into <strong>user space</strong> (~/.local/share/ryzora/lockscreens/) for active Wayland session locks, and copies into <strong>system space</strong> (/usr/share/sddm/themes/) with root confirmation.
          </p>
          <div className="flex items-center gap-3 mt-2 text-[10.5px] text-sky-800 dark:text-sky-300/80 font-mono">
            <span>✓ Wayland ext-session-lock-v1</span>
            <span>·</span>
            <span>⚠ Admin privileges (pkexec)</span>
            <span>·</span>
            <span>Atomic Backup</span>
          </div>
        </div>
      </div>
    );
  }

  // SDDM Login Greeter Target
  if (selectedTarget === "sddm") {
    if (!privilegedHelperStatus?.installed) {
      return (
        <div className="p-4 rounded-xl border border-amber-500/35 bg-amber-500/10 text-amber-900 dark:text-amber-200 my-3 text-xs flex flex-col gap-3">
          <div className="flex items-start gap-2.5">
            <ShieldAlert className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="font-bold text-amber-950 dark:text-amber-100 text-sm">System Integration Required</span>
                <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-amber-500/20 text-amber-300 border border-amber-400/30">
                  Display Manager Greeter
                </span>
              </div>
              <p className="mt-1 text-amber-900 dark:text-amber-900/90 dark:text-amber-200/90 text-xs leading-relaxed">
                SDDM installation and activation requires Ryzora's restricted privileged helper (<code className="bg-black/40 px-1 py-0.5 rounded text-amber-300">/usr/lib/ryzora/ryzora-sddm-helper</code>) and Polkit policy.
              </p>
            </div>
          </div>

          <div className="flex items-center justify-between gap-3 pt-2 border-t border-amber-500/20">
            <div className="text-[11px] text-amber-800 dark:text-amber-300/80">
              Administrator authentication will be requested once during setup.
            </div>
            <button
              type="button"
              disabled={isSettingUpHelper}
              onClick={() => setupPrivilegedHelper()}
              className="px-3.5 py-1.5 rounded-lg text-xs font-bold bg-amber-600 hover:bg-amber-500 text-white shadow-md transition-all cursor-pointer select-none flex items-center gap-1.5 shrink-0"
            >
              {isSettingUpHelper ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  <span>Setting Up...</span>
                </>
              ) : (
                <>
                  <ShieldCheck className="w-3.5 h-3.5" />
                  <span>Set Up System Integration</span>
                </>
              )}
            </button>
          </div>
        </div>
      );
    }

    return (
      <div className="p-3.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-200 my-3 text-xs flex items-start gap-2.5">
        <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="font-bold text-emerald-950 dark:text-emerald-100">System Integration Active</span>
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-emerald-500/20 text-emerald-300 border border-emerald-400/30">
              Privileged Helper Verified
            </span>
          </div>
          <p className="mt-1 text-emerald-900 dark:text-emerald-900/90 dark:text-emerald-200/90 text-[11px] leading-relaxed">
            Ryzora privileged helper is installed at <code className="bg-black/30 px-1 py-0.5 rounded text-emerald-300">/usr/lib/ryzora/ryzora-sddm-helper</code>. SDDM operations will be authorized securely via Polkit.
          </p>

          {/* SDDM Status Diagnostic Card (Requirement 7) */}
          <div className="mt-3 p-2.5 rounded-lg border border-emerald-500/20 bg-black/20 text-[11px] space-y-1.5">
            <div className="flex items-center justify-between font-bold text-emerald-200 border-b border-white/10 pb-1">
              <span>SDDM Diagnostic Status</span>
              <span className={`px-1.5 py-0.2 rounded text-[9px] font-mono ${
                sddmRuntimeStatus?.active
                  ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                  : sddmRuntimeStatus?.is_overridden
                  ? "bg-amber-500/20 text-amber-300 border border-amber-500/30"
                  : "bg-white/10 text-neutral-300"
              }`}>
                {sddmRuntimeStatus?.active ? "Active" : sddmRuntimeStatus?.is_overridden ? "Overridden" : "Standby"}
              </span>
            </div>

            <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-[10.5px]">
              <div className="flex items-center justify-between">
                <span className="text-emerald-300/70">Installed:</span>
                <span className="font-semibold text-emerald-200">{sddmRuntimeStatus?.installed ? "✓ Yes" : "— Not installed"}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-emerald-300/70">Ryzora Applied:</span>
                <span className="font-semibold text-emerald-200">{sddmRuntimeStatus?.applied ? "✓ Yes" : "— Standby"}</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-emerald-300/70">Effective Theme:</span>
                <span className={`font-mono font-bold ${sddmRuntimeStatus?.active ? "text-emerald-300" : sddmRuntimeStatus?.is_overridden ? "text-amber-300" : "text-white"}`}>
                  {sddmRuntimeStatus?.effective_theme ? `${sddmRuntimeStatus.is_overridden ? "⚠ " : "✓ "}${sddmRuntimeStatus.effective_theme}` : "System Default"}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-emerald-300/70">Previous Theme:</span>
                <span className="font-mono text-emerald-200/80">{sddmRuntimeStatus?.previous_theme || "None recorded"}</span>
              </div>
            </div>

            {sddmRuntimeStatus?.effective_file && (
              <div className="text-[10px] font-mono text-emerald-300/60 flex items-center justify-between pt-0.5 border-t border-white/5">
                <span>Configuration:</span>
                <span className="text-emerald-200/80 truncate max-w-[240px]" title={sddmRuntimeStatus.effective_file}>
                  {sddmRuntimeStatus.effective_file}
                </span>
              </div>
            )}

            {sddmRuntimeStatus?.is_overridden && sddmRuntimeStatus?.overridden_by && (
              <div className="p-1.5 rounded bg-amber-500/20 border border-amber-500/40 text-amber-200 text-[10px] leading-tight flex items-start gap-1">
                <AlertTriangle className="w-3 h-3 shrink-0 mt-0.5 text-amber-300" />
                <div>
                  <span className="font-bold">Overridden by: </span>
                  <code className="bg-black/50 px-1 py-0.5 rounded text-amber-100">{sddmRuntimeStatus.overridden_by}</code>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  }

  // Standard Session Lock (Hyprlock, Quickshell, Swaylock)
  return (
    <div className="p-3.5 rounded-xl border border-emerald-500/30 bg-emerald-500/10 text-emerald-900 dark:text-emerald-200 my-3 text-xs flex items-start gap-2.5">
      <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-2">
          <div className="flex items-center gap-1.5 font-bold text-emerald-950 dark:text-emerald-100">
            <span>Compatible with your system</span>
          </div>
          <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-emerald-500/20 text-emerald-300 border border-emerald-400/30 shrink-0">
            Session Lock
          </span>
        </div>

        <p className="mt-1 text-emerald-900 dark:text-emerald-900/90 dark:text-emerald-200/90 text-[11px] leading-relaxed">
          Runs safely within your active user session. No root permissions required. Materialized directly in your local directory (<code className="bg-black/30 px-1 py-0.5 rounded">~/.local/share/ryzora/</code>).
        </p>

        <div className="flex items-center gap-3 mt-2 text-[11px] text-emerald-800 dark:text-emerald-300/80">
          <div className="flex items-center gap-1">
            <Monitor className="w-3 h-3 text-emerald-400" />
            <span>{currentWm ? currentWm.toUpperCase() : "Hyprland / Wayland"}</span>
          </div>
          <span>·</span>
          <div className="flex items-center gap-1">
            <Cpu className="w-3 h-3 text-emerald-400" />
            <span>{isWayland ? "ext-session-lock-v1" : "Wayland / X11"}</span>
          </div>
          <span>·</span>
          <span>Snapshot available</span>
        </div>
      </div>
    </div>
    );
  };

  return (
    <div className="space-y-1">
      {renderSystemIntegrationCard()}
      {renderTargetPanel()}
    </div>
  );
};