import React, { useState, useEffect } from "react";
import {
  RefreshCw,
  Check,
  X,
  HardDrive,
  Database,
  Trash2,
  AlertCircle,
  Clock,
  ShieldCheck,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { CacheStats } from "../types";

export const SystemView: React.FC = () => {
  const {
    systemInfo,
    loadingSystem,
    refreshSystem,
    repositories,
    refreshRepositories,
    getCacheStats,
    clearAllCache,
  } = useApp();

  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null);
  const [cleaningCache, setCleaningCache] = useState<boolean>(false);
  const [refreshingRepos, setRefreshingRepos] = useState<boolean>(false);

  const loadStats = async () => {
    try {
      const stats = await getCacheStats();
      setCacheStats(stats);
    } catch {
      // Handled silently
    }
  };

  useEffect(() => {
    loadStats();
  }, []);

  const handleCleanCache = async () => {
    setCleaningCache(true);
    try {
      await clearAllCache();
      await loadStats();
    } finally {
      setCleaningCache(false);
    }
  };

  const handleRefreshRepos = async () => {
    setRefreshingRepos(true);
    try {
      await refreshRepositories();
      await loadStats();
    } finally {
      setRefreshingRepos(false);
    }
  };

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return "0 B";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return (
    <div className="space-y-6 pb-10">
      {/* Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex items-center justify-between">
        <div>
          <h1 className="text-lg font-bold text-[var(--text-primary)] mb-1">
            System & Infrastructure Diagnostics
          </h1>
          <p className="text-xs text-[var(--text-muted)]">
            Detected hardware, display protocol, repositories, and cache storage confinement.
          </p>
        </div>

        <button
          onClick={refreshSystem}
          disabled={loadingSystem}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-primary)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors disabled:opacity-50"
        >
          <RefreshCw className={`w-3 h-3 ${loadingSystem ? "animate-spin" : ""}`} />
          <span>{loadingSystem ? "Probing..." : "Re-probe"}</span>
        </button>
      </div>

      {/* Grid of 4 Key Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Distribution</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.distro_name || "Garuda Linux"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.distro_version}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Window Manager</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.window_manager || "Hyprland"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5 capitalize">
            {systemInfo?.session_type || "wayland"}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Kernel</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.kernel_version || "6.18"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.distro_id}
          </div>
        </div>

        <div className="p-3.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)]">
          <div className="text-[10px] text-[var(--text-faint)] uppercase mb-1">Terminal & Shell</div>
          <div className="text-sm font-semibold text-[var(--text-primary)] truncate">
            {systemInfo?.terminal || "Kitty"}
          </div>
          <div className="text-[11px] font-mono text-[var(--text-muted)] mt-0.5">
            {systemInfo?.shell || "zsh"}
          </div>
        </div>
      </div>

      {/* Configured Repositories Section */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
          <div className="flex items-center gap-1.5 font-semibold text-[var(--text-primary)]">
            <Database className="w-3.5 h-3.5 text-[var(--accent-text)]" />
            <span>Configured Repositories ({repositories.length})</span>
          </div>

          <button
            onClick={handleRefreshRepos}
            disabled={refreshingRepos}
            className="flex items-center gap-1 text-[11px] text-[var(--text-muted)] hover:text-[var(--rz-text)] transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-3 h-3 ${refreshingRepos ? "animate-spin" : ""}`} />
            <span>{refreshingRepos ? "Refreshing..." : "Refresh All"}</span>
          </button>
        </div>

        <div className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] divide-y divide-[var(--border-subtle)] overflow-hidden">
          {repositories.map((repo) => {
            const isOffline = repo.status === "offline";
            const isFailed = repo.status === "refresh_failed";

            return (
              <div key={repo.id} className="p-3.5 space-y-2 text-xs">
                <div className="flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-[var(--text-primary)]">{repo.name}</span>
                    <span className="font-mono text-[10px] text-[var(--text-faint)] px-1.5 py-0.2 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
                      {repo.id}
                    </span>
                    <span className="text-[11px] text-[var(--text-faint)]">
                      ({repo.package_count} package{repo.package_count === 1 ? "" : "s"})
                    </span>
                  </div>

                  <div className="flex items-center gap-2">
                    {isOffline ? (
                      <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase text-amber-400 bg-amber-400/10 border border-amber-400/30">
                        Offline (Cached)
                      </span>
                    ) : isFailed ? (
                      <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase text-rose-400 bg-rose-400/10 border border-rose-400/30">
                        Refresh Failed
                      </span>
                    ) : (
                      <span className="text-[10px] px-1.5 py-0.5 rounded font-mono uppercase text-emerald-400 bg-emerald-400/10 border border-emerald-400/30">
                        Online
                      </span>
                    )}
                  </div>
                </div>

                <div className="flex items-center justify-between text-[11px] font-mono text-[var(--text-muted)]">
                  <div className="truncate max-w-md text-[var(--text-faint)]">
                    {repo.path}
                  </div>

                  {repo.last_refreshed && (
                    <div className="flex items-center gap-1 text-[var(--text-faint)] text-[10px]">
                      <Clock className="w-3 h-3" />
                      <span>Refreshed: {repo.last_refreshed}</span>
                    </div>
                  )}
                </div>

                {repo.last_error && (
                  <div className="p-2 rounded bg-rose-950/20 border border-rose-800/40 text-rose-300 text-[11px] flex items-center gap-1.5">
                    <AlertCircle className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                    <span className="truncate">{repo.last_error}</span>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Cache & Confinement Section */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
          <div className="flex items-center gap-1.5 font-semibold text-[var(--text-primary)]">
            <HardDrive className="w-3.5 h-3.5 text-[var(--accent-text)]" />
            <span>Package Cache & Storage Confinement</span>
          </div>

          <button
            onClick={handleCleanCache}
            disabled={cleaningCache || !cacheStats || cacheStats.total_size_bytes === 0}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded text-[11px] font-medium text-rose-400 hover:text-rose-300 border border-rose-500/30 hover:bg-rose-500/10 transition-colors disabled:opacity-40"
          >
            <Trash2 className="w-3 h-3" />
            <span>{cleaningCache ? "Cleaning..." : "Clean Package Cache"}</span>
          </button>
        </div>

        <div className="p-4 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-3">
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
            <div className="p-2.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-0.5">
              <div className="text-[10px] uppercase text-[var(--text-faint)] font-mono">Total Cache Size</div>
              <div className="text-sm font-semibold text-[var(--text-primary)] font-mono">
                {formatBytes(cacheStats?.total_size_bytes || 0)}
              </div>
            </div>

            <div className="p-2.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-0.5">
              <div className="text-[10px] uppercase text-[var(--text-faint)] font-mono">Cached Packages</div>
              <div className="text-sm font-semibold text-[var(--text-primary)] font-mono">
                {cacheStats?.cached_packages_count || 0}
              </div>
            </div>

            <div className="p-2.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] space-y-0.5">
              <div className="text-[10px] uppercase text-[var(--text-faint)] font-mono">Cached Repositories</div>
              <div className="text-sm font-semibold text-[var(--text-primary)] font-mono">
                {cacheStats?.cached_repositories_count || 0}
              </div>
            </div>
          </div>

          <div className="p-2.5 rounded bg-black/40 border border-[var(--border-subtle)] text-[11px] font-mono text-[var(--text-muted)] space-y-1">
            <div className="text-[10px] uppercase text-[var(--text-faint)]">Cache Directory Root</div>
            <div className="text-[var(--text-primary)] truncate">
              {cacheStats?.cache_dir || "~/.cache/ryzora"}
            </div>
          </div>

          <div className="flex items-center gap-1.5 text-[11px] text-[var(--text-faint)] pt-1 border-t border-[var(--border-subtle)]">
            <ShieldCheck className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
            <span>
              Cache operations are strictly confined to Ryzora's cache root and cannot affect ~/.config, installed packages, or snapshots.
            </span>
          </div>
        </div>
      </div>

      {/* Installed Components Table */}
      <div className="space-y-2.5">
        <div className="flex items-center justify-between text-xs text-[var(--text-faint)]">
          <span>Component Status ({systemInfo?.installed_components.filter((c) => c.installed).length || 0} installed)</span>
        </div>

        <div className="rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] divide-y divide-[var(--border-subtle)] overflow-hidden">
          {systemInfo?.installed_components.map((c, idx) => (
            <div
              key={idx}
              className="p-3 flex items-center justify-between gap-3 text-xs"
            >
              <div className="flex items-center gap-2.5">
                {c.installed ? (
                  <Check className="w-3.5 h-3.5 text-emerald-400 flex-shrink-0" />
                ) : (
                  <X className="w-3.5 h-3.5 text-[var(--text-faint)] flex-shrink-0" />
                )}
                <div>
                  <span className="font-medium text-[var(--text-primary)] mr-2">{c.name}</span>
                  <span className="text-[11px] text-[var(--text-faint)]">{c.category}</span>
                </div>
              </div>

              <div className="flex items-center gap-3 font-mono text-[11px]">
                {c.path ? (
                  <span className="text-[var(--text-muted)]">{c.path}</span>
                ) : (
                  <span className="text-[var(--text-faint)] italic">not found</span>
                )}
                <span
                  className={`text-[10px] px-1.5 py-0.2 rounded font-mono ${
                    c.installed
                      ? "text-emerald-400 bg-emerald-400/10 border border-emerald-400/20"
                      : "text-[var(--text-faint)] bg-[var(--bg-canvas)] border border-[var(--border-subtle)]"
                  }`}
                >
                  {c.installed ? "present" : "missing"}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
