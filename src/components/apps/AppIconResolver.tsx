/**
 * AppIconResolver — Phase 23B
 *
 * Resolves application icons via the Tauri desktop-entry / icon-theme backend.
 * All fake hand-authored brand SVGs have been removed.
 *
 * Fallback hierarchy:
 *   1. System icon (SVG) from desktop entry / icon theme via Tauri IPC
 *   2. System icon (PNG/XPM data URI) from desktop entry / icon theme via Tauri IPC
 *   3. Distinct category-specific geometric fallback (unique per category; never shared)
 */

import React, { useEffect, useState } from "react";
import { resolveAppMetadata } from "./appMetadata.ts";

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

interface AppIconProps {
  appId?: string;
  packageId?: string;
  className?: string;
  size?: AppIconSize;
  preferSystemIcon?: boolean;
}

interface DesktopIconInfo {
  desktop_file: string;
  name: string;
  generic_name: string | null;
  icon_name: string | null;
  icon_path: string | null;
  icon_svg_content: string | null;
  icon_data_uri: string | null;
  exec: string | null;
  startup_wm_class: string | null;
  categories: string[];
}

// Global in-memory cache for resolved desktop system icons
const desktopIconCache = new Map<string, DesktopIconInfo | null>();

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

export const AppIcon: React.FC<AppIconProps> = ({
  appId,
  packageId,
  className = "",
  size = "lg",
  preferSystemIcon = true,
}) => {
  const targetId = (appId || packageId || "").toLowerCase().trim();
  const px = resolvePixelSize(size);
  const [systemIconSvg, setSystemIconSvg] = useState<string | null>(null);
  const [systemIconUri, setSystemIconUri] = useState<string | null>(null);

  useEffect(() => {
    if (!preferSystemIcon || !targetId) return;

    if (desktopIconCache.has(targetId)) {
      const cached = desktopIconCache.get(targetId);
      if (cached?.icon_svg_content) setSystemIconSvg(cached.icon_svg_content);
      else if (cached?.icon_data_uri) setSystemIconUri(cached.icon_data_uri);
      return;
    }

    import("@tauri-apps/api/core")
      .then(({ invoke }) => {
        invoke<DesktopIconInfo | null>("resolve_desktop_app_icon", { packageId: targetId })
          .then((res) => {
            desktopIconCache.set(targetId, res || null);
            if (res?.icon_svg_content) setSystemIconSvg(res.icon_svg_content);
            else if (res?.icon_data_uri) setSystemIconUri(res.icon_data_uri);
          })
          .catch(() => { desktopIconCache.set(targetId, null); });
      })
      .catch(() => { desktopIconCache.set(targetId, null); });
  }, [targetId, preferSystemIcon]);

  const containerStyle: React.CSSProperties = {
    width: px, height: px, minWidth: px, minHeight: px, maxWidth: px, maxHeight: px,
  };

  // Tier 1: System SVG from icon theme (with injected viewBox if needed)
  if (systemIconSvg) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <div
          className="w-full h-full flex items-center justify-center [&>svg]:w-full [&>svg]:h-full [&>svg]:max-w-full [&>svg]:max-h-full [&>svg]:object-contain"
          dangerouslySetInnerHTML={{ __html: systemIconSvg }}
        />
      </div>
    );
  }

  // Tier 2: System PNG/XPM data URI
  if (systemIconUri) {
    return (
      <div
        className={`aspect-square shrink-0 flex items-center justify-center overflow-hidden select-none ${className}`}
        style={containerStyle}
      >
        <img
          src={systemIconUri}
          alt={targetId}
          width={px}
          height={px}
          className="w-full h-full object-contain pointer-events-none"
        />
      </div>
    );
  }

  // Tier 2.5: Trusted application metadata icon (authentic bundled icon for catalog apps)
  const meta = resolveAppMetadata(targetId);
  if (meta.iconUrl) {
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
        />
      </div>
    );
  }

  // Tier 3: Neutral Ryzora application fallback (communicates artwork unavailable without pretending)
  return (
    <div
      className={`aspect-square shrink-0 flex items-center justify-center rounded-2xl select-none transition-all bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] text-[var(--rz-text-muted)] shadow-xs ${className}`}
      style={containerStyle}
      title="Application artwork unavailable"
    >
      <svg
        width={Math.round(px * 0.44)}
        height={Math.round(px * 0.44)}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="3" y="3" width="18" height="18" rx="4" />
        <path d="M8 8h5a3 3 0 0 1 0 6H8V8z" />
        <path d="M12 14l4 4" />
      </svg>
    </div>
  );
};
