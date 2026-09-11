import React, { useState, useRef, useEffect } from "react";
import { ChevronDown, Check, Circle, Package } from "lucide-react";
import {
  type PackageProviderOption,
  SUPPORTED_PROVIDERS,
} from "./appState.ts";

interface ProviderSelectorProps {
  selectedProvider: PackageProviderOption;
  onSelectProvider: (provider: PackageProviderOption) => void;
  availableProviders?: PackageProviderOption[];
  disabled?: boolean;
}

export const ProviderSelector: React.FC<ProviderSelectorProps> = ({
  selectedProvider,
  onSelectProvider,
  availableProviders,
  disabled = false,
}) => {
  const [isOpen, setIsOpen] = useState(() => {
    try {
      return new URLSearchParams(window.location.search).get("openMenu") === "provider";
    } catch {
      return false;
    }
  });
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const handleOutsideClick = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    if (isOpen) {
      document.addEventListener("mousedown", handleOutsideClick);
    }
    return () => {
      document.removeEventListener("mousedown", handleOutsideClick);
    };
  }, [isOpen]);

  const providersList = availableProviders && availableProviders.length > 0
    ? availableProviders
    : SUPPORTED_PROVIDERS;

  // Single provider: render static source pill with NO dropdown caret
  if (providersList.length <= 1) {
    const single = providersList[0] || selectedProvider;
    return (
      <div className="inline-flex items-center gap-2 px-3.5 py-2.5 rounded-xl font-medium text-xs sm:text-sm bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)] text-[var(--rz-text)] shadow-xs">
        <Package size={14} className="text-blue-500 dark:text-blue-400" />
        <span className="font-semibold">{single.shortName}</span>
        {single.version && (
          <span className="text-[11px] text-[var(--rz-text-muted)] font-mono">
            {single.version}
          </span>
        )}
      </div>
    );
  }

  // Multiple providers: render interactive selector dropdown
  return (
    <div ref={containerRef} className="relative inline-block text-left">
      <button
        type="button"
        disabled={disabled}
        onClick={() => setIsOpen((prev) => !prev)}
        className={[
          "inline-flex items-center gap-2 px-3.5 py-2.5 rounded-xl font-medium text-xs sm:text-sm",
          "bg-[var(--rz-surface-elevated)] hover:bg-[var(--rz-surface-hover)] text-[var(--rz-text)]",
          "border border-[var(--rz-border)] shadow-xs transition-all cursor-pointer",
          "disabled:opacity-50 disabled:cursor-not-allowed",
        ].join(" ")}
        aria-haspopup="true"
        aria-expanded={isOpen}
      >
        <Package size={14} className="text-[var(--rz-text-muted)]" />
        <span className="font-semibold">{selectedProvider.shortName}</span>
        <ChevronDown size={14} className={`text-[var(--rz-text-muted)] transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`} />
      </button>

      {isOpen && (
        <div
          className={[
            "absolute left-0 top-full mt-2 w-72 rounded-2xl z-50 p-2",
            "bg-[var(--rz-surface-elevated)] border border-[var(--rz-border)]",
            "shadow-xl backdrop-blur-xl animate-fadeIn",
          ].join(" ")}
          role="menu"
        >
          <div className="px-3 py-1.5 text-[10px] font-bold uppercase tracking-wider text-[var(--rz-text-muted)]">
            Installation Source
          </div>

          <div className="space-y-1 mt-1">
            {providersList.map((provider) => {
              const isSelected = selectedProvider.id === provider.id;
              const isActionable = provider.available !== false;

              return (
                <button
                  key={`${provider.id}-${provider.targetId}`}
                  type="button"
                  disabled={!isActionable}
                  onClick={() => {
                    if (isActionable) {
                      onSelectProvider(provider);
                      setIsOpen(false);
                    }
                  }}
                  className={[
                    "w-full flex items-start justify-between p-2.5 rounded-xl text-left transition-all",
                    isActionable
                      ? "hover:bg-[var(--rz-surface-hover)] cursor-pointer"
                      : "opacity-60 cursor-not-allowed",
                    isSelected ? "bg-blue-600/10 border border-blue-500/30" : "border border-transparent",
                  ].join(" ")}
                  role="menuitem"
                >
                  <div className="min-w-0 pr-2">
                    <div className="flex items-center gap-1.5">
                      {isSelected ? (
                        <Check size={13} className="text-blue-600 dark:text-blue-400 shrink-0" />
                      ) : (
                        <Circle size={13} className="text-[var(--rz-text-muted)] shrink-0 opacity-40" />
                      )}
                      <span className="text-xs font-bold text-[var(--rz-text)]">
                        {provider.shortName}
                      </span>
                      {provider.statusNote && (
                        <span className="text-[9px] px-1.5 py-0.2 rounded-full font-mono bg-zinc-500/20 text-[var(--rz-text-muted)] border border-zinc-500/20 uppercase">
                          {provider.statusNote}
                        </span>
                      )}
                    </div>
                    {provider.version && (
                      <div className="text-[11px] font-mono text-blue-600 dark:text-blue-400 pl-5 pt-0.5">
                        {provider.version}
                      </div>
                    )}
                    <p className="text-[11px] text-[var(--rz-text-muted)] pl-5 pt-0.5 leading-tight">
                      {provider.description}
                    </p>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
