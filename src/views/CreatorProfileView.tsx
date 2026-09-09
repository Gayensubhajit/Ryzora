import React, { useState } from "react";
import {
  Users,
  ShieldCheck,
  Star,
  Search,
  Key,
  Copy,
  Check,
  Package,
} from "lucide-react";
import { useApp } from "../context/AppContext";

export const CreatorProfileView: React.FC = () => {
  const { creatorProfiles, setSelectedPackage, packages } = useApp();
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [verifiedOnly, setVerifiedOnly] = useState<boolean>(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const filteredCreators = creatorProfiles.filter((creator) => {
    const matchesSearch =
      creator.display_name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      creator.id.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesVerified = !verifiedOnly || creator.verified;
    return matchesSearch && matchesVerified;
  });

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(text);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleSelectPackage = (pkgId: string) => {
    const found = packages.find((p) => p.id === pkgId);
    if (found) {
      setSelectedPackage(found);
    }
  };

  const totalVerified = creatorProfiles.filter((c) => c.verified).length;
  const totalPackages = creatorProfiles.reduce((sum, c) => sum + c.total_packages, 0);

  return (
    <div className="space-y-6 pb-12">
      {/* View Header */}
      <div className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div>
          <div className="flex items-center space-x-2 mb-1">
            <Users className="w-5 h-5 text-violet-400" />
            <h1 className="text-lg font-bold text-[var(--text-primary)]">Creator Directory</h1>
          </div>
          <p className="text-xs text-[var(--text-muted)]">
            Explore maintainers with authentic cryptographic Ed25519 identities and package portfolios.
          </p>
        </div>

        <div className="flex items-center space-x-3 text-xs text-[var(--text-muted)]">
          <span><strong>{creatorProfiles.length}</strong> Creators</span>
          <span>•</span>
          <span><strong className="text-sky-400">{totalVerified}</strong> Verified</span>
          <span>•</span>
          <span><strong>{totalPackages}</strong> Packages</span>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="p-3 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-card)] flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="relative flex-1 max-w-md">
          <Search className="w-3.5 h-3.5 absolute left-3 top-2.5 text-[var(--text-muted)]" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search creator name or handle..."
            className="w-full pl-8 pr-3 py-1.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs text-[var(--text-primary)] outline-none focus:border-[var(--accent)]"
          />
        </div>

        <div className="flex items-center space-x-2">
          <label className="flex items-center space-x-1.5 text-xs text-[var(--text-secondary)] cursor-pointer select-none">
            <input
              type="checkbox"
              checked={verifiedOnly}
              onChange={(e) => setVerifiedOnly(e.target.checked)}
              className="rounded border-[var(--border-subtle)] bg-[var(--bg-surface)] text-[var(--accent)]"
            />
            <span>Keyring Verified Only</span>
          </label>
        </div>
      </div>

      {/* Creator Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {filteredCreators.map((creator) => (
          <div
            key={creator.id}
            className="p-5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-strong)] transition-all space-y-4"
          >
            {/* Header: Avatar, Name, Badges */}
            <div className="flex items-start justify-between gap-3">
              <div className="flex items-center space-x-3">
                <div className="w-10 h-10 rounded-full bg-violet-950/60 border border-violet-800/40 flex items-center justify-center text-sm font-bold text-violet-300 shrink-0">
                  {creator.display_name.charAt(0).toUpperCase()}
                </div>
                <div>
                  <div className="flex items-center space-x-1.5">
                    <span className="text-sm font-bold text-[var(--text-primary)]">
                      {creator.display_name}
                    </span>
                    {creator.verified && (
                      <span title="Keyring Verified"><ShieldCheck className="w-4 h-4 text-sky-400" /></span>
                    )}
                  </div>
                  <div className="text-[11px] font-mono text-[var(--text-muted)]">
                    @{creator.id}
                  </div>
                </div>
              </div>

              <span
                className={`px-2 py-0.5 rounded text-[10px] font-mono uppercase font-semibold shrink-0 ${
                  creator.trust_tier === "Official"
                    ? "bg-purple-950/80 text-purple-300 border border-purple-800/50"
                    : creator.trust_tier === "Verified"
                    ? "bg-sky-950/80 text-sky-300 border border-sky-800/50"
                    : "bg-[var(--bg-card)] text-[var(--text-muted)] border border-[var(--border-subtle)]"
                }`}
              >
                {creator.trust_tier}
              </span>
            </div>

            {/* Public Key Fingerprint Badge */}
            {creator.public_key_fingerprint ? (
              <div className="p-2 rounded bg-[var(--bg-card)] border border-[var(--border-subtle)] flex items-center justify-between text-[11px] font-mono">
                <div className="flex items-center space-x-1.5 text-[var(--text-secondary)] truncate">
                  <Key className="w-3.5 h-3.5 text-sky-400 shrink-0" />
                  <span className="truncate">{creator.public_key_fingerprint}</span>
                </div>
                <button
                  onClick={() => copyToClipboard(creator.public_key_fingerprint!)}
                  className="p-1 hover:text-white text-[var(--text-muted)] shrink-0 ml-2"
                  title="Copy Fingerprint"
                >
                  {copiedKey === creator.public_key_fingerprint ? (
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                  ) : (
                    <Copy className="w-3.5 h-3.5" />
                  )}
                </button>
              </div>
            ) : (
              <div className="text-[11px] font-mono text-[var(--text-faint)] italic">
                Unvetted author • Key not in Trusted Keyring
              </div>
            )}

            {/* Metrics */}
            <div className="grid grid-cols-3 gap-2 py-2 border-y border-[var(--border-subtle)]/60 text-center text-xs">
              <div>
                <div className="text-[10px] text-[var(--text-muted)]">Packages</div>
                <div className="font-bold text-[var(--text-primary)]">{creator.total_packages}</div>
              </div>
              <div>
                <div className="text-[10px] text-[var(--text-muted)]">Downloads</div>
                <div className="font-bold text-[var(--text-primary)]">
                  {creator.total_downloads.toLocaleString()}
                </div>
              </div>
              <div>
                <div className="text-[10px] text-[var(--text-muted)]">Rating</div>
                <div className="font-bold text-[var(--text-primary)] flex items-center justify-center space-x-0.5">
                  <Star className="w-3 h-3 text-amber-400 fill-amber-400" />
                  <span>{creator.average_rating ? creator.average_rating.toFixed(1) : "—"}</span>
                </div>
              </div>
            </div>

            {/* Packages Portfolio */}
            <div className="space-y-1.5">
              <div className="text-[10px] font-semibold text-[var(--text-faint)] uppercase tracking-wider">
                Published Packages
              </div>
              <div className="flex flex-wrap gap-1.5">
                {creator.recent_packages.slice(0, 4).map((p) => (
                  <button
                    key={p.id}
                    onClick={() => handleSelectPackage(p.id)}
                    className="px-2 py-1 rounded bg-[var(--bg-card)] hover:bg-[var(--bg-surface-elevated)] border border-[var(--border-subtle)] text-[11px] text-[var(--text-secondary)] hover:text-white transition-colors flex items-center space-x-1"
                  >
                    <Package className="w-3 h-3 text-[var(--accent)]" />
                    <span>{p.name || p.id}</span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
