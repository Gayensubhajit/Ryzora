import React, { useState } from "react";
import {
  FileText,
  FolderGit2,
  Cpu,
  DownloadCloud,
  History,
  ShieldCheck,
  Lock,
  Code,
  Monitor,
  KeyRound,
} from "lucide-react";
import { PackageItem, SystemInfo } from "../../types";

type TabKey = "overview" | "files" | "dependencies" | "installation" | "changelog";

interface DetailTabsProps {
  packageItem: PackageItem;
  systemInfo: SystemInfo | null;
  selectedTarget?: "quickshell" | "sddm" | "both";
}

export const DetailTabs: React.FC<DetailTabsProps> = ({
  packageItem,
  systemInfo: _systemInfo,
  selectedTarget = "quickshell",
}) => {
  const [activeTab, setActiveTab] = useState<TabKey>("overview");

  const manifest = packageItem.lockscreen;
  const isVideo = packageItem.media_type === "video";

  // Compute dynamic dependencies based on selected target
  const getDependencies = () => {
    const multimediaDeps = isVideo ? ["qt6-multimedia", "gst-plugins-good"] : [];
    if (selectedTarget === "sddm") {
      return ["sddm", "qt6-declarative", "qt6-svg", ...multimediaDeps];
    }
    if (selectedTarget === "both") {
      return ["quickshell", "sddm", "qt6-declarative", "qt6-svg", ...multimediaDeps];
    }
    return ["quickshell", "qt6-declarative", ...multimediaDeps];
  };

  // Compute dynamic files based on selected target
  const getTargetFiles = () => {
    const themeId = packageItem.id;
    const assets = manifest?.runtime.assets || ["Main.qml", "theme.conf", "metadata.desktop"];

    if (selectedTarget === "sddm") {
      return [
        ...assets.map((a) => ({
          target: `/usr/share/sddm/themes/ryzora-${themeId}/${a}`,
          type: a.endsWith(".qml") ? "QML Script" : a.endsWith(".mp4") ? "Video Asset" : "Config",
          isSystem: true,
        })),
        {
          target: `/etc/sddm.conf.d/ryzora-theme.conf`,
          type: "SDDM Greeter Config",
          isSystem: true,
        },
      ];
    }

    if (selectedTarget === "both") {
      return [
        // User Space
        ...assets.map((a) => ({
          target: `~/.local/share/ryzora/lockscreens/qylock/${themeId}/${a}`,
          type: a.endsWith(".qml") ? "QML Script" : a.endsWith(".mp4") ? "Video Asset" : "Asset",
          isSystem: false,
        })),
        {
          target: `~/.local/share/ryzora/integrations/quickshell/lock.sh`,
          type: "Session Lock Launcher",
          isSystem: false,
        },
        // System Space
        ...assets.map((a) => ({
          target: `/usr/share/sddm/themes/ryzora-${themeId}/${a}`,
          type: a.endsWith(".qml") ? "QML Script" : a.endsWith(".mp4") ? "Video Asset" : "Asset",
          isSystem: true,
        })),
        {
          target: `/etc/sddm.conf.d/ryzora-theme.conf`,
          type: "SDDM Greeter Config",
          isSystem: true,
        },
      ];
    }

    // Default Quickshell user space
    return [
      ...assets.map((a) => ({
        target: `~/.local/share/ryzora/lockscreens/qylock/${themeId}/${a}`,
        type: a.endsWith(".qml") ? "QML Script" : a.endsWith(".mp4") ? "Video Asset" : "Asset",
        isSystem: false,
      })),
      {
        target: `~/.local/share/ryzora/integrations/quickshell/lock.sh`,
        type: "Session Lock Launcher",
        isSystem: false,
      },
    ];
  };

  const dependencies = getDependencies();
  const targetFiles = getTargetFiles();

  return (
    <div className="mt-6 border-t border-[var(--rz-border-subtle)] pt-4">
      {/* ── Tab Navigation Header ── */}
      <div className="flex items-center gap-1.5 border-b border-[var(--rz-border-subtle)] pb-2 overflow-x-auto scrollbar-none">
        {[
          { id: "overview", label: "Overview", icon: <FileText className="w-3.5 h-3.5" /> },
          { id: "files", label: "Files & Code", icon: <FolderGit2 className="w-3.5 h-3.5" />, badge: targetFiles.length },
          { id: "dependencies", label: "Dependencies", icon: <Cpu className="w-3.5 h-3.5" />, badge: dependencies.length },
          { id: "installation", label: "Installation", icon: <DownloadCloud className="w-3.5 h-3.5" /> },
          { id: "changelog", label: "Changelog", icon: <History className="w-3.5 h-3.5" /> },
        ].map((tab) => {
          const isActive = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveTab(tab.id as TabKey)}
              className={[
                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer select-none shrink-0",
                isActive
                  ? "bg-[var(--rz-accent)] text-white shadow-xs"
                  : "text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:bg-[var(--rz-surface-elevated)]",
              ].join(" ")}
            >
              {tab.icon}
              <span>{tab.label}</span>
              {typeof tab.badge === "number" && (
                <span
                  className={[
                    "ml-1 px-1.5 py-0.2 rounded-full text-[10px] font-mono",
                    isActive ? "bg-white/20 text-white" : "bg-[var(--rz-surface)] text-[var(--rz-text-muted)]",
                  ].join(" ")}
                >
                  {tab.badge}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* ── Tab Content Panels ── */}
      <div className="pt-4 text-xs">
        {/* TAB 1: OVERVIEW */}
        {activeTab === "overview" && (
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-bold text-[var(--rz-text)] mb-1">About this Lock Screen</h3>
              <p className="text-[var(--rz-text-secondary)] leading-relaxed text-xs">
                {packageItem.description}
              </p>
            </div>

            {/* Target Support Matrix */}
            <div className="p-3 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] space-y-2">
              <span className="text-[10px] font-mono uppercase tracking-wider text-[var(--rz-text-muted)] font-bold block">
                Target Architecture & Environment
              </span>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs">
                <div className="p-2 rounded-lg bg-[var(--rz-surface-elevated)] flex items-center gap-2">
                  <Lock className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  <div>
                    <span className="font-semibold text-[var(--rz-text)] block">Session Lock (Quickshell)</span>
                    <span className="text-[10px] text-[var(--rz-text-muted)]">User space · ext-session-lock-v1</span>
                  </div>
                </div>
                <div className="p-2 rounded-lg bg-[var(--rz-surface-elevated)] flex items-center gap-2">
                  <Monitor className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                  <div>
                    <span className="font-semibold text-[var(--rz-text)] block">Login Screen (SDDM)</span>
                    <span className="text-[10px] text-[var(--rz-text-muted)]">System space · Admin permission</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Security Guarantee */}
            <div className="flex items-start gap-2.5 p-3 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
              <ShieldCheck className="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
              <div>
                <div className="font-semibold text-[var(--rz-text)] text-xs">Ryzora Keyring & Provenance</div>
                <div className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 leading-relaxed">
                  Manifest signed and verified. All theme files are self-contained in Ryzora-managed directories and backed by automatic snapshots.
                </div>
              </div>
            </div>

            {/* Desktop Keybind & Idle Separation */}
            <div className="flex items-start gap-2.5 p-3 rounded-xl bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)]">
              <KeyRound className="w-4 h-4 text-sky-400 shrink-0 mt-0.5" />
              <div>
                <div className="font-semibold text-[var(--rz-text)] text-xs">Keybind & Desktop Idle Separation</div>
                <div className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 leading-relaxed">
                  Theme installation never silently overwrites your <code className="text-[var(--rz-accent-text)]">Super+L</code> shortcut or <code className="text-[var(--rz-accent-text)]">hypridle.conf</code>. Ryzora generates a self-contained launch script at <code className="text-[var(--rz-accent-text)]">~/.local/share/ryzora/integrations/quickshell/lock.sh</code> for explicit user configuration.
                </div>
              </div>
            </div>
          </div>
        )}

        {/* TAB 2: FILES & CODE */}
        {activeTab === "files" && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-[11px] text-[var(--rz-text-muted)] font-mono pb-1 border-b border-[var(--rz-border-subtle)]">
              <span>Target Destination Path ({selectedTarget.toUpperCase()})</span>
              <span>Classification</span>
            </div>
            <div className="divide-y divide-[var(--rz-border-subtle)]/50 font-mono text-[11px]">
              {targetFiles.map((file, idx) => (
                <div key={idx} className="py-2 flex items-center justify-between gap-3">
                  <div className="flex items-center gap-2 truncate">
                    <Code className={`w-3.5 h-3.5 shrink-0 ${file.isSystem ? "text-amber-400" : "text-[var(--rz-accent)]"}`} />
                    <span className={`truncate ${file.isSystem ? "text-amber-200" : "text-[var(--rz-text)]"}`}>
                      {file.target}
                    </span>
                  </div>
                  <span className={`px-1.5 py-0.5 rounded text-[9px] uppercase tracking-wider shrink-0 ${
                    file.isSystem
                      ? "bg-amber-500/20 text-amber-300 border border-amber-500/30"
                      : "bg-[var(--rz-surface-elevated)] text-[var(--rz-text-secondary)] border border-[var(--rz-border-subtle)]"
                  }`}>
                    {file.type}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* TAB 3: DEPENDENCIES */}
        {activeTab === "dependencies" && (
          <div className="space-y-3">
            <p className="text-[11px] text-[var(--rz-text-secondary)]">
              The following packages must be installed on your distribution for <strong>{selectedTarget.toUpperCase()}</strong>:
            </p>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 font-mono text-xs">
              {dependencies.map((dep) => (
                <div
                  key={dep}
                  className="flex items-center justify-between p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]"
                >
                  <span className="font-semibold text-[var(--rz-text)]">{dep}</span>
                  <span className="px-1.5 py-0.5 rounded text-[9px] bg-emerald-500/15 text-emerald-300">
                    Required
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* TAB 4: INSTALLATION PREVIEW */}
        {activeTab === "installation" && (
          <div className="space-y-3">
            <p className="text-[11px] text-[var(--rz-text-secondary)]">
              Step-by-step execution plan for <strong>{selectedTarget.toUpperCase()}</strong>:
            </p>
            <ol className="space-y-2 list-decimal list-inside text-[11px] text-[var(--rz-text-secondary)]">
              <li className="p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <strong className="text-[var(--rz-text)]">Configuration Snapshot:</strong> Create atomic backup of existing configurations in <code className="text-[var(--rz-accent-text)]">~/.local/share/ryzora/backups/</code>.
              </li>
              <li className="p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <strong className="text-[var(--rz-text)]">Staging & Checksum Verification:</strong> Download and verify all theme assets against sha256 hashes.
              </li>
              <li className="p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <strong className="text-[var(--rz-text)]">Materialization:</strong> Copy files directly into destination directory. {selectedTarget !== "quickshell" && <span className="text-amber-400 font-semibold">(Requires administrator pkexec confirmation)</span>}
              </li>
              <li className="p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <strong className="text-[var(--rz-text)]">Registration:</strong> Register ownership in Ryzora database for clean 1-click uninstalls.
              </li>
              <li className="p-2 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
                <strong className="text-[var(--rz-text)]">Integration (Optional):</strong> Desktop keybindings and idle configurations remain untouched; the launcher wrapper is created without silently modifying window manager settings.
              </li>
            </ol>
          </div>
        )}

        {/* TAB 5: CHANGELOG */}
        {activeTab === "changelog" && (
          <div className="space-y-2 text-xs">
            <div className="p-2.5 rounded-lg bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)]">
              <div className="flex items-center justify-between font-mono text-[11px]">
                <span className="font-bold text-[var(--rz-text)]">v{packageItem.version}</span>
                <span className="text-[var(--rz-text-muted)]">Initial Ryzora Release</span>
              </div>
              <ul className="mt-1.5 space-y-1 text-[11px] text-[var(--rz-text-secondary)] list-disc list-inside">
                <li>Materialized self-contained theme assets without external symlinks</li>
                <li>Support for video backgrounds and high-framerate playback</li>
                <li>Full integration with Quickshell session lock and SDDM greeter</li>
              </ul>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
