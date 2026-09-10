
function getOptionEffect(key: string, value: any, packageSlug: string): string {
  if (key === "themeMode") {
    return value === "light"
      ? "Sets high-contrast #ffffff light surface in Main.qml"
      : "Sets deep #000000 dark surface in Main.qml";
  }
  if (key === "enableWindup") {
    return Boolean(value)
      ? "Plays mechanical gear train windup animation on unlock"
      : "Bypasses gear windup animation for instant session unlock";
  }
  if (key === "background_mode") {
    if (value === "time") return "Transitions automatically with real-time day/night cycle";
    if (value === "random") return "Picks a random biome/atmosphere on each lock sequence";
    return "Locks background to the selected static wallpaper index";
  }
  if (key === "background_index") {
    if (packageSlug.includes("terraria")) {
      const biomes = ["", "Forest mountains (ter1.png)", "Tall mountains (ter2.png)", "Halloween lands (ter3.png)", "Midnight scary (ter4.png)", "Icy mountains (ter5.png)"];
      return "Sets wallpaper to " + (biomes[Number(value)] || `Biome ${value}`);
    }
    if (packageSlug.includes("genshin")) {
      const atmos = ["", "Dawn video (dawn.mp4)", "Day video (day.mp4)", "Dusk video (dusk.mp4)", "Night video (night.mp4)"];
      return "Sets atmosphere to " + (atmos[Number(value)] || `Atmosphere ${value}`);
    }
    return `Selects static asset index ${value}`;
  }
  if (key === "gameMode") {
    return value === "menu"
      ? "Bypasses rhythm minigame; renders direct password input"
      : "Requires completing rhythm hit-circle game before unlock";
  }
  return "Persists " + key + "=" + value + " in theme.conf under [General]";
}

import React from "react";
import {
  Sliders,
  RotateCcw,
  Palette,
  Check,
  Sparkles,
  Sun,
  Moon,
  Shuffle,
  Clock,
  Gamepad2,
  KeyRound,
  Image as ImageIcon,
  Play,
} from "lucide-react";
import type { LockscreenConfigSchema, PackageItem } from "../../types/index.ts";

interface LockScreenCustomizerProps {
  packageItem: PackageItem;
  configSchema: LockscreenConfigSchema;
  currentConfig: Record<string, any>;
  selectedVariantId: string;
  onChangeConfig: (newConfig: Record<string, any>) => void;
  onSelectVariant: (variantId: string) => void;
}

export const LockScreenCustomizer: React.FC<LockScreenCustomizerProps> = ({
  packageItem,
  configSchema,
  currentConfig,
  selectedVariantId,
  onChangeConfig,
  onSelectVariant,
}) => {
  const variants = configSchema.variants || [];
  const options = configSchema.options || {};
  const optionEntries = Object.entries(options);

  // If there are no variants and no options, do NOT render any customization UI
  if (variants.length === 0 && optionEntries.length === 0) {
    return null;
  }

  const activeVariantId = selectedVariantId || variants[0]?.id || "";
  const activeVariant = variants.find((v) => v.id === activeVariantId) || variants[0];

  const handleOptionChange = (key: string, value: any) => {
    onChangeConfig({
      ...currentConfig,
      [key]: value,
    });
  };

  const handleResetDefaults = () => {
    const defaults: Record<string, any> = {};
    for (const [key, spec] of optionEntries) {
      defaults[key] = spec.default;
    }
    onChangeConfig(defaults);
    if (variants.length > 0) {
      onSelectVariant(variants[0].id);
    }
  };

  // Extract authentic known controls
  const themeModeSpec = options["themeMode"];
  const enableWindupSpec = options["enableWindup"];
  const bgModeSpec = options["background_mode"];
  const bgIndexSpec = options["background_index"];
  const gameModeSpec = options["gameMode"];

  // Remaining options not handled by dedicated UI controls
  const handledKeys = new Set(["themeMode", "enableWindup", "background_mode", "background_index", "gameMode"]);
  const remainingOptions = optionEntries.filter(([k]) => !handledKeys.has(k));

  const currentBgMode = currentConfig["background_mode"] ?? bgModeSpec?.default ?? "time";
  const isStaticBgActive = currentBgMode === "static";

  return (
    <section
      className="rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] overflow-hidden shadow-xl my-5"
    >
      {/* ── Header ── */}
      <div className="px-5 py-4 border-b border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)]/80 flex items-center justify-between gap-4 flex-wrap">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-400">
            <Sliders size={18} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h2 className="text-sm font-bold text-[var(--rz-text)] tracking-tight">
                Authentic Theme Customization
              </h2>
              <span className="px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                UPSTREAM VERIFIED
              </span>
            </div>
            <p className="text-xs text-[var(--rz-text-secondary)] mt-0.5">
              Configure supported runtime parameters for {packageItem.title}.
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={handleResetDefaults}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong,#363e50)] transition-all cursor-pointer select-none"
        >
          <RotateCcw size={13} />
          <span>Reset Defaults</span>
        </button>
      </div>

      <div className="p-5 space-y-6">
        {/* ── 1. THEME VARIANTS (when variants are present) ── */}
        {variants.length > 0 && (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <Palette size={14} className="text-purple-400" />
              <h3 className="text-[11px] font-mono font-bold uppercase tracking-wider text-[var(--rz-text-secondary)]">
                Theme Variant
              </h3>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5">
              {variants.map((v) => {
                const isSelected = v.id === activeVariantId;
                const previewImg = v.preview_image || packageItem.preview_poster_url || packageItem.hero_image;

                return (
                  <button
                    key={v.id}
                    type="button"
                    onClick={() => onSelectVariant(v.id)}
                    className={`relative p-3.5 rounded-xl border text-left transition-all cursor-pointer flex gap-3 select-none ${
                      isSelected
                        ? "bg-emerald-950/25 border-emerald-500 ring-1 ring-emerald-500/50 shadow-md"
                        : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong,#363e50)] hover:bg-[var(--rz-surface-hover,#222938)]"
                    }`}
                  >
                    {previewImg && (
                      <div className="w-14 h-14 rounded-lg overflow-hidden shrink-0 border border-white/10 bg-black/40">
                        <img
                          src={previewImg}
                          alt={v.name}
                          className="w-full h-full object-cover"
                          loading="lazy"
                        />
                      </div>
                    )}

                    <div className="flex-1 min-w-0 pr-6">
                      <div className="flex items-center gap-1.5">
                        <h4 className="text-xs font-bold text-[var(--rz-text)] truncate">
                          {v.name}
                        </h4>
                      </div>
                      {v.description && (
                        <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 line-clamp-2 leading-relaxed">
                          {v.description}
                        </p>
                      )}
                    </div>

                    {isSelected && (
                      <div className="absolute top-3 right-3 w-5 h-5 rounded-full bg-emerald-500 text-black flex items-center justify-center shadow-sm">
                        <Check size={12} strokeWidth={3} />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* ── 2. AUTHENTIC OPTIONS GRID ── */}
        {optionEntries.length > 0 && (
          <div className="space-y-4">
            <div className="flex items-center gap-2">
              <Sliders size={14} className="text-emerald-400" />
              <h3 className="text-[11px] font-mono font-bold uppercase tracking-wider text-[var(--rz-text-secondary)]">
                Runtime Options
              </h3>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* Option: themeMode (Dark / Light) */}
              {themeModeSpec && (
                <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold text-[var(--rz-text)] flex items-center gap-1.5">
                      <Moon size={14} className="text-purple-400" />
                      <span>{themeModeSpec.label || "Theme Appearance"}</span>
                    </span>
                    <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 leading-normal">
                      Select light or dark contrast palette for interface elements.
                    </p>
                  </div>

                  <div className="grid grid-cols-2 gap-1.5 p-1 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    {themeModeSpec.options?.map((opt) => {
                      const currentVal = currentConfig["themeMode"] ?? themeModeSpec.default;
                      const active = String(currentVal) === String(opt.value);

                      return (
                        <button
                          key={opt.value}
                          type="button"
                          onClick={() => handleOptionChange("themeMode", opt.value)}
                          className={`flex items-center justify-center gap-2 px-3 py-2 text-xs font-semibold rounded-md transition-all cursor-pointer ${
                            active
                              ? "bg-emerald-500 text-black shadow-xs font-bold"
                              : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                          }`}
                        >
                          {opt.value === "light" ? <Sun size={13} /> : <Moon size={13} />}
                          <span>{opt.label}</span>
                        </button>
                      );
                    })}
                  </div>
                  <div className="pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono flex items-center justify-between text-[var(--rz-text-muted)] flex-wrap gap-1">
                    <span className="flex items-center gap-1">
                      <span className="text-emerald-400 font-bold">themeMode</span>
                      <span>→</span>
                      <span className="text-[var(--rz-text)] font-semibold">{String(currentConfig["themeMode"] ?? themeModeSpec.default)}</span>
                    </span>
                    <span className="text-[10px] text-purple-300/90 text-right">
                      {getOptionEffect("themeMode", currentConfig["themeMode"] ?? themeModeSpec.default, packageItem.id)}
                    </span>
                  </div>
                </div>
              )}

              {/* Option: enableWindup (Windup toggle) */}
              {enableWindupSpec && (
                <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold text-[var(--rz-text)] flex items-center gap-1.5">
                      <Play size={14} className="text-amber-400" />
                      <span>{enableWindupSpec.label || "Windup Animation"}</span>
                    </span>
                    <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 leading-normal">
                      {enableWindupSpec.description || "Play mechanical gear train windup sequence during unlock."}
                    </p>
                  </div>

                  {(() => {
                    const val = currentConfig["enableWindup"] !== undefined ? currentConfig["enableWindup"] : enableWindupSpec.default;
                    const isChecked = Boolean(val);

                    return (
                      <div className="flex items-center justify-between p-2.5 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                        <span className="text-xs font-medium text-[var(--rz-text)]">
                          {isChecked ? "Windup Enabled (On)" : "Windup Disabled (Off)"}
                        </span>
                        <button
                          type="button"
                          role="switch"
                          aria-checked={isChecked}
                          onClick={() => handleOptionChange("enableWindup", !isChecked)}
                          className={`w-10 h-6 rounded-full p-0.5 transition-colors cursor-pointer flex items-center ${
                            isChecked ? "bg-emerald-500 justify-end" : "bg-zinc-700 justify-start"
                          }`}
                        >
                          <span className="w-5 h-5 rounded-full bg-white shadow-xs" />
                        </button>
                      </div>
                    );
                  })()}
                  <div className="pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono flex items-center justify-between text-[var(--rz-text-muted)] flex-wrap gap-1">
                    <span className="flex items-center gap-1">
                      <span className="text-emerald-400 font-bold">enableWindup</span>
                      <span>→</span>
                      <span className="text-[var(--rz-text)] font-semibold">{String(currentConfig["enableWindup"] !== undefined ? currentConfig["enableWindup"] : enableWindupSpec.default)}</span>
                    </span>
                    <span className="text-[10px] text-purple-300/90 text-right">
                      {getOptionEffect("enableWindup", currentConfig["enableWindup"] !== undefined ? currentConfig["enableWindup"] : enableWindupSpec.default, packageItem.id)}
                    </span>
                  </div>
                </div>
              )}

              {/* Option: background_mode (Terraria / Genshin atmosphere transition) */}
              {bgModeSpec && (
                <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold text-[var(--rz-text)] flex items-center gap-1.5">
                      <Shuffle size={14} className="text-cyan-400" />
                      <span>{bgModeSpec.label || "Background Mode"}</span>
                    </span>
                    <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 leading-normal">
                      {bgModeSpec.description || "Controls how dynamic wallpapers transition."}
                    </p>
                  </div>

                  <div className="grid grid-cols-3 gap-1 p-1 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    {bgModeSpec.options?.map((opt) => {
                      const currentVal = currentConfig["background_mode"] ?? bgModeSpec.default;
                      const active = String(currentVal) === String(opt.value);

                      return (
                        <button
                          key={opt.value}
                          type="button"
                          onClick={() => handleOptionChange("background_mode", opt.value)}
                          className={`flex items-center justify-center gap-1.5 px-2 py-2 text-xs font-medium rounded-md transition-all cursor-pointer text-center truncate ${
                            active
                              ? "bg-emerald-500 text-black font-bold shadow-xs"
                              : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                          }`}
                        >
                          {opt.value === "time" ? <Clock size={12} /> : opt.value === "random" ? <Shuffle size={12} /> : <ImageIcon size={12} />}
                          <span className="truncate">{opt.label}</span>
                        </button>
                      );
                    })}
                  </div>
                  <div className="pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono flex items-center justify-between text-[var(--rz-text-muted)] flex-wrap gap-1">
                    <span className="flex items-center gap-1">
                      <span className="text-emerald-400 font-bold">background_mode</span>
                      <span>→</span>
                      <span className="text-[var(--rz-text)] font-semibold">{String(currentBgMode)}</span>
                    </span>
                    <span className="text-[10px] text-purple-300/90 text-right">
                      {getOptionEffect("background_mode", currentBgMode, packageItem.id)}
                    </span>
                  </div>
                </div>
              )}

              {/* Option: background_index (Terraria / Genshin static index) */}
              {bgIndexSpec && (
                <div
                  className={`p-4 rounded-xl border transition-all flex flex-col justify-between gap-3 ${
                    isStaticBgActive
                      ? "border-emerald-500/60 bg-[var(--rz-surface-elevated)] shadow-sm"
                      : "border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)]/60 opacity-75"
                  }`}
                >
                  <div>
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-xs font-bold text-[var(--rz-text)] flex items-center gap-1.5">
                        <ImageIcon size={14} className="text-amber-400" />
                        <span>{bgIndexSpec.label || "Static Biome Wallpaper"}</span>
                      </span>
                      {!isStaticBgActive && (
                        <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-400 border border-zinc-700">
                          Applies in Static Mode
                        </span>
                      )}
                    </div>
                    <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 leading-normal">
                      {bgIndexSpec.description || "Active wallpaper selection when in static mode."}
                    </p>
                  </div>

                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5 p-1 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    {bgIndexSpec.options?.map((opt) => {
                      const currentVal = currentConfig["background_index"] ?? bgIndexSpec.default;
                      const active = String(currentVal) === String(opt.value);

                      return (
                        <button
                          key={opt.value}
                          type="button"
                          onClick={() => {
                            handleOptionChange("background_index", opt.value);
                            if (!isStaticBgActive) {
                              handleOptionChange("background_mode", "static");
                            }
                          }}
                          className={`px-2.5 py-1.5 text-xs font-medium rounded-md transition-all cursor-pointer text-center truncate ${
                            active
                              ? "bg-emerald-500 text-black font-bold shadow-xs"
                              : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                          }`}
                        >
                          {opt.label}
                        </button>
                      );
                    })}
                  </div>
                  <div className="pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono flex items-center justify-between text-[var(--rz-text-muted)] flex-wrap gap-1">
                    <span className="flex items-center gap-1">
                      <span className="text-emerald-400 font-bold">background_index</span>
                      <span>→</span>
                      <span className="text-[var(--rz-text)] font-semibold">{String(currentConfig["background_index"] ?? bgIndexSpec?.default)}</span>
                    </span>
                    <span className="text-[10px] text-purple-300/90 text-right">
                      {getOptionEffect("background_index", currentConfig["background_index"] ?? bgIndexSpec?.default, packageItem.id)}
                    </span>
                  </div>
                </div>
              )}

              {/* Option: gameMode (osu / osumania rhythm gate vs direct login) */}
              {gameModeSpec && (
                <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
                  <div>
                    <span className="text-xs font-bold text-[var(--rz-text)] flex items-center gap-1.5">
                      <Gamepad2 size={14} className="text-pink-400" />
                      <span>{gameModeSpec.label || "Login Mode"}</span>
                    </span>
                    <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5 leading-normal">
                      {gameModeSpec.description || "Select between rhythm game unlock gate or direct password authentication."}
                    </p>
                  </div>

                  <div className="grid grid-cols-2 gap-1.5 p-1 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                    {gameModeSpec.options?.map((opt) => {
                      const currentVal = currentConfig["gameMode"] ?? gameModeSpec.default;
                      const active = String(currentVal) === String(opt.value);

                      return (
                        <button
                          key={opt.value}
                          type="button"
                          onClick={() => handleOptionChange("gameMode", opt.value)}
                          className={`flex items-center justify-center gap-2 px-3 py-2 text-xs font-semibold rounded-md transition-all cursor-pointer ${
                            active
                              ? "bg-emerald-500 text-black font-bold shadow-xs"
                              : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                          }`}
                        >
                          {opt.value === "game" ? <Gamepad2 size={13} /> : <KeyRound size={13} />}
                          <span>{opt.label}</span>
                        </button>
                      );
                    })}
                  </div>
                  <div className="pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono flex items-center justify-between text-[var(--rz-text-muted)] flex-wrap gap-1">
                    <span className="flex items-center gap-1">
                      <span className="text-emerald-400 font-bold">gameMode</span>
                      <span>→</span>
                      <span className="text-[var(--rz-text)] font-semibold">{String(currentConfig["gameMode"] ?? gameModeSpec.default)}</span>
                    </span>
                    <span className="text-[10px] text-purple-300/90 text-right">
                      {getOptionEffect("gameMode", currentConfig["gameMode"] ?? gameModeSpec.default, packageItem.id)}
                    </span>
                  </div>
                </div>
              )}
            </div>

            {/* Any remaining authentic options (if any) */}
            {remainingOptions.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2">
                {remainingOptions.map(([key, spec]) => {
                  const val = currentConfig[key] !== undefined ? currentConfig[key] : spec.default;

                  return (
                    <div
                      key={key}
                      className="p-3.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-2"
                    >
                      <div>
                        <label className="text-xs font-bold text-[var(--rz-text)]">
                          {spec.label}
                        </label>
                        {spec.description && (
                          <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5">
                            {spec.description}
                          </p>
                        )}
                      </div>

                      {spec.type === "select" && spec.options && (
                        <select
                          value={String(val)}
                          onChange={(e) => handleOptionChange(key, e.target.value)}
                          className="w-full text-xs font-medium px-3 py-1.5 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] focus:border-emerald-500 focus:outline-none transition-colors cursor-pointer"
                        >
                          {spec.options.map((opt) => (
                            <option key={opt.value} value={opt.value} className="bg-zinc-900 text-zinc-100">
                              {opt.label}
                            </option>
                          ))}
                        </select>
                      )}

                      {spec.type === "boolean" && (
                        <div className="flex items-center justify-between p-2 rounded-lg bg-[var(--rz-bg)] border border-[var(--rz-border-subtle)]">
                          <span className="text-xs font-medium text-[var(--rz-text)]">
                            {val ? "Enabled" : "Disabled"}
                          </span>
                          <button
                            type="button"
                            role="switch"
                            aria-checked={Boolean(val)}
                            onClick={() => handleOptionChange(key, !val)}
                            className={`w-9 h-5 rounded-full p-0.5 transition-colors cursor-pointer flex items-center ${
                              val ? "bg-emerald-500 justify-end" : "bg-zinc-700 justify-start"
                            }`}
                          >
                            <span className="w-4 h-4 rounded-full bg-white shadow-xs" />
                          </button>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* ── 3. Live Configuration Summary Readout ── */}
        <div className="p-3.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex flex-wrap items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
            <span className="font-semibold text-[var(--rz-text)]">Materialization Target:</span>
            <span className="font-mono text-emerald-400 font-bold">
              {activeVariant ? activeVariant.name : "Default"}
            </span>
            {optionEntries.length > 0 && (
              <span className="text-[11px] text-[var(--rz-text-muted)]">
                ({optionEntries.length} verified parameters)
              </span>
            )}
          </div>
          <div className="flex items-center gap-1.5 text-[11px] text-emerald-400 font-medium">
            <Sparkles size={13} />
            <span>Parameters write directly to theme.conf and ryzora_config.json</span>
          </div>
        </div>
      </div>
    </section>
  );
};
