import React, { useState } from "react";
import {
  X,
  ShieldCheck,
  Download,
  Trash2,
  ExternalLink,
  Layers,
  HardDrive,
  Scale,
  CheckCircle2,
  AlertCircle,
  Loader2,
  Sparkles,
  Terminal,
} from "lucide-react";
import type { PackageItem } from "../../types/index.ts";
import { pacmanAppProvider } from "../../providers/pacmanProvider.ts";
import { AppIcon, resolveAppMetadata } from "./AppIconResolver.tsx";

interface AppDetailModalProps {
  app: PackageItem | null;
  onClose: () => void;
  onStatusChanged?: () => void;
}

export const AppDetailModal: React.FC<AppDetailModalProps> = ({
  app,
  onClose,
  onStatusChanged,
}) => {
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionMessage, setActionMessage] = useState<string | null>(null);

  if (!app) return null;

  const meta = pacmanAppProvider.getMeta(app);
  const isInstalled = meta?.isInstalled ?? app.tags.includes("installed");
  const appMeta = resolveAppMetadata(app.id, app.title);

  const formatBytes = (bytes?: number) => {
    if (!bytes || bytes <= 0) return "Unknown";
    const mb = bytes / (1024 * 1024);
    if (mb < 1) return `${Math.round(bytes / 1024)} KB`;
    return `${mb.toFixed(1)} MB`;
  };

  const handleInstall = async () => {
    setBusy(true);
    setActionError(null);
    setActionMessage(null);
    try {
      await pacmanAppProvider.install(app, "native");
      setActionMessage(`Installed ${appMeta.displayName} successfully.`);
      if (onStatusChanged) onStatusChanged();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setActionError(msg);
    } finally {
      setBusy(false);
    }
  };

  const handleUninstall = async () => {
    setBusy(true);
    setActionError(null);
    setActionMessage(null);
    try {
      await pacmanAppProvider.uninstall(app, "native");
      setActionMessage(`Uninstalled ${appMeta.displayName} successfully.`);
      if (onStatusChanged) onStatusChanged();
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setActionError(msg);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-md animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className="relative w-full max-w-2xl overflow-hidden rounded-3xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] shadow-2xl flex flex-col max-h-[85vh]"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Close Button */}
        <button
          onClick={onClose}
          className="absolute top-5 right-5 z-10 p-2 rounded-full bg-[var(--rz-surface)]/80 hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors shadow-sm"
          title="Close dialog"
        >
          <X className="w-4 h-4" />
        </button>

        {/* Hero Header */}
        <div className="p-8 pb-6 border-b border-[var(--rz-border-subtle)] bg-gradient-to-b from-[var(--rz-surface-elevated)]/60 to-[var(--rz-bg)] flex items-start gap-6">
          <AppIcon packageId={app.id} size="xl" className="shadow-lg" />

          <div className="flex-1 min-w-0 pr-6">
            <div className="flex items-center gap-2 flex-wrap mb-1.5">
              <h2 className="text-2xl font-black tracking-tight text-[var(--rz-text)]">
                {appMeta.displayName}
              </h2>
              {isInstalled && (
                <span className="px-2.5 py-0.5 text-xs font-semibold rounded-full bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 flex items-center gap-1.5 shadow-xs">
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Installed
                </span>
              )}
            </div>

            <div className="flex items-center gap-2 text-sm text-[var(--rz-text-muted)] font-medium">
              <span>{appMeta.publisher}</span>
              <span>•</span>
              <span className="text-[var(--rz-accent)]">{appMeta.category}</span>
              <span>•</span>
              <span className="font-mono text-xs opacity-75">{app.id}</span>
            </div>

            {/* Quick Badges */}
            <div className="flex items-center gap-2 mt-3 flex-wrap">
              <span className="px-2.5 py-1 text-xs font-semibold rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] flex items-center gap-1.5">
                <Terminal className="w-3 h-3 text-[var(--rz-accent)]" />
                pacman • {meta?.repository || "extra"}
              </span>
              <span className="px-2.5 py-1 text-xs font-semibold rounded-lg bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 flex items-center gap-1.5">
                <ShieldCheck className="w-3 h-3" />
                Official Arch Repository
              </span>
            </div>
          </div>
        </div>

        {/* Scrollable Body */}
        <div className="p-8 py-6 space-y-6 overflow-y-auto flex-1">
          {/* Action Notifications */}
          {actionMessage && (
            <div className="p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-sm flex items-center gap-2.5 animate-in fade-in">
              <CheckCircle2 className="w-4 h-4 shrink-0" />
              <span className="font-medium">{actionMessage}</span>
            </div>
          )}

          {actionError && (
            <div className="p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-400 text-sm flex items-center gap-2.5 animate-in fade-in">
              <AlertCircle className="w-4 h-4 shrink-0" />
              <span className="font-medium">{actionError}</span>
            </div>
          )}

          {/* Description */}
          <div>
            <h3 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] mb-2.5">
              About this Application
            </h3>
            <p className="text-sm text-[var(--rz-text)] leading-relaxed font-normal">
              {app.description}
            </p>
          </div>

          {/* Spec Cards */}
          <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <div className="p-3.5 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-1.5 text-xs text-[var(--rz-text-muted)] mb-1 font-medium">
                <Layers className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                <span>Version</span>
              </div>
              <div className="text-sm font-bold text-[var(--rz-text)] truncate">{app.version}</div>
            </div>

            <div className="p-3.5 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-1.5 text-xs text-[var(--rz-text-muted)] mb-1 font-medium">
                <HardDrive className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                <span>Download Size</span>
              </div>
              <div className="text-sm font-bold text-[var(--rz-text)]">
                {formatBytes(app.package_size_bytes)}
              </div>
            </div>

            <div className="p-3.5 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-1.5 text-xs text-[var(--rz-text-muted)] mb-1 font-medium">
                <Scale className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                <span>License</span>
              </div>
              <div className="text-sm font-bold text-[var(--rz-text)] truncate">
                {meta?.license || "Open Source"}
              </div>
            </div>

            <div className="p-3.5 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-1.5 text-xs text-[var(--rz-text-muted)] mb-1 font-medium">
                <Sparkles className="w-3.5 h-3.5 text-emerald-400" />
                <span>Quality</span>
              </div>
              <div className="text-sm font-bold text-emerald-400">Verified Native</div>
            </div>
          </div>

          {/* Multi-source Availability Card */}
          <div className="p-4 rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] space-y-3">
            <div className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)]">
              Package Source
            </div>

            <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-accent)]/30 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-lg bg-[var(--rz-accent)]/15 text-[var(--rz-accent)] flex items-center justify-center font-bold text-sm">
                  PAC
                </div>
                <div>
                  <div className="text-sm font-bold text-[var(--rz-text)] flex items-center gap-2">
                    Arch Linux Official ({meta?.repository || "extra"})
                    <span className="text-[10px] uppercase font-bold px-1.5 py-0.5 rounded bg-[var(--rz-accent)]/20 text-[var(--rz-accent)]">
                      Primary
                    </span>
                  </div>
                  <div className="text-xs text-[var(--rz-text-muted)] mt-0.5">
                    Maintained by Arch Linux core packagers • Native binary
                  </div>
                </div>
              </div>

              <span className="text-xs font-semibold text-[var(--rz-accent)]">
                {isInstalled ? "Installed" : "Available"}
              </span>
            </div>
          </div>

          {/* Dependencies */}
          {meta?.dependencies && meta.dependencies.length > 0 && (
            <div>
              <h3 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] mb-2.5">
                Runtime Dependencies ({meta.dependencies.length})
              </h3>
              <div className="flex flex-wrap gap-1.5 max-h-24 overflow-y-auto p-1">
                {meta.dependencies.map((dep) => (
                  <span
                    key={dep}
                    className="px-2.5 py-1 text-xs rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)] font-mono"
                  >
                    {dep}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Footer Actions */}
        <div className="p-6 border-t border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)]/40 flex items-center justify-between gap-4">
          <div>
            {meta?.url ? (
              <a
                href={meta.url}
                target="_blank"
                rel="noopener noreferrer"
                className="px-4 py-2 text-xs font-medium rounded-xl bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] hover:text-[var(--rz-accent)] transition-colors inline-flex items-center gap-2"
              >
                <span>Visit Upstream Website</span>
                <ExternalLink className="w-3.5 h-3.5" />
              </a>
            ) : (
              <span className="text-xs text-[var(--rz-text-muted)]">Native Arch Package</span>
            )}
          </div>

          <div className="flex items-center gap-3">
            {isInstalled ? (
              <button
                onClick={handleUninstall}
                disabled={busy}
                className="px-5 py-2.5 text-sm font-semibold rounded-xl bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 transition-all flex items-center gap-2 disabled:opacity-50 shadow-sm"
              >
                {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : <Trash2 className="w-4 h-4" />}
                <span>Uninstall Application</span>
              </button>
            ) : (
              <button
                onClick={handleInstall}
                disabled={busy}
                className="px-6 py-2.5 text-sm font-semibold rounded-xl bg-[var(--rz-accent)] hover:opacity-90 text-[var(--rz-text-on-accent,#fff)] shadow-lg shadow-[var(--rz-accent)]/25 transition-all flex items-center gap-2 disabled:opacity-50 active:scale-95"
              >
                {busy ? <Loader2 className="w-4 h-4 animate-spin" /> : <Download className="w-4 h-4" />}
                <span>Install Application</span>
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
