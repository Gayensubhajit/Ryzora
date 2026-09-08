import React, { useState } from "react";
import { RotateCcw, Clock, Folder } from "lucide-react";
import { useApp } from "../context/AppContext";

export const BackupsView: React.FC = () => {
  const { snapshots, rollbackSnapshot } = useApp();
  const [rollingBackId, setRollingBackId] = useState<string | null>(null);

  const handleRollback = async (id: string) => {
    setRollingBackId(id);
    await new Promise((r) => setTimeout(r, 500));
    await rollbackSnapshot(id);
    setRollingBackId(null);
  };

  return (
    <div className="space-y-6 pb-10">
      {/* Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            Configuration Snapshots
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Timestamped snapshots of modified directories in <code className="text-slate-300">~/.config/</code>. Restore previous states at any time.
          </p>
        </div>

        <div className="text-xs font-mono text-[var(--text-muted)] px-2.5 py-1 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
          {snapshots.length} Snapshots
        </div>
      </div>

      {/* Snapshot List */}
      <div className="space-y-2.5">
        {snapshots.length === 0 ? (
          <div className="p-12 text-center rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs text-[var(--text-muted)]">
            No snapshots created yet.
          </div>
        ) : (
          snapshots.map((snap) => {
            const isRestored = snap.status === "restored";

            return (
              <div
                key={snap.id}
                className="p-4 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-strong)] transition-colors flex flex-col md:flex-row md:items-center justify-between gap-3 text-xs"
              >
                <div className="space-y-1.5">
                  <div className="flex items-center gap-2.5">
                    <span className="font-semibold text-sm text-[var(--text-primary)]">
                      {snap.package_name}
                    </span>
                    <span className="text-[10px] font-mono text-[var(--text-faint)] px-1.5 py-0.2 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
                      {snap.id}
                    </span>
                    {isRestored && (
                      <span className="text-[10px] px-1.5 py-0.2 rounded text-amber-400 bg-amber-400/10 border border-amber-400/20">
                        Restored
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-3 text-[11px] text-[var(--text-muted)]">
                    <div className="flex items-center gap-1">
                      <Clock className="w-3 h-3 text-[var(--text-faint)]" />
                      <span>{snap.formatted_date}</span>
                    </div>
                    <div className="flex items-center gap-1">
                      <Folder className="w-3 h-3 text-[var(--text-faint)]" />
                      <span>{snap.backed_up_paths.length} protected paths</span>
                    </div>
                  </div>

                  {/* Paths */}
                  <div className="flex flex-wrap gap-1 pt-1">
                    {snap.backed_up_paths.map((p, idx) => (
                      <code
                        key={idx}
                        className="px-1.5 py-0.5 rounded bg-[var(--bg-canvas)] text-[10px] font-mono text-[var(--text-faint)] border border-[var(--border-subtle)]"
                      >
                        {p}
                      </code>
                    ))}
                  </div>
                </div>

                {/* Rollback button */}
                <div className="flex items-center gap-2 flex-shrink-0">
                  <button
                    onClick={() => handleRollback(snap.id)}
                    disabled={rollingBackId === snap.id}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors disabled:opacity-50"
                  >
                    <RotateCcw className="w-3 h-3" />
                    <span>{rollingBackId === snap.id ? "Restoring..." : "Restore Snapshot"}</span>
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
