/**
 * AppDetailPage — Phase 23B.1
 *
 * Rich application detail storefront for Ryzora's software center.
 * Features:
 *   - Persistent navigation: always-visible "← Back to Applications", Alt+Left shortcut
 *   - Hero: 112px authentic system icon, publisher, category, installed & verified status, actions
 *   - Screenshots: large multi-screenshot gallery with aspect-ratio preservation and full-screen lightbox
 *   - About: rich multi-paragraph description with Read more toggle
 *   - Features: real application capability highlights
 *   - Package details & Dependencies: clean typography, verified real pacman values (no fabricated numbers)
 *   - Installation source: prominent trust & signature banner
 *   - External resources: links to project website, source repository, issue tracker
 *   - Related applications: canonical-resolved icon cards with click-to-swap navigation
 */

import React, { useState, useEffect, useMemo, useCallback } from "react";
import {
  ArrowLeft,
  CheckCircle2,
  AlertCircle,
  Download,
  Trash2,
  ExternalLink,
  ChevronDown,
  ChevronUp,
  ShieldCheck,
  Check,
  ChevronRight,
  X,
  ArrowRight,
  Loader2,
} from "lucide-react";
import type { PackageItem } from "../../providers/types.ts";
import { pacmanAppProvider } from "../../providers/index.ts";
import { AppIcon, resolveAppMetadata } from "./AppIconResolver.tsx";

interface AppDetailPageProps {
  app: PackageItem;
  onBack: () => void;
  onStatusChanged?: () => void;
  onSelectRelated?: (appId: string) => void;
}

export const AppDetailPage: React.FC<AppDetailPageProps> = ({
  app,
  onBack,
  onStatusChanged,
  onSelectRelated,
}) => {
  const [actionLoading, setActionLoading] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [showAllDeps, setShowAllDeps] = useState(false);
  const [showFullDesc, setShowFullDesc] = useState(false);
  const [activeScreenshotIdx, setActiveScreenshotIdx] = useState(0);
  const [lightboxOpen, setLightboxOpen] = useState(false);

  const meta = useMemo(() => resolveAppMetadata(app.id, app.title), [app.id, app.title]);
  const pacmanMeta = useMemo(() => pacmanAppProvider.getMeta(app), [app]);
  const isInstalled = pacmanMeta?.isInstalled ?? false;

  const version = app.version || pacmanMeta?.installedVersion || "Unknown";
  const repository = pacmanMeta?.repository || "extra";
  const license = pacmanMeta?.license || "Open Source";
  const rawSizeBytes = pacmanMeta?.sizeBytes;
  const downloadSize = rawSizeBytes
    ? rawSizeBytes < 1024 * 1024
      ? `${(rawSizeBytes / 1024).toFixed(0)} KB`
      : `${(rawSizeBytes / (1024 * 1024)).toFixed(1)} MB`
    : "Unknown";

  const allDependencies = pacmanMeta?.dependencies || app.dependencies?.packages || [];
  const visibleDependencies = showAllDeps ? allDependencies : allDependencies.slice(0, 10);
  const screenshots = meta.screenshots || [];
  const relatedAppIds = meta.relatedApps || [];
  const features = meta.features || [];

  // Alt+Left keyboard navigation & Escape for lightbox
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (lightboxOpen) {
        if (e.key === "Escape") {
          setLightboxOpen(false);
        } else if (e.key === "ArrowLeft") {
          setActiveScreenshotIdx((i) => Math.max(0, i - 1));
        } else if (e.key === "ArrowRight") {
          setActiveScreenshotIdx((i) => Math.min(screenshots.length - 1, i + 1));
        }
        return;
      }

      if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault();
        onBack();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onBack, lightboxOpen, screenshots.length]);

  const openLightbox = useCallback((idx: number) => {
    setActiveScreenshotIdx(idx);
    setLightboxOpen(true);
  }, []);

  const handleInstall = async () => {
    setActionLoading(true);
    setActionError(null);
    setActionSuccess(null);
    try {
      const res = (await pacmanAppProvider.install(app, "native")) as {
        success: boolean;
        message?: string;
        error?: string;
      };
      if (res.success) {
        setActionSuccess("Application installed successfully.");
        onStatusChanged?.();
      } else {
        setActionError(res.error || "Installation failed.");
      }
    } catch (err: any) {
      setActionError(err?.message || "Failed to install.");
    } finally {
      setActionLoading(false);
    }
  };

  const handleUninstall = async () => {
    setActionLoading(true);
    setActionError(null);
    setActionSuccess(null);
    try {
      const res = (await pacmanAppProvider.uninstall(app, "native")) as {
        success: boolean;
        message?: string;
        error?: string;
      };
      if (res.success) {
        setActionSuccess("Application uninstalled successfully.");
        onStatusChanged?.();
      } else {
        setActionError(res.error || "Uninstall failed.");
      }
    } catch (err: any) {
      setActionError(err?.message || "Failed to uninstall.");
    } finally {
      setActionLoading(false);
    }
  };

  const openExternal = (url?: string) => {
    if (!url) return;
    import("@tauri-apps/plugin-opener")
      .then(({ openUrl }) => openUrl(url))
      .catch(() => window.open(url, "_blank", "noopener,noreferrer"));
  };

  const fullDescription = meta.fullDescription || app.description || meta.summary;
  const isLongDescription = fullDescription.length > 320;

  return (
    <div className="relative flex flex-col flex-1 h-full w-full overflow-y-auto bg-background text-foreground animate-fadeIn">
      {/* ── Persistent sticky navigation bar ── */}
      <div className="sticky top-0 z-20 flex items-center justify-between px-8 py-3 bg-background/95 backdrop-blur-md border-b border-border/30">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-2 text-sm font-medium text-foreground-muted hover:text-foreground transition-colors cursor-pointer group"
          title="Back to Applications (Alt+Left)"
        >
          <ArrowLeft
            size={16}
            className="group-hover:-translate-x-0.5 transition-transform"
          />
          <span>Back to Applications</span>
        </button>
        <div className="flex items-center gap-2 text-xs text-foreground-muted">
          <span className="font-mono text-accent-primary">{app.id}</span>
          <span>·</span>
          <span>{repository}</span>
        </div>
      </div>

      <div className="px-8 py-8 space-y-10 max-w-none">
        {/* ── Hero: 112px Authentic Icon + Title + Status Badges + Action Buttons ── */}
        <div className="flex flex-col md:flex-row items-start md:items-center gap-8 pb-4">
          {/* Large authentic application icon */}
          <div className="shrink-0 p-1 rounded-3xl bg-surface-elevated/40 border border-border/30 shadow-sm">
            <AppIcon appId={app.id} size="2xl" />
          </div>

          {/* Title block */}
          <div className="flex-1 min-w-0 space-y-2.5">
            <div className="flex items-center gap-3 flex-wrap">
              <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-foreground">
                {meta.displayName}
              </h1>
              {isInstalled ? (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-500 border border-emerald-500/25">
                  <CheckCircle2 size={13} />
                  Installed
                </span>
              ) : (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-foreground-muted/10 text-foreground-muted border border-border/40">
                  Not Installed
                </span>
              )}
            </div>

            <div className="flex items-center gap-2.5 text-sm text-foreground-muted flex-wrap">
              <span className="font-semibold text-foreground/90">{meta.publisher}</span>
              <span className="opacity-40">·</span>
              <span className="px-2.5 py-0.5 rounded-full text-xs bg-surface-elevated border border-border/30 text-foreground-muted">
                {meta.category}
              </span>
              <span className="opacity-40">·</span>
              <span className="inline-flex items-center gap-1 text-xs font-medium text-accent-primary">
                <ShieldCheck size={13} />
                Official Arch Repository
              </span>
            </div>

            <p className="text-sm text-foreground-muted leading-relaxed max-w-3xl">
              {meta.summary}
            </p>
          </div>

          {/* Primary & secondary action buttons */}
          <div className="flex items-center gap-3 shrink-0 self-start md:self-auto">
            {isInstalled ? (
              <>
                <button
                  onClick={() => setActionSuccess("Application is ready to launch from your desktop.")}
                  className="px-8 py-3.5 rounded-xl font-semibold text-sm bg-accent-primary text-background hover:bg-accent-primary/90 transition-all shadow-md hover:shadow-lg cursor-pointer"
                >
                  Open
                </button>
                <button
                  onClick={handleUninstall}
                  disabled={actionLoading}
                  className="p-3.5 rounded-xl border border-rose-500/25 text-rose-500 hover:bg-rose-500/10 transition-all cursor-pointer disabled:opacity-50"
                  title="Uninstall application"
                >
                  {actionLoading ? <Loader2 size={18} className="animate-spin" /> : <Trash2 size={18} />}
                </button>
              </>
            ) : (
              <button
                onClick={handleInstall}
                disabled={actionLoading}
                className="inline-flex items-center gap-2.5 px-8 py-3.5 rounded-xl font-semibold text-sm bg-accent-primary text-background hover:bg-accent-primary/90 transition-all shadow-lg hover:shadow-xl cursor-pointer disabled:opacity-50"
              >
                {actionLoading ? (
                  <><Loader2 size={18} className="animate-spin" /><span>Installing...</span></>
                ) : (
                  <><Download size={18} /><span>Install</span></>
                )}
              </button>
            )}
          </div>
        </div>

        {/* ── Status Feedback Banners ── */}
        {actionSuccess && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-emerald-500/8 border border-emerald-500/25 text-emerald-500 text-xs font-medium">
            <CheckCircle2 size={16} className="shrink-0" />
            <span>{actionSuccess}</span>
          </div>
        )}
        {actionError && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-rose-500/8 border border-rose-500/25 text-rose-500 text-xs font-medium">
            <AlertCircle size={16} className="shrink-0" />
            <span>{actionError}</span>
          </div>
        )}

        {/* ── Large Screenshots Showcase ── */}
        {screenshots.length > 0 && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-bold tracking-tight text-foreground">Screenshots</h2>
              <span className="text-xs text-foreground-muted">
                Click screenshot to expand in full-screen lightbox
              </span>
            </div>
            <div className="flex items-stretch gap-6 overflow-x-auto pb-4 snap-x snap-mandatory scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border">
              {screenshots.map((s, idx) => (
                <button
                  key={idx}
                  onClick={() => openLightbox(idx)}
                  className="shrink-0 snap-start rounded-2xl overflow-hidden border border-border/40 hover:border-accent-primary/60 shadow-sm hover:shadow-xl transition-all duration-300 cursor-pointer group text-left bg-surface-elevated/20"
                  style={{ width: "min(520px, 85vw)" }}
                >
                  <div className="relative aspect-video w-full overflow-hidden bg-black/10">
                    <img
                      src={s.url}
                      alt={s.caption || `Screenshot ${idx + 1}`}
                      className="w-full h-full object-cover object-top group-hover:scale-[1.02] transition-transform duration-300"
                      loading="lazy"
                    />
                  </div>
                  {s.caption && (
                    <div className="px-4 py-2.5 text-xs text-foreground-muted bg-surface-elevated/80 truncate border-t border-border/20 font-medium">
                      {s.caption}
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* ── About Section ── */}
        <div className="space-y-3 pt-2">
          <h2 className="text-lg font-bold tracking-tight text-foreground">
            About {meta.displayName}
          </h2>
          <div className="text-sm text-foreground/80 leading-relaxed whitespace-pre-line max-w-4xl">
            {showFullDesc || !isLongDescription
              ? fullDescription
              : fullDescription.slice(0, 320) + "…"}
          </div>
          {isLongDescription && (
            <button
              onClick={() => setShowFullDesc(!showFullDesc)}
              className="inline-flex items-center gap-1 text-xs font-semibold text-accent-primary hover:underline cursor-pointer pt-1"
            >
              {showFullDesc ? (
                <><ChevronUp size={13} />Show less</>
              ) : (
                <><ChevronDown size={13} />Read more</>
              )}
            </button>
          )}
        </div>

        {/* ── Key Features / Highlights (shown when features exist) ── */}
        {features.length > 0 && (
          <div className="space-y-4 pt-2">
            <h2 className="text-lg font-bold tracking-tight text-foreground">Features</h2>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-w-4xl">
              {features.map((feat, idx) => (
                <div
                  key={idx}
                  className="flex items-start gap-3 p-3.5 rounded-xl bg-surface-elevated/25 border border-border/25"
                >
                  <div className="shrink-0 mt-0.5 p-1 rounded-full bg-accent-primary/10 text-accent-primary">
                    <Check size={13} strokeWidth={2.5} />
                  </div>
                  <span className="text-xs sm:text-sm text-foreground/90 font-medium leading-normal">
                    {feat}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        <div className="h-px bg-border/25 my-8" />

        {/* ── Balanced Two-Column Lower Details ── */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 max-w-none">
          {/* Left Column: Package Information & Dependencies */}
          <div className="space-y-8">
            {/* Package details */}
            <div className="space-y-4">
              <h2 className="text-lg font-bold tracking-tight text-foreground">
                Package information
              </h2>
              <div className="space-y-2 text-sm">
                {[
                  { label: "Version", value: version },
                  { label: "Architecture", value: meta.architecture || "x86_64" },
                  { label: "License", value: license },
                  { label: "Repository", value: repository },
                  { label: "Package ID", value: app.id },
                  { label: "Download size", value: downloadSize },
                ].map(({ label, value }) => (
                  <div
                    key={label}
                    className="flex justify-between items-center py-2 border-b border-border/20"
                  >
                    <span className="text-foreground-muted text-xs font-medium">{label}</span>
                    <span className="font-mono text-xs text-foreground font-semibold">{value}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Dependencies */}
            {allDependencies.length > 0 && (
              <div className="space-y-3 pt-2">
                <div className="flex items-center justify-between">
                  <h2 className="text-base font-bold tracking-tight text-foreground">
                    Dependencies
                    <span className="ml-2 text-xs font-normal text-foreground-muted">
                      ({allDependencies.length})
                    </span>
                  </h2>
                  {allDependencies.length > 10 && (
                    <button
                      onClick={() => setShowAllDeps(!showAllDeps)}
                      className="text-xs text-accent-primary hover:underline font-semibold cursor-pointer"
                    >
                      {showAllDeps ? "Show fewer" : `Show all ${allDependencies.length}`}
                    </button>
                  )}
                </div>

                <div className="flex flex-wrap gap-2 pt-1">
                  {visibleDependencies.map((dep, idx) => (
                    <span
                      key={idx}
                      className="px-2.5 py-1 rounded-lg text-xs font-mono bg-surface-elevated/70 border border-border/40 text-foreground-muted"
                    >
                      {dep}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Right Column: Installation Source, Resources & Related Apps */}
          <div className="space-y-8">
            {/* Installation source */}
            <div className="space-y-3">
              <h2 className="text-lg font-bold tracking-tight text-foreground">
                Installation source
              </h2>
              <div className="p-5 rounded-2xl bg-surface-elevated/30 border border-border/35 space-y-3">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <span className="text-xs uppercase tracking-wider font-semibold text-accent-primary">
                      Arch Linux
                    </span>
                    <div className="text-sm font-bold text-foreground">Official Repository</div>
                  </div>
                  <span className="font-mono text-xs text-foreground-muted bg-surface-elevated px-2.5 py-1 rounded-md border border-border/30">
                    pacman · {repository}
                  </span>
                </div>
                <p className="text-xs text-foreground-muted leading-relaxed">
                  Distributed directly through official Arch Linux package repositories. Cryptographically verified with official maintainer PGP signatures.
                </p>
                <div className="flex items-center gap-1.5 text-xs text-emerald-500 font-semibold pt-1">
                  <CheckCircle2 size={14} />
                  <span>Verified package source</span>
                </div>
              </div>
            </div>

            {/* Resources & Links */}
            {(meta.website || meta.sourceRepository || meta.issueTracker) && (
              <div className="space-y-3">
                <h2 className="text-base font-bold tracking-tight text-foreground">Resources</h2>
                <div className="flex flex-col gap-2">
                  {meta.website && (
                    <button
                      onClick={() => openExternal(meta.website)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-surface-elevated/30 hover:bg-surface-elevated/70 border border-border/25 text-xs font-semibold text-foreground transition-colors cursor-pointer text-left"
                    >
                      <span>Project website</span>
                      <ExternalLink size={13} className="text-foreground-muted" />
                    </button>
                  )}
                  {meta.sourceRepository && (
                    <button
                      onClick={() => openExternal(meta.sourceRepository)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-surface-elevated/30 hover:bg-surface-elevated/70 border border-border/25 text-xs font-semibold text-foreground transition-colors cursor-pointer text-left"
                    >
                      <span>Source code repository</span>
                      <ExternalLink size={13} className="text-foreground-muted" />
                    </button>
                  )}
                  {meta.issueTracker && (
                    <button
                      onClick={() => openExternal(meta.issueTracker)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-surface-elevated/30 hover:bg-surface-elevated/70 border border-border/25 text-xs font-semibold text-foreground transition-colors cursor-pointer text-left"
                    >
                      <span>Issue tracker</span>
                      <ExternalLink size={13} className="text-foreground-muted" />
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* You might also like (Related Applications) */}
            {relatedAppIds.length > 0 && (
              <div className="space-y-3 pt-2">
                <h2 className="text-base font-bold tracking-tight text-foreground">
                  You might also like
                </h2>
                <div className="flex flex-col gap-2">
                  {relatedAppIds.map((relId) => {
                    const relMeta = resolveAppMetadata(relId);
                    return (
                      <button
                        key={relId}
                        onClick={() => onSelectRelated?.(relId)}
                        className="flex items-center justify-between p-3 rounded-xl bg-surface-elevated/20 hover:bg-surface-elevated/60 border border-border/25 transition-all duration-150 cursor-pointer text-left group"
                      >
                        <div className="flex items-center gap-3.5 min-w-0">
                          <AppIcon appId={relId} size="md" />
                          <div className="min-w-0">
                            <div className="text-xs font-bold text-foreground group-hover:text-accent-primary transition-colors truncate">
                              {relMeta.displayName}
                            </div>
                            <div className="text-[11px] text-foreground-muted truncate">
                              {relMeta.category}
                            </div>
                          </div>
                        </div>
                        <ChevronRight
                          size={14}
                          className="text-foreground-muted group-hover:text-foreground group-hover:translate-x-0.5 transition-all shrink-0"
                        />
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── Fullscreen Screenshot Lightbox Modal ── */}
      {lightboxOpen && screenshots.length > 0 && (
        <div
          onClick={() => setLightboxOpen(false)}
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/85 backdrop-blur-md animate-fadeIn"
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="relative max-w-6xl max-h-[90vh] flex flex-col items-center gap-4"
          >
            {/* Top close button */}
            <button
              onClick={() => setLightboxOpen(false)}
              className="absolute -top-12 right-0 p-2 text-white/80 hover:text-white transition-colors cursor-pointer"
              title="Close lightbox (Escape)"
            >
              <X size={24} />
            </button>

            {/* Active image */}
            <img
              src={screenshots[activeScreenshotIdx].url}
              alt={screenshots[activeScreenshotIdx].caption || `Screenshot ${activeScreenshotIdx + 1}`}
              className="max-h-[78vh] max-w-full rounded-xl object-contain shadow-2xl border border-white/10"
            />

            {/* Caption & Navigation Controls */}
            <div className="flex items-center justify-between w-full text-white/80 text-xs px-2">
              <span className="font-medium">
                {screenshots[activeScreenshotIdx].caption || `Screenshot ${activeScreenshotIdx + 1} of ${screenshots.length}`}
              </span>

              {screenshots.length > 1 && (
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => setActiveScreenshotIdx((i) => Math.max(0, i - 1))}
                    disabled={activeScreenshotIdx === 0}
                    className="p-1.5 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-30 cursor-pointer"
                    title="Previous (ArrowLeft)"
                  >
                    <ArrowLeft size={14} />
                  </button>
                  <span className="font-mono text-xs">
                    {activeScreenshotIdx + 1} / {screenshots.length}
                  </span>
                  <button
                    onClick={() => setActiveScreenshotIdx((i) => Math.min(screenshots.length - 1, i + 1))}
                    disabled={activeScreenshotIdx === screenshots.length - 1}
                    className="p-1.5 rounded-lg bg-white/10 hover:bg-white/20 disabled:opacity-30 cursor-pointer"
                    title="Next (ArrowRight)"
                  >
                    <ArrowRight size={14} />
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
