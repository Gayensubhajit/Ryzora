import React from "react";
import {
  Globe,
  ArrowUpCircle,
  DownloadCloud,
  ShieldCheck,
  RefreshCw,
  Server,
  Users,
  Clock,
  ChevronRight,
  Sparkles,
  AlertTriangle,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";

export const HubView: React.FC = () => {
  const {
    hubOverview,
    loadingHub,
    refreshHub,
    refreshAllRepositoriesSync,
    loadingRepoSync,
    setActiveCategory,
    packages,
  } = useApp();

  const syncStatuses = hubOverview?.sync_summary || [];
  const featuredCreators = hubOverview?.featured_creators || [];
  const recentInstalls = hubOverview?.recent_installs || [];
  const updatesCount = hubOverview?.available_updates_count || 0;
  const securityUpdatesCount = hubOverview?.security_updates_count || 0;
  const totalInstalled = hubOverview?.total_installed || 0;
  const totalRepos = hubOverview?.total_repositories || 0;
  const onlineRepos = hubOverview?.online_repositories_count || 0;

  // Format timestamp helper
  const formatTime = (ts?: string) => {
    if (!ts) return "Never";
    const num = parseInt(ts, 10);
    if (!isNaN(num) && num > 1000000000) {
      return new Date(num * 1000).toLocaleTimeString([], {
        hour: "2-digit",
        minute: "2-digit",
      });
    }
    return ts;
  };

  return (
    <div className="space-y-6 pb-12">
      {/* Top Banner & Refresh */}
      <div className="p-5 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] shadow-xs flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2 mb-1">
            <Globe className="w-5 h-5 text-[var(--accent)]" />
            <h1 className="text-lg font-bold text-[var(--text-primary)]">Ryzora Hub</h1>
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-medium bg-[var(--accent)]/10 text-[var(--accent)] border border-[var(--accent)]/20">
              Ecosystem
            </span>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Synchronized repository sources, intelligent updates, and creator identities.
          </p>
        </div>

        <div className="flex items-center space-x-2">
          <button
            onClick={() => refreshAllRepositoriesSync()}
            disabled={loadingRepoSync}
            className="px-3.5 py-1.5 rounded-lg text-xs font-semibold text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] shadow-xs transition-colors flex items-center space-x-1.5 disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingRepoSync ? "animate-spin" : ""}`} />
            <span>{loadingRepoSync ? "Syncing Repos..." : "Sync Repositories"}</span>
          </button>

          <button
            onClick={() => refreshHub()}
            disabled={loadingHub}
            className="p-1.5 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] border border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] shadow-xs transition-colors"
            title="Refresh Hub Overview"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${loadingHub ? "animate-spin" : ""}`} />
          </button>
        </div>
      </div>

      {/* Security Alert Notice if security updates exist */}
      {securityUpdatesCount > 0 && (
        <div className="p-4 rounded-lg border border-amber-800/40 bg-amber-950/20 flex items-center justify-between gap-4">
          <div className="flex items-center space-x-3">
            <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0" />
            <div>
              <div className="text-xs font-semibold text-amber-300">
                Security Advisory Updates Available
              </div>
              <div className="text-[11px] text-amber-400/80">
                {securityUpdatesCount} package{securityUpdatesCount === 1 ? "" : "s"} require security or cryptographic attention.
              </div>
            </div>
          </div>
          <button
            onClick={() => setActiveCategory("updates")}
            className="px-3 py-1.5 rounded-md text-xs font-medium bg-amber-500/20 text-amber-200 border border-amber-500/30 hover:bg-amber-500/30 transition-colors shrink-0"
          >
            View Security Updates
          </button>
        </div>
      )}

      {/* Quick Metrics Grid */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3.5">
        <div
          onClick={() => setActiveCategory("installed")}
          className="p-4 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer group shadow-xs hover:shadow-md"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] font-medium text-[var(--text-muted)]">Installed</span>
            <DownloadCloud className="w-4 h-4 text-[var(--accent)]" />
          </div>
          <div className="text-xl font-bold text-[var(--text-primary)] mb-1">
            {totalInstalled}
          </div>
          <div className="text-[10px] text-[var(--text-faint)] flex items-center group-hover:text-[var(--text-secondary)] transition-colors">
            <span>Manage active packages</span>
            <ChevronRight className="w-3 h-3 ml-0.5" />
          </div>
        </div>

        <div
          onClick={() => setActiveCategory("updates")}
          className="p-4 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer group shadow-xs hover:shadow-md"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] font-medium text-[var(--text-muted)]">Updates</span>
            <ArrowUpCircle className={`w-4 h-4 ${updatesCount > 0 ? "text-sky-400" : "text-[var(--text-muted)]"}`} />
          </div>
          <div className="text-xl font-bold text-[var(--text-primary)] mb-1 flex items-center gap-2">
            <span>{updatesCount}</span>
            {updatesCount > 0 && (
              <span className="text-[10px] px-2 py-0.5 rounded font-mono font-medium border border-[var(--rz-badge-info-border)] bg-[var(--rz-badge-info-bg)] text-[var(--rz-badge-info-text)]">
                Available
              </span>
            )}
          </div>
          <div className="text-[10px] text-[var(--text-faint)] flex items-center group-hover:text-[var(--text-secondary)] transition-colors">
            <span>{updatesCount > 0 ? "Review update diffs" : "All packages up to date"}</span>
            <ChevronRight className="w-3 h-3 ml-0.5" />
          </div>
        </div>

        <div
          onClick={() => setActiveCategory("repositories")}
          className="p-4 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer group shadow-xs hover:shadow-md"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] font-medium text-[var(--text-muted)]">Repositories</span>
            <Server className="w-4 h-4 text-emerald-400" />
          </div>
          <div className="text-xl font-bold text-[var(--text-primary)] mb-1 flex items-center gap-2">
            <span>{onlineRepos}/{totalRepos}</span>
            <span className="text-[10px] px-2 py-0.5 rounded font-mono font-medium border border-[var(--rz-badge-success-border)] bg-[var(--rz-badge-success-bg)] text-[var(--rz-badge-success-text)]">
              Online
            </span>
          </div>
          <div className="text-[10px] text-[var(--text-faint)] flex items-center group-hover:text-[var(--text-secondary)] transition-colors">
            <span>Explorer & channels</span>
            <ChevronRight className="w-3 h-3 ml-0.5" />
          </div>
        </div>

        <div
          onClick={() => setActiveCategory("creators")}
          className="p-4 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer group shadow-xs hover:shadow-md"
        >
          <div className="flex items-center justify-between mb-2">
            <span className="text-[11px] font-medium text-[var(--text-muted)]">Creators</span>
            <Users className="w-4 h-4 text-violet-400" />
          </div>
          <div className="text-xl font-bold text-[var(--text-primary)] mb-1">
            {featuredCreators.length}
          </div>
          <div className="text-[10px] text-[var(--text-faint)] flex items-center group-hover:text-[var(--text-secondary)] transition-colors">
            <span>Keyring-verified profiles</span>
            <ChevronRight className="w-3 h-3 ml-0.5" />
          </div>
        </div>
      </div>

      {/* Repository Sync Status Summary */}
      <div className="p-5 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] shadow-xs space-y-3.5">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Server className="w-4 h-4 text-[var(--text-secondary)]" />
            <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
              Repository Synchronization
            </h2>
          </div>
          <button
            onClick={() => setActiveCategory("repositories")}
            className="text-[11px] text-[var(--accent)] hover:underline flex items-center space-x-0.5"
          >
            <span>Manage Sources</span>
            <ChevronRight className="w-3 h-3" />
          </button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          {syncStatuses.map((repo) => (
            <div
              key={repo.id}
              className="p-3.5 rounded-lg border border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] space-y-2 text-xs shadow-xs hover:border-[var(--rz-border-strong)] transition-all"
            >
              <div className="flex items-center justify-between">
                <div className="font-semibold text-[var(--text-primary)] truncate max-w-[150px]" title={repo.name}>
                  {repo.name}
                </div>
                <span
                  className={`px-1.5 py-0.5 rounded text-[10px] font-mono uppercase font-semibold ${
                    repo.status === "online"
                      ? "border border-[var(--rz-badge-success-border)] bg-[var(--rz-badge-success-bg)] text-[var(--rz-badge-success-text)]"
                      : repo.status === "cached"
                      ? "border border-[var(--rz-badge-info-border)] bg-[var(--rz-badge-info-bg)] text-[var(--rz-badge-info-text)]"
                      : "border border-[var(--rz-badge-danger-border)] bg-[var(--rz-badge-danger-bg)] text-[var(--rz-badge-danger-text)]"
                  }`}
                >
                  {repo.status}
                </span>
              </div>

              <div className="flex items-center justify-between text-[11px] text-[var(--text-muted)] font-mono">
                <span>Channel: <strong>{repo.channel}</strong></span>
                <span>{repo.package_count} pkgs</span>
              </div>

              <div className="flex items-center justify-between text-[10px] text-[var(--text-faint)]">
                <span className="flex items-center space-x-1">
                  <Clock className="w-3 h-3" />
                  <span>Synced {formatTime(repo.last_synced)}</span>
                </span>
                <span className="capitalize">{repo.repo_type}</span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Featured Creators & Recent Installs */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5">
        {/* Verified Creators */}
        <div className="p-5 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] shadow-xs space-y-3.5">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <Users className="w-4 h-4 text-violet-400" />
              <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
                Featured Creators
              </h2>
            </div>
            <button
              onClick={() => setActiveCategory("creators")}
              className="text-[11px] text-[var(--accent)] hover:underline flex items-center space-x-0.5"
            >
              <span>View All</span>
              <ChevronRight className="w-3 h-3" />
            </button>
          </div>

          <div className="space-y-2.5">
            {featuredCreators.slice(0, 4).map((creator) => (
              <div
                key={creator.id}
                onClick={() => setActiveCategory("creators")}
                className="p-3.5 rounded-lg border border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all flex items-center justify-between cursor-pointer group shadow-xs"
              >
                <div className="flex items-center space-x-3">
                  <div className="w-8 h-8 rounded-full border border-[var(--rz-badge-purple-border)] bg-[var(--rz-badge-purple-bg)] text-[var(--rz-badge-purple-text)] flex items-center justify-center text-xs font-bold shrink-0">
                    {creator.display_name.charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <div className="flex items-center space-x-1.5">
                      <span className="text-xs font-semibold text-[var(--text-primary)] group-hover:text-[var(--rz-accent)] transition-colors">
                        {creator.display_name}
                      </span>
                      {creator.verified && (
                        <span title="Cryptographically Verified Key"><ShieldCheck className="w-3.5 h-3.5 text-sky-400" /></span>
                      )}
                    </div>
                    {creator.public_key_fingerprint && (
                      <div className="text-[10px] font-mono text-[var(--text-faint)]">
                        {creator.public_key_fingerprint}
                      </div>
                    )}
                  </div>
                </div>

                <div className="text-right">
                  <div className="text-xs font-medium text-[var(--text-secondary)]">
                    {creator.total_packages} {creator.total_packages === 1 ? "pkg" : "pkgs"}
                  </div>
                  <div className="text-[10px] text-[var(--text-faint)]">
                    {creator.total_downloads.toLocaleString()} downloads
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Recent Active Installs */}
        <div className="p-5 rounded-xl border border-[var(--rz-border)] bg-[var(--rz-surface)] shadow-xs space-y-3.5">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <DownloadCloud className="w-4 h-4 text-[var(--accent)]" />
              <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
                Active Configurations
              </h2>
            </div>
            <button
              onClick={() => setActiveCategory("installed")}
              className="text-[11px] text-[var(--accent)] hover:underline flex items-center space-x-0.5"
            >
              <span>Manage Installed</span>
              <ChevronRight className="w-3 h-3" />
            </button>
          </div>

          <div className="space-y-2.5">
            {recentInstalls.length === 0 ? (
              <div className="p-6 text-center text-xs text-[var(--text-muted)]">
                No active packages installed yet.
              </div>
            ) : (
              recentInstalls.slice(0, 4).map((inst) => (
                <div
                  key={inst.package_id}
                  onClick={() => setActiveCategory("installed")}
                  className="p-3.5 rounded-lg border border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] hover:border-[var(--rz-border-strong)] transition-all flex items-center justify-between cursor-pointer group shadow-xs"
                >
                  <div>
                    <div className="text-xs font-semibold text-[var(--text-primary)] group-hover:text-[var(--rz-accent)] transition-colors">
                      {inst.name || inst.package_id}
                    </div>
                    <div className="text-[10px] text-[var(--text-muted)] font-mono">
                      v{inst.version} • {inst.installed_files.length} applied files
                    </div>
                  </div>

                  <div className="text-right">
                    <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[var(--bg-surface-elevated)] text-[var(--text-faint)]">
                      {inst.snapshot_id ? inst.snapshot_id.substring(0, 8) : "snap"}
                    </span>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {/* Latest Store Releases */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-2">
            <Sparkles className="w-4 h-4 text-[var(--accent)]" />
            <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--text-secondary)]">
              Trending Store Releases
            </h2>
          </div>
          <button
            onClick={() => setActiveCategory("discover")}
            className="text-[11px] text-[var(--accent)] hover:underline flex items-center space-x-0.5"
          >
            <span>Browse Catalog</span>
            <ChevronRight className="w-3 h-3" />
          </button>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {packages.slice(0, 3).map((pkg) => (
            <PackageCard key={pkg.id} packageItem={pkg} />
          ))}
        </div>
      </div>
    </div>
  );
};
