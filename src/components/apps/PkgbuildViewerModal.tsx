/**
 * PkgbuildViewerModal — Phase 25
 *
 * Displays the full raw PKGBUILD script of an AUR package before build/installation.
 * Highlights security warnings:
 *  - User-contributed package
 *  - Builds locally as normal user (never root)
 *  - Review source URLs and build functions (build(), package())
 */

import React, { useState, useEffect } from "react";
import { X, ShieldAlert, Terminal, Loader2, Download } from "lucide-react";
import { aurAppProvider } from "../../providers/index.ts";

interface PkgbuildViewerModalProps {
  packageName: string;
  isOpen: boolean;
  onClose: () => void;
  onConfirmInstall: () => void;
}

export const PkgbuildViewerModal: React.FC<PkgbuildViewerModalProps> = ({
  packageName,
  isOpen,
  onClose,
  onConfirmInstall,
}) => {
  const [pkgbuild, setPkgbuild] = useState<string>("");
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen || !packageName) return;
    setLoading(true);
    setError(null);

    aurAppProvider
      .getPkgbuild(packageName)
      .then((content) => {
        setPkgbuild(content);
        setLoading(false);
      })
      .catch((err) => {
        setError(typeof err === "string" ? err : "Failed to load PKGBUILD from AUR.");
        setLoading(false);
      });
  }, [isOpen, packageName]);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-fadeIn">
      <div className="bg-[var(--rz-surface)] border border-[var(--rz-border)] rounded-2xl shadow-2xl max-w-2xl w-full flex flex-col max-h-[85vh] overflow-hidden">
        {/* Header */}
        <div className="p-4 sm:p-5 border-b border-[var(--rz-border)] flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-amber-500/10 text-amber-500 flex items-center justify-center font-mono text-sm font-bold">
              <Terminal size={18} />
            </div>
            <div>
              <h2 className="text-base font-bold text-[var(--rz-text)]">
                Review PKGBUILD · {packageName}
              </h2>
              <p className="text-xs text-[var(--rz-text-muted)]">
                Inspect build instructions before installing from Arch User Repository
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] transition-colors"
          >
            <X size={18} />
          </button>
        </div>

        {/* Security Alert Banner */}
        <div className="p-3.5 mx-4 mt-4 rounded-xl bg-amber-500/10 border border-amber-500/20 text-xs text-amber-700 dark:text-amber-300 flex items-start gap-2.5">
          <ShieldAlert size={18} className="shrink-0 mt-0.5 text-amber-500" />
          <div className="space-y-0.5 leading-relaxed">
            <span className="font-semibold">User-Contributed AUR Package</span>
            <p className="text-[11px] opacity-90">
              AUR packages execute local build scripts (`PKGBUILD`). Ryzora builds this package strictly as an
              unprivileged user. Always check source URLs and compilation commands.
            </p>
          </div>
        </div>

        {/* PKGBUILD Code Viewer */}
        <div className="flex-1 overflow-y-auto p-4 font-mono text-xs">
          {loading ? (
            <div className="h-48 flex flex-col items-center justify-center gap-2 text-[var(--rz-text-muted)]">
              <Loader2 size={24} className="animate-spin text-blue-500" />
              <span>Fetching PKGBUILD from aur.archlinux.org…</span>
            </div>
          ) : error ? (
            <div className="p-4 rounded-xl bg-red-500/10 border border-red-500/20 text-red-500 text-xs">
              {error}
            </div>
          ) : (
            <pre className="p-4 rounded-xl bg-[var(--rz-surface-hover)] border border-[var(--rz-border)] overflow-x-auto text-[var(--rz-text)] leading-relaxed text-[11px]">
              {pkgbuild}
            </pre>
          )}
        </div>

        {/* Footer Actions */}
        <div className="p-4 border-t border-[var(--rz-border)] bg-[var(--rz-surface-elevated)] flex items-center justify-between">
          <button
            type="button"
            onClick={onClose}
            className="px-4 py-2 rounded-xl text-xs font-semibold text-[var(--rz-text-muted)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-hover)] transition-colors"
          >
            Cancel
          </button>

          <button
            type="button"
            disabled={loading}
            onClick={() => {
              onClose();
              onConfirmInstall();
            }}
            className="inline-flex items-center gap-2 px-5 py-2 rounded-xl text-xs font-semibold bg-amber-600 hover:bg-amber-500 text-white transition-all shadow-md hover:shadow-amber-500/20 cursor-pointer disabled:opacity-50"
          >
            <Download size={14} />
            <span>Approve & Build</span>
          </button>
        </div>
      </div>
    </div>
  );
};
