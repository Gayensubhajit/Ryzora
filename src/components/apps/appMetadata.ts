/**
 * Shared application metadata, branding, screenshots and upstream specifications
 * Kept pure TypeScript (no JSX) for direct node test compatibility.
 */

export interface AppScreenshot {
  url: string;
  caption?: string;
}

export interface AppMetadata {
  displayName: string;
  publisher: string;
  category: "Internet" | "Development" | "Multimedia" | "Graphics" | "Games" | "Utilities" | "System";
  accentColor: string;
  brandColor: string;
  summary: string;
  fullDescription?: string;
  website: string;
  sourceRepository?: string;
  issueTracker?: string;
  firstReleased?: string;
  architecture?: string;
  screenshots?: AppScreenshot[];
  relatedApps?: string[];
  isCuratedApp: boolean;
}

export const KNOWN_APPS: Record<string, AppMetadata> = {
  firefox: {
    displayName: "Firefox",
    publisher: "Mozilla",
    category: "Internet",
    accentColor: "#FF7139",
    brandColor: "#FF7139",
    summary: "Fast, private and extensible web browser from Mozilla",
    fullDescription:
      "Firefox is a fast, full-featured Web browser. Firefox includes great features such as tabbed browsing, privacy browsing, spell checking, incremental find, live bookmarking, a download manager, and an integrated search system.\n\nEnjoy enhanced tracking protection, seamless cross-device synchronization, and thousands of customization add-ons to build your perfect browsing experience.",
    website: "https://www.mozilla.org/firefox",
    sourceRepository: "https://hg.mozilla.org/mozilla-central",
    issueTracker: "https://bugzilla.mozilla.org",
    firstReleased: "2002-09-23",
    architecture: "x86_64",
    screenshots: [
      {
        url: "/assets/apps/firefox/screen1.jpg",
        caption: "Firefox Tabbed Browsing Interface",
      },
      {
        url: "/assets/apps/firefox/screen2.jpg",
        caption: "Customizable Toolbars and Privacy Protections",
      },
    ],
    relatedApps: ["chromium", "discord", "alacritty", "thunderbird"],
    isCuratedApp: true,
  },
  chromium: {
    displayName: "Chromium",
    publisher: "The Chromium Authors",
    category: "Internet",
    accentColor: "#4285F4",
    brandColor: "#4285F4",
    summary: "Open-source web browser engine powering modern web experiences",
    fullDescription:
      "Chromium is an open-source browser project that aims to build a safer, faster, and more stable way for all Internet users to experience the web. It provides the foundation for Google Chrome and countless other modern web tools.",
    website: "https://www.chromium.org",
    sourceRepository: "https://chromium.googlesource.com/chromium/src",
    issueTracker: "https://issues.chromium.org",
    firstReleased: "2008-09-02",
    architecture: "x86_64",
    screenshots: [
    ],
    relatedApps: ["firefox", "code", "telegram", "discord"],
    isCuratedApp: true,
  },
  "visual-studio-code-bin": {
    displayName: "VS Code",
    publisher: "Microsoft",
    category: "Development",
    accentColor: "#007ACC",
    brandColor: "#007ACC",
    summary: "Code editing redefined. Powerful lightweight extensible IDE",
    fullDescription:
      "Visual Studio Code is a lightweight but powerful source code editor which runs on your desktop and is available for Linux, macOS and Windows. It comes with built-in support for JavaScript, TypeScript and Node.js and has a rich ecosystem of extensions for other languages and runtimes.",
    website: "https://code.visualstudio.com",
    sourceRepository: "https://github.com/microsoft/vscode",
    issueTracker: "https://github.com/microsoft/vscode/issues",
    firstReleased: "2015-04-29",
    architecture: "x86_64",
    screenshots: [
    ],
    relatedApps: ["neovim", "alacritty", "git", "kitty"],
    isCuratedApp: true,
  },
  code: {
    displayName: "Code (OSS)",
    publisher: "Arch Linux / Microsoft",
    category: "Development",
    accentColor: "#007ACC",
    brandColor: "#007ACC",
    summary: "Open-source build of Visual Studio Code editor",
    fullDescription:
      "Code (OSS) is the community open-source binary build of Microsoft's Visual Studio Code repository, compiled natively for Arch Linux.",
    website: "https://code.visualstudio.com",
    sourceRepository: "https://github.com/microsoft/vscode",
    issueTracker: "https://github.com/microsoft/vscode/issues",
    firstReleased: "2015-04-29",
    architecture: "x86_64",
    relatedApps: ["neovim", "alacritty", "git"],
    isCuratedApp: true,
  },
  discord: {
    displayName: "Discord",
    publisher: "Discord Inc.",
    category: "Internet",
    accentColor: "#5865F2",
    brandColor: "#5865F2",
    summary: "All-in-one voice and text chat platform for communities",
    fullDescription:
      "Discord is the easiest way to talk over voice, video, and text. Talk, chat, hang out, and stay close with your friends and communities.",
    website: "https://discord.com",
    firstReleased: "2015-05-13",
    architecture: "x86_64",
    relatedApps: ["telegram", "steam", "spotify", "firefox"],
    isCuratedApp: true,
  },
  steam: {
    displayName: "Steam",
    publisher: "Valve Corporation",
    category: "Games",
    accentColor: "#171A21",
    brandColor: "#171A21",
    summary: "The ultimate entertainment platform for playing and creating games",
    fullDescription:
      "Steam is the ultimate destination for playing, discussing, and creating games. Featuring thousands of native Linux titles with Proton compatibility.",
    website: "https://store.steampowered.com",
    issueTracker: "https://github.com/ValveSoftware/steam-for-linux/issues",
    firstReleased: "2003-09-12",
    architecture: "x86_64",
    relatedApps: ["discord", "obs-studio", "vlc", "spotify"],
    isCuratedApp: true,
  },
  vlc: {
    displayName: "VLC Media Player",
    publisher: "VideoLAN Organization",
    category: "Multimedia",
    accentColor: "#FF8800",
    brandColor: "#FF8800",
    summary: "Free and open-source cross-platform multimedia player and framework",
    fullDescription:
      "VLC is a free and open source cross-platform multimedia player and framework that plays most multimedia files as well as DVDs, Audio CDs, VCDs, and various streaming protocols.",
    website: "https://www.videolan.org/vlc",
    sourceRepository: "https://code.videolan.org/videolan/vlc",
    firstReleased: "2001-02-01",
    architecture: "x86_64",
    relatedApps: ["mpv", "obs-studio", "spotify"],
    isCuratedApp: true,
  },
  gimp: {
    displayName: "GIMP",
    publisher: "The GIMP Development Team",
    category: "Graphics",
    accentColor: "#5C5543",
    brandColor: "#5C5543",
    summary: "GNU Image Manipulation Program for high-level photo retouching",
    fullDescription:
      "GIMP is an acronym for GNU Image Manipulation Program. It is a freely distributed program for such tasks as photo retouching, image composition and image authoring.",
    website: "https://www.gimp.org",
    sourceRepository: "https://gitlab.gnome.org/GNOME/gimp",
    firstReleased: "1996-01-15",
    architecture: "x86_64",
    relatedApps: ["inkscape", "blender", "obs-studio"],
    isCuratedApp: true,
  },
  blender: {
    displayName: "Blender",
    publisher: "Blender Foundation",
    category: "Graphics",
    accentColor: "#EA7600",
    brandColor: "#EA7600",
    summary: "Open source 3D creation suite supporting modeling, rigging, and animation",
    fullDescription:
      "Blender is the free and open source 3D creation suite. It supports the entirety of the 3D pipeline—modeling, rigging, animation, simulation, rendering, compositing and motion tracking, video editing and 2D animation pipeline.",
    website: "https://www.blender.org",
    sourceRepository: "https://projects.blender.org/blender/blender",
    firstReleased: "1994-01-02",
    architecture: "x86_64",
    relatedApps: ["gimp", "inkscape", "obs-studio"],
    isCuratedApp: true,
  },
  "obs-studio": {
    displayName: "OBS Studio",
    publisher: "OBS Project",
    category: "Multimedia",
    accentColor: "#302E31",
    brandColor: "#302E31",
    summary: "Free and open source software for video recording and live streaming",
    fullDescription:
      "Free and open source software for video recording and live streaming. Download and start streaming quickly and easily on Linux, Mac or Windows.",
    website: "https://obsproject.com",
    sourceRepository: "https://github.com/obsproject/obs-studio",
    firstReleased: "2012-09-01",
    architecture: "x86_64",
    relatedApps: ["vlc", "blender", "steam"],
    isCuratedApp: true,
  },
  alacritty: {
    displayName: "Alacritty",
    publisher: "Alacritty Team",
    category: "Utilities",
    accentColor: "#F46036",
    brandColor: "#F46036",
    summary: "GPU-accelerated terminal emulator focused on simplicity and performance",
    fullDescription:
      "Alacritty is a modern terminal emulator that comes with sensible defaults, but allows for extensive configuration. By integrating with other applications, rather than reimplementing their functionality, it manages to deliver a flexible set of features with high performance.",
    website: "https://alacritty.org",
    sourceRepository: "https://github.com/alacritty/alacritty",
    firstReleased: "2017-01-06",
    architecture: "x86_64",
    relatedApps: ["kitty", "neovim", "btop", "git"],
    isCuratedApp: true,
  },
  kitty: {
    displayName: "Kitty",
    publisher: "Kovid Goyal",
    category: "Utilities",
    accentColor: "#1793D1",
    brandColor: "#1793D1",
    summary: "Cross-platform, fast, feature-rich, GPU based terminal emulator",
    fullDescription:
      "kitty is the fast, feature-rich, GPU based terminal emulator. It uses OpenGL for rendering all terminal content. Supports graphics, ligatures, and tabs.",
    website: "https://sw.kovidgoyal.net/kitty",
    sourceRepository: "https://github.com/kovidgoyal/kitty",
    firstReleased: "2018-04-01",
    architecture: "x86_64",
    relatedApps: ["alacritty", "neovim", "btop"],
    isCuratedApp: true,
  },
  neovim: {
    displayName: "Neovim",
    publisher: "Neovim Project",
    category: "Development",
    accentColor: "#57A143",
    brandColor: "#57A143",
    summary: "Vim-fork focused on extensibility and modern terminal/GUI integration",
    fullDescription:
      "Vim-fork focused on extensibility and usability. Neovim is a hyperextensible Vim-based text editor featuring Lua scripting, LSP client integration, and modern async APIs.",
    website: "https://neovim.io",
    sourceRepository: "https://github.com/neovim/neovim",
    firstReleased: "2014-02-21",
    architecture: "x86_64",
    relatedApps: ["alacritty", "code", "git"],
    isCuratedApp: true,
  },
  git: {
    displayName: "Git",
    publisher: "Git Authors",
    category: "Development",
    accentColor: "#F05032",
    brandColor: "#F05032",
    summary: "Fast, scalable, distributed revision control system",
    website: "https://git-scm.com",
    isCuratedApp: true,
  },
  spotify: {
    displayName: "Spotify",
    publisher: "Spotify AB",
    category: "Multimedia",
    accentColor: "#1DB954",
    brandColor: "#1DB954",
    summary: "Digital music service providing access to millions of songs",
    website: "https://www.spotify.com",
    isCuratedApp: true,
  },
  btop: {
    displayName: "Btop++",
    publisher: "Aristocratos",
    category: "Utilities",
    accentColor: "#88C0D0",
    brandColor: "#88C0D0",
    summary: "Resource monitor that shows usage and stats for processor, memory, disks",
    website: "https://github.com/aristocratos/btop",
    isCuratedApp: true,
  },
  thunderbird: {
    displayName: "Thunderbird",
    publisher: "Mozilla",
    category: "Internet",
    accentColor: "#0A84FF",
    brandColor: "#0A84FF",
    summary: "Free and open-source email, calendar, and chat client",
    website: "https://www.thunderbird.net",
    isCuratedApp: true,
  },
  inkscape: {
    displayName: "Inkscape",
    publisher: "Inkscape Community",
    category: "Graphics",
    accentColor: "#000000",
    brandColor: "#000000",
    summary: "Professional vector graphics editor for Linux, Windows and macOS",
    website: "https://inkscape.org",
    isCuratedApp: true,
  },
  mpv: {
    displayName: "mpv",
    publisher: "mpv project",
    category: "Multimedia",
    accentColor: "#610080",
    brandColor: "#610080",
    summary: "Free, open source, and cross-platform media player",
    website: "https://mpv.io",
    isCuratedApp: true,
  },
  telegram: {
    displayName: "Telegram Desktop",
    publisher: "Telegram FZ-LLC",
    category: "Internet",
    accentColor: "#2AABEE",
    brandColor: "#2AABEE",
    summary: "Fast and secure desktop messaging app connected to Telegram cloud",
    website: "https://desktop.telegram.org",
    isCuratedApp: true,
  },
};

export function resolveAppMetadata(pkgId: string, fallbackTitle?: string): AppMetadata {
  const normalized = pkgId.toLowerCase().trim();
  if (KNOWN_APPS[normalized]) {
    return KNOWN_APPS[normalized];
  }

  // Check prefix or partial match
  for (const [key, meta] of Object.entries(KNOWN_APPS)) {
    if (normalized.startsWith(key) || key.startsWith(normalized)) {
      return meta;
    }
  }

  // Generic fallback classification
  const title = fallbackTitle || pkgId;
  return {
    displayName: title.charAt(0).toUpperCase() + title.slice(1),
    publisher: "Arch Linux Packagers",
    category: "System",
    accentColor: "#64748B",
    brandColor: "#64748B",
    summary: `Arch Linux official repository package (${pkgId})`,
    fullDescription: `Official package distributed via the Arch Linux package ecosystem (${pkgId}). Built with native toolchains and packaged according to Arch Linux packaging standards.`,
    website: `https://archlinux.org/packages/?q=${encodeURIComponent(pkgId)}`,
    sourceRepository: `https://gitlab.archlinux.org/archlinux/packaging/packages/${encodeURIComponent(pkgId)}`,
    architecture: "x86_64",
    isCuratedApp: false,
  };
}
