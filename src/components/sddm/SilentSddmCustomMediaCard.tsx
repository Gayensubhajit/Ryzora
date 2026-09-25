import React, { useState, useRef } from "react";
import {
  UploadCloud,
  Film,
  Image as ImageIcon,
  Check,
  AlertCircle,
  Play,
  RotateCcw,
  Loader2,
  Monitor,
  Lock,
} from "lucide-react";
import type { SilentSddmCachedAsset } from "../../types/index.ts";
import { SilentSddmService } from "../../services/silentSddmService.ts";
import { resolveLocalAssetSrc } from "../../providers/customMediaProvider.ts";

interface SilentSddmCustomMediaCardProps {
  onMediaApplied?: (manifest: any) => void;
  onTestMode?: (target: "sddm") => void;
}

export const SilentSddmCustomMediaCard: React.FC<SilentSddmCustomMediaCardProps> = ({
  onMediaApplied,
  onTestMode,
}) => {
  const [isPicking, setIsPicking] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedAsset, setSelectedAsset] = useState<SilentSddmCachedAsset | null>(null);
  const [targetScreen, setTargetScreen] = useState<"login" | "lock" | "both">("both");
  const [mediaDimensions, setMediaDimensions] = useState<{ width: number; height: number } | null>(null);

  const videoRef = useRef<HTMLVideoElement>(null);

  const handlePickFile = async () => {
    setError(null);
    setIsPicking(true);
    try {
      const pickedPath = await SilentSddmService.pickCustomMediaFile();
      if (!pickedPath) {
        setIsPicking(false);
        return;
      }

      // Pre-validation
      const val = await SilentSddmService.validateCustomMedia(pickedPath);
      if (!val.valid) {
        setError(val.error || "Selected file is not supported");
        setIsPicking(false);
        return;
      }

      setIsImporting(true);
      const imported = await SilentSddmService.importCustomMedia(pickedPath);
      setSelectedAsset(imported);
      setMediaDimensions(null);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsPicking(false);
      setIsImporting(false);
    }
  };

  const handleVideoLoadedMetadata = () => {
    if (videoRef.current) {
      setMediaDimensions({
        width: videoRef.current.videoWidth,
        height: videoRef.current.videoHeight,
      });
    }
  };

  const handleImageLoaded = (e: React.SyntheticEvent<HTMLImageElement>) => {
    setMediaDimensions({
      width: e.currentTarget.naturalWidth,
      height: e.currentTarget.naturalHeight,
    });
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes >= 1024 * 1024 * 1024) {
      return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
    }
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const handleApply = async () => {
    if (!selectedAsset) return;
    setError(null);
    setIsApplying(true);
    try {
      const currentConfig = await SilentSddmService.getConfiguration();
      if (targetScreen === "login" || targetScreen === "both") {
        currentConfig.login_screen.background = selectedAsset.id;
      }
      if (targetScreen === "lock" || targetScreen === "both") {
        currentConfig.lock_screen.background = selectedAsset.id;
      }
      const manifest = await SilentSddmService.applyConfiguration(currentConfig);
      onMediaApplied?.(manifest);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsApplying(false);
    }
  };

  const mediaSrc = selectedAsset
    ? resolveLocalAssetSrc(
        `~/.local/share/ryzora/lockscreens/silentsddm/custom/${selectedAsset.filename}`
      )
    : "";

  return (
    <div className="rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] p-6 shadow-sm space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-[var(--rz-text)] flex items-center gap-2">
            <Film className="w-4 h-4 text-[var(--rz-accent)]" />
            <span>Custom Wallpaper / Video</span>
          </h3>
          <p className="text-xs text-[var(--rz-text-secondary)] mt-0.5">
            Add your own high-resolution video or photo to run directly on the SilentSDDM login & lock screen.
          </p>
        </div>
        <span className="text-[10px] font-mono uppercase px-2 py-0.5 rounded-full bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)]">
          Max 1 GiB
        </span>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/10 p-3 text-xs text-rose-400 flex items-start gap-2">
          <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      {!selectedAsset ? (
        /* Empty / Dropzone State */
        <button
          type="button"
          disabled={isPicking || isImporting}
          onClick={handlePickFile}
          className="w-full h-48 rounded-xl border-2 border-dashed border-[var(--rz-border-strong)] hover:border-[var(--rz-accent)] bg-[var(--rz-surface)]/60 hover:bg-[var(--rz-surface-hover)] transition-all cursor-pointer flex flex-col items-center justify-center p-6 text-center group"
        >
          {isImporting ? (
            <div className="flex flex-col items-center gap-2">
              <Loader2 className="w-8 h-8 text-[var(--rz-accent)] animate-spin" />
              <span className="text-xs font-semibold text-[var(--rz-text)]">Importing & verifying custom media…</span>
            </div>
          ) : (
            <div className="flex flex-col items-center gap-2">
              <div className="w-12 h-12 rounded-2xl bg-[var(--rz-accent)]/10 text-[var(--rz-accent)] flex items-center justify-center group-hover:scale-105 transition-transform shadow-xs">
                <UploadCloud className="w-6 h-6" />
              </div>
              <div>
                <span className="text-sm font-semibold text-[var(--rz-text)] block">
                  ＋ Add your wallpaper or video
                </span>
                <span className="text-xs text-[var(--rz-text-muted)] mt-1 block">
                  JPG · PNG · MP4 · WEBM · MKV · MOV · AVI · M4V (up to 1 GB)
                </span>
              </div>
            </div>
          )}
        </button>
      ) : (
        /* Media Loaded & Configured State */
        <div className="space-y-4">
          <div className="relative rounded-xl overflow-hidden bg-black/40 border border-[var(--rz-border-strong)] aspect-video max-h-72 w-full flex items-center justify-center">
            {selectedAsset.media_type === "video" ? (
              <video
                ref={videoRef}
                src={mediaSrc}
                controls
                autoPlay
                loop
                muted
                onLoadedMetadata={handleVideoLoadedMetadata}
                className="w-full h-full object-contain"
              />
            ) : (
              <img
                src={mediaSrc}
                alt={selectedAsset.filename}
                onLoad={handleImageLoaded}
                className="w-full h-full object-contain"
              />
            )}
          </div>

          {/* Metadata Row */}
          <div className="flex flex-wrap items-center justify-between gap-3 p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-xs">
            <div className="flex items-center gap-2">
              {selectedAsset.media_type === "video" ? (
                <Film className="w-4 h-4 text-[var(--rz-accent)]" />
              ) : (
                <ImageIcon className="w-4 h-4 text-[var(--rz-accent)]" />
              )}
              <span className="font-semibold text-[var(--rz-text)] truncate max-w-[200px]" title={selectedAsset.filename}>
                {selectedAsset.filename}
              </span>
            </div>

            <div className="flex items-center gap-2 text-[var(--rz-text-secondary)] font-mono text-[11px]">
              {mediaDimensions && (
                <span className="px-2 py-0.5 rounded bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                  {mediaDimensions.width} × {mediaDimensions.height}
                </span>
              )}
              <span className="px-2 py-0.5 rounded bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
                {formatFileSize(selectedAsset.size_bytes)}
              </span>
              <span className="px-2 py-0.5 rounded bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]" title={selectedAsset.sha256}>
                SHA: {selectedAsset.sha256.slice(0, 8)}…
              </span>
            </div>
          </div>

          {/* Target Selector */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold text-[var(--rz-text-secondary)]">Apply Target:</label>
            <div className="grid grid-cols-3 gap-2">
              <button
                type="button"
                onClick={() => setTargetScreen("login")}
                className={`py-2 px-3 rounded-xl text-xs font-medium border flex items-center justify-center gap-1.5 transition-all cursor-pointer ${
                  targetScreen === "login"
                    ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-accent)] font-semibold shadow-xs"
                    : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                }`}
              >
                <Monitor className="w-3.5 h-3.5" />
                <span>Login Screen</span>
              </button>

              <button
                type="button"
                onClick={() => setTargetScreen("lock")}
                className={`py-2 px-3 rounded-xl text-xs font-medium border flex items-center justify-center gap-1.5 transition-all cursor-pointer ${
                  targetScreen === "lock"
                    ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-accent)] font-semibold shadow-xs"
                    : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                }`}
              >
                <Lock className="w-3.5 h-3.5" />
                <span>Lock Screen</span>
              </button>

              <button
                type="button"
                onClick={() => setTargetScreen("both")}
                className={`py-2 px-3 rounded-xl text-xs font-medium border flex items-center justify-center gap-1.5 transition-all cursor-pointer ${
                  targetScreen === "both"
                    ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-accent)] font-semibold shadow-xs"
                    : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                }`}
              >
                <Check className="w-3.5 h-3.5" />
                <span>Both Screens</span>
              </button>
            </div>
          </div>

          {/* Action Row */}
          <div className="flex items-center gap-2 pt-2">
            <button
              type="button"
              onClick={handlePickFile}
              className="py-2.5 px-4 rounded-xl text-xs font-medium bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-colors cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
            >
              <RotateCcw className="w-3.5 h-3.5" />
              <span>Replace</span>
            </button>

            {onTestMode && (
              <button
                type="button"
                onClick={() => onTestMode("sddm")}
                className="py-2.5 px-4 rounded-xl text-xs font-medium bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-colors cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
              >
                <Play className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                <span>Test SDDM</span>
              </button>
            )}

            <button
              type="button"
              disabled={isApplying}
              onClick={handleApply}
              className="flex-1 py-2.5 px-5 rounded-xl text-xs font-semibold bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white transition-all cursor-pointer select-none flex items-center justify-center gap-2 shadow-xs"
            >
              {isApplying ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  <span>Applying to {targetScreen === "both" ? "Both Screens" : targetScreen === "login" ? "Login Screen" : "Lock Screen"}…</span>
                </>
              ) : (
                <>
                  <Check className="w-4 h-4" />
                  <span>Apply to {targetScreen === "both" ? "Both Screens" : targetScreen === "login" ? "Login Screen" : "Lock Screen"}</span>
                </>
              )}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
