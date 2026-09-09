import React, { useState, useEffect, useMemo } from "react";
import {
  PackagePlus,
  FileCode,
  Layers,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Plus,
  Trash2,
  Folder,
  ArrowRight,
  ArrowLeft,
  Sparkles,
  UploadCloud,
  Check,
  RefreshCw,
  Sliders,
  ShieldCheck,
  Hash,
  Palette,
  Tag,
  Copy,
} from "lucide-react";
import { useApp } from "../context/AppContext";
import {
  PackageDraft,
  FileMappingDraft,
  PackageType,
  DependencySpec,
  ManifestValidationResult,
  AuthoringResult,
  PublishResult,
} from "../types";

const COMMON_PRESETS = [
  {
    name: "Hyprland Config",
    target: "~/.config/hypr/hyprland.conf",
    desc: "Main Hyprland compositor configuration",
  },
  {
    name: "Waybar Config",
    target: "~/.config/waybar/config.jsonc",
    desc: "Waybar panel layout configuration",
  },
  {
    name: "Waybar CSS",
    target: "~/.config/waybar/style.css",
    desc: "Waybar styling and theme colors",
  },
  {
    name: "Kitty Terminal",
    target: "~/.config/kitty/kitty.conf",
    desc: "Kitty terminal emulator styling and keybinds",
  },
  {
    name: "Rofi Launcher",
    target: "~/.config/rofi/config.rasi",
    desc: "Rofi application launcher configuration",
  },
  {
    name: "Fastfetch Info",
    target: "~/.config/fastfetch/config.jsonc",
    desc: "Fastfetch system information layout",
  },
  {
    name: "Hyprlock Lockscreen",
    target: "~/.config/hypr/hyprlock.conf",
    desc: "Hyprlock screen locker layout and visuals",
  },
];

const DESKTOP_OPTIONS = [
  "hyprland",
  "sway",
  "kde",
  "gnome",
  "xfce",
  "cinnamon",
  "cosmic",
  "i3",
];

const PACKAGE_TYPES: PackageType[] = [
  "rice",
  "theme",
  "waybar",
  "fastfetch",
  "lockscreen",
  "wallpaper",
  "terminal",
  "icon",
  "cursor",
  "font",
  "widget",
  "bundle",
];

export const AuthorView: React.FC = () => {
  const {
    systemInfo,
    repositories,
    validatePackageDraft,
    createPackage,
    publishPackageToRepository,
    setActiveCategory,
    setToast,
  } = useApp();

  const [currentStep, setCurrentStep] = useState<1 | 2 | 3 | 4 | 5>(1);

  // Form State
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [idTouched, setIdTouched] = useState(false);
  const [version, setVersion] = useState("1.0.0");
  const [author, setAuthor] = useState("");
  const [packageType, setPackageType] = useState<PackageType>("rice");
  const [description, setDescription] = useState("");
  const [tagInput, setTagInput] = useState("");
  const [tags, setTags] = useState<string[]>(["custom", "rice"]);
  const [colorInput, setColorInput] = useState("#ff007f");
  const [colorPalette, setColorPalette] = useState<string[]>(["#ff007f", "#00f0ff", "#1a1b26"]);

  // Compatibility & Dependencies State
  const [selectedDesktops, setSelectedDesktops] = useState<string[]>(["hyprland"]);
  const [sessionType, setSessionType] = useState<"wayland" | "x11" | "any">("wayland");
  const [requiredBins, setRequiredBins] = useState<string[]>([]);
  const [binInput, setBinInput] = useState("");
  const [dependencies, setDependencies] = useState<DependencySpec[]>([]);
  const [depIdInput, setDepIdInput] = useState("");
  const [depVersionInput, setDepVersionInput] = useState("");

  // File Mappings
  const [files, setFiles] = useState<FileMappingDraft[]>([
    {
      source_path: "~/.config/hypr/hyprland.conf",
      target: "~/.config/hypr/hyprland.conf",
      package_rel_path: "files/hypr/hyprland.conf",
      description: "Hyprland main configuration",
    },
  ]);

  // Validation State
  const [validationResult, setValidationResult] = useState<ManifestValidationResult | null>(null);
  const [validating, setValidating] = useState(false);

  // Build & Publishing State
  const [destinationDir, setDestinationDir] = useState("~/RyzoraPackages");
  const [selectedRepoPath, setSelectedRepoPath] = useState("");
  const [customRepoPath, setCustomRepoPath] = useState("");
  const [isBuilding, setIsBuilding] = useState(false);
  const [authoringResult, setAuthoringResult] = useState<AuthoringResult | null>(null);
  const [isPublishing, setIsPublishing] = useState(false);
  const [publishResult, setPublishResult] = useState<PublishResult | null>(null);

  // Set default author from systemInfo if available
  useEffect(() => {
    if (!author && systemInfo?.shell) {
      setAuthor("Ryzora User");
    }
  }, [systemInfo]);

  // Set default repo path from available repositories
  useEffect(() => {
    const localRepo = repositories.find((r) => r.repo_type === "local");
    if (localRepo && !selectedRepoPath) {
      setSelectedRepoPath(localRepo.path);
    }
  }, [repositories, selectedRepoPath]);

  // Auto slugify name to id if not touched manually
  const handleNameChange = (val: string) => {
    setName(val);
    if (!idTouched) {
      const slug = val
        .toLowerCase()
        .replace(/[^a-z0-9_-]/g, "-")
        .replace(/-+/g, "-")
        .replace(/^-|-$/g, "");
      setId(slug);
    }
  };

  const handleAddTag = () => {
    const t = tagInput.trim().toLowerCase();
    if (t && !tags.includes(t)) {
      setTags([...tags, t]);
      setTagInput("");
    }
  };

  const handleRemoveTag = (tag: string) => {
    setTags(tags.filter((t) => t !== tag));
  };

  const handleAddColor = () => {
    const c = colorInput.trim();
    if (c && !colorPalette.includes(c)) {
      setColorPalette([...colorPalette, c]);
    }
  };

  const handleRemoveColor = (col: string) => {
    setColorPalette(colorPalette.filter((c) => c !== col));
  };

  const handleAddFile = (preset?: { target: string; desc: string }) => {
    const newFile: FileMappingDraft = preset
      ? {
          source_path: preset.target,
          target: preset.target,
          package_rel_path: `files/${preset.target.replace(/^~\/?\.?/, "")}`,
          description: preset.desc,
        }
      : {
          source_path: "",
          target: "~/.config/",
          package_rel_path: "",
          description: "",
        };
    setFiles([...files, newFile]);
  };

  const handleUpdateFile = (index: number, patch: Partial<FileMappingDraft>) => {
    const updated = [...files];
    updated[index] = { ...updated[index], ...patch };
    setFiles(updated);
  };

  const handleRemoveFile = (index: number) => {
    setFiles(files.filter((_, i) => i !== index));
  };

  const handleAddRequiredBin = () => {
    const b = binInput.trim().toLowerCase();
    if (b && !requiredBins.includes(b)) {
      setRequiredBins([...requiredBins, b]);
      setBinInput("");
    }
  };

  const handleRemoveRequiredBin = (bin: string) => {
    setRequiredBins(requiredBins.filter((b) => b !== bin));
  };

  const handleAddDependency = () => {
    const depId = depIdInput.trim().toLowerCase();
    if (!depId) return;
    const newDep: DependencySpec = {
      id: depId,
      kind: "package",
      version_req: depVersionInput.trim() ? depVersionInput.trim() : undefined,
      required: true,
      description: `Required package ${depId}`,
    };
    setDependencies([...dependencies, newDep]);
    setDepIdInput("");
    setDepVersionInput("");
  };

  const handleRemoveDependency = (depId: string) => {
    setDependencies(dependencies.filter((d) => d.id !== depId));
  };

  // Build the PackageDraft object
  const currentDraft: PackageDraft = useMemo(() => {
    return {
      id: id.trim(),
      name: name.trim(),
      version: version.trim(),
      author: author.trim(),
      package_type: packageType,
      description: description.trim(),
      tags,
      color_palette: colorPalette,
      compatibility: {
        desktops: selectedDesktops,
        sessions: sessionType === "any" ? [] : [sessionType],
        distros: [],
        required: requiredBins,
        optional: [],
      },
      dependencies,
      files,
    };
  }, [
    id,
    name,
    version,
    author,
    packageType,
    description,
    tags,
    colorPalette,
    selectedDesktops,
    sessionType,
    requiredBins,
    dependencies,
    files,
  ]);

  // Live validation
  const runValidation = async () => {
    setValidating(true);
    try {
      const res = await validatePackageDraft(currentDraft);
      setValidationResult(res);
    } catch (e: any) {
      setValidationResult({
        valid: false,
        errors: [String(e)],
        warnings: [],
      });
    } finally {
      setValidating(false);
    }
  };

  useEffect(() => {
    if (currentStep === 4) {
      runValidation();
    }
  }, [currentStep, currentDraft]);

  // Bundle creation
  const handleBuildPackage = async () => {
    setIsBuilding(true);
    try {
      const res = await createPackage(currentDraft, destinationDir);
      setAuthoringResult(res);
      setToast({
        message: `Package ${currentDraft.id} v${currentDraft.version} successfully built!`,
        type: "success",
      });
    } catch (e: any) {
      setToast({
        message: `Package bundling failed: ${e}`,
        type: "warning",
      });
    } finally {
      setIsBuilding(false);
    }
  };

  // Publish to repository
  const handlePublish = async () => {
    if (!authoringResult?.package_dir) {
      setToast({
        message: "Please build the package bundle first before publishing",
        type: "warning",
      });
      return;
    }
    const repoTarget = customRepoPath.trim() || selectedRepoPath;
    if (!repoTarget) {
      setToast({
        message: "Please specify a target repository directory",
        type: "warning",
      });
      return;
    }

    setIsPublishing(true);
    try {
      const res = await publishPackageToRepository(authoringResult.package_dir, repoTarget);
      setPublishResult(res);
      setToast({
        message: `Package published successfully to ${res.package_id}!`,
        type: "success",
      });
    } catch (e: any) {
      setToast({
        message: `Publishing failed: ${e}`,
        type: "warning",
      });
    } finally {
      setIsPublishing(false);
    }
  };

  return (
    <div className="space-y-6 pb-12 animate-fade-in">
      {/* Header Banner */}
      <div className="flex items-center justify-between border-b border-[var(--border-subtle)] pb-5">
        <div>
          <div className="flex items-center gap-3">
            <div className="p-2.5 rounded-xl bg-gradient-to-tr from-[var(--accent)] to-purple-600 text-white shadow-lg shadow-[var(--accent)]/20">
              <PackagePlus className="w-6 h-6" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight text-[var(--text-primary)]">
                Package Creator & Publisher
              </h1>
              <p className="text-sm text-[var(--text-muted)]">
                Author, bundle, checksum, and publish declarative Ryzora packages with zero manual JSON editing
              </p>
            </div>
          </div>
        </div>

        {/* Step Indicator */}
        <div className="flex items-center gap-1.5 bg-[var(--bg-surface)] p-1.5 rounded-xl border border-[var(--border-subtle)]">
          {[
            { num: 1, label: "Metadata" },
            { num: 2, label: "Compatibility" },
            { num: 3, label: "Files" },
            { num: 4, label: "Validate" },
            { num: 5, label: "Publish" },
          ].map((s) => (
            <button
              key={s.num}
              onClick={() => setCurrentStep(s.num as any)}
              className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all ${
                currentStep === s.num
                  ? "bg-[var(--accent)] text-white shadow-sm"
                  : currentStep > s.num
                  ? "text-emerald-400 bg-emerald-500/10 hover:bg-emerald-500/20"
                  : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
              }`}
            >
              <span>{s.num}.</span>
              <span>{s.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* STEP 1: IDENTITY & METADATA */}
      {currentStep === 1 && (
        <div className="bg-[var(--bg-card)] border border-[var(--border-subtle)] rounded-2xl p-6 shadow-sm space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-[var(--accent)]" />
              Step 1: Package Identity & Visuals
            </h2>
            <p className="text-xs text-[var(--text-muted)] mt-1">
              Define the identity, category, and visual presentation for your package
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
            {/* Package Display Name */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                Package Display Name *
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => handleNameChange(e.target.value)}
                placeholder="e.g. Cyberpunk Neon Hyprland"
                className="w-full px-3.5 py-2 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)]"
              />
            </div>

            {/* Package ID Slug */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                Package ID (Slug) *
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={id}
                  onChange={(e) => {
                    setId(e.target.value);
                    setIdTouched(true);
                  }}
                  placeholder="e.g. cyberpunk-neon-hyprland"
                  className="w-full px-3.5 py-2 font-mono rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)]"
                />
              </div>
              <p className="text-[11px] text-[var(--text-muted)] mt-1">
                Unique lowercase slug (letters, numbers, dashes)
              </p>
            </div>

            {/* Version */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                SemVer Version *
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="text"
                  value={version}
                  onChange={(e) => setVersion(e.target.value)}
                  placeholder="1.0.0"
                  className="w-full px-3.5 py-2 font-mono rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)]"
                />
                <button
                  type="button"
                  onClick={() => {
                    const parts = version.split(".");
                    if (parts.length === 3) {
                      setVersion(`${parts[0]}.${parts[1]}.${parseInt(parts[2] || "0") + 1}`);
                    }
                  }}
                  className="px-2.5 py-2 text-xs font-medium rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] hover:bg-[var(--bg-card-hover)]"
                >
                  +Patch
                </button>
                <button
                  type="button"
                  onClick={() => {
                    const parts = version.split(".");
                    if (parts.length === 3) {
                      setVersion(`${parts[0]}.${parseInt(parts[1] || "0") + 1}.0`);
                    }
                  }}
                  className="px-2.5 py-2 text-xs font-medium rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] hover:bg-[var(--bg-card-hover)]"
                >
                  +Minor
                </button>
              </div>
            </div>

            {/* Author */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                Author Handle / Name *
              </label>
              <input
                type="text"
                value={author}
                onChange={(e) => setAuthor(e.target.value)}
                placeholder="e.g. your_username"
                className="w-full px-3.5 py-2 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)]"
              />
            </div>

            {/* Package Type */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                Package Category / Type *
              </label>
              <select
                value={packageType}
                onChange={(e) => setPackageType(e.target.value as PackageType)}
                className="w-full px-3.5 py-2 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)] capitalize"
              >
                {PACKAGE_TYPES.map((pt) => (
                  <option key={pt} value={pt}>
                    {pt.toUpperCase()}
                  </option>
                ))}
              </select>
            </div>

            {/* Description */}
            <div className="md:col-span-2">
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5">
                Description
              </label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                rows={3}
                placeholder="Describe the aesthetic, wallpaper sources, keybinds, and visual features of this package..."
                className="w-full px-3.5 py-2 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-sm focus:outline-none focus:border-[var(--accent)]"
              />
            </div>

            {/* Tags */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5 flex items-center gap-1.5">
                <Tag className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                Searchable Tags
              </label>
              <div className="flex items-center gap-2 mb-2">
                <input
                  type="text"
                  value={tagInput}
                  onChange={(e) => setTagInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), handleAddTag())}
                  placeholder="Add tag (e.g. hyprland, dark, oled)..."
                  className="w-full px-3.5 py-1.5 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs focus:outline-none focus:border-[var(--accent)]"
                />
                <button
                  type="button"
                  onClick={handleAddTag}
                  className="px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--accent)] text-white hover:opacity-90"
                >
                  Add
                </button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {tags.map((t) => (
                  <span
                    key={t}
                    className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-[var(--text-primary)]"
                  >
                    #{t}
                    <button
                      onClick={() => handleRemoveTag(t)}
                      className="hover:text-red-400 ml-1 text-xs"
                    >
                      &times;
                    </button>
                  </span>
                ))}
              </div>
            </div>

            {/* Color Palette */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5 flex items-center gap-1.5">
                <Palette className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                Color Palette
              </label>
              <div className="flex items-center gap-2 mb-2">
                <input
                  type="color"
                  value={colorInput}
                  onChange={(e) => setColorInput(e.target.value)}
                  className="w-9 h-9 rounded-xl border border-[var(--border-subtle)] bg-transparent cursor-pointer p-0.5"
                />
                <input
                  type="text"
                  value={colorInput}
                  onChange={(e) => setColorInput(e.target.value)}
                  className="w-28 px-3 py-1.5 font-mono rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs"
                />
                <button
                  type="button"
                  onClick={handleAddColor}
                  className="px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--accent)] text-white hover:opacity-90"
                >
                  Add Color
                </button>
              </div>
              <div className="flex flex-wrap gap-2">
                {colorPalette.map((c) => (
                  <div
                    key={c}
                    className="flex items-center gap-1.5 px-2 py-1 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs"
                  >
                    <span
                      className="w-3.5 h-3.5 rounded-full border border-white/20"
                      style={{ backgroundColor: c }}
                    />
                    <span className="font-mono text-[11px]">{c}</span>
                    <button
                      onClick={() => handleRemoveColor(c)}
                      className="hover:text-red-400 ml-1 text-xs"
                    >
                      &times;
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div className="flex justify-end pt-4 border-t border-[var(--border-subtle)]">
            <button
              type="button"
              onClick={() => setCurrentStep(2)}
              disabled={!id.trim() || !name.trim() || !version.trim() || !author.trim()}
              className="flex items-center gap-2 px-5 py-2 rounded-xl bg-[var(--accent)] text-white font-medium text-sm hover:opacity-90 disabled:opacity-40 disabled:cursor-not-allowed"
            >
              Next: Compatibility & Dependencies
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* STEP 2: COMPATIBILITY & DEPENDENCIES */}
      {currentStep === 2 && (
        <div className="bg-[var(--bg-card)] border border-[var(--border-subtle)] rounded-2xl p-6 shadow-sm space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
              <Sliders className="w-5 h-5 text-[var(--accent)]" />
              Step 2: Compatibility & Dependencies
            </h2>
            <p className="text-xs text-[var(--text-muted)] mt-1">
              Ensure users will have the correct compositor, display server, and system tools installed
            </p>
          </div>

          <div className="space-y-6">
            {/* Desktop Environments */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-2">
                Supported Desktop Environments / Window Managers
              </label>
              <div className="flex flex-wrap gap-2">
                {DESKTOP_OPTIONS.map((dt) => {
                  const isSelected = selectedDesktops.includes(dt);
                  return (
                    <button
                      key={dt}
                      type="button"
                      onClick={() => {
                        if (isSelected) {
                          setSelectedDesktops(selectedDesktops.filter((d) => d !== dt));
                        } else {
                          setSelectedDesktops([...selectedDesktops, dt]);
                        }
                      }}
                      className={`px-3 py-1.5 rounded-xl text-xs font-medium border transition-all capitalize ${
                        isSelected
                          ? "bg-[var(--accent)]/15 border-[var(--accent)] text-[var(--accent)] font-semibold"
                          : "bg-[var(--bg-surface)] border-[var(--border-subtle)] text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                      }`}
                    >
                      {dt}
                    </button>
                  );
                })}
              </div>
              <p className="text-[11px] text-[var(--text-muted)] mt-1.5">
                Leaving empty implies Universal compatibility with all desktop environments.
              </p>
            </div>

            {/* Display Session */}
            <div>
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-2">
                Display Session Protocol
              </label>
              <div className="flex gap-3">
                {[
                  { id: "wayland", label: "Wayland" },
                  { id: "x11", label: "X11" },
                  { id: "any", label: "Universal (Any)" },
                ].map((s) => (
                  <label
                    key={s.id}
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-xs border cursor-pointer ${
                      sessionType === s.id
                        ? "bg-[var(--accent)]/15 border-[var(--accent)] text-[var(--accent)] font-semibold"
                        : "bg-[var(--bg-surface)] border-[var(--border-subtle)] text-[var(--text-muted)]"
                    }`}
                  >
                    <input
                      type="radio"
                      name="sessionType"
                      value={s.id}
                      checked={sessionType === s.id}
                      onChange={() => setSessionType(s.id as any)}
                      className="hidden"
                    />
                    <span>{s.label}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* Required System Binaries */}
            <div className="pt-4 border-t border-[var(--border-subtle)]">
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5 flex items-center gap-1.5">
                <Hash className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                Required System Binaries (must exist in PATH)
              </label>
              <div className="flex items-center gap-2 mb-2">
                <input
                  type="text"
                  value={binInput}
                  onChange={(e) => setBinInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), handleAddRequiredBin())}
                  placeholder="e.g. hyprland, waybar, kitty, rofi, swww..."
                  className="w-full max-w-sm px-3.5 py-1.5 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs focus:outline-none focus:border-[var(--accent)] font-mono"
                />
                <button
                  type="button"
                  onClick={handleAddRequiredBin}
                  className="px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--accent)] text-white hover:opacity-90"
                >
                  Add Binary
                </button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {requiredBins.map((bin) => (
                  <span
                    key={bin}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[var(--text-primary)]"
                  >
                    <span>{bin}</span>
                    <button
                      onClick={() => handleRemoveRequiredBin(bin)}
                      className="hover:text-red-400 text-xs"
                    >
                      &times;
                    </button>
                  </span>
                ))}
                {requiredBins.length === 0 && (
                  <span className="text-xs text-[var(--text-muted)] italic">
                    No required binaries specified.
                  </span>
                )}
              </div>
            </div>

            {/* Package Dependencies (Phase 9) */}
            <div className="pt-4 border-t border-[var(--border-subtle)]">
              <label className="block text-xs font-semibold text-[var(--text-primary)] mb-1.5 flex items-center gap-1.5">
                <Layers className="w-3.5 h-3.5 text-[var(--text-muted)]" />
                Ryzora Package Dependencies (SemVer)
              </label>
              <div className="flex items-center gap-2 mb-2">
                <input
                  type="text"
                  value={depIdInput}
                  onChange={(e) => setDepIdInput(e.target.value)}
                  placeholder="Package ID (e.g. catppuccin-hyprland)"
                  className="w-64 px-3.5 py-1.5 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                />
                <input
                  type="text"
                  value={depVersionInput}
                  onChange={(e) => setDepVersionInput(e.target.value)}
                  placeholder="Constraint (e.g. ^1.0.0 or >=2.0.0)"
                  className="w-48 px-3.5 py-1.5 rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                />
                <button
                  type="button"
                  onClick={handleAddDependency}
                  className="px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--accent)] text-white hover:opacity-90"
                >
                  Add Dep
                </button>
              </div>
              <div className="space-y-1.5">
                {dependencies.map((dep) => (
                  <div
                    key={dep.id}
                    className="flex items-center justify-between px-3 py-1.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs font-mono"
                  >
                    <span>{dep.id} {dep.version_req ? `(${dep.version_req})` : "(any)"}</span>
                    <button
                      onClick={() => handleRemoveDependency(dep.id)}
                      className="text-red-400 hover:text-red-300"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
                {dependencies.length === 0 && (
                  <span className="text-xs text-[var(--text-muted)] italic">
                    No package dependencies declared.
                  </span>
                )}
              </div>
            </div>
          </div>

          <div className="flex justify-between pt-4 border-t border-[var(--border-subtle)]">
            <button
              type="button"
              onClick={() => setCurrentStep(1)}
              className="flex items-center gap-2 px-4 py-2 rounded-xl border border-[var(--border-subtle)] text-xs font-medium hover:bg-[var(--bg-card-hover)]"
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              type="button"
              onClick={() => setCurrentStep(3)}
              className="flex items-center gap-2 px-5 py-2 rounded-xl bg-[var(--accent)] text-white font-medium text-sm hover:opacity-90"
            >
              Next: File Mappings
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* STEP 3: FILE MAPPINGS */}
      {currentStep === 3 && (
        <div className="bg-[var(--bg-card)] border border-[var(--border-subtle)] rounded-2xl p-6 shadow-sm space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
                <FileCode className="w-5 h-5 text-[var(--accent)]" />
                Step 3: Configuration File Bundling
              </h2>
              <p className="text-xs text-[var(--text-muted)] mt-1">
                Map files from your machine into target home-relative paths (~/.config/...)
              </p>
            </div>
            <button
              type="button"
              onClick={() => handleAddFile()}
              className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--accent)] text-white hover:opacity-90 shadow-sm"
            >
              <Plus className="w-3.5 h-3.5" />
              Add Custom File
            </button>
          </div>

          {/* Preset Buttons */}
          <div>
            <label className="block text-xs font-semibold text-[var(--text-muted)] mb-2 uppercase tracking-wider">
              Quick Add Common Presets:
            </label>
            <div className="flex flex-wrap gap-2">
              {COMMON_PRESETS.map((p) => (
                <button
                  key={p.name}
                  type="button"
                  onClick={() => handleAddFile(p)}
                  className="px-2.5 py-1.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-surface)] text-xs font-medium text-[var(--text-primary)] hover:border-[var(--accent)] hover:text-[var(--accent)] transition-all flex items-center gap-1.5"
                >
                  <Plus className="w-3 h-3" />
                  {p.name}
                </button>
              ))}
            </div>
          </div>

          {/* File Entries */}
          <div className="space-y-4">
            {files.map((file, idx) => (
              <div
                key={idx}
                className="p-4 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-3"
              >
                <div className="flex items-center justify-between">
                  <span className="text-xs font-bold text-[var(--text-primary)] flex items-center gap-2">
                    <FileCode className="w-4 h-4 text-[var(--accent)]" />
                    File #{idx + 1}
                  </span>
                  <button
                    type="button"
                    onClick={() => handleRemoveFile(idx)}
                    className="text-xs text-red-400 hover:text-red-300 flex items-center gap-1"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                    Remove
                  </button>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  <div>
                    <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                      Source File Path on Author Disk *
                    </label>
                    <input
                      type="text"
                      value={file.source_path}
                      onChange={(e) => handleUpdateFile(idx, { source_path: e.target.value })}
                      placeholder="~/.config/hypr/hyprland.conf or /home/user/..."
                      className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                    />
                  </div>

                  <div>
                    <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                      Target Destination Path (MUST start with ~/) *
                    </label>
                    <input
                      type="text"
                      value={file.target}
                      onChange={(e) => handleUpdateFile(idx, { target: e.target.value })}
                      placeholder="~/.config/hypr/hyprland.conf"
                      className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                    />
                  </div>

                  <div>
                    <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                      Path in Package files/ (Optional - derived automatically)
                    </label>
                    <input
                      type="text"
                      value={file.package_rel_path || ""}
                      onChange={(e) => handleUpdateFile(idx, { package_rel_path: e.target.value })}
                      placeholder="e.g. files/hypr/hyprland.conf"
                      className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                    />
                  </div>

                  <div>
                    <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                      Description
                    </label>
                    <input
                      type="text"
                      value={file.description}
                      onChange={(e) => handleUpdateFile(idx, { description: e.target.value })}
                      placeholder="What this file configures"
                      className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs focus:outline-none focus:border-[var(--accent)]"
                    />
                  </div>
                </div>
              </div>
            ))}

            {files.length === 0 && (
              <div className="p-8 text-center border-2 border-dashed border-[var(--border-subtle)] rounded-xl text-xs text-[var(--text-muted)]">
                No files added yet. Click "Add Custom File" or pick a preset above.
              </div>
            )}
          </div>

          <div className="flex justify-between pt-4 border-t border-[var(--border-subtle)]">
            <button
              type="button"
              onClick={() => setCurrentStep(2)}
              className="flex items-center gap-2 px-4 py-2 rounded-xl border border-[var(--border-subtle)] text-xs font-medium hover:bg-[var(--bg-card-hover)]"
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              type="button"
              onClick={() => setCurrentStep(4)}
              disabled={files.length === 0}
              className="flex items-center gap-2 px-5 py-2 rounded-xl bg-[var(--accent)] text-white font-medium text-sm hover:opacity-90 disabled:opacity-40"
            >
              Next: Validate & Review
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* STEP 4: VALIDATE & REVIEW */}
      {currentStep === 4 && (
        <div className="bg-[var(--bg-card)] border border-[var(--border-subtle)] rounded-2xl p-6 shadow-sm space-y-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
                <ShieldCheck className="w-5 h-5 text-[var(--accent)]" />
                Step 4: Live Verification & Manifest Audit
              </h2>
              <p className="text-xs text-[var(--text-muted)] mt-1">
                Ryzora performs a dry-run safety verification against path traversals, missing source files, and schema rules
              </p>
            </div>
            <button
              type="button"
              onClick={runValidation}
              disabled={validating}
              className="flex items-center gap-2 px-3 py-1.5 text-xs font-semibold rounded-xl bg-[var(--bg-surface)] border border-[var(--border-subtle)] hover:bg-[var(--bg-card-hover)]"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${validating ? "animate-spin" : ""}`} />
              Re-validate
            </button>
          </div>

          {/* Validation Status Banner */}
          {validationResult && (
            <div
              className={`p-4 rounded-xl border ${
                validationResult.valid
                  ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-300"
                  : "bg-red-500/10 border-red-500/30 text-red-300"
              }`}
            >
              <div className="flex items-center gap-2 font-semibold text-sm">
                {validationResult.valid ? (
                  <>
                    <CheckCircle2 className="w-5 h-5 text-emerald-400" />
                    <span>Manifest Validation Passed — 100% Safe & Standard Compliant</span>
                  </>
                ) : (
                  <>
                    <XCircle className="w-5 h-5 text-red-400" />
                    <span>Validation Failed — {validationResult.errors.length} Issue(s) Detected</span>
                  </>
                )}
              </div>

              {validationResult.errors.length > 0 && (
                <ul className="mt-3 space-y-1 text-xs text-red-200 list-disc list-inside font-mono">
                  {validationResult.errors.map((err, i) => (
                    <li key={i}>{err}</li>
                  ))}
                </ul>
              )}

              {validationResult.warnings.length > 0 && (
                <div className="mt-3 pt-3 border-t border-amber-500/20 text-amber-200 text-xs space-y-1">
                  <div className="font-semibold flex items-center gap-1.5">
                    <AlertTriangle className="w-4 h-4 text-amber-400" />
                    Warnings:
                  </div>
                  <ul className="list-disc list-inside font-mono">
                    {validationResult.warnings.map((w, i) => (
                      <li key={i}>{w}</li>
                    ))}
                  </ul>
                </div>
              )}
            </div>
          )}

          {/* Live Manifest Preview */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-xs font-semibold text-[var(--text-muted)] uppercase tracking-wider">
                Generated ryzora.json Manifest Preview
              </span>
              <button
                type="button"
                onClick={() => {
                  navigator.clipboard.writeText(JSON.stringify(currentDraft, null, 2));
                  setToast({ message: "Manifest JSON copied to clipboard!", type: "success" });
                }}
                className="text-xs text-[var(--accent)] hover:underline flex items-center gap-1"
              >
                <Copy className="w-3 h-3" />
                Copy JSON
              </button>
            </div>
            <pre className="p-4 rounded-xl bg-black/50 border border-[var(--border-subtle)] text-[11px] font-mono text-emerald-400 overflow-x-auto max-h-80 select-all">
              {JSON.stringify(
                {
                  id: currentDraft.id,
                  name: currentDraft.name,
                  version: currentDraft.version,
                  ryzora_spec: "1",
                  author: currentDraft.author,
                  package_type: currentDraft.package_type,
                  description: currentDraft.description,
                  tags: currentDraft.tags,
                  color_palette: currentDraft.color_palette,
                  compatibility: currentDraft.compatibility,
                  files: currentDraft.files.map((f) => ({
                    source: f.package_rel_path || `files/${f.target.replace(/^~\/?\.?/, "")}`,
                    target: f.target,
                    description: f.description,
                  })),
                  dependencies: currentDraft.dependencies,
                },
                null,
                2
              )}
            </pre>
          </div>

          <div className="flex justify-between pt-4 border-t border-[var(--border-subtle)]">
            <button
              type="button"
              onClick={() => setCurrentStep(3)}
              className="flex items-center gap-2 px-4 py-2 rounded-xl border border-[var(--border-subtle)] text-xs font-medium hover:bg-[var(--bg-card-hover)]"
            >
              <ArrowLeft className="w-4 h-4" />
              Back
            </button>
            <button
              type="button"
              onClick={() => setCurrentStep(5)}
              disabled={validationResult ? !validationResult.valid : false}
              className="flex items-center gap-2 px-5 py-2 rounded-xl bg-[var(--accent)] text-white font-medium text-sm hover:opacity-90 disabled:opacity-40"
            >
              Next: Build & Publish
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* STEP 5: BUILD & PUBLISH */}
      {currentStep === 5 && (
        <div className="bg-[var(--bg-card)] border border-[var(--border-subtle)] rounded-2xl p-6 shadow-sm space-y-6">
          <div>
            <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
              <UploadCloud className="w-5 h-5 text-[var(--accent)]" />
              Step 5: Bundle Generation & Store Publishing
            </h2>
            <p className="text-xs text-[var(--text-muted)] mt-1">
              Build the standalone package directory and optionally publish directly into a local or community repository
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* Stage A: Build Package Bundle */}
            <div className="p-5 rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-4">
              <h3 className="text-sm font-bold text-[var(--text-primary)] flex items-center gap-2">
                <Folder className="w-4 h-4 text-[var(--accent)]" />
                Phase A: Build Package Bundle
              </h3>
              <p className="text-xs text-[var(--text-muted)]">
                Copies all configuration files into a canonical package directory with `ryzora.json` and computed SHA-256 digests.
              </p>

              <div>
                <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                  Output Destination Directory
                </label>
                <input
                  type="text"
                  value={destinationDir}
                  onChange={(e) => setDestinationDir(e.target.value)}
                  placeholder="~/RyzoraPackages"
                  className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                />
              </div>

              <button
                type="button"
                onClick={handleBuildPackage}
                disabled={isBuilding}
                className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-[var(--accent)] text-white font-medium text-xs hover:opacity-90 disabled:opacity-50 shadow-sm"
              >
                {isBuilding ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Bundling Files & Hashing...
                  </>
                ) : (
                  <>
                    <CheckCircle2 className="w-4 h-4" />
                    Build Package Bundle
                  </>
                )}
              </button>

              {authoringResult && (
                <div className="p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 text-xs space-y-1.5">
                  <div className="font-semibold flex items-center gap-1.5">
                    <Check className="w-4 h-4 text-emerald-400" />
                    Package Bundle Created Successfully!
                  </div>
                  <div className="text-[11px] space-y-0.5 font-mono text-emerald-200/80">
                    <div>Dir: {authoringResult.package_dir}</div>
                    <div>Files Copied: {authoringResult.files_copied}</div>
                    <div>Size: {(authoringResult.total_bytes / 1024).toFixed(1)} KB</div>
                    <div>SHA-256: {authoringResult.sha256_checksum.slice(0, 16)}...</div>
                  </div>
                </div>
              )}
            </div>

            {/* Stage B: Publish to Repository */}
            <div className="p-5 rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] space-y-4">
              <h3 className="text-sm font-bold text-[var(--text-primary)] flex items-center gap-2">
                <UploadCloud className="w-4 h-4 text-[var(--accent)]" />
                Phase B: Publish to Repository
              </h3>
              <p className="text-xs text-[var(--text-muted)]">
                Registers the package in a repository's `repository.json` catalog and stages the bundle into `packages/`.
              </p>

              <div>
                <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                  Select Configured Repository
                </label>
                <select
                  value={selectedRepoPath}
                  onChange={(e) => setSelectedRepoPath(e.target.value)}
                  className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs focus:outline-none focus:border-[var(--accent)]"
                >
                  <option value="">Select a repository...</option>
                  {repositories.map((repo) => (
                    <option key={repo.id} value={repo.path}>
                      {repo.name} ({repo.id}) — {repo.path}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-[11px] font-semibold text-[var(--text-muted)] mb-1">
                  Or Custom Repository Directory
                </label>
                <input
                  type="text"
                  value={customRepoPath}
                  onChange={(e) => setCustomRepoPath(e.target.value)}
                  placeholder="/path/to/my-custom-repo or ~/my-repo"
                  className="w-full px-3 py-1.5 rounded-lg bg-[var(--bg-card)] border border-[var(--border-subtle)] text-xs font-mono focus:outline-none focus:border-[var(--accent)]"
                />
              </div>

              <button
                type="button"
                onClick={handlePublish}
                disabled={isPublishing || !authoringResult}
                className="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-xl bg-purple-600 text-white font-medium text-xs hover:opacity-90 disabled:opacity-50 shadow-sm"
              >
                {isPublishing ? (
                  <>
                    <RefreshCw className="w-4 h-4 animate-spin" />
                    Publishing & Verifying Catalog...
                  </>
                ) : (
                  <>
                    <UploadCloud className="w-4 h-4" />
                    Publish to Repository
                  </>
                )}
              </button>

              {publishResult && (
                <div className="p-3.5 rounded-xl bg-emerald-500/10 border border-emerald-500/30 text-emerald-300 text-xs space-y-2">
                  <div className="font-semibold flex items-center gap-1.5">
                    <Check className="w-4 h-4 text-emerald-400" />
                    {publishResult.message}
                  </div>
                  <p className="text-[11px] text-emerald-200/80">
                    The package is now officially published in the repository index and is immediately visible in Ryzora's catalog!
                  </p>
                  <button
                    type="button"
                    onClick={() => setActiveCategory("discover")}
                    className="mt-2 px-3 py-1.5 rounded-lg bg-emerald-500 text-black font-semibold text-xs hover:opacity-90"
                  >
                    View in Store / Discover
                  </button>
                </div>
              )}
            </div>
          </div>

          <div className="flex justify-start pt-4 border-t border-[var(--border-subtle)]">
            <button
              type="button"
              onClick={() => setCurrentStep(4)}
              className="flex items-center gap-2 px-4 py-2 rounded-xl border border-[var(--border-subtle)] text-xs font-medium hover:bg-[var(--bg-card-hover)]"
            >
              <ArrowLeft className="w-4 h-4" />
              Back to Validation
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
