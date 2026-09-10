import type { PackageItem } from "../../types/index.ts";

export const MAX_CONCURRENT_VIDEOS = 2;

export function normalizeMediaUrl(url?: string): string {
  if (!url) return "";
  if (
    url.startsWith("http://") ||
    url.startsWith("https://") ||
    url.startsWith("/") ||
    url.startsWith("data:") ||
    url.startsWith("blob:")
  ) {
    return url;
  }
  return "/" + url;
}

export class VideoPlaybackLimiter {
  private activeElements: Set<any> = new Set();
  private maxConcurrent: number;

  constructor(maxConcurrent: number = MAX_CONCURRENT_VIDEOS) {
    this.maxConcurrent = maxConcurrent;
  }

  register(video: any): { evicted: any | null } {
    let evicted: any | null = null;
    if (this.activeElements.size >= this.maxConcurrent) {
      const oldest = this.activeElements.values().next().value;
      if (oldest && oldest !== video) {
        if (typeof oldest.pause === "function") {
          try {
            oldest.pause();
          } catch {}
        }
        this.activeElements.delete(oldest);
        evicted = oldest;
      }
    }
    this.activeElements.add(video);
    return { evicted };
  }

  unregister(video: any): boolean {
    return this.activeElements.delete(video);
  }

  getActiveCount(): number {
    return this.activeElements.size;
  }

  has(video: any): boolean {
    return this.activeElements.has(video);
  }

  clear(): void {
    this.activeElements.clear();
  }
}

export const globalVideoLimiter = new VideoPlaybackLimiter(MAX_CONCURRENT_VIDEOS);

export interface MediaDisplayOptions {
  poster: string;
  videoSrc?: string;
  animatedSrc?: string;
  mediaType?: "image" | "video" | "animated";
  prefersReducedMotion?: boolean;
  videoError?: boolean;
}

export interface MediaDisplayState {
  hasVideo: boolean;
  hasAnimatedImage: boolean;
  finalPoster: string;
  finalVideo: string;
  finalAnimated: string;
  effectiveDisplay: "video" | "animated" | "poster";
}

export function determineMediaDisplayState(options: MediaDisplayOptions): MediaDisplayState {
  const finalPoster = normalizeMediaUrl(options.poster);
  const finalVideo = normalizeMediaUrl(options.videoSrc);
  const finalAnimated = normalizeMediaUrl(options.animatedSrc);
  const mediaType = options.mediaType || "image";
  const prefersReducedMotion = Boolean(options.prefersReducedMotion);
  const videoError = Boolean(options.videoError);

  const hasVideo = Boolean(
    finalVideo &&
    (mediaType === "video" || mediaType === "animated") &&
    !prefersReducedMotion &&
    !videoError
  );

  const hasAnimatedImage = Boolean(
    !hasVideo &&
    finalAnimated &&
    mediaType === "animated" &&
    !prefersReducedMotion
  );

  let effectiveDisplay: "video" | "animated" | "poster" = "poster";
  if (hasVideo) {
    effectiveDisplay = "video";
  } else if (hasAnimatedImage) {
    effectiveDisplay = "animated";
  } else {
    effectiveDisplay = "poster";
  }

  return {
    hasVideo,
    hasAnimatedImage,
    finalPoster,
    finalVideo,
    finalAnimated,
    effectiveDisplay,
  };
}

export function resolvePackageMedia(pkg: PackageItem) {
  const poster =
    pkg.preview_poster_url ||
    pkg.hero_image ||
    pkg.lockscreen?.media?.poster ||
    "";
  const videoSrc =
    pkg.preview_video_url ||
    pkg.preview_video ||
    pkg.lockscreen?.media?.preview_video ||
    undefined;
  const animatedSrc =
    pkg.preview_animated ||
    pkg.lockscreen?.media?.preview_animated ||
    undefined;
  const mediaType =
    (pkg.media_type as "image" | "video" | "animated") ||
    (videoSrc ? "video" : animatedSrc ? "animated" : "image");

  return {
    poster,
    videoSrc,
    animatedSrc,
    mediaType,
    isVideo: mediaType === "video",
    isAnimated: mediaType === "animated",
    isStatic: mediaType === "image" || (!videoSrc && !animatedSrc),
  };
}
