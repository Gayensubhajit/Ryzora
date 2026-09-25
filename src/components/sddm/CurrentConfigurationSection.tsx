import React, { useState, useEffect } from "react";
import {
  Play,
  Sliders,
  RotateCcw,
  CheckCircle2,
  Image as ImageIcon,
  Video as VideoIcon,
  RefreshCw,
  Sparkles,
} from "lucide-react";
import type {
  SilentSddmHostReport,
  SilentSddmActivationManifest,
  SilentSddmConfiguration,
} from "../../types/index.ts";
import { SilentSddmService } from "../../services/silentSddmService.ts";
import { resolveLocalAssetSrc } from "../../providers/customMediaProvider.ts";
import { SilentSddmConfigPanel } from "./SilentSddmConfigPanel.tsx";

interface CurrentConfigurationSectionProps {
  report: SilentSddmHostReport | null;
  onRefresh?: () => Promise<void>;
  onChangeWallpaper?: (target: "login" | "lock") => void;
  onLaunchTest?: (target: "sddm") => void;
}

export const CurrentConfigurationSection: React.FC<CurrentConfigurationSectionProps> = ({
  report,
  onRefresh,
  onChangeWallpaper,
  onLaunchTest,
}) => {
  const [manifest, setManifest] = useState<SilentSddmActivationManifest | null>(null);
  const [config, setConfig] = useState<SilentSddmConfiguration | null>(null);
  const [isDeactivating, setIsDeactivating] = useState(false);
  const [showConfigModal, setShowConfigModal] = useState(false);

  useEffect(() => {
    let mounted = true;
    async function loadState() {
      try {
        const [m, c] = await Promise.all([
          SilentSddmService.getActivationManifest().catch(() => null),
          SilentSddmService.getConfiguration().catch(() => null),
        ]);
        if (mounted) {
          setManifest(m);
          setConfig(c);
        }
      } catch {
        // Handled silently
      }
    }
    loadState();
    return () => {
      mounted = false;
    };
  }, [report]);

  const isSddmActive = Boolean(report?.ryzora_owns_sddm && manifest);

  if (!isSddmActive && !manifest) {
    return null;
  }

  const findAssetDetails = (assetKey: string | undefined): {
    title: string;
    isCustom: boolean;
    isVideo: boolean;
    previewUrl?: string;
    posterUrl?: string;
    sizeFormatted?: string;
    sourceLabel: string;
  } => {
    if (!assetKey) {
      return {
        title: "Default Background",
        isCustom: false,
        isVideo: false,
        sourceLabel: "SilentSDDM",
      };
    }

    // 1. Check custom assets (My Media)
    const customAsset = report?.cached_custom?.find(
      (c) => c.id === assetKey || c.filename === assetKey
    );
    if (customAsset) {
      const isVideo = customAsset.media_type === "video";
      const rawPath = customAsset.original_path || `~/.local/share/ryzora/lockscreens/silentsddm/custom/${customAsset.filename}`;
      const previewUrl = resolveLocalAssetSrc(rawPath);
      const rawPoster = `~/.local/share/ryzora/lockscreens/silentsddm/posters/${customAsset.filename}.poster.jpg`;
      const posterUrl = isVideo ? resolveLocalAssetSrc(rawPoster) : previewUrl;
      const sizeMb = customAsset.size_bytes ? (customAsset.size_bytes / (1024 * 1024)).toFixed(1) + " MB" : undefined;

      return {
        title: customAsset.display_name || customAsset.filename,
        isCustom: true,
        isVideo,
        previewUrl,
        posterUrl,
        sizeFormatted: sizeMb,
        sourceLabel: "My Media",
      };
    }

    // 2. Check built-in wallpapers
    const wpAsset = report?.cached_wallpapers?.find(
      (w) => w.id === assetKey || w.filename === assetKey
    );
    if (wpAsset) {
      const isVideo = wpAsset.media_type === "video";
      const rawWp = `~/.local/share/ryzora/lockscreens/silentsddm/wallpapers/${wpAsset.filename}`;
      const previewUrl = resolveLocalAssetSrc(rawWp);
      const sizeMb = wpAsset.size_bytes ? (wpAsset.size_bytes / (1024 * 1024)).toFixed(1) + " MB" : undefined;

      return {
        title: wpAsset.display_name || wpAsset.filename,
        isCustom: false,
        isVideo,
        previewUrl,
        sizeFormatted: sizeMb,
        sourceLabel: "Built-in",
      };
    }

    // 3. Fallback based on manifest
    const isVideo = manifest?.media_type === "video" || assetKey.endsWith(".mp4") || assetKey.endsWith(".webm");
    return {
      title: assetKey.replace(/\.[^.]+$/, ""),
      isCustom: assetKey.startsWith("custom:"),
      isVideo,
      previewUrl: undefined,
      sourceLabel: assetKey.startsWith("custom:") ? "My Media" : "Built-in",
    };
  };

  const loginAssetKey = manifest?.active_asset_id || config?.login_screen?.background;
  const lockAssetKey = config?.lock_screen?.background;

  const loginDetails = loginAssetKey ? findAssetDetails(loginAssetKey) : null;
  const lockDetails = lockAssetKey ? findAssetDetails(lockAssetKey) : null;

  const handleDeactivate = async () => {
    if (isDeactivating) return;
    setIsDeactivating(true);
    try {
      await SilentSddmService.deactivate();
      if (onRefresh) await onRefresh();
    } catch (err: any) {
      console.error("Deactivate error:", err);
    } finally {
      setIsDeactivating(false);
    }
  };

  const handleTest = () => {
    if (onLaunchTest) {
      onLaunchTest("sddm");
    }
  };

  return (
    <div className="mb-6 rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] p-5 shadow-xl relative overflow-hidden">
      {/* Section Header: Current Wallpaper */}
      <div className="flex flex-wrap items-center justify-between gap-3 mb-4 border-b border-[var(--rz-border-subtle)] pb-3">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-xl bg-[var(--rz-accent)]/15 border border-[var(--rz-accent)]/30 flex items-center justify-center text-[var(--rz-accent)]">
            <Sparkles className="w-4 h-4" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-bold tracking-wide uppercase text-[var(--rz-text)]">
                Current Wallpaper
              </h2>
              <span className="flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-emerald-500/20 border border-emerald-500/40 text-emerald-400">
                <CheckCircle2 className="w-3 h-3" />
                Active
              </span>
            </div>
            <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5">
              SilentSDDM Greeter Theme · Independent Login and Lock Screen Targets
            </p>
          </div>
        </div>

        {/* Global Deactivate */}
        <button
          type="button"
          disabled={isDeactivating}
          onClick={handleDeactivate}
          className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-[var(--rz-surface)] hover:bg-rose-500/10 border border-[var(--rz-border-strong)] hover:border-rose-500/40 text-[var(--rz-text-muted)] hover:text-rose-400 transition-all cursor-pointer flex items-center gap-1.5 shadow-xs"
          title="Deactivate SilentSDDM and restore previous display manager configuration"
        >
          <RotateCcw className={`w-3.5 h-3.5 ${isDeactivating ? "animate-spin" : ""}`} />
          <span>{isDeactivating ? "Deactivating…" : "Deactivate"}</span>
        </button>
      </div>

      {/* Target Cards Grid: Only render targets that actually have configured wallpapers */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* TARGET 1: Login Screen */}
        {loginDetails && (
          <div className="rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] p-3.5 flex flex-col justify-between">
            <div className="flex gap-3.5">
              {/* Media Preview Box */}
              <div className="w-32 h-24 rounded-lg overflow-hidden bg-black/60 border border-[var(--rz-border-strong)] relative flex-shrink-0 flex items-center justify-center">
                {loginDetails.posterUrl || loginDetails.previewUrl ? (
                  <img
                    src={loginDetails.posterUrl || loginDetails.previewUrl}
                    alt={loginDetails.title}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="flex flex-col items-center gap-1 text-[var(--rz-text-muted)]">
                    {loginDetails.isVideo ? <VideoIcon className="w-5 h-5 opacity-60" /> : <ImageIcon className="w-5 h-5 opacity-60" />}
                    <span className="text-[9px] uppercase font-mono tracking-wider">Preview</span>
                  </div>
                )}
                {loginDetails.isVideo && (
                  <div className="absolute bottom-1 right-1 p-1 rounded bg-black/70 backdrop-blur-xs text-white/90">
                    <VideoIcon className="w-3 h-3" />
                  </div>
                )}
                <div className="absolute top-1 left-1 px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider bg-black/80 text-white/95 border border-white/10 backdrop-blur-xs">
                  Login
                </div>
              </div>

              {/* Details */}
              <div className="flex-1 min-w-0 flex flex-col justify-between py-0.5">
                <div>
                  <div className="flex items-center gap-1.5 mb-1">
                    <span className="text-[10px] font-bold uppercase tracking-wider text-[var(--rz-accent)]">
                      Login Screen
                    </span>
                    <span className="text-[10px] text-[var(--rz-text-muted)]">·</span>
                    <span className="text-[10px] text-[var(--rz-text-muted)] font-medium">
                      {loginDetails.sourceLabel}
                    </span>
                  </div>

                  <h3 className="text-xs font-bold text-[var(--rz-text)] truncate" title={loginDetails.title}>
                    {loginDetails.title}
                  </h3>

                  <div className="flex items-center gap-1.5 mt-1 text-[11px] text-[var(--rz-text-muted)] font-mono">
                    <span>{loginDetails.isVideo ? "Video" : "Image"}</span>
                    {loginDetails.sizeFormatted && (
                      <>
                        <span>·</span>
                        <span>{loginDetails.sizeFormatted}</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Target Actions: [ Change ] [ Test ] [ Configure ] */}
            <div className="flex items-center gap-2 mt-3 pt-2.5 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => onChangeWallpaper?.("login")}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Change login wallpaper"
              >
                <RefreshCw className="w-3 h-3 text-[var(--rz-text-muted)]" />
                <span>Change</span>
              </button>

              <button
                type="button"
                onClick={handleTest}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Test Login Screen live in isolated window"
              >
                <Play className="w-3 h-3 text-[var(--rz-accent)] fill-[var(--rz-accent)]/20" />
                <span>Test</span>
              </button>

              <button
                type="button"
                onClick={() => setShowConfigModal(true)}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Configure layout and visual effects"
              >
                <Sliders className="w-3 h-3 text-[var(--rz-text-muted)]" />
                <span>Configure</span>
              </button>
            </div>
          </div>
        )}

        {/* TARGET 2: Lock Screen */}
        {lockDetails && (
          <div className="rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] p-3.5 flex flex-col justify-between">
            <div className="flex gap-3.5">
              {/* Media Preview Box */}
              <div className="w-32 h-24 rounded-lg overflow-hidden bg-black/60 border border-[var(--rz-border-strong)] relative flex-shrink-0 flex items-center justify-center">
                {lockDetails.posterUrl || lockDetails.previewUrl ? (
                  <img
                    src={lockDetails.posterUrl || lockDetails.previewUrl}
                    alt={lockDetails.title}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="flex flex-col items-center gap-1 text-[var(--rz-text-muted)]">
                    {lockDetails.isVideo ? <VideoIcon className="w-5 h-5 opacity-60" /> : <ImageIcon className="w-5 h-5 opacity-60" />}
                    <span className="text-[9px] uppercase font-mono tracking-wider">Preview</span>
                  </div>
                )}
                {lockDetails.isVideo && (
                  <div className="absolute bottom-1 right-1 p-1 rounded bg-black/70 backdrop-blur-xs text-white/90">
                    <VideoIcon className="w-3 h-3" />
                  </div>
                )}
                <div className="absolute top-1 left-1 px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider bg-black/80 text-white/95 border border-white/10 backdrop-blur-xs">
                  Lock
                </div>
              </div>

              {/* Details */}
              <div className="flex-1 min-w-0 flex flex-col justify-between py-0.5">
                <div>
                  <div className="flex items-center gap-1.5 mb-1">
                    <span className="text-[10px] font-bold uppercase tracking-wider text-purple-400">
                      Lock Screen
                    </span>
                    <span className="text-[10px] text-[var(--rz-text-muted)]">·</span>
                    <span className="text-[10px] text-[var(--rz-text-muted)] font-medium">
                      {lockDetails.sourceLabel}
                    </span>
                  </div>

                  <h3 className="text-xs font-bold text-[var(--rz-text)] truncate" title={lockDetails.title}>
                    {lockDetails.title}
                  </h3>

                  <div className="flex items-center gap-1.5 mt-1 text-[11px] text-[var(--rz-text-muted)] font-mono">
                    <span>{lockDetails.isVideo ? "Video" : "Image"}</span>
                    {lockDetails.sizeFormatted && (
                      <>
                        <span>·</span>
                        <span>{lockDetails.sizeFormatted}</span>
                      </>
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Target Actions: [ Change ] [ Test ] [ Configure ] */}
            <div className="flex items-center gap-2 mt-3 pt-2.5 border-t border-[var(--rz-border-subtle)]">
              <button
                type="button"
                onClick={() => onChangeWallpaper?.("lock")}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Change lock wallpaper"
              >
                <RefreshCw className="w-3 h-3 text-[var(--rz-text-muted)]" />
                <span>Change</span>
              </button>

              <button
                type="button"
                onClick={handleTest}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Test Lock Screen live in isolated window"
              >
                <Play className="w-3 h-3 text-[var(--rz-accent)] fill-[var(--rz-accent)]/20" />
                <span>Test</span>
              </button>

              <button
                type="button"
                onClick={() => setShowConfigModal(true)}
                className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1 shadow-xs"
                title="Configure layout and visual effects"
              >
                <Sliders className="w-3 h-3 text-[var(--rz-text-muted)]" />
                <span>Configure</span>
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Configuration Modal */}
      {showConfigModal && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-xs animate-in fade-in duration-200"
          onClick={() => setShowConfigModal(false)}
        >
          <div
            className="w-full max-w-xl max-h-[85vh] overflow-y-auto rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] p-6 shadow-2xl space-y-4"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between pb-3 border-b border-[var(--rz-border-subtle)]">
              <div>
                <h3 className="text-sm font-bold text-[var(--rz-text)]">
                  SilentSDDM Theme Configuration
                </h3>
                <p className="text-[11px] text-[var(--rz-text-muted)]">
                  Tune wallpaper fill modes, lock clock placement, and visual effects
                </p>
              </div>
              <button
                type="button"
                onClick={() => setShowConfigModal(false)}
                className="w-7 h-7 rounded-lg text-xs font-semibold text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] flex items-center justify-center cursor-pointer"
              >
                ✕
              </button>
            </div>

            <SilentSddmConfigPanel
              onApplied={async () => {
                setShowConfigModal(false);
                if (onRefresh) await onRefresh();
              }}
              onTestMode={() => handleTest()}
            />
          </div>
        </div>
      )}
    </div>
  );
};
