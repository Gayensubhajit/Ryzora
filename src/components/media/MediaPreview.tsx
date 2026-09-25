import React, { useRef, useState, useEffect, useCallback, memo } from "react";
import { AlertCircle, Film } from "lucide-react";
import {
  determineMediaDisplayState,
  globalVideoLimiter,
  PlaybackPriority,
} from "./mediaUtils.ts";

export interface MediaPreviewProps {
  poster?: string;
  videoSrc?: string;
  animatedSrc?: string;
  mediaType?: "video" | "animated" | "image";
  alt?: string;
  mode?: "card" | "hero";
  isHovered?: boolean;
  aspectRatio?: "16/9" | "4/3" | "square";
  showBadge?: boolean;
  onVideoError?: (err: any) => void;
  className?: string;
}

export const MediaPreview: React.FC<MediaPreviewProps> = memo(({
  poster = "",
  videoSrc,
  animatedSrc,
  mediaType,
  alt = "Media preview",
  mode = "card",
  isHovered = false,
  aspectRatio = "16/9",
  showBadge = false,
  onVideoError,
  className = "",
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted] = useState(true);
  const [videoLoaded, setVideoLoaded] = useState(false);
  const [videoError, setVideoError] = useState(false);
  const [imgFailed, setImgFailed] = useState(false);
  const [isInViewport, setIsInViewport] = useState(false);

  const isMountedRef = useRef(true);
  const isIntentionalPauseRef = useRef(false);
  const retryTimeoutRef = useRef<number | null>(null);

  const [prefersReducedMotion, setPrefersReducedMotion] = useState<boolean>(() => {
    if (typeof window === "undefined" || !window.matchMedia) return false;
    return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  });

  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;
    const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handler = (e: MediaQueryListEvent) => setPrefersReducedMotion(e.matches);
    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, []);

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      if (retryTimeoutRef.current !== null) {
        clearTimeout(retryTimeoutRef.current);
        retryTimeoutRef.current = null;
      }
      if (videoRef.current) {
        globalVideoLimiter.releasePlayback(videoRef.current);
      }
    };
  }, []);

  const displayState = determineMediaDisplayState({
    poster,
    videoSrc,
    animatedSrc,
    mediaType,
    prefersReducedMotion,
    videoError,
  });

  const { hasVideo, hasAnimatedImage, finalPoster, finalVideo, finalAnimated } = displayState;

  // Viewport IntersectionObserver: pause off-screen, play in-viewport
  useEffect(() => {
    if (!hasVideo || typeof IntersectionObserver === "undefined") {
      setIsInViewport(true);
      return;
    }

    const el = containerRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        if (entry) {
          setIsInViewport(entry.isIntersecting);
        }
      },
      { threshold: 0.15 }
    );

    observer.observe(el);
    return () => {
      observer.disconnect();
    };
  }, [hasVideo]);

  const priority: PlaybackPriority = mode === "hero" ? "hero" : isHovered ? "hover" : "normal";

  // Playback request through the priority-aware limiter
  const attemptPlay = useCallback(() => {
    if (!isMountedRef.current || prefersReducedMotion || videoError || !isInViewport) return;
    const video = videoRef.current;
    if (!video) return;

    if (isIntentionalPauseRef.current) return;
    if (mode === "card" && !isHovered) return;

    const req = globalVideoLimiter.requestPlayback(video, priority);
    if (!req.allowed) {
      // Limiter reached capacity for this priority — remain paused with poster visible underneath
      setIsPlaying(false);
      return;
    }

    const playPromise = video.play();
    if (playPromise !== undefined) {
      playPromise
        .then(() => {
          if (isMountedRef.current) {
            setIsPlaying(true);
          }
        })
        .catch(() => {
          if (isMountedRef.current) {
            setIsPlaying(false);
          }
        });
    }
  }, [prefersReducedMotion, videoError, isInViewport, priority]);

  // Viewport & priority synchronization
  useEffect(() => {
    if (!hasVideo) return;
    const video = videoRef.current;
    if (!video) return;

    const shouldPlay = isInViewport && !isIntentionalPauseRef.current && (mode === "hero" || isHovered);

    if (shouldPlay) {
      attemptPlay();
    } else {
      globalVideoLimiter.releasePlayback(video);
      try {
        video.pause();
      } catch {}
      setIsPlaying(false);
    }
  }, [isInViewport, hasVideo, priority, attemptPlay, mode, isHovered]);

  // Window visibility listener: resume when window is focused
  useEffect(() => {
    const handleVisChange = () => {
      if (document.visibilityState === "visible" && isInViewport && !isIntentionalPauseRef.current) {
        attemptPlay();
      } else if (document.visibilityState !== "visible" && videoRef.current) {
        globalVideoLimiter.releasePlayback(videoRef.current);
        try {
          videoRef.current.pause();
        } catch {}
        setIsPlaying(false);
      }
    };

    document.addEventListener("visibilitychange", handleVisChange);
    return () => document.removeEventListener("visibilitychange", handleVisChange);
  }, [isInViewport, attemptPlay]);

  // Media event handlers
  const handleLoadedData = () => {
    setVideoLoaded(true);
    if (isInViewport && !isIntentionalPauseRef.current) {
      attemptPlay();
    }
  };

  const handleCanPlay = () => {
    if (isInViewport && videoRef.current?.paused && !isIntentionalPauseRef.current) {
      attemptPlay();
    }
  };

  const handleEnded = () => {
    const video = videoRef.current;
    if (video) {
      video.currentTime = 0;
      attemptPlay();
    }
  };

  const handlePause = () => {
    if (!isMountedRef.current) return;
    setIsPlaying(false);
    if (videoRef.current) {
      globalVideoLimiter.releasePlayback(videoRef.current);
    }
  };

  const handlePlay = () => {
    setIsPlaying(true);
  };

  const handleWaiting = () => {
    // Buffering chunk: retain video element without tearing down
  };

  const handleStalled = () => {
    // Pipeline stall: retain video element
  };

  const handleVideoError = (err: any) => {
    const video = videoRef.current;
    if (video?.error) {
      setVideoError(true);
      setIsPlaying(false);
      if (video) {
        globalVideoLimiter.releasePlayback(video);
      }
      if (onVideoError) {
        onVideoError(err);
      }
    }
  };

  const aspectClass =
    aspectRatio === "16/9"
      ? "aspect-video"
      : aspectRatio === "4/3"
      ? "aspect-[4/3]"
      : "aspect-square";

  return (
    <div
      ref={containerRef}
      className={`relative w-full overflow-hidden select-none bg-[var(--surface-base,#141417)] ${aspectClass} ${className}`}
    >
      {/* 
        1. Base Layer: Poster Image (always mounted underneath)
        Ensures zero black/empty flashes while video is buffering or if decode fails.
      */}
      {(!imgFailed && (finalPoster || (hasAnimatedImage && finalAnimated))) ? (
        <img
          src={hasAnimatedImage ? finalAnimated : finalPoster}
          alt={alt}
          onError={() => setImgFailed(true)}
          className="absolute inset-0 w-full h-full object-cover pointer-events-none"
          loading="lazy"
        />
      ) : (
        <div className="absolute inset-0 w-full h-full flex flex-col items-center justify-center bg-[var(--rz-surface-elevated,#1e1e24)] text-[var(--rz-text-muted,#71717a)] p-4 text-center">
          <Film className="w-8 h-8 mb-2 opacity-40 stroke-[1.5]" />
          <span className="text-xs font-medium max-w-[200px] truncate">{alt || "Video preview"}</span>
        </div>
      )}

      {/* 
        2. Video Player: Mounted on top of poster.
        Viewport-aware priority decoding with zero teardown.
      */}
      {hasVideo && (
        <video
          ref={videoRef}
          src={finalVideo}
          poster={finalPoster || undefined}
          muted={isMuted}
          loop
          playsInline
          preload="metadata"
          onLoadedData={handleLoadedData}
          onCanPlay={handleCanPlay}
          onEnded={handleEnded}
          onPause={handlePause}
          onPlay={handlePlay}
          onWaiting={handleWaiting}
          onStalled={handleStalled}
          onError={handleVideoError}
          className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-300 pointer-events-none ${
            videoLoaded && isPlaying ? "opacity-100" : "opacity-0"
          }`}
        />
      )}

      {/* Media Type Badge */}
      {showBadge && (
        <div className="absolute top-2.5 left-2.5 z-10 flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/60 text-white/90 border border-white/10">
          {mediaType === "video" && (
            <>
              <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
              <span>Video</span>
            </>
          )}
          {mediaType === "animated" && (
            <>
              <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
              <span>Animated</span>
            </>
          )}
          {mediaType === "image" && <span>Static</span>}
        </div>
      )}

      {/* Video decode failure notice */}
      {videoError && (
        <div className="absolute bottom-2.5 right-2.5 z-10 flex flex-col items-end gap-0.5 px-2.5 py-1.5 rounded-lg text-[11px] font-medium bg-[var(--rz-surface-elevated)]/95 text-[var(--rz-text)] border border-[var(--rz-border-strong)] shadow-lg backdrop-blur-md">
          <div className="flex items-center gap-1.5 text-amber-400 font-semibold">
            <AlertCircle size={12} className="shrink-0" />
            <span>Video preview unavailable</span>
          </div>
          <span className="text-[10px] text-[var(--rz-text-muted)]">Open SDDM Test to preview the original</span>
        </div>
      )}


    </div>
  );
});
