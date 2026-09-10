import React from "react";
import { Sliders, Check, RotateCcw, Sparkles, Clock, Palette, Eye } from "lucide-react";
import type {
  PackageItem,
  LockscreenConfigSchema,
} from "../../types/index.ts";

export interface LockScreenCustomizerProps {
  packageItem: PackageItem;
  configSchema: LockscreenConfigSchema;
  currentConfig: Record<string, any>;
  selectedVariantId?: string;
  onChangeConfig: (newConfig: Record<string, any>) => void;
  onSelectVariant: (variantId: string) => void;
  onResetDefaults?: () => void;
}

export const LockScreenCustomizer: React.FC<LockScreenCustomizerProps> = ({
  packageItem,
  configSchema,
  currentConfig,
  selectedVariantId,
  onChangeConfig,
  onSelectVariant,
  onResetDefaults,
}) => {
  const variants = configSchema.variants || [];
  const options = configSchema.options || {};
  const optionEntries = Object.entries(options);

  if (variants.length === 0 && optionEntries.length === 0) {
    return null;
  }

  const handleOptionChange = (key: string, value: any) => {
    onChangeConfig({
      ...currentConfig,
      [key]: value,
    });
  };

  const handleReset = () => {
    if (onResetDefaults) {
      onResetDefaults();
      return;
    }
    const defaults: Record<string, any> = {};
    for (const [key, spec] of optionEntries) {
      defaults[key] = spec.default;
    }
    onChangeConfig(defaults);
    if (variants.length > 0) {
      onSelectVariant(variants[0].id);
    }
  };

  const activeVariant = variants.find((v) => v.id === selectedVariantId) || variants[0];

  return (
    <section className="mt-8 rounded-2xl border border-[var(--rz-border-subtle,#27272a)] bg-[var(--rz-surface,#18181b)]/60 backdrop-blur-md overflow-hidden shadow-sm">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-3 px-5 py-4 border-b border-[var(--rz-border-subtle,#27272a)] bg-gradient-to-r from-purple-500/5 via-cyan-500/5 to-transparent">
        <div className="flex items-center gap-2.5">
          <div className="p-2 rounded-xl bg-purple-500/10 text-purple-400 border border-purple-500/20">
            <Sliders size={18} />
          </div>
          <div>
            <h2 className="text-sm font-semibold tracking-tight text-[var(--rz-text,#f4f4f5)] flex items-center gap-2">
              <span>Theme Customization</span>
              <span className="text-[10px] uppercase font-mono px-2 py-0.5 rounded-full bg-purple-500/20 text-purple-300 font-bold border border-purple-500/30">
                Interactive
              </span>
            </h2>
            <p className="text-xs text-[var(--rz-text-secondary,#a1a1aa)] mt-0.5">
              Select visual styles, clock formats, and interface widgets for {packageItem.title}.
            </p>
          </div>
        </div>

        <button
          type="button"
          onClick={handleReset}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium text-[var(--rz-text-secondary,#a1a1aa)] hover:text-[var(--rz-text,#f4f4f5)] bg-[var(--rz-surface-elevated,#27272a)] hover:bg-[var(--rz-surface-elevated,#27272a)]/80 border border-[var(--rz-border-subtle,#3f3f46)] transition-colors cursor-pointer"
          title="Reset to default options"
        >
          <RotateCcw size={13} />
          <span>Reset Defaults</span>
        </button>
      </div>

      <div className="p-5 space-y-6">
        {/* ── 1. Variant Selector ── */}
        {variants.length > 0 && (
          <div>
            <label className="block text-xs font-semibold uppercase tracking-wider text-[var(--rz-text-secondary,#a1a1aa)] mb-2.5 flex items-center gap-1.5">
              <Palette size={13} className="text-purple-400" />
              <span>Visual Variant</span>
            </label>
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2.5">
              {variants.map((v) => {
                const isSelected = v.id === selectedVariantId || (!selectedVariantId && v === variants[0]);
                return (
                  <button
                    key={v.id}
                    type="button"
                    onClick={() => onSelectVariant(v.id)}
                    className={`text-left p-3 rounded-xl border transition-all cursor-pointer relative overflow-hidden ${
                      isSelected
                        ? "bg-purple-950/30 border-purple-500/60 ring-1 ring-purple-500/40 shadow-sm"
                        : "bg-[var(--rz-surface-elevated,#202024)] border-[var(--rz-border-subtle,#2e2e33)] hover:border-[var(--rz-border,#3e3e44)]"
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <span className={`text-xs font-semibold ${isSelected ? "text-purple-200" : "text-[var(--rz-text,#f4f4f5)]"}`}>
                        {v.name}
                      </span>
                      {isSelected && (
                        <div className="p-0.5 rounded-full bg-purple-500 text-black shrink-0">
                          <Check size={11} strokeWidth={3} />
                        </div>
                      )}
                    </div>
                    {v.description && (
                      <p className="text-[11px] text-[var(--rz-text-muted,#71717a)] mt-1 line-clamp-2 leading-relaxed">
                        {v.description}
                      </p>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* ── 2. Component Options ── */}
        {optionEntries.length > 0 && (
          <div className="space-y-4 pt-2 border-t border-[var(--rz-border-subtle,#27272a)]">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-[var(--rz-text-secondary,#a1a1aa)] flex items-center gap-1.5">
              <Clock size={13} className="text-cyan-400" />
              <span>Component Options</span>
            </h3>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {optionEntries.map(([key, spec]) => {
                const val = currentConfig[key] !== undefined ? currentConfig[key] : spec.default;

                return (
                  <div
                    key={key}
                    className="p-3.5 rounded-xl border border-[var(--rz-border-subtle,#27272a)] bg-[var(--rz-surface-elevated,#1e1e22)]/50 flex flex-col justify-between gap-2.5"
                  >
                    <div>
                      <div className="flex items-center justify-between gap-2">
                        <label className="text-xs font-medium text-[var(--rz-text,#f4f4f5)]">
                          {spec.label}
                        </label>
                        {spec.type === "boolean" && (
                          <button
                            type="button"
                            role="switch"
                            aria-checked={Boolean(val)}
                            onClick={() => handleOptionChange(key, !val)}
                            className={`w-9 h-5 rounded-full p-0.5 transition-colors cursor-pointer flex items-center ${
                              Boolean(val) ? "bg-emerald-500 justify-end" : "bg-zinc-700 justify-start"
                            }`}
                          >
                            <span className="w-4 h-4 rounded-full bg-white shadow-xs" />
                          </button>
                        )}
                      </div>
                      {spec.description && (
                        <p className="text-[11px] text-[var(--rz-text-muted,#71717a)] mt-0.5 leading-normal">
                          {spec.description}
                        </p>
                      )}
                    </div>

                    {/* Select / Segmented Control */}
                    {spec.type === "select" && spec.options && (
                      <div className="mt-1">
                        {spec.options.length <= 3 ? (
                          /* 2-3 choices: Sleek Segmented Pill Switch */
                          <div className="grid grid-cols-2 sm:grid-cols-3 gap-1 p-1 rounded-lg bg-black/40 border border-white/5">
                            {spec.options.map((opt) => {
                              const active = String(val) === String(opt.value);
                              return (
                                <button
                                  key={opt.value}
                                  type="button"
                                  onClick={() => handleOptionChange(key, opt.value)}
                                  className={`px-2 py-1 text-xs font-medium rounded-md transition-all cursor-pointer truncate ${
                                    active
                                      ? "bg-purple-600 text-white shadow-xs font-semibold"
                                      : "text-zinc-400 hover:text-zinc-200"
                                  }`}
                                >
                                  {opt.label}
                                </button>
                              );
                            })}
                          </div>
                        ) : (
                          /* 4+ choices: Modern Custom Select */
                          <select
                            value={String(val)}
                            onChange={(e) => handleOptionChange(key, e.target.value)}
                            className="w-full text-xs font-medium px-3 py-1.5 rounded-lg bg-black/50 border border-[var(--rz-border-subtle,#3f3f46)] text-[var(--rz-text,#f4f4f5)] focus:border-purple-500 focus:outline-none transition-colors cursor-pointer"
                          >
                            {spec.options.map((opt) => (
                              <option key={opt.value} value={opt.value} className="bg-zinc-900 text-zinc-100">
                                {opt.label}
                              </option>
                            ))}
                          </select>
                        )}
                      </div>
                    )}

                    {/* Numeric Range Slider */}
                    {spec.type === "number" && (
                      <div className="mt-1 flex items-center gap-3">
                        <input
                          type="range"
                          min={spec.min ?? 0}
                          max={spec.max ?? 100}
                          step={spec.step ?? 1}
                          value={Number(val)}
                          onChange={(e) => handleOptionChange(key, Number(e.target.value))}
                          className="flex-1 accent-purple-500 cursor-pointer"
                        />
                        <span className="text-xs font-mono font-semibold px-2 py-0.5 rounded bg-black/40 border border-white/10 text-purple-300">
                          {val} {spec.unit || ""}
                        </span>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* ── 3. Live Configuration Summary Readout ── */}
        <div className="p-3.5 rounded-xl bg-purple-950/20 border border-purple-500/20 flex flex-wrap items-center justify-between gap-3 text-xs">
          <div className="flex items-center gap-2 text-purple-300">
            <Eye size={14} />
            <span className="font-semibold">Target Setup:</span>
            <span className="font-mono text-zinc-300">
              {activeVariant ? activeVariant.name : "Default"}
            </span>
            {optionEntries.length > 0 && (
              <span className="text-[11px] text-zinc-400">
                ({optionEntries.length} option{optionEntries.length > 1 ? "s" : ""} configured)
              </span>
            )}
          </div>
          <div className="flex items-center gap-1.5 text-[11px] text-emerald-400 font-medium">
            <Sparkles size={12} />
            <span>Config will be materialized on Apply</span>
          </div>
        </div>
      </div>
    </section>
  );
};
