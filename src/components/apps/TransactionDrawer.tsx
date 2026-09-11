import React, { useState, useEffect, useRef } from "react";
import {
  transactionManager,
  TransactionState,
} from "../../services/transactionManager";
import {
  Loader2,
  CheckCircle2,
  XCircle,
  ChevronUp,
  ChevronDown,
  Copy,
  Check,
  Play,
  X,
  AlertTriangle,
  RotateCcw,
  ShieldAlert,
} from "lucide-react";
import { invokeTauri, isTauri } from "../../services/tauri";

interface TransactionDrawerProps {
  onOpenApp?: (packageId: string) => void;
  onRefreshApps?: () => void;
}

export const TransactionDrawer: React.FC<TransactionDrawerProps> = ({
  onOpenApp,
  onRefreshApps,
}) => {
  const [tx, setTx] = useState<TransactionState | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const [settingUpHelper, setSettingUpHelper] = useState(false);
  const [setupError, setSetupError] = useState<string | null>(null);
  const logContainerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const unsubscribe = transactionManager.subscribe((state) => {
      setTx(state);
      if (state && (state.stage === "completed" || state.stage === "failed")) {
        onRefreshApps?.();
      }
    });
    return unsubscribe;
  }, [onRefreshApps]);

  // Auto-scroll logs to bottom when expanded
  useEffect(() => {
    if (expanded && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [expanded, tx?.logs.length]);

  if (!tx) return null;

  const isBusy =
    tx.stage !== "completed" && tx.stage !== "failed";
  const isSuccess = tx.stage === "completed";
  const isFailed = tx.stage === "failed";

  const handleCopyLogs = () => {
    if (!tx) return;
    const text = tx.logs.join("\n");
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleOpen = async () => {
    if (onOpenApp) {
      onOpenApp(tx.packageName);
    } else if (isTauri) {
      try {
        await invokeTauri("launch_desktop_app", { packageId: tx.packageName });
      } catch (err) {
        console.warn("Failed to launch app:", err);
      }
    }
    transactionManager.clearCurrentTransaction();
  };

  const handleRetry = () => {
    const pkg = tx.packageName;
    const op = tx.operation;
    transactionManager.clearCurrentTransaction();
    transactionManager.runTransaction(pkg, op).catch(() => {});
  };

  const handleSetupHelper = async () => {
    setSettingUpHelper(true);
    setSetupError(null);
    try {
      await transactionManager.setupHelper();
      handleRetry();
    } catch (err: any) {
      setSetupError(typeof err === "string" ? err : err?.message || "Failed to setup helper");
    } finally {
      setSettingUpHelper(false);
    }
  };

  const opTitle =
    tx.operation === "install"
      ? "Installing"
      : tx.operation === "uninstall"
      ? "Uninstalling"
      : "Reinstalling";

  // Stages checklist definitions
  const stagesList = [
    { key: "preparing", label: "Preparing transaction" },
    { key: "resolving", label: "Resolving dependencies" },
    { key: "downloading", label: "Downloading packages" },
    { key: "installing", label: tx.operation === "uninstall" ? "Removing package" : "Installing package" },
    { key: "configuring", label: "Finalizing system integration" },
  ];

  const currentStageIndex = stagesList.findIndex((s) => s.key === tx.stage);

  return (
    <aside
      aria-label="Transaction progress"
      className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50 w-[94%] max-w-xl transition-all duration-300 ease-out"
    >
      <div className="overflow-hidden rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-card-bg)]/95 shadow-2xl backdrop-blur-2xl text-[var(--rz-text)]">
        {/* Progress Bar Header */}
        {isBusy && (
          <div className="relative h-1.5 w-full bg-[var(--rz-border-subtle)] overflow-hidden">
            {tx.percentage !== null ? (
              <div
                className="h-full bg-blue-500 transition-all duration-300 ease-out"
                style={{ width: `${Math.max(5, Math.min(100, tx.percentage))}%` }}
              />
            ) : (
              <div className="h-full w-1/3 bg-blue-500 animate-pulse rounded-full" />
            )}
          </div>
        )}

        {/* Main Summary Bar */}
        <div className="flex items-center justify-between gap-4 px-5 py-3.5">
          <div className="flex items-center gap-3.5 min-w-0 flex-1">
            {isBusy && (
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-blue-500/10 text-blue-400">
                <Loader2 size={18} className="animate-spin" />
              </div>
            )}
            {isSuccess && (
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-emerald-500/10 text-emerald-400">
                <CheckCircle2 size={18} />
              </div>
            )}
            {isFailed && (
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-rose-500/10 text-rose-400">
                <XCircle size={18} />
              </div>
            )}

            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="font-semibold text-sm truncate">
                  {isSuccess
                    ? `✓ ${tx.packageName} ${
                        tx.operation === "uninstall"
                          ? "uninstalled"
                          : tx.operation === "reinstall"
                          ? "reinstalled"
                          : "installed"
                      }`
                    : isFailed
                    ? `Failed to ${tx.operation} ${tx.packageName}`
                    : `${opTitle} ${tx.packageName}`}
                </span>
                {isBusy && tx.percentage !== null && (
                  <span className="shrink-0 w-12 text-center rounded-md bg-blue-500/15 px-1.5 py-0.5 text-[11px] font-mono tabular-nums font-medium text-blue-400">
                    {tx.percentage}%
                  </span>
                )}
                {tx.isCached && isBusy && (
                  <span className="shrink-0 rounded-md bg-emerald-500/15 px-1.5 py-0.5 text-[10px] font-medium text-emerald-400">
                    Cached
                  </span>
                )}
              </div>
              <div className="flex items-center justify-between text-xs text-[var(--rz-text-muted)] h-4 leading-4 pt-0.5 min-w-0">
                <p className="truncate min-w-0 flex-1">
                  {tx.isCached && tx.stage !== "downloading"
                    ? "Using cached package • Ready for installation"
                    : tx.message}
                </p>
                {(tx.downloadedSizeStr || tx.speedStr) && (
                  <span className="shrink-0 pl-2 font-mono tabular-nums text-[11px] text-[var(--rz-text-secondary)]">
                    {tx.downloadedSizeStr && `${tx.downloadedSizeStr}`}
                    {tx.totalSizeStr && ` / ${tx.totalSizeStr}`}
                    {tx.speedStr && ` (${tx.speedStr})`}
                  </span>
                )}
              </div>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="flex items-center gap-2 shrink-0">
            {isSuccess && tx.operation !== "uninstall" && (
              <button
                type="button"
                onClick={handleOpen}
                className="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg font-medium text-xs bg-blue-600 hover:bg-blue-500 text-white transition-all shadow-sm cursor-pointer"
              >
                <Play size={13} fill="currentColor" />
                <span>Open</span>
              </button>
            )}

            {isFailed && (
              <button
                type="button"
                onClick={handleRetry}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg font-medium text-xs bg-[var(--rz-border-subtle)] hover:bg-[var(--rz-bg-hover)] text-[var(--rz-text)] transition-all cursor-pointer"
              >
                <RotateCcw size={12} />
                <span>Retry</span>
              </button>
            )}

            <button
              type="button"
              onClick={() => setExpanded(!expanded)}
              aria-label={expanded ? "Collapse details" : "Expand details"}
              className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-bg-hover)] transition-all cursor-pointer"
            >
              <span>{expanded ? "Hide" : "Details"}</span>
              {expanded ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
            </button>

            {!isBusy && (
              <button
                type="button"
                onClick={() => transactionManager.clearCurrentTransaction()}
                aria-label="Dismiss notification"
                className="p-1.5 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-bg-hover)] transition-all cursor-pointer"
              >
                <X size={14} />
              </button>
            )}
          </div>
        </div>

        {/* Expandable Details Drawer */}
        {expanded && (
          <div className="border-t border-[var(--rz-border-subtle)] bg-[var(--rz-bg)]/60 px-5 py-4 space-y-4 animate-fadeIn">
            {/* Database Lock or Helper Setup Notice */}
            {tx.isDbLocked && (
              <div className="flex items-start gap-3 p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-300 text-xs">
                <AlertTriangle size={16} className="shrink-0 mt-0.5" />
                <div>
                  <p className="font-semibold">Package database is locked</p>
                  <p className="text-amber-300/80 pt-0.5">
                    Another package manager (e.g. pamac, pacman) is currently active. Ryzora will not force or delete the lock file. Please wait until it completes or close conflicting operations.
                  </p>
                </div>
              </div>
            )}

            {tx.error && (tx.error.includes("package service is not installed") || tx.error.includes("Privileged Package Service is not installed")) && (
              <div className="flex items-start justify-between gap-3 p-3.5 rounded-xl bg-blue-500/10 border border-blue-500/20 text-blue-300 text-xs">
                <div className="flex items-start gap-2.5">
                  <ShieldAlert size={16} className="shrink-0 mt-0.5" />
                  <div>
                    <p className="font-semibold">Ryzora Package Service Setup Required</p>
                    <p className="text-blue-300/80 pt-0.5">
                      Install the Ryzora privileged helper to manage packages securely without repeated password prompts.
                    </p>
                    {setupError && <p className="text-rose-400 pt-1">{setupError}</p>}
                  </div>
                </div>
                <button
                  type="button"
                  onClick={handleSetupHelper}
                  disabled={settingUpHelper}
                  className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white font-medium text-xs shrink-0 cursor-pointer disabled:opacity-50"
                >
                  {settingUpHelper ? "Setting up..." : "Set Up Service"}
                </button>
              </div>
            )}

            {/* Stage Checklist */}
            <div className="grid grid-cols-2 sm:grid-cols-5 gap-2 pt-1">
              {stagesList.map((st, idx) => {
                const isPast =
                  isSuccess || (currentStageIndex >= 0 && currentStageIndex > idx);
                const isCurrent = currentStageIndex === idx && isBusy;
                const isPending = !isPast && !isCurrent;

                return (
                  <div
                    key={st.key}
                    className={`flex items-center gap-2 px-2.5 py-2 rounded-lg text-[11px] font-medium transition-colors ${
                      isCurrent
                        ? "bg-blue-500/15 text-blue-400 border border-blue-500/20"
                        : isPast
                        ? "text-emerald-400/90"
                        : "text-[var(--rz-text-muted)] opacity-60"
                    }`}
                  >
                    {isCurrent && <Loader2 size={12} className="animate-spin shrink-0" />}
                    {isPast && <CheckCircle2 size={12} className="shrink-0" />}
                    {isPending && <div className="w-3 h-3 rounded-full border border-current shrink-0" />}
                    <span className="truncate">{st.label}</span>
                  </div>
                );
              })}
            </div>

            {/* Live Terminal Output Stream */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-[11px] text-[var(--rz-text-muted)] font-medium">
                <span>Transaction Log</span>
                <button
                  type="button"
                  onClick={handleCopyLogs}
                  className="inline-flex items-center gap-1 hover:text-[var(--rz-text)] transition-colors cursor-pointer"
                >
                  {copied ? <Check size={12} className="text-emerald-400" /> : <Copy size={12} />}
                  <span>{copied ? "Copied" : "Copy Output"}</span>
                </button>
              </div>

              <div
                ref={logContainerRef}
                className="h-32 overflow-y-auto rounded-xl bg-black/40 p-3 font-mono text-[11px] text-emerald-400/90 leading-relaxed border border-[var(--rz-border-subtle)] select-text"
              >
                {tx.logs.length === 0 ? (
                  <p className="text-[var(--rz-text-muted)] italic">Waiting for pacman stream...</p>
                ) : (
                  tx.logs.map((log, i) => (
                    <div key={i} className="whitespace-pre-wrap break-all">
                      {log}
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>
        )}
      </div>
    </aside>
  );
};
