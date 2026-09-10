import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  FlaskConical,
  Shield,
  Monitor,
  Lock,
  LogIn,
  Sliders,
  Layers,
  RefreshCw,
} from "lucide-react";
import { HostProfileFixture, PackageItem } from "../types";
import { RAW_QYLOCK_THEMES, normalizeQylockTheme, resolveLockscreenCapabilities, TargetResolutionResult } from "../providers/qylockProvider";

export const CompatibilityLabView: React.FC = () => {
  const [profiles, setProfiles] = useState<HostProfileFixture[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState<string>("live");
  const [selectedPackageId, setSelectedPackageId] = useState<string>("clockwork-orbital");
  const [loading, setLoading] = useState(true);
  const [evaluationResult, setEvaluationResult] = useState<TargetResolutionResult | null>(null);

  // Normalized package items from Qylock repository
  const availablePackages: PackageItem[] = RAW_QYLOCK_THEMES.map(normalizeQylockTheme);
  const selectedPackage = availablePackages.find((p) => p.id === selectedPackageId) || availablePackages[0];

  // Load profiles from backend
  const loadProfiles = async () => {
    setLoading(true);
    try {
      const data = await invoke<HostProfileFixture[]>("get_compatibility_lab_profiles");
      setProfiles(data);
    } catch (err) {
      console.error("Failed to load profiles from Tauri command, fallback to client fixture:", err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadProfiles();
  }, []);

  const activeProfile = profiles.find((p) => p.id === selectedProfileId) || profiles[0];

  // Evaluate compatibility whenever profile or package changes
  useEffect(() => {
    if (!activeProfile || !selectedPackage?.lockscreen) return;

    // Form SystemInfo from activeProfile capabilities
    const mockSystemInfo = {
      os: activeProfile.capabilities.os,
      kernel: "6.x",
      desktop_environment: activeProfile.capabilities.desktop_environment,
      window_manager: activeProfile.capabilities.compositor,
      session_type: activeProfile.capabilities.session_type,
      display_manager: activeProfile.capabilities.display_manager,
      installed_commands: activeProfile.capabilities.installed_commands,
      supported_adapters: activeProfile.capabilities.supported_adapters,
    };

    const evaluated = resolveLockscreenCapabilities(
      mockSystemInfo as any,
      selectedPackage.lockscreen,
      "both"
    );
    setEvaluationResult(evaluated);
  }, [activeProfile, selectedPackage]);

  return (
    <div className="space-y-6 pb-12 antialiased text-[var(--rz-text)]">
      {/* ── Header ── */}
      <div className="flex items-center justify-between gap-4 flex-wrap border-b border-[var(--rz-border-subtle)] pb-5">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-purple-500/10 border border-purple-500/30 text-purple-400">
            <FlaskConical className="w-6 h-6" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-xl font-bold tracking-tight">Compatibility Lab</h1>
              <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase tracking-wider bg-amber-500/20 text-amber-300 border border-amber-500/30">
                DEV
              </span>
            </div>
            <p className="text-xs text-[var(--rz-text-muted)] mt-0.5">
              Visual Debugger for Host Capabilities & Concrete Runtime Adapter Intersections
            </p>
          </div>
        </div>

        <button
          type="button"
          disabled={loading}
          onClick={loadProfiles}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold border border-[var(--rz-border-subtle)] hover:bg-[var(--rz-surface-elevated)] transition-colors cursor-pointer disabled:opacity-50"
        >
          <RefreshCw className={["w-3.5 h-3.5", loading ? "animate-spin text-purple-400" : ""].join(" ")} />
          <span>{loading ? "Probing..." : "Refresh Live Probe"}</span>
        </button>
      </div>

      {/* ── Profile Switcher ── */}
      <div className="space-y-2">
        <div className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
          <Monitor className="w-3.5 h-3.5 text-purple-400" />
          <span>Select Environment Profile</span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5">
          {profiles.map((profile) => {
            const isSelected = profile.id === selectedProfileId;
            return (
              <button
                key={profile.id}
                type="button"
                onClick={() => setSelectedProfileId(profile.id)}
                className={[
                  "p-3 rounded-xl border text-left transition-all cursor-pointer flex flex-col justify-between",
                  isSelected
                    ? "bg-[var(--rz-surface-elevated)] border-purple-500/60 shadow-md ring-1 ring-purple-500/30"
                    : "bg-[var(--rz-surface)] border-[var(--rz-border-subtle)] hover:border-white/20 hover:bg-[var(--rz-surface-elevated)]/60",
                ].join(" ")}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-bold text-xs truncate">{profile.name}</span>
                  {profile.is_live ? (
                    <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 shrink-0">
                      LIVE
                    </span>
                  ) : (
                    <span className="px-1.5 py-0.5 rounded text-[9px] font-mono uppercase bg-neutral-500/20 text-neutral-300 border border-neutral-500/30 shrink-0">
                      SIMULATED
                    </span>
                  )}
                </div>
                <p className="text-[11px] text-[var(--rz-text-muted)] mt-1 line-clamp-2">
                  {profile.description}
                </p>
                <div className="flex items-center gap-2 mt-2 pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono text-[var(--rz-text-muted)]">
                  <span>{profile.capabilities.desktop_environment}</span>
                  <span>·</span>
                  <span>{profile.capabilities.compositor}</span>
                  <span>·</span>
                  <span className="uppercase">{profile.capabilities.display_manager}</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {activeProfile && (
        <>
          {/* ── Active Profile Detail Card ── */}
          <div className="p-4 rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] shadow-sm">
            <div className="flex items-center justify-between gap-2 mb-3 pb-2 border-b border-[var(--rz-border-subtle)]">
              <div className="flex items-center gap-2">
                <Shield className="w-4 h-4 text-purple-400" />
                <h2 className="text-sm font-bold">Host Profile Specification</h2>
              </div>
              <div className="text-[11px] font-mono text-[var(--rz-text-muted)]">
                Status: {activeProfile.is_live ? "Real Hardware Telemetry" : "Simulated Fixture (Zero Side Effects)"}
              </div>
            </div>

            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3 text-xs">
              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Operating System</div>
                <div className="font-semibold truncate mt-0.5" title={activeProfile.capabilities.os}>
                  {activeProfile.capabilities.os}
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Desktop / DE</div>
                <div className="font-semibold truncate mt-0.5">
                  {activeProfile.capabilities.desktop_environment}
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Compositor / WM</div>
                <div className="font-semibold truncate mt-0.5">
                  {activeProfile.capabilities.compositor}
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Session Protocol</div>
                <div className="font-semibold truncate mt-0.5">
                  {activeProfile.capabilities.session_lock_protocol}
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Login Manager</div>
                <div className="font-semibold uppercase truncate mt-0.5">
                  {activeProfile.capabilities.display_manager}
                </div>
              </div>

              <div className="p-2.5 rounded-xl bg-[var(--rz-surface-elevated)]/60 border border-[var(--rz-border-subtle)]">
                <div className="text-[10px] text-[var(--rz-text-muted)] uppercase font-mono">Session Type</div>
                <div className="font-semibold uppercase truncate mt-0.5">
                  {activeProfile.capabilities.session_type}
                </div>
              </div>
            </div>
          </div>

          {/* ── Host Adapters Inspection ── */}
          <div className="space-y-2">
            <div className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
              <Layers className="w-3.5 h-3.5 text-purple-400" />
              <span>Host Adapters Compatibility State</span>
            </div>

            <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
              {activeProfile.capabilities.supported_adapters.map((adapter) => {
                const isSession = adapter.category === "session_lock";
                return (
                  <div
                    key={adapter.adapter}
                    className="p-3.5 rounded-xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] flex flex-col justify-between"
                  >
                    <div>
                      <div className="flex items-center justify-between gap-2">
                        <div className="flex items-center gap-2">
                          {isSession ? <Lock className="w-4 h-4 text-purple-400" /> : <LogIn className="w-4 h-4 text-amber-400" />}
                          <span className="font-bold text-xs">{adapter.name}</span>
                        </div>
                        <span
                          className={[
                            "px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase",
                            adapter.supported
                              ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                              : "bg-red-500/20 text-red-300 border border-red-500/30",
                          ].join(" ")}
                        >
                          {adapter.supported ? "Supported" : "Unsupported"}
                        </span>
                      </div>

                      <p className="text-[11px] text-[var(--rz-text-muted)] mt-2 leading-relaxed">
                        {adapter.reason}
                      </p>
                    </div>

                    <div className="flex items-center justify-between gap-2 mt-3 pt-2 border-t border-[var(--rz-border-subtle)] text-[10.5px] font-mono text-[var(--rz-text-muted)]">
                      <span>Protocol: {adapter.protocol}</span>
                      <span>Privilege: {adapter.required_privilege}</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* ── Package Intersection Tester ── */}
          <div className="space-y-3 pt-2">
            <div className="flex items-center justify-between gap-2 flex-wrap">
              <div className="text-xs font-bold uppercase tracking-wider text-[var(--rz-text-muted)] flex items-center gap-1.5">
                <Sliders className="w-3.5 h-3.5 text-purple-400" />
                <span>Package Capability Intersection Resolver</span>
              </div>

              {/* Package Selector */}
              <div className="flex items-center gap-2">
                <span className="text-xs text-[var(--rz-text-muted)]">Package:</span>
                <select
                  value={selectedPackageId}
                  onChange={(e) => setSelectedPackageId(e.target.value)}
                  className="bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] rounded-lg px-2.5 py-1 text-xs font-semibold text-[var(--rz-text)] cursor-pointer"
                >
                  {availablePackages.map((pkg) => (
                    <option key={pkg.id} value={pkg.id}>
                      {pkg.title} ({pkg.id})
                    </option>
                  ))}
                </select>
              </div>
            </div>

            {/* Resolved Evaluation Results */}
            {evaluationResult?.evaluations && (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                {/* 1. Quickshell Session Lock Evaluation */}
                {evaluationResult.evaluations.quickshell && (
                  <div className="p-4 rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] flex flex-col justify-between">
                    <div>
                      <div className="flex items-center justify-between gap-2 pb-2 border-b border-[var(--rz-border-subtle)]">
                        <div className="flex items-center gap-2">
                          <Lock className="w-4 h-4 text-purple-400" />
                          <span className="font-bold text-xs">{evaluationResult.evaluations.quickshell.target_name}</span>
                        </div>
                        <span
                          className={[
                            "px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase",
                            evaluationResult.evaluations.quickshell.supported
                              ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                              : "bg-red-500/20 text-red-300 border border-red-500/30",
                          ].join(" ")}
                        >
                          {evaluationResult.evaluations.quickshell.supported ? "Compatible" : "Unsupported"}
                        </span>
                      </div>

                      {/* Checks breakdown */}
                      <div className="mt-3 space-y-2 text-xs">
                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Package Capability:</span>
                          <span className={evaluationResult.evaluations.quickshell.checks.package_capability.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.quickshell.checks.package_capability.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Compositor Protocol:</span>
                          <span className={evaluationResult.evaluations.quickshell.checks.compositor_protocol.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.quickshell.checks.compositor_protocol.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Runtime Binary:</span>
                          <span className={evaluationResult.evaluations.quickshell.checks.runtime_binary.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.quickshell.checks.runtime_binary.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Authentication / PAM:</span>
                          <span className={evaluationResult.evaluations.quickshell.checks.authentication.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.quickshell.checks.authentication.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Privilege Boundary:</span>
                          <span className="text-emerald-400 font-semibold">
                            {evaluationResult.evaluations.quickshell.checks.privilege_boundary.detected}
                          </span>
                        </div>
                      </div>

                      {/* Why unsupported? */}
                      <div
                        className={[
                          "mt-3 p-2.5 rounded-xl border text-[11px] leading-relaxed",
                          evaluationResult.evaluations.quickshell.supported
                            ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-200"
                            : "bg-red-500/10 border-red-500/30 text-red-200",
                        ].join(" ")}
                      >
                        <div className="font-bold mb-0.5">
                          {evaluationResult.evaluations.quickshell.supported ? "Compatibility Rationale:" : "Why Unsupported?"}
                        </div>
                        <div>{evaluationResult.evaluations.quickshell.reason}</div>
                      </div>
                    </div>

                    <div className="mt-3 pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono text-[var(--rz-text-muted)] flex items-center justify-between">
                      <span>Zero Side Effect Guarantee</span>
                      <span>System Changed: No</span>
                    </div>
                  </div>
                )}

                {/* 2. SDDM Login Screen Evaluation */}
                {evaluationResult.evaluations.sddm && (
                  <div className="p-4 rounded-2xl border border-[var(--rz-border-subtle)] bg-[var(--rz-surface)] flex flex-col justify-between">
                    <div>
                      <div className="flex items-center justify-between gap-2 pb-2 border-b border-[var(--rz-border-subtle)]">
                        <div className="flex items-center gap-2">
                          <LogIn className="w-4 h-4 text-amber-400" />
                          <span className="font-bold text-xs">{evaluationResult.evaluations.sddm.target_name}</span>
                        </div>
                        <span
                          className={[
                            "px-2 py-0.5 rounded text-[10px] font-mono font-bold uppercase",
                            evaluationResult.evaluations.sddm.supported
                              ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                              : "bg-amber-500/20 text-amber-300 border border-amber-500/30",
                          ].join(" ")}
                        >
                          {evaluationResult.evaluations.sddm.supported ? "Compatible" : "Unsupported on Host"}
                        </span>
                      </div>

                      {/* Checks breakdown */}
                      <div className="mt-3 space-y-2 text-xs">
                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Package Capability:</span>
                          <span className={evaluationResult.evaluations.sddm.checks.package_capability.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.sddm.checks.package_capability.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Display Manager Match:</span>
                          <span className={evaluationResult.evaluations.sddm.checks.display_manager?.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.sddm.checks.display_manager?.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Runtime Binary:</span>
                          <span className={evaluationResult.evaluations.sddm.checks.runtime_binary.passed ? "text-emerald-400 font-semibold" : "text-red-400 font-semibold"}>
                            {evaluationResult.evaluations.sddm.checks.runtime_binary.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Authentication:</span>
                          <span className="text-emerald-400 font-semibold">
                            {evaluationResult.evaluations.sddm.checks.authentication.detected}
                          </span>
                        </div>

                        <div className="flex items-center justify-between text-[11px]">
                          <span className="text-[var(--rz-text-muted)]">Privilege Boundary:</span>
                          <span className="text-amber-400 font-semibold">
                            {evaluationResult.evaluations.sddm.checks.privilege_boundary.detected}
                          </span>
                        </div>
                      </div>

                      {/* Why unsupported? */}
                      <div
                        className={[
                          "mt-3 p-2.5 rounded-xl border text-[11px] leading-relaxed",
                          evaluationResult.evaluations.sddm.supported
                            ? "bg-emerald-500/10 border-emerald-500/30 text-emerald-200"
                            : "bg-amber-500/10 border-amber-500/30 text-amber-200",
                        ].join(" ")}
                      >
                        <div className="font-bold mb-0.5">
                          {evaluationResult.evaluations.sddm.supported ? "Compatibility Rationale:" : "Why Unsupported?"}
                        </div>
                        <div>{evaluationResult.evaluations.sddm.reason}</div>
                      </div>
                    </div>

                    <div className="mt-3 pt-2 border-t border-[var(--rz-border-subtle)] text-[10px] font-mono text-[var(--rz-text-muted)] flex items-center justify-between">
                      <span>Zero Side Effect Guarantee</span>
                      <span>System Changed: No</span>
                    </div>
                  </div>
                )}
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
};
