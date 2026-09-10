import React, { useState, useEffect } from "react";
import {
  Sparkles,
  ShieldCheck,
  Sliders,
  Star,
  Download,
  Layers,
  Terminal,
  Palette,
  Image as ImageIcon,
  Lock,
  Layout,
  Package as PackageIcon,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageItem } from "../types";
import {
  getPackageSubtype,
  formatDownloads,
  isLoginScreen,
} from "./catalogue/catalogueUtils";
import { InstalledBadge } from "./catalogue/InstalledBadge";
import { MediaPreview } from "./media/MediaPreview";

interface StoreCardProps {
  packageItem: PackageItem;
}

/** Generates deterministic gradient accents for fallback artwork */
function getGenerativeGradient(id: string): string {
  const hash = id.split("").reduce((acc, char) => acc + char.charCodeAt(0), 0);
  const gradients = [
    "from-slate-900 via-indigo-950 to-slate-900",
    "from-zinc-900 via-purple-950 to-zinc-950",
    "from-stone-900 via-neutral-900 to-cyan-950",
    "from-slate-950 via-teal-950 to-slate-900",
    "from-neutral-950 via-rose-950 to-zinc-900",
    "from-gray-900 via-sky-950 to-slate-950",
  ];
  return gradients[hash % gradients.length];
}

/** Category icon helper for fallback artwork */
function getCategoryIcon(cat: string) {
  const c = cat.toLowerCase();
  if (c.includes("lock")) return <Lock className="w-4 h-4 text-emerald-400" />;
  if (c.includes("rice")) return <Layout className="w-4 h-4 text-violet-400" />;
  if (c.includes("theme")) return <Palette className="w-4 h-4 text-pink-400" />;
  if (c.includes("wall")) return <ImageIcon className="w-4 h-4 text-amber-400" />;
  if (c.includes("bar")) return <Layers className="w-4 h-4 text-sky-400" />;
  if (c.includes("term") || c.includes("fetch")) return <Terminal className="w-4 h-4 text-cyan-400" />;
  return <PackageIcon className="w-4 h-4 text-zinc-400" />;
}

export const StoreCard: React.FC<StoreCardProps> = ({ packageItem }) => {
  const { setSelectedPackage, installedPackages } = useApp();
  const [imgError, setImgError] = useState(false);
  const [isHovered, setIsHovered] = useState(false);

  useEffect(() => {
    setImgError(false);
  }, [packageItem.hero_image]);

  const installedRecord = installedPackages.find((p) => p.package_id === packageItem.id);
  const isInstalled = !!installedRecord;
  const isUpdateAvailable =
    isInstalled && installedRecord
      ? parseFloat(packageItem.version) > parseFloat(installedRecord.version)
      : false;

  const handleClick = () => setSelectedPackage(packageItem);

  const subtype = getPackageSubtype(packageItem);
  const authorName = packageItem.author?.name || "Community";
  const isCustomizable = Boolean(
    packageItem.customizable ||
    (packageItem.lockscreen?.config_schema?.variants && packageItem.lockscreen.config_schema.variants.length > 0) ||
    (packageItem.lockscreen?.config_schema?.options && Object.keys(packageItem.lockscreen.config_schema.options).length > 0)
  );

  const isSddmLogin = isLoginScreen(packageItem);

  const posterSrc =
    packageItem.preview_poster_url ||
    packageItem.hero_image ||
    packageItem.lockscreen?.media.poster ||
    "";
  const videoSrc =
    packageItem.preview_video_url ||
    packageItem.preview_video ||
    packageItem.lockscreen?.media.preview_video;
  const animatedSrc =
    packageItem.preview_animated ||
    packageItem.lockscreen?.media.preview_animated;
  const mediaType =
    packageItem.media_type ||
    (videoSrc ? "video" : animatedSrc ? "animated" : "image");

  const hasImage = !imgError && (posterSrc || videoSrc || animatedSrc);

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
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className="group relative flex flex-col h-full rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] overflow-hidden cursor-pointer transition-all duration-300 ease-out hover:-translate-y-1.5 hover:shadow-2xl hover:shadow-black/25 hover:border-[var(--rz-accent)]/50 select-none"
    >
      {/* ── 1. Media Preview Section (16:9) ── */}
      <div className="aspect-video w-full relative overflow-hidden bg-[var(--surface-base,#141417)]">
        {hasImage ? (
          <MediaPreview
            poster={posterSrc}
            videoSrc={videoSrc}
            animatedSrc={animatedSrc}
            mediaType={mediaType}
            alt={packageItem.title}
            mode="card"
            isHovered={isHovered}
            aspectRatio="16/9"
            showBadge={false}
            onVideoError={() => {}}
            className="transition-transform duration-500 ease-out group-hover:scale-[1.03]"
          />
        ) : (
          /* Sleek Generative Tech Artwork Fallback */
          <div
            className={`w-full h-full flex flex-col justify-between p-4 relative overflow-hidden bg-gradient-to-br ${getGenerativeGradient(
              packageItem.id
            )} transition-transform duration-500 ease-out group-hover:scale-[1.03]`}
          >
            <div
              className="absolute inset-0 opacity-15 pointer-events-none"
              style={{
                backgroundImage:
                  "radial-gradient(rgba(255,255,255,0.4) 1px, transparent 1px)",
                backgroundSize: "12px 12px",
              }}
            />
            <div className="flex items-center justify-between z-10">
              <div className="p-1.5 rounded-lg bg-black/40 border border-white/10 backdrop-blur-xs">
                {getCategoryIcon(packageItem.category || packageItem.package_type)}
              </div>
              <span className="text-[11px] font-mono text-white/70 tracking-wider font-semibold">
                {subtype}
              </span>
            </div>
            <div className="z-10 mt-auto">
              <div className="text-[10px] font-mono text-white/50 truncate">
                ~/.config/{packageItem.title.toLowerCase().replace(/\s+/g, "-")}
              </div>
              <div className="text-sm font-semibold text-white/90 truncate tracking-tight mt-0.5">
                {packageItem.title}
              </div>
            </div>
          </div>
        )}

        {/* Subtle gradient vignette at the bottom of artwork */}
        <div className="absolute inset-x-0 bottom-0 h-10 bg-gradient-to-t from-black/50 to-transparent pointer-events-none" />

        {/* Top-Left Media Badges */}
        <div className="absolute top-2.5 left-2.5 flex items-center gap-1.5 z-10">
          {mediaType === "video" && (
            <span className="px-2 py-0.5 rounded-full text-[10px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/65 text-white/95 border border-white/15 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
              Video
            </span>
          )}
          {mediaType === "animated" && (
            <span className="px-2 py-0.5 rounded-full text-[10px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/65 text-white/95 border border-white/15 flex items-center gap-1.5">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
              Animated
            </span>
          )}
          {isSddmLogin && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider bg-amber-500 text-black shadow-xs">
              SDDM
            </span>
          )}
          {isCustomizable && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider bg-purple-500/30 text-purple-200 border border-purple-400/40 backdrop-blur-xs flex items-center gap-1 shadow-xs">
              <Sliders className="w-2.5 h-2.5" />
              Customizable
            </span>
          )}
          {packageItem.trust_tier === "official" && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase bg-amber-500/30 text-amber-200 border border-amber-400/40 backdrop-blur-xs flex items-center gap-0.5 shadow-xs">
              <Sparkles className="w-2.5 h-2.5" />
              Official
            </span>
          )}
          {packageItem.trust_tier === "verified" && !isSddmLogin && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase bg-sky-500/30 text-sky-200 border border-sky-400/40 backdrop-blur-xs flex items-center gap-0.5 shadow-xs">
              <ShieldCheck className="w-2.5 h-2.5" />
              Verified
            </span>
          )}
        </div>

        {/* Top-Right Installation State Badge */}
        <div className="absolute top-2.5 right-2.5 z-10">
          <InstalledBadge
            isInstalled={isInstalled}
            isUpdateAvailable={isUpdateAvailable}
          />
        </div>
      </div>

      {/* ── 2. Card Body: Marketplace Metadata & Metrics ── */}
      <div className="p-3.5 sm:p-4 flex-1 flex flex-col justify-between bg-[var(--rz-surface)]">
        {/* Title & Description */}
        <div>
          <div className="flex items-start justify-between gap-2">
            <h3 className="text-[15px] sm:text-[16px] font-bold text-[var(--rz-text)] group-hover:text-[var(--rz-accent)] transition-colors leading-snug tracking-tight truncate">
              {packageItem.title}
            </h3>
            {packageItem.color_palette && packageItem.color_palette.length > 0 && (
              <div className="flex items-center gap-1 shrink-0 pt-0.5">
                {packageItem.color_palette.slice(0, 3).map((c, i) => (
                  <span
                    key={i}
                    className="w-2 h-2 rounded-full border border-black/30"
                    style={{ backgroundColor: c }}
                    title={c}
                  />
                ))}
              </div>
            )}
          </div>

          <p className="text-[12px] sm:text-[13px] text-[var(--rz-text-secondary)] line-clamp-2 mt-1 leading-relaxed">
            {packageItem.description || `${subtype} configuration for your Linux desktop.`}
          </p>
        </div>

        {/* Bottom Row: Author + Rating & Downloads */}
        <div className="mt-3.5 pt-2.5 border-t border-[var(--rz-border-subtle)]/70 flex items-center justify-between text-xs text-[var(--rz-text-muted)]">
          <div className="flex items-center gap-1.5 truncate max-w-[55%]">
            <span className="font-semibold text-[var(--rz-text)] truncate">{authorName}</span>
            <span>·</span>
            <span className="text-[var(--rz-text-secondary)] truncate">{subtype}</span>
          </div>

          <div className="flex items-center gap-3 shrink-0 font-mono text-[11px] sm:text-xs">
            {packageItem.rating > 0 && (
              <span className="flex items-center gap-1 text-[var(--rz-text)] font-semibold">
                <Star className="w-3.5 h-3.5 text-amber-400 fill-amber-400" />
                {packageItem.rating.toFixed(1)}
              </span>
            )}
            <span className="flex items-center gap-1 text-[var(--rz-text-secondary)]">
              <Download className="w-3.5 h-3.5 text-[var(--rz-text-muted)]" />
              {formatDownloads(packageItem.downloads)}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
