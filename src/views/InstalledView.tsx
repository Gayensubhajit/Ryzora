import React from "react";
import { DownloadCloud, RotateCcw } from "lucide-react";
import { useApp } from "../context/AppContext";
import { PackageCard } from "../components/PackageCard";

export const InstalledView: React.FC = () => {
  const { packages, installedPackageIds, setActiveCategory } = useApp();

  const installedPackages = packages.filter((pkg) => installedPackageIds.includes(pkg.id));

  return (
    <div className="space-y-6 pb-12">
      <div className="p-8 rounded-3xl bg-gradient-to-r from-slate-900/90 via-slate-900/60 to-transparent border border-slate-800 flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2 text-emerald-400 text-xs font-bold uppercase tracking-wider mb-2">
            <DownloadCloud className="w-4 h-4" />
            <span>Active Configurations</span>
          </div>
          <h1 className="text-3xl font-extrabold text-white tracking-tight mb-2">
            Installed Packages
          </h1>
          <p className="text-sm text-slate-400 max-w-xl leading-relaxed">
            Manage customization packages currently applied to your Linux system. Configurations are sandboxed and backed up.
          </p>
        </div>

        <button
          onClick={() => setActiveCategory("backups")}
          className="hidden sm:flex items-center gap-2 px-4 py-2.5 rounded-xl bg-slate-800 hover:bg-slate-700 text-xs font-semibold text-slate-200 border border-slate-700 transition-colors"
        >
          <RotateCcw className="w-4 h-4 text-indigo-400" />
          <span>View Backups & Snapshots</span>
        </button>
      </div>

      {installedPackages.length === 0 ? (
        <div className="p-16 text-center rounded-3xl bg-slate-900/40 border border-slate-800 space-y-4">
          <div className="text-slate-300 font-semibold">No packages installed yet.</div>
          <p className="text-xs text-slate-400 max-w-sm mx-auto">
            Discover complete rices, status bars, and fastfetch layouts in the marketplace.
          </p>
          <button
            onClick={() => setActiveCategory("discover")}
            className="px-5 py-2 rounded-xl text-xs font-bold bg-cyan-500 hover:bg-cyan-400 text-black transition-colors"
          >
            Explore Discover
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
              Active Packages ({installedPackages.length})
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
            {installedPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
