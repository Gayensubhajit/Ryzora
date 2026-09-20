import React, { useState } from "react";
import {
  RotateCcw,
  Check,
  Sun,
  Moon,
  Shuffle,
  Clock,
  Gamepad2,
  KeyRound,
  Image as ImageIcon,
  ChevronRight,
} from "lucide-react";
import type { LockscreenConfigSchema, PackageItem } from "../../types/index.ts";

function getOptionEffect(key: string, value: any, packageSlug: string): string {
  if (key === "themeMode") {
    return value === "light"
      ? "Sets light appearance in theme configuration"
      : "Sets dark appearance in theme configuration";
  }
  if (key === "enableWindup") {
    return Boolean(value)
      ? "Plays mechanical gear train windup animation on unlock"
      : "Instant session unlock without windup animation";
  }
  if (key === "background_mode") {
    if (value === "time") return "Transitions with real-time day/night cycle";
    if (value === "random") return "Picks a random atmosphere on each lock sequence";
    return "Uses selected static wallpaper index";
  }
  if (key === "background_index") {
    if (packageSlug.includes("terraria")) {
      const biomes = ["", "Forest mountains", "Tall mountains", "Halloween lands", "Midnight scary", "Icy mountains"];
      return "Sets wallpaper to " + (biomes[Number(value)] || `Biome ${value}`);
    }
    if (packageSlug.includes("genshin")) {
      const atmos = ["", "Dawn video", "Day video", "Dusk video", "Night video"];
      return "Sets atmosphere to " + (atmos[Number(value)] || `Atmosphere ${value}`);
    }
    return `Selects static asset index ${value}`;
  }
  if (key === "gameMode") {
    return value === "menu"
      ? "Direct password authentication input"
      : "Rhythm hit-circle minigame before unlock";
  }
  return "Persists " + key + "=" + value + " in theme configuration";
}

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
  const [showAdvanced, setShowAdvanced] = useState(false);
  const variants = configSchema.variants || [];
  const options = configSchema.options || {};
  const optionEntries = Object.entries(options);

  if (variants.length === 0 && optionEntries.length === 0) {
    return null;
  }

  const activeVariantId = selectedVariantId || variants[0]?.id || "";

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

  const themeModeSpec = options["themeMode"];
  const enableWindupSpec = options["enableWindup"];
  const bgModeSpec = options["background_mode"];
  const bgIndexSpec = options["background_index"];
  const gameModeSpec = options["gameMode"];

  const handledKeys = new Set(["themeMode", "enableWindup", "background_mode", "background_index", "gameMode"]);
  const remainingOptions = optionEntries.filter(([k]) => !handledKeys.has(k));

  const currentBgMode = currentConfig["background_mode"] ?? bgModeSpec?.default ?? "time";
  const isStaticBgActive = currentBgMode === "static";

  return (
    <section className="rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] text-[var(--rz-text)] overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.03),0_8px_24px_rgba(0,0,0,0.02)] my-6">
      {/* ── Header: Clean & Apple-Like ── */}
      <div className="px-6 py-4 border-b border-[var(--rz-border-subtle)] flex items-center justify-between gap-4 flex-wrap">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-sm sm:text-base font-semibold text-[var(--rz-text)]">
              Theme Customization
            </h2>
            <span className="px-2 py-0.5 rounded text-[10px] font-medium bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]">
              Upstream ✓
            </span>
          </div>
          <p className="text-xs text-[var(--rz-text-secondary)] mt-0.5">
            Configure supported runtime parameters for {packageItem.title}.
          </p>
        </div>

        <button
          type="button"
          onClick={handleResetDefaults}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] hover:bg-[var(--rz-surface-hover)] transition-colors cursor-pointer select-none shadow-xs"
        >
          <RotateCcw size={12} />
          <span>Reset Defaults</span>
        </button>
      </div>

      <div className="p-6 space-y-5">
        {/* ── 1. Theme Variants (if present) ── */}
        {variants.length > 0 && (
          <div className="space-y-2.5">
            <span className="text-xs font-medium text-[var(--rz-text)] block">
              Theme Variant
            </span>

            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
              {variants.map((v) => {
                const isSelected = v.id === activeVariantId;
                const previewImg = v.preview_image || packageItem.preview_poster_url || packageItem.hero_image;

                return (
                  <button
                    key={v.id}
                    type="button"
                    onClick={() => onSelectVariant(v.id)}
                    className={`relative p-3 rounded-xl border text-left transition-all cursor-pointer flex gap-3 select-none ${
                      isSelected
                        ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] ring-1 ring-[var(--rz-accent)] shadow-xs"
                        : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)]"
                    }`}
                  >
                    {previewImg && (
                      <div className="w-12 h-12 rounded-lg overflow-hidden shrink-0 border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)]">
                        <img
                          src={previewImg}
                          alt={v.name}
                          className="w-full h-full object-cover"
                          loading="lazy"
                        />
                      </div>
                    )}

                    <div className="flex-1 min-w-0 pr-5">
                      <h4 className="text-xs font-semibold text-[var(--rz-text)] truncate">
                        {v.name}
                      </h4>
                      {v.description && (
                        <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 line-clamp-2 leading-relaxed">
                          {v.description}
                        </p>
                      )}
                    </div>

                    {isSelected && (
                      <div className="absolute top-3 right-3 w-4 h-4 rounded-full bg-[var(--rz-accent)] text-white flex items-center justify-center shadow-xs">
                        <Check size={10} strokeWidth={3} />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* ── 2. Settings Grid (Monochrome & Restrained) ── */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Option: themeMode (Apple Segmented Control) */}
          {themeModeSpec && (
            <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
              <div>
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  {themeModeSpec.label || "Theme Mode"}
                </span>
                <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                  Select light or dark contrast palette for interface elements.
                </p>
              </div>

              <div className="flex bg-[var(--rz-surface)] p-1 rounded-xl gap-1 border border-[var(--rz-border-subtle)]">
                {themeModeSpec.options?.map((opt) => {
                  const currentVal = currentConfig["themeMode"] ?? themeModeSpec.default;
                  const active = String(currentVal) === String(opt.value);

                  return (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => handleOptionChange("themeMode", opt.value)}
                      className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 px-3 text-xs font-medium rounded-lg transition-all cursor-pointer ${
                        active
                          ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text)] shadow-xs font-semibold"
                          : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                      }`}
                    >
                      {opt.value === "light" ? <Sun size={13} /> : <Moon size={13} />}
                      <span>{opt.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Option: enableWindup (Clean macOS Switch) */}
          {enableWindupSpec && (
            <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
              <div>
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  {enableWindupSpec.label || "Windup Animation"}
                </span>
                <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                  {enableWindupSpec.description || "Play mechanical gear train animation during unlock."}
                </p>
              </div>

              {(() => {
                const val = currentConfig["enableWindup"] !== undefined ? currentConfig["enableWindup"] : enableWindupSpec.default;
                const isChecked = Boolean(val);

                return (
                  <div className="flex items-center justify-between p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                    <span className="text-xs font-medium text-[var(--rz-text)]">
                      {isChecked ? "Enabled" : "Disabled"}
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={isChecked}
                      onClick={() => handleOptionChange("enableWindup", !isChecked)}
                      className={`w-10 h-6 rounded-full p-0.5 transition-colors cursor-pointer flex items-center ${
                        isChecked ? "bg-[var(--rz-accent)] justify-end" : "bg-[var(--rz-border-strong)] justify-start"
                      }`}
                    >
                      <span className="w-5 h-5 rounded-full bg-white shadow-xs" />
                    </button>
                  </div>
                );
              })()}
            </div>
          )}

          {/* Option: background_mode (Dynamic wallpaper mode) */}
          {bgModeSpec && (
            <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
              <div>
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  {bgModeSpec.label || "Background Mode"}
                </span>
                <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                  {bgModeSpec.description || "Controls how dynamic backgrounds transition."}
                </p>
              </div>

              <div className="flex bg-[var(--rz-surface)] p-1 rounded-xl gap-1 border border-[var(--rz-border-subtle)]">
                {bgModeSpec.options?.map((opt) => {
                  const currentVal = currentConfig["background_mode"] ?? bgModeSpec.default;
                  const active = String(currentVal) === String(opt.value);

                  return (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => handleOptionChange("background_mode", opt.value)}
                      className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 px-2 text-xs font-medium rounded-lg transition-all cursor-pointer ${
                        active
                          ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text)] shadow-xs font-semibold"
                          : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                      }`}
                    >
                      {opt.value === "time" ? <Clock size={12} /> : opt.value === "random" ? <Shuffle size={12} /> : <ImageIcon size={12} />}
                      <span>{opt.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Option: background_index (Static asset index) */}
          {bgIndexSpec && (
            <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
              <div>
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  {bgIndexSpec.label || "Static Background Selection"}
                </span>
                <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                  {bgIndexSpec.description || "Active wallpaper when static background mode is selected."}
                </p>
              </div>

              <div className="grid grid-cols-3 gap-1 p-1 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
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
                      className={`py-1.5 px-2 text-xs font-medium rounded-lg transition-all cursor-pointer text-center truncate ${
                        active
                          ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text)] shadow-xs font-semibold"
                          : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                      }`}
                    >
                      {opt.label}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Option: gameMode */}
          {gameModeSpec && (
            <div className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-3">
              <div>
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  {gameModeSpec.label || "Login Mode"}
                </span>
                <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                  {gameModeSpec.description || "Select rhythm minigame or direct password authentication."}
                </p>
              </div>

              <div className="flex bg-[var(--rz-surface)] p-1 rounded-xl gap-1 border border-[var(--rz-border-subtle)]">
                {gameModeSpec.options?.map((opt) => {
                  const currentVal = currentConfig["gameMode"] ?? gameModeSpec.default;
                  const active = String(currentVal) === String(opt.value);

                  return (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => handleOptionChange("gameMode", opt.value)}
                      className={`flex-1 flex items-center justify-center gap-1.5 py-1.5 px-3 text-xs font-medium rounded-lg transition-all cursor-pointer ${
                        active
                          ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-text)] shadow-xs font-semibold"
                          : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                      }`}
                    >
                      {opt.value === "game" ? <Gamepad2 size={13} /> : <KeyRound size={13} />}
                      <span>{opt.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Remaining Options */}
          {remainingOptions.map(([key, spec]) => {
            const val = currentConfig[key] !== undefined ? currentConfig[key] : spec.default;

            return (
              <div
                key={key}
                className="p-4 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] flex flex-col justify-between gap-2"
              >
                <div>
                  <label className="text-xs font-semibold text-[var(--rz-text)] block">
                    {spec.label}
                  </label>
                  {spec.description && (
                    <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
                      {spec.description}
                    </p>
                  )}
                </div>

                {spec.type === "select" && spec.options && (
                  <select
                    value={String(val)}
                    onChange={(e) => handleOptionChange(key, e.target.value)}
                    className="w-full text-xs font-medium px-3 py-1.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text)] focus:outline-none transition-colors cursor-pointer"
                  >
                    {spec.options.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>
                )}

                {spec.type === "boolean" && (
                  <div className="flex items-center justify-between p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                    <span className="text-xs font-medium text-[var(--rz-text)]">
                      {val ? "Enabled" : "Disabled"}
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(val)}
                      onClick={() => handleOptionChange(key, !val)}
                      className={`w-10 h-6 rounded-full p-0.5 transition-colors cursor-pointer flex items-center ${
                        val ? "bg-[var(--rz-accent)] justify-end" : "bg-[var(--rz-border-strong)] justify-start"
                      }`}
                    >
                      <span className="w-5 h-5 rounded-full bg-white shadow-xs" />
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {/* ── 3. Technical Materialization Details (Hidden Behind Disclosure) ── */}
        <div className="pt-2 border-t border-[var(--rz-border-subtle)]">
          <button
            type="button"
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center gap-1.5 text-xs text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors cursor-pointer select-none"
          >
            <span>{showAdvanced ? "Hide advanced details" : "Advanced details"}</span>
            <ChevronRight className={`w-3.5 h-3.5 transition-transform duration-150 ${showAdvanced ? "rotate-90" : ""}`} />
          </button>

          {showAdvanced && (
            <div className="mt-3 p-3.5 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-xs font-mono space-y-2 text-[var(--rz-text-secondary)] animate-in fade-in duration-100">
              <div className="text-[11px] font-sans font-semibold text-[var(--rz-text)]">
                Active Runtime Materialization
              </div>
              <p className="text-[11px] leading-relaxed font-sans text-[var(--rz-text-muted)]">
                Parameters write directly to <code>theme.conf</code> and <code>ryzora_config.json</code> in <code>~/.local/share/ryzora/lockscreens/</code> during application.
              </p>
              <div className="space-y-1 pt-1 border-t border-[var(--rz-border-subtle)] text-[10.5px]">
                {optionEntries.map(([k]) => (
                  <div key={k} className="flex items-center justify-between truncate">
                    <span className="font-semibold text-[var(--rz-text)]">{k} = {String(currentConfig[k] ?? options[k]?.default)}</span>
                    <span className="text-[var(--rz-text-muted)] text-[10px]">{getOptionEffect(k, currentConfig[k] ?? options[k]?.default, packageItem.id)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </section>
  );
};
