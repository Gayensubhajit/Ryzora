import React, { useState, useEffect, useRef } from "react";
import {
  Terminal,
  RotateCcw,
  Save,
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
  onConsumeInitialPresetId?: () => void;
}


export function ansiToHtml(raw: string): string {
  if (!raw) return "";

  // Strip kitty graphics protocol sequences: _G...(\|)
  let cleaned = raw.replace(/\x1b_G[^\x1b\x07]*(\x1b\\|\x07)/g, "");

  const stdColors = [
    "#3e4451", "#e06c75", "#98c379", "#e5c07b",
    "#61afef", "#c678dd", "#56b6c2", "#abb2bf"
  ];
  const brightColors = [
    "#5c6370", "#be5046", "#98c379", "#d19a66",
    "#61afef", "#c678dd", "#56b6c2", "#ffffff"
  ];

  const escapeHtml = (str: string) =>
    str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  const tokens = cleaned.split(/(\x1b\[[0-9;]*m)/);
  const out: string[] = [];

  let currentFg: string | null = null;
  let isBold = false;
  let isDim = false;
  let isItalic = false;
  let isUnderline = false;

  for (const token of tokens) {
    if (!token) continue;
    if (token.startsWith("\x1b[")) {
      const codeStr = token.slice(2, -1);
      const codes = codeStr ? codeStr.split(";").map(Number) : [0];

      let i = 0;
      while (i < codes.length) {
        const c = codes[i];
        if (c === 0) {
          currentFg = null;
          isBold = false;
          isDim = false;
          isItalic = false;
          isUnderline = false;
        } else if (c === 1) {
          isBold = true;
        } else if (c === 2) {
          isDim = true;
        } else if (c === 3) {
          isItalic = true;
        } else if (c === 4) {
          isUnderline = true;
        } else if (c === 22) {
          isBold = false;
          isDim = false;
        } else if (c === 23) {
          isItalic = false;
        } else if (c === 24) {
          isUnderline = false;
        } else if (c >= 30 && c <= 37) {
          currentFg = stdColors[c - 30];
        } else if (c === 39) {
          currentFg = null;
        } else if (c >= 90 && c <= 97) {
          currentFg = brightColors[c - 90];
        } else if (c === 38 && i + 4 < codes.length && codes[i + 1] === 2) {
          const r = codes[i + 2];
          const g = codes[i + 3];
          const b = codes[i + 4];
          currentFg = `rgb(${r},${g},${b})`;
          i += 4;
        }
        i++;
      }
    } else {
      const styles: string[] = [];
      if (currentFg) styles.push(`color: ${currentFg}`);
      if (isBold) styles.push("font-weight: 700");
      if (isDim) styles.push("opacity: 0.75");
      if (isItalic) styles.push("font-style: italic");
      if (isUnderline) styles.push("text-decoration: underline");

      const styleAttr = styles.length ? ` style="${styles.join("; ")}"` : "";
      out.push(`<span${styleAttr}>${escapeHtml(token)}</span>`);
    }
  }

  return out.join("");
}

export const FastfetchView: React.FC<FastfetchViewProps> = ({
  initialStyleId,
  onConsumeInitialPresetId,
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

  // All available fastfetch packages in catalog
  const fastfetchPackages = packages.filter(
    (p) => p.category === "fastfetch" || p.package_type === "fastfetch"
  );

  // Filter to ONLY installed fastfetch packages (or active/staged preset)
  const isInstalledPkg = (id: string) =>
    Boolean(
      installedPackageIds?.includes(id) ||
      installedPackages?.some((rec) => rec.package_id === id)
    );

  const installedFastfetchPackages = fastfetchPackages.filter((p) =>
    isInstalledPkg(p.id) || (initialStyleId && p.id === initialStyleId)
  );

  const targetInitialId =
    (initialStyleId && installedFastfetchPackages.some((p) => p.id === initialStyleId) ? initialStyleId : null) ||
    (selectedPackage && installedFastfetchPackages.some((p) => p.id === selectedPackage.id) ? selectedPackage.id : null) ||
    installedFastfetchPackages[0]?.id ||
    fastfetchPackages[0]?.id ||
    "fastfetch-preset-2";

  // Selected Style in Left Column
  const [selectedStyleId, setSelectedStyleId] = useState<string>(targetInitialId);

  // Sync selectedStyleId when initialStyleId is provided
  useEffect(() => {
    if (initialStyleId) {
      handleSelectStyle(initialStyleId);
      onConsumeInitialPresetId?.();
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
  const [customEmblemUrl, setCustomEmblemUrl] = useState<string>("");
  const [widthCols, setWidthCols] = useState<number>(28);
  const [heightLines, setHeightLines] = useState<number>(14);
  const [paddingCols, setPaddingCols] = useState<number>(3);
  const [dither, setDither] = useState<boolean>(false);
  const [accentColor, setAccentColor] = useState<string>("#e2342a");
  const [rows, setRows] = useState<FastfetchRowItem[]>(INITIAL_ROWS);
  const [previewAnsi, setPreviewAnsi] = useState<string>("");
  const [isRenderingPreview, setIsRenderingPreview] = useState<boolean>(false);

  // Status & Edit state
  const [isSaved, setIsSaved] = useState<boolean>(true);
  const [activeAppliedStyleId, setActiveAppliedStyleId] = useState<string>("fastfetch-preset-2");
  const [showConfigModal, setShowConfigModal] = useState<boolean>(false);

  const fileInputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

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
          if (emblems.length > 0 && emblemType === "image" && !selectedEmblemId) {
            setSelectedEmblemId(emblems[0].id);
          }
        }
      })
      .catch(() => {});
  }, []);

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

  // Live Fastfetch execution preview pipeline
  const renderPreview = async () => {
    setIsRenderingPreview(true);
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    try {
      const res = await invoke<{ raw_ansi: string; success: boolean }>("render_fastfetch_preview", {
        req: {
          presetId: selectedStyleId,
          accentColor,
          emblemType,
          emblemPath: customEmblemUrl || selectedEmblem?.path,
          widthCols,
          heightLines,
          paddingCols,
          dither,
          activeModules: rows
            .filter((r) => r.enabled)
            .map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type)),
        },
      });
      if (res && res.raw_ansi) {
        setPreviewAnsi(res.raw_ansi);
      }
    } catch (e) {
      console.error("Preview render failed:", e);
    } finally {
      setIsRenderingPreview(false);
    }
  };

  // Trigger live Fastfetch execution whenever parameters change (with 120ms debounce)
  useEffect(() => {
    const timer = setTimeout(() => {
      renderPreview();
    }, 120);
    return () => clearTimeout(timer);
  }, [
    selectedStyleId,
    accentColor,
    emblemType,
    selectedEmblemId,
    customEmblemUrl,
    widthCols,
    heightLines,
    paddingCols,
    dither,
    rows,
    installedEmblems,
  ]);

  // When a style preset is selected from the left column
  const handleSelectStyle = async (pkgId: string) => {
    setSelectedStyleId(pkgId);
    const found = fastfetchPackages.find((p) => p.id === pkgId);
    try {
      const configStr = await invoke<string>("get_fastfetch_preset_config", { presetId: pkgId });
      if (configStr) {
        const cleanJson = configStr.replace(/\/\/.*$/gm, "").replace(/\/\*[\s\S]*?\*\//g, "");
        const parsed = JSON.parse(cleanJson);
        if (parsed.logo) {
          if (parsed.logo.type === "none") {
            setEmblemType("none");
          } else if (parsed.logo.type === "builtin") {
            setEmblemType("builtin");
          } else if (parsed.logo.type === "file" || parsed.logo.type?.includes("ascii")) {
            setEmblemType("ascii");
          } else if (parsed.logo.source) {
            const matchingEmblem = installedEmblems.find(
              (e) => e.path === parsed.logo.source || e.id === parsed.logo.source
            );
            if (matchingEmblem) {
              setEmblemType("image");
              setSelectedEmblemId(matchingEmblem.id);
            }
          }
          if (typeof parsed.logo.width === "number") setWidthCols(parsed.logo.width);
          if (typeof parsed.logo.height === "number") setHeightLines(parsed.logo.height);
          if (parsed.logo.padding && typeof parsed.logo.padding.left === "number") {
            setPaddingCols(parsed.logo.padding.left);
          }
        }
        if (parsed.display?.color?.keys) {
          const keyCol = parsed.display.color.keys;
          if (keyCol.startsWith("38;2;")) {
            const parts = keyCol.split(";");
            if (parts.length >= 5) {
              const r = parseInt(parts[2]).toString(16).padStart(2, "0");
              const g = parseInt(parts[3]).toString(16).padStart(2, "0");
              const b = parseInt(parts[4]).toString(16).padStart(2, "0");
              setAccentColor(`#${r}${g}${b}`);
            }
          }
        }
      }
    } catch {}
    if (found?.color_palette?.[0]) {
      setAccentColor(found.color_palette[0]);
    }
    setIsSaved(false);
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
    setIsSaved(true);
    const found = fastfetchPackages.find((p) => p.id === selectedStyleId);
    setToast({
      message: `Applied installed style "${found?.title || selectedStyleId}" to ~/.config/fastfetch/config.jsonc.`,
      type: "success",
    });
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
    setWidthCols(28);
    setHeightLines(14);
    setPaddingCols(3);
    setDither(false);
    setAccentColor("#e2342a");
    setRows(INITIAL_ROWS);
    setIsSaved(false);
    setToast({
      message: "Reset Fastfetch configuration to default layout.",
      type: "info",
    });
  };

  const handleRevert = async () => {
    try {
      const savedConfig = await invoke<string | null>("get_fastfetch_saved_config");
      if (savedConfig) {
        const clean = savedConfig.replace(/\/\/.*$/gm, "").replace(/\/\*[\s\S]*?\*\//g, "");
        const parsed = JSON.parse(clean);
        if (parsed.logo) {
          if (parsed.logo.type === "none") setEmblemType("none");
          else if (parsed.logo.type === "builtin") setEmblemType("builtin");
          else if (parsed.logo.type === "file") setEmblemType("ascii");
          if (typeof parsed.logo.width === "number") setWidthCols(parsed.logo.width);
          if (typeof parsed.logo.height === "number") setHeightLines(parsed.logo.height);
          if (parsed.logo.padding && typeof parsed.logo.padding.left === "number") {
            setPaddingCols(parsed.logo.padding.left);
          }
        }
      }
    } catch {
      handleResetToDefault();
    }
    setIsSaved(true);
    setToast({
      message: "Reverted unsaved changes to active config.jsonc.",
      type: "info",
    });
  };

  const handleSave = async () => {
    setIsSaved(true);
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    await invoke("apply_fastfetch_configuration", {
      req: {
        presetId: selectedStyleId,
        accentColor,
        emblemType,
        emblemPath: customEmblemUrl || selectedEmblem?.path,
        widthCols,
        heightLines,
        paddingCols,
        dither,
        activeModules: rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type)),
      },
    }).catch(() => {});
    setToast({
      message: "Saved configuration to ~/.config/fastfetch/config.jsonc.",
      type: "success",
    });
  };

  const handlePreviewInTerminal = async () => {
    const selectedEmblem = installedEmblems.find((e) => e.id === selectedEmblemId);
    try {
      await invoke("preview_fastfetch_terminal", {
        req: {
          presetId: selectedStyleId,
          accentColor,
          emblemType,
          emblemPath: customEmblemUrl || selectedEmblem?.path,
          widthCols,
          heightLines,
          paddingCols,
          dither,
          activeModules: rows.filter((r) => r.enabled).map((r) => (r.type === "telemetry" ? r.telemetryKey : r.type)),
        },
      });
      setToast({
        message: "Preview in terminal launched with active configuration.",
        type: "info",
      });
    } catch (err: any) {
      setToast({
        message: `Failed to launch preview: ${err?.message || err}`,
        type: "warning",
      });
    }
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      const reader = new FileReader();
      reader.onload = () => {
        if (typeof reader.result === "string") {
          setCustomEmblemUrl(reader.result);
          setEmblemType("image");
          setIsSaved(false);
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
    setIsSaved(false);
  };

  const handleToggleRow = (id: string) => {
    setRows((prev) =>
      prev.map((r) => (r.id === id ? { ...r, enabled: !r.enabled } : r))
    );
    setIsSaved(false);
  };

  const handleRemoveRow = (id: string) => {
    setRows((prev) => prev.filter((r) => r.id !== id));
    setIsSaved(false);
  };

  const handleUpdateRowText = (id: string, text: string) => {
    setRows((prev) =>
      prev.map((r) => (r.id === id ? { ...r, customText: text } : r))
    );
    setIsSaved(false);
  };

  // ANSI spectrum palette
  const ansiColors = [
    "#1e1e2e",
    accentColor || "#e2342a",
    "#a6e3a1",
    "#f9e2af",
    "#89b4fa",
    "#f5c2e7",
    "#94e2d5",
    "#cdd6f4",
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
    setIsSaved(false);
  };

  // Active emblem resolution
  const activePresetEmblem = installedEmblems.find((p) => p.id === selectedEmblemId);
  const activeEmblemImage = customEmblemUrl || (activePresetEmblem ? convertFileSrc(activePresetEmblem.path) : "");

  // Filtered style list
  const filteredStyles = installedFastfetchPackages.filter((p) =>
    p.title.toLowerCase().includes(styleSearch.toLowerCase())
  );

  const selectedPackageItem = fastfetchPackages.find((p) => p.id === selectedStyleId);

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
        <div className="lg:col-span-7 flex flex-col h-full bg-[#0b0f17] rounded-2xl border border-[var(--rz-border-subtle)] overflow-hidden shadow-2xl font-mono">
          {/* Terminal Titlebar */}
          <div className="flex items-center justify-between px-4 py-2.5 bg-[#121824] border-b border-[#1f2937]/80 text-xs text-slate-400 select-none">
            <div className="flex items-center gap-2">
              <div className="flex items-center gap-1.5">
                <span className="w-3 h-3 rounded-full bg-[#f35757]/80 border border-[#f35757] inline-block" />
                <span className="w-3 h-3 rounded-full bg-[#e6c149]/80 border border-[#e6c149] inline-block" />
                <span className="w-3 h-3 rounded-full bg-[#49cc6a]/80 border border-[#49cc6a] inline-block" />
              </div>
              <div className="flex items-center gap-1.5 ml-3 font-medium text-slate-300">
                <Terminal className="w-3.5 h-3.5 text-slate-400" />
                <span>silentbyte@slayer: ~</span>
              </div>
            </div>

            <div className="flex items-center gap-2 font-mono text-[10px]">
              <span
                className={[
                  "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full font-medium border",
                  isSaved
                    ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20"
                    : "bg-amber-500/10 text-amber-400 border-amber-500/20",
                ].join(" ")}
              >
                <span
                  className={[
                    "w-1.5 h-1.5 rounded-full",
                    isSaved ? "bg-emerald-400" : "bg-amber-400 animate-pulse",
                  ].join(" ")}
                />
                {isSaved ? "Live · Current configuration" : "Live · Unsaved configuration"}
              </span>
            </div>
          </div>

          {/* Terminal Canvas */}
          <div className="flex-1 p-6 overflow-auto flex items-center justify-center bg-[#0b0f17]">
            {previewAnsi ? (
              <div className="w-full flex flex-col sm:flex-row items-center sm:items-start justify-center gap-6 min-h-[320px]">
                {/* Emblem Area (if image mode and image available) */}
                {emblemType === "image" && activeEmblemImage && (
                  <div
                    className={[
                      "shrink-0 relative overflow-hidden rounded-xl border border-white/10 shadow-lg bg-black/40 flex items-center justify-center",
                      dither ? "contrast-125 brightness-95" : "",
                    ].join(" ")}
                    style={{
                      width: `${Math.max(120, widthCols * 6.5)}px`,
                      height: `${Math.max(120, heightLines * 13)}px`,
                      marginRight: `${paddingCols * 4}px`,
                    }}
                  >
                    <img
                      src={activeEmblemImage}
                      alt="Fastfetch Emblem"
                      className="w-full h-full object-contain filter drop-shadow-md"
                    />
                  </div>
                )}

                {/* Real Fastfetch ANSI Output */}
                <div
                  className="font-mono text-xs sm:text-[12.5px] leading-relaxed select-text whitespace-pre overflow-x-auto text-[#c9d1d9] flex-1 max-w-[560px]"
                  dangerouslySetInnerHTML={{ __html: ansiToHtml(previewAnsi) }}
                />
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center gap-2 text-[var(--rz-text-muted)] font-mono text-xs">
                <Terminal className="w-6 h-6 animate-pulse text-[var(--rz-accent)]" />
                <span>{isRenderingPreview ? "Rendering live Fastfetch preview…" : "Loading Fastfetch configuration…"}</span>
              </div>
            )}
          </div>

          {/* Bottom Terminal Control Footer */}
          <div className="px-4 py-2 bg-[#121824] border-t border-[#1f2937]/80 flex items-center justify-between text-xs text-slate-400 select-none">
            <button
              type="button"
              onClick={handlePreviewInTerminal}
              className="flex items-center gap-1.5 px-3 py-1 rounded bg-[#1c2433] hover:bg-[#253043] text-slate-200 border border-[#2d3748] transition-colors cursor-pointer"
            >
              <Terminal className="w-3 h-3 text-[var(--rz-accent)]" />
              <span>PREVIEW IN TERMINAL</span>
            </button>
            <span className="text-[11px] text-slate-500 italic">
              Live · Generated from current config
            </span>
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
                      <span className="font-semibold text-[var(--rz-text)]">Emblem Library</span>
                      <span className="text-[10px] text-[var(--rz-text-muted)] font-mono">
                        {installedEmblems.length} Installed
                      </span>
                    </div>

                    {installedEmblems.length === 0 && !customEmblemUrl ? (
                      <div className="rounded-xl border border-dashed border-[var(--rz-border)] bg-[var(--rz-bg)]/60 p-6 flex flex-col items-center justify-center text-center">
                        <ImageIcon className="w-7 h-7 text-[var(--rz-text-muted)] mb-2" />
                        <span className="text-xs font-semibold text-[var(--rz-text)]">No installed emblems</span>
                        <span className="text-[10px] text-[var(--rz-text-muted)] mt-0.5 mb-3">
                          No local emblem assets found on filesystem
                        </span>
                        <button
                          type="button"
                          onClick={() => fileInputRef.current?.click()}
                          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-[var(--rz-accent)] hover:opacity-90 text-white text-xs font-medium cursor-pointer transition-opacity"
                        >
                          <Upload className="w-3.5 h-3.5" />
                          Upload Custom Image
                        </button>
                      </div>
                    ) : (
                      <div className="grid grid-cols-4 gap-2">
                        {/* Upload Card */}
                        <button
                          type="button"
                          onClick={() => fileInputRef.current?.click()}
                          className="aspect-square rounded-xl border border-dashed border-[var(--rz-border)] hover:border-[var(--rz-accent)] bg-[var(--rz-bg)] flex flex-col items-center justify-center p-1 text-center cursor-pointer group transition-colors"
                          title="Upload Custom Image Emblem"
                        >
                          <Upload className="w-3.5 h-3.5 text-[var(--rz-text-secondary)] group-hover:text-[var(--rz-accent)] mb-1" />
                          <span className="text-[9px] font-medium text-[var(--rz-text-secondary)]">Custom</span>
                        </button>

                        {/* Installed local emblem items */}
                        {installedEmblems.map((emb) => {
                          const isSelected = selectedEmblemId === emb.id && !customEmblemUrl;
                          return (
                            <button
                              key={emb.id}
                              type="button"
                              onClick={() => {
                                setSelectedEmblemId(emb.id);
                                setCustomEmblemUrl("");
                                setEmblemType("image");
                                setIsSaved(false);
                              }}
                              className={[
                                "aspect-square rounded-xl border p-1 flex flex-col items-center justify-center bg-[var(--rz-bg)] transition-all cursor-pointer relative overflow-hidden group",
                                isSelected
                                  ? "border-[var(--rz-accent)] shadow-xs ring-1 ring-[var(--rz-accent)]"
                                  : "border-[var(--rz-border-subtle)] hover:border-[var(--rz-border)]",
                              ].join(" ")}
                              title={emb.name}
                            >
                              <img
                                src={convertFileSrc(emb.path)}
                                alt={emb.name}
                                className="w-7 h-7 object-contain mb-1"
                                onError={(e) => {
                                  (e.target as HTMLElement).style.display = "none";
                                }}
                              />
                              <span className="text-[8px] font-mono text-[var(--rz-text-secondary)] truncate w-full text-center">
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
                          setPaddingCols((p) => Math.max(1, p - 1));
                          setIsSaved(false);
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
                        setIsSaved(false);
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
                            setIsSaved(false);
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
              All changes saved
            </span>
          )}
        </div>

        <div className="flex items-center gap-3">
          {/* Revert */}
          <button
            type="button"
            disabled={isSaved}
            onClick={handleRevert}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] disabled:opacity-30 disabled:cursor-not-allowed transition-all cursor-pointer"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>REVERT</span>
          </button>

          {/* Preview in Terminal */}
          <button
            type="button"
            onClick={handlePreviewInTerminal}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-subtle)] transition-all cursor-pointer shadow-xs"
          >
            <Terminal className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
            <span>PREVIEW IN TERMINAL</span>
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
            onClick={handleSave}
            className="flex items-center gap-2 px-6 py-2 rounded-xl text-xs font-bold bg-[var(--rz-accent)] text-white hover:bg-[var(--rz-accent-hover)] active:scale-[0.98] shadow-md shadow-[var(--rz-accent)]/20 transition-all cursor-pointer"
          >
            <Save className="w-3.5 h-3.5" />
            <span>SAVE</span>
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
