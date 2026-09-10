import type { PackageItem } from "../../types/index.ts";

/** Formats download count concisely (e.g., 2.4k, 120k, 850) */
export function formatDownloads(count: number): string {
  if (!count && count !== 0) return "0";
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1).replace(/\.0$/, "")}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1).replace(/\.0$/, "")}k`;
  }
  return count.toString();
}

/** Determines if a package represents or supports a system-level login screen (like SDDM) */
export function isLoginScreen(pkg: PackageItem): boolean {
  if (pkg.supports_login_screen !== undefined) {
    return pkg.supports_login_screen;
  }
  if (pkg.lockscreen?.targets?.sddm !== undefined) {
    return true;
  }
  const normTitle = pkg.title.toLowerCase();
  const normId = pkg.id.toLowerCase();
  const hasTag = pkg.tags.some((t) => t.toLowerCase() === "sddm");
  const hasComp = pkg.components?.some((c) => c.component_type?.toLowerCase().includes("sddm"));
  const hasDep = pkg.dependencies?.packages?.some((d) => d.toLowerCase().includes("sddm"));
  return hasTag || hasComp || hasDep || normTitle.includes("sddm") || normId.includes("sddm");
}

/** Determines if a package represents or supports a session lock (Hyprlock, Quickshell, Swaylock) */
export function isSessionLock(pkg: PackageItem): boolean {
  if (pkg.category !== "lockscreens" && pkg.package_type !== "lockscreen") {
    return false;
  }
  if (pkg.supports_session_lock !== undefined) {
    return pkg.supports_session_lock;
  }
  if (
    pkg.lockscreen?.targets?.quickshell !== undefined ||
    pkg.lockscreen?.targets?.hyprlock !== undefined ||
    pkg.lockscreen?.targets?.swaylock !== undefined
  ) {
    return true;
  }
  return !isLoginScreen(pkg);
}

/** Resolves the specific technology / engine subtype for any PackageItem */
export function getPackageSubtype(pkg: PackageItem): string {
  const allTags = pkg.tags.map((t) => t.toLowerCase());
  const title = pkg.title.toLowerCase();

  if (pkg.category === "lockscreens" || pkg.package_type === "lockscreen") {
    if (pkg.lockscreen?.targets?.quickshell && pkg.lockscreen?.targets?.sddm) {
      return "Quickshell · SDDM";
    }
    if (allTags.includes("sddm") || title.includes("sddm") || pkg.lockscreen?.targets?.sddm) return "SDDM";
    if (allTags.includes("quickshell") || title.includes("quickshell") || allTags.includes("qylock") || pkg.lockscreen?.targets?.quickshell) return "Quickshell";
    if (allTags.includes("swaylock") || title.includes("swaylock") || pkg.lockscreen?.targets?.swaylock) return "Swaylock";
    if (allTags.includes("hyprlock") || title.includes("hyprlock") || pkg.lockscreen?.targets?.hyprlock) return "Hyprlock";
    return "Lock Screen";
  }

  if (pkg.category === "rices" || pkg.package_type === "rice") {
    if (pkg.supported_desktops.includes("hyprland") || allTags.includes("hyprland")) return "Hyprland";
    if (pkg.supported_desktops.includes("sway") || allTags.includes("sway")) return "Sway";
    if (pkg.supported_desktops.includes("kde") || allTags.includes("kde")) return "KDE";
    if (pkg.supported_desktops.includes("gnome") || allTags.includes("gnome")) return "GNOME";
    return "Rice";
  }

  if (pkg.category === "bars" || pkg.package_type === "waybar") {
    if (allTags.includes("waybar") || title.includes("waybar")) return "Waybar";
    if (allTags.includes("eww") || title.includes("eww")) return "Eww";
    if (allTags.includes("polybar") || title.includes("polybar")) return "Polybar";
    return "Bar";
  }

  if (pkg.category === "terminal" || pkg.package_type === "terminal") {
    if (allTags.includes("kitty") || title.includes("kitty")) return "Kitty";
    if (allTags.includes("alacritty") || title.includes("alacritty")) return "Alacritty";
    if (allTags.includes("foot") || title.includes("foot")) return "Foot";
    if (allTags.includes("starship") || title.includes("starship")) return "Starship";
    return "Terminal";
  }

  if (pkg.category === "themes" || pkg.package_type === "theme") {
    if (allTags.includes("gtk")) return "GTK";
    if (allTags.includes("qt")) return "Qt";
    return "Theme";
  }

  if (pkg.category === "wallpapers" || pkg.package_type === "wallpaper") {
    return "Wallpaper";
  }

  if (pkg.category === "fastfetch" || pkg.package_type === "fastfetch") {
    return "Fastfetch";
  }

  if (pkg.category === "bundles") {
    return "Bundle";
  }

  return pkg.category ? pkg.category.charAt(0).toUpperCase() + pkg.category.slice(1) : "Package";
}

/** Resolves a concise compatibility badge string for display on cards */
export function getConciseCompatibility(pkg: PackageItem, currentWm?: string): { label: string; isWarning?: boolean } {
  if (pkg.supports_session_lock && pkg.supports_login_screen) {
    return { label: "✓ Session & Login" };
  }

  if (isLoginScreen(pkg) && !pkg.supports_session_lock) {
    return { label: "SDDM Login Screen", isWarning: true };
  }

  const normWm = currentWm?.toLowerCase() || "";
  const supportsCurrent =
    normWm &&
    (pkg.supported_desktops.includes("universal") ||
      pkg.supported_desktops.some((d) => d.toLowerCase() === normWm || normWm.includes(d.toLowerCase())));

  if (supportsCurrent) {
    const wmName = normWm.charAt(0).toUpperCase() + normWm.slice(1);
    return { label: `✓ ${wmName}` };
  }

  const firstDesktop = pkg.supported_desktops[0];
  if (firstDesktop && firstDesktop !== "universal") {
    return { label: `✓ ${firstDesktop.charAt(0).toUpperCase() + firstDesktop.slice(1)}` };
  }

  if (pkg.supported_display.includes("wayland")) {
    return { label: "✓ Wayland" };
  }

  return { label: "✓ Universal" };
}
