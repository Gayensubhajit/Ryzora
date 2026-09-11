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
  features?: string[];
  website: string;
  sourceRepository?: string;
  issueTracker?: string;
  firstReleased?: string;
  architecture?: string;
  iconUrl?: string;
  screenshots?: AppScreenshot[];
  relatedApps?: string[];
  tagline?: string;
  documentationUrl?: string;
  highlights?: string[];
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
      "Firefox is an independent, user-first web browser designed for speed, privacy, and full open-standards compliance. Equipped with Enhanced Tracking Protection by default, Firefox blocks thousands of third-party trackers, cryptominers, and fingerprinting scripts automatically.  Enjoy multi-account containers for separating work and personal browsing, Picture-in-Picture video with multi-subtitle support, seamless cross-device synchronization, and thousands of customization add-ons to tailor your browsing experience exactly how you want.",
    features: [
      "Enhanced Tracking Protection & Total Cookie Protection",
      "Multi-Account Containers to separate work and personal accounts",
      "Seamless cross-device tab, bookmark, and history synchronization",
      "Picture-in-Picture video overlay with multiple window support",
      "Extensive add-ons and theme customization library",
    ],
    tagline: "Browse the web on your terms",
    documentationUrl: "https://support.mozilla.org",
    highlights: [
      "Enhanced Tracking Protection",
      "Extensive Customization",
      "Sync Across Devices",
      "Open Source and Community Driven"
    ],
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
      "Chromium is an open-source browser project that aims to build a safer, faster, and more stable way for all Internet users to experience the web. It serves as the open-source foundation for Google Chrome, Brave, and countless modern web applications.  Chromium offers an ultra-responsive multi-process architecture where each tab and extension executes in an isolated sandbox, preventing any single malfunctioning tab from crashing the browser. It features cutting-edge V8 JavaScript performance, strict adherence to W3C standards, and the industry-standard Chrome DevTools suite.",
    features: [
      "Multi-process architecture with sandboxed tabs and extensions",
      "V8 high-performance JavaScript and WebAssembly engine",
      "Industry-standard DevTools for frontend debugging and profiling",
      "Native modern web standards: WebGPU, WebAssembly, and PWA",
      "Granular site permissions and security control center",
    ],
    tagline: "The open-source engine of the modern web",
    documentationUrl: "https://www.chromium.org/developers/",
    highlights: [
      "Sandboxed Tab Security",
      "V8 High-Speed Engine",
      "Full Developer Tools",
      "Open Standards Compliance"
    ],
    website: "https://www.chromium.org",
    sourceRepository: "https://chromium.googlesource.com/chromium/src",
    issueTracker: "https://issues.chromium.org",
    firstReleased: "2008-09-02",
    architecture: "x86_64",
    screenshots: [
      {
        url: "/assets/apps/chromium/screen1.jpg",
        caption: "Chromium Clean Tabbed Web Navigation",
      },
      {
        url: "/assets/apps/chromium/screen2.jpg",
        caption: "Integrated Chrome DevTools and Network Inspector",
      },
    ],
    relatedApps: ["firefox", "code", "telegram-desktop", "discord"],
    isCuratedApp: true,
  },
  "visual-studio-code-bin": {
    displayName: "VS Code",
    publisher: "Microsoft",
    category: "Development",
    accentColor: "#007ACC",
    brandColor: "#007ACC",
    summary: "Code editing redefined — built-in Git, debugging and vast extensions",
    fullDescription:
      "Visual Studio Code is a streamlined code editor with support for development operations like debugging, task running, and version control. It aims to provide just the tools a developer needs for a quick code-build-debug cycle and leaves more complex workflows to fuller featured IDEs.  Featuring intelligent code completion (IntelliSense) powered by language servers, interactive debugging breakpoints, integrated multi-tab terminals, and a rich marketplace boasting tens of thousands of extensions.",
    features: [
      "IntelliSense intelligent syntax completions and parameter hints",
      "Interactive source code debugging with breakpoints and call stack inspection",
      "Built-in Git and GitHub source control management with visual diffs",
      "Integrated multi-split terminal running your favorite Linux shell",
      "Extensive marketplace covering Python, Rust, Go, TypeScript, and more",
    ],
    tagline: "Code editing redefined",
    documentationUrl: "https://code.visualstudio.com/docs",
    highlights: [
      "IntelliSense Completion",
      "Integrated Git Control",
      "Built-in Terminal Shell",
      "Rich Extensions Ecosystem"
    ],
    website: "https://code.visualstudio.com",
    sourceRepository: "https://github.com/microsoft/vscode",
    issueTracker: "https://github.com/microsoft/vscode/issues",
    firstReleased: "2015-04-29",
    architecture: "x86_64",
    screenshots: [
      {
        url: "/assets/apps/visual-studio-code-bin/screen1.jpg",
        caption: "VS Code Modern Dark Editing Workspace",
      },
      {
        url: "/assets/apps/visual-studio-code-bin/screen2.jpg",
        caption: "Extensions Marketplace & Visual Git Diff Viewer",
      },
    ],
    relatedApps: ["neovim", "alacritty", "kitty", "git"],
    isCuratedApp: true,
  },
  code: {
    displayName: "Code (OSS)",
    publisher: "Arch Linux / Community",
    category: "Development",
    accentColor: "#007ACC",
    brandColor: "#007ACC",
    summary: "Open source build of the Visual Studio Code editor",
    fullDescription:
      "Code - OSS is the open-source release of Microsoft's Visual Studio Code built directly from source without proprietary telemetry or Microsoft-branded packaging. It delivers the same ultra-fast editing experience, language server protocol integration, and customizable workspace.",
    features: [
      "100% open source build without proprietary tracking or telemetry",
      "IntelliSense completions and real-time syntax error checking",
      "Integrated terminal, Git integration, and multi-cursor editing",
      "Compatible with Open VSX Registry and open-source tooling",
    ],
    website: "https://github.com/microsoft/vscode",
    sourceRepository: "https://github.com/microsoft/vscode",
    issueTracker: "https://github.com/microsoft/vscode/issues",
    architecture: "x86_64",
    screenshots: [
      {
        url: "/assets/apps/code/screen1.jpg",
        caption: "Code OSS Development Environment",
      },
      {
        url: "/assets/apps/code/screen2.jpg",
        caption: "Source Control and Extension Marketplace",
      },
    ],
    relatedApps: ["visual-studio-code-bin", "neovim", "alacritty", "git"],
    isCuratedApp: true,
  },
  discord: {
    displayName: "Discord",
    publisher: "Discord Inc.",
    category: "Internet",
    accentColor: "#5865F2",
    brandColor: "#5865F2",
    summary: "Voice, video and text chat platform for communities and friends",
    fullDescription:
      "Discord is an all-in-one communication platform where you can talk over low-latency voice, share high-framerate screens, and chat with friends and worldwide communities. Servers are organized into topic-based channels where you can collaborate, share, or just talk about your day.",
    features: [
      "Crystal clear low-latency voice channels with Krisp noise suppression",
      "Screen sharing and stream broadcasting in up to 1080p 60fps",
      "Organized server hierarchy with customizable roles and permissions",
      "Direct messaging, group calls, and media sharing",
    ],
    website: "https://discord.com",
    issueTracker: "https://support.discord.com",
    architecture: "x86_64",
    relatedApps: ["telegram-desktop", "spotify", "steam", "firefox"],
    isCuratedApp: true,
  },
  steam: {
    displayName: "Steam",
    publisher: "Valve Corporation",
    category: "Games",
    accentColor: "#171A21",
    brandColor: "#171A21",
    summary: "Ultimate entertainment platform — play thousands of games on Linux",
    fullDescription:
      "Steam is the premier digital distribution platform for PC gaming. On Linux, Steam revolutionizes gaming with Proton (Steam Play), enabling tens of thousands of Windows titles to run natively with zero configuration.  Features include automatic cloud saves, Steam Workshop modding, community forums, broadcasting, and full controller remapping support.",
    features: [
      "Proton compatibility layer running thousands of Windows games on Linux",
      "Steam Cloud automatic cross-device save synchronization",
      "Steam Workshop for one-click mod downloads and community creations",
      "Big Picture UI optimized for TV and handheld gaming",
    ],
    website: "https://store.steampowered.com",
    issueTracker: "https://github.com/ValveSoftware/steam-for-linux/issues",
    architecture: "x86_64",
    relatedApps: ["discord", "obs-studio", "spotify"],
    isCuratedApp: true,
  },
  vlc: {
    displayName: "VLC Media Player",
    publisher: "VideoLAN",
    category: "Multimedia",
    accentColor: "#FF8800",
    brandColor: "#FF8800",
    summary: "Free and open-source cross-platform multimedia player",
    fullDescription:
      "VLC is a free and open source cross-platform multimedia player and framework that plays most multimedia files as well as DVDs, Audio CDs, VCDs, and various streaming protocols. It contains its own internal codecs, freeing users from having to install third-party codec packs.",
    features: [
      "Plays virtually all video and audio formats without codec packs",
      "Hardware decoding acceleration on modern GPUs",
      "Network streaming support (HTTP, RTP, RTSP, MMS)",
      "Subtitle synchronization and real-time audio/video filters",
    ],
    tagline: "Plays everything, everywhere",
    documentationUrl: "https://www.videolan.org/support/",
    highlights: [
      "Plays All Formats & Codecs",
      "Hardware GPU Acceleration",
      "Network Stream Playback",
      "Audio & Subtitle Sync"
    ],
    website: "https://www.videolan.org/vlc",
    sourceRepository: "https://code.videolan.org/videolan/vlc",
    issueTracker: "https://trac.videolan.org/vlc",
    architecture: "x86_64",
    iconUrl: "/assets/apps/vlc/icon.svg",
    relatedApps: ["mpv", "obs-studio", "spotify"],
    isCuratedApp: true,
  },
  gimp: {
    displayName: "GIMP",
    publisher: "The GIMP Development Team",
    category: "Graphics",
    accentColor: "#5C5543",
    brandColor: "#5C5543",
    summary: "GNU Image Manipulation Program — professional photo editing",
    fullDescription:
      "GIMP is an acronym for GNU Image Manipulation Program. It is a freely distributed program for such tasks as photo retouching, image composition and image authoring. It has many capabilities: it can be used as a simple paint program, an expert quality photo retouching program, an online batch processing system, or a mass production image renderer.",
    features: [
      "Full suite of painting tools including Brush, Pencil, Airbrush, and Clone",
      "Sub-pixel sampling for high-quality anti-aliasing across all brushes",
      "Advanced layers, channels, and customizable masks",
      "Multi-format support for RAW, PSD, TIFF, PNG, and SVG",
    ],
    tagline: "The free & open source image editor",
    documentationUrl: "https://www.gimp.org/docs/",
    highlights: [
      "Professional Photo Retouching",
      "Extensible Python Plugins",
      "Multi-Layer Composition",
      "Comprehensive File Formats"
    ],
    website: "https://www.gimp.org",
    sourceRepository: "https://gitlab.gnome.org/GNOME/gimp",
    issueTracker: "https://gitlab.gnome.org/GNOME/gimp/-/issues",
    architecture: "x86_64",
    iconUrl: "/assets/apps/gimp/icon.svg",
    screenshots: [
      {
        url: "/assets/apps/gimp/screen1.jpg",
        caption: "GIMP Professional Digital Photo Editing Workspace",
      },
    ],
    relatedApps: ["inkscape", "blender", "obs-studio"],
    isCuratedApp: true,
  },
  blender: {
    displayName: "Blender",
    publisher: "Blender Foundation",
    category: "Graphics",
    accentColor: "#EA7600",
    brandColor: "#EA7600",
    summary: "Free and open 3D creation suite — modeling, VFX, animation",
    fullDescription:
      "Blender is the free and open source 3D creation suite. It supports the entirety of the 3D pipeline—modeling, rigging, animation, simulation, rendering, compositing and motion tracking, even video editing and game creation. Built for professionals and hobbyists alike.",
    features: [
      "Complete 3D modeling, sculpting, and retopology toolset",
      "Cycles ray-tracing and EEVEE real-time GPU rendering engines",
      "Advanced rigging, character animation, and motion tracking",
      "VFX, compositing, and integrated Python scripting API",
    ],
    tagline: "Open source 3D creation suite",
    documentationUrl: "https://docs.blender.org",
    highlights: [
      "3D Modeling & Sculpting",
      "Real-time EEVEE & Cycles",
      "VFX & Character Rigging",
      "Integrated Video Sequencer"
    ],
    website: "https://www.blender.org",
    sourceRepository: "https://projects.blender.org/blender/blender",
    issueTracker: "https://projects.blender.org/blender/blender/issues",
    architecture: "x86_64",
    iconUrl: "/assets/apps/blender/icon.svg",
    screenshots: [
      {
        url: "/assets/apps/blender/screen1.jpg",
        caption: "Blender 3D Modeling Workspace and Viewport",
      },
    ],
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
      "OBS Studio is a free and open-source program for video recording and live streaming. Capture and mix video/audio in real time, create scenes made of multiple sources (window captures, images, text, browser windows, webcams), and broadcast smoothly to YouTube, Twitch, Kick, and custom RTMP destinations.",
    features: [
      "High-performance real-time video and audio capturing and mixing",
      "Unlimited scenes with custom seamless transitions",
      "Intuitive audio mixer with per-source noise suppression and gain filters",
      "Native direct streaming to Twitch, YouTube, Kick, and custom RTMP",
    ],
    tagline: "Broadcast your world in real-time",
    documentationUrl: "https://obsproject.com/wiki",
    highlights: [
      "Real-time Video/Audio Capture",
      "Multi-Scene Compositing",
      "Per-Source Audio Filters",
      "Native Twitch & YouTube RTMP"
    ],
    website: "https://obsproject.com",
    sourceRepository: "https://github.com/obsproject/obs-studio",
    issueTracker: "https://github.com/obsproject/obs-studio/issues",
    architecture: "x86_64",
    relatedApps: ["vlc", "blender", "gimp", "discord"],
    isCuratedApp: true,
  },
  alacritty: {
    displayName: "Alacritty",
    publisher: "Alacritty Contributors",
    category: "Utilities",
    accentColor: "#F46036",
    brandColor: "#F46036",
    summary: "A cross-platform, OpenGL terminal emulator",
    fullDescription:
      "Alacritty is a modern terminal emulator that comes with sensible defaults, but allows for extensive configuration. By integrating with other applications, rather than reimplementing their functionality, it manages to provide a flexible set of features with high performance.",
    features: [
      "GPU-accelerated rendering utilizing OpenGL",
      "Minimalist, lightweight resource footprint",
      "Rich YAML/TOML configuration and color themes",
      "Vi mode navigation and URL clicking",
    ],
    website: "https://alacritty.org",
    sourceRepository: "https://github.com/alacritty/alacritty",
    issueTracker: "https://github.com/alacritty/alacritty/issues",
    architecture: "x86_64",
    iconUrl: "/assets/apps/alacritty/icon.svg",
    relatedApps: ["kitty", "neovim", "visual-studio-code-bin"],
    isCuratedApp: true,
  },
  kitty: {
    displayName: "Kitty",
    publisher: "Kovid Goyal",
    category: "Utilities",
    accentColor: "#1F232A",
    brandColor: "#1F232A",
    summary: "Fast, feature-rich, GPU based terminal emulator",
    fullDescription:
      "Kitty is the fast, feature-rich, GPU based terminal emulator. It offloads rendering to the GPU for lower system load and buttery smooth scrolling. It uses threaded rendering for absolute minimal latency.",
    features: [
      "GPU offloaded rendering with threaded pipeline",
      "Native graphics protocol for in-terminal image viewing",
      "Split windows and tab layouts without tmux",
      "Kittens framework for extensible terminal automation",
    ],
    website: "https://sw.kovidgoyal.net/kitty",
    sourceRepository: "https://github.com/kovidgoyal/kitty",
    issueTracker: "https://github.com/kovidgoyal/kitty/issues",
    architecture: "x86_64",
    relatedApps: ["alacritty", "neovim", "visual-studio-code-bin"],
    isCuratedApp: true,
  },
  neovim: {
    displayName: "Neovim",
    publisher: "Neovim Project",
    category: "Development",
    accentColor: "#57A143",
    brandColor: "#57A143",
    summary: "Vim-fork focused on extensibility and usability",
    fullDescription:
      "Neovim is a refactor, and sometimes redactor, in the tradition of Vim. It is not a rewrite but rather a continuation and extension of Vim with a modern architecture, built-in LSP client, Lua configuration engine, and asynchronous plugin framework.",
    features: [
      "Built-in Language Server Protocol (LSP) client",
      "Native Lua 5.1 / LuaJIT scripting engine",
      "Tree-sitter integration for precise syntax highlighting",
      "Asynchronous remote plugin architecture via RPC",
    ],
    website: "https://neovim.io",
    sourceRepository: "https://github.com/neovim/neovim",
    issueTracker: "https://github.com/neovim/neovim/issues",
    architecture: "x86_64",
    relatedApps: ["visual-studio-code-bin", "kitty", "alacritty", "git"],
    isCuratedApp: true,
  },
  git: {
    displayName: "Git",
    publisher: "Software Freedom Conservancy",
    category: "Development",
    accentColor: "#F05032",
    brandColor: "#F05032",
    summary: "Fast, scalable, distributed revision control system",
    fullDescription:
      "Git is a free and open source distributed version control system designed to handle everything from small to very large projects with speed and efficiency. Easy to learn with tiny footprint and lightning fast performance.",
    features: [
      "Distributed architecture with complete local history",
      "Branching and merging workflows with lightning speed",
      "Cryptographic integrity with SHA-1 / SHA-256 commit hashing",
      "Standard foundation for modern software development",
    ],
    website: "https://git-scm.com",
    sourceRepository: "https://github.com/git/git",
    issueTracker: "https://git-scm.com/community",
    architecture: "x86_64",
    relatedApps: ["visual-studio-code-bin", "neovim", "alacritty"],
    isCuratedApp: true,
  },
  spotify: {
    displayName: "Spotify",
    publisher: "Spotify AB",
    category: "Multimedia",
    accentColor: "#1ED760",
    brandColor: "#1ED760",
    summary: "Digital music service giving access to millions of songs",
    fullDescription:
      "Spotify is a digital music, podcast, and video service that gives you access to millions of songs and other content from creators all over the world. Build playlists, discover personalized recommendations, and listen across all your devices.",
    features: [
      "On-demand streaming of over 100 million songs and podcasts",
      "Personalized playlists like Discover Weekly and Daily Mixes",
      "Spotify Connect to stream seamlessly to speakers and TVs",
      "Curated audiobooks, podcasts, and artist radio stations",
    ],
    website: "https://www.spotify.com",
    architecture: "x86_64",
    relatedApps: ["discord", "vlc", "obs-studio"],
    isCuratedApp: true,
  },
  btop: {
    displayName: "btop",
    publisher: "Aristocratos",
    category: "System",
    accentColor: "#FF5555",
    brandColor: "#FF5555",
    summary: "Resource monitor that shows usage and stats for processor, memory, disks and network",
    fullDescription:
      "btop++ is an aesthetic Linux resource monitor in C++. Shows usage and stats for processor, memory, disks, network and processes. Responsive game-like UI with full mouse support and customizable color schemes.",
    features: [
      "Real-time CPU, memory, disk, and network graphs",
      "Interactive process tree with filtering and sorting",
      "Full mouse support and keyboard shortcuts",
      "Customizable themes and layout presets",
    ],
    website: "https://github.com/aristocratos/btop",
    sourceRepository: "https://github.com/aristocratos/btop",
    issueTracker: "https://github.com/aristocratos/btop/issues",
    architecture: "x86_64",
    relatedApps: ["kitty", "alacritty", "neovim"],
    isCuratedApp: true,
  },
  thunderbird: {
    displayName: "Thunderbird",
    publisher: "MZLA Technologies Corporation",
    category: "Internet",
    accentColor: "#0A84FF",
    brandColor: "#0A84FF",
    summary: "Free and open-source email, newsfeed, chat, and calendar client",
    fullDescription:
      "Thunderbird is a free, open-source, multiplatform application for managing email, newsfeeds, chat, and calendars. It is easy to set up and customize, loaded with features, and prioritizes user privacy and security.",
    features: [
      "Unified inbox for multiple email accounts (IMAP/POP3)",
      "Integrated calendar with task scheduling and reminders",
      "OpenPGP and S/MIME end-to-end email encryption",
      "Tabbed email browsing and powerful search filters",
    ],
    website: "https://www.thunderbird.net",
    sourceRepository: "https://hg.mozilla.org/comm-central",
    issueTracker: "https://bugzilla.mozilla.org",
    architecture: "x86_64",
    relatedApps: ["firefox", "telegram-desktop"],
    isCuratedApp: true,
  },
  inkscape: {
    displayName: "Inkscape",
    publisher: "Inkscape Community",
    category: "Graphics",
    accentColor: "#000000",
    brandColor: "#000000",
    summary: "Professional vector graphics editor for Linux, Windows and macOS",
    fullDescription:
      "Inkscape is a professional vector graphics editor for Linux, Windows and macOS. It is free and open source, using standard W3C SVG as its native format. Used by design professionals and hobbyists worldwide for creating a wide variety of graphics such as illustrations, icons, logos, diagrams, maps and web graphics.",
    features: [
      "Full compliance with W3C Scalable Vector Graphics (SVG) standards",
      "Object creation with pencil, pen, calligraphy, and shape tools",
      "Node editing, path operations, and boolean transformations",
      "Comprehensive text support with multi-line text and text on paths",
    ],
    website: "https://inkscape.org",
    sourceRepository: "https://gitlab.com/inkscape/inkscape",
    issueTracker: "https://gitlab.com/inkscape/inkscape/-/issues",
    architecture: "x86_64",
    iconUrl: "/assets/apps/inkscape/icon.svg",
    relatedApps: ["gimp", "blender"],
    isCuratedApp: true,
  },
  mpv: {
    displayName: "mpv",
    publisher: "mpv-player team",
    category: "Multimedia",
    accentColor: "#5C1542",
    brandColor: "#5C1542",
    summary: "Command line video player with minimalist on-screen controller",
    fullDescription:
      "mpv is a free (as in freedom) media player for the command line. It supports a wide variety of media file formats, audio and video codecs, and subtitle types. Featuring high quality video output with color management, interpolation, and hardware acceleration.",
    features: [
      "Minimalist On-Screen Controller (OSC) without clunky window chrome",
      "High quality video scaling, HDR tone mapping, and interpolation",
      "GPU hardware decoding via VA-API and NVDEC",
      "Extensive Lua and JavaScript scripting capabilities",
    ],
    website: "https://mpv.io",
    sourceRepository: "https://github.com/mpv-player/mpv",
    issueTracker: "https://github.com/mpv-player/mpv/issues",
    architecture: "x86_64",
    relatedApps: ["vlc", "spotify", "obs-studio"],
    isCuratedApp: true,
  },
  telegram: {
    displayName: "Telegram Desktop",
    publisher: "Telegram FZ-LLC",
    category: "Internet",
    accentColor: "#229ED9",
    brandColor: "#229ED9",
    summary: "Fast, cloud-based messaging app with sync across all devices",
    fullDescription:
      "Telegram Desktop is the official native desktop client for Telegram. Pure instant messaging—simple, fast, secure, and synced across all your devices. Send messages, photos, videos and files of any type (doc, zip, mp3, etc), as well as create groups for up to 200,000 people or channels for broadcasting to unlimited audiences.",
    features: [
      "Instant multi-device cloud synchronization",
      "Group chats up to 200,000 members and broadcast channels",
      "Large file and media transfers up to 2 GB per file",
      "End-to-end encrypted voice and video calls",
    ],
    website: "https://desktop.telegram.org",
    sourceRepository: "https://github.com/telegramdesktop/tdesktop",
    issueTracker: "https://github.com/telegramdesktop/tdesktop/issues",
    architecture: "x86_64",
    relatedApps: ["discord", "firefox", "thunderbird"],
    isCuratedApp: true,
  },
  "telegram-desktop": {
    displayName: "Telegram Desktop",
    publisher: "Telegram FZ-LLC",
    category: "Internet",
    accentColor: "#229ED9",
    brandColor: "#229ED9",
    summary: "Fast, cloud-based messaging app with sync across all devices",
    fullDescription:
      "Telegram Desktop is the official native desktop client for Telegram. Pure instant messaging—simple, fast, secure, and synced across all your devices. Send messages, photos, videos and files of any type (doc, zip, mp3, etc), as well as create groups for up to 200,000 people or channels for broadcasting to unlimited audiences.",
    features: [
      "Instant multi-device cloud synchronization",
      "Group chats up to 200,000 members and broadcast channels",
      "Large file and media transfers up to 2 GB per file",
      "End-to-end encrypted voice and video calls",
    ],
    website: "https://desktop.telegram.org",
    sourceRepository: "https://github.com/telegramdesktop/tdesktop",
    issueTracker: "https://github.com/telegramdesktop/tdesktop/issues",
    architecture: "x86_64",
    relatedApps: ["discord", "firefox", "thunderbird"],
    isCuratedApp: true,
  },
};

export function resolveAppMetadata(packageId: string, title?: string): AppMetadata {
  const normId = packageId.toLowerCase().trim();

  // 1. Direct match
  if (KNOWN_APPS[normId]) {
    return KNOWN_APPS[normId];
  }

  // 2. Fuzzy match by prefix/suffix
  for (const [key, meta] of Object.entries(KNOWN_APPS)) {
    if (normId.startsWith(key) || normId.endsWith(key) || normId.includes(key)) {
      return meta;
    }
  }

  // 3. Fallback for generic pacman packages
  const cleanTitle = title || packageId.charAt(0).toUpperCase() + packageId.slice(1);
  return {
    displayName: cleanTitle,
    publisher: "Arch Linux / Community",
    category: "Utilities",
    accentColor: "#64748B",
    brandColor: "#64748B",
    summary: `${cleanTitle} package for Arch Linux`,
    website: `https://archlinux.org/packages/?q=${encodeURIComponent(packageId)}`,
    architecture: "x86_64",
    isCuratedApp: false,
  };
}
