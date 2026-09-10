import React from "react";
import { X, ShieldCheck, ExternalLink, Sparkles } from "lucide-react";
import { useApp } from "../context/AppContext";

interface AboutModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  const { systemInfo } = useApp();

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4 animate-in fade-in duration-150">
      <div 
        className="w-full max-w-sm rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] p-5 shadow-2xl space-y-4 text-[var(--text-primary)]"
        role="dialog"
        aria-modal="true"
        aria-labelledby="about-dialog-title"
      >
        {/* Header */}
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-gradient-to-tr from-cyan-500 to-indigo-600 flex items-center justify-center shadow-md">
              <Sparkles className="w-5 h-5 text-white" />
            </div>
            <div>
              <h2 id="about-dialog-title" className="text-base font-bold tracking-tight">Ryzora</h2>
              <p className="text-[11px] text-[var(--text-muted)]">Linux Customization Platform</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] p-1 rounded-md hover:bg-[var(--rz-surface-hover)] transition-colors"
            title="Close dialog"
            aria-label="Close"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Version & Build */}
        <div className="flex items-center justify-between text-xs py-1.5 px-2.5 rounded-md bg-[var(--bg-canvas)] border border-[var(--border-subtle)] font-mono">
          <span className="text-[var(--text-muted)]">Version 0.1.0</span>
          <span className="text-[10px] text-[var(--accent)] font-semibold">Release Candidate</span>
        </div>

        {/* System Environment */}
        <div className="space-y-1.5 text-xs">
          <div className="text-[10px] uppercase font-semibold text-[var(--text-faint)] tracking-wider">
            Environment
          </div>
          <div className="grid grid-cols-2 gap-2 text-[11px] p-2.5 rounded-md bg-[var(--bg-canvas)]/50 border border-[var(--border-subtle)]">
            <div>
              <span className="text-[var(--text-faint)] block text-[10px]">Desktop</span>
              <span className="font-medium text-[var(--text-primary)]">{systemInfo?.desktop_environment || "Hyprland"}</span>
            </div>
            <div>
              <span className="text-[var(--text-faint)] block text-[10px]">Session</span>
              <span className="font-medium text-[var(--text-primary)] capitalize">{systemInfo?.session_type || "wayland"}</span>
            </div>
            <div>
              <span className="text-[var(--text-faint)] block text-[10px]">Distribution</span>
              <span className="font-medium text-[var(--text-primary)] truncate block">{systemInfo?.distro_name || "Linux Host"}</span>
            </div>
            <div>
              <span className="text-[var(--text-faint)] block text-[10px]">Terminal</span>
              <span className="font-medium text-[var(--text-primary)]">{systemInfo?.terminal || "kitty"}</span>
            </div>
          </div>
        </div>

        {/* Security Invariants */}
        <div className="space-y-1.5 text-xs">
          <div className="text-[10px] uppercase font-semibold text-[var(--text-faint)] tracking-wider flex items-center gap-1">
            <ShieldCheck className="w-3 h-3 text-emerald-400" />
            <span>Security Certified</span>
          </div>
          <ul className="text-[11px] space-y-1 text-[var(--text-muted)] px-1">
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
              <span>Declarative packages (no install scripts)</span>
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
              <span>Ed25519 cryptographic trust verification</span>
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
              <span>Zero subprocess execution in installer</span>
            </li>
            <li className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
              <span>Atomic snapshot and complete rollback</span>
            </li>
          </ul>
        </div>

        {/* Action Links */}
        <div className="pt-2 border-t border-[var(--border-subtle)] flex items-center justify-between text-xs">
          <a
            href="https://github.com/Gayensubhajit/Ryzora"
            target="_blank"
            rel="noopener noreferrer"
            className="text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] transition-colors flex items-center gap-1 text-[11px]"
          >
            <span>GitHub</span>
            <ExternalLink className="w-2.5 h-2.5" />
          </a>
          <a
            href="https://github.com/Gayensubhajit/Ryzora/blob/main/docs/SECURITY.md"
            target="_blank"
            rel="noopener noreferrer"
            className="text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] transition-colors flex items-center gap-1 text-[11px]"
          >
            <span>Security Policy</span>
            <ExternalLink className="w-2.5 h-2.5" />
          </a>
          <a
            href="https://github.com/Gayensubhajit/Ryzora/issues"
            target="_blank"
            rel="noopener noreferrer"
            className="text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] transition-colors flex items-center gap-1 text-[11px]"
          >
            <span>Report Issue</span>
            <ExternalLink className="w-2.5 h-2.5" />
          </a>
        </div>
      </div>
    </div>
  );
};
