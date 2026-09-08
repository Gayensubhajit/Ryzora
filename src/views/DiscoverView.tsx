import React from "react";
import { TrendingUp, CheckCircle2 } from "lucide-react";
import { useApp } from "../context/AppContext";
import { HeroBanner } from "../components/HeroBanner";
import { PackageCard } from "../components/PackageCard";
import { CategoryId } from "../types";

export const DiscoverView: React.FC = () => {
  const { packages, searchQuery, desktopFilter, systemInfo, setActiveCategory } = useApp();

  const filteredPackages = packages.filter((pkg) => {
    const matchesSearch =
      !searchQuery ||
      pkg.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.subtitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
      pkg.tags.some((t) => t.toLowerCase().includes(searchQuery.toLowerCase())) ||
      pkg.category.toLowerCase().includes(searchQuery.toLowerCase());

    const matchesDesktop =
      desktopFilter === "all" ||
      pkg.supported_desktops.includes("universal") ||
      pkg.supported_desktops.includes(desktopFilter);

    return matchesSearch && matchesDesktop;
  });

  const featuredRice = packages.find((p) => p.featured) || packages[0];

  const currentWm = systemInfo?.window_manager.toLowerCase() || "hyprland";
  const compatiblePackages = filteredPackages.filter(
    (p) =>
      p.supported_desktops.includes("universal") ||
      p.supported_desktops.some((d) => d === currentWm || currentWm.includes(d))
  );

  const quickCategories: { id: CategoryId; label: string; count: number }[] = [
    { id: "rices", label: "Complete Rices", count: packages.filter((p) => p.category === "rices").length },
    { id: "bars", label: "Waybar & Bars", count: packages.filter((p) => p.category === "bars").length },
    { id: "fastfetch", label: "Fastfetch Specs", count: packages.filter((p) => p.category === "fastfetch").length },
    { id: "lockscreens", label: "Lockscreens", count: packages.filter((p) => p.category === "lockscreens").length },
    { id: "themes", label: "GTK / Qt Themes", count: packages.filter((p) => p.category === "themes").length },
    { id: "wallpapers", label: "Wallpapers", count: packages.filter((p) => p.category === "wallpapers").length },
    { id: "terminal", label: "Terminal & Shell", count: packages.filter((p) => p.category === "terminal").length },
  ];

  return (
    <div className="space-y-10 pb-12">
      {!searchQuery && desktopFilter === "all" && featuredRice && (
        <HeroBanner featuredPackage={featuredRice} />
      )}

      {!searchQuery && (
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-slate-400">
              Browse Categories
            </span>
          </div>
          <div className="flex items-center gap-2.5 overflow-x-auto pb-1">
            {quickCategories.map((c) => (
              <button
                key={c.id}
                onClick={() => setActiveCategory(c.id)}
                className="flex items-center gap-2 px-4 py-2 rounded-xl bg-slate-900/80 hover:bg-slate-800/80 border border-slate-800 hover:border-cyan-500/30 text-xs font-medium text-slate-300 transition-all flex-shrink-0 group"
              >
                <span className="group-hover:text-cyan-300 transition-colors">{c.label}</span>
                <span className="px-1.5 py-0.2 rounded-md bg-slate-800 group-hover:bg-cyan-500/20 text-slate-400 group-hover:text-cyan-300 font-mono text-[10px]">
                  {c.count}
                </span>
              </button>
            ))}
          </div>
        </div>
      )}

      {!searchQuery && compatiblePackages.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <CheckCircle2 className="w-4 h-4 text-emerald-400" />
              <h2 className="text-lg font-bold text-white tracking-tight">
                Recommended for your {systemInfo?.window_manager || "Desktop"}
              </h2>
            </div>
            <span className="text-xs text-slate-400 font-medium">
              Matched against active session ({systemInfo?.session_type || "wayland"})
            </span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
            {compatiblePackages.slice(0, 3).map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        </div>
      )}

      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <TrendingUp className="w-4 h-4 text-cyan-400" />
            <h2 className="text-lg font-bold text-white tracking-tight">
              {searchQuery ? `Search Results for "${searchQuery}"` : "Trending Across Linux"}
            </h2>
          </div>
          <span className="text-xs text-slate-400 font-mono">
            {filteredPackages.length} packages available
          </span>
        </div>

        {filteredPackages.length === 0 ? (
          <div className="p-12 text-center rounded-3xl bg-slate-900/40 border border-slate-800 space-y-3">
            <div className="text-slate-400 text-sm">No packages match your search or filter.</div>
            <button
              onClick={() => setActiveCategory("discover")}
              className="text-xs text-cyan-400 font-semibold hover:underline"
            >
              Reset filters
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-5">
            {filteredPackages.map((pkg) => (
              <PackageCard key={pkg.id} packageItem={pkg} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
