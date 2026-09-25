import { resolveLocalAssetSrc } from "../../providers/customMediaProvider";
import React, { useState, useEffect, useMemo } from "react";
import { X, Check, AlertCircle, Loader2, Film, Image as ImageIcon, ShieldCheck } from "lucide-react";
import { SilentSddmService } from "../../services/silentSddmService";
import type { SilentSddmCustomMediaValidation } from "../../types";

interface CustomMediaImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  fileOrPath: { path?: string; file?: File } | null;
  onImportSuccess: (importedId: string) => void;
}

function formatBytes(bytes?: number | null): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function toDisplayName(filename: string): string {
  const base = filename.replace(/\.[^.]+$/, "");
  return base
    .replace(/[_-]/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase())
    .trim();
}

export const CustomMediaImportModal: React.FC<CustomMediaImportModalProps> = ({
  isOpen,
  onClose,
  fileOrPath,
  onImportSuccess,
}) => {
  const [displayName, setDisplayName] = useState("");
  const [validation, setValidation] = useState<SilentSddmCustomMediaValidation | null>(null);
  const [isValidating, setIsValidating] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  // Derive preview URL safely
  const previewUrl = useMemo(() => {
    if (!fileOrPath) return "";
    if (fileOrPath.file) {
      try {
        return URL.createObjectURL(fileOrPath.file);
      } catch {
        // Fallback
      }
    }
    if (fileOrPath.path) {
      return resolveLocalAssetSrc(fileOrPath.path);
    }
    return "";
  }, [fileOrPath]);

  // Clean up object URL on unmount
  useEffect(() => {
    return () => {
      if (previewUrl && previewUrl.startsWith("blob:")) {
        URL.revokeObjectURL(previewUrl);
      }
    };
  }, [previewUrl]);

  // Validate selected file/path on mount or when fileOrPath changes
  useEffect(() => {
    if (!isOpen || !fileOrPath) {
      setValidation(null);
      setErrorMessage(null);
      return;
    }

    const filename = fileOrPath.file?.name || fileOrPath.path?.split("/").pop() || "";
    setDisplayName(toDisplayName(filename));

    const runValidation = async () => {
      setIsValidating(true);
      setErrorMessage(null);

      // Check GIF extension immediately on client side
      const lower = filename.toLowerCase();
      if (lower.endsWith(".gif")) {
        setErrorMessage(
          ".gif files are not supported by SilentSDDM — they can crash SDDM. Please use .mp4, .webm, .mkv, .mov, .avi, or .m4v instead."
        );
        setIsValidating(false);
        return;
      }

      // If we have a native path, validate with backend
      if (fileOrPath.path) {
        try {
          const res = await SilentSddmService.validateCustomMedia(fileOrPath.path);
          setValidation(res);
          if (!res.valid) {
            setErrorMessage(res.error || "File validation failed.");
          }
        } catch (e: any) {
          setErrorMessage(e?.message || "Failed to validate file with system.");
        } finally {
          setIsValidating(false);
        }
      } else if (fileOrPath.file) {
        // Browser file validation
        const ext = filename.split(".").pop()?.toLowerCase() || "";
        const videoExts = ["mp4", "webm", "mkv", "mov", "m4v", "avi"];
        const imageExts = ["jpg", "jpeg", "png"];
        const isVideo = videoExts.includes(ext);
        const isImage = imageExts.includes(ext);

        if (!isVideo && !isImage) {
          setErrorMessage(`Unsupported format .${ext}. Supported: JPG, PNG, MP4, WEBM, MKV, MOV.`);
        } else {
          setValidation({
            valid: true,
            path: fileOrPath.file.name,
            filename,
            extension: ext,
            media_type: isVideo ? "video" : "image",
            size_bytes: fileOrPath.file.size,
            error: null,
          });
        }
        setIsValidating(false);
      }
    };

    runValidation();
  }, [isOpen, fileOrPath]);

  if (!isOpen || !fileOrPath) return null;

  const isVideo =
    validation?.media_type === "video" ||
    fileOrPath.file?.type.startsWith("video/") ||
    /\.(mp4|webm|mkv|mov|m4v|avi)$/i.test(fileOrPath.file?.name || fileOrPath.path || "");

  const filename = fileOrPath.file?.name || fileOrPath.path?.split("/").pop() || "media";
  const sizeBytes = validation?.size_bytes ?? fileOrPath.file?.size;

  const handleImport = async () => {
    if (!fileOrPath.path) {
      setErrorMessage(
        "Direct browser file import requires a filesystem path in desktop mode. Please use the native file selector."
      );
      return;
    }

    setIsImporting(true);
    setErrorMessage(null);
    try {
      const asset = await SilentSddmService.importCustomMedia(
        fileOrPath.path,
        displayName.trim() || undefined
      );
      setIsImporting(false);
      onImportSuccess(asset.id);
      onClose();
    } catch (e: any) {
      setIsImporting(false);
      setErrorMessage(e?.toString() || "Failed to import custom background.");
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-xs animate-fade-in"
      onClick={onClose}
    >
      <div
        className="relative w-full max-w-lg overflow-hidden rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] shadow-2xl animate-scale-up"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--rz-border-subtle)]">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-[var(--rz-accent)]/15 text-[var(--rz-accent)] border border-[var(--rz-accent)]/30 flex items-center justify-center">
              {isVideo ? <Film className="w-4 h-4" /> : <ImageIcon className="w-4 h-4" />}
            </div>
            <div>
              <h2 className="text-base font-bold text-[var(--rz-text)]">
                Add Custom Background
              </h2>
              <p className="text-[11px] text-[var(--rz-text-muted)] font-mono">
                Max 1 GiB · Stored locally · Source file is not copied on import
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-5 max-h-[80vh] overflow-y-auto">
          {/* Preview Area */}
          <div className="relative w-full overflow-hidden rounded-xl bg-black/40 border border-[var(--border-subtle)] flex items-center justify-center min-h-[200px] max-h-[280px]">
            {isValidating ? (
              <div className="flex flex-col items-center gap-2 text-xs text-[var(--rz-text-muted)]">
                <Loader2 className="w-6 h-6 animate-spin text-[var(--rz-accent)]" />
                <span>Checking media compatibility...</span>
              </div>
            ) : isVideo ? (
              <video
                src={previewUrl}
                controls
                autoPlay
                loop
                muted
                playsInline
                className="w-full h-full max-h-[280px] object-cover rounded-xl"
              />
            ) : (
              <img
                src={previewUrl}
                alt="Preview"
                className="w-full h-full max-h-[280px] object-cover rounded-xl"
              />
            )}
          </div>

          {/* Error Banner if invalid */}
          {errorMessage && (
            <div className="flex items-start gap-2.5 p-3 rounded-lg bg-red-500/10 border border-red-500/25 text-red-400 text-xs">
              <AlertCircle className="w-4 h-4 flex-shrink-0 mt-0.5" />
              <div className="leading-relaxed">{errorMessage}</div>
            </div>
          )}

          {/* File Metadata & Validation Badges */}
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs">
              <span className="font-mono font-medium text-[var(--rz-text)] truncate max-w-[280px]">
                {filename}
              </span>
              <span className="text-[var(--rz-text-muted)] font-mono">
                {formatBytes(sizeBytes)} · {isVideo ? "Video" : "Photo"}
              </span>
            </div>

            <div className="flex flex-wrap items-center gap-2 text-[10px] text-[var(--rz-text-muted)]">
              <span className="inline-flex items-center gap-1 text-emerald-400">
                <Check className="w-3 h-3" /> Supported format
              </span>
              <span>•</span>
              <span className="inline-flex items-center gap-1 text-emerald-400">
                <Check className="w-3 h-3" /> Regular file
              </span>
              <span>•</span>
              <span className="inline-flex items-center gap-1 text-emerald-400">
                <Check className="w-3 h-3" /> Within 1 GiB limit
              </span>
            </div>
          </div>

          {/* Name Input */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-[var(--rz-text-muted)]">
              Background Name
            </label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="e.g. Anime Night"
              className="ryz-input w-full text-xs"
              maxLength={64}
            />
          </div>

          {/* Target capability (Display only — no checkbox) */}
          <div className="space-y-1.5">
            <label className="text-xs font-semibold uppercase tracking-wider text-[var(--rz-text-muted)]">
              Target
            </label>
            <div className="p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)] space-y-1">
              <div className="flex items-center gap-2 text-xs font-semibold text-[var(--rz-text)]">
                <ShieldCheck className="w-4 h-4 text-emerald-400" />
                <span>SilentSDDM · Login Screen (SDDM)</span>
              </div>
              <p className="text-[11px] text-[var(--rz-text-muted)] pl-6">
                Applied only when you choose Apply in Ryzora.
              </p>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2.5 px-6 py-4 border-t border-[var(--border-subtle)] bg-[var(--rz-surface-elevated)]/50">
          <button
            type="button"
            onClick={onClose}
            disabled={isImporting}
            className="ryz-btn ryz-btn-secondary text-xs px-4 py-2"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleImport}
            disabled={isImporting || isValidating || Boolean(errorMessage)}
            className="ryz-btn ryz-btn-primary text-xs px-5 py-2 flex items-center gap-1.5"
          >
            {isImporting ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>Importing...</span>
              </>
            ) : (
              <span>Add Background</span>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
