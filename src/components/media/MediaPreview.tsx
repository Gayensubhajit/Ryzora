import React, { useState, useEffect, useRef } from "react";
import { Play, Pause, Volume2, VolumeX, AlertCircle } from "lucide-react";

// Global limiter for concurrent active video decoders in cards to prevent GPU strain
const activeVideoElements = new Set<HTMLVideoElement>();
const MAX_CONCURRENT_VIDEOS = 2;

function registerActiveVideo(video: HTMLVideoElement) {
  if (activeVideoElements.size >= MAX_CONCURRENT_VIDEOS) {
    const oldest = activeVideoElements.values().next().value;
    if (oldest && oldest !== video) {
      oldest.pause();
      activeVideoElements.delete(oldest);
    }
  }
  activeVideoElements.add(video);
}

function unregisterActiveVideo(video: HTMLVideoElement) {
  activeVideoElements.delete(video);
}

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
  isHovered = false,
  aspectRatio = "16/9",
  className = "",
  showBadge = false,
  onVideoError,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);

  const [isVisible, setIsVisible] = useState(false);
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

  // IntersectionObserver to observe visibility in viewport
  useEffect(() => {
    const element = containerRef.current;
    if (!element || typeof IntersectionObserver === "undefined") {
      setIsVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        const [entry] = entries;
        setIsVisible(entry.isIntersecting);
      },
      { threshold: 0.25 }
    );

    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const normalizeUrl = (url?: string) => {
    if (!url) return "";
    if (url.startsWith("http://") || url.startsWith("https://") || url.startsWith("/") || url.startsWith("data:") || url.startsWith("blob:")) {
      return url;
    }
    return "/" + url;
  };

  const finalPoster = normalizeUrl(poster);
  const finalVideo = normalizeUrl(videoSrc);
  const finalAnimated = normalizeUrl(animatedSrc);

  // Video is active if a video source is available, it is marked video or animated, and reduced motion is not requested
  const hasVideo = Boolean(
    finalVideo &&
    (mediaType === "video" || mediaType === "animated") &&
    !prefersReducedMotion &&
    !videoError
  );

  const hasAnimatedImage = Boolean(
    !hasVideo &&
    animatedSrc &&
    mediaType === "animated" &&
    !prefersReducedMotion
  );

  // Playback control for Card Mode (muted loop when visible, limited to MAX_CONCURRENT_VIDEOS = 2)
  useEffect(() => {
    if (mode !== "card" || !hasVideo) return;
    const video = videoRef.current;
    if (!video) return;

    if (isVisible && !prefersReducedMotion) {
      registerActiveVideo(video);
      video.play().then(() => setIsPlaying(true)).catch(() => {
        setIsPlaying(false);
      });
    } else {
      video.pause();
      setIsPlaying(false);
      unregisterActiveVideo(video);
    }

    return () => {
      unregisterActiveVideo(video);
    };
  }, [mode, hasVideo, isVisible, prefersReducedMotion]);

  // Hover prioritization: hovering immediately claims playback priority
  useEffect(() => {
    if (mode !== "card" || !hasVideo) return;
    const video = videoRef.current;
    if (!video || !isVisible || prefersReducedMotion) return;

    if (isHovered) {
      registerActiveVideo(video);
      video.play().then(() => setIsPlaying(true)).catch(() => {});
    }
  }, [mode, hasVideo, isVisible, isHovered, prefersReducedMotion]);

  // Hero Mode: Autoplays muted loop when visible
  useEffect(() => {
    if (mode !== "hero" || !hasVideo) return;
    const video = videoRef.current;
    if (!video) return;

    if (isVisible) {
      video.play().then(() => setIsPlaying(true)).catch(() => {
        setIsPlaying(false);
      });
    } else {
      video.pause();
      setIsPlaying(false);
    }
  }, [mode, hasVideo, isVisible]);

  const togglePlay = (e: React.MouseEvent) => {
    e.stopPropagation();
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      video.play().then(() => setIsPlaying(true)).catch(() => {});
    } else {
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

  const handleVideoError = (err: any) => {
    setVideoError(true);
    if (videoRef.current) {
      unregisterActiveVideo(videoRef.current);
    }
    if (onVideoError) {
      onVideoError(err);
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
      {/* Fallback & Poster Image (or Animated GIF fallback) */}
      <img
        src={hasAnimatedImage && isVisible ? finalAnimated : finalPoster}
        alt={alt}
        className={`w-full h-full object-cover transition-opacity duration-500 ${
          hasVideo && videoLoaded && isPlaying ? "opacity-0" : "opacity-100"
        }`}
        loading="lazy"
      />

      {/* Video Player */}
      {hasVideo && (
        <video
          ref={videoRef}
          src={finalVideo}
          poster={finalPoster}
          muted={isMuted}
          loop
          playsInline
          autoPlay
          preload="auto"
          onLoadedData={() => setVideoLoaded(true)}
          onError={handleVideoError}
          className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-500 ${
            videoLoaded && isPlaying ? "opacity-100" : "opacity-0 pointer-events-none"
          }`}
        />
      )}

      {/* Media Type Badge */}
      {showBadge && (
        <div className="absolute top-2.5 left-2.5 z-10 flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium tracking-wide uppercase shadow-sm backdrop-blur-md bg-black/55 text-white/90 border border-white/10">
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

      {/* Video decode failure notice (quiet pill) */}
      {videoError && (
        <div className="absolute bottom-2.5 right-2.5 z-10 flex items-center gap-1 px-2 py-0.5 rounded text-[10px] bg-black/60 text-white/70 border border-white/10 backdrop-blur-sm">
          <AlertCircle size={11} className="text-amber-400" />
          <span>Poster Fallback</span>
        </div>
      )}

      {/* Hero Mode Controls Overlay */}
      {mode === "hero" && hasVideo && (
        <div className="absolute bottom-3 right-3 z-20 flex items-center gap-2">
          <button
            onClick={togglePlay}
            className="p-2 rounded-full backdrop-blur-md bg-black/60 hover:bg-black/80 text-white/90 border border-white/15 transition-all shadow-lg hover:scale-105 active:scale-95"
            title={isPlaying ? "Pause preview" : "Play preview"}
            type="button"
          >
            {isPlaying ? <Pause size={14} /> : <Play size={14} className="translate-x-0.5" />}
          </button>
          <button
            onClick={toggleMute}
            className="p-2 rounded-full backdrop-blur-md bg-black/60 hover:bg-black/80 text-white/90 border border-white/15 transition-all shadow-lg hover:scale-105 active:scale-95"
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
