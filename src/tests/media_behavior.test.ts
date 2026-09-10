import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import {
  MAX_CONCURRENT_VIDEOS,
  VideoPlaybackLimiter,
  determineMediaDisplayState,
  resolvePackageMedia,
  normalizeMediaUrl,
} from "../components/media/mediaUtils.ts";
import { getCatalogueLockScreens } from "../providers/qylockProvider.ts";

test("Media Behavior 1: VIDEO package with valid preview_video produces video display state", () => {
  const catalogue = getCatalogueLockScreens();
  const dogSamurai = catalogue.find((p) => p.id === "dog-samurai");
  assert.ok(dogSamurai, "dog-samurai package must exist in canonical catalogue");

  const media = resolvePackageMedia(dogSamurai);
  assert.equal(media.mediaType, "video");
  assert.equal(media.isVideo, true);
  assert.ok(media.videoSrc, "dog-samurai must have videoSrc defined");
  assert.ok(media.poster, "dog-samurai must have poster defined");

  const displayState = determineMediaDisplayState({
    poster: media.poster,
    videoSrc: media.videoSrc,
    mediaType: media.mediaType,
    prefersReducedMotion: false,
    videoError: false,
  });

  assert.equal(displayState.hasVideo, true);
  assert.equal(displayState.effectiveDisplay, "video");
  assert.ok(displayState.finalVideo.startsWith("/"), "Video URL must be normalized with leading slash");
  assert.ok(displayState.finalPoster.startsWith("/"), "Poster URL must be normalized with leading slash");

  // Verify asset physically exists in repository / public directory
  const relPath = displayState.finalVideo.replace(/^\//, "");
  assert.ok(
    fs.existsSync(path.resolve("public", relPath)) || fs.existsSync(path.resolve(relPath)),
    `Video file ${relPath} must exist on disk`
  );
});

test("Media Behavior 2: STATIC package resolves strictly to static poster without video", () => {
  const catalogue = getCatalogueLockScreens();
  const sunlitField = catalogue.find((p) => p.id === "field");
  assert.ok(sunlitField, "field package must exist in canonical catalogue");

  const media = resolvePackageMedia(sunlitField);
  assert.equal(media.mediaType, "image");
  assert.equal(media.isVideo, false);
  assert.equal(media.isStatic, true);
  assert.equal(media.videoSrc, undefined);
  assert.ok(media.poster, "field must have poster defined");

  const displayState = determineMediaDisplayState({
    poster: media.poster,
    videoSrc: media.videoSrc,
    mediaType: media.mediaType,
    prefersReducedMotion: false,
    videoError: false,
  });

  assert.equal(displayState.hasVideo, false);
  assert.equal(displayState.effectiveDisplay, "poster");
  assert.equal(displayState.finalVideo, "");
  assert.ok(displayState.finalPoster.length > 0, "Poster URL must be valid");

  // Verify poster physically exists on disk
  const relPath = displayState.finalPoster.replace(/^\//, "");
  assert.ok(
    fs.existsSync(path.resolve("public", relPath)) || fs.existsSync(path.resolve(relPath)),
    `Poster file ${relPath} must exist on disk`
  );
});

test("Media Behavior 3: Graceful fallback from failed video to static poster", () => {
  const displayState = determineMediaDisplayState({
    poster: "/previews/lockscreens/dog-samurai.jpg",
    videoSrc: "/previews/lockscreens/corrupt-video.mp4",
    mediaType: "video",
    prefersReducedMotion: false,
    videoError: true, // Simulating video decoder failure / network error
  });

  assert.equal(displayState.hasVideo, false, "hasVideo must be false when videoError is true");
  assert.equal(displayState.effectiveDisplay, "poster", "Display must fall back to poster image");
  assert.equal(displayState.finalPoster, "/previews/lockscreens/dog-samurai.jpg");
});

test("Media Behavior 4: Reduced motion accessibility prevents video and animated autoplay", () => {
  const videoDisplay = determineMediaDisplayState({
    poster: "/previews/lockscreens/dog-samurai.jpg",
    videoSrc: "/previews/lockscreens/dog-samurai.mp4",
    mediaType: "video",
    prefersReducedMotion: true, // User prefers reduced motion
    videoError: false,
  });

  assert.equal(videoDisplay.hasVideo, false, "Video autoplay must be suppressed under prefers-reduced-motion");
  assert.equal(videoDisplay.effectiveDisplay, "poster", "Reduced motion must display static poster");

  const animatedDisplay = determineMediaDisplayState({
    poster: "/previews/lockscreens/animated-preview.jpg",
    animatedSrc: "/previews/lockscreens/animated-preview.gif",
    mediaType: "animated",
    prefersReducedMotion: true,
    videoError: false,
  });

  assert.equal(animatedDisplay.hasAnimatedImage, false, "Animated GIF must be suppressed under prefers-reduced-motion");
  assert.equal(animatedDisplay.effectiveDisplay, "poster", "Reduced motion must display static poster for animated items");
});

test("Media Behavior 5: Concurrency limiter strictly limits active video decoders to MAX_CONCURRENT_VIDEOS = 2", () => {
  assert.equal(MAX_CONCURRENT_VIDEOS, 2, "Default max concurrent videos must be 2");

  const limiter = new VideoPlaybackLimiter(2);
  let pauseCount = 0;

  const mockVideo1 = { id: "v1", pause: () => { pauseCount++; } };
  const mockVideo2 = { id: "v2", pause: () => { pauseCount++; } };
  const mockVideo3 = { id: "v3", pause: () => { pauseCount++; } };

  // Register first video
  limiter.register(mockVideo1);
  assert.equal(limiter.getActiveCount(), 1);
  assert.equal(limiter.has(mockVideo1), true);

  // Register second video
  limiter.register(mockVideo2);
  assert.equal(limiter.getActiveCount(), 2);
  assert.equal(limiter.has(mockVideo2), true);
  assert.equal(pauseCount, 0, "No eviction needed for <= 2 videos");

  // Register third video -> must evict oldest (v1) and call pause()
  const { evicted } = limiter.register(mockVideo3);
  assert.equal(limiter.getActiveCount(), 2, "Active count must remain capped at 2");
  assert.equal(evicted, mockVideo1, "Oldest video v1 must be evicted");
  assert.equal(pauseCount, 1, "Evicted video must be paused");
  assert.equal(limiter.has(mockVideo1), false, "v1 must no longer be active");
  assert.equal(limiter.has(mockVideo2), true, "v2 must remain active");
  assert.equal(limiter.has(mockVideo3), true, "v3 must be active");

  // Unregister v2
  limiter.unregister(mockVideo2);
  assert.equal(limiter.getActiveCount(), 1);
  assert.equal(limiter.has(mockVideo2), false);

  limiter.clear();
  assert.equal(limiter.getActiveCount(), 0);
});

test("Media Behavior 6: Library (PackageCard) and Discover (StoreCard) share unified media abstraction", () => {
  const catalogue = getCatalogueLockScreens();
  for (const pkg of catalogue) {
    const resolved = resolvePackageMedia(pkg);

    // Both cards must receive non-empty poster
    assert.ok(resolved.poster.length > 0, `Package ${pkg.id} must resolve non-empty poster`);

    // Normalized paths must always start with /
    const normPoster = normalizeMediaUrl(resolved.poster);
    assert.ok(
      normPoster.startsWith("/") || normPoster.startsWith("http://") || normPoster.startsWith("https://"),
      `Poster for ${pkg.id} must be normalized with valid URL or leading /`
    );

    if (resolved.videoSrc) {
      const normVideo = normalizeMediaUrl(resolved.videoSrc);
      assert.ok(normVideo.startsWith("/"), `Video for ${pkg.id} must be normalized with leading /`);
      assert.ok(
        resolved.mediaType === "video" || resolved.mediaType === "animated",
        `Package ${pkg.id} with videoSrc must be classified as video or animated`
      );
    }

    if (resolved.isStatic) {
      assert.equal(resolved.mediaType, "image", `Static package ${pkg.id} must have mediaType image`);
      assert.equal(resolved.videoSrc, undefined, `Static package ${pkg.id} must have no videoSrc`);
    }
  }
});

test("Media Behavior 7: Unique media identity across canonical Qylock packages", () => {
  const catalogue = getCatalogueLockScreens();
  const qylockThemes = catalogue.filter((p) => p.lockscreen?.provider === "qylock");
  assert.equal(qylockThemes.length, 25, "Must contain all 25 canonical Qylock themes");

  const seenPosters = new Set<string>();
  const seenVideos = new Set<string>();

  for (const theme of qylockThemes) {
    const media = resolvePackageMedia(theme);
    assert.ok(
      !seenPosters.has(media.poster),
      `Duplicate poster detected: ${media.poster} used in ${theme.id}`
    );
    seenPosters.add(media.poster);

    if (media.videoSrc) {
      assert.ok(
        !seenVideos.has(media.videoSrc),
        `Duplicate video preview detected: ${media.videoSrc} used in ${theme.id}`
      );
      seenVideos.add(media.videoSrc);
    }
  }

  assert.equal(seenPosters.size, 25, "All 25 Qylock themes must have distinct posters");
  assert.ok(seenVideos.size >= 14, "Must have distinct video assets for all video-enabled themes");
});
