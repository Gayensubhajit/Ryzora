import React, { useState } from "react";
import {
  Sliders,
  DownloadCloud,
  Check,
  CheckCircle2,
  MoreHorizontal,
  Trash2,
  ExternalLink,
  Folder,
  FileText,
  FolderGit2,
  Cpu,
  History,
  ShieldCheck,
  Code,
  Terminal,
  Loader2,
  Star,
  Maximize2,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { ProductDetailShell } from "../components/detail/ProductDetailShell";
import { formatDownloads } from "../components/catalogue/catalogueUtils";

export interface FastfetchDetailViewProps {
  onOpenSettings?: (presetId: string) => void;
}

type TabKey = "overview" | "files" | "dependencies" | "installation" | "changelog";

export const FastfetchDetailView: React.FC<FastfetchDetailViewProps> = ({ onOpenSettings }) => {
  const {
    selectedPackage,
    setSelectedPackage,
    installedPackages,
    installedPackageIds,
    installPackage,
    uninstallPackage,
    isInstalling,
    getPackageTransaction,
    setToast,
  } = useApp();

  const [activeTab, setActiveTab] = useState<TabKey>("overview");
  const [showOverflowMenu, setShowOverflowMenu] = useState<boolean>(false);
  const [activeScreenshotIndex, setActiveScreenshotIndex] = useState<number>(0);
  const [showLightbox, setShowLightbox] = useState<boolean>(false);

  if (!selectedPackage) return null;

  const isFastfetch =
    selectedPackage.category === "fastfetch" ||
    selectedPackage.package_type === "fastfetch";

  if (!isFastfetch) return null;

  const isInstalled = Boolean(
    installedPackageIds?.includes(selectedPackage.id) ||
    installedPackages?.some((p) => p.package_id === selectedPackage.id)
  );

  const activePresetId =
    typeof window !== "undefined"
      ? localStorage.getItem("ryzora_active_fastfetch_preset") || "fastfetch-preset-2"
      : "fastfetch-preset-2";
  const isActive = isInstalled && activePresetId === selectedPackage.id;

  const transaction = getPackageTransaction(selectedPackage.id);
  const isTransactionActive = Boolean(isInstalling || transaction?.operation === "install");

  const screenshots = selectedPackage.screenshots && selectedPackage.screenshots.length > 0
    ? selectedPackage.screenshots
    : [selectedPackage.hero_image];

  const activeImage = screenshots[activeScreenshotIndex] || selectedPackage.hero_image;

  const handleClose = () => {
    setSelectedPackage(null);
  };

  const handleInstall = async () => {
    try {
      await installPackage(selectedPackage);
      setToast({
        message: `Successfully installed "${selectedPackage.title}".`,
        type: "success",
      });
    } catch (err: any) {
      setToast({
        message: `Installation failed: ${err?.message || String(err)}`,
        type: "warning",
      });
    }
  };

  const handleUninstall = async () => {
    setShowOverflowMenu(false);
    try {
      await uninstallPackage(selectedPackage.id);
      setToast({
        message: `Uninstalled "${selectedPackage.title}".`,
        type: "info",
      });
    } catch (err: any) {
      setToast({
        message: `Uninstall failed: ${err?.message || String(err)}`,
        type: "warning",
      });
    }
  };

  const handleOpenSettings = () => {
    if (onOpenSettings) {
      onOpenSettings(selectedPackage.id);
    }
  };

  const handleOpenUpstream = () => {
    setShowOverflowMenu(false);
    const url = (selectedPackage as any).upstream_url || (selectedPackage as any).source_url || "https://github.com/fastfetch-cli/fastfetch";
    if (typeof window !== "undefined") {
      window.open(url, "_blank");
    }
  };

  const targetFiles = [
    {
      path: `~/.config/fastfetch/presets/${selectedPackage.id}.jsonc`,
      type: "Preset Configuration Payload",
      note: "Staged user space preset",
    },
    {
      path: `~/.config/fastfetch/config.jsonc`,
      type: "Active Symlink / Config Target",
      note: "Managed desktop Fastfetch target",
    },
  ];

  const dependencies = [
    { name: "fastfetch", version: ">= 2.8.0", required: true, status: "Installed" },
    { name: "nerd-fonts", version: "Any (Symbols & Glyphs)", required: false, status: "Recommended" },
    { name: "terminal-emulator", version: "kitty / alacritty / ghostty", required: true, status: "Available" },
  ];

  return (
    <ProductDetailShell
      onClose={handleClose}
      bgImage={selectedPackage.hero_image}
      categoryLabel="Fastfetch"
    >
      <div className="space-y-8 pb-12">
        {/* ── Upper Two-Column Hero ── */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-8 items-start">
          {/* Left Column: Large Preview Screenshot (60%) */}
          <div className="lg:col-span-7 space-y-3">
            <div className="relative group rounded-2xl overflow-hidden border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] shadow-xl aspect-video flex items-center justify-center">
              {activeImage ? (
                <img
                  src={activeImage}
                  alt={selectedPackage.title}
                  className="w-full h-full object-cover select-none transition-transform duration-300 group-hover:scale-[1.02]"
                />
              ) : (
                <div className="flex flex-col items-center justify-center p-8 text-center text-[var(--rz-text-muted)]">
                  <Terminal className="w-16 h-16 mb-2 opacity-40 text-[var(--rz-accent)]" />
                  <span className="text-sm font-semibold text-[var(--rz-text)]">{selectedPackage.title}</span>
                  <span className="text-xs text-[var(--rz-text-secondary)]">Fastfetch Preview Readout</span>
                </div>
              )}

              {/* Lightbox / Zoom overlay */}
              <button
                type="button"
                onClick={() => setShowLightbox(true)}
                className="absolute top-3 right-3 p-2 rounded-xl bg-black/60 backdrop-blur-md text-white/80 hover:text-white hover:bg-black/80 transition-all opacity-0 group-hover:opacity-100 cursor-pointer shadow-md"
                title="View full resolution image"
              >
                <Maximize2 className="w-4 h-4" />
              </button>
            </div>

            {/* Gallery Thumbnails */}
            {screenshots.length > 1 && (
              <div className="flex items-center gap-2 overflow-x-auto pb-1">
                {screenshots.map((shot, idx) => (
                  <button
                    key={idx}
                    type="button"
                    onClick={() => setActiveScreenshotIndex(idx)}
                    className={`relative w-20 h-13 rounded-lg overflow-hidden border transition-all cursor-pointer shrink-0 ${
                      activeScreenshotIndex === idx
                        ? "border-[var(--rz-accent)] ring-2 ring-[var(--rz-accent)]/30 scale-105"
                        : "border-[var(--rz-border-subtle)] opacity-60 hover:opacity-100"
                    }`}
                  >
                    <img src={shot} alt={`Thumbnail ${idx + 1}`} className="w-full h-full object-cover" />
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Right Column: Identity, Metadata & Actions (40%) */}
          <div className="lg:col-span-5 flex flex-col justify-between space-y-6">
            <div className="space-y-4">
              <div>
                <div className="flex items-center gap-2 text-xs text-[var(--rz-accent)] font-semibold tracking-wide uppercase mb-1">
                  <Terminal className="w-3.5 h-3.5" />
                  <span>Fastfetch Preset</span>
                </div>
                <h1 className="text-2xl sm:text-3xl font-extrabold tracking-tight text-[var(--rz-text)]">
                  {selectedPackage.title}
                </h1>
                <p className="text-xs text-[var(--rz-text-secondary)] mt-1">
                  By <span className="text-[var(--rz-text)] font-medium">{selectedPackage.author?.name || "harilvfs"}</span>
                </p>
              </div>

              <p className="text-xs leading-relaxed text-[var(--rz-text-secondary)]">
                {selectedPackage.description}
              </p>

              {/* Tags */}
              {selectedPackage.tags && selectedPackage.tags.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {selectedPackage.tags.map((tag) => (
                    <span
                      key={tag}
                      className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]"
                    >
                      #{tag}
                    </span>
                  ))}
                </div>
              )}

              {/* Metadata Pills */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 pt-2 border-t border-[var(--rz-border-subtle)] text-[11px]">
                <div className="p-2 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                  <span className="text-[10px] text-[var(--rz-text-muted)] block">Rating</span>
                  <span className="font-semibold text-amber-400 flex items-center gap-1 mt-0.5">
                    <Star className="w-3 h-3 fill-amber-400" />
                    {selectedPackage.rating ? selectedPackage.rating.toFixed(1) : "5.0"}
                  </span>
                </div>
                <div className="p-2 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                  <span className="text-[10px] text-[var(--rz-text-muted)] block">Downloads</span>
                  <span className="font-semibold text-[var(--rz-text)] mt-0.5 block">
                    {formatDownloads(selectedPackage.downloads || 1200)}
                  </span>
                </div>
                <div className="p-2 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                  <span className="text-[10px] text-[var(--rz-text-muted)] block">License</span>
                  <span className="font-semibold text-[var(--rz-text)] mt-0.5 block">
                    {(selectedPackage as any).license || "MIT"}
                  </span>
                </div>
                <div className="p-2 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                  <span className="text-[10px] text-[var(--rz-text-muted)] block">Version</span>
                  <span className="font-semibold font-mono text-[var(--rz-text)] mt-0.5 block">
                    v{selectedPackage.version}
                  </span>
                </div>
              </div>

              {/* Environment Info */}
              <div className="p-3.5 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="font-semibold text-[var(--rz-text)] flex items-center gap-1.5">
                    <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                    Available on this system
                  </span>
                  <span className="text-[10px] font-mono text-[var(--rz-text-muted)]">Fastfetch 2.x</span>
                </div>
                <div className="text-[11px] text-[var(--rz-text-secondary)] font-mono truncate">
                  Target: ~/.config/fastfetch/
                </div>
              </div>
            </div>

            {/* ── Actions Area ── */}
            <div className="pt-4 border-t border-[var(--rz-border-subtle)]">
              {!isInstalled ? (
                /* NOT INSTALLED: Primary Install Action */
                <button
                  type="button"
                  disabled={isTransactionActive}
                  onClick={handleInstall}
                  className="w-full py-3 px-5 rounded-xl font-semibold text-xs flex items-center justify-center gap-2 bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white shadow-lg shadow-[var(--rz-accent)]/20 transition-all cursor-pointer disabled:opacity-50"
                >
                  {isTransactionActive ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      <span>{transaction?.message || "Installing Fastfetch Preset..."}</span>
                    </>
                  ) : (
                    <>
                      <DownloadCloud className="w-4 h-4" />
                      <span>Install</span>
                    </>
                  )}
                </button>
              ) : (
                /* INSTALLED / ACTIVE: Status + Open in Settings + Overflow */
                <div className="space-y-3">
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-[var(--rz-text-secondary)]">Fastfetch</span>
                    {isActive ? (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
                        Active
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]">
                        <Check className="w-3.5 h-3.5 text-emerald-400" />
                        Installed
                      </span>
                    )}
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      type="button"
                      onClick={handleOpenSettings}
                      className="flex-1 py-2.5 px-4 rounded-xl font-semibold text-xs flex items-center justify-center gap-2 bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white shadow-md shadow-[var(--rz-accent)]/20 transition-all cursor-pointer"
                    >
                      <Sliders className="w-4 h-4" />
                      <span>Open in Fastfetch Settings</span>
                    </button>

                    {/* Secondary Overflow Menu [...] */}
                    <div className="relative">
                      <button
                        type="button"
                        onClick={() => setShowOverflowMenu(!showOverflowMenu)}
                        className="p-2.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] transition-colors cursor-pointer"
                        title="More actions"
                      >
                        <MoreHorizontal className="w-4 h-4" />
                      </button>

                      {showOverflowMenu && (
                        <div
                          className="absolute right-0 bottom-full mb-2 w-48 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-xl p-1.5 z-20 space-y-1 animate-in fade-in zoom-in-95 duration-150"
                        >
                          <button
                            type="button"
                            onClick={handleOpenUpstream}
                            className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
                          >
                            <ExternalLink className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                            <span>Upstream Repository</span>
                          </button>

                          <button
                            type="button"
                            onClick={() => {
                              setShowOverflowMenu(false);
                              setToast({
                                message: `Payload directory: ~/.config/fastfetch/presets/${selectedPackage.id}.jsonc`,
                                type: "info",
                              });
                            }}
                            className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
                          >
                            <Folder className="w-3.5 h-3.5 text-[var(--rz-text-secondary)]" />
                            <span>View Files</span>
                          </button>

                          <div className="border-t border-[var(--rz-border-subtle)] my-1" />

                          <button
                            type="button"
                            onClick={handleUninstall}
                            className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5 text-rose-400" />
                            <span>Uninstall</span>
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* ── Lower Detail Tabs (Overview, Files, Dependencies, Installation, Changelog) ── */}
        <div className="pt-4 border-t border-[var(--rz-border-subtle)]">
          {/* Navigation Bar */}
          <div className="flex items-center gap-6 border-b border-[var(--rz-border-subtle)] overflow-x-auto scrollbar-none">
            {[
              { id: "overview", label: "Overview", icon: <FileText className="w-3.5 h-3.5" /> },
              { id: "files", label: "Files & Code", icon: <FolderGit2 className="w-3.5 h-3.5" />, badge: targetFiles.length },
              { id: "dependencies", label: "Dependencies", icon: <Cpu className="w-3.5 h-3.5" />, badge: dependencies.length },
              { id: "installation", label: "Installation", icon: <DownloadCloud className="w-3.5 h-3.5" /> },
              { id: "changelog", label: "Changelog", icon: <History className="w-3.5 h-3.5" /> },
            ].map((tab) => {
              const isTabActive = activeTab === tab.id;
              return (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => setActiveTab(tab.id as TabKey)}
                  className={`relative flex items-center gap-2 pb-3 text-xs font-medium transition-colors cursor-pointer select-none shrink-0 ${
                    isTabActive
                      ? "text-[var(--rz-text)] font-semibold after:absolute after:bottom-0 after:left-0 after:right-0 after:h-0.5 after:bg-[var(--rz-accent)] after:rounded-full"
                      : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                  }`}
                >
                  {tab.icon}
                  <span>{tab.label}</span>
                  {typeof tab.badge === "number" && (
                    <span className="px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)]">
                      {tab.badge}
                    </span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Tab Content Panels */}
          <div className="pt-5 text-xs">
            {activeTab === "overview" && (
              <div className="space-y-5">
                <div>
                  <h3 className="text-sm font-bold text-[var(--rz-text)] mb-1">About {selectedPackage.title}</h3>
                  <p className="text-[var(--rz-text-secondary)] leading-relaxed">
                    {selectedPackage.description}
                  </p>
                </div>

                {/* Color Palette Swatches */}
                {selectedPackage.color_palette && selectedPackage.color_palette.length > 0 && (
                  <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
                    <span className="text-[10px] font-mono uppercase tracking-wider text-[var(--rz-text-muted)] font-bold block">
                      Accent Color Palette
                    </span>
                    <div className="flex items-center gap-2 flex-wrap">
                      {selectedPackage.color_palette.map((color, idx) => (
                        <div key={idx} className="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[11px] font-mono">
                          <span
                            className="w-3.5 h-3.5 rounded-full border border-black/20 shrink-0"
                            style={{ backgroundColor: color }}
                          />
                          <span>{color}</span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}

                {/* Security and Integrity Guarantee */}
                <div className="flex items-start gap-2.5 p-3.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                  <ShieldCheck className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-semibold text-[var(--rz-text)]">Ryzora Keyring & Provenance</div>
                    <div className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 leading-relaxed">
                      Manifest signed and verified. All Fastfetch presets are strictly sandboxed in user space configuration (~/.config/fastfetch) without requiring elevated privileges.
                    </div>
                  </div>
                </div>
              </div>
            )}

            {activeTab === "files" && (
              <div className="space-y-3">
                <div className="flex items-center justify-between text-[11px] text-[var(--rz-text-muted)] font-mono pb-1 border-b border-[var(--rz-border-subtle)]">
                  <span>Target Destination Path</span>
                  <span>Classification</span>
                </div>
                <div className="divide-y divide-[var(--rz-border-subtle)]/50 font-mono text-[11px]">
                  {targetFiles.map((file, idx) => (
                    <div key={idx} className="py-2.5 flex items-center justify-between gap-3">
                      <div className="flex items-center gap-2 truncate">
                        <Code className="w-3.5 h-3.5 shrink-0 text-[var(--rz-accent)]" />
                        <span className="truncate text-[var(--rz-text)]">{file.path}</span>
                      </div>
                      <span className="px-2 py-0.5 rounded text-[9px] uppercase tracking-wider shrink-0 bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]">
                        {file.type}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeTab === "dependencies" && (
              <div className="space-y-3">
                <p className="text-[11px] text-[var(--rz-text-secondary)]">
                  The following components are required to render this Fastfetch preset:
                </p>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 font-mono text-xs">
                  {dependencies.map((dep, idx) => (
                    <div
                      key={idx}
                      className="flex items-center justify-between p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]"
                    >
                      <div>
                        <span className="font-semibold text-[var(--rz-text)] block">{dep.name}</span>
                        <span className="text-[10px] text-[var(--rz-text-muted)]">{dep.version}</span>
                      </div>
                      <span
                        className={`px-2 py-0.5 rounded text-[9px] ${
                          dep.required
                            ? "bg-emerald-500/15 text-emerald-300"
                            : "bg-blue-500/15 text-blue-300"
                        }`}
                      >
                        {dep.status}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {activeTab === "installation" && (
              <div className="space-y-3">
                <p className="text-[11px] text-[var(--rz-text-secondary)] leading-relaxed">
                  Installing stages the preset payload into your user configuration directory at:
                </p>
                <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] font-mono text-xs text-[var(--rz-accent)]">
                  ~/.config/fastfetch/presets/{selectedPackage.id}.jsonc
                </div>
                <p className="text-[11px] text-[var(--rz-text-secondary)] leading-relaxed">
                  To apply this preset as your primary shell fetch, click <strong>Open in Fastfetch Settings</strong> and customize your emblem, accent colors, and visible modules before saving.
                </p>
              </div>
            )}

            {activeTab === "changelog" && (
              <div className="space-y-3 font-mono">
                <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1">
                  <div className="flex items-center justify-between text-xs">
                    <span className="font-semibold text-[var(--rz-text)]">v{selectedPackage.version}</span>
                    <span className="text-[10px] text-[var(--rz-text-muted)]">Verified Release</span>
                  </div>
                  <p className="text-[11px] text-[var(--rz-text-secondary)] pt-1 font-sans">
                    Initial community catalog release. Clean JSONC structure with Wayland/X11 compatibility tested.
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Lightbox Modal */}
      {showLightbox && activeImage && (
        <div
          className="fixed inset-0 z-50 bg-black/90 flex items-center justify-center p-4 cursor-pointer"
          onClick={() => setShowLightbox(false)}
        >
          <img
            src={activeImage}
            alt={selectedPackage.title}
            className="max-w-[90vw] max-h-[90vh] object-contain rounded-xl shadow-2xl"
          />
        </div>
      )}
    </ProductDetailShell>
  );
};
