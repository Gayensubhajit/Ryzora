import React, { useState } from "react";
import { ChevronDown, ChevronRight, Check } from "lucide-react";
import { RepositorySummary } from "../../types";

interface FilterPanelProps {
  // Types (Hyprlock, Quickshell, Swaylock, SDDM)
  availableTypes: string[];
  selectedTypes: string[];
  onToggleType: (type: string) => void;

  // Compatibility (Hyprland, KDE, Sway, Wayland)
  availableCompatibility: string[];
  selectedCompatibility: string[];
  onToggleCompatibility: (comp: string) => void;

  // Status (all, installed, not_installed, updates)
  status: string;
  onChangeStatus: (status: string) => void;

  // Advanced: Verified only
  verifiedOnly: boolean;
  onToggleVerified: (val: boolean) => void;

  // Advanced: Repository
  repositories: RepositorySummary[];
  selectedRepo: string;
  onChangeRepo: (repo: string) => void;
}

export const FilterPanel: React.FC<FilterPanelProps> = ({
  availableTypes,
  selectedTypes,
  onToggleType,
  availableCompatibility,
  selectedCompatibility,
  onToggleCompatibility,
  status,
  onChangeStatus,
  verifiedOnly,
  onToggleVerified,
  repositories,
  selectedRepo,
  onChangeRepo,
}) => {
  const [showAdvanced, setShowAdvanced] = useState(false);

  return (
    <div className="mb-4 p-3.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] space-y-3.5 text-xs shadow-xs animate-in fade-in duration-150">
      {/* ── Basic Filters ── */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-3.5">
        {/* Filter by Type / Subcategory */}
        {availableTypes.length > 0 && (
          <div>
            <span className="block text-[11px] font-bold uppercase tracking-wider text-[var(--rz-text-secondary)] mb-1.5">
              Type
            </span>
            <div className="flex flex-wrap gap-1.5">
              {availableTypes.map((type) => {
                const isSelected = selectedTypes.includes(type);
                const isSddm = type.toLowerCase() === "sddm";
                return (
                  <button
                    key={type}
                    type="button"
                    onClick={() => onToggleType(type)}
                    className={[
                      "px-2.5 py-1 rounded-md text-[11px] font-medium border transition-all cursor-pointer select-none flex items-center gap-1",
                      isSelected
                        ? "bg-[var(--rz-accent)]/20 border-[var(--rz-accent)] text-[var(--rz-accent-text)] font-semibold"
                        : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)]",
                    ].join(" ")}
                  >
                    {isSelected && <Check className="w-2.5 h-2.5" />}
                    <span>{type}</span>
                    {isSddm && (
                      <span className="text-[8px] font-mono uppercase bg-amber-500/20 text-amber-400 px-1 py-0.2 rounded ml-0.5">
                        Login
                      </span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Compatibility */}
        <div>
          <span className="block text-[11px] font-bold uppercase tracking-wider text-[var(--rz-text-secondary)] mb-1.5">
            Desktop Compatibility
          </span>
          <div className="flex flex-wrap gap-1.5">
            {availableCompatibility.map((comp) => {
              const isSelected = selectedCompatibility.includes(comp);
              return (
                <button
                  key={comp}
                  type="button"
                  onClick={() => onToggleCompatibility(comp)}
                  className={[
                    "px-2.5 py-1 rounded-md text-[11px] font-medium border transition-all cursor-pointer select-none flex items-center gap-1",
                    isSelected
                      ? "bg-[var(--rz-accent)]/20 border-[var(--rz-accent)] text-[var(--rz-accent-text)] font-semibold"
                      : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)]",
                  ].join(" ")}
                >
                  {isSelected && <Check className="w-2.5 h-2.5" />}
                  <span>{comp}</span>
                </button>
              );
            })}
          </div>
        </div>

        {/* Status */}
        <div>
          <span className="block text-[11px] font-bold uppercase tracking-wider text-[var(--rz-text-secondary)] mb-1.5">
            Installation Status
          </span>
          <div className="flex flex-wrap gap-1.5">
            {[
              { id: "all", label: "All" },
              { id: "installed", label: "Installed" },
              { id: "not_installed", label: "Not Installed" },
              { id: "updates", label: "Updates Available" },
            ].map((st) => {
              const isSelected = status === st.id;
              return (
                <button
                  key={st.id}
                  type="button"
                  onClick={() => onChangeStatus(st.id)}
                  className={[
                    "px-2.5 py-1 rounded-md text-[11px] font-medium border transition-all cursor-pointer select-none",
                    isSelected
                      ? "bg-[var(--rz-accent)]/20 border-[var(--rz-accent)] text-[var(--rz-accent-text)] font-semibold"
                      : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)]",
                  ].join(" ")}
                >
                  {st.label}
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* ── Advanced Section (Collapsible) ── */}
      <div className="pt-2 border-t border-[var(--rz-border-subtle)]/70">
        <button
          type="button"
          onClick={() => setShowAdvanced((v) => !v)}
          className="flex items-center gap-1.5 text-[11px] font-semibold text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] transition-colors cursor-pointer select-none"
        >
          {showAdvanced ? (
            <ChevronDown className="w-3.5 h-3.5" />
          ) : (
            <ChevronRight className="w-3.5 h-3.5" />
          )}
          <span>Advanced Options</span>
          {(verifiedOnly || selectedRepo !== "all") && (
            <span className="w-1.5 h-1.5 rounded-full bg-[var(--rz-accent)] ml-1" />
          )}
        </button>

        {showAdvanced && (
          <div className="mt-2.5 grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
            {/* Verified status checkbox */}
            <label className="flex items-center gap-2 text-xs text-[var(--rz-text)] cursor-pointer select-none">
              <input
                type="checkbox"
                checked={verifiedOnly}
                onChange={(e) => onToggleVerified(e.target.checked)}
                className="rounded border-[var(--rz-border-subtle)] text-[var(--rz-accent)] focus:ring-[var(--rz-accent)]"
              />
              <span>Only show verified & official packages</span>
            </label>

            {/* Repository filter */}
            {repositories.length > 1 && (
              <div className="flex items-center gap-2">
                <span className="text-xs text-[var(--rz-text-secondary)]">Repository:</span>
                <select
                  value={selectedRepo}
                  onChange={(e) => onChangeRepo(e.target.value)}
                  className="ryz-select text-[11px] py-1 px-2"
                >
                  <option value="all">All Repositories</option>
                  {repositories.map((r) => (
                    <option key={r.id} value={r.id}>
                      {r.name}
                    </option>
                  ))}
                </select>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
