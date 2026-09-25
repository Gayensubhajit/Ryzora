import React, { useRef, useState } from "react";
import { Plus, Film, Image as ImageIcon } from "lucide-react";
import { SilentSddmService } from "../../services/silentSddmService";

interface UploadMediaCardProps {
  onFileSelected: (fileOrPath: { path?: string; file?: File }) => void;
}

export const UploadMediaCard: React.FC<UploadMediaCardProps> = ({
  onFileSelected,
}) => {
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [isOpeningPicker, setIsOpeningPicker] = useState(false);

  const handleClick = async () => {
    if (isOpeningPicker) return;
    setIsOpeningPicker(true);
    try {
      // 1. Attempt native desktop picker (zenity / kdialog) via backend
      const pickedPath = await SilentSddmService.pickCustomMediaFile();
      if (pickedPath) {
        onFileSelected({ path: pickedPath });
        return;
      }
    } catch (e) {
      console.warn("Native file picker unavailable, falling back to input:", e);
    } finally {
      setIsOpeningPicker(false);
    }

    // 2. Fallback to HTML input if native dialog was cancelled or unhandled
    fileInputRef.current?.click();
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;

    // In WebKitGTK / Tauri webview, file may have a native path property
    const nativePath = (file as any).path || (file as any).webkitRelativePath;
    if (nativePath && typeof nativePath === "string" && nativePath.startsWith("/")) {
      onFileSelected({ path: nativePath, file });
    } else {
      onFileSelected({ file });
    }

    // Reset input so re-selecting same file triggers change
    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    const file = e.dataTransfer.files?.[0];
    if (!file) return;

    const nativePath = (file as any).path;
    if (nativePath && typeof nativePath === "string" && nativePath.startsWith("/")) {
      onFileSelected({ path: nativePath, file });
    } else {
      onFileSelected({ file });
    }
  };

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleClick();
        }
      }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={[
        "relative group flex flex-col items-center justify-center p-6 text-center cursor-pointer transition-all duration-200 select-none rounded-xl",
        "border-2 border-dashed min-h-[240px] h-full",
        isDragging
          ? "border-[var(--accent)] bg-[var(--accent)]/10 scale-[1.01]"
          : "border-[var(--rz-border-strong)] hover:border-[var(--rz-accent)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] hover:shadow-lg",
      ].join(" ")}
      style={{ minHeight: "260px" }}
    >
      <input
        ref={fileInputRef}
        type="file"
        accept=".jpg,.jpeg,.png,.mp4,.webm,.mkv,.mov,.m4v,.avi"
        onChange={handleInputChange}
        className="hidden"
        aria-hidden="true"
      />

      <div className="w-14 h-14 rounded-full bg-[var(--rz-accent)]/15 text-[var(--rz-accent)] flex items-center justify-center mb-3 transition-transform group-hover:scale-110 duration-200 border border-[var(--rz-accent)]/30 shadow-sm">
        <Plus className="w-7 h-7 stroke-[2.2]" />
      </div>

      <div className="space-y-1">
        <h3 className="text-sm font-bold tracking-tight text-[var(--rz-text)] group-hover:text-[var(--rz-accent)] transition-colors">
          Upload Media
        </h3>
        <p className="text-[11px] text-[var(--rz-text-muted)]">
          Photo or Video
        </p>
      </div>

      <div className="flex items-center gap-2 mt-4 text-[10px] text-[var(--rz-text-muted)] font-mono">
        <span className="flex items-center gap-1">
          <ImageIcon className="w-3 h-3 text-[var(--rz-text-muted)]" />
          JPG · PNG
        </span>
        <span>•</span>
        <span className="flex items-center gap-1">
          <Film className="w-3 h-3 text-[var(--rz-text-muted)]" />
          MP4 · WEBM · MKV
        </span>
      </div>

      <div className="mt-3 flex items-center gap-2">
        <span className="px-2 py-0.5 rounded bg-[var(--rz-surface-base)] border border-[var(--rz-border-subtle)] text-[9px] font-mono tracking-wider text-[var(--rz-text-muted)] uppercase">
          Login Screen (SDDM)
        </span>
        <span className="px-2 py-0.5 rounded bg-[var(--rz-surface-base)] border border-[var(--rz-border-subtle)] text-[9px] font-mono tracking-wider text-emerald-400 font-semibold uppercase">
          Max 1 GiB
        </span>
      </div>
    </div>
  );
};
