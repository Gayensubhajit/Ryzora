import React, { useState, useEffect } from "react";
import {
  Sliders,
  Check,
  Play,
  Loader2,
  AlertCircle,
  Monitor,
  Lock,
  Layers,
} from "lucide-react";
import type {
  SilentSddmConfiguration,
  SilentSddmClockPosition,
  SilentSddmLoginAreaPosition,
  SilentSddmFillMode,
} from "../../types/index.ts";
import { SilentSddmService } from "../../services/silentSddmService.ts";

interface SilentSddmConfigPanelProps {
  onApplied?: (manifest: any) => void;
  onTestMode?: (target: "sddm") => void;
}

const CLOCK_POSITIONS: { id: SilentSddmClockPosition; label: string; icon: string }[] = [
  { id: "top-left", label: "Top-Left", icon: "↖" },
  { id: "top-center", label: "Top-Center", icon: "↑" },
  { id: "top-right", label: "Top-Right", icon: "↗" },
  { id: "center-left", label: "Center-Left", icon: "←" },
  { id: "center", label: "Center", icon: "•" },
  { id: "center-right", label: "Center-Right", icon: "→" },
  { id: "bottom-left", label: "Bottom-Left", icon: "↙" },
  { id: "bottom-center", label: "Bottom-Center", icon: "↓" },
  { id: "bottom-right", label: "Bottom-Right", icon: "↘" },
];

export const SilentSddmConfigPanel: React.FC<SilentSddmConfigPanelProps> = ({
  onApplied,
  onTestMode,
}) => {
  const [activeTab, setActiveTab] = useState<"lock" | "login" | "global">("lock");
  const [config, setConfig] = useState<SilentSddmConfiguration | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    SilentSddmService.getConfiguration()
      .then((cfg) => {
        if (mounted) {
          setConfig(cfg);
          setIsLoading(false);
        }
      })
      .catch((err) => {
        if (mounted) {
          setError(String(err));
          setIsLoading(false);
        }
      });
    return () => {
      mounted = false;
    };
  }, []);

  const handleSave = async () => {
    if (!config) return;
    setError(null);
    setIsSaving(true);
    setSuccessMsg(null);
    try {
      await SilentSddmService.saveConfiguration(config);
      setSuccessMsg("Configuration saved.");
      setTimeout(() => setSuccessMsg(null), 2500);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const handleApply = async () => {
    if (!config) return;
    setError(null);
    setIsApplying(true);
    setSuccessMsg(null);
    try {
      const manifest = await SilentSddmService.applyConfiguration(config);
      setSuccessMsg("Configuration applied to SDDM.");
      onApplied?.(manifest);
      setTimeout(() => setSuccessMsg(null), 3000);
    } catch (err: any) {
      setError(err?.message || String(err));
    } finally {
      setIsApplying(false);
    }
  };

  if (isLoading) {
    return (
      <div className="p-8 flex flex-col items-center justify-center rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] text-xs text-[var(--rz-text-muted)] gap-2">
        <Loader2 className="w-5 h-5 animate-spin text-[var(--rz-accent)]" />
        <span>Loading SilentSDDM configuration…</span>
      </div>
    );
  }

  if (!config) {
    return null;
  }

  return (
    <div className="rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] p-6 shadow-sm space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-base font-semibold text-[var(--rz-text)] flex items-center gap-2">
            <Sliders className="w-4 h-4 text-[var(--rz-accent)]" />
            <span>SilentSDDM Layout & Visual Customization</span>
          </h3>
          <p className="text-xs text-[var(--rz-text-secondary)] mt-0.5">
            Configure background scaling, filters, lock screen clock positioning, and login area layout.
          </p>
        </div>

        {/* Tab Switcher */}
        <div className="flex items-center p-1 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
          <button
            type="button"
            onClick={() => setActiveTab("lock")}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all cursor-pointer ${
              activeTab === "lock"
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] font-semibold shadow-xs"
                : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
            }`}
          >
            <Lock className="w-3.5 h-3.5" />
            <span>Lock Screen</span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("login")}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all cursor-pointer ${
              activeTab === "login"
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] font-semibold shadow-xs"
                : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
            }`}
          >
            <Monitor className="w-3.5 h-3.5" />
            <span>Login Screen</span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("global")}
            className={`px-3 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all cursor-pointer ${
              activeTab === "global"
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] font-semibold shadow-xs"
                : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
            }`}
          >
            <Layers className="w-3.5 h-3.5" />
            <span>Global Fill</span>
          </button>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/10 p-3 text-xs text-rose-400 flex items-start gap-2">
          <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
          <span>{error}</span>
        </div>
      )}

      {successMsg && (
        <div className="rounded-xl border border-emerald-500/20 bg-emerald-500/10 p-3 text-xs text-emerald-400 flex items-center gap-2">
          <Check className="w-4 h-4 shrink-0" />
          <span>{successMsg}</span>
        </div>
      )}

      {/* TAB 1: LOCK SCREEN */}
      {activeTab === "lock" && (
        <div className="space-y-6">
          {/* Visual Effects */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)]">
              Background Filters
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Blur</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.lock_screen.blur}</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={config.lock_screen.blur}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      lock_screen: { ...config.lock_screen, blur: parseInt(e.target.value, 10) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Brightness</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.lock_screen.brightness.toFixed(1)}</span>
                </div>
                <input
                  type="range"
                  min="-1.0"
                  max="1.0"
                  step="0.1"
                  value={config.lock_screen.brightness}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      lock_screen: { ...config.lock_screen, brightness: parseFloat(e.target.value) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Saturation</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.lock_screen.saturation.toFixed(1)}</span>
                </div>
                <input
                  type="range"
                  min="-1.0"
                  max="1.0"
                  step="0.1"
                  value={config.lock_screen.saturation}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      lock_screen: { ...config.lock_screen, saturation: parseFloat(e.target.value) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>
            </div>
          </div>

          {/* Clock Layout (9-grid) */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center justify-between">
              <span>Clock Layout</span>
              <label className="flex items-center gap-1.5 cursor-pointer text-xs font-normal normal-case text-[var(--rz-text)]">
                <input
                  type="checkbox"
                  checked={config.lock_screen.clock.display}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      lock_screen: {
                        ...config.lock_screen,
                        clock: { ...config.lock_screen.clock, display: e.target.checked },
                      },
                    })
                  }
                  className="rounded accent-[var(--rz-accent)]"
                />
                <span>Display Clock</span>
              </label>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 items-start">
              {/* 3x3 Position Grid */}
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
                <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">Position:</span>
                <div className="grid grid-cols-3 gap-1.5">
                  {CLOCK_POSITIONS.map((pos) => {
                    const isSelected = config.lock_screen.clock.position === pos.id;
                    return (
                      <button
                        key={pos.id}
                        type="button"
                        onClick={() =>
                          setConfig({
                            ...config,
                            lock_screen: {
                              ...config.lock_screen,
                              clock: { ...config.lock_screen.clock, position: pos.id },
                            },
                          })
                        }
                        className={`py-2 px-1 rounded-lg text-xs flex flex-col items-center justify-center transition-all cursor-pointer ${
                          isSelected
                            ? "bg-[var(--rz-accent)] text-white font-semibold shadow-xs"
                            : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] border border-[var(--rz-border-subtle)]"
                        }`}
                      >
                        <span className="text-sm font-bold">{pos.icon}</span>
                        <span className="text-[10px] mt-0.5">{pos.label}</span>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Clock Details */}
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-[var(--rz-text)] font-medium">Clock Font Size</span>
                    <span className="font-mono text-[var(--rz-accent)]">{config.lock_screen.clock.font_size}px</span>
                  </div>
                  <input
                    type="range"
                    min="30"
                    max="120"
                    value={config.lock_screen.clock.font_size}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        lock_screen: {
                          ...config.lock_screen,
                          clock: {
                            ...config.lock_screen.clock,
                            font_size: parseInt(e.target.value, 10),
                          },
                        },
                      })
                    }
                    className="w-full accent-[var(--rz-accent)] cursor-pointer"
                  />
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs text-[var(--rz-text-secondary)] block">Unlock Message Text:</label>
                  <input
                    type="text"
                    value={config.lock_screen.message.text}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        lock_screen: {
                          ...config.lock_screen,
                          message: { ...config.lock_screen.message, text: e.target.value },
                        },
                      })
                    }
                    className="w-full px-3 py-1.5 rounded-lg text-xs bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] focus:outline-none focus:border-[var(--rz-accent)]"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 2: LOGIN SCREEN */}
      {activeTab === "login" && (
        <div className="space-y-6">
          {/* Visual Effects */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)]">
              Login Screen Background Filters
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Blur</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.login_screen.blur}</span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={config.login_screen.blur}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      login_screen: { ...config.login_screen, blur: parseInt(e.target.value, 10) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Brightness</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.login_screen.brightness.toFixed(1)}</span>
                </div>
                <input
                  type="range"
                  min="-1.0"
                  max="1.0"
                  step="0.1"
                  value={config.login_screen.brightness}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      login_screen: { ...config.login_screen, brightness: parseFloat(e.target.value) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Saturation</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.login_screen.saturation.toFixed(1)}</span>
                </div>
                <input
                  type="range"
                  min="-1.0"
                  max="1.0"
                  step="0.1"
                  value={config.login_screen.saturation}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      login_screen: { ...config.login_screen, saturation: parseFloat(e.target.value) },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>
            </div>
          </div>

          {/* Login Area Alignment & Avatar */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
              <span className="text-xs font-semibold text-[var(--rz-text)] block">
                Login Form Alignment
              </span>
              <div className="grid grid-cols-3 gap-2">
                {(["left", "center", "right"] as SilentSddmLoginAreaPosition[]).map((pos) => (
                  <button
                    key={pos}
                    type="button"
                    onClick={() =>
                      setConfig({
                        ...config,
                        login_screen: {
                          ...config.login_screen,
                          login_area: { ...config.login_screen.login_area, position: pos },
                        },
                      })
                    }
                    className={`py-2 px-3 rounded-xl text-xs capitalize transition-all cursor-pointer ${
                      config.login_screen.login_area.position === pos
                        ? "bg-[var(--rz-accent)] text-white font-semibold shadow-xs"
                        : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] border border-[var(--rz-border-subtle)]"
                    }`}
                  >
                    {pos}
                  </button>
                ))}
              </div>
            </div>

            <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
              <span className="text-xs font-semibold text-[var(--rz-text)] block">Avatar Shape & Size</span>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() =>
                    setConfig({
                      ...config,
                      login_screen: {
                        ...config.login_screen,
                        avatar: { ...config.login_screen.avatar, shape: "circle" },
                      },
                    })
                  }
                  className={`flex-1 py-1.5 rounded-lg text-xs transition-all cursor-pointer ${
                    config.login_screen.avatar.shape === "circle"
                      ? "bg-[var(--rz-accent)] text-white font-semibold"
                      : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]"
                  }`}
                >
                  Circle
                </button>
                <button
                  type="button"
                  onClick={() =>
                    setConfig({
                      ...config,
                      login_screen: {
                        ...config.login_screen,
                        avatar: { ...config.login_screen.avatar, shape: "square" },
                      },
                    })
                  }
                  className={`flex-1 py-1.5 rounded-lg text-xs transition-all cursor-pointer ${
                    config.login_screen.avatar.shape === "square"
                      ? "bg-[var(--rz-accent)] text-white font-semibold"
                      : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]"
                  }`}
                >
                  Square
                </button>
              </div>

              <div className="space-y-1">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text-secondary)]">Size:</span>
                  <span className="font-mono text-[var(--rz-accent)]">{config.login_screen.avatar.active_size}px</span>
                </div>
                <input
                  type="range"
                  min="60"
                  max="200"
                  value={config.login_screen.avatar.active_size}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      login_screen: {
                        ...config.login_screen,
                        avatar: {
                          ...config.login_screen.avatar,
                          active_size: parseInt(e.target.value, 10),
                        },
                      },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>
            </div>
          </div>
        </div>
      )}

      {/* TAB 3: GLOBAL FILL */}
      {activeTab === "global" && (
        <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
          <div>
            <span className="text-xs font-semibold text-[var(--rz-text)] block">
              Background Fill Mode
            </span>
            <p className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5">
              Defines how video and image backgrounds fit your display aspect ratio.
            </p>
          </div>

          <div className="grid grid-cols-3 gap-3">
            {[
              { id: "fill", label: "Fill (Aspect Crop)", desc: "Crops to completely fill the screen without borders" },
              { id: "fit", label: "Fit (Aspect Fit)", desc: "Preserves the entire media, adding letterbox bars if needed" },
              { id: "stretch", label: "Stretch", desc: "Stretches to fill entire display, altering aspect ratio" },
            ].map((mode) => (
              <button
                key={mode.id}
                type="button"
                onClick={() =>
                  setConfig({
                    ...config,
                    background: { ...config.background, fill_mode: mode.id as SilentSddmFillMode },
                  })
                }
                className={`p-3 rounded-xl text-left border transition-all cursor-pointer ${
                  config.background.fill_mode === mode.id
                    ? "bg-[var(--rz-accent)]/15 border-[var(--rz-accent)] text-[var(--rz-text)] shadow-xs"
                    : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
                }`}
              >
                <span className="text-xs font-semibold block">{mode.label}</span>
                <span className="text-[10px] text-[var(--rz-text-muted)] mt-1 block">{mode.desc}</span>
              </button>
            ))}
          </div>
        </div>
      )}

      {/* Footer Controls */}
      <div className="flex items-center justify-between pt-2 border-t border-[var(--rz-border-subtle)]">
        <button
          type="button"
          disabled={isSaving || isApplying}
          onClick={handleSave}
          className="py-2.5 px-4 rounded-xl text-xs font-medium bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-colors cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
        >
          {isSaving ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Check className="w-3.5 h-3.5" />}
          <span>Save Settings</span>
        </button>

        <div className="flex items-center gap-2">
          {onTestMode && (
            <button
              type="button"
              onClick={() => onTestMode("sddm")}
              className="py-2.5 px-4 rounded-xl text-xs font-medium bg-[var(--rz-surface)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-colors cursor-pointer select-none flex items-center gap-1.5 shadow-xs"
            >
              <Play className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Test SDDM</span>
            </button>
          )}

          <button
            type="button"
            disabled={isApplying}
            onClick={handleApply}
            className="py-2.5 px-5 rounded-xl text-xs font-semibold bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white transition-all cursor-pointer select-none flex items-center gap-2 shadow-xs"
          >
            {isApplying ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Applying Configuration…</span>
              </>
            ) : (
              <>
                <Check className="w-4 h-4" />
                <span>Apply to SDDM</span>
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
};
