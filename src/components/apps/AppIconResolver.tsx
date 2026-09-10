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
import {
  Globe,
  Terminal,
  Code2,
  Gamepad2,
  Tv,
  Image as ImageIcon,
  Cpu,
  Sparkles,
  Brush,
  Box,
  Disc3,
  Video,
  Music2,
  Camera,
  MonitorPlay,
} from "lucide-react";
import { KNOWN_APPS, resolveAppMetadata, type AppMetadata } from "./appMetadata.ts";

export { resolveAppMetadata, KNOWN_APPS, type AppMetadata };

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

/**
 * Pick a distinct fallback icon for a given application, based on category and app ID.
 * Ensures GIMP, Blender, VLC, etc. each have visually distinct placeholder artwork.
 */
function getFallbackIcon(appId: string, category: string) {
  const id = appId.toLowerCase();

  // App-specific distinct fallbacks for common non-installed apps
  if (id.includes("gimp") || id.includes("inkscape") || id.includes("krita")) return Brush;
  if (id.includes("blender")) return Box;
  if (id.includes("vlc") || id.includes("celluloid")) return Disc3;
  if (id.includes("obs")) return Video;
  if (id.includes("audacity") || id.includes("ardour") || id.includes("spotify") || id.includes("music")) return Music2;
  if (id.includes("darktable") || id.includes("shotwell") || id.includes("digikam")) return Camera;
  if (id.includes("mpv") || id.includes("totem") || id.includes("player")) return MonitorPlay;

  // Category fallbacks
  switch (category) {
    case "Internet": return Globe;
    case "Development": return Code2;
    case "Multimedia": return Tv;
    case "Graphics": return ImageIcon;
    case "Games": return Gamepad2;
    case "Utilities": return Terminal;
    case "System": return Cpu;
    default: return Sparkles;
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

  // Tier 3: Distinct geometric category fallback
  const meta = resolveAppMetadata(targetId);
  const FallbackIcon = getFallbackIcon(targetId, meta.category);

  return (
    <div
      className={`aspect-square shrink-0 flex items-center justify-center rounded-2xl select-none transition-all ${className}`}
      style={{
        ...containerStyle,
        background: `linear-gradient(135deg, ${meta.accentColor}22, ${meta.accentColor}08)`,
        color: meta.accentColor,
        border: `1.5px solid ${meta.accentColor}35`,
      }}
    >
      <FallbackIcon size={Math.round(px * 0.52)} />
    </div>
  );
};
