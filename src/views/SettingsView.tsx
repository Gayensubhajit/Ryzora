import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RyzoraSettings } from "../types";
import {
  Settings,
  RefreshCw,
  Bell,
  Shield,
  Eye,
  RotateCcw,
  Save,
  CheckCircle,
  AlertTriangle,
  Palette,
  Sun,
  Moon,
  Monitor,
  Sliders,
  Check,
} from "lucide-react";
import { useTheme, AppearanceMode, AccentColor, ACCENT_SWATCHES, ResolvedTheme } from "../theme";
import { detectSystemTheme, getCachedSystemTheme } from "../theme/appearance";

const DEFAULTS: RyzoraSettings = {
  default_release_channel: "stable",
  auto_refresh_enabled: true,
  auto_refresh_interval_minutes: 60,
  notification_level: "all",
  update_notification_policy: "notify",
  integrity_scan_on_startup: false,
  show_unverified_packages: true,
  show_nightly_packages: false,
  compact_ui: false,
};

export const SettingsView: React.FC = () => {
  const [settings, setSettings] = useState<RyzoraSettings>(DEFAULTS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [feedback, setFeedback] = useState<{ msg: string; ok: boolean } | null>(null);

  const {
    mode,
    accent,
    accessibility,
    setMode,
    setAccent,
    updateAccessibility,
  } = useTheme();

  const [systemScheme, setSystemScheme] = useState<ResolvedTheme>(() => getCachedSystemTheme());
  const [snapshotOnInstall, setSnapshotOnInstall] = useState<boolean>(() => {
    const pref = localStorage.getItem("ryzora_snapshot_on_install");
    return pref !== null ? pref === "true" : false;
  });

  const toggleSnapshotOnInstall = () => {
    const next = !snapshotOnInstall;
    setSnapshotOnInstall(next);
    localStorage.setItem("ryzora_snapshot_on_install", String(next));
  };


  useEffect(() => {
    detectSystemTheme().then(setSystemScheme).catch(() => {});
  }, []);

  useEffect(() => {
    invoke<RyzoraSettings>("get_settings")
      .then(setSettings)
      .catch(() => setSettings(DEFAULTS))
      .finally(() => setLoading(false));
  }, []);

  const patch = (updates: Partial<RyzoraSettings>) =>
    setSettings((s) => ({ ...s, ...updates }));

  const save = async () => {
    setSaving(true);
    setFeedback(null);
    try {
      await invoke("save_settings", { settings });
      setFeedback({ msg: "Settings saved.", ok: true });
    } catch (e: any) {
      setFeedback({ msg: String(e), ok: false });
    } finally {
      setSaving(false);
      setTimeout(() => setFeedback(null), 3500);
    }
  };

  const reset = async () => {
    setSaving(true);
    try {
      const defaults = await invoke<RyzoraSettings>("reset_settings");
      setSettings(defaults);
      setFeedback({ msg: "Reset to defaults.", ok: true });
    } catch (e: any) {
      setFeedback({ msg: String(e), ok: false });
    } finally {
      setSaving(false);
      setTimeout(() => setFeedback(null), 3500);
    }
  };

  if (loading) {
    return (
      <div className="view-container">
        <div className="loading-state">
          <Settings className="w-8 h-8 animate-spin" style={{ color: "var(--rz-accent)" }} />
          <p>Loading settings…</p>
        </div>
      </div>
    );
  }

  const appearanceCards: {
    id: AppearanceMode;
    label: string;
    description: string;
    icon: React.ReactNode;
  }[] = [
    {
      id: "system",
      label: "System",
      description: `Follows Linux desktop scheme (${systemScheme})`,
      icon: <Monitor className="w-4 h-4" />,
    },
    {
      id: "light",
      label: "Light",
      description: "Clean & luminous translucent Ryzora Glass",
      icon: <Sun className="w-4 h-4" />,
    },
    {
      id: "dark",
      label: "Dark",
      description: "Midnight glass & deep dark surfaces",
      icon: <Moon className="w-4 h-4" />,
    },
  ];

  const accents: AccentColor[] = ["blue", "purple", "green", "orange"];

  return (
    <div className="view-container">
      {/* Header */}
      <div className="view-header">
        <div>
          <h1 className="view-title">Settings</h1>
          <p className="view-subtitle">Preferences for Ryzora. Security invariants are enforced by the engine and cannot be overridden here.</p>
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <button className="btn btn-ghost" onClick={reset} disabled={saving}>
            <RotateCcw className="w-4 h-4" /> Defaults
          </button>
          <button className="btn btn-primary" onClick={save} disabled={saving}>
            <Save className="w-4 h-4" /> {saving ? "Saving…" : "Save"}
          </button>
        </div>
      </div>

      {feedback && (
        <div className={`settings-feedback ${feedback.ok ? "ok" : "err"}`}>
          {feedback.ok ? <CheckCircle className="w-4 h-4" /> : <AlertTriangle className="w-4 h-4" />}
          {feedback.msg}
        </div>
      )}

      <div className="settings-grid">
        {/* Appearance Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Palette className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Appearance</h2>
          </div>

          {/* Theme Selection Cards */}
          <div className="settings-field">
            <label>Theme Mode</label>
            <p className="settings-hint">Separated from content packages: Ryzora UI stays readable regardless of installed desktop themes.</p>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3 mt-1">
              {appearanceCards.map((card) => {
                const isSelected = mode === card.id;
                return (
                  <button
                    key={card.id}
                    type="button"
                    onClick={() => setMode(card.id)}
                    className={[
                      "flex flex-col p-3.5 rounded-xl border text-left transition-all relative",
                      isSelected
                        ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] ring-1 ring-[var(--rz-accent)] shadow-sm"
                        : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] hover:bg-[var(--rz-surface-hover)]",
                    ].join(" ")}
                  >
                    <div className="flex items-center justify-between w-full mb-2">
                      <div className="flex items-center gap-2">
                        <span className={isSelected ? "text-[var(--rz-accent)]" : "text-[var(--rz-text-muted)]"}>
                          {card.icon}
                        </span>
                        <span className="text-xs font-semibold text-[var(--rz-text)]">{card.label}</span>
                      </div>
                      {isSelected && (
                        <span className="w-4 h-4 rounded-full bg-[var(--rz-accent)] text-white flex items-center justify-center">
                          <Check className="w-2.5 h-2.5" />
                        </span>
                      )}
                    </div>
                    <span className="text-[11px] text-[var(--rz-text-muted)] leading-normal">{card.description}</span>
                  </button>
                );
              })}
            </div>
          </div>

          {/* Accent Color Selection */}
          <div className="settings-field mt-1">
            <label>Accent Color</label>
            <p className="settings-hint">Functional focus color for primary buttons, focus rings, links, and active navigation.</p>

            <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 mt-1">
              {accents.map((ac) => {
                const swatch = ACCENT_SWATCHES[ac];
                const isSelected = accent === ac;
                return (
                  <button
                    key={ac}
                    type="button"
                    onClick={() => setAccent(ac)}
                    className={[
                      "flex items-center gap-2.5 px-3 py-2 rounded-lg border text-xs font-medium transition-all",
                      isSelected
                        ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] shadow-sm text-[var(--rz-text)]"
                        : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] hover:border-[var(--rz-border-strong)] text-[var(--rz-text-secondary)]",
                    ].join(" ")}
                  >
                    <span className={`w-3.5 h-3.5 rounded-full ${swatch.bgClass} flex-shrink-0 flex items-center justify-center`}>
                      {isSelected && <Check className="w-2 h-2 text-white" />}
                    </span>
                    <span className="truncate">{swatch.name}</span>
                  </button>
                );
              })}
            </div>
          </div>
        </section>

        {/* Accessibility Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Sliders className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Accessibility</h2>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Reduce Transparency
                <span className="settings-hint">Replaces frosted translucent glass with solid high-contrast surfaces.</span>
              </span>
              <button
                className={`toggle ${accessibility.reduceTransparency ? "on" : "off"}`}
                onClick={() => updateAccessibility({ reduceTransparency: !accessibility.reduceTransparency })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Reduce Motion
                <span className="settings-hint">Disables fluid animations and transitions for instant layout response.</span>
              </span>
              <button
                className={`toggle ${accessibility.reduceMotion ? "on" : "off"}`}
                onClick={() => updateAccessibility({ reduceMotion: !accessibility.reduceMotion })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Increase Contrast
                <span className="settings-hint">Sharpens border outlines and strengthens typographic contrast across all surfaces.</span>
              </span>
              <button
                className={`toggle ${accessibility.increaseContrast ? "on" : "off"}`}
                onClick={() => updateAccessibility({ increaseContrast: !accessibility.increaseContrast })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>
        </section>

        {/* Repositories Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <RefreshCw className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Repositories</h2>
          </div>

          <div className="settings-field">
            <label>Default Release Channel</label>
            <p className="settings-hint">Applied to newly added repositories.</p>
            <div className="radio-group">
              {(["stable", "beta", "nightly"] as const).map((ch) => (
                <label key={ch} className={`radio-option ${settings.default_release_channel === ch ? "selected" : ""}`}>
                  <input
                    type="radio"
                    name="channel"
                    value={ch}
                    checked={settings.default_release_channel === ch}
                    onChange={() => patch({ default_release_channel: ch })}
                  />
                  <span className="radio-label">{ch.charAt(0).toUpperCase() + ch.slice(1)}</span>
                </label>
              ))}
            </div>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Automatic Repository Refresh
                <span className="settings-hint">Periodically refresh repository indexes.</span>
              </span>
              <button
                className={`toggle ${settings.auto_refresh_enabled ? "on" : "off"}`}
                onClick={() => patch({ auto_refresh_enabled: !settings.auto_refresh_enabled })}
                aria-pressed={settings.auto_refresh_enabled}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          {settings.auto_refresh_enabled && (
            <div className="settings-field">
              <label>Refresh Interval</label>
              <p className="settings-hint">Minutes between automatic refreshes (15–1440).</p>
              <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                <input
                  type="range"
                  min={15}
                  max={1440}
                  step={15}
                  value={settings.auto_refresh_interval_minutes}
                  onChange={(e) => patch({ auto_refresh_interval_minutes: Number(e.target.value) })}
                  style={{ flex: 1 }}
                />
                <span className="settings-value-badge">{settings.auto_refresh_interval_minutes} min</span>
              </div>
            </div>
          )}
        </section>

        {/* Display Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Eye className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Display</h2>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Compact UI
                <span className="settings-hint">Denser layout with smaller spacing.</span>
              </span>
              <button
                className={`toggle ${settings.compact_ui ? "on" : "off"}`}
                onClick={() => patch({ compact_ui: !settings.compact_ui })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Show Unverified Packages
                <span className="settings-hint">Show Community/Unvetted packages in Discover.</span>
              </span>
              <button
                className={`toggle ${settings.show_unverified_packages ? "on" : "off"}`}
                onClick={() => patch({ show_unverified_packages: !settings.show_unverified_packages })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Show Nightly Packages
                <span className="settings-hint">Show packages on the nightly release channel.</span>
              </span>
              <button
                className={`toggle ${settings.show_nightly_packages ? "on" : "off"}`}
                onClick={() => patch({ show_nightly_packages: !settings.show_nightly_packages })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>
        </section>

        {/* Notifications Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Bell className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Notifications</h2>
          </div>

          <div className="settings-field">
            <label>Notification Level</label>
            <p className="settings-hint">Security alerts are always shown regardless of this setting.</p>
            <div className="radio-group">
              {(["all", "security_only", "none"] as const).map((level) => (
                <label key={level} className={`radio-option ${settings.notification_level === level ? "selected" : ""}`}>
                  <input
                    type="radio"
                    name="notif-level"
                    value={level}
                    checked={settings.notification_level === level}
                    onChange={() => patch({ notification_level: level })}
                  />
                  <span className="radio-label">
                    {level === "all" ? "All" : level === "security_only" ? "Security only" : "None"}
                  </span>
                </label>
              ))}
            </div>
          </div>

          <div className="settings-field">
            <label>Update Notifications</label>
            <div className="radio-group">
              {(["notify", "silent"] as const).map((policy) => (
                <label key={policy} className={`radio-option ${settings.update_notification_policy === policy ? "selected" : ""}`}>
                  <input
                    type="radio"
                    name="update-policy"
                    value={policy}
                    checked={settings.update_notification_policy === policy}
                    onChange={() => patch({ update_notification_policy: policy })}
                  />
                  <span className="radio-label">
                    {policy === "notify" ? "Show in activity log" : "Silent"}
                  </span>
                </label>
              ))}
            </div>
          </div>
        </section>

        {/* Safety & Snapshots Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Shield className="w-4 h-4" style={{ color: "var(--rz-accent)" }} />
            <h2>Safety & Snapshots</h2>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Default Backup Snapshot on Install
                <span className="settings-hint">
                  Enable backup snapshots by default when installing packages. When disabled, packages install directly without taking a pre-install snapshot (you can still choose in the install modal).
                </span>
              </span>
              <button
                className={`toggle ${snapshotOnInstall ? "on" : "off"}`}
                onClick={toggleSnapshotOnInstall}
                type="button"
                aria-label="Toggle default backup snapshot on install"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-field">
            <label className="toggle-label">
              <span>
                Startup Integrity Scan
                <span className="settings-hint">Run a read-only integrity scan on application start. Off by default to avoid startup latency.</span>
              </span>
              <button
                className={`toggle ${settings.integrity_scan_on_startup ? "on" : "off"}`}
                onClick={() => patch({ integrity_scan_on_startup: !settings.integrity_scan_on_startup })}
                type="button"
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-note">
            <Shield className="w-3 h-3" />
            Signature verification, trust chain enforcement, and sandboxed staging cannot be bypassed. Pre-installation snapshot creation is user-configurable on demand.
          </div>
        </section>
      </div>
    </div>
  );
};
