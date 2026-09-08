import React from "react";
import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";
import { useApp } from "../context/AppContext";

export const Toast: React.FC = () => {
  const { toast, setToast } = useApp();

  if (!toast) return null;

  const isSuccess = toast.type === "success";
  const isWarning = toast.type === "warning";

  return (
    <div className="fixed bottom-5 right-5 z-50 flex items-center gap-2.5 px-3.5 py-2.5 rounded-lg bg-[var(--bg-surface-elevated)] border border-[var(--border-strong)] shadow-xl animate-in slide-in-from-bottom-2 duration-200">
      {isSuccess && <CheckCircle2 className="w-4 h-4 text-emerald-400 flex-shrink-0" />}
      {isWarning && <AlertTriangle className="w-4 h-4 text-amber-400 flex-shrink-0" />}
      {!isSuccess && !isWarning && <Info className="w-4 h-4 text-[var(--accent-text)] flex-shrink-0" />}

      <span className="text-xs text-[var(--text-primary)] font-medium max-w-sm">
        {toast.message}
      </span>

      <button
        onClick={() => setToast(null)}
        className="p-0.5 text-[var(--text-muted)] hover:text-white rounded transition-colors ml-1"
      >
        <X className="w-3 h-3" />
      </button>
    </div>
  );
};
