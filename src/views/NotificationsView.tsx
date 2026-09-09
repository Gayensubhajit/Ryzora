import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ActivityEntry } from "../types";
import {
  Bell,
  CheckCheck,
  Trash2,
  Shield,
  Package,
  Server,
  RotateCcw,
  AlertTriangle,
  Info,
  XCircle,
  Filter,
} from "lucide-react";

const SEVERITY_ICON: Record<string, React.ReactNode> = {
  info: <Info className="w-4 h-4" style={{ color: "var(--accent)" }} />,
  warning: <AlertTriangle className="w-4 h-4" style={{ color: "#e8a53a" }} />,
  error: <XCircle className="w-4 h-4" style={{ color: "#e05c5c" }} />,
  critical: <Shield className="w-4 h-4" style={{ color: "#e05c5c" }} />,
};

const CATEGORY_ICON: Record<string, React.ReactNode> = {
  package_installed: <Package className="w-3 h-3" />,
  package_updated: <Package className="w-3 h-3" />,
  package_uninstalled: <Package className="w-3 h-3" />,
  repository_synced: <Server className="w-3 h-3" />,
  repository_sync_failed: <Server className="w-3 h-3" />,
  repository_recovered: <Server className="w-3 h-3" />,
  update_available: <RotateCcw className="w-3 h-3" />,
  integrity_failure: <Shield className="w-3 h-3" />,
  signature_invalid: <Shield className="w-3 h-3" />,
  key_revoked: <Shield className="w-3 h-3" />,
  rollback_occurred: <RotateCcw className="w-3 h-3" />,
  integrity_scan_completed: <Shield className="w-3 h-3" />,
};

const FILTER_OPTIONS = [
  { value: "", label: "All" },
  { value: "integrity", label: "Security" },
  { value: "package", label: "Packages" },
  { value: "repository", label: "Repositories" },
  { value: "update", label: "Updates" },
];

export const NotificationsView: React.FC = () => {
  const [entries, setEntries] = useState<ActivityEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState("");
  const [clearing, setClearing] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<ActivityEntry[]>("list_notifications", {
        limit: 200,
        filter: filter || null,
      });
      setEntries(result);
    } catch {
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => { load(); }, [load]);

  const markAllRead = async () => {
    await invoke("mark_all_notifications_read").catch(() => {});
    setEntries((e) => e.map((n) => ({ ...n, read: true })));
  };

  const markRead = async (id: string) => {
    await invoke("mark_notification_read", { id }).catch(() => {});
    setEntries((e) => e.map((n) => (n.id === id ? { ...n, read: true } : n)));
  };

  const clearOld = async () => {
    setClearing(true);
    try {
      const removed = await invoke<number>("clear_old_notifications", { days: 30 });
      await load();
      if (removed > 0) alert(`${removed} old notification(s) cleared. Security alerts are always retained.`);
    } catch (e: any) {
      alert(String(e));
    } finally {
      setClearing(false);
    }
  };

  const unreadCount = entries.filter((e) => !e.read).length;
  const securityEntries = entries.filter((e) =>
    ["integrity_failure", "signature_invalid", "key_revoked"].includes(e.category)
  );

  return (
    <div className="view-container">
      <div className="view-header">
        <div>
          <h1 className="view-title">
            Activity
            {unreadCount > 0 && (
              <span className="notif-badge">{unreadCount}</span>
            )}
          </h1>
          <p className="view-subtitle">Persistent log of all Ryzora events. Security alerts are never auto-deleted.</p>
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <button className="btn btn-ghost" onClick={markAllRead} disabled={unreadCount === 0}>
            <CheckCheck className="w-4 h-4" /> Mark all read
          </button>
          <button className="btn btn-ghost" onClick={clearOld} disabled={clearing}>
            <Trash2 className="w-4 h-4" /> {clearing ? "Clearing…" : "Clear old (30d)"}
          </button>
        </div>
      </div>

      {/* Security alerts pinned at top */}
      {securityEntries.length > 0 && filter === "" && (
        <div className="notif-security-banner">
          <Shield className="w-4 h-4" />
          <span>
            <strong>{securityEntries.length} security alert{securityEntries.length > 1 ? "s" : ""}</strong> — review required.
          </span>
        </div>
      )}

      {/* Filter bar */}
      <div className="notif-filter-bar">
        <Filter className="w-4 h-4" style={{ opacity: 0.5 }} />
        {FILTER_OPTIONS.map((opt) => (
          <button
            key={opt.value}
            className={`filter-chip ${filter === opt.value ? "active" : ""}`}
            onClick={() => setFilter(opt.value)}
          >
            {opt.label}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="loading-state">
          <Bell className="w-8 h-8 animate-pulse" style={{ color: "var(--accent)" }} />
          <p>Loading activity…</p>
        </div>
      ) : entries.length === 0 ? (
        <div className="empty-state">
          <Bell className="w-12 h-12" style={{ opacity: 0.3 }} />
          <p>No activity yet.</p>
        </div>
      ) : (
        <div className="notif-list">
          {entries.map((entry) => {
            const isSecurity = ["integrity_failure", "signature_invalid", "key_revoked"].includes(entry.category);
            return (
              <div
                key={entry.id}
                className={`notif-entry ${!entry.read ? "unread" : ""} ${isSecurity ? "security" : ""}`}
                onClick={() => !entry.read && markRead(entry.id)}
              >
                <div className="notif-icon">
                  {SEVERITY_ICON[entry.severity] ?? <Bell className="w-4 h-4" />}
                </div>
                <div className="notif-body">
                  <div className="notif-meta">
                    <span className="notif-category">
                      {CATEGORY_ICON[entry.category]}
                      {entry.category.replace(/_/g, " ")}
                    </span>
                    <span className="notif-ts">{entry.timestamp.replace("T", " ").replace("Z", " UTC")}</span>
                    {!entry.read && <span className="notif-dot" />}
                  </div>
                  <p className="notif-title">{entry.title}</p>
                  <p className="notif-message">{entry.message}</p>
                  {entry.related_package_id && (
                    <span className="notif-tag">pkg: {entry.related_package_id}</span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
