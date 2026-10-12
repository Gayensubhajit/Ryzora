import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

test("Theme Tokens: Blue and Silver accent contrast contracts", () => {
  const tokensCss = fs.readFileSync(path.resolve(process.cwd(), "src/theme/tokens.css"), "utf-8");

  // 1. Blue accent token contract
  assert.ok(tokensCss.includes('[data-accent="blue"]'), "tokens.css must define [data-accent='blue']");
  assert.ok(tokensCss.includes("--rz-accent: #2563eb"), "Blue theme must set --rz-accent to #2563eb");
  assert.ok(tokensCss.includes("--rz-accent-contrast: #ffffff"), "Blue theme must set white contrast text #ffffff");

  // 2. Silver / Deep Black monochrome contract
  assert.ok(tokensCss.includes('[data-accent="silver"]'), "tokens.css must define [data-accent='silver']");
  assert.ok(tokensCss.includes("--rz-accent: #D9DCE2"), "Silver theme must set --rz-accent to #D9DCE2");
  assert.ok(tokensCss.includes("--rz-accent-contrast: #050608"), "Silver theme must set dark contrast text #050608");
});

test("Theme Contrast Ratio: Both Blue and Silver exceed WCAG requirements", () => {
  // Relative luminance calculation according to WCAG 2.1
  const srgbToLin = (c: number) => {
    const v = c / 255;
    return v <= 0.04045 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  };
  const luminance = (r: number, g: number, b: number) =>
    0.2126 * srgbToLin(r) + 0.7152 * srgbToLin(g) + 0.0722 * srgbToLin(b);
  const contrastRatio = (l1: number, l2: number) => {
    const lighter = Math.max(l1, l2);
    const darker = Math.min(l1, l2);
    return (lighter + 0.05) / (darker + 0.05);
  };

  const lumWhite = luminance(255, 255, 255);
  const lumBlue = luminance(37, 99, 235); // #2563eb
  const lumSilver = luminance(217, 220, 226); // #D9DCE2
  const lumDarkText = luminance(5, 6, 8); // #050608

  const blueContrast = contrastRatio(lumWhite, lumBlue);
  const silverContrast = contrastRatio(lumSilver, lumDarkText);

  // Both should provide strong readable contrast
  assert.ok(blueContrast >= 4.5, `Blue contrast ${blueContrast.toFixed(2)} must be >= 4.5:1 (AA)`);
  assert.ok(silverContrast >= 7.0, `Silver contrast ${silverContrast.toFixed(2)} must be >= 7.0:1 (AAA)`);
});

test("Global Button System: Button contrast rules in src/index.css", () => {
  const indexCss = fs.readFileSync(path.resolve(process.cwd(), "src/index.css"), "utf-8");

  assert.ok(indexCss.includes(".btn-primary"), "index.css must define .btn-primary");
  assert.ok(indexCss.includes("var(--rz-accent-contrast"), "index.css .btn-primary must use var(--rz-accent-contrast)");
  assert.ok(
    indexCss.includes("button.bg-") && indexCss.includes("var(--rz-accent-contrast"),
    "index.css must provide contrast protection for button.bg-[var(--rz-accent)]"
  );
  assert.ok(
    indexCss.includes("stroke: currentColor") || indexCss.includes("var(--rz-accent-contrast"),
    "index.css must ensure button icons match contrast text"
  );
});

test("Fastfetch View: Sequence counter and caching prevent stale results and excess runs", () => {
  const fastfetchView = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");

  // 1. Request sequence ref present
  assert.ok(fastfetchView.includes("requestSeqRef = useRef<number>(0)"), "Must track request sequence number");
  assert.ok(fastfetchView.includes("currentSeq === requestSeqRef.current"), "Must drop stale out-of-order responses");

  // 2. In-memory preview cache present
  assert.ok(fastfetchView.includes("previewCacheRef = useRef"), "Must maintain in-memory preview cache");
  assert.ok(fastfetchView.includes("previewCacheRef.current.has(cacheKey)"), "Must check preview cache before IPC invoke");

  // 3. Debounce delay set to 300ms
  assert.ok(fastfetchView.includes("}, 300);"), "Preview trigger must use 300ms debounce");

  // 4. installedEmblems removed from preview effect deps to prevent gratuitous preview spawns
  const previewTimerIdx = fastfetchView.indexOf("renderPreview();\n    }, 300);");
  const previewDepsBlock = fastfetchView.slice(previewTimerIdx, previewTimerIdx + 300);
  assert.ok(
    !previewDepsBlock.includes("installedEmblems"),
    "Preview effect must NOT include installedEmblems in deps"
  );
});

test("Catalog Grid Performance: StoreCard and PackageCard are memoized and images are lazy loaded", () => {
  const storeCard = fs.readFileSync(path.resolve(process.cwd(), "src/components/StoreCard.tsx"), "utf-8");
  const packageCard = fs.readFileSync(path.resolve(process.cwd(), "src/components/PackageCard.tsx"), "utf-8");
  const mediaPreview = fs.readFileSync(path.resolve(process.cwd(), "src/components/media/MediaPreview.tsx"), "utf-8");
  const fastfetchView = fs.readFileSync(path.resolve(process.cwd(), "src/views/FastfetchView.tsx"), "utf-8");

  assert.ok(storeCard.includes("React.memo"), "StoreCard must be wrapped in React.memo");
  assert.ok(packageCard.includes("React.memo"), "PackageCard must be wrapped in React.memo");
  assert.ok(mediaPreview.includes('loading="lazy"'), "MediaPreview must use loading='lazy'");
  assert.ok(fastfetchView.includes('loading="lazy"'), "Fastfetch emblem thumbnails must use loading='lazy'");
});
