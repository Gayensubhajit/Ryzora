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
  Save,
  CheckCircle2,
  Sparkles,
  Calendar,
  Clock as ClockIcon,
  MessageSquare,
} from "lucide-react";
import type {
  SilentSddmConfiguration,
  SilentSddmClockPosition,
  SilentSddmLoginAreaPosition,
  SilentSddmFillMode,
} from "../../types/index.ts";
import { SilentSddmService } from "../../services/silentSddmService.ts";

interface SilentSddmConfigPanelProps {
  initialTab?: "login" | "lock";
  onApplied?: (manifest: any) => void;
  onTestMode?: (target: "sddm", configDraft?: SilentSddmConfiguration) => void;
  onClose?: () => void;
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
  initialTab = "login",
  onApplied,
  onTestMode,
}) => {
  const [activeTab, setActiveTab] = useState<"login" | "lock">(initialTab);
  const [config, setConfig] = useState<SilentSddmConfiguration | null>(null);
  const [savedConfigSnapshot, setSavedConfigSnapshot] = useState<string>("");
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
          // Ensure safe defaults for nested objects if missing
          const safeCfg: SilentSddmConfiguration = {
            ...cfg,
            lock_screen: {
              ...cfg.lock_screen,
              clock: cfg.lock_screen?.clock || {
                display: true,
                position: "top-center",
                align: "center",
                format: "hh:mm",
                font_size: 70,
                font_weight: 900,
                color: "#FFFFFF",
              },
              date: cfg.lock_screen?.date || {
                display: true,
                format: "dddd, MMMM d",
                font_size: 18,
                color: "#FFFFFF",
                margin_top: 8,
              },
              message: cfg.lock_screen?.message || {
                display: true,
                position: "bottom-center",
                text: "Press any key or click to unlock",
                font_size: 13,
                color: "#FFFFFF",
              },
            },
            login_screen: {
              ...cfg.login_screen,
              login_area: cfg.login_screen?.login_area || {
                position: "center",
                margin: -1,
              },
              avatar: cfg.login_screen?.avatar || {
                shape: "circle",
                active_size: 120,
                inactive_size: 80,
                inactive_opacity: 0.35,
              },
            },
          };
          setConfig(safeCfg);
          setSavedConfigSnapshot(JSON.stringify(safeCfg));
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

  const hasUnsavedChanges = Boolean(
    config && savedConfigSnapshot && JSON.stringify(config) !== savedConfigSnapshot
  );

  const handleSaveDraft = async () => {
    if (!config) return;
    setError(null);
    setIsSaving(true);
    setSuccessMsg(null);
    try {
      await SilentSddmService.saveConfiguration(config);
      setSavedConfigSnapshot(JSON.stringify(config));
      setSuccessMsg("Draft configuration saved successfully.");
      setTimeout(() => setSuccessMsg(null), 3000);
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
      // First save configuration to disk
      await SilentSddmService.saveConfiguration(config);
      setSavedConfigSnapshot(JSON.stringify(config));
      // Then apply to SDDM system greeter
      const manifest = await SilentSddmService.applyConfiguration(config);
      setSuccessMsg("Configuration applied to SDDM successfully!");
      onApplied?.(manifest);
      setTimeout(() => setSuccessMsg(null), 3500);
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
    <div className="rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface-elevated)] p-5 sm:p-6 shadow-sm space-y-6">
      {/* ── Lifecycle Stepper Banner: Configure → Preview/Test → Apply ── */}
      <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-strong)] flex flex-col sm:flex-row items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <div className="w-7 h-7 rounded-lg bg-[var(--rz-accent)]/15 border border-[var(--rz-accent)]/30 flex items-center justify-center text-[var(--rz-accent)]">
            <Sliders className="w-3.5 h-3.5" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <span className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text)]">
                Configuration Lifecycle
              </span>
              {hasUnsavedChanges ? (
                <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-500/20 text-amber-400 border border-amber-500/30">
                  Unsaved Draft
                </span>
              ) : (
                <span className="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 flex items-center gap-1">
                  <CheckCircle2 className="w-2.5 h-2.5" />
                  Synced
                </span>
              )}
            </div>
            <p className="text-[11px] text-[var(--rz-text-muted)] mt-0.5">
              Configure parameters → Preview / Test in isolated sandbox → Apply to system greeter
            </p>
          </div>
        </div>

        {/* Stepper Actions */}
        <div className="flex items-center gap-2 w-full sm:w-auto justify-end">
          <button
            type="button"
            disabled={isSaving || isApplying || !hasUnsavedChanges}
            onClick={handleSaveDraft}
            className={`px-3 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer ${
              hasUnsavedChanges
                ? "bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-amber-500/40 text-amber-300 shadow-xs"
                : "bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] text-[var(--rz-text-muted)] opacity-60 cursor-not-allowed"
            }`}
            title="Save draft changes to configuration file"
          >
            {isSaving ? <Loader2 className="w-3 h-3 animate-spin" /> : <Save className="w-3 h-3" />}
            <span>Save Draft</span>
          </button>

          {onTestMode && (
            <button
              type="button"
              onClick={() => onTestMode("sddm", config)}
              className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] transition-all cursor-pointer flex items-center gap-1.5 shadow-xs"
              title="Launch sandboxed SDDM test window to preview layout before applying"
            >
              <Play className="w-3 h-3 text-[var(--rz-accent)] fill-[var(--rz-accent)]/20" />
              <span>Preview / Test</span>
            </button>
          )}

          <button
            type="button"
            disabled={isApplying}
            onClick={handleApply}
            className="px-3.5 py-1.5 rounded-lg text-xs font-semibold bg-[var(--rz-accent)] hover:bg-[var(--rz-accent-hover)] text-white transition-all cursor-pointer flex items-center gap-1.5 shadow-xs disabled:opacity-60"
            title="Apply configuration atomically to SDDM"
          >
            {isApplying ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Check className="w-3.5 h-3.5" />}
            <span>Apply</span>
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

      {/* ── Screen Target Tabs: Login Screen vs Lock Screen ── */}
      <div className="flex items-center justify-between border-b border-[var(--rz-border-subtle)] pb-3">
        <div className="flex items-center gap-1 p-1 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
          <button
            type="button"
            onClick={() => setActiveTab("login")}
            className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer ${
              activeTab === "login"
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] shadow-xs"
                : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
            }`}
          >
            <Monitor className="w-3.5 h-3.5" />
            <span>Login Screen</span>
          </button>

          <button
            type="button"
            onClick={() => setActiveTab("lock")}
            className={`px-3.5 py-1.5 rounded-lg text-xs font-semibold flex items-center gap-1.5 transition-all cursor-pointer ${
              activeTab === "lock"
                ? "bg-[var(--rz-surface-elevated)] text-[var(--rz-accent)] shadow-xs"
                : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)]"
            }`}
          >
            <Lock className="w-3.5 h-3.5" />
            <span>Lock Screen</span>
          </button>
        </div>

        <span className="text-[11px] text-[var(--rz-text-muted)] font-mono">
          Target: {activeTab === "login" ? "SilentSDDM Login Screen" : "SilentSDDM Lock Screen"}
        </span>
      </div>

      {/* ══════════════════════════════════════════════════════════════════ */}
      {/* ── TAB 1: LOGIN SCREEN ── */}
      {/* ══════════════════════════════════════════════════════════════════ */}
      {activeTab === "login" && (
        <div className="space-y-6 animate-in fade-in duration-150">
          {/* Section A: Background & Fill Mode */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Background & Fill Mode</span>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">
                  Active Background Key
                </span>
                <div className="px-3 py-2 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-xs font-mono text-[var(--rz-text)] truncate">
                  {config.login_screen.background || "default"}
                </div>
              </div>

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
                <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">
                  Display Fill Mode
                </span>
                <div className="grid grid-cols-3 gap-2">
                  {(["fill", "fit", "stretch"] as SilentSddmFillMode[]).map((mode) => (
                    <button
                      key={mode}
                      type="button"
                      onClick={() =>
                        setConfig({
                          ...config,
                          background: { ...config.background, fill_mode: mode },
                        })
                      }
                      className={`py-1.5 px-2 rounded-lg text-xs capitalize transition-all cursor-pointer text-center font-medium ${
                        config.background.fill_mode === mode
                          ? "bg-[var(--rz-accent)] text-white shadow-xs"
                          : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] border border-[var(--rz-border-subtle)]"
                      }`}
                    >
                      {mode}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Section B: Visual Filters (Blur, Brightness, Saturation) */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Sparkles className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Visual Filters</span>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
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

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Brightness</span>
                  <span className="font-mono text-[var(--rz-accent)]">
                    {config.login_screen.brightness.toFixed(1)}
                  </span>
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
                      login_screen: {
                        ...config.login_screen,
                        brightness: parseFloat(e.target.value),
                      },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Saturation</span>
                  <span className="font-mono text-[var(--rz-accent)]">
                    {config.login_screen.saturation.toFixed(1)}
                  </span>
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
                      login_screen: {
                        ...config.login_screen,
                        saturation: parseFloat(e.target.value),
                      },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>
            </div>
          </div>

          {/* Section C: Login Layout & Avatar */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Monitor className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Login Area Layout & Avatar</span>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* Login Area Alignment */}
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

              {/* Avatar Shape & Size */}
              <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
                <span className="text-xs font-semibold text-[var(--rz-text)] block">
                  Avatar Shape & Size
                </span>
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
                    className={`flex-1 py-1.5 rounded-lg text-xs transition-all cursor-pointer font-medium ${
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
                    className={`flex-1 py-1.5 rounded-lg text-xs transition-all cursor-pointer font-medium ${
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
                    <span className="font-mono text-[var(--rz-accent)]">
                      {config.login_screen.avatar.active_size}px
                    </span>
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
        </div>
      )}

      {/* ══════════════════════════════════════════════════════════════════ */}
      {/* ── TAB 2: LOCK SCREEN ── */}
      {/* ══════════════════════════════════════════════════════════════════ */}
      {activeTab === "lock" && (
        <div className="space-y-6 animate-in fade-in duration-150">
          {/* Section A: Background & Fill Mode */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Background & Fill Mode</span>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">
                  Active Background Key
                </span>
                <div className="px-3 py-2 rounded-lg bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] text-xs font-mono text-[var(--rz-text)] truncate">
                  {config.lock_screen.background || "default"}
                </div>
              </div>

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
                <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">
                  Display Fill Mode
                </span>
                <div className="grid grid-cols-3 gap-2">
                  {(["fill", "fit", "stretch"] as SilentSddmFillMode[]).map((mode) => (
                    <button
                      key={mode}
                      type="button"
                      onClick={() =>
                        setConfig({
                          ...config,
                          background: { ...config.background, fill_mode: mode },
                        })
                      }
                      className={`py-1.5 px-2 rounded-lg text-xs capitalize transition-all cursor-pointer text-center font-medium ${
                        config.background.fill_mode === mode
                          ? "bg-[var(--rz-accent)] text-white shadow-xs"
                          : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] border border-[var(--rz-border-subtle)]"
                      }`}
                    >
                      {mode}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          {/* Section B: Visual Filters (Blur, Brightness, Saturation) */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Sparkles className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
              <span>Visual Filters</span>
            </h4>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
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

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Brightness</span>
                  <span className="font-mono text-[var(--rz-accent)]">
                    {config.lock_screen.brightness.toFixed(1)}
                  </span>
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
                      lock_screen: {
                        ...config.lock_screen,
                        brightness: parseFloat(e.target.value),
                      },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>

              <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-1.5">
                <div className="flex items-center justify-between text-xs">
                  <span className="text-[var(--rz-text)] font-medium">Saturation</span>
                  <span className="font-mono text-[var(--rz-accent)]">
                    {config.lock_screen.saturation.toFixed(1)}
                  </span>
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
                      lock_screen: {
                        ...config.lock_screen,
                        saturation: parseFloat(e.target.value),
                      },
                    })
                  }
                  className="w-full accent-[var(--rz-accent)] cursor-pointer"
                />
              </div>
            </div>
          </div>

          {/* Section C: Clock, Date, Message */}
          <div className="space-y-4">
            {/* Clock */}
            <div className="space-y-3">
              <h4 className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center justify-between">
                <div className="flex items-center gap-1.5">
                  <ClockIcon className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                  <span>Clock Settings</span>
                </div>
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
                  <span className="text-xs font-medium text-[var(--rz-text-secondary)] block">
                    Position Grid:
                  </span>
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

                {/* Clock Size & Alignment */}
                <div className="p-3.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
                  <div className="space-y-1.5">
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-[var(--rz-text)] font-medium">Clock Font Size</span>
                      <span className="font-mono text-[var(--rz-accent)]">
                        {config.lock_screen.clock.font_size}px
                      </span>
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
                </div>
              </div>
            </div>

            {/* Date & Unlock Message */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {/* Date */}
              <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-[var(--rz-text)] flex items-center gap-1.5">
                    <Calendar className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                    <span>Date Format</span>
                  </span>
                  <label className="flex items-center gap-1.5 cursor-pointer text-xs font-normal normal-case text-[var(--rz-text)]">
                    <input
                      type="checkbox"
                      checked={config.lock_screen.date?.display ?? true}
                      onChange={(e) =>
                        setConfig({
                          ...config,
                          lock_screen: {
                            ...config.lock_screen,
                            date: { ...config.lock_screen.date, display: e.target.checked },
                          },
                        })
                      }
                      className="rounded accent-[var(--rz-accent)]"
                    />
                    <span>Display Date</span>
                  </label>
                </div>

                <div className="space-y-1.5">
                  <label className="text-[11px] text-[var(--rz-text-secondary)] block">
                    Date Format String:
                  </label>
                  <input
                    type="text"
                    value={config.lock_screen.date?.format || "dddd, MMMM d"}
                    onChange={(e) =>
                      setConfig({
                        ...config,
                        lock_screen: {
                          ...config.lock_screen,
                          date: { ...config.lock_screen.date, format: e.target.value },
                        },
                      })
                    }
                    className="w-full px-3 py-1.5 rounded-lg text-xs bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-strong)] text-[var(--rz-text)] focus:outline-none focus:border-[var(--rz-accent)] font-mono"
                  />
                </div>
              </div>

              {/* Message */}
              <div className="p-4 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-3">
                <span className="text-xs font-semibold text-[var(--rz-text)] flex items-center gap-1.5">
                  <MessageSquare className="w-3.5 h-3.5 text-[var(--rz-accent)]" />
                  <span>Unlock Prompt Message</span>
                </span>

                <div className="space-y-1.5">
                  <label className="text-[11px] text-[var(--rz-text-secondary)] block">
                    Unlock Message Text:
                  </label>
                  <input
                    type="text"
                    value={config.lock_screen.message?.text || "Press any key to unlock"}
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
    </div>
  );
};
