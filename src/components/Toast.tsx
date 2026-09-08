import React from "react";
import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";
import { useApp } from "../context/AppContext";

export const Toast: React.FC = () => {
  const { toast, setToast } = useApp();

  if (!toast) return null;

  const isSuccess = toast.type === "success";
  const isWarning = toast.type === "warning";

  return (
    <div className="fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-2xl bg-slate-900/95 border border-slate-700 shadow-2xl backdrop-blur-xl animate-in slide-in-from-bottom-5 duration-300">
      {isSuccess && <CheckCircle2 className="w-5 h-5 text-emerald-400 flex-shrink-0" />}
      {isWarning && <AlertTriangle className="w-5 h-5 text-amber-400 flex-shrink-0" />}
      {!isSuccess && !isWarning && <Info className="w-5 h-5 text-cyan-400 flex-shrink-0" />}

      <span className="text-xs font-semibold text-white max-w-sm">
        {toast.message}
      </span>

      <button
        onClick={() => setToast(null)}
        className="p-1 text-slate-400 hover:text-white rounded-lg transition-colors ml-2"
      >
        <X className="w-3.5 h-3.5" />
      </button>
    </div>
  );
};
