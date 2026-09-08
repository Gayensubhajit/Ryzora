import React, { useState } from "react";
import { ShieldCheck, RotateCcw, Clock, Folder } from "lucide-react";
import { useApp } from "../context/AppContext";

export const BackupsView: React.FC = () => {
  const { snapshots, rollbackSnapshot } = useApp();
  const [rollingBackId, setRollingBackId] = useState<string | null>(null);

  const handleRollback = async (id: string) => {
    setRollingBackId(id);
    await new Promise((r) => setTimeout(r, 600));
    await rollbackSnapshot(id);
    setRollingBackId(null);
  };

  return (
    <div className="space-y-6 pb-12">
      <div className="p-8 rounded-3xl bg-gradient-to-r from-indigo-950/50 via-slate-900/60 to-transparent border border-indigo-500/30 flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2 text-indigo-400 text-xs font-bold uppercase tracking-wider mb-2">
            <ShieldCheck className="w-4 h-4 text-cyan-400" />
            <span>Safety & Rollback Engine</span>
          </div>
          <h1 className="text-3xl font-extrabold text-white tracking-tight mb-2">
            Configuration Snapshots
          </h1>
          <p className="text-sm text-slate-400 max-w-xl leading-relaxed">
            Ryzora automatically takes timestamped snapshots of your configuration files prior to applying modifications. If a rice conflicts with your workflow, restore any snapshot with one click.
          </p>
        </div>

        <div className="p-4 rounded-2xl bg-slate-900/80 border border-slate-800 text-xs space-y-1">
          <div className="text-slate-400">Total Snapshots</div>
          <div className="font-mono text-xl font-extrabold text-cyan-300">
            {snapshots.length} Available
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
            Snapshot Timeline
          </span>
        </div>

        {snapshots.length === 0 ? (
          <div className="p-16 text-center rounded-3xl bg-slate-900/40 border border-slate-800 space-y-2">
            <div className="text-slate-300 font-semibold">No snapshots available.</div>
            <p className="text-xs text-slate-400">
              Snapshots will be created automatically when you install a customization package.
            </p>
          </div>
        ) : (
          <div className="space-y-3">
            {snapshots.map((snap) => {
              const isRestored = snap.status === "restored";

              return (
                <div
                  key={snap.id}
                  className="p-5 rounded-2xl bg-slate-900/70 border border-slate-800 hover:border-indigo-500/40 transition-all flex flex-col md:flex-row md:items-center justify-between gap-4 select-none"
                >
                  <div className="space-y-2">
                    <div className="flex items-center gap-3">
                      <span className="font-bold text-base text-white">
                        {snap.package_name}
                      </span>
                      <span className="px-2.5 py-0.5 rounded-full text-[10px] font-mono bg-slate-800 text-indigo-300 border border-slate-700">
                        {snap.id}
                      </span>
                      {isRestored && (
                        <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/20 text-amber-300 border border-amber-500/30">
                          Restored State
                        </span>
                      )}
                    </div>

                    <div className="flex items-center gap-4 text-xs text-slate-400">
                      <div className="flex items-center gap-1.5">
                        <Clock className="w-3.5 h-3.5 text-slate-500" />
                        <span>{snap.formatted_date}</span>
                      </div>
                      <div className="flex items-center gap-1.5">
                        <Folder className="w-3.5 h-3.5 text-slate-500" />
                        <span>{snap.backed_up_paths.length} Paths Protected</span>
                      </div>
                    </div>

                    <div className="flex flex-wrap gap-1.5 pt-1">
                      {snap.backed_up_paths.map((p, idx) => (
                        <code
                          key={idx}
                          className="px-2 py-0.5 rounded bg-slate-950 text-slate-400 font-mono text-[10px] border border-slate-800"
                        >
                          {p}
                        </code>
                      ))}
                    </div>
                  </div>

                  <div className="flex items-center gap-3">
                    <button
                      onClick={() => handleRollback(snap.id)}
                      disabled={rollingBackId === snap.id}
                      className="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-bold bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-300 border border-indigo-500/30 hover:border-indigo-500/50 transition-all disabled:opacity-50"
                    >
                      {rollingBackId === snap.id ? (
                        <>
                          <span className="w-3.5 h-3.5 rounded-full border-2 border-indigo-300/30 border-t-indigo-300 animate-spin" />
                          <span>Restoring...</span>
                        </>
                      ) : (
                        <>
                          <RotateCcw className="w-3.5 h-3.5" />
                          <span>Rollback to this state</span>
                        </>
                      )}
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
