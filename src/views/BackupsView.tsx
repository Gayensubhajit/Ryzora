import React, { useState } from "react";
import { RotateCcw, Clock, Folder, Trash2, ShieldCheck, AlertTriangle, HardDrive } from "lucide-react";
import { useApp } from "../context/AppContext";
import { SnapshotFileEntry, FileEntryType } from "../types";

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function fileEntryIcon(type: FileEntryType): string {
  switch (type) {
    case "Regular": return "·";
    case "Directory": return "▸";
    case "Symlink": return "⇢";
    case "Absent": return "∅";
  }
}

function fileEntryLabel(entry: SnapshotFileEntry): string {
  if (entry.file_type === "Absent") return `${entry.original_path} (absent — was not installed)`;
  if (entry.file_type === "Symlink")
    return `${entry.original_path} → ${entry.symlink_target ?? "?"}`;
  return entry.original_path;
}

export const BackupsView: React.FC = () => {
  const { snapshots, rollbackSnapshot, deleteSnapshot } = useApp();
  const [actionId, setActionId] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);

  const handleRestore = async (id: string) => {
    setActionId(id);
    await rollbackSnapshot(id);
    setActionId(null);
  };

  const handleDelete = async (id: string) => {
    setActionId(id);
    await deleteSnapshot(id);
    setActionId(null);
  };

  const regularCount = (entries: SnapshotFileEntry[]) =>
    entries.filter((e) => e.file_type === "Regular").length;

  return (
    <div className="space-y-6 pb-10">
      {/* Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            Configuration Snapshots
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Real filesystem backups stored in{" "}
            <code className="text-slate-300">~/.local/share/ryzora/snapshots/</code>.
            Restore previous states safely at any time.
          </p>
        </div>

        <div className="flex items-center gap-2">
          <div className="text-xs font-mono text-[var(--text-muted)] px-2.5 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
            {snapshots.length} {snapshots.length === 1 ? "Snapshot" : "Snapshots"}
          </div>
        </div>
      </div>

      {/* Snapshot List */}
      <div className="space-y-2.5">
        {snapshots.length === 0 ? (
          <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)]">
            <HardDrive className="w-8 h-8 mx-auto mb-3 text-[var(--text-faint)] opacity-40" />
            <div>No snapshots yet.</div>
            <div className="mt-1 text-[var(--text-faint)]">
              Snapshots will be created here automatically when you install packages.
            </div>
          </div>
        ) : (
          snapshots.map((snap) => {
            const isRestored = snap.status === "Restored";
            const isCorrupted = snap.status === "Corrupted";
            const isExpanded = expandedId === snap.id;
            const isBusy = actionId === snap.id;
            const fileCount = regularCount(snap.entries);

            return (
              <div
                key={snap.id}
                className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-strong)] transition-colors text-xs"
              >
                {/* Main row */}
                <div className="p-4 flex flex-col md:flex-row md:items-start justify-between gap-3">
                  <div className="space-y-1.5 min-w-0">
                    {/* Title + badges */}
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold text-sm text-[var(--text-primary)]">
                        {snap.label}
                      </span>

                      {/* Verification badge */}
                      {snap.verified && !isCorrupted ? (
                        <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded text-green-400 bg-green-400/10 border border-green-400/20">
                          <ShieldCheck className="w-2.5 h-2.5" />
                          Verified
                        </span>
                      ) : isCorrupted ? (
                        <span className="flex items-center gap-1 text-[10px] px-1.5 py-0.5 rounded text-red-400 bg-red-400/10 border border-red-400/20">
                          <AlertTriangle className="w-2.5 h-2.5" />
                          Corrupted
                        </span>
                      ) : null}

                      {isRestored && (
                        <span className="text-[10px] px-1.5 py-0.5 rounded text-amber-400 bg-amber-400/10 border border-amber-400/20">
                          Restored
                        </span>
                      )}
                    </div>

                    {/* Metadata row */}
                    <div className="flex flex-wrap items-center gap-3 text-[11px] text-[var(--text-muted)]">
                      <div className="flex items-center gap-1">
                        <Clock className="w-3 h-3 text-[var(--text-faint)]" />
                        <span>{snap.formatted_date}</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <Folder className="w-3 h-3 text-[var(--text-faint)]" />
                        <span>{fileCount} file{fileCount !== 1 ? "s" : ""} backed up</span>
                      </div>
                      <div className="flex items-center gap-1">
                        <HardDrive className="w-3 h-3 text-[var(--text-faint)]" />
                        <span>{formatBytes(snap.total_size_bytes)}</span>
                      </div>
                      <span className="font-mono text-[10px] text-[var(--text-faint)] hidden md:inline">
                        v{snap.ryzora_version}
                      </span>
                    </div>

                    {/* Snapshot ID */}
                    <div className="font-mono text-[10px] text-[var(--text-faint)]">
                      {snap.id}
                    </div>
                  </div>

                  {/* Actions */}
                  <div className="flex items-center gap-2 flex-shrink-0">
                    <button
                      onClick={() => setExpandedId(isExpanded ? null : snap.id)}
                      className="px-2.5 py-1.5 rounded-md text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors"
                    >
                      {isExpanded ? "Hide files" : "Show files"}
                    </button>

                    <button
                      onClick={() => handleRestore(snap.id)}
                      disabled={isBusy || isCorrupted}
                      className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-[var(--rz-text)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                    >
                      <RotateCcw className="w-3 h-3" />
                      <span>{isBusy ? "Working..." : "Restore"}</span>
                    </button>

                    <button
                      onClick={() => handleDelete(snap.id)}
                      disabled={isBusy}
                      className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-md text-xs text-[var(--text-faint)] hover:text-red-400 border border-[var(--border-subtle)] hover:border-red-400/30 transition-colors disabled:opacity-40"
                      title="Delete snapshot"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                </div>

                {/* Expanded file list */}
                {isExpanded && (
                  <div className="border-t border-[var(--border-subtle)] px-4 py-3 bg-[var(--bg-canvas)] rounded-b-lg">
                    <div className="text-[11px] text-[var(--text-muted)] mb-2">
                      Backed-up entries ({snap.entries.length}):
                    </div>
                    <div className="space-y-1 max-h-56 overflow-y-auto">
                      {snap.entries.map((entry, idx) => (
                        <div
                          key={idx}
                          className="flex items-center gap-2 font-mono text-[10px] text-[var(--text-faint)]"
                        >
                          <span className="text-[var(--text-muted)] w-3 flex-shrink-0 text-center">
                            {fileEntryIcon(entry.file_type)}
                          </span>
                          <span className={entry.file_type === "Absent" ? "text-amber-500/70" : ""}>
                            {fileEntryLabel(entry)}
                          </span>
                          {entry.size_bytes > 0 && (
                            <span className="ml-auto text-[var(--text-faint)] flex-shrink-0">
                              {formatBytes(entry.size_bytes)}
                            </span>
                          )}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>

      {/* Storage note */}
      {snapshots.length > 0 && (
        <div className="text-[11px] text-[var(--text-faint)] text-center">
          Snapshots are stored in{" "}
          <code className="text-[var(--text-muted)]">~/.local/share/ryzora/snapshots/</code>
          {" "}and persist across Ryzora restarts.
        </div>
      )}
    </div>
  );
};
