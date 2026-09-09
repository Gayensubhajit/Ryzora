import React, { useState } from "react";
import {
  Server,
  RefreshCw,
  Plus,
  Trash2,
  Clock,
  HardDrive,
  Globe,
} from "lucide-react";
import { useApp } from "../context/AppContext";

export const RepositoryView: React.FC = () => {
  const {
    repositorySyncStatuses,
    loadingRepoSync,
    refreshRepositorySync,
    refreshAllRepositoriesSync,
    switchRepositoryChannel,
    addRepositorySource,
    removeRepositorySource,
    setToast,
  } = useApp();

  const [syncingId, setSyncingId] = useState<string | null>(null);
  const [switchingChannelId, setSwitchingChannelId] = useState<string | null>(null);
  const [showAddModal, setShowAddModal] = useState<boolean>(false);
  const [newId, setNewId] = useState<string>("");
  const [newName, setNewName] = useState<string>("");
  const [newUrl, setNewUrl] = useState<string>("");

  const handleSyncSingle = async (repoId: string) => {
    setSyncingId(repoId);
    try {
      await refreshRepositorySync(repoId);
    } finally {
      setSyncingId(null);
    }
  };

  const handleChannelChange = async (repoId: string, channel: string) => {
    setSwitchingChannelId(repoId);
    try {
      await switchRepositoryChannel(repoId, channel);
    } catch (e: any) {
      setToast({
        message: `Failed to switch channel: ${e?.message || e}`,
        type: "warning",
      });
    } finally {
      setSwitchingChannelId(null);
    }
  };

  const handleAddSource = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newId.trim() || !newName.trim() || !newUrl.trim()) {
      setToast({ message: "All fields are required", type: "warning" });
      return;
    }
    if (!newUrl.startsWith("https://")) {
      setToast({ message: "Remote repository must use HTTPS scheme only", type: "warning" });
      return;
    }

    try {
      await addRepositorySource({
        id: newId.trim(),
        name: newName.trim(),
        url: newUrl.trim(),
        enabled: true,
        repo_type: "remote",
      });
      setShowAddModal(false);
      setNewId("");
      setNewName("");
      setNewUrl("");
    } catch (err: any) {
      setToast({ message: `Add source failed: ${err?.message || err}`, type: "warning" });
    }
  };

  const formatTime = (ts?: string) => {
    if (!ts) return "Never";
    const num = parseInt(ts, 10);
    if (!isNaN(num) && num > 1000000000) {
      return new Date(num * 1000).toLocaleString([], {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    }
    return ts;
  };

  return (
    <div className="space-y-6 pb-12">
      {/* View Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2 mb-1">
            <Server className="w-5 h-5 text-[var(--accent)]" />
            <h1 className="text-lg font-bold text-[var(--text-primary)]">Repositories & Channels</h1>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Manage upstream sources, synchronize index caches, and select release channels (Stable, Beta, Nightly).
          </p>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => refreshAllRepositoriesSync()}
            disabled={loadingRepoSync}
            className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1.5 disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingRepoSync ? "animate-spin" : ""}`} />
            <span>{loadingRepoSync ? "Syncing All..." : "Sync All Sources"}</span>
          </button>

          <button
            onClick={() => setShowAddModal(true)}
            className="px-3 py-1.5 rounded-md text-xs font-semibold bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] transition-colors flex items-center space-x-1.5 shadow-sm"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Source</span>
          </button>
        </div>
      </div>

      {/* Repositories List */}
      <div className="space-y-4">
        {repositorySyncStatuses.map((repo) => (
          <div
            key={repo.id}
            className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-card)] space-y-4 transition-all hover:border-[var(--border-strong)]"
          >
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
              <div className="space-y-1">
                <div className="flex items-center space-x-2">
                  {repo.repo_type === "local" ? (
                    <HardDrive className="w-4 h-4 text-sky-400 shrink-0" />
                  ) : (
                    <Globe className="w-4 h-4 text-emerald-400 shrink-0" />
                  )}
                  <span className="text-sm font-bold text-[var(--text-primary)]">{repo.name}</span>
                  <span className="text-[10px] font-mono text-[var(--text-faint)]">({repo.id})</span>
                  <span
                    className={`px-2 py-0.5 rounded text-[10px] font-mono uppercase font-semibold ${
                      repo.status === "online"
                        ? "bg-emerald-950/80 text-emerald-400 border border-emerald-800/40"
                        : repo.status === "cached"
                        ? "bg-sky-950/80 text-sky-400 border border-sky-800/40"
                        : "bg-red-950/80 text-red-400 border border-red-800/40"
                    }`}
                  >
                    {repo.status}
                  </span>
                </div>

                <div className="text-xs text-[var(--text-muted)] font-mono break-all">
                  {repo.url}
                </div>
              </div>

              <div className="flex items-center space-x-2 shrink-0">
                <button
                  onClick={() => handleSyncSingle(repo.id)}
                  disabled={syncingId === repo.id}
                  className="px-3 py-1.5 rounded-md text-xs font-medium text-[var(--text-secondary)] hover:text-white border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-elevated)] transition-colors flex items-center space-x-1.5 disabled:opacity-50"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${syncingId === repo.id ? "animate-spin" : ""}`} />
                  <span>{syncingId === repo.id ? "Syncing..." : "Sync Now"}</span>
                </button>

                {repo.repo_type === "remote" && repo.id !== "ryzora-official" && repo.id !== "ryzora-community" && (
                  <button
                    onClick={() => removeRepositorySource(repo.id)}
                    className="p-1.5 rounded-md text-red-400 hover:text-red-300 hover:bg-red-950/30 border border-red-900/40 transition-colors"
                    title="Remove Repository"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>
            </div>

            {/* Repository Info & Channel Selector */}
            <div className="pt-3 border-t border-[var(--border-subtle)]/60 flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
              <div className="flex items-center space-x-4 text-[var(--text-muted)]">
                <span>
                  <strong>{repo.package_count}</strong> packages discovered
                </span>
                <span className="flex items-center space-x-1">
                  <Clock className="w-3.5 h-3.5" />
                  <span>Last sync: {formatTime(repo.last_synced)}</span>
                </span>
              </div>

              {/* Channel Selector */}
              <div className="flex items-center space-x-2">
                <span className="text-[11px] font-semibold text-[var(--text-secondary)]">
                  Release Channel:
                </span>
                <div className="flex items-center space-x-1 p-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                  {repo.available_channels.map((ch) => (
                    <button
                      key={ch}
                      onClick={() => handleChannelChange(repo.id, ch)}
                      disabled={switchingChannelId === repo.id}
                      className={`px-2 py-0.5 rounded text-[11px] font-mono capitalize transition-all ${
                        repo.channel === ch
                          ? "bg-[var(--accent)] text-white font-semibold shadow-xs"
                          : "text-[var(--text-muted)] hover:text-white"
                      }`}
                    >
                      {ch}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Add Repository Modal */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <form
            onSubmit={handleAddSource}
            className="w-full max-w-md bg-[var(--bg-surface)] border border-[var(--border-strong)] rounded-lg shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150"
          >
            <div className="p-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
              <h3 className="text-sm font-bold text-[var(--text-primary)]">Add Remote Repository</h3>
              <button
                type="button"
                onClick={() => setShowAddModal(false)}
                className="text-[var(--text-muted)] hover:text-white text-xs px-2 py-1"
              >
                ✕
              </button>
            </div>

            <div className="p-5 space-y-4 text-xs">
              <div className="space-y-1">
                <label className="text-[11px] font-semibold text-[var(--text-secondary)]">
                  Repository Identifier (slug)
                </label>
                <input
                  type="text"
                  value={newId}
                  onChange={(e) => setNewId(e.target.value)}
                  placeholder="e.g. archcraft-themes"
                  className="w-full px-3 py-1.5 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] text-[var(--text-primary)] font-mono text-xs focus:border-[var(--accent)] outline-none"
                  required
                />
              </div>

              <div className="space-y-1">
                <label className="text-[11px] font-semibold text-[var(--text-secondary)]">
                  Display Name
                </label>
                <input
                  type="text"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  placeholder="e.g. Archcraft Official Themes"
                  className="w-full px-3 py-1.5 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] text-[var(--text-primary)] text-xs focus:border-[var(--accent)] outline-none"
                  required
                />
              </div>

              <div className="space-y-1">
                <label className="text-[11px] font-semibold text-[var(--text-secondary)]">
                  HTTPS Base URL
                </label>
                <input
                  type="url"
                  value={newUrl}
                  onChange={(e) => setNewUrl(e.target.value)}
                  placeholder="https://raw.githubusercontent.com/user/repo/main"
                  className="w-full px-3 py-1.5 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] text-[var(--text-primary)] font-mono text-xs focus:border-[var(--accent)] outline-none"
                  required
                />
                <p className="text-[10px] text-[var(--text-faint)]">
                  Must point to an HTTPS endpoint hosting repository.json schema v1.
                </p>
              </div>
            </div>

            <div className="p-4 border-t border-[var(--border-subtle)] bg-[var(--bg-card)] flex items-center justify-end space-x-2">
              <button
                type="button"
                onClick={() => setShowAddModal(false)}
                className="px-3 py-1.5 rounded text-xs font-medium text-[var(--text-muted)] hover:text-white border border-[var(--border-subtle)]"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-1.5 rounded text-xs font-semibold bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] transition-colors"
              >
                Add Repository
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};
