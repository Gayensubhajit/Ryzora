/**
 * AppDetailPage — Phase 23B
 *
 * Premium application detail view for Ryzora's software center.
 * Design philosophy: strong typography, whitespace over borders,
 * real 112px system icons, full-width screenshot gallery, lightbox.
 *
 * Navigation:
 *   - ← Back to Applications: always visible top-left
 *   - Alt+Left: back keyboard shortcut
 *   - Escape: close screenshot lightbox
 */

import React, { useState, useMemo, useEffect, useCallback } from "react";
import {
  ArrowLeft,
  ShieldCheck,
  Download,
  Trash2,
  ExternalLink,
  ChevronDown,
  ChevronUp,
  ChevronLeft,
  ChevronRight,
  CheckCircle2,
  AlertCircle,
  Loader2,
  X,
} from "lucide-react";
import type { PackageItem } from "../../types/index.ts";
import { pacmanAppProvider } from "../../providers/pacmanProvider.ts";
import { AppIcon, resolveAppMetadata } from "./AppIconResolver.tsx";

interface AppDetailPageProps {
  app: PackageItem;
  onBack: () => void;
  onStatusChanged?: () => void;
  onSelectRelated?: (pkgId: string) => void;
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

  const version = app.version || "Unknown";
  const repository = pacmanMeta?.repository || "extra";
  const license = pacmanMeta?.license || "Open Source";
  const rawSizeBytes = pacmanMeta?.sizeBytes;
  const downloadSize = rawSizeBytes
    ? rawSizeBytes < 1024 * 1024
      ? `${(rawSizeBytes / 1024).toFixed(0)} KB`
      : `${(rawSizeBytes / (1024 * 1024)).toFixed(1)} MB`
    : "Unknown";

  const allDependencies = pacmanMeta?.dependencies || app.dependencies?.packages || [];
  const visibleDependencies = showAllDeps ? allDependencies : allDependencies.slice(0, 8);
  const screenshots = meta.screenshots || [];
  const relatedAppIds = meta.relatedApps || [];

  // Alt+Left keyboard navigation
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.altKey && e.key === "ArrowLeft") {
        e.preventDefault();
        onBack();
      }
      if (lightboxOpen) {
        if (e.key === "Escape") {
          setLightboxOpen(false);
        } else if (e.key === "ArrowLeft") {
          setActiveScreenshotIdx((i) => Math.max(0, i - 1));
        } else if (e.key === "ArrowRight") {
          setActiveScreenshotIdx((i) => Math.min(screenshots.length - 1, i + 1));
        }
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

  return (
    <div className="relative flex flex-col flex-1 h-full w-full overflow-y-auto bg-background text-foreground animate-fadeIn">
      {/* Sticky back navigation bar */}
      <div className="sticky top-0 z-20 flex items-center justify-between px-8 py-3 bg-background/90 backdrop-blur-sm border-b border-border/30">
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
        {/* ── Hero: Icon + Title + Action ── */}
        <div className="flex flex-col md:flex-row items-start md:items-center gap-7">
          {/* Large authentic icon */}
          <div className="shrink-0">
            <AppIcon appId={app.id} size="2xl" />
          </div>

          {/* Title block */}
          <div className="flex-1 min-w-0 space-y-2">
            <div className="flex items-center gap-3 flex-wrap">
              <h1 className="text-4xl font-bold tracking-tight text-foreground">
                {meta.displayName}
              </h1>
              {isInstalled && (
                <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-500 border border-emerald-500/25">
                  <CheckCircle2 size={12} />
                  Installed
                </span>
              )}
            </div>

            <div className="flex items-center gap-2.5 text-sm text-foreground-muted flex-wrap">
              <span className="font-medium text-foreground/80">{meta.publisher}</span>
              <span className="opacity-40">·</span>
              <span className="text-foreground-muted">{meta.category}</span>
              <span className="opacity-40">·</span>
              <span className="inline-flex items-center gap-1 text-xs text-accent-primary">
                <ShieldCheck size={13} />
                Official Arch Repository
              </span>
            </div>

            <p className="text-sm text-foreground-muted leading-relaxed max-w-2xl">
              {meta.summary}
            </p>
          </div>

          {/* Primary action */}
          <div className="flex items-center gap-3 shrink-0 self-start md:self-auto">
            {isInstalled ? (
              <>
                <button
                  onClick={() => setActionSuccess("Application is ready to launch from your desktop.")}
                  className="px-7 py-3 rounded-xl font-semibold text-sm bg-accent-primary text-background hover:bg-accent-primary/90 transition-all shadow-md cursor-pointer"
                >
                  Open
                </button>
                <button
                  onClick={handleUninstall}
                  disabled={actionLoading}
                  className="p-3 rounded-xl border border-rose-500/25 text-rose-500 hover:bg-rose-500/10 transition-all cursor-pointer disabled:opacity-50"
                  title="Uninstall"
                >
                  {actionLoading ? <Loader2 size={18} className="animate-spin" /> : <Trash2 size={18} />}
                </button>
              </>
            ) : (
              <button
                onClick={handleInstall}
                disabled={actionLoading}
                className="inline-flex items-center gap-2.5 px-8 py-3.5 rounded-xl font-semibold text-sm bg-accent-primary text-background hover:bg-accent-primary/90 transition-all shadow-lg cursor-pointer disabled:opacity-50"
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

        {/* ── Feedback banners ── */}
        {actionSuccess && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-emerald-500/8 border border-emerald-500/25 text-emerald-500 text-xs font-medium">
            <CheckCircle2 size={15} className="shrink-0" />
            <span>{actionSuccess}</span>
          </div>
        )}
        {actionError && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-rose-500/8 border border-rose-500/25 text-rose-500 text-xs font-medium">
            <AlertCircle size={15} className="shrink-0" />
            <span>{actionError}</span>
          </div>
        )}

        {/* ── Screenshot gallery ── */}
        {screenshots.length > 0 && (
          <div className="space-y-3">
            <h2 className="text-base font-semibold text-foreground">Screenshots</h2>
            <div className="flex items-stretch gap-4 overflow-x-auto pb-3 snap-x snap-mandatory scrollbar-thin scrollbar-track-transparent scrollbar-thumb-border">
              {screenshots.map((s, idx) => (
                <button
                  key={idx}
                  onClick={() => openLightbox(idx)}
                  className="shrink-0 snap-start rounded-xl overflow-hidden border border-border/40 hover:border-accent-primary/50 shadow-sm hover:shadow-lg transition-all duration-200 cursor-pointer group"
                  style={{ width: "min(480px, 80vw)" }}
                >
                  <img
                    src={s.url}
                    alt={s.caption || `Screenshot ${idx + 1}`}
                    className="w-full h-60 object-cover object-top group-hover:scale-[1.02] transition-transform duration-300"
                    loading="lazy"
                  />
                  {s.caption && (
                    <div className="px-3 py-2 text-xs text-foreground-muted bg-surface-elevated/80 truncate text-center">
                      {s.caption}
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* ── Main content: 2/3 + 1/3 layout ── */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-10">
          {/* Left: About + Dependencies */}
          <div className="lg:col-span-2 space-y-8">
            {/* About */}
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-foreground">About this app</h2>
              <div className="text-sm text-foreground-muted leading-relaxed whitespace-pre-line">
                {showFullDesc || !meta.fullDescription
                  ? meta.fullDescription || meta.summary
                  : meta.fullDescription.slice(0, 280) + "…"}
              </div>
              {meta.fullDescription && meta.fullDescription.length > 280 && (
                <button
                  onClick={() => setShowFullDesc(!showFullDesc)}
                  className="inline-flex items-center gap-1 text-xs font-semibold text-accent-primary hover:underline cursor-pointer"
                >
                  {showFullDesc ? (
                    <><ChevronUp size={13} />Show less</>
                  ) : (
                    <><ChevronDown size={13} />Read more</>
                  )}
                </button>
              )}
            </div>

            <div className="h-px bg-border/25" />

            {/* Technical metadata — clean key/value rows, no card borders */}
            <div className="space-y-3">
              <h2 className="text-base font-semibold text-foreground">Package details</h2>
              <div className="grid grid-cols-2 gap-x-8 gap-y-2.5 text-sm">
                {[
                  { label: "Version", value: version },
                  { label: "Download size", value: downloadSize },
                  { label: "License", value: license },
                  { label: "Repository", value: repository },
                  { label: "Architecture", value: meta.architecture || "x86_64" },
                  { label: "Package ID", value: app.id },
                ].map(({ label, value }) => (
                  <div key={label} className="flex justify-between items-center py-1 border-b border-border/20">
                    <span className="text-foreground-muted text-xs">{label}</span>
                    <span className="font-mono text-xs text-foreground">{value}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Dependencies */}
            {allDependencies.length > 0 && (
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <h2 className="text-base font-semibold text-foreground">
                    Dependencies
                    <span className="ml-2 text-xs font-normal text-foreground-muted">
                      ({allDependencies.length})
                    </span>
                  </h2>
                  {allDependencies.length > 8 && (
                    <button
                      onClick={() => setShowAllDeps(!showAllDeps)}
                      className="inline-flex items-center gap-1 text-xs text-accent-primary hover:underline cursor-pointer"
                    >
                      {showAllDeps ? "Collapse" : `Show all ${allDependencies.length}`}
                      {showAllDeps ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
                    </button>
                  )}
                </div>
                <div className="flex flex-wrap gap-2">
                  {visibleDependencies.map((dep, i) => (
                    <span
                      key={i}
                      className="px-2.5 py-1 rounded-lg text-xs font-mono bg-surface-elevated/60 border border-border/40 text-foreground-muted"
                    >
                      {dep}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Right sidebar: Source + Resources + Related */}
          <div className="space-y-8">
            {/* Installation source */}
            <div className="space-y-3">
              <h3 className="text-sm font-semibold text-foreground">Installation source</h3>
              <div className="flex items-center gap-3 py-3 border-b border-border/25">
                <div className="w-9 h-9 rounded-lg bg-accent-primary/12 text-accent-primary flex items-center justify-center font-bold text-[10px] shrink-0">
                  PAC
                </div>
                <div className="min-w-0">
                  <div className="text-xs font-semibold text-foreground">Official Arch Repository</div>
                  <div className="text-[11px] text-foreground-muted">pacman · {repository}</div>
                </div>
                <ShieldCheck size={15} className="text-accent-primary shrink-0 ml-auto" />
              </div>
            </div>

            {/* Resources */}
            {(meta.website || meta.sourceRepository) && (
              <div className="space-y-3">
                <h3 className="text-sm font-semibold text-foreground">Resources</h3>
                <div className="space-y-2">
                  {meta.website && (
                    <button
                      onClick={() => openExternal(meta.website)}
                      className="w-full flex items-center justify-between text-xs py-2 border-b border-border/20 text-foreground-muted hover:text-accent-primary transition-colors cursor-pointer group"
                    >
                      <span>Project website</span>
                      <ExternalLink size={12} className="group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
                    </button>
                  )}
                  {meta.sourceRepository && (
                    <button
                      onClick={() => openExternal(meta.sourceRepository)}
                      className="w-full flex items-center justify-between text-xs py-2 border-b border-border/20 text-foreground-muted hover:text-accent-primary transition-colors cursor-pointer group"
                    >
                      <span>Source code</span>
                      <ExternalLink size={12} className="group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
                    </button>
                  )}
                  {meta.issueTracker && (
                    <button
                      onClick={() => openExternal(meta.issueTracker)}
                      className="w-full flex items-center justify-between text-xs py-2 border-b border-border/20 text-foreground-muted hover:text-accent-primary transition-colors cursor-pointer group"
                    >
                      <span>Issue tracker</span>
                      <ExternalLink size={12} className="group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-transform" />
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* Related apps */}
            {relatedAppIds.length > 0 && (
              <div className="space-y-3">
                <h3 className="text-sm font-semibold text-foreground">You might also like</h3>
                <div className="space-y-2.5">
                  {relatedAppIds.slice(0, 5).map((relatedId) => {
                    const relMeta = resolveAppMetadata(relatedId);
                    return (
                      <button
                        key={relatedId}
                        onClick={() => onSelectRelated?.(relatedId)}
                        className="w-full flex items-center gap-3 py-2 text-left hover:text-accent-primary transition-colors cursor-pointer group"
                      >
                        <AppIcon appId={relatedId} size="sm" />
                        <div className="min-w-0">
                          <div className="text-xs font-medium text-foreground group-hover:text-accent-primary truncate">
                            {relMeta.displayName}
                          </div>
                          <div className="text-[10px] text-foreground-muted truncate">
                            {relMeta.category}
                          </div>
                        </div>
                        <ChevronRight size={13} className="ml-auto text-foreground-muted group-hover:text-accent-primary shrink-0" />
                      </button>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── Screenshot lightbox ── */}
      {lightboxOpen && screenshots.length > 0 && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/85 backdrop-blur-md"
          onClick={() => setLightboxOpen(false)}
        >
          {/* Close button */}
          <button
            onClick={() => setLightboxOpen(false)}
            className="absolute top-4 right-4 p-2 rounded-full bg-white/10 hover:bg-white/20 text-white transition-colors cursor-pointer"
          >
            <X size={20} />
          </button>

          {/* Prev */}
          {activeScreenshotIdx > 0 && (
            <button
              onClick={(e) => { e.stopPropagation(); setActiveScreenshotIdx((i) => i - 1); }}
              className="absolute left-4 p-3 rounded-full bg-white/10 hover:bg-white/20 text-white transition-colors cursor-pointer"
            >
              <ChevronLeft size={22} />
            </button>
          )}

          {/* Image */}
          <div
            className="max-w-5xl w-full mx-16 flex flex-col items-center"
            onClick={(e) => e.stopPropagation()}
          >
            <img
              src={screenshots[activeScreenshotIdx]?.url}
              alt={screenshots[activeScreenshotIdx]?.caption || `Screenshot ${activeScreenshotIdx + 1}`}
              className="w-full max-h-[80vh] object-contain rounded-xl shadow-2xl"
            />
            {screenshots[activeScreenshotIdx]?.caption && (
              <p className="mt-3 text-sm text-white/70 text-center">
                {screenshots[activeScreenshotIdx].caption}
              </p>
            )}
            <p className="mt-1 text-xs text-white/40">
              {activeScreenshotIdx + 1} / {screenshots.length}
            </p>
          </div>

          {/* Next */}
          {activeScreenshotIdx < screenshots.length - 1 && (
            <button
              onClick={(e) => { e.stopPropagation(); setActiveScreenshotIdx((i) => i + 1); }}
              className="absolute right-4 p-3 rounded-full bg-white/10 hover:bg-white/20 text-white transition-colors cursor-pointer"
            >
              <ChevronRight size={22} />
            </button>
          )}
        </div>
      )}
    </div>
  );
};
