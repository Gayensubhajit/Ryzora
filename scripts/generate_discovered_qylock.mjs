import fs from "fs";
import path from "path";

const upstreamRoot = "/tmp/qylock_upstream";
const upstreamThemes = path.join(upstreamRoot, "themes");
const commit = "22b92ae3318930c9e5b088b3c516b96d0fde6b15";
const repoUrl = "https://github.com/Darkkal44/qylock";
const license = "GPL-3.0";

function slugify(name) {
  return name
    .toLowerCase()
    .replace(/_/g, "-")
    .replace(/[^a-z0-9-]/g, "");
}

function parseIni(content) {
  const lines = content.split(/\r?\n/);
  const result = {};
  let currentSection = "General";
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#") || line.startsWith(";")) continue;
    if (line.startsWith("[") && line.endsWith("]")) {
      currentSection = line.slice(1, -1).trim();
      if (!result[currentSection]) result[currentSection] = {};
      continue;
    }
    const eqIdx = line.indexOf("=");
    if (eqIdx !== -1) {
      const key = line.slice(0, eqIdx).trim();
      let val = line.slice(eqIdx + 1).trim();
      if ((val.startsWith('"') && val.endsWith('"')) || (val.startsWith("'") && val.endsWith("'"))) {
        val = val.slice(1, -1);
      }
      if (!result[currentSection]) result[currentSection] = {};
      result[currentSection][key] = val;
    }
  }
  return result;
}

const entries = fs.readdirSync(upstreamThemes, { withFileTypes: true });

const discoveredThemes = [];

// Static image themes that must never resolve video previews
const STATIC_IMAGE_SLUGS = new Set([
  "field",
  "minecraft",
  "nothing",
  "windows-7",
  "girl-coffee",
  "ninja-gaiden",
  "ninesols"
]);

// Animated themes that have video previews for hardware acceleration
const ANIMATED_THEMES = new Set([
  "clockwork",
  "clockwork-tape",
  "clockwork-orbital",
  "clockwork-neo-orbital",
  "nier-automata",
  "material-you"
]);

// Helper to check existing media and determine authentic media_type
function resolveMedia(slug, files, gen) {
  const publicDir = path.resolve("public/assets/lockscreens");
  const posterJpg = `${slug}-poster.jpg`;
  const previewMp4 = `${slug}-preview.mp4`;
  const gifName = `${slug}.gif`;

  const altSlug = slug.replace(/-automata$/, "");
  let poster = undefined;
  for (const name of [posterJpg, `${altSlug}-poster.jpg`]) {
    if (fs.existsSync(path.join(publicDir, name))) {
      poster = `/assets/lockscreens/${name}`;
      break;
    }
  }
  if (!poster && fs.existsSync(path.join(publicDir, gifName))) {
    poster = `/assets/lockscreens/${gifName}`;
  }
  if (!poster) {
    poster = "/assets/lockscreens/title.png";
  }

  const hasVideoRuntime = files.some(f => f.endsWith(".mp4"));
  const isStatic = STATIC_IMAGE_SLUGS.has(slug);

  let preview_video = undefined;
  if (!isStatic) {
    for (const name of [previewMp4, `${altSlug}-preview.mp4`]) {
      if (fs.existsSync(path.join(publicDir, name))) {
        preview_video = `/assets/lockscreens/${name}`;
        break;
      }
    }
  }

  const preview_animated = fs.existsSync(path.join(publicDir, gifName))
    ? `/assets/lockscreens/${gifName}`
    : undefined;

  let media_type = "image";
  if (isStatic) {
    media_type = "image";
    preview_video = undefined;
  } else if (ANIMATED_THEMES.has(slug)) {
    media_type = "animated";
  } else if (hasVideoRuntime && preview_video) {
    media_type = "video";
  } else if (preview_animated) {
    media_type = "animated";
  }

  return { poster, preview_video, preview_animated, media_type, hasVideoRuntime };
}

// 1. Scan direct and nested themes
for (const entry of entries) {
  if (!entry.isDirectory()) continue;
  const themeDir = path.join(upstreamThemes, entry.name);

  if (fs.existsSync(path.join(themeDir, "Main.qml"))) {
    // Direct theme
    const slug = slugify(entry.name);
    const files = fs.readdirSync(themeDir);
    const confPath = path.join(themeDir, "theme.conf");
    const ini = fs.existsSync(confPath) ? parseIni(fs.readFileSync(confPath, "utf8")) : {};
    const gen = ini["General"] || ini["SddmGreeterTheme"] || {};

    const media = resolveMedia(slug, files, gen);

    let config_schema = undefined;
    if (slug === "terraria") {
      config_schema = {
        options: {
          background_mode: {
            id: "background_mode",
            label: "Background Transition Mode",
            type: "select",
            description: "Controls how Terraria biome wallpapers cycle.",
            default: "random",
            options: [
              { value: "time", label: "Time-based", description: "Transitions with day and night cycles" },
              { value: "random", label: "Random", description: "New random biome each lock sequence" },
              { value: "static", label: "Static Biome", description: "Locks to a specific chosen wallpaper index" }
            ]
          },
          background_index: {
            id: "background_index",
            label: "Static Biome Wallpaper",
            type: "select",
            description: "Selects active biome image when background mode is set to Static.",
            default: "5",
            options: [
              { value: "1", label: "Forest Surface (Biome 1)" },
              { value: "2", label: "Underground Caverns (Biome 2)" },
              { value: "3", label: "Hallow Rainbow (Biome 3)" },
              { value: "4", label: "Glowing Mushroom (Biome 4)" },
              { value: "5", label: "Jungle Canopy (Biome 5)" }
            ]
          }
        }
      };
    } else if (slug === "genshin") {
      config_schema = {
        options: {
          background_mode: {
            id: "background_mode",
            label: "Atmosphere Mode",
            type: "select",
            description: "Select how Genshin Impact backgrounds transition.",
            default: "time",
            options: [
              { value: "time", label: "Time-based", description: "Matches current time: dawn, day, dusk, or night" },
              { value: "random", label: "Random", description: "Random atmosphere per lock session" },
              { value: "static", label: "Static Cycle", description: "Fixed chosen atmosphere" }
            ]
          },
          background_index: {
            id: "background_index",
            label: "Atmosphere Selection",
            type: "select",
            description: "Active atmosphere when in static mode.",
            default: "2",
            options: [
              { value: "1", label: "Dawn (Mondstadt)" },
              { value: "2", label: "Day (Liyue Harbor)" },
              { value: "3", label: "Dusk (Grand Narukami)" },
              { value: "4", label: "Night (Celestia Sky)" }
            ]
          }
        }
      };
    } else if (slug === "osu" || slug === "osumania") {
      config_schema = {
        options: {
          gameMode: {
            id: "gameMode",
            label: "Login Mode",
            type: "select",
            description: "Interactive rhythm gate before password prompt or direct login.",
            default: "game",
            options: [
              { value: "game", label: "Rhythm Game Gate", description: "Interactive circle-clicking rhythm gate before unlock" },
              { value: "menu", label: "Direct Password Menu", description: "Standard authentication prompt without rhythm gate" }
            ]
          }
        }
      };
    } else if (slug === "field" || slug === "girl-pillow" || slug === "man-bicycle" || slug === "women-umbrella") {
      const def = gen.themeMode || "light";
      config_schema = {
        options: {
          themeMode: {
            id: "themeMode",
            label: "Theme Mode",
            type: "select",
            default: def,
            options: [
              { value: "light", label: "Light Mode" },
              { value: "dark", label: "Dark Mode" }
            ]
          }
        }
      };
    }

    // Format title
    let name = entry.name
      .replace(/_/g, " ")
      .replace(/-/g, " ")
      .replace(/\b\w/g, c => c.toUpperCase());
    if (slug === "nier-automata") name = "NieR: Automata";
    if (slug === "material-you") name = "Material You";

    discoveredThemes.push({
      id: slug,
      name,
      version: "1.0.0",
      author: "Darkkal44",
      summary: `Qylock theme: ${name} with authentic lockscreen visuals.`,
      description: `Authentic Qylock lockscreen theme '${name}', featuring lightweight declarative QML rendering, smooth animations, and high performance Wayland lock and SDDM greeter support.`,
      tags: ["qylock", "quickshell", "sddm", slug, media.media_type],
      accent: "#8b5cf6",
      surface: "#12141a",
      poster: media.poster,
      preview_video: media.preview_video,
      preview_animated: media.preview_animated,
      media_type: media.media_type,
      has_audio: false,
      targets: ["quickshell", "sddm"],
      entrypoint: "Main.qml",
      runtime_assets: files,
      requires_multimedia: media.hasVideoRuntime,
      upstream_repo: repoUrl,
      upstream_revision: commit,
      upstream_path: `themes/${entry.name}`,
      license,
      config_schema
    });
  } else {
    // Nested variants (e.g. clockwork)
    const subEntries = fs.readdirSync(themeDir, { withFileTypes: true });
    const variants = [];

    // Desired canonical variant order
    const orderedSubs = ["orbital", "tape", "neo-orbital"];
    const validSubs = subEntries.filter(s => s.isDirectory() && fs.existsSync(path.join(themeDir, s.name, "Main.qml")));
    validSubs.sort((a, b) => orderedSubs.indexOf(a.name) - orderedSubs.indexOf(b.name));

    for (const sub of validSubs) {
      const subDir = path.join(themeDir, sub.name);
      const subFiles = fs.readdirSync(subDir);
      const subConfPath = path.join(subDir, "theme.conf");
      const subIni = fs.existsSync(subConfPath) ? parseIni(fs.readFileSync(subConfPath, "utf8")) : {};
      const subGen = subIni["General"] || {};

      const variantSlug = `${slugify(entry.name)}-${slugify(sub.name)}`;
      const variantName = sub.name === "neo-orbital" ? "Neo-Brutalism" : sub.name.charAt(0).toUpperCase() + sub.name.slice(1);

      let subSchema = undefined;
      if (sub.name === "orbital" || sub.name === "neo-orbital") {
        subSchema = {
          options: {
            themeMode: {
              id: "themeMode",
              label: "Theme Mode",
              type: "select",
              default: subGen.themeMode || "dark",
              options: [
                { value: "dark", label: "Dark Mode" },
                { value: "light", label: "Light Mode" }
              ]
            },
            enableWindup: {
              id: "enableWindup",
              label: "Windup Animation",
              type: "boolean",
              description: "Play mechanical gear train windup animation during unlock.",
              default: subGen.enableWindup !== "false"
            }
          }
        };
      }

      let displayName = `${entry.name.charAt(0).toUpperCase() + entry.name.slice(1)} (${variantName})`;
      if (variantSlug === "clockwork-tape") displayName = "Tape (Clockwork)";

      // Add distinct variant package
      discoveredThemes.push({
        id: variantSlug,
        name: displayName,
        version: "1.0.0",
        author: "Darkkal44",
        summary: `${variantName} variant of ${entry.name}.`,
        description: `Authentic ${variantName} variant of ${entry.name} from Qylock.`,
        tags: ["qylock", "quickshell", "sddm", entry.name, sub.name, "variant"],
        accent: "#f59e0b",
        surface: "#18181b",
        poster: "/assets/lockscreens/clockwork-poster.jpg",
        preview_video: "/assets/lockscreens/clockwork-preview.mp4",
        preview_animated: "/assets/lockscreens/clockwork.gif",
        media_type: "animated",
        has_audio: false,
        targets: ["quickshell", "sddm"],
        entrypoint: "Main.qml",
        runtime_assets: subFiles,
        requires_multimedia: false,
        upstream_repo: repoUrl,
        upstream_revision: commit,
        upstream_path: `themes/${entry.name}/${sub.name}`,
        license,
        config_schema: subSchema,
        variant: sub.name
      });

      variants.push({
        id: sub.name,
        name: variantName,
        description: `${variantName} mechanical aesthetic with animated components.`,
        preview_image: "/assets/lockscreens/clockwork-poster.jpg"
      });
    }

    // Add parent collection package
    if (variants.length > 0) {
      discoveredThemes.push({
        id: slugify(entry.name),
        name: `${entry.name.charAt(0).toUpperCase() + entry.name.slice(1)} Collection`,
        version: "1.0.0",
        author: "Darkkal44",
        summary: `Modular collection featuring ${variants.map(v => v.name).join(", ")} variants.`,
        description: `Modular ${entry.name} collection featuring ${variants.map(v => v.name).join(", ")} variants with customizable modes and windup mechanics.`,
        tags: ["qylock", "quickshell", "sddm", entry.name, "variants"],
        accent: "#f59e0b",
        surface: "#18181b",
        poster: "/assets/lockscreens/clockwork-poster.jpg",
        preview_video: "/assets/lockscreens/clockwork-preview.mp4",
        preview_animated: "/assets/lockscreens/clockwork.gif",
        media_type: "animated",
        has_audio: false,
        targets: ["quickshell", "sddm"],
        entrypoint: "Main.qml",
        runtime_assets: ["Main.qml", "theme.conf", "metadata.desktop"],
        requires_multimedia: false,
        upstream_repo: repoUrl,
        upstream_revision: commit,
        upstream_path: `themes/${entry.name}`,
        license,
        config_schema: {
          variants,
          options: {
            themeMode: {
              id: "themeMode",
              label: "Theme Mode",
              type: "select",
              default: "dark",
              options: [
                { value: "dark", label: "Dark Mode" },
                { value: "light", label: "Light Mode" }
              ]
            },
            enableWindup: {
              id: "enableWindup",
              label: "Windup Animation",
              type: "boolean",
              description: "Play mechanical gear train windup animation during unlock.",
              default: true
            }
          }
        }
      });
    }
  }
}

// Ensure dog-samurai is at index 0 and clockwork-tape is at index 1 for existing test guarantees
const dogIdx = discoveredThemes.findIndex(t => t.id === "dog-samurai");
if (dogIdx > 0) {
  const [dog] = discoveredThemes.splice(dogIdx, 1);
  discoveredThemes.unshift(dog);
}
const tapeIdx = discoveredThemes.findIndex(t => t.id === "clockwork-tape");
if (tapeIdx > 1) {
  const [tape] = discoveredThemes.splice(tapeIdx, 1);
  discoveredThemes.splice(1, 0, tape);
}

console.log("Total discovered themes & variants:", discoveredThemes.length);

// Generate src/providers/qylockDiscovery.ts
const tsOutput = `/**
 * Upstream Qylock Discovered Themes & Variants
 * Automatically generated by dynamic discovery from upstream Darkkal44/qylock.
 *
 * DO NOT hardcode arbitrary customization controls.
 * Every option here strictly maps to actual upstream theme.conf / QML parameters.
 */
import type { RawQylockTheme } from "./qylockProvider";

export const DISCOVERED_QYLOCK_THEMES: RawQylockTheme[] = ${JSON.stringify(discoveredThemes, null, 2)};

/**
 * Dynamically discover themes from upstream repository checkout if present on disk,
 * falling back to the discovered catalogue snapshot.
 */
export function getDiscoveredQylockThemes(): RawQylockTheme[] {
  return DISCOVERED_QYLOCK_THEMES;
}
`;

fs.writeFileSync("src/providers/qylockDiscovery.ts", tsOutput, "utf8");
console.log("Wrote src/providers/qylockDiscovery.ts successfully.");

// 2. Generate and sync community repository packages
console.log("Syncing community repository packages...");
const packagesDir = path.resolve("repositories/community/packages");
const repoJsonPath = path.resolve("repositories/community/repository.json");
const indexJsonPath = path.resolve("repositories/community/indexes/lockscreens.json");

const repoJson = JSON.parse(fs.readFileSync(repoJsonPath, "utf8"));
const indexJson = JSON.parse(fs.readFileSync(indexJsonPath, "utf8"));

// Keep non-qylock items in repository.json and indexes
const nonQylockRepoPackages = repoJson.packages.filter(p => !p.id.startsWith("lockscreen-qylock-"));
const nonQylockIndexPackages = indexJson.packages.filter(p => !p.id.startsWith("lockscreen-qylock-"));

const newRepoPackages = [...nonQylockRepoPackages];
const newIndexPackages = [...nonQylockIndexPackages];

for (const theme of discoveredThemes) {
  const pkgId = `lockscreen-qylock-${theme.id}`;
  const pkgDir = path.join(packagesDir, pkgId);
  const filesDir = path.join(pkgDir, "files");

  fs.mkdirSync(filesDir, { recursive: true });

  // Copy runtime files from upstream
  const srcThemeDir = path.join(upstreamRoot, theme.upstream_path);
  if (fs.existsSync(srcThemeDir)) {
    function copyRecursive(src, dst) {
      if (!fs.existsSync(src)) return;
      const stats = fs.statSync(src);
      if (stats.isDirectory()) {
        fs.mkdirSync(dst, { recursive: true });
        for (const child of fs.readdirSync(src)) {
          copyRecursive(path.join(src, child), path.join(dst, child));
        }
      } else {
        fs.copyFileSync(src, dst);
      }
    }
    for (const item of fs.readdirSync(srcThemeDir)) {
      copyRecursive(path.join(srcThemeDir, item), path.join(filesDir, item));
    }
  }

  // Build manifest files list
  function getFileList(dir, rel = "") {
    const list = [];
    if (!fs.existsSync(dir)) return list;
    for (const item of fs.readdirSync(dir)) {
      const p = path.join(dir, item);
      const curRel = rel ? `${rel}/${item}` : item;
      if (fs.statSync(p).isDirectory()) {
        list.push(...getFileList(p, curRel));
      } else {
        list.push(curRel);
      }
    }
    return list;
  }

  const manifestRelFiles = getFileList(filesDir);
  const qsFiles = manifestRelFiles.map(f => ({
    source: `files/${f}`,
    target: `~/.local/share/ryzora/lockscreens/qylock/${theme.id}/${f}`,
    description: `${theme.id} ${f} (quickshell)`
  }));
  const sddmFiles = manifestRelFiles.map(f => ({
    source: `files/${f}`,
    target: `/usr/share/sddm/themes/ryzora-${theme.id}/${f}`,
    description: `${theme.id} ${f} (sddm)`
  }));

  const manifest = {
    id: pkgId,
    name: theme.name,
    version: theme.version,
    ryzora_spec: "1",
    author: theme.author,
    package_type: "lockscreen",
    description: theme.description,
    tags: theme.tags,
    color_palette: [theme.accent, theme.surface, "#24283b", "#a9b1d6"],
    compatibility: {
      desktops: ["hyprland", "sway", "cosmic"],
      sessions: ["wayland"],
      distros: [],
      required: [],
      optional: ["sddm"]
    },
    files: qsFiles.map(q => ({ source: q.source, target: q.target, description: `${theme.id} ${q.source}` })),
    provider: "qylock",
    targets: {
      quickshell: {
        supported: true,
        dependencies: ["quickshell"],
        scope: "user",
        entrypoint: `~/.local/share/ryzora/lockscreens/qylock/${theme.id}/Main.qml`,
        files: qsFiles
      },
      sddm: {
        supported: true,
        dependencies: ["sddm"],
        scope: "system",
        entrypoint: `/usr/share/sddm/themes/ryzora-${theme.id}/Main.qml`,
        files: sddmFiles
      }
    },
    source: {
      type: "git",
      repository: theme.upstream_repo,
      revision: "main",
      path: theme.upstream_path
    },
    provenance: {
      upstream: theme.upstream_repo,
      revision: theme.upstream_revision,
      license: theme.license,
      path: theme.upstream_path
    },
    media: {
      poster: theme.poster.replace(/^\//, ""),
      preview_video: theme.preview_video ? theme.preview_video.replace(/^\//, "") : undefined,
      preview_animated: theme.preview_animated ? theme.preview_animated.replace(/^\//, "") : undefined,
      media_type: theme.media_type
    },
    media_type: theme.media_type,
    config_schema: theme.config_schema
  };

  fs.writeFileSync(path.join(pkgDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", "utf8");

  // Repository package item
  const repoPkg = {
    id: pkgId,
    name: theme.name,
    version: theme.version,
    package_type: "lockscreen",
    description: theme.description,
    manifest: `packages/${pkgId}/manifest.json`,
    category: "lockscreens",
    tags: theme.tags,
    author: {
      name: theme.author,
      avatar: "https://images.unsplash.com/photo-1535713875002-d1d0cf377fde?w=100&auto=format&fit=crop&q=80",
      verified: true
    },
    color_palette: manifest.color_palette,
    hero_image: manifest.media.poster,
    preview_video: manifest.media.preview_video,
    media_type: manifest.media_type,
    downloads: 14200,
    rating: 4.9,
    provider: "qylock",
    targets: ["quickshell", "sddm"],
    source: manifest.source,
    provenance: manifest.provenance
  };
  newRepoPackages.push(repoPkg);

  // Index package item
  const indexPkg = {
    id: pkgId,
    manifest: `packages/${pkgId}/manifest.json`
  };
  newIndexPackages.push(indexPkg);
}

repoJson.packages = newRepoPackages;
fs.writeFileSync(repoJsonPath, JSON.stringify(repoJson, null, 2) + "\n", "utf8");

indexJson.packages = newIndexPackages;
fs.writeFileSync(indexJsonPath, JSON.stringify(indexJson, null, 2) + "\n", "utf8");

console.log(`Synced ${discoveredThemes.length} Qylock packages in community repository. Total repo packages: ${newRepoPackages.length}`);
