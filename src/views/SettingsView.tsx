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
} from "lucide-react";

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
          <Settings className="w-8 h-8 animate-spin" style={{ color: "var(--accent)" }} />
          <p>Loading settings…</p>
        </div>
      </div>
    );
  }

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
        {/* Repository Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <RefreshCw className="w-4 h-4" style={{ color: "var(--accent)" }} />
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
            <Eye className="w-4 h-4" style={{ color: "var(--accent)" }} />
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
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>
        </section>

        {/* Notifications Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Bell className="w-4 h-4" style={{ color: "var(--accent)" }} />
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

        {/* Integrity Section */}
        <section className="settings-section">
          <div className="settings-section-header">
            <Shield className="w-4 h-4" style={{ color: "var(--accent)" }} />
            <h2>Integrity</h2>
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
              >
                <span className="toggle-thumb" />
              </button>
            </label>
          </div>

          <div className="settings-note">
            <Shield className="w-3 h-3" />
            Signature verification, trust chain enforcement, and snapshot creation cannot be disabled via settings. These are engine-level invariants.
          </div>
        </section>
      </div>
    </div>
  );
};
