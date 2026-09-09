import React, { useState, useEffect } from "react";
import { Sparkles, X, Palette, Image as ImageIcon, LayoutGrid, CheckCircle2 } from "lucide-react";
import { useApp } from "../context/AppContext";

export const FirstRunBanner: React.FC = () => {
  const { setActiveCategory, systemInfo } = useApp();
  const [dismissed, setDismissed] = useState<boolean>(true);

  useEffect(() => {
    try {
      const isDismissed = localStorage.getItem("ryzora_first_run_dismissed");
      if (!isDismissed) {
        setDismissed(false);
      }
    } catch {
      // LocalStorage access fallback
      setDismissed(false);
    }
  }, []);

  const handleDismiss = () => {
    try {
      localStorage.setItem("ryzora_first_run_dismissed", "true");
    } catch {
      // Ignore
    }
    setDismissed(true);
  };

  if (dismissed) return null;

  return (
    <div className="mb-6 rounded-xl border border-[var(--border-subtle)] bg-gradient-to-r from-cyan-950/40 via-indigo-950/30 to-purple-950/30 p-4 shadow-lg backdrop-blur-sm relative animate-in fade-in slide-in-from-top-2 duration-200">
      <button
        onClick={handleDismiss}
        className="absolute top-3 right-3 text-[var(--text-muted)] hover:text-white p-1 rounded-md hover:bg-white/10 transition-colors"
        title="Dismiss onboarding banner"
        aria-label="Dismiss banner"
      >
        <X className="w-4 h-4" />
      </button>

      <div className="flex items-start gap-3">
        <div className="w-9 h-9 rounded-lg bg-[var(--accent)]/20 border border-[var(--accent)]/30 flex items-center justify-center shrink-0">
          <Sparkles className="w-5 h-5 text-[var(--accent)]" />
        </div>

        <div className="space-y-3 flex-1">
          <div>
            <h3 className="text-sm font-bold text-white flex items-center gap-2">
              <span>Welcome to Ryzora</span>
              <span className="text-[10px] font-normal px-2 py-0.5 rounded-full bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                Ready
              </span>
            </h3>
            <p className="text-xs text-[var(--text-muted)] mt-0.5">
              Declarative, cryptographic desktop customizer tailored for your system.
            </p>
          </div>

          {/* System status checks */}
          <div className="flex flex-wrap items-center gap-3 text-xs text-[var(--text-secondary)]">
            <span className="flex items-center gap-1.5">
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              <span>System detected ({systemInfo?.distro_name || "Linux Host"})</span>
            </span>
            <span className="flex items-center gap-1.5">
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              <span>Desktop ({systemInfo?.desktop_environment || "Hyprland"})</span>
            </span>
            <span className="flex items-center gap-1.5">
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
              <span>Providers online</span>
            </span>
          </div>

          {/* Non-intrusive navigation options */}
          <div className="flex flex-wrap items-center gap-2 pt-1">
            <span className="text-xs text-[var(--text-muted)] mr-1">Choose what to explore:</span>
            <button
              onClick={() => {
                setActiveCategory("themes");
                handleDismiss();
              }}
              className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-[var(--bg-surface-elevated)] hover:bg-[var(--accent)] hover:text-white text-slate-200 border border-[var(--border-subtle)] transition-colors"
            >
              <Palette className="w-3 h-3" />
              <span>Browse Themes</span>
            </button>
            <button
              onClick={() => {
                setActiveCategory("rices");
                handleDismiss();
              }}
              className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-[var(--bg-surface-elevated)] hover:bg-[var(--accent)] hover:text-white text-slate-200 border border-[var(--border-subtle)] transition-colors"
            >
              <LayoutGrid className="w-3 h-3" />
              <span>Browse Rices</span>
            </button>
            <button
              onClick={() => {
                setActiveCategory("wallpapers");
                handleDismiss();
              }}
              className="flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium bg-[var(--bg-surface-elevated)] hover:bg-[var(--accent)] hover:text-white text-slate-200 border border-[var(--border-subtle)] transition-colors"
            >
              <ImageIcon className="w-3 h-3" />
              <span>Browse Wallpapers</span>
            </button>
            <button
              onClick={handleDismiss}
              className="px-2.5 py-1 rounded-md text-xs text-[var(--text-faint)] hover:text-[var(--text-muted)] transition-colors ml-auto"
            >
              Skip
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
