/**
 * AppIconResolver — Phase 24.2
 *
 * Resolves application icons via:
 *   1. Batch IPC icon resolution via iconCache (data URI / SVG from swcatalog & system icon themes)
 *   2. Trusted app metadata icon URL (curated list)
 *   3. Unique colorful letter-avatar fallback (never an empty space or broken image)
 *
 * Performance guarantee:
 *   - Zero per-card IPC on initial render (batched per page)
 *   - In-memory Map cache lookup is 0ms
 *   - If any image fails to load, gracefully falls back to letter avatar
 */

import React, { useState } from "react";
import { resolveAppMetadata } from "./appMetadata.ts";
import { useIconCached } from "../../services/iconCache.ts";

export {
  resolveAppMetadata,
  KNOWN_APPS,
  type AppMetadata,
  resolveCanonicalAppId,
  deduplicateAppPackages,
  POPULAR_CANONICAL_IDS,
  RECOMMENDED_CANONICAL_IDS,
} from './appMetadata.ts';

export type AppIconSize = "sm" | "md" | "lg" | "xl" | "2xl" | number;

export interface AppIconProps {
  appId?: string;
  packageId?: string;
  className?: string;
  size?: AppIconSize;
  preferSystemIcon?: boolean;
  /** Pre-resolved icon_name from CatalogItem */
  iconName?: string | null;
  /** Pre-resolved icon_path from CatalogItem */
  iconPath?: string | null;
}

export function resolvePixelSize(size: AppIconSize): number {
  if (typeof size === "number") return size;
  switch (size) {
    case "sm": return 28;
    case "md": return 52;
    case "lg": return 68;
    case "xl": return 88;
    case "2xl": return 112;
    default: return 68;
  }
}

// ── Fallback geometric icon (unique per first letter, never a shared generic icon) ──
const FALLBACK_COLORS = [
  "#3b82f6","#8b5cf6","#ec4899","#f97316","#10b981",
  "#06b6d4","#f59e0b","#6366f1","#14b8a6","#84cc16",
];
function fallbackColor(id: string): string {
  let hash = 0;
  for (let i = 0; i < id.length; i++) hash = (hash * 31 + id.charCodeAt(i)) >>> 0;
  return FALLBACK_COLORS[hash % FALLBACK_COLORS.length];
}
function fallbackLetter(id: string): string {
  return (id || "?").replace(/^[^a-zA-Z0-9]*/, "").charAt(0).toUpperCase() || "?";
}

// ── Main AppIcon component ───────────────────────────────────────────────────

export const AppIcon: React.FC<AppIconProps> = ({
  appId,
  packageId,
  className = "",
  size = "lg",
  preferSystemIcon: _preferSystemIcon = true,
  iconName,
  iconPath,
}) => {
  const targetId = (appId || packageId || "").toLowerCase().trim();
  const px = resolvePixelSize(size);
  const [imgFailed, setImgFailed] = useState(false);

  // Use the centralized batch-based cache — no per-card IPC
  const resolved = useIconCached(targetId, iconName, iconPath);

  const containerStyle: React.CSSProperties = {
    width: px, height: px, minWidth: px, minHeight: px, maxWidth: px, maxHeight: px,
  };

  // Tier 1: Inline SVG from icon theme (vector quality)
  if (resolved?.svgContent) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <div
          className="w-full h-full flex items-center justify-center [&>svg]:w-full [&>svg]:h-full [&>svg]:max-w-full [&>svg]:max-h-full [&>svg]:object-contain"
          dangerouslySetInnerHTML={{ __html: resolved.svgContent }}
        />
      </div>
    );
  }

  // Tier 2: Base64 Data URI (PNG/XPM from AppStream or icon theme)
  if (resolved?.dataUri && !imgFailed) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <img
          src={resolved.dataUri}
          alt={targetId}
          width={px}
          height={px}
          className="w-full h-full object-contain pointer-events-none"
          onError={() => setImgFailed(true)}
        />
      </div>
    );
  }

  // Tier 3: Direct fileUrl if explicitly provided and not failed
  if (resolved?.fileUrl && !imgFailed) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <img
          src={resolved.fileUrl}
          alt={targetId}
          width={px}
          height={px}
          className="w-full h-full object-contain pointer-events-none"
          onError={() => setImgFailed(true)}
        />
      </div>
    );
  }

  // Tier 4: Trusted curated metadata icon (only for well-known apps)
  const meta = resolveAppMetadata(targetId);
  if (meta.iconUrl && !imgFailed) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <img
          src={meta.iconUrl}
          alt={targetId}
          width={px}
          height={px}
          className="w-full h-full object-contain pointer-events-none"
          onError={() => setImgFailed(true)}
        />
      </div>
    );
  }

  // Tier 5: Fallback unique colorful letter avatar (used while resolving or when no icon exists)
  const color = fallbackColor(targetId);
  const letter = fallbackLetter(targetId);
  const isResolving = resolved === undefined;

  return (
    <div
      className={`aspect-square shrink-0 flex items-center justify-center rounded-2xl select-none transition-all ${className}`}
      style={{
        ...containerStyle,
        background: isResolving ? `${color}22` : `${color}18`,
        border: `1.5px solid ${isResolving ? `${color}44` : `${color}33`}`,
      }}
      title={isResolving ? `Loading ${targetId}...` : targetId}
    >
      <span
        style={{
          fontSize: Math.round(px * 0.38),
          fontWeight: 700,
          color: isResolving ? color : `${color}bb`,
          lineHeight: 1,
          userSelect: "none",
        }}
      >
        {letter}
      </span>
    </div>
  );
};
