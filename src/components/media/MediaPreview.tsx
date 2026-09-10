import React, { useState, useEffect, useRef, useCallback } from "react";
import { Play, Pause, Volume2, VolumeX, AlertCircle } from "lucide-react";
import { determineMediaDisplayState } from "./mediaUtils.ts";

export interface MediaPreviewProps {
  poster: string;
  videoSrc?: string;
  animatedSrc?: string;
  mediaType?: "image" | "video" | "animated";
  alt: string;
  mode?: "card" | "hero";
  isHovered?: boolean;
  aspectRatio?: "16/9" | "4/3" | "1/1";
  className?: string;
  showBadge?: boolean;
  onVideoError?: (err: any) => void;
}

export const MediaPreview: React.FC<MediaPreviewProps> = ({
  poster,
  videoSrc,
  animatedSrc,
  mediaType = "image",
  alt,
  mode = "card",
  aspectRatio = "16/9",
  className = "",
  showBadge = false,
  onVideoError,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const isMountedRef = useRef(true);
  const retryTimeoutRef = useRef<number | null>(null);
  const retryCountRef = useRef(0);
  const isIntentionalPauseRef = useRef(false);

  const [isPlaying, setIsPlaying] = useState(false);
  const [isMuted, setIsMuted] = useState(true);
  const [videoError, setVideoError] = useState(false);
  const [videoLoaded, setVideoLoaded] = useState(false);
  const [prefersReducedMotion, setPrefersReducedMotion] = useState(false);

  // Check prefers-reduced-motion accessibility setting
  useEffect(() => {
    if (typeof window !== "undefined") {
      const mediaQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
      setPrefersReducedMotion(mediaQuery.matches);
      const listener = (e: MediaQueryListEvent) => setPrefersReducedMotion(e.matches);
      mediaQuery.addEventListener("change", listener);
      return () => mediaQuery.removeEventListener("change", listener);
    }
  }, []);

  // Track component mount status
  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
      if (retryTimeoutRef.current !== null) {
        clearTimeout(retryTimeoutRef.current);
        retryTimeoutRef.current = null;
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

  // Resilient autoplay function with exponential backoff retry
  const attemptPlay = useCallback(() => {
    if (!isMountedRef.current || prefersReducedMotion || videoError) return;
    const video = videoRef.current;
    if (!video) return;

    if (video.paused && !isIntentionalPauseRef.current) {
      const playPromise = video.play();
      if (playPromise !== undefined) {
        playPromise
          .then(() => {
            if (isMountedRef.current) {
              setIsPlaying(true);
              retryCountRef.current = 0;
            }
          })
          .catch((_err) => {
            if (!isMountedRef.current) return;
            setIsPlaying(false);
            // Intelligent backoff retry: 300ms, 600ms, 1200ms, up to 3000ms max (capped at 6 tries)
            if (retryCountRef.current < 6) {
              const delay = Math.min(300 * Math.pow(1.5, retryCountRef.current), 3000);
              retryCountRef.current += 1;
              if (retryTimeoutRef.current !== null) {
                clearTimeout(retryTimeoutRef.current);
              }
              retryTimeoutRef.current = window.setTimeout(attemptPlay, delay);
            }
          });
      }
    }
  }, [prefersReducedMotion, videoError]);

  // Initial playback launch when hasVideo or finalVideo updates
  useEffect(() => {
    if (!hasVideo || prefersReducedMotion || videoError) return;
    isIntentionalPauseRef.current = false;
    attemptPlay();
  }, [hasVideo, finalVideo, prefersReducedMotion, videoError, attemptPlay]);

  // Event handlers for resilient media playback
  const handleLoadedData = () => {
    setVideoLoaded(true);
    attemptPlay();
  };

  const handleCanPlay = () => {
    if (videoRef.current?.paused && !isIntentionalPauseRef.current) {
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
    // If pause occurred without user pressing pause, schedule automatic recovery
    if (!isIntentionalPauseRef.current && hasVideo && !prefersReducedMotion && !videoError) {
      setIsPlaying(false);
      if (retryTimeoutRef.current !== null) {
        clearTimeout(retryTimeoutRef.current);
      }
      retryTimeoutRef.current = window.setTimeout(attemptPlay, 250);
    } else {
      setIsPlaying(false);
    }
  };

  const handlePlay = () => {
    setIsPlaying(true);
    retryCountRef.current = 0;
  };

  const handleWaiting = () => {
    // Normal chunk buffering — do NOT treat as an error or pause
  };

  const handleStalled = () => {
    // Normal network pipeline stall — do NOT treat as an error
  };

  const handleVideoError = (err: any) => {
    // Only permanently fall back on genuine fatal video errors
    const video = videoRef.current;
    if (video?.error) {
      setVideoError(true);
      setIsPlaying(false);
      if (onVideoError) {
        onVideoError(err);
      }
    }
  };

  const togglePlay = (e: React.MouseEvent) => {
    e.stopPropagation();
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      isIntentionalPauseRef.current = false;
      attemptPlay();
    } else {
      isIntentionalPauseRef.current = true;
      video.pause();
      setIsPlaying(false);
    }
  };

  const toggleMute = (e: React.MouseEvent) => {
    e.stopPropagation();
    const video = videoRef.current;
    if (!video) return;
    video.muted = !video.muted;
    setIsMuted(video.muted);
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
      <img
        src={hasAnimatedImage ? finalAnimated : finalPoster}
        alt={alt}
        className="absolute inset-0 w-full h-full object-cover pointer-events-none"
        loading="lazy"
      />

      {/* 
        2. Video Player: Mounted on top of poster.
        Autoplays muted in loop with zero scroll-pause eviction.
      */}
      {hasVideo && (
        <video
          ref={videoRef}
          src={finalVideo}
          muted={isMuted}
          loop
          playsInline
          autoPlay
          preload="auto"
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
        <div className="absolute bottom-2.5 right-2.5 z-10 flex items-center gap-1 px-2 py-0.5 rounded text-[10px] bg-black/70 text-white/80 border border-white/10 backdrop-blur-sm">
          <AlertCircle size={11} className="text-amber-400" />
          <span>Poster Fallback</span>
        </div>
      )}

      {/* Hero Mode Controls Overlay */}
      {mode === "hero" && hasVideo && (
        <div className="absolute bottom-3 right-3 z-20 flex items-center gap-2 pointer-events-auto">
          <button
            onClick={togglePlay}
            className="p-2 rounded-full backdrop-blur-md bg-black/60 hover:bg-black/80 text-white/90 border border-white/15 transition-all shadow-lg hover:scale-105 active:scale-95 cursor-pointer"
            title={isPlaying ? "Pause preview" : "Play preview"}
            type="button"
          >
            {isPlaying ? <Pause size={14} /> : <Play size={14} className="translate-x-0.5" />}
          </button>
          <button
            onClick={toggleMute}
            className="p-2 rounded-full backdrop-blur-md bg-black/60 hover:bg-black/80 text-white/90 border border-white/15 transition-all shadow-lg hover:scale-105 active:scale-95 cursor-pointer"
            title={isMuted ? "Unmute" : "Mute"}
            type="button"
          >
            {isMuted ? <VolumeX size={14} /> : <Volume2 size={14} />}
          </button>
        </div>
      )}
    </div>
  );
};
