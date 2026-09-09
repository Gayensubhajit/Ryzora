import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { IntegrityScanReport, PackageIntegrityResult, IntegrityStatus } from "../types";
import {
  Shield,
  CheckCircle,
  AlertTriangle,
  XCircle,
  HelpCircle,
  Play,
  Clock,
  RefreshCw,
} from "lucide-react";

const STATUS_CONFIG: Record<
  IntegrityStatus,
  { icon: React.ReactNode; color: string; label: string }
> = {
  healthy: { icon: <CheckCircle className="w-4 h-4" />, color: "#4ade80", label: "Healthy" },
  modified: { icon: <AlertTriangle className="w-4 h-4" />, color: "#e8a53a", label: "Modified" },
  missing_files: { icon: <XCircle className="w-4 h-4" />, color: "#e05c5c", label: "Missing Files" },
  unexpected_files: { icon: <AlertTriangle className="w-4 h-4" />, color: "#e8a53a", label: "Unexpected Files" },
  signature_failed: { icon: <XCircle className="w-4 h-4" />, color: "#e05c5c", label: "Signature Failed" },
  key_revoked: { icon: <Shield className="w-4 h-4" />, color: "#e05c5c", label: "Key Revoked" },
  unable_to_verify: { icon: <HelpCircle className="w-4 h-4" />, color: "#6b7280", label: "Unable to Verify" },
};

export const IntegrityView: React.FC = () => {
  const [report, setReport] = useState<IntegrityScanReport | null>(null);
  const [scanning, setScanning] = useState(false);
  const [verifying, setVerifying] = useState<string | null>(null);

  useEffect(() => {
    invoke<IntegrityScanReport | null>("get_last_integrity_report")
      .then((r) => setReport(r ?? null))
      .catch(() => {});
  }, []);

  const runScan = async () => {
    setScanning(true);
    try {
      const r = await invoke<IntegrityScanReport>("run_integrity_scan");
      setReport(r);
    } catch (e: any) {
      alert(String(e));
    } finally {
      setScanning(false);
    }
  };

  const reVerify = async (packageId: string) => {
    setVerifying(packageId);
    try {
      const result = await invoke<PackageIntegrityResult>("verify_package_integrity", { packageId });
      if (!report) return;
      setReport((prev) => {
        if (!prev) return prev;
        const newAll = prev.all_results.map((r) =>
          r.package_id === packageId ? result : r
        );
        const newIssues = newAll.filter((r) => r.status !== "healthy");
        const newHealthy = newAll.filter((r) => r.status === "healthy").length;
        return { ...prev, all_results: newAll, issues: newIssues, healthy: newHealthy };
      });
    } catch (e: any) {
      alert(String(e));
    } finally {
      setVerifying(null);
    }
  };

  const healthPercent = report
    ? report.total_checked > 0
      ? Math.round((report.healthy / report.total_checked) * 100)
      : 100
    : null;

  return (
    <div className="view-container">
      <div className="view-header">
        <div>
          <h1 className="view-title">Integrity Health</h1>
          <p className="view-subtitle">
            Read-only verification of installed packages. No automatic repair — issues require manual review.
          </p>
        </div>
        <button className="btn btn-primary" onClick={runScan} disabled={scanning}>
          {scanning ? (
            <><RefreshCw className="w-4 h-4 animate-spin" /> Scanning…</>
          ) : (
            <><Play className="w-4 h-4" /> Scan Now</>
          )}
        </button>
      </div>

      {/* Summary cards */}
      {report && (
        <div className="integrity-summary">
          <div className="integrity-stat-card">
            <span className="stat-value" style={{ color: "#4ade80" }}>{report.healthy}</span>
            <span className="stat-label">Healthy</span>
          </div>
          <div className="integrity-stat-card">
            <span className="stat-value" style={{ color: report.issues.length > 0 ? "#e05c5c" : "var(--text-muted)" }}>
              {report.issues.length}
            </span>
            <span className="stat-label">Issues</span>
          </div>
          <div className="integrity-stat-card">
            <span className="stat-value">{report.total_checked}</span>
            <span className="stat-label">Total Scanned</span>
          </div>
          {healthPercent !== null && (
            <div className="integrity-stat-card">
              <span className="stat-value" style={{ color: healthPercent === 100 ? "#4ade80" : "#e8a53a" }}>
                {healthPercent}%
              </span>
              <span className="stat-label">Health</span>
            </div>
          )}
        </div>
      )}

      {report && (
        <div className="integrity-meta">
          <Clock className="w-3 h-3" />
          Last scan: {report.scanned_at.replace("T", " ").replace("Z", " UTC")} · {report.duration_ms}ms
        </div>
      )}

      {/* Issues */}
      {report && report.issues.length > 0 && (
        <div className="integrity-section">
          <h2 className="integrity-section-title">
            <AlertTriangle className="w-4 h-4" style={{ color: "#e8a53a" }} />
            Issues Detected
          </h2>
          <div className="integrity-table">
            {report.issues.map((r) => (
              <IntegrityRow
                key={r.package_id}
                result={r}
                onReVerify={reVerify}
                verifying={verifying}
              />
            ))}
          </div>
        </div>
      )}

      {/* All results */}
      {report && report.all_results.length > 0 && (
        <div className="integrity-section">
          <h2 className="integrity-section-title">
            <Shield className="w-4 h-4" style={{ color: "var(--accent)" }} />
            All Installed Packages
          </h2>
          <div className="integrity-table">
            {report.all_results.map((r) => (
              <IntegrityRow
                key={r.package_id}
                result={r}
                onReVerify={reVerify}
                verifying={verifying}
              />
            ))}
          </div>
        </div>
      )}

      {!report && !scanning && (
        <div className="empty-state">
          <Shield className="w-12 h-12" style={{ opacity: 0.2 }} />
          <p>No scan results yet.</p>
          <p style={{ fontSize: "12px", opacity: 0.5 }}>Click "Scan Now" to run a read-only integrity check.</p>
        </div>
      )}

      <div className="integrity-disclaimer">
        <Shield className="w-3 h-3" />
        Integrity scans are read-only. No files are modified or repaired automatically.
        "Unable to Verify" is never treated as Healthy.
      </div>
    </div>
  );
};

const IntegrityRow: React.FC<{
  result: PackageIntegrityResult;
  onReVerify: (id: string) => void;
  verifying: string | null;
}> = ({ result, onReVerify, verifying }) => {
  const cfg = STATUS_CONFIG[result.status] ?? STATUS_CONFIG.unable_to_verify;
  const isVerifying = verifying === result.package_id;

  return (
    <div className={`integrity-row ${result.status !== "healthy" ? "issue" : ""}`}>
      <div className="integrity-row-status" style={{ color: cfg.color }}>
        {cfg.icon}
      </div>
      <div className="integrity-row-info">
        <div className="integrity-row-name">{result.name}</div>
        <div className="integrity-row-meta">
          <span className="integrity-pkg-id">{result.package_id}</span>
          <span className="integrity-version">v{result.version}</span>
          <span className="integrity-trust">{result.trust_tier}</span>
        </div>
        {result.status !== "healthy" && (
          <p className="integrity-detail">{result.detail}</p>
        )}
      </div>
      <div className="integrity-row-badge" style={{ color: cfg.color }}>
        {cfg.label}
      </div>
      <button
        className="btn btn-ghost btn-sm"
        onClick={() => onReVerify(result.package_id)}
        disabled={isVerifying}
        title="Re-verify this package"
      >
        <RefreshCw className={`w-3 h-3 ${isVerifying ? "animate-spin" : ""}`} />
      </button>
    </div>
  );
};
