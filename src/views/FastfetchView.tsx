import { resolveLocalAssetSrc } from "../providers/customMediaProvider.ts";
import React, { useState, useEffect, useRef, useMemo } from "react";
import {
  Terminal,
  RotateCcw,
  Save,
  Plus,
  Trash2,
  Upload,
  ChevronUp,
  ChevronDown,
  Eye,
  EyeOff,
  Search,
  FileCode,
  Sliders,
  X,
  ArrowLeft,
  Check,
  CheckCircle2,
  MoreHorizontal,
  Palette,
  Layout as LayoutIcon,
  Info,
  Image as ImageIcon,
  Loader2,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export interface FastfetchRowItem {
  id: string;
  type: "host_user" | "tagline" | "section_header" | "spacer" | "telemetry";
  label?: string;
  keyPrefix?: string;
  customText?: string;
  telemetryKey?: string;
  value?: string;
  enabled: boolean;
}

export type EmblemType = "image" | "ascii" | "builtin" | "none";

export interface FastfetchEmblem {
  id: string;
  name: string;
  path: string;
  is_image: boolean;
}

const INITIAL_ROWS: FastfetchRowItem[] = [
  { id: "r-host", type: "host_user", enabled: true },
  {
    id: "r-tagline",
    type: "tagline",
    customText: "Ryzora — hand-crafted Linux desktop",
    enabled: true,
  },
  { id: "r-sp1", type: "spacer", enabled: true },
  { id: "r-h-vitals", type: "section_header", customText: "VITALS", enabled: true },
  {
    id: "r-cpu",
    type: "telemetry",
    label: "CPU",
    keyPrefix: " ",
    telemetryKey: "cpu",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-gpu-d",
    type: "telemetry",
    label: "GPU",
    keyPrefix: "󰢮 ",
    telemetryKey: "gpu",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-mem",
    type: "telemetry",
    label: "MEMORY",
    keyPrefix: "󰘚 ",
    telemetryKey: "memory",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-disk1",
    type: "telemetry",
    label: "DISK",
    keyPrefix: "󰋊 ",
    telemetryKey: "disk",
    value: "Detecting…",
    enabled: true,
  },
  { id: "r-sp2", type: "spacer", enabled: true },
  { id: "r-h-sys", type: "section_header", customText: "SYSTEM", enabled: true },
  {
    id: "r-os",
    type: "telemetry",
    label: "OS",
    keyPrefix: " ",
    telemetryKey: "os",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-kernel",
    type: "telemetry",
    label: "KERNEL",
    keyPrefix: " ",
    telemetryKey: "kernel",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-wm",
    type: "telemetry",
    label: "WM",
    keyPrefix: "󱂬 ",
    telemetryKey: "wm",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-shell",
    type: "telemetry",
    label: "SHELL",
    keyPrefix: " ",
    telemetryKey: "shell",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-pkgs",
    type: "telemetry",
    label: "PKGS",
    keyPrefix: "󰏖 ",
    telemetryKey: "packages",
    value: "Detecting…",
    enabled: true,
  },
  { id: "r-sp3", type: "spacer", enabled: true },
  { id: "r-h-sess", type: "section_header", customText: "SESSION", enabled: true },
  {
    id: "r-uptime",
    type: "telemetry",
    label: "UPTIME",
    keyPrefix: "󱑂 ",
    telemetryKey: "uptime",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-age",
    type: "telemetry",
    label: "AGE",
    keyPrefix: "󰃮 ",
    telemetryKey: "age",
    value: "Detecting…",
    enabled: true,
  },
  {
    id: "r-term",
    type: "telemetry",
    label: "TERM",
    keyPrefix: " ",
    telemetryKey: "terminal",
    value: "Detecting…",
    enabled: true,
  },
];

export interface FastfetchViewProps {
  initialStyleId?: string;
  initialEmblemId?: string;
  onConsumeInitialPresetId?: () => void;
  onConsumeInitialEmblemId?: () => void;
}


import { parseFastfetchAnsi, stripJsoncComments } from "../services/fastfetchAnsi.ts";

export const FastfetchView: React.FC<FastfetchViewProps> = ({
  initialStyleId,
  initialEmblemId,
  onConsumeInitialPresetId,
  onConsumeInitialEmblemId,
}) => {
  const {
    packages,
    installedPackages,
    installedPackageIds,
    setActiveCategory,
    systemInfo,
    setToast,
    selectedPackage,
  } = useApp();

  // Fastfetch style/preset packages in catalog (excluding standalone emblems)
  const fastfetchPackages = packages.filter(
    (p) =>
      (p.category === "fastfetch" || p.package_type === "fastfetch") &&
      !p.tags?.some((t) => t.toLowerCase() === "emblem") &&
      p.subfilter !== "Emblems" &&
      !p.id.includes("emblem")
  );

  const [fsInstalledPresetIds, setFsInstalledPresetIds] = useState<string[]>([]);

  useEffect(() => {
    invoke<string[]>("list_fastfetch_installed_presets")
      .then((ids) => {
        if (Array.isArray(ids)) {
          setFsInstalledPresetIds(ids);
        }
      })
      .catch(() => {});
  }, []);

  // Filter to ONLY installed fastfetch packages (or active/staged preset)
  const isInstalledPkg = (id: string) =>
    Boolean(
      installedPackageIds?.includes(id) ||
      installedPackages?.some((rec) => rec.package_id === id) ||
      fsInstalledPresetIds.includes(id)
    );

  const installedFastfetchPackages = useMemo(() => {
    const list = [
      ...fastfetchPackages.filter(
        (p) => isInstalledPkg(p.id) || (initialStyleId && p.id === initialStyleId)
      ),
    ];
    for (const fsId of fsInstalledPresetIds) {
      if (!list.some((p) => p.id === fsId)) {
        const catalogPkg = packages.find((p) => p.id === fsId);
        if (catalogPkg) {
          list.push(catalogPkg);
        } else {
          const cleanName = fsId.replace(/^fastfetch-/, "").replace(/[-_]/g, " ");
          const title = cleanName.charAt(0).toUpperCase() + cleanName.slice(1);
          list.push({
            id: fsId,
            title,
            name: title,
            version: "1.0.0",
            category: "fastfetch",
            package_type: "fastfetch",
            description: `Installed Fastfetch preset: ${title}`,
            tags: ["Fastfetch", "Preset"],
            preview_poster_url: "",
            components: [],
            author: { name: "Community" },
          } as any);
        }
      }
    }
    return list;
  }, [fastfetchPackages, installedPackageIds, installedPackages, fsInstalledPresetIds, initialStyleId, packages]);

  const storedActive = typeof window !== "undefined" ? localStorage.getItem("ryzora_active_fastfetch_preset") : null;
  const targetInitialId =
    (initialStyleId && installedFastfetchPackages.some((p) => p.id === initialStyleId) ? initialStyleId : null) ||
    (selectedPackage && installedFastfetchPackages.some((p) => p.id === selectedPackage.id) ? selectedPackage.id : null) ||
    (storedActive && installedFastfetchPackages.some((p) => p.id === storedActive) ? storedActive : null) ||
    installedFastfetchPackages[0]?.id ||
    fastfetchPackages[0]?.id ||
    "fastfetch-preset-2";

  // Selected Style in Left Column
  const [selectedStyleId, setSelectedStyleId] = useState<string>(targetInitialId);

  // Sync selectedStyleId when initialStyleId is provided or on initial mount
  useEffect(() => {
    if (initialStyleId) {
      handleSelectStyle(initialStyleId);
      onConsumeInitialPresetId?.();
    } else if (targetInitialId) {
      handleSelectStyle(targetInitialId);
    }
  }, [initialStyleId]);
  const [styleSearch, setStyleSearch] = useState<string>("");
  const [showStyleDropdown, setShowStyleDropdown] = useState<boolean>(false);
  const [showSecondaryMenu, setShowSecondaryMenu] = useState<boolean>(false);
  const [configTab, setConfigTab] = useState<"appearance" | "layout" | "colors" | "info">("appearance");

  // Configuration State
  const [installedEmblems, setInstalledEmblems] = useState<FastfetchEmblem[]>([]);
  const [emblemType, setEmblemType] = useState<EmblemType>("none");
  const [selectedEmblemId, setSelectedEmblemId] = useState<string>("");
  const [userEmblemOverridden, setUserEmblemOverridden] = useState<boolean>(false);
  const [missingLogoPath, setMissingLogoPath] = useState<string | null>(null);
  const [customEmblemUrl, setCustomEmblemUrl] = useState<string>("");
  const [widthCols, setWidthCols] = useState<number>(34);
  const [heightLines, setHeightLines] = useState<number>(15);
  const [paddingCols, setPaddingCols] = useState<number>(3);
  const [dither, setDither] = useState<boolean>(false);
  const [accentColor, setAccentColor] = useState<string>("#e2342a");
  const [rows, setRows] = useState<FastfetchRowItem[]>(INITIAL_ROWS);
  const [previewAnsi, setPreviewAnsi] = useState<string>("");
  const [isRenderingPreview, setIsRenderingPreview] = useState<boolean>(false);

  // Status & Edit state
  const [isSaved, setIsSaved] = useState<boolean>(true);
  const [isModified, setIsModified] = useState<boolean>(false);
  const [colorModified, setColorModified] = useState<boolean>(false);
  const [modulesModified, setModulesModified] = useState<boolean>(false);
  const [baseConfigJsonc, setBaseConfigJsonc] = useState<string>("");
  const [activeAppliedStyleId, setActiveAppliedStyleId] = useState<string>(() => {
    if (typeof window !== "undefined") {
      return localStorage.getItem("ryzora_active_fastfetch_preset") || "fastfetch-preset-2";
    }
    return "fastfetch-preset-2";
  });
  const [showConfigModal, setShowConfigModal] = useState<boolean>(false);
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [isLaunchingTerminal, setIsLaunchingTerminal] = useState<boolean>(false);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const requestSeqRef = useRef<number>(0);
  const previewCacheRef = useRef<Map<string, { raw_ansi: string; missing_logo_path?: string }>>(new Map());

  // Click outside to close style dropdown
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowStyleDropdown(false);
      }
    };
    if (showStyleDropdown) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [showStyleDropdown]);

  // Discover local installed emblems on filesystem
  useEffect(() => {
    invoke<FastfetchEmblem[]>("list_fastfetch_local_emblems")
      .then((emblems) => {
        if (emblems && Array.isArray(emblems)) {
          setInstalledEmblems(emblems);
          if (initialEmblemId) {
            const matched = emblems.find(
              (e) =>
                e.id === initialEmblemId ||
                e.id.toLowerCase() === initialEmblemId.toLowerCase() ||
                e.id === initialEmblemId.replace("fastfetch-emblem-", "") ||
                e.path.includes(initialEmblemId) ||
                (initialEmblemId.includes(e.id) && e.id.length > 2)
            );
            if (matched) {
              setUserEmblemOverridden(true);
              setSelectedEmblemId(matched.id);
              setCustomEmblemUrl("");
              setEmblemType("image");
              setIsSaved(false);
              setIsModified(true);
            }
            if (onConsumeInitialEmblemId) onConsumeInitialEmblemId();
          }
        }
      })
      .catch(() => {});
  }, [initialEmblemId]);

  // Fetch real host telemetry without fake hardware values
  useEffect(() => {
    invoke<any>("get_fastfetch_telemetry")
      .then((tel) => {
        if (!tel) return;
        setRows((prev) =>
          prev.map((row) => {
            if (row.telemetryKey === "cpu") return { ...row, value: tel.cpu || "Unavailable" };
            if (row.telemetryKey === "gpu") return { ...row, value: tel.gpu || "Unavailable" };
            if (row.telemetryKey === "memory") return { ...row, value: tel.memory || "Unavailable" };
            if (row.telemetryKey === "disk") return { ...row, value: tel.disk || "Unavailable" };
            if (row.telemetryKey === "os") return { ...row, value: tel.os || (systemInfo?.distro_name ? `${systemInfo.distro_name} x86_64` : "Unavailable") };
            if (row.telemetryKey === "kernel") return { ...row, value: tel.kernel || (systemInfo?.kernel_version ? `Linux ${systemInfo.kernel_version}` : "Unavailable") };
            if (row.telemetryKey === "wm") return { ...row, value: tel.wm || systemInfo?.window_manager || "Unavailable" };
            if (row.telemetryKey === "shell") return { ...row, value: tel.shell || systemInfo?.shell || "Unavailable" };
            if (row.telemetryKey === "terminal") return { ...row, value: tel.terminal || systemInfo?.terminal || "Unavailable" };
            if (row.telemetryKey === "packages") return { ...row, value: tel.packages || "Unavailable" };
            if (row.telemetryKey === "uptime") return { ...row, value: tel.uptime || "Unavailable" };
            if (row.telemetryKey === "age") return { ...row, value: tel.age || "Unavailable" };
            return row;
          })
        );
      })
      .catch(() => {});
  }, [systemInfo]);

  // Live Fastfetch execution preview pipeline with request sequencing & caching
  const renderPreview = async () => {
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    const explicitEmblemPath =
      userEmblemOverridden && (customEmblemUrl || selectedEmblem?.path)
        ? customEmblemUrl || selectedEmblem?.path
        : undefined;

    const cacheKey = JSON.stringify({
      presetId: selectedStyleId,
      baseConfigJsonc,
      isModified,
      accentColor: colorModified ? accentColor : undefined,
      emblemType,
      emblemPath: explicitEmblemPath,
      widthCols,
      heightLines,
      paddingCols,
      dither,
      activeModules: modulesModified
        ? rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type))
        : undefined,
    });

    if (previewCacheRef.current.has(cacheKey)) {
      const cached = previewCacheRef.current.get(cacheKey)!;
      setPreviewAnsi(cached.raw_ansi);
      setMissingLogoPath(cached.missing_logo_path || null);
      setIsRenderingPreview(false);
      return;
    }

    const currentSeq = ++requestSeqRef.current;
    setIsRenderingPreview(true);

    try {
      const res = await invoke<{
        raw_ansi: string;
        success: boolean;
        missing_logo_path?: string;
      }>("render_fastfetch_preview", {
        req: {
          presetId: selectedStyleId,
          baseConfigJsonc,
          isModified,
          accentColor: colorModified ? accentColor : undefined,
          emblemType,
          emblemPath: explicitEmblemPath,
          widthCols,
          heightLines,
          paddingCols,
          dither,
          activeModules: modulesModified
            ? rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type))
            : undefined,
        },
      });
      if (currentSeq === requestSeqRef.current && res && res.raw_ansi) {
        previewCacheRef.current.set(cacheKey, {
          raw_ansi: res.raw_ansi,
          missing_logo_path: res.missing_logo_path,
        });
        setPreviewAnsi(res.raw_ansi);
        setMissingLogoPath(res.missing_logo_path || null);
      }
    } catch (e) {
      if (currentSeq === requestSeqRef.current) {
        console.error("Preview render failed:", e);
      }
    } finally {
      if (currentSeq === requestSeqRef.current) {
        setIsRenderingPreview(false);
      }
    }
  };

  const activeModulesKey = useMemo(() => {
    return modulesModified
      ? rows
          .filter((r) => r.enabled)
          .map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type))
          .join(",")
      : "";
  }, [modulesModified, rows]);

  // Trigger live Fastfetch execution whenever parameters change (with 300ms debounce)
  useEffect(() => {
    const timer = setTimeout(() => {
      renderPreview();
    }, 300);
    return () => clearTimeout(timer);
  }, [
    selectedStyleId,
    baseConfigJsonc,
    isModified,
    colorModified,
    accentColor,
    emblemType,
    selectedEmblemId,
    customEmblemUrl,
    widthCols,
    heightLines,
    paddingCols,
    dither,
    activeModulesKey,
  ]);

  const applyParsedLogoDimensions = (logo: any) => {
    const hasW = typeof logo?.width === "number" && logo.width > 0;
    const hasH = typeof logo?.height === "number" && logo.height > 0;
    if (hasW && hasH) {
      setWidthCols(logo.width);
      setHeightLines(logo.height);
    } else if (hasH) {
      setHeightLines(logo.height);
      setWidthCols(Math.round(logo.height * 2.2));
    } else if (hasW) {
      setWidthCols(logo.width);
      setHeightLines(Math.max(8, Math.round(logo.width / 2.2)));
    } else {
      setWidthCols(34);
      setHeightLines(15);
    }
    const rightPad =
      typeof logo?.padding?.right === "number"
        ? logo.padding.right
        : typeof logo?.padding?.left === "number" && logo.padding.left > 0
        ? logo.padding.left
        : 3;
    setPaddingCols(rightPad);
  };

  // When a style preset is selected from the left column
  const handleSelectStyle = async (pkgId: string) => {
    setSelectedStyleId(pkgId);
    setIsModified(false);
    setIsSaved(true);
    setColorModified(false);
    setModulesModified(false);
    setUserEmblemOverridden(false);
    setMissingLogoPath(null);
    setCustomEmblemUrl("");
    const found = fastfetchPackages.find((p) => p.id === pkgId);
    try {
      const configStr = await invoke<string>("get_fastfetch_preset_config", { presetId: pkgId });
      if (configStr) {
        let newEmblemType: EmblemType = "none";
        let newEmblemId = "";
        let newWidth = 34;
        let newHeight = 15;
        let newPad = 3;
        let newAccent = found?.color_palette?.[0] || "#e2342a";

        const cleanJson = stripJsoncComments(configStr);
        try {
          const parsed = JSON.parse(cleanJson);
          if (parsed.logo) {
            if (parsed.logo.type === "none") {
              newEmblemType = "none";
            } else if (parsed.logo.type === "builtin") {
              newEmblemType = "builtin";
            } else if (parsed.logo.type === "file" || parsed.logo.type?.includes("ascii")) {
              newEmblemType = "ascii";
            } else if (parsed.logo.source) {
              newEmblemType = "image";
              const matchingEmblem = installedEmblems.find(
                (e) =>
                  e.path === parsed.logo.source ||
                  e.id === parsed.logo.source ||
                  parsed.logo.source.endsWith("/" + e.id + ".png")
              );
              if (matchingEmblem) {
                newEmblemId = matchingEmblem.id;
              }
            }
            const hasW = typeof parsed.logo?.width === "number" && parsed.logo.width > 0;
            const hasH = typeof parsed.logo?.height === "number" && parsed.logo.height > 0;
            if (hasW && hasH) {
              newWidth = parsed.logo.width;
              newHeight = parsed.logo.height;
            } else if (hasH) {
              newHeight = parsed.logo.height;
              newWidth = Math.round(parsed.logo.height * 2.2);
            } else if (hasW) {
              newWidth = parsed.logo.width;
              newHeight = Math.max(8, Math.round(parsed.logo.width / 2.2));
            }
            newPad =
              typeof parsed.logo?.padding?.right === "number"
                ? parsed.logo.padding.right
                : typeof parsed.logo?.padding?.left === "number" && parsed.logo.padding.left > 0
                ? parsed.logo.padding.left
                : 3;
          }
          if (parsed.display?.color?.keys?.startsWith("38;2;")) {
            const parts = parsed.display.color.keys.split(";");
            if (parts.length >= 5) {
              const r = parseInt(parts[2]).toString(16).padStart(2, "0");
              const g = parseInt(parts[3]).toString(16).padStart(2, "0");
              const b = parseInt(parts[4]).toString(16).padStart(2, "0");
              newAccent = `#${r}${g}${b}`;
            }
          }
        } catch {}

        setEmblemType(newEmblemType);
        setSelectedEmblemId(newEmblemId);
        setWidthCols(newWidth);
        setHeightLines(newHeight);
        setPaddingCols(newPad);
        setAccentColor(newAccent);
        setBaseConfigJsonc(configStr);
      }
    } catch {}
    setToast({
      message: `Loaded starting style "${found?.title || pkgId}".`,
      type: "info",
    });
  };

  const handleApplyInstalledStyle = async () => {
    setActiveAppliedStyleId(selectedStyleId);
    try {
      localStorage.setItem("ryzora_active_fastfetch_preset", selectedStyleId);
    } catch {}
    await handleSave();
  };

  const handleRemoveInstalledStyle = () => {
    const found = fastfetchPackages.find((p) => p.id === selectedStyleId);
    setToast({
      message: `Removed installed style "${found?.title || selectedStyleId}".`,
      type: "info",
    });
  };

  const handleResetToDefault = () => {
    setEmblemType("none");
    setSelectedEmblemId("");
    setCustomEmblemUrl("");
    setWidthCols(34);
    setHeightLines(15);
    setPaddingCols(3);
    setDither(false);
    setAccentColor("#e2342a");
    setColorModified(true);
    setRows(INITIAL_ROWS);
    setModulesModified(true);
    setIsSaved(false);
    setIsModified(true);
    setToast({
      message: "Reset Fastfetch configuration to default layout.",
      type: "info",
    });
  };

  const handleRevert = async () => {
    setIsModified(false);
    setIsSaved(true);
    setColorModified(false);
    setModulesModified(false);
    if (baseConfigJsonc) {
      try {
        const clean = stripJsoncComments(baseConfigJsonc);
        const parsed = JSON.parse(clean);
        if (parsed.logo) {
          if (parsed.logo.type === "none") setEmblemType("none");
          else if (parsed.logo.type === "builtin") setEmblemType("builtin");
          else if (parsed.logo.type === "file" || parsed.logo.type?.includes("ascii")) setEmblemType("ascii");
          else if (parsed.logo.source) setEmblemType("image");
          applyParsedLogoDimensions(parsed.logo);
        }
      } catch {}
    }
    setToast({
      message: "Reverted unsaved changes to exact original preset.",
      type: "info",
    });
  };

  const handleSave = async () => {
    setIsSaving(true);
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    const explicitEmblemPath =
      userEmblemOverridden && (customEmblemUrl || selectedEmblem?.path)
        ? customEmblemUrl || selectedEmblem?.path
        : undefined;
    try {
      await invoke("apply_fastfetch_configuration", {
        req: {
          presetId: selectedStyleId,
          baseConfigJsonc,
          isModified,
          accentColor: colorModified ? accentColor : undefined,
          emblemType,
          emblemPath: explicitEmblemPath,
          widthCols,
          heightLines,
          paddingCols,
          dither,
          activeModules: modulesModified
            ? rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type))
            : undefined,
        },
      });
      setIsSaved(true);
      setIsModified(false);
      if (typeof window !== "undefined") {
        localStorage.setItem("ryzora_active_fastfetch_preset", selectedStyleId);
      }
      setActiveAppliedStyleId(selectedStyleId);
      setToast({
        message: "Saved configuration to ~/.config/fastfetch/config.jsonc.",
        type: "success",
      });
    } catch (err: any) {
      setToast({
        message: `Failed to save configuration: ${err?.message || String(err)}`,
        type: "warning",
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handlePreviewInTerminal = async () => {
    setIsLaunchingTerminal(true);
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    const explicitEmblemPath =
      userEmblemOverridden && (customEmblemUrl || selectedEmblem?.path)
        ? customEmblemUrl || selectedEmblem?.path
        : undefined;
    try {
      await invoke("preview_fastfetch_terminal", {
        req: {
          presetId: selectedStyleId,
          baseConfigJsonc,
          isModified,
          accentColor: colorModified ? accentColor : undefined,
          emblemType,
          emblemPath: explicitEmblemPath,
          widthCols,
          heightLines,
          paddingCols,
          dither,
          activeModules: modulesModified
            ? rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type))
            : undefined,
        },
      });
      setToast({
        message: isModified
          ? "Preview in terminal launched with temporary modified configuration."
          : "Preview in terminal launched with exact preset configuration.",
        type: "info",
      });
    } catch (err: any) {
      setToast({
        message: `Failed to launch preview: ${err?.message || err}`,
        type: "warning",
      });
    } finally {
      setIsLaunchingTerminal(false);
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === "string") {
          setUserEmblemOverridden(true);
          setCustomEmblemUrl(reader.result);
          setEmblemType("image");
          setIsSaved(false);
          setIsModified(true);
        }
      };
      reader.readAsDataURL(file);
    }
  };

  const handleMoveRow = (index: number, direction: "up" | "down") => {
    const target = direction === "up" ? index - 1 : index + 1;
    if (target < 0 || target >= rows.length) return;
    const next = [...rows];
    const [moved] = next.splice(index, 1);
    next.splice(target, 0, moved);
    setRows(next);
    setModulesModified(true);
    setIsSaved(false);
    setIsModified(true);
  };

  const handleToggleRow = (id: string) => {
    setRows((prev) =>
      prev.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r))
    );
    setModulesModified(true);
    setIsSaved(false);
    setIsModified(true);
  };

  const handleRemoveRow = (id: string) => {
    setRows((prev) => prev.filter((r) => r.id !== id));
    setModulesModified(true);
    setIsSaved(false);
    setIsModified(true);
  };

  const handleUpdateRowText = (id: string, text: string) => {
    setRows((prev) =>
      prev.map((r) => (r.id === id ? { ...r, customText: text } : r))
    );
    setIsSaved(false);
    setIsModified(true);
  };

  // ANSI spectrum palette
  const ansiColors = [
    "#1e1e1e",
    accentColor || "#ef4444",
    "#22c55e",
    "#eab308",
    "#3b82f6",
    "#a855f7",
    "#06b6d4",
    "#e2e8f0",
  ];

  const COLOR_PALETTE_SWATCHES = ansiColors;

  const handleAddModule = (label: string, prefix: string, teleKey: string) => {
    const newId = `mod-${Date.now()}`;
    setRows((prev) => [
      ...prev,
      {
        id: newId,
        type: "telemetry",
        label,
        keyPrefix: prefix,
        telemetryKey: teleKey,
        value: "Enabled Telemetry Item",
        enabled: true,
      },
    ]);
    setModulesModified(true);
    setIsSaved(false);
    setIsModified(true);
  };

  // Active emblem resolution
  const activePresetEmblem = installedEmblems.find((p) => p.id === selectedEmblemId);
  const activeEmblemImage = customEmblemUrl || (activePresetEmblem ? convertFileSrc(activePresetEmblem.path) : "");

  // Filtered style list
  const filteredStyles = installedFastfetchPackages.filter((p) =>
    p.title.toLowerCase().includes(styleSearch.toLowerCase())
  );

  const selectedPackageItem = fastfetchPackages.find((p) => p.id === selectedStyleId);

  const parsedOutput = useMemo(() => parseFastfetchAnsi(previewAnsi), [previewAnsi]);

  return (
    <div className="flex flex-col h-[calc(100vh-100px)] min-h-[640px] text-[var(--rz-text)] antialiased select-none font-sans">
      {/* ── Top Header ── */}
      <div className="shrink-0 pb-4 border-b border-[var(--rz-border-subtle)] flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => setActiveCategory("discover")}
            className="p-2 rounded-xl bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] transition-colors cursor-pointer"
            title="Back to Store"
          >
            <ArrowLeft className="w-4 h-4" />
          </button>
          <div>
            <h1 className="text-xl sm:text-2xl font-extrabold tracking-tight text-[var(--rz-text)]">
              Fastfetch
            </h1>
            <p className="text-xs text-[var(--rz-text-secondary)]">
              Configure your terminal readout
            </p>
          </div>
        </div>

        {/* Current style selector & Browse Store */}
        <div className="flex items-center gap-3">
          {/* Compact Style Dropdown */}
          <div className="relative" ref={dropdownRef}>
            <div className="flex items-center gap-2">
              <span className="text-xs font-medium text-[var(--rz-text-secondary)] hidden md:inline">
                Current style:
              </span>
              <button
                type="button"
                onClick={() => setShowStyleDropdown(!showStyleDropdown)}
                className="flex items-center gap-2 px-3.5 py-1.5 rounded-xl bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-accent)] text-xs font-semibold text-[var(--rz-text)] shadow-xs transition-all cursor-pointer min-w-[210px] justify-between"
              >
                {installedFastfetchPackages.length > 0 ? (
                  <div className="flex items-center gap-2 truncate">
                    <Terminal className="w-3.5 h-3.5 text-[var(--rz-accent)] shrink-0" />
                    <span className="truncate">{selectedPackageItem?.title || "Two-Column Spectrum"}</span>
                  </div>
                ) : (
                  <span className="text-[var(--rz-text-muted)] italic">No installed styles</span>
                )}
                <ChevronDown className="w-3.5 h-3.5 text-[var(--rz-text-muted)] shrink-0" />
              </button>
            </div>

            {/* Dropdown Popover */}
            {showStyleDropdown && (
              <div className="absolute right-0 top-full mt-2 w-72 rounded-2xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-2xl p-2 z-30 space-y-2 animate-in fade-in zoom-in-95 duration-150">
                <div className="px-2 pt-1 pb-1 flex items-center justify-between border-b border-[var(--rz-border-subtle)]">
                  <span className="text-[11px] font-bold uppercase tracking-wider text-[var(--rz-text-muted)]">
                    Fastfetch Style / Presets
                  </span>
                  <span className="text-[10px] font-mono text-[var(--rz-text-muted)]">
                    {filteredStyles.length} Available
                  </span>
                </div>

                <div className="relative px-1">
                  <Search className="w-3 h-3 absolute left-3 top-2.5 text-[var(--rz-text-muted)]" />
                  <input
                    type="text"
                    value={styleSearch}
                    onChange={(e) => setStyleSearch(e.target.value)}
                    placeholder="Search styles..."
                    className="w-full text-xs pl-7 pr-2.5 py-1.5 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] placeholder:text-[var(--rz-text-muted)] focus:outline-none focus:border-[var(--rz-accent)]"
                    autoFocus
                  />
                </div>

                <div className="max-h-64 overflow-y-auto space-y-1 p-1">
                  {filteredStyles.length > 0 ? (
                    filteredStyles.map((item) => {
                      const isSelected = item.id === selectedStyleId;
                      const isActive = item.id === activeAppliedStyleId;
                      return (
                        <button
                          key={item.id}
                          type="button"
                          onClick={() => {
                            handleSelectStyle(item.id);
                            setShowStyleDropdown(false);
                          }}
                          className={[
                            "w-full text-left p-2 rounded-xl text-xs flex items-center justify-between transition-colors cursor-pointer",
                            isSelected
                              ? "bg-[var(--rz-surface)] text-[var(--rz-accent)] font-semibold border border-[var(--rz-border-subtle)]"
                              : "hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)]",
                          ].join(" ")}
                        >
                          <div className="flex items-center gap-2 truncate">
                            <span className="truncate">{item.title}</span>
                            {isActive ? (
                              <span className="text-[9px] font-bold text-emerald-400 bg-emerald-500/15 border border-emerald-500/30 px-1.5 py-0.2 rounded shrink-0">
                                Active
                              </span>
                            ) : (
                              <span className="text-[9px] font-medium text-[var(--rz-text-secondary)] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] px-1.5 py-0.2 rounded shrink-0">
                                Installed
                              </span>
                            )}
                          </div>
                          {isSelected && <Check className="w-3.5 h-3.5 text-[var(--rz-accent)] shrink-0" />}
                        </button>
                      );
                    })
                  ) : (
                    <div className="p-4 text-center text-xs space-y-2">
                      <p className="text-[var(--rz-text-muted)]">No installed styles found.</p>
                      <button
                        type="button"
                        onClick={() => {
                          setShowStyleDropdown(false);
                          setActiveCategory("discover");
                        }}
                        className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-[var(--rz-accent)] text-white hover:bg-[var(--rz-accent-hover)] transition-colors cursor-pointer"
                      >
                        Browse Fastfetch
                      </button>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>

          {/* Browse Store Button */}
          <button
            type="button"
            onClick={() => setActiveCategory("discover")}
            className="px-3.5 py-1.5 rounded-xl text-xs font-semibold bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer"
          >
            Browse Fastfetch
          </button>
        </div>
      </div>

      {/* ── Main Workspace: Dominant Preview (Left 60-65%) + Clean Configuration (Right 35-40%) ── */}
      <div className="flex-1 grid grid-cols-1 lg:grid-cols-12 gap-6 pt-4 min-h-0">
        {/* ── LEFT COLUMN: Dominant Readout Preview (lg:col-span-7) ── */}
        <div className="lg:col-span-7 flex flex-col h-full bg-[#050608] rounded-2xl border border-[var(--rz-border)] overflow-hidden shadow-2xl">
          {/* Minimal Terminal Canvas (silentbyte@slayer host preview) */}
          <div
            className={[
              "flex-1 p-7 overflow-auto flex bg-[#050608]",
              previewAnsi ? "items-start justify-start" : "items-center justify-center",
            ].join(" ")}
          >
            {previewAnsi ? (
              <div className="flex flex-col">
                {missingLogoPath && (
                  <div className="mb-3 px-3 py-1.5 rounded-lg border border-amber-500/20 bg-amber-500/10 text-amber-300 text-xs font-mono">
                    Preset logo not found ({missingLogoPath}). Fastfetch rendered default layout.
                  </div>
                )}
              <div
                className="relative text-[14px] select-text whitespace-pre text-[#f1f5f9] max-w-full"
                style={{
                  fontFamily:
                    "'JetBrainsMono Nerd Font', 'JetBrainsMonoNL Nerd Font', 'FiraCode Nerd Font', 'Hack Nerd Font', 'JetBrainsMono Nerd Font Mono', monospace",
                  fontFeatureSettings: '"liga" 0',
                  fontVariantLigatures: "none",
                  letterSpacing: "0px",
                  lineHeight: "19px",
                  minHeight: `${
                    Math.max(
                      parsedOutput.textRows ?? 0,
                      parsedOutput.hasKittyImage
                        ? (parsedOutput.kittyTopPadding ?? 0) +
                            Math.max(
                              parsedOutput.kittyRows ?? heightLines,
                              (parsedOutput.textRows ?? 0) - (parsedOutput.kittyTopPadding ?? 0)
                            )
                        : 0
                    ) * 19
                  }px`,
                }}
              >
                {/* Emblem Image positioned at exact terminal cell coordinates */}
                {parsedOutput.hasKittyImage && parsedOutput.kittyImageUrl && (
                  <div
                    className={[
                      "absolute overflow-hidden pointer-events-none z-10 flex items-start justify-center",
                      dither ? "contrast-125 brightness-95" : "",
                    ].join(" ")}
                    style={{
                      left: `${parsedOutput.kittyLeftPadding ?? 0}ch`,
                      top: `${(parsedOutput.kittyTopPadding ?? 0) * 19}px`,
                      width: `${parsedOutput.kittyCols ?? widthCols}ch`,
                      height: `${
                        Math.max(
                          parsedOutput.kittyRows ?? heightLines,
                          (parsedOutput.textRows ?? 0) - (parsedOutput.kittyTopPadding ?? 0)
                        ) * 19
                      }px`,
                    }}
                  >
                    <img
                      src={parsedOutput.kittyImageUrl}
                      alt=""
                      className="w-full h-full object-contain object-top filter drop-shadow-md"
                      onError={(e) => {
                        (e.target as HTMLElement).style.display = "none";
                      }}
                    />
                  </div>
                )}

                {/* Unified Terminal text output (with identical monospace cell metrics) */}
                <div dangerouslySetInnerHTML={{ __html: parsedOutput.textHtml }} />
              </div>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center gap-2 text-[var(--rz-text-muted)] font-mono text-xs">
                <Terminal className="w-6 h-6 animate-pulse text-[var(--rz-accent)]" />
                <span>{isRenderingPreview ? "Rendering live Fastfetch preview…" : "Loading Fastfetch configuration…"}</span>
              </div>
            )}
          </div>
        </div>

{/* ── RIGHT COLUMN: Configuration Inspector (lg:col-span-5) ── */}
        <div className="lg:col-span-5 flex flex-col h-full bg-[var(--rz-surface)] rounded-2xl border border-[var(--rz-border-subtle)] overflow-hidden shadow-xs">
          {/* Header */}
          <div className="px-5 py-3.5 border-b border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Sliders className="w-4 h-4 text-[var(--rz-accent)]" />
              <h2 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text)]">
                Configuration
              </h2>
            </div>
            {/* Section Tabs */}
            <div className="flex items-center gap-1 p-0.5 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
              {[
                { id: "appearance", label: "Appearance", icon: <ImageIcon className="w-3 h-3" /> },
                { id: "layout", label: "Layout", icon: <LayoutIcon className="w-3 h-3" /> },
                { id: "colors", label: "Colors", icon: <Palette className="w-3 h-3" /> },
                { id: "info", label: "Information", icon: <Info className="w-3 h-3" /> },
              ].map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  onClick={() => setConfigTab(tab.id as any)}
                  className={[
                    "flex items-center gap-1 px-2.5 py-1 rounded-lg text-[10px] font-semibold transition-all cursor-pointer",
                    configTab === tab.id
                      ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] shadow-xs border border-[var(--rz-border-subtle)]"
                      : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]",
                  ].join(" ")}
                >
                  {tab.icon}
                  <span>{tab.label}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Inspector Content */}
          <div className="flex-1 overflow-y-auto p-5 space-y-6 text-xs">
            {/* TAB 1: APPEARANCE (Emblem controls) */}
            {configTab === "appearance" && (
              <section className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[var(--rz-text)]">
                    • Emblem Mode
                  </span>
                  <span className="text-[11px] text-[var(--rz-text-secondary)] capitalize">
                    {emblemType}
                  </span>
                </div>

                {/* Segmented Mode Picker */}
                <div className="grid grid-cols-4 gap-1 p-1 bg-[var(--rz-bg)] rounded-xl border border-[var(--rz-border-subtle)]">
                  {(["image", "ascii", "builtin", "none"] as EmblemType[]).map((type) => (
                    <button
                      key={type}
                      type="button"
                      onClick={() => {
                        setEmblemType(type);
                        setIsSaved(false);
    setIsModified(true);
                      }}
                      className={[
                        "py-1.5 px-1 rounded-lg text-center font-medium capitalize text-[11px] transition-all cursor-pointer",
                        emblemType === type
                          ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] font-semibold shadow-xs border border-[var(--rz-border-subtle)]"
                          : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]",
                      ].join(" ")}
                    >
                      {type}
                    </button>
                  ))}
                </div>

                {/* Emblem Gallery */}
                {emblemType === "image" && (
                  <div className="space-y-3 pt-2">
                    <div className="flex items-center justify-between">
                      <span className="font-semibold text-[var(--rz-text)]">Installed Emblems</span>
                      <div className="flex items-center gap-2">
                        <span className="text-[10px] text-[var(--rz-text-muted)] font-mono">
                          {installedEmblems.length} Installed
                        </span>
                        <button
                          type="button"
                          onClick={() => setActiveCategory("fastfetch")}
                          className="inline-flex items-center gap-1 text-[11px] font-semibold text-[var(--rz-accent)] hover:underline cursor-pointer"
                        >
                          <Plus className="w-3 h-3" />
                          <span>Browse Emblems</span>
                        </button>
                      </div>
                    </div>

                    {installedEmblems.length === 0 && !customEmblemUrl ? (
                      <div className="rounded-xl border border-dashed border-[var(--rz-border)] bg-[var(--rz-bg)]/60 p-6 flex flex-col items-center justify-center text-center">
                        <ImageIcon className="w-7 h-7 text-[var(--rz-text-muted)] mb-2" />
                        <span className="text-xs font-semibold text-[var(--rz-text)]">No installed emblems</span>
                        <span className="text-[10px] text-[var(--rz-text-muted)] mt-0.5 mb-3">
                          Discover and install emblems from the Ryzora Store.
                        </span>
                        <div className="flex items-center gap-2">
                          <button
                            type="button"
                            onClick={() => setActiveCategory("fastfetch")}
                            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--rz-accent)] hover:opacity-90 text-white text-xs font-medium cursor-pointer transition-opacity"
                          >
                            <Plus className="w-3.5 h-3.5" />
                            Browse Emblems
                          </button>
                          <button
                            type="button"
                            onClick={() => fileInputRef.current?.click()}
                            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] text-xs font-medium cursor-pointer transition-colors"
                          >
                            <Upload className="w-3.5 h-3.5" />
                            Upload Custom Image
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div className="grid grid-cols-4 gap-2">
                        {/* Upload Card */}
                        <button
                          type="button"
                          onClick={() => fileInputRef.current?.click()}
                          className="aspect-square rounded-xl border border-dashed border-[var(--rz-border)] hover:border-[var(--rz-accent)] bg-[var(--rz-bg)] hover:bg-[var(--rz-surface-hover)] flex flex-col items-center justify-between p-2 text-center cursor-pointer group transition-all"
                          title="Upload Custom Image Emblem"
                        >
                          <div className="flex-1 w-full flex items-center justify-center min-h-[72px] max-h-[88px]">
                            <div className="w-14 h-14 rounded-xl bg-[var(--rz-surface-elevated)] group-hover:bg-[var(--rz-accent)]/10 flex items-center justify-center transition-colors">
                              <Upload className="w-6 h-6 text-[var(--rz-text-secondary)] group-hover:text-[var(--rz-accent)] transition-colors" />
                            </div>
                          </div>
                          <span className="text-[10px] font-medium text-[var(--rz-text-secondary)] group-hover:text-[var(--rz-text)] truncate w-full text-center mt-1">
                            Custom
                          </span>
                        </button>

                        {/* Installed local emblem items */}
                        {installedEmblems.map((emb) => {
                          const isSelected = selectedEmblemId === emb.id && !customEmblemUrl;
                          return (
                            <button
                              key={emb.id}
                              type="button"
                              onClick={() => {
                                setUserEmblemOverridden(true);
                                setSelectedEmblemId(emb.id);
                                setCustomEmblemUrl("");
                                setEmblemType("image");
                                setIsSaved(false);
                                setIsModified(true);
                              }}
                              className={[
                                "aspect-square rounded-xl border p-2 flex flex-col items-center justify-between bg-[var(--rz-bg)] transition-all cursor-pointer relative overflow-hidden group",
                                isSelected
                                  ? "border-[var(--rz-accent)] shadow-xs ring-1.5 ring-[var(--rz-accent)] bg-[var(--rz-surface)]"
                                  : "border-[var(--rz-border-subtle)] hover:border-[var(--rz-border)] hover:bg-[var(--rz-surface-hover)]",
                              ].join(" ")}
                              title={emb.name}
                            >
                              {isSelected && (
                                <div className="absolute top-1.5 right-1.5 w-4 h-4 rounded-full bg-[var(--rz-accent)] text-white flex items-center justify-center shadow-xs z-10">
                                  <Check className="w-2.5 h-2.5 stroke-[3]" />
                                </div>
                              )}
                              <div className="flex-1 w-full flex items-center justify-center min-h-[72px] max-h-[88px] p-1 overflow-hidden">
                                <img
                                  src={resolveLocalAssetSrc(emb.path)}
                                  alt={emb.name}
                                  loading="lazy"
                                  className="max-w-[88px] max-h-[88px] w-auto h-auto object-contain group-hover:scale-105 transition-transform"
                                  style={{ maxWidth: "88px", maxHeight: "88px" }}
                                  onError={(e) => {
                                    (e.target as HTMLElement).style.display = "none";
                                  }}
                                />
                              </div>
                              <span className="text-[10px] font-medium text-[var(--rz-text)] truncate w-full text-center mt-1">
                                {emb.name}
                              </span>
                            </button>
                          );
                        })}
                      </div>
                    )}

                    <input
                      ref={fileInputRef}
                      type="file"
                      accept="image/png,image/jpeg,image/webp"
                      onChange={handleFileUpload}
                      className="hidden"
                    />
                  </div>
                )}
              </section>
            )}

            {/* TAB 2: LAYOUT (Width, Height, Padding, Dither) */}
            {configTab === "layout" && (
              <section className="space-y-4">
                <div className="space-y-3">
                  <span className="font-bold text-[var(--rz-text)] block">
                    • Layout Dimensions
                  </span>

                  {/* Width */}
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    <div>
                      <span className="font-semibold text-[var(--rz-text)] block">Width</span>
                      <span className="text-[10px] text-[var(--rz-text-muted)]">Terminal columns</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => {
                          setWidthCols((w) => Math.max(12, w - 2));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        -
                      </button>
                      <span className="w-8 text-center font-mono font-bold">{widthCols}</span>
                      <button
                        type="button"
                        onClick={() => {
                          setWidthCols((w) => Math.min(60, w + 2));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        +
                      </button>
                    </div>
                  </div>

                  {/* Height */}
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    <div>
                      <span className="font-semibold text-[var(--rz-text)] block">Height</span>
                      <span className="text-[10px] text-[var(--rz-text-muted)]">Emblem lines</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => {
                          setHeightLines((h) => Math.max(8, h - 2));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        -
                      </button>
                      <span className="w-8 text-center font-mono font-bold">{heightLines}</span>
                      <button
                        type="button"
                        onClick={() => {
                          setHeightLines((h) => Math.min(32, h + 2));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        +
                      </button>
                    </div>
                  </div>

                  {/* Padding */}
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    <div>
                      <span className="font-semibold text-[var(--rz-text)] block">Padding</span>
                      <span className="text-[10px] text-[var(--rz-text-muted)]">Separation columns</span>
                    </div>
                    <div className="flex items-center gap-2">
                      <button
                        type="button"
                        onClick={() => {
                          setPaddingCols((p) => Math.max(0, p - 1));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        -
                      </button>
                      <span className="w-8 text-center font-mono font-bold">{paddingCols}</span>
                      <button
                        type="button"
                        onClick={() => {
                          setPaddingCols((p) => Math.min(10, p + 1));
                          setIsSaved(false);
    setIsModified(true);
                        }}
                        className="w-7 h-7 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center text-xs hover:border-[var(--rz-accent)] cursor-pointer"
                      >
                        +
                      </button>
                    </div>
                  </div>

                  {/* Dither */}
                  <div className="flex items-center justify-between p-3 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    <div>
                      <span className="font-semibold text-[var(--rz-text)] block">Dither</span>
                      <span className="text-[10px] text-[var(--rz-text-muted)]">Terminal dithering shader</span>
                    </div>
                    <button
                      type="button"
                      onClick={() => {
                        setDither(!dither);
                        setIsSaved(false);
    setIsModified(true);
                      }}
                      className={[
                        "px-3 py-1 rounded-lg text-xs font-semibold border transition-all cursor-pointer",
                        dither
                          ? "bg-[var(--rz-accent)] text-white border-[var(--rz-accent)] shadow-xs"
                          : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border-[var(--rz-border-subtle)]",
                      ].join(" ")}
                    >
                      {dither ? "ON" : "OFF"}
                    </button>
                  </div>
                </div>
              </section>
            )}

            {/* TAB 3: COLORS (READOUT ACCENT) */}
            {configTab === "colors" && (
              <section className="space-y-4">
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[var(--rz-text)]">
                    • READOUT ACCENT
                  </span>
                  <span className="text-[11px] font-mono text-[var(--rz-text-secondary)]">
                    {accentColor}
                  </span>
                </div>

                <div className="p-3.5 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] space-y-3">
                  <div className="flex items-center gap-3">
                    <input
                      type="color"
                      value={accentColor}
                      onChange={(e) => {
                        setAccentColor(e.target.value);
                        setColorModified(true);
                        setIsSaved(false);
                        setIsModified(true);
                      }}
                      className="w-10 h-10 rounded-xl cursor-pointer border border-white/20 bg-transparent shrink-0"
                    />
                    <div className="flex-1">
                      <span className="text-xs font-mono font-semibold block text-[var(--rz-text)]">
                        {accentColor}
                      </span>
                      <span className="text-[10px] text-[var(--rz-text-muted)]">
                        Primary telemetry key color
                      </span>
                    </div>
                  </div>

                  {/* Curated Swatches */}
                  <div className="space-y-1.5 pt-1">
                    <span className="text-[10px] font-mono uppercase tracking-wider text-[var(--rz-text-muted)] block">
                      Quick Palette
                    </span>
                    <div className="flex items-center gap-1.5 flex-wrap">
                      {COLOR_PALETTE_SWATCHES.map((swatch, idx) => (
                        <button
                          key={idx}
                          type="button"
                          onClick={() => {
                            setAccentColor(swatch);
                            setColorModified(true);
                            setIsSaved(false);
                            setIsModified(true);
                          }}
                          className={[
                            "w-6 h-6 rounded-lg border transition-transform cursor-pointer",
                            accentColor === swatch
                              ? "scale-110 border-white ring-2 ring-[var(--rz-accent)]"
                              : "border-black/20 hover:scale-105",
                          ].join(" ")}
                          style={{ backgroundColor: swatch }}
                        />
                      ))}
                    </div>
                  </div>
                </div>
              </section>
            )}

            {/* TAB 4: INFORMATION (INFO MODULES) */}
            {configTab === "info" && (
              <section className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="font-bold text-[var(--rz-text)]">
                    • INFO MODULES
                  </span>
                  <span className="text-[10px] font-mono text-[var(--rz-text-secondary)]">
                    {rows.filter((r) => r.enabled).length}/{rows.length} Visible
                  </span>
                </div>

                <div className="space-y-1.5 max-h-72 overflow-y-auto pr-1">
                  {rows.map((row, idx) => (
                    <div
                      key={row.id}
                      className="flex items-center justify-between p-2 rounded-xl bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-border)] transition-colors group"
                    >
                      <div className="flex items-center gap-2 min-w-0 pr-2">
                        {/* Up / Down arrows */}
                        <div className="flex flex-col shrink-0">
                          <button
                            type="button"
                            disabled={idx === 0}
                            onClick={() => handleMoveRow(idx, "up")}
                            className="p-0.5 text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] disabled:opacity-20 cursor-pointer"
                          >
                            <ChevronUp className="w-3 h-3" />
                          </button>
                          <button
                            type="button"
                            disabled={idx === rows.length - 1}
                            onClick={() => handleMoveRow(idx, "down")}
                            className="p-0.5 text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] disabled:opacity-20 cursor-pointer"
                          >
                            <ChevronDown className="w-3 h-3" />
                          </button>
                        </div>

                        {row.type === "tagline" || row.type === "section_header" ? (
                          <input
                            type="text"
                            value={row.customText || ""}
                            onChange={(e) => handleUpdateRowText(row.id, e.target.value)}
                            className="text-xs bg-transparent border-b border-transparent focus:border-[var(--rz-accent)] focus:outline-none text-[var(--rz-text)] truncate max-w-[150px]"
                            title="Edit text"
                          />
                        ) : (
                          <span className="text-xs truncate font-medium text-[var(--rz-text)]">
                            {row.label || row.type}
                          </span>
                        )}
                      </div>

                      <div className="flex items-center gap-1.5 shrink-0">
                        {/* Eye toggle */}
                        <button
                          type="button"
                          onClick={() => handleToggleRow(row.id)}
                          className={[
                            "p-1.5 rounded-lg border transition-colors cursor-pointer",
                            row.enabled
                              ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] border-[var(--rz-border-subtle)]"
                              : "text-[var(--rz-text-muted)] border-transparent hover:bg-[var(--rz-surface-elevated)]",
                          ].join(" ")}
                          title={row.enabled ? "Hide module" : "Show module"}
                        >
                          {row.enabled ? <Eye className="w-3 h-3" /> : <EyeOff className="w-3 h-3" />}
                        </button>

                        {/* Delete module */}
                        <button
                          type="button"
                          onClick={() => handleRemoveRow(row.id)}
                          className="p-1.5 rounded-lg text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                          title="Remove module"
                        >
                          <Trash2 className="w-3 h-3" />
                        </button>
                      </div>
                    </div>
                  ))}
                </div>

                {/* Quick Add Module Pills */}
                <div className="pt-2 border-t border-[var(--rz-border-subtle)] space-y-1.5">
                  <span className="text-[10px] font-mono uppercase text-[var(--rz-text-muted)] block">
                    Add Module
                  </span>
                  <div className="flex items-center gap-1.5 flex-wrap">
                    {[
                      { l: "Battery", p: "󰁹 ", k: "battery" },
                      { l: "Display", p: "󰍹 ", k: "display" },
                      { l: "Theme", p: "󰔎 ", k: "theme" },
                      { l: "Icons", p: "󰀻 ", k: "icons" },
                    ].map((m) => (
                      <button
                        key={m.l}
                        type="button"
                        onClick={() => handleAddModule(m.l, m.p, m.k)}
                        className="px-2.5 py-1 rounded-lg text-[10px] font-semibold bg-[var(--rz-bg)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] transition-colors cursor-pointer"
                      >
                        + {m.l}
                      </button>
                    ))}
                  </div>
                </div>
              </section>
            )}
          </div>
        </div>
      </div>

      {/* ── Bottom Action Bar ── */}
      <div className="shrink-0 flex items-center justify-between px-6 py-3.5 mt-4 rounded-2xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] shadow-md">
        <div className="flex items-center gap-3">
          {!isSaved ? (
            <span className="flex items-center gap-1.5 text-xs font-medium text-amber-400">
              <span className="w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
              Unsaved changes
            </span>
          ) : (
            <span className="flex items-center gap-1.5 text-xs font-medium text-emerald-400">
              <CheckCircle2 className="w-3.5 h-3.5" />
              Saved preset
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {/* Revert */}
          <button
            type="button"
            disabled={!isModified}
            onClick={handleRevert}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] disabled:opacity-30 disabled:cursor-not-allowed transition-all cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>REVERT</span>
          </button>

          {/* Preview in Terminal */}
          <button
            type="button"
            disabled={isLaunchingTerminal}
            onClick={handlePreviewInTerminal}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] transition-all cursor-pointer shadow-xs disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLaunchingTerminal ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin text-[var(--rz-accent)]" />
            ) : (
              <Terminal className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
            )}
            <span>{isLaunchingTerminal ? "LAUNCHING…" : "PREVIEW IN TERMINAL"}</span>
          </button>

          {/* Secondary More Menu [...] */}
          <div className="relative">
            <button
              type="button"
              onClick={() => setShowSecondaryMenu(!showSecondaryMenu)}
              className="p-2 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)] transition-colors cursor-pointer"
              title="More options"
            >
              <MoreHorizontal className="w-4 h-4" />
            </button>
            {showSecondaryMenu && (
              <div className="absolute right-0 bottom-full mb-2 w-52 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] shadow-xl p-1.5 z-30 space-y-1 animate-in fade-in zoom-in-95 duration-150">
                <button
                  type="button"
                  onClick={() => {
                    setShowSecondaryMenu(false);
                    handleApplyInstalledStyle();
                  }}
                  className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
                >
                  <Check className="w-3.5 h-3.5 text-emerald-400" />
                  <span>APPLY INSTALLED STYLE</span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowSecondaryMenu(false);
                    setShowConfigModal(true);
                  }}
                  className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
                >
                  <FileCode className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                  <span>Raw JSONC Config</span>
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowSecondaryMenu(false);
                    handleResetToDefault();
                  }}
                  className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer"
                >
                  <RotateCcw className="w-3.5 h-3.5 text-amber-400" />
                  <span>RESET TO DEFAULT</span>
                </button>
                <div className="border-t border-[var(--rz-border-subtle)] my-1" />
                <button
                  type="button"
                  onClick={() => {
                    setShowSecondaryMenu(false);
                    handleRemoveInstalledStyle();
                  }}
                  className="w-full text-left px-3 py-2 rounded-lg text-xs flex items-center gap-2 text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                >
                  <Trash2 className="w-3.5 h-3.5 text-rose-400" />
                  <span>REMOVE INSTALLED STYLE</span>
                </button>
              </div>
            )}
          </div>

          {/* Primary Save Button */}
          <button
            type="button"
            disabled={isSaving}
            onClick={handleSave}
            className="flex items-center gap-2 px-6 py-2 rounded-xl text-xs font-bold bg-[var(--rz-accent)] text-[var(--rz-accent-contrast)] hover:bg-[var(--rz-accent-hover)] active:scale-[0.98] shadow-md shadow-[var(--rz-accent)]/20 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSaving ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin text-[var(--rz-accent-contrast)]" />
            ) : (
              <Save className="w-3.5 h-3.5" />
            )}
            <span>{isSaving ? "SAVING…" : "SAVE"}</span>
          </button>
        </div>
      </div>

      {/* Raw JSONC Config Modal */}
      {showConfigModal && (
        <div
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-xs flex items-center justify-center p-4"
          onClick={() => setShowConfigModal(false)}
        >
          <div
            className="w-full max-w-2xl bg-[var(--rz-surface)] border border-[var(--rz-border)] rounded-2xl shadow-2xl p-6 space-y-4 font-mono text-xs"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between pb-3 border-b border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-2">
                <FileCode className="w-4 h-4 text-[var(--rz-accent)]" />
                <span className="font-bold text-[var(--rz-text)]">
                  ~/.config/fastfetch/config.jsonc
                </span>
              </div>
              <button
                type="button"
                onClick={() => setShowConfigModal(false)}
                className="p-1 rounded-lg hover:bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <pre className="p-4 rounded-xl bg-black/60 border border-white/5 overflow-x-auto text-emerald-400 font-mono text-[11px] leading-relaxed max-h-[420px] overflow-y-auto">
              {JSON.stringify(
                {
                  $schema: "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
                  logo: {
                    type: emblemType,
                    source: emblemType === "image" ? activeEmblemImage : undefined,
                    width: widthCols,
                    height: heightLines,
                    padding: { right: paddingCols },
                  },
                  display: {
                    color: { keys: accentColor },
                  },
                  modules: rows
                    .filter((r) => r.enabled)
                    .map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type)),
                },
                null,
                2
              )}
            </pre>

            <div className="flex justify-end pt-2">
              <button
                type="button"
                onClick={() => setShowConfigModal(false)}
                className="px-4 py-2 rounded-xl text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] cursor-pointer"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
