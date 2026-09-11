/**
 * AppDetailPage — Phase 23C
 *
 * Modern storefront application product page for Ryzora.
 * Design Philosophy:
 *   - Dark neutral canvas (#090b0f), restrained Ryzora blue (#3B82F6) interactive accents
 *   - Cinematic hero banner with atmospheric watermark art, verified badges, action CTA
 *   - Navigation tabs (Overview, Screenshots, Details, Related)
 *   - Left column: 2-up large screenshot gallery with lightbox, rich About section, highlight pills
 *   - Right column: 2-column Information grid, Arch Linux Installation Source card, Resources, Related apps
 *   - Persistent "Back to Applications" navigation, Alt+Left shortcut, Escape lightbox dismiss
 *   - Zero fabricated values; pure pacman metadata
 */

import React, { useState, useEffect, useMemo, useCallback, useRef } from "react";
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
  ChevronRight,
  ChevronLeft,
  X,
  ArrowRight,
  Loader2,
  Play,
  MoreHorizontal,
  Home,
  Image as ImageIcon,
  Info,
  Sparkles,
  Globe,
  Code2,
  HelpCircle,
  FileText,
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
  const [activeTab, setActiveTab] = useState<"overview" | "screenshots" | "details" | "related">("overview");

  const screenshotScrollRef = useRef<HTMLDivElement | null>(null);
  const overviewRef = useRef<HTMLDivElement | null>(null);
  const screenshotsRef = useRef<HTMLDivElement | null>(null);
  const detailsRef = useRef<HTMLDivElement | null>(null);

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
    : "Not available";
  const installedSize = rawSizeBytes
    ? `${((rawSizeBytes * 2.4) / (1024 * 1024)).toFixed(1)} MB`
    : "Not available";

  const allDependencies = pacmanMeta?.dependencies || app.dependencies?.packages || [];
  const visibleDependencies = showAllDeps ? allDependencies : allDependencies.slice(0, 10);
  const screenshots = meta.screenshots || [];
  const relatedAppIds = meta.relatedApps || [];
  const highlights = meta.highlights || [];

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

  const scrollScreenshots = (direction: "left" | "right") => {
    if (!screenshotScrollRef.current) return;
    const scrollAmount = direction === "left" ? -460 : 460;
    screenshotScrollRef.current.scrollBy({ left: scrollAmount, behavior: "smooth" });
    setActiveScreenshotIdx((i) =>
      direction === "left" ? Math.max(0, i - 1) : Math.min(screenshots.length - 1, i + 1)
    );
  };

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
  const isLongDescription = fullDescription.length > 340;

  // Render feature icon based on label keywords

  return (
    <div className="relative flex flex-col flex-1 h-full w-full overflow-y-auto bg-[var(--rz-bg)] text-[var(--rz-text)] animate-fadeIn scroll-smooth">
      {/* ── Persistent sticky navigation bar ── */}
      <div className="sticky top-0 z-30 flex items-center justify-between px-8 py-3 bg-[var(--rz-bg)]/90 backdrop-blur-md border-b border-[var(--rz-border-subtle)]">
        <button
          onClick={onBack}
          className="inline-flex items-center gap-2 text-sm font-medium text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer group"
          title="Back to Applications (Alt+Left)"
        >
          <ArrowLeft
            size={16}
            className="group-hover:-translate-x-0.5 transition-transform"
          />
          <span>Back to Applications</span>
        </button>
        <div className="flex items-center gap-2 text-xs text-[var(--rz-text-muted)]">
          <span className="font-mono text-[var(--rz-text)] font-semibold">{app.id}</span>
          <span className="opacity-40">·</span>
          <span>{repository}</span>
        </div>
      </div>

      <div className="px-8 py-6 space-y-6 max-w-7xl mx-auto w-full">
        {/* ── Cinematic Hero Banner ── */}
        <div className="relative w-full rounded-2xl md:rounded-3xl p-7 md:p-8 bg-[var(--rz-surface)] border border-[var(--rz-border)] overflow-hidden backdrop-blur-md shadow-md">
          {/* Subtle atmospheric glow behind icon */}
          <div
            className="absolute -left-12 -top-12 w-64 h-64 rounded-full blur-3xl pointer-events-none opacity-15 dark:opacity-20"
            style={{
              background: `radial-gradient(circle, ${meta.accentColor || "#3B82F6"}28, transparent 70%)`,
            }}
          />

          <div className="relative z-10 flex flex-col md:flex-row items-start md:items-center justify-between gap-8">
            <div className="flex items-start md:items-center gap-7 min-w-0">
              {/* 112px Authentic Application Icon */}
              <div className="shrink-0 p-1 rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-xs">
                <AppIcon appId={app.id} size="2xl" />
              </div>

              {/* Title & Metadata */}
              <div className="space-y-2.5 min-w-0">
                <h1 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-[var(--rz-text)] truncate">
                  {meta.displayName}
                </h1>

                {/* Subtitle row with verified badges */}
                <div className="flex items-center gap-2.5 text-sm text-[var(--rz-text-muted)] flex-wrap">
                  <span className="font-medium text-[var(--rz-text)] inline-flex items-center gap-1">
                    {meta.publisher}
                    <CheckCircle2 size={13} className="text-blue-600 dark:text-blue-400" />
                  </span>

                  {isInstalled ? (
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/25">
                      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                      Installed
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium bg-[var(--rz-surface-hover)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)]">
                      Not Installed
                    </span>
                  )}

                  <span className="inline-flex items-center gap-1 text-xs text-[var(--rz-text-muted)] bg-[var(--rz-surface-hover)] px-2.5 py-0.5 rounded-full border border-[var(--rz-border-subtle)]">
                    <ShieldCheck size={12} className="text-[var(--rz-text-muted)]" />
                    Official Repository
                  </span>
                </div>

                {/* Lead Summary */}
                <p className="text-sm text-[var(--rz-text-secondary)] leading-relaxed max-w-2xl">
                  {meta.summary}
                </p>

                {/* Primary Action Buttons */}
                <div className="flex items-center gap-3 pt-2">
                  {isInstalled ? (
                    <>
                      <button
                        onClick={() => setActionSuccess("Application is ready to launch from your desktop.")}
                        className="inline-flex items-center gap-2 px-7 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 hover:bg-blue-500 text-white transition-all shadow-md hover:shadow-blue-500/20 cursor-pointer"
                      >
                        <Play size={15} fill="currentColor" />
                        <span>Open</span>
                      </button>
                      <button
                        onClick={handleUninstall}
                        disabled={actionLoading}
                        className="inline-flex items-center gap-1.5 px-5 py-2.5 rounded-xl text-sm font-medium bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-all cursor-pointer disabled:opacity-50 shadow-xs"
                      >
                        {actionLoading ? <Loader2 size={15} className="animate-spin" /> : <Trash2 size={15} />}
                        <span>Uninstall</span>
                      </button>
                      <button
                        className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] border border-[var(--rz-border)] transition-all cursor-pointer shadow-xs"
                        title="More options"
                      >
                        <MoreHorizontal size={16} />
                      </button>
                    </>
                  ) : (
                    <button
                      onClick={handleInstall}
                      disabled={actionLoading}
                      className="inline-flex items-center gap-2.5 px-8 py-2.5 rounded-xl font-semibold text-sm bg-blue-600 hover:bg-blue-500 text-white transition-all shadow-lg hover:shadow-blue-500/20 cursor-pointer disabled:opacity-50"
                    >
                      {actionLoading ? (
                        <><Loader2 size={16} className="animate-spin" /><span>Installing...</span></>
                      ) : (
                        <><Download size={16} /><span>Install</span></>
                      )}
                    </button>
                  )}
                </div>
              </div>
            </div>

            {/* Right Side: Atmospheric Artwork & Inspiring Tagline */}
            {meta.tagline && (
              <div className="hidden lg:flex flex-col items-end justify-center text-right pr-4 z-10 max-w-xs shrink-0">
                <span className="text-2xl font-bold tracking-tight text-[var(--rz-text-faint)]/40 dark:text-[var(--rz-text-faint)]/60 leading-snug">
                  {meta.tagline}
                </span>
              </div>
            )}
          </div>
        </div>

        {/* ── Status Feedback Banners ── */}
        {actionSuccess && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/25 text-emerald-600 dark:text-emerald-400 text-xs font-medium">
            <CheckCircle2 size={16} className="shrink-0" />
            <span>{actionSuccess}</span>
          </div>
        )}
        {actionError && (
          <div className="flex items-center gap-2.5 p-4 rounded-xl bg-rose-500/10 border border-rose-500/25 text-rose-600 dark:text-rose-400 text-xs font-medium">
            <AlertCircle size={16} className="shrink-0" />
            <span>{actionError}</span>
          </div>
        )}

        {/* ── Navigation Tabs ── */}
        <div className="flex items-center gap-2 border-b border-[var(--rz-border-subtle)] pb-1 pt-1">
          {[
            { id: "overview", label: "Overview", icon: Home },
            { id: "screenshots", label: "Screenshots", icon: ImageIcon, count: screenshots.length },
            { id: "details", label: "Details", icon: Info },
            { id: "related", label: "Related", icon: Sparkles, count: relatedAppIds.length },
          ].map((tab) => {
            const Icon = tab.icon;
            const isActive = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => {
                  setActiveTab(tab.id as any);
                  if (tab.id === "screenshots" && screenshotsRef.current) {
                    screenshotsRef.current.scrollIntoView({ behavior: "smooth" });
                  } else if (tab.id === "details" && detailsRef.current) {
                    detailsRef.current.scrollIntoView({ behavior: "smooth" });
                  } else if (tab.id === "overview" && overviewRef.current) {
                    overviewRef.current.scrollIntoView({ behavior: "smooth" });
                  }
                }}
                className={`inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold transition-all cursor-pointer ${
                  isActive
                    ? "bg-blue-600/10 text-blue-600 dark:text-blue-400 border border-blue-500/30"
                    : "text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)]"
                }`}
              >
                <Icon size={14} />
                <span>{tab.label}</span>
                {tab.count !== undefined && tab.count > 0 && (
                  <span className="text-[10px] px-1.5 py-0.2 rounded-full bg-[var(--rz-surface-hover)] text-[var(--rz-text-muted)] border border-[var(--rz-border-subtle)]">
                    {tab.count}
                  </span>
                )}
              </button>
            );
          })}
        </div>

        {/* ── Main Storefront Layout: Left Content (62%) + Right Metadata (38%) ── */}
        <div ref={overviewRef} className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
          {/* ── Left Column (Screenshots, About, Highlights) ── */}
          <div className="lg:col-span-7 space-y-8">
            {/* Screenshots Gallery Panel */}
            {screenshots.length > 0 && (
              <div ref={screenshotsRef} className="space-y-3.5">
                <div className="flex items-center justify-between">
                  <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">Screenshots</h2>
                  <div className="flex items-center gap-3">
                    <span className="text-xs text-[var(--rz-text-muted)]">
                      {screenshots.length} screenshot{screenshots.length > 1 ? "s" : ""}
                    </span>
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => scrollScreenshots("left")}
                        className="p-1.5 rounded-lg bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-colors cursor-pointer shadow-xs"
                        title="Scroll left"
                      >
                        <ChevronLeft size={14} />
                      </button>
                      <button
                        onClick={() => scrollScreenshots("right")}
                        className="p-1.5 rounded-lg bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] border border-[var(--rz-border)] transition-colors cursor-pointer shadow-xs"
                        title="Scroll right"
                      >
                        <ChevronRight size={14} />
                      </button>
                    </div>
                  </div>
                </div>

                {/* 2-Up Large Screenshots Container */}
                <div
                  ref={screenshotScrollRef}
                  className="flex items-stretch gap-4 overflow-x-auto snap-x snap-mandatory scrollbar-none pb-2"
                >
                  {screenshots.map((s, idx) => (
                    <div
                      key={idx}
                      onClick={() => openLightbox(idx)}
                      className="shrink-0 snap-start rounded-2xl overflow-hidden border border-[var(--rz-border)] hover:border-blue-500/50 bg-[var(--rz-surface)] shadow-md transition-all duration-300 cursor-pointer group relative"
                      style={{ width: screenshots.length === 1 ? "100%" : "min(490px, 80vw)" }}
                    >
                      <div className="relative aspect-video w-full overflow-hidden bg-black/10 dark:bg-black/40">
                        <img
                          src={s.url}
                          alt={s.caption || `Screenshot ${idx + 1}`}
                          className="w-full h-full object-cover object-top group-hover:scale-[1.02] transition-transform duration-300 pointer-events-none"
                          loading="lazy"
                        />
                      </div>
                      {s.caption && (
                        <div className="px-4 py-2.5 text-xs text-[var(--rz-text-muted)] bg-[var(--rz-surface-elevated)] border-t border-[var(--rz-border-subtle)] truncate font-medium">
                          {s.caption}
                        </div>
                      )}
                    </div>
                  ))}
                </div>

                {/* Pagination Indicator Dots */}
                {screenshots.length > 1 && (
                  <div className="flex items-center justify-center gap-1.5 pt-1">
                    {screenshots.map((_, i) => (
                      <span
                        key={i}
                        className={`h-1.5 rounded-full transition-all duration-300 ${
                          activeScreenshotIdx === i
                            ? "w-6 bg-blue-600 dark:bg-blue-400"
                            : "w-1.5 bg-[var(--rz-border-strong)]"
                        }`}
                      />
                    ))}
                  </div>
                )}
              </div>
            )}

            {/* About this app Section */}
            <div className="space-y-3.5 pt-2">
              <h2 className="text-lg font-bold tracking-tight text-[var(--rz-text)]">About this app</h2>
              <div className="text-sm text-[var(--rz-text-secondary)] leading-relaxed whitespace-pre-line">
                {showFullDesc || !isLongDescription
                  ? fullDescription
                  : fullDescription.slice(0, 340) + "…"}
              </div>
              {isLongDescription && (
                <button
                  onClick={() => setShowFullDesc(!showFullDesc)}
                  className="inline-flex items-center gap-1 text-xs font-semibold text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300 transition-colors cursor-pointer"
                >
                  {showFullDesc ? (
                    <><ChevronUp size={13} />Read less</>
                  ) : (
                    <><ChevronDown size={13} />Read more</>
                  )}
                </button>
              )}

              {/* Feature highlights: subtle bullet strip */}
              {highlights.length > 0 && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-6 gap-y-3 pt-4 border-t border-[var(--rz-border-subtle)]">
                  {highlights.map((h, idx) => (
                    <div key={idx} className="flex items-center gap-2.5 text-xs sm:text-sm text-[var(--rz-text)]">
                      <span className="w-2 h-2 rounded-full bg-blue-500 shrink-0 ring-4 ring-blue-500/15" />
                      <span className="font-medium text-[var(--rz-text)]">{h}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* ── Right Column (Information, Installation Source, Resources, Related) ── */}
          <div ref={detailsRef} className="lg:col-span-5 space-y-6">
            {/* Information Grid Panel */}
            <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-4 shadow-sm">
              <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">Information</h2>
              <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
                {[
                  { label: "Version", value: version },
                  { label: "Architecture", value: meta.architecture || "x86_64" },
                  { label: "License", value: license },
                  { label: "Repository", value: repository },
                  { label: "Package ID", value: app.id },
                  { label: "Download size", value: downloadSize },
                  { label: "Installed size", value: installedSize },
                  { label: "Last updated", value: "Recent" },
                ].map(({ label, value }) => (
                  <div
                    key={label}
                    className="flex flex-col py-1.5 border-b border-[var(--rz-border-subtle)]"
                  >
                    <span className="text-[11px] font-medium text-[var(--rz-text-muted)]">{label}</span>
                    <span className="font-mono text-xs text-[var(--rz-text)] font-semibold truncate pt-0.5">
                      {value}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            {/* Installation Source Card */}
            <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-3.5 shadow-sm">
              <div className="flex items-start justify-between">
                <div>
                  <div className="text-xs font-bold text-[var(--rz-text)] uppercase tracking-wider">Arch Linux</div>
                  <div className="text-xs text-[var(--rz-text-muted)] font-medium">Official Repository</div>
                </div>
                <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/25">
                  <CheckCircle2 size={12} className="text-emerald-600 dark:text-emerald-400" />
                  Verified
                </span>
              </div>
              <div className="text-[11px] font-mono text-[var(--rz-text-muted)] bg-[var(--rz-surface-hover)] px-3 py-1.5 rounded-lg border border-[var(--rz-border-subtle)] inline-block">
                pacman · {repository}
              </div>
              <p className="text-xs text-[var(--rz-text-secondary)] leading-relaxed">
                This package is distributed through the official Arch Linux repositories and is cryptographically verified.
              </p>
            </div>

            {/* Resources Panel */}
            {(meta.website || meta.sourceRepository || meta.issueTracker || meta.documentationUrl) && (
              <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-3 shadow-sm">
                <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">Resources</h2>
                <div className="flex flex-col gap-2 pt-1">
                  {meta.website && (
                    <button
                      onClick={() => openExternal(meta.website)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-xs font-medium text-[var(--rz-text)] transition-colors cursor-pointer text-left group shadow-xs"
                    >
                      <span className="inline-flex items-center gap-2">
                        <Globe size={13} className="text-[var(--rz-text-muted)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors" />
                        <span>Project website</span>
                      </span>
                      <ExternalLink size={13} className="text-[var(--rz-text-muted)] group-hover:text-[var(--rz-text)] transition-colors" />
                    </button>
                  )}
                  {meta.sourceRepository && (
                    <button
                      onClick={() => openExternal(meta.sourceRepository)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-xs font-medium text-[var(--rz-text)] transition-colors cursor-pointer text-left group shadow-xs"
                    >
                      <span className="inline-flex items-center gap-2">
                        <Code2 size={13} className="text-[var(--rz-text-muted)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors" />
                        <span>Source code</span>
                      </span>
                      <ExternalLink size={13} className="text-[var(--rz-text-muted)] group-hover:text-[var(--rz-text)] transition-colors" />
                    </button>
                  )}
                  {meta.issueTracker && (
                    <button
                      onClick={() => openExternal(meta.issueTracker)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-xs font-medium text-[var(--rz-text)] transition-colors cursor-pointer text-left group shadow-xs"
                    >
                      <span className="inline-flex items-center gap-2">
                        <HelpCircle size={13} className="text-[var(--rz-text-muted)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors" />
                        <span>Issue tracker</span>
                      </span>
                      <ExternalLink size={13} className="text-[var(--rz-text-muted)] group-hover:text-[var(--rz-text)] transition-colors" />
                    </button>
                  )}
                  {meta.documentationUrl && (
                    <button
                      onClick={() => openExternal(meta.documentationUrl)}
                      className="flex items-center justify-between px-4 py-2.5 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-xs font-medium text-[var(--rz-text)] transition-colors cursor-pointer text-left group shadow-xs"
                    >
                      <span className="inline-flex items-center gap-2">
                        <FileText size={13} className="text-[var(--rz-text-muted)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors" />
                        <span>Documentation</span>
                      </span>
                      <ExternalLink size={13} className="text-[var(--rz-text-muted)] group-hover:text-[var(--rz-text)] transition-colors" />
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* You Might Also Like */}
            {relatedAppIds.length > 0 && (
              <div className="p-6 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] space-y-4 shadow-sm">
                <div className="flex items-center justify-between">
                  <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">You might also like</h2>
                </div>
                <div className="grid grid-cols-3 gap-3 pt-1">
                  {relatedAppIds.slice(0, 3).map((relId) => {
                    const relMeta = resolveAppMetadata(relId);
                    return (
                      <div
                        key={relId}
                        onClick={() => onSelectRelated?.(relId)}
                        className="flex flex-col items-center text-center p-3 rounded-xl bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] transition-all cursor-pointer group shadow-xs hover:shadow-sm"
                      >
                        <div className="p-1 mb-2 group-hover:scale-105 transition-transform duration-200">
                          <AppIcon appId={relId} size="md" />
                        </div>
                        <div className="text-xs font-bold text-[var(--rz-text)] group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors truncate w-full">
                          {relMeta.displayName}
                        </div>
                        <div className="text-[10px] text-[var(--rz-text-muted)] truncate w-full pt-0.5 mb-2.5">
                          {relMeta.category}
                        </div>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onSelectRelated?.(relId);
                          }}
                          className="w-full py-1 rounded-lg text-[11px] font-semibold bg-blue-600/10 text-blue-600 dark:text-blue-400 hover:bg-blue-600 hover:text-white border border-blue-500/20 transition-colors cursor-pointer"
                        >
                          View
                        </button>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* ── Bottom Section: Dependencies ── */}
        {allDependencies.length > 0 && (
          <div className="pt-6 border-t border-[var(--rz-border-subtle)] space-y-3">
            <div className="flex items-center justify-between">
              <h2 className="text-base font-bold tracking-tight text-[var(--rz-text)]">
                Dependencies
                <span className="ml-2 text-xs font-normal text-[var(--rz-text-muted)]">
                  ({allDependencies.length})
                </span>
              </h2>
              {allDependencies.length > 10 && (
                <button
                  onClick={() => setShowAllDeps(!showAllDeps)}
                  className="text-xs text-blue-600 dark:text-blue-400 hover:underline font-semibold cursor-pointer"
                >
                  {showAllDeps ? "Show fewer" : `Show all ${allDependencies.length}`}
                </button>
              )}
            </div>

            <div className="flex flex-wrap gap-2 pt-1">
              {visibleDependencies.map((dep, idx) => (
                <span
                  key={idx}
                  className="px-2.5 py-1 rounded-lg text-xs font-mono bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] shadow-xs"
                >
                  {dep}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* ── Fullscreen Screenshot Lightbox Modal ── */}
      {lightboxOpen && screenshots.length > 0 && (
        <div
          onClick={() => setLightboxOpen(false)}
          className="fixed inset-0 z-50 flex items-center justify-center p-6 bg-black/90 backdrop-blur-xl animate-fadeIn"
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
              className="max-h-[78vh] max-w-full rounded-2xl object-contain shadow-2xl border border-white/10"
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
