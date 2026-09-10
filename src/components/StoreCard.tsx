import React, { useState, useEffect } from "react";
import {
  Sparkles,
  ShieldCheck,
  Star,
  Download,
  Eye,
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
  getConciseCompatibility,
} from "./catalogue/catalogueUtils";
import { InstalledBadge } from "./catalogue/InstalledBadge";
import { CompatibilityBadge } from "./catalogue/CompatibilityBadge";
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
  const { setSelectedPackage, installedPackages, systemInfo } = useApp();
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
  const isSddmLogin = isLoginScreen(packageItem);
  const currentWm = systemInfo?.window_manager?.toLowerCase();
  const compatibility = getConciseCompatibility(packageItem, currentWm);

  const hasImage = Boolean(
    packageItem.hero_image && packageItem.hero_image.trim().length > 0 && !imgError
  );

  return (
    <div
      onClick={handleClick}
      onMouseEnter={() => setIsHovered(true)}
      onMouseOver={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onFocus={() => setIsHovered(true)}
      onBlur={() => setIsHovered(false)}
      className="store-card group relative overflow-hidden rounded-xl cursor-pointer select-none border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] hover:border-[var(--rz-border-strong)] transition-all duration-200 shadow-xs hover:shadow-md flex flex-col"
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === "Enter" && handleClick()}
      aria-label={`${packageItem.title} — ${subtype} by ${authorName}`}
    >
      {/* ── Artwork Container (Hero) ── */}
      <div className="aspect-video w-full relative overflow-hidden bg-[var(--rz-surface-elevated)]">
        {hasImage ? (
          <MediaPreview
            poster={packageItem.preview_poster_url || packageItem.hero_image}
            videoSrc={packageItem.preview_video_url}
            animatedSrc={packageItem.lockscreen?.media.preview_animated}
            mediaType={packageItem.media_type || "image"}
            alt={packageItem.title}
            mode="card"
            isHovered={isHovered}
            aspectRatio="16/9"
            showBadge={false}
            className="transition-transform duration-500 ease-out group-hover:scale-105"
          />
        ) : (
          /* Sleek Generative Tech Artwork Fallback */
          <div
            className={`w-full h-full flex flex-col justify-between p-3 relative overflow-hidden bg-gradient-to-br ${getGenerativeGradient(
              packageItem.id
            )} transition-transform duration-500 ease-out group-hover:scale-105`}
          >
            {/* Tech grid texture */}
            <div
              className="absolute inset-0 opacity-15 pointer-events-none"
              style={{
                backgroundImage:
                  "radial-gradient(rgba(255,255,255,0.4) 1px, transparent 1px)",
                backgroundSize: "12px 12px",
              }}
            />

            {/* Top row: category icon and subtle type */}
            <div className="flex items-center justify-between z-10">
              <div className="p-1 rounded-md bg-black/40 border border-white/10 backdrop-blur-xs">
                {getCategoryIcon(packageItem.category || packageItem.package_type)}
              </div>
              <span className="text-[10px] font-mono text-white/60 tracking-wider">
                {subtype}
              </span>
            </div>

            {/* Bottom visual: preview title and command snippet */}
            <div className="z-10 mt-auto">
              <div className="text-[9px] font-mono text-white/50 truncate">
                ~/.config/{packageItem.title.toLowerCase().replace(/\s+/g, "-")}
              </div>
              <div className="text-xs font-semibold text-white/90 truncate tracking-tight mt-0.5">
                {packageItem.title}
              </div>
            </div>
          </div>
        )}

        {/* Subtle gradient vignette at the bottom of artwork */}
        <div className="absolute inset-0 bg-gradient-to-t from-black/50 via-transparent to-transparent pointer-events-none" />

        {/* Top-Left Trust & Login badges (Normal state) */}
        <div className="absolute top-2 left-2 flex items-center gap-1.5 z-10">
          {isSddmLogin && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-mono font-bold uppercase tracking-wider bg-amber-500/90 text-black shadow-xs">
              SDDM
            </span>
          )}
          {packageItem.media_type === "video" && (
            <span className="px-2 py-0.5 rounded-full text-[9px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/60 text-white/95 border border-white/10 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
              Video
            </span>
          )}
          {packageItem.media_type === "animated" && (
            <span className="px-2 py-0.5 rounded-full text-[9px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/60 text-white/95 border border-white/10 flex items-center gap-1">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
              Animated
            </span>
          )}
          {packageItem.trust_tier === "official" && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase bg-amber-500/25 text-amber-200 border border-amber-400/35 backdrop-blur-xs flex items-center gap-0.5">
              <Sparkles className="w-2.5 h-2.5" />
              Official
            </span>
          )}
          {packageItem.trust_tier === "verified" && !isSddmLogin && (
            <span className="px-1.5 py-0.5 rounded text-[9px] font-bold uppercase bg-sky-500/25 text-sky-200 border border-sky-400/35 backdrop-blur-xs flex items-center gap-0.5">
              <ShieldCheck className="w-2.5 h-2.5" />
              Verified
            </span>
          )}
        </div>

        {/* Top-Right Installation State Badge (Normal state) */}
        <div className="absolute top-1.5 right-1.5 z-10">
          <InstalledBadge
            isInstalled={isInstalled}
            isUpdateAvailable={isUpdateAvailable}
          />
        </div>

        {/* ── Hover Overlay: Revealing Secondary Information ── */}
        <div
          className={[
            "absolute inset-0 bg-black/55 backdrop-blur-xs flex flex-col justify-between p-2.5 z-20 pointer-events-none transition-opacity duration-200",
            isHovered ? "opacity-100" : "opacity-0 group-hover:opacity-100",
          ].join(" ")}
        >
          {/* Top row of hover overlay: compatibility */}
          <div className="flex items-center justify-between gap-1">
            <CompatibilityBadge
              label={compatibility.label}
              isWarning={compatibility.isWarning}
            />
          </div>

          {/* Center action button */}
          <div className="flex items-center justify-center">
            <span className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-white/90 text-black shadow-lg flex items-center gap-1.5 font-sans">
              <Eye className="w-3.5 h-3.5 text-black" />
              Details
            </span>
          </div>

          {/* Bottom row of hover overlay: metrics & rating */}
          <div className="flex items-center justify-between text-[11px] font-medium text-white/90 pt-1">
            <span className="flex items-center gap-1 font-mono">
              <Download className="w-3 h-3 text-white/70" />
              {formatDownloads(packageItem.downloads)}
            </span>
            {packageItem.rating > 0 && (
              <span className="flex items-center gap-1 font-mono">
                <Star className="w-3 h-3 text-amber-400 fill-amber-400" />
                {packageItem.rating.toFixed(1)}
              </span>
            )}
          </div>
        </div>
      </div>

      {/* ── Card Footer: Art-First Title & Subtitle (Always Visible) ── */}
      <div className="p-2.5 bg-[var(--rz-surface)] border-t border-[var(--rz-border-subtle)]/70 flex-1 flex flex-col justify-center">
        {/* Primary Title */}
        <div className="text-xs sm:text-[13px] font-bold text-[var(--rz-text)] truncate leading-tight tracking-tight">
          {packageItem.title}
        </div>

        {/* Secondary Subtitle: Type · Author */}
        <div className="flex items-center justify-between gap-1.5 mt-1 text-[11px] text-[var(--rz-text-secondary)]">
          <span className="truncate">
            <strong className="font-semibold text-[var(--rz-text)]">{subtype}</strong>
            <span className="mx-1 text-[var(--rz-text-muted)]">·</span>
            <span>{authorName}</span>
          </span>
          {packageItem.color_palette && packageItem.color_palette.length > 0 && (
            <div className="flex items-center gap-0.5 shrink-0">
              {packageItem.color_palette.slice(0, 3).map((c, i) => (
                <span
                  key={i}
                  className="w-1.5 h-1.5 rounded-full border border-black/20"
                  style={{ backgroundColor: c }}
                  title={c}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
