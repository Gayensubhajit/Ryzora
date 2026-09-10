import { invoke } from "@tauri-apps/api/core";
export type AppearanceMode = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type AccentColor = "blue" | "purple" | "green" | "orange";

export interface AccessibilitySettings {
  reduceTransparency: boolean;
  reduceMotion: boolean;
  increaseContrast: boolean;
}

export const STORAGE_KEYS = {
  MODE: "ryzora_appearance_mode",
  ACCENT: "ryzora_accent_color",
  REDUCE_TRANSPARENCY: "ryzora_reduce_transparency",
  REDUCE_MOTION: "ryzora_reduce_motion",
  INCREASE_CONTRAST: "ryzora_increase_contrast",
} as const;

export const ACCENT_SWATCHES: Record<AccentColor, { name: string; hex: string; bgClass: string }> = {
  blue: { name: "Ocean Blue", hex: "#3478e5", bgClass: "bg-[#3478e5]" },
  purple: { name: "Amethyst", hex: "#8b5cf6", bgClass: "bg-[#8b5cf6]" },
  green: { name: "Emerald", hex: "#10b981", bgClass: "bg-[#10b981]" },
  orange: { name: "Amber Fire", hex: "#f97316", bgClass: "bg-[#f97316]" },
};

export function getSystemPreferredTheme(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) {
    return "dark";
  }
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function getSystemPrefersReducedMotion(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function getSystemPrefersReducedTransparency(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-reduced-transparency: reduce)").matches;
}

export function getSystemPrefersMoreContrast(): boolean {
  if (typeof window === "undefined" || !window.matchMedia) return false;
  return window.matchMedia("(prefers-contrast: more)").matches;
}

export function loadSavedAppearance(): {
  mode: AppearanceMode;
  accent: AccentColor;
  accessibility: AccessibilitySettings;
} {
  if (typeof window === "undefined") {
    return {
      mode: "system",
      accent: "blue",
      accessibility: { reduceTransparency: false, reduceMotion: false, increaseContrast: false },
    };
  }

  const mode = (localStorage.getItem(STORAGE_KEYS.MODE) as AppearanceMode) || "system";
  const accent = (localStorage.getItem(STORAGE_KEYS.ACCENT) as AccentColor) || "blue";

  const reduceTransparency =
    localStorage.getItem(STORAGE_KEYS.REDUCE_TRANSPARENCY) !== null
      ? localStorage.getItem(STORAGE_KEYS.REDUCE_TRANSPARENCY) === "true"
      : getSystemPrefersReducedTransparency();

  const reduceMotion =
    localStorage.getItem(STORAGE_KEYS.REDUCE_MOTION) !== null
      ? localStorage.getItem(STORAGE_KEYS.REDUCE_MOTION) === "true"
      : getSystemPrefersReducedMotion();

  const increaseContrast =
    localStorage.getItem(STORAGE_KEYS.INCREASE_CONTRAST) !== null
      ? localStorage.getItem(STORAGE_KEYS.INCREASE_CONTRAST) === "true"
      : getSystemPrefersMoreContrast();

  return {
    mode,
    accent,
    accessibility: { reduceTransparency, reduceMotion, increaseContrast },
  };
}

let cachedSystemTheme: ResolvedTheme = "dark";

export async function detectSystemTheme(): Promise<ResolvedTheme> {
  try {
    const res = await invoke<string>("get_system_appearance");
    if (res === "light" || res === "dark") {
      cachedSystemTheme = res;
      return res;
    }
  } catch {
    // Fallback to matchMedia
  }
  const theme = getSystemPreferredTheme();
  cachedSystemTheme = theme;
  return theme;
}

export function getCachedSystemTheme(): ResolvedTheme {
  return cachedSystemTheme;
}
