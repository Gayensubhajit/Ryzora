import React from "react";
import { CheckCircle2, AlertTriangle, ShieldAlert, Monitor, Cpu, Layers } from "lucide-react";
import { PackageItem, SystemInfo } from "../../types";

interface CompatibilityPanelProps {
  packageItem: PackageItem;
  systemInfo: SystemInfo | null;
  missingDependencies?: string[];
  selectedTarget?: "quickshell" | "sddm" | "both";
  isSessionLockSupported?: boolean;
}

export const CompatibilityPanel: React.FC<CompatibilityPanelProps> = ({
  packageItem: _packageItem,
  systemInfo,
  missingDependencies = [],
  selectedTarget = "quickshell",
  isSessionLockSupported = true,
}) => {
  const currentWm = systemInfo?.window_manager?.toLowerCase() || "";
  const isWayland = systemInfo?.session_type?.toLowerCase() === "wayland";
  const isMissingDeps = missingDependencies.length > 0;

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

  // KDE / KWin protocol failure warning for Session Lock
  if (!isSessionLockSupported && (selectedTarget === "quickshell" || selectedTarget === "both")) {
    return (
      <div className="p-3.5 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-900 dark:text-amber-200 my-3 text-xs flex items-start gap-2.5">
        <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="font-bold text-amber-950 dark:text-amber-100">
            Protocol Unsupported: ext-session-lock-v1
          </div>
          <p className="mt-1 text-amber-900 dark:text-amber-900/90 dark:text-amber-200/90 text-[11px] leading-relaxed">
            Your current window manager ({currentWm || "KWin"}) does not implement the Wayland <code>ext-session-lock-v1</code> protocol required by Quickshell. You can still install this theme as an <strong>SDDM Login Screen</strong>.
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
    return (
      <div className="p-3.5 rounded-xl border border-amber-500/35 bg-amber-500/10 text-amber-900 dark:text-amber-200 my-3 text-xs flex items-start gap-2.5">
        <ShieldAlert className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-bold text-amber-950 dark:text-amber-100">System Integration Required</span>
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-amber-500/20 text-amber-300 border border-amber-400/30">
              Display Manager Greeter
            </span>
          </div>
          <p className="mt-1 text-amber-900 dark:text-amber-900/90 dark:text-amber-200/90 text-[11px] leading-relaxed">
            This theme modifies the <strong>SDDM Login Screen</strong> (/usr/share/sddm/themes/). It controls the display manager shown before your desktop session begins and requires administrator privilege to write.
          </p>
          <div className="flex items-center gap-2 mt-2 text-[10.5px] text-amber-800 dark:text-amber-300/80 font-mono">
            <span>SDDM 0.20+</span>
            <span>·</span>
            <span>Qt 6 Declarative</span>
            <span>·</span>
            <span>Display Manager Scope</span>
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
