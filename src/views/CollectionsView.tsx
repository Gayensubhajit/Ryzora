import React, { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Collection,
  CollectionSummary,
  CollectionInstallPreview,
  CollectionInstallResult,
} from "../types";
import {
  Bookmark,
  Plus,
  Download,
  Upload,
  Trash2,
  Package,
  Play,
  CheckCircle,
  AlertTriangle,
  ChevronRight,
  X,
} from "lucide-react";

export const CollectionsView: React.FC = () => {
  const [summaries, setSummaries] = useState<CollectionSummary[]>([]);
  const [selected, setSelected] = useState<Collection | null>(null);
  const [loading, setLoading] = useState(true);
  const [preview, setPreview] = useState<CollectionInstallPreview | null>(null);
  const [installResult, setInstallResult] = useState<CollectionInstallResult | null>(null);
  const [installing, setInstalling] = useState(false);
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDesc, setNewDesc] = useState("");
  const [importJson, setImportJson] = useState("");
  const [showImport, setShowImport] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<CollectionSummary[]>("list_collections");
      setSummaries(list);
    } catch {
      setSummaries([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const selectCollection = async (id: string) => {
    try {
      const col = await invoke<Collection>("get_collection", { id });
      setSelected(col);
      setPreview(null);
      setInstallResult(null);
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const createCollection = async () => {
    if (!newName.trim()) return;
    try {
      await invoke("create_collection", {
        name: newName.trim(),
        description: newDesc.trim() || null,
      });
      setCreating(false);
      setNewName("");
      setNewDesc("");
      await load();
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const deleteCollection = async (id: string) => {
    if (!confirm("Delete this collection? This cannot be undone.")) return;
    try {
      await invoke("delete_collection", { id });
      if (selected?.id === id) setSelected(null);
      await load();
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const exportCollection = async (id: string) => {
    try {
      const json = await invoke<string>("export_collection", { id });
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${id}.ryzlist`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const importCollection = async () => {
    if (!importJson.trim()) return;
    try {
      await invoke("import_collection", { json: importJson });
      setImportJson("");
      setShowImport(false);
      await load();
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const removePackage = async (collectionId: string, packageId: string) => {
    try {
      const updated = await invoke<Collection>("remove_package_from_collection", {
        collectionId,
        packageId,
      });
      setSelected(updated);
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const previewInstall = async (id: string) => {
    try {
      const p = await invoke<CollectionInstallPreview>("preview_collection_install", {
        collectionId: id,
      });
      setPreview(p);
    } catch (e: any) {
      setFeedback(String(e));
    }
  };

  const installCollection = async (id: string) => {
    setInstalling(true);
    setInstallResult(null);
    try {
      const result = await invoke<CollectionInstallResult>("install_collection", {
        collectionId: id,
      });
      setInstallResult(result);
    } catch (e: any) {
      setFeedback(String(e));
    } finally {
      setInstalling(false);
    }
  };

  return (
    <div className="view-container">
      <div className="view-header">
        <div>
          <h1 className="view-title">Collections</h1>
          <p className="view-subtitle">Saved package lists in .ryzlist format — importable, exportable, and git-friendly.</p>
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <button className="btn btn-ghost" onClick={() => setShowImport(true)}>
            <Upload className="w-4 h-4" /> Import .ryzlist
          </button>
          <button className="btn btn-primary" onClick={() => setCreating(true)}>
            <Plus className="w-4 h-4" /> New Collection
          </button>
        </div>
      </div>

      {feedback && (
        <div className="settings-feedback err" onClick={() => setFeedback(null)}>
          <AlertTriangle className="w-4 h-4" /> {feedback}
        </div>
      )}

      <div className="collections-layout">
        {/* Sidebar list */}
        <div className="collections-sidebar">
          {loading ? (
            <div className="loading-state" style={{ padding: "24px" }}>
              <Bookmark className="w-6 h-6 animate-pulse" style={{ color: "var(--accent)" }} />
            </div>
          ) : summaries.length === 0 ? (
            <div className="empty-state" style={{ padding: "24px", textAlign: "center" }}>
              <Bookmark className="w-8 h-8" style={{ opacity: 0.3 }} />
              <p style={{ fontSize: "12px", marginTop: "8px" }}>No collections yet.</p>
            </div>
          ) : (
            summaries.map((s) => (
              <div
                key={s.id}
                className={`collection-item ${selected?.id === s.id ? "active" : ""}`}
                onClick={() => selectCollection(s.id)}
              >
                <div className="collection-item-main">
                  <Bookmark className="w-4 h-4" style={{ color: "var(--accent)", flexShrink: 0 }} />
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <p className="collection-name">{s.name}</p>
                    <p className="collection-meta">{s.package_count} package{s.package_count !== 1 ? "s" : ""}</p>
                  </div>
                  <ChevronRight className="w-3 h-3" style={{ opacity: 0.4, flexShrink: 0 }} />
                </div>
              </div>
            ))
          )}
        </div>

        {/* Detail panel */}
        <div className="collections-detail">
          {!selected ? (
            <div className="empty-state">
              <Bookmark className="w-12 h-12" style={{ opacity: 0.2 }} />
              <p>Select a collection to view details.</p>
            </div>
          ) : (
            <>
              <div className="collection-detail-header">
                <div>
                  <h2 className="collection-detail-name">{selected.name}</h2>
                  {selected.description && (
                    <p className="collection-detail-desc">{selected.description}</p>
                  )}
                  <p className="collection-meta">
                    {selected.packages.length} packages · Updated {selected.updated_at.split("T")[0]}
                  </p>
                </div>
                <div style={{ display: "flex", gap: "8px" }}>
                  <button className="btn btn-ghost" onClick={() => exportCollection(selected.id)}>
                    <Download className="w-4 h-4" /> Export
                  </button>
                  <button className="btn btn-ghost danger" onClick={() => deleteCollection(selected.id)}>
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              </div>

              {/* Package list */}
              <div className="collection-packages">
                {selected.packages.length === 0 ? (
                  <p style={{ opacity: 0.5, fontSize: "13px" }}>No packages in this collection.</p>
                ) : (
                  selected.packages.map((pkg) => (
                    <div key={pkg.id} className="collection-pkg-row">
                      <Package className="w-4 h-4" style={{ color: "var(--accent)", flexShrink: 0 }} />
                      <span className="collection-pkg-id">{pkg.id}</span>
                      {pkg.version_req && (
                        <span className="collection-version-req">{pkg.version_req}</span>
                      )}
                      {pkg.repository && (
                        <span className="collection-repo-tag">{pkg.repository}</span>
                      )}
                      <button
                        className="btn-icon"
                        onClick={() => removePackage(selected.id, pkg.id)}
                        title="Remove from collection"
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </div>
                  ))
                )}
              </div>

              {/* Install section */}
              <div className="collection-install-section">
                {!preview && !installResult && (
                  <button className="btn btn-ghost" onClick={() => previewInstall(selected.id)}>
                    <Play className="w-4 h-4" /> Preview Install
                  </button>
                )}

                {preview && !installResult && (
                  <div className="collection-preview">
                    <div className="preview-row">
                      <span>To install:</span>
                      <strong>{preview.to_install.length}</strong>
                    </div>
                    <div className="preview-row">
                      <span>Already installed:</span>
                      <strong>{preview.already_installed.length}</strong>
                    </div>
                    {preview.unavailable.length > 0 && (
                      <div className="preview-row warn">
                        <AlertTriangle className="w-3 h-3" />
                        <span>{preview.unavailable.length} unavailable: {preview.unavailable.join(", ")}</span>
                      </div>
                    )}
                    {preview.to_install.length > 0 && (
                      <button
                        className="btn btn-primary"
                        onClick={() => installCollection(selected.id)}
                        disabled={installing}
                      >
                        <Download className="w-4 h-4" />
                        {installing ? "Installing…" : `Install ${preview.to_install.length} package${preview.to_install.length !== 1 ? "s" : ""}`}
                      </button>
                    )}
                  </div>
                )}

                {installResult && (
                  <div className="collection-result">
                    <CheckCircle className="w-4 h-4" style={{ color: "var(--accent)" }} />
                    <span>
                      {installResult.installed.length} installed · {installResult.skipped.length} skipped
                      {installResult.failed.length > 0 && ` · ${installResult.failed.length} failed`}
                    </span>
                    <button className="btn btn-ghost" onClick={() => { setPreview(null); setInstallResult(null); }}>
                      Dismiss
                    </button>
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Create modal */}
      {creating && (
        <div className="modal-overlay" onClick={() => setCreating(false)}>
          <div className="modal-panel" onClick={(e) => e.stopPropagation()}>
            <h3 className="modal-title">New Collection</h3>
            <div className="form-field">
              <label>Name</label>
              <input
                className="form-input"
                placeholder="My Hyprland Rice"
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && createCollection()}
                autoFocus
              />
            </div>
            <div className="form-field">
              <label>Description (optional)</label>
              <input
                className="form-input"
                placeholder="A curated set of packages for my setup"
                value={newDesc}
                onChange={(e) => setNewDesc(e.target.value)}
              />
            </div>
            <div className="modal-actions">
              <button className="btn btn-ghost" onClick={() => setCreating(false)}>Cancel</button>
              <button className="btn btn-primary" onClick={createCollection} disabled={!newName.trim()}>Create</button>
            </div>
          </div>
        </div>
      )}

      {/* Import modal */}
      {showImport && (
        <div className="modal-overlay" onClick={() => setShowImport(false)}>
          <div className="modal-panel" onClick={(e) => e.stopPropagation()}>
            <h3 className="modal-title">Import .ryzlist</h3>
            <p style={{ fontSize: "13px", opacity: 0.7, marginBottom: "12px" }}>
              Paste the contents of a .ryzlist JSON file. All package IDs are validated before import.
            </p>
            <textarea
              className="form-input"
              style={{ height: "200px", resize: "vertical", fontFamily: "monospace", fontSize: "12px" }}
              placeholder={'{\n  "ryzora_collection": "1",\n  "name": "...",\n  "packages": [...]\n}'}
              value={importJson}
              onChange={(e) => setImportJson(e.target.value)}
            />
            <div className="modal-actions">
              <button className="btn btn-ghost" onClick={() => setShowImport(false)}>Cancel</button>
              <button className="btn btn-primary" onClick={importCollection} disabled={!importJson.trim()}>Import</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
