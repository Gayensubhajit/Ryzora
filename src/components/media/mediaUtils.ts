import type { PackageItem } from "../../types/index.ts";

export const MAX_CONCURRENT_VIDEOS = 4;

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

export type PlaybackPriority = "hero" | "hover" | "normal";

interface RegisteredVideoItem {
  video: any;
  priority: PlaybackPriority;
  registeredAt: number;
}

export class VideoPlaybackLimiter {
  private activeItems: Map<any, RegisteredVideoItem> = new Map();
  private maxConcurrent: number;

  constructor(maxConcurrent: number = MAX_CONCURRENT_VIDEOS) {
    this.maxConcurrent = maxConcurrent;
  }

  requestPlayback(video: any, priority: PlaybackPriority = "normal"): { allowed: boolean; evicted: any | null } {
    if (this.activeItems.has(video)) {
      const item = this.activeItems.get(video)!;
      item.priority = priority;
      return { allowed: true, evicted: null };
    }

    let evicted: any | null = null;
    if (this.activeItems.size >= this.maxConcurrent) {
      // Find lowest priority candidate (normal < hover < hero), then oldest
      let candidateKey: any = null;
      let candidateItem: RegisteredVideoItem | null = null;

      const priorityWeight: Record<PlaybackPriority, number> = {
        normal: 1,
        hover: 2,
        hero: 3,
      };

      for (const [key, item] of this.activeItems.entries()) {
        if (!candidateItem) {
          candidateKey = key;
          candidateItem = item;
          continue;
        }

        const candidateScore = priorityWeight[candidateItem.priority];
        const currentScore = priorityWeight[item.priority];

        if (currentScore < candidateScore) {
          candidateKey = key;
          candidateItem = item;
        } else if (currentScore === candidateScore && item.registeredAt < candidateItem.registeredAt) {
          candidateKey = key;
          candidateItem = item;
        }
      }

      // If incoming priority is lower than or equal to lowest candidate and we cannot evict
      const incomingScore = priorityWeight[priority];
      if (candidateItem && priorityWeight[candidateItem.priority] > incomingScore) {
        return { allowed: false, evicted: null };
      }

      if (candidateKey) {
        if (typeof candidateKey.pause === "function") {
          try {
            candidateKey.pause();
          } catch {}
        }
        this.activeItems.delete(candidateKey);
        evicted = candidateKey;
      }
    }

    this.activeItems.set(video, {
      video,
      priority,
      registeredAt: Date.now(),
    });

    return { allowed: true, evicted };
  }

  // Backward compatibility alias for register()
  register(video: any): { evicted: any | null } {
    const res = this.requestPlayback(video, "normal");
    return { evicted: res.evicted };
  }

  releasePlayback(video: any): boolean {
    return this.activeItems.delete(video);
  }

  // Backward compatibility alias for unregister()
  unregister(video: any): boolean {
    return this.releasePlayback(video);
  }

  getActiveCount(): number {
    return this.activeItems.size;
  }

  has(video: any): boolean {
    return this.activeItems.has(video);
  }

  clear(): void {
    this.activeItems.clear();
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
