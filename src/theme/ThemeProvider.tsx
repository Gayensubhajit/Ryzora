import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import React, { createContext, useContext, useState, useEffect, useCallback } from "react";
import {
  AppearanceMode,
  ResolvedTheme,
  AccentColor,
  AccessibilitySettings,
  STORAGE_KEYS,
  getSystemPreferredTheme,
  loadSavedAppearance,
  detectSystemTheme,
} from "./appearance";

interface ThemeContextType {
  mode: AppearanceMode;
  resolvedTheme: ResolvedTheme;
  accent: AccentColor;
  accessibility: AccessibilitySettings;
  setMode: (mode: AppearanceMode) => void;
  setAccent: (accent: AccentColor) => void;
  updateAccessibility: (settings: Partial<AccessibilitySettings>) => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export const ThemeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [initial] = useState(() => loadSavedAppearance());
  const [mode, setModeState] = useState<AppearanceMode>(initial.mode);
  const [accent, setAccentState] = useState<AccentColor>(initial.accent);
  const [accessibility, setAccessibilityState] = useState<AccessibilitySettings>(initial.accessibility);

  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() => {
    if (initial.mode === "system") {
      return getSystemPreferredTheme();
    }
    return initial.mode;
  });

  // Track OS system appearance changes when in "system" mode
  useEffect(() => {
    if (typeof window === "undefined" || !window.matchMedia) return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    const updateResolved = () => {
      if (mode === "system") {
        detectSystemTheme().then((sysTheme) => {
          setResolvedTheme(sysTheme);
        }).catch(() => {
          setResolvedTheme(mediaQuery.matches ? "dark" : "light");
        });
      } else {
        setResolvedTheme(mode);
      }
    };

    updateResolved();

    const handler = (e: MediaQueryListEvent) => {
      if (mode === "system") {
        setResolvedTheme(e.matches ? "dark" : "light");
      }
    };

    mediaQuery.addEventListener("change", handler);
    return () => mediaQuery.removeEventListener("change", handler);
  }, [mode]);

  // Synchronize DOM documentElement attributes and classes
  useEffect(() => {
    if (typeof document === "undefined") return;
    const root = document.documentElement;

    root.setAttribute("data-theme", resolvedTheme);
    root.setAttribute("data-accent", accent);
    root.setAttribute("data-reduced-transparency", String(accessibility.reduceTransparency));
    root.setAttribute("data-reduced-motion", String(accessibility.reduceMotion));
    root.setAttribute("data-increase-contrast", String(accessibility.increaseContrast));

    if (resolvedTheme === "dark") {
      root.classList.add("dark");
      root.classList.remove("light");
    } else {
      root.classList.add("light");
      root.classList.remove("dark");
    }

    // Inform Tauri / GTK window of the requested theme
    try {
      invoke("set_window_appearance", { theme: resolvedTheme }).catch(() => {});
      const win = getCurrentWindow();
      if (mode === "system") {
        win.setTheme(null).catch(() => {});
      } else {
        win.setTheme(resolvedTheme === "dark" ? "dark" : "light").catch(() => {});
      }
    } catch {
      // Ignored outside native Tauri window context
    }
  }, [resolvedTheme, accent, accessibility, mode]);

  const setMode = useCallback((newMode: AppearanceMode) => {
    setModeState(newMode);
    localStorage.setItem(STORAGE_KEYS.MODE, newMode);
    if (newMode === "system") {
      detectSystemTheme().then((sysTheme) => {
        setResolvedTheme(sysTheme);
      }).catch(() => {
        setResolvedTheme(getSystemPreferredTheme());
      });
    } else {
      setResolvedTheme(newMode);
    }
  }, []);

  const setAccent = useCallback((newAccent: AccentColor) => {
    setAccentState(newAccent);
    localStorage.setItem(STORAGE_KEYS.ACCENT, newAccent);
  }, []);

  const updateAccessibility = useCallback((partial: Partial<AccessibilitySettings>) => {
    setAccessibilityState((prev) => {
      const next = { ...prev, ...partial };
      if (partial.reduceTransparency !== undefined) {
        localStorage.setItem(STORAGE_KEYS.REDUCE_TRANSPARENCY, String(next.reduceTransparency));
      }
      if (partial.reduceMotion !== undefined) {
        localStorage.setItem(STORAGE_KEYS.REDUCE_MOTION, String(next.reduceMotion));
      }
      if (partial.increaseContrast !== undefined) {
        localStorage.setItem(STORAGE_KEYS.INCREASE_CONTRAST, String(next.increaseContrast));
      }
      return next;
    });
  }, []);

  return (
    <ThemeContext.Provider
      value={{
        mode,
        resolvedTheme,
        accent,
        accessibility,
        setMode,
        setAccent,
        updateAccessibility,
      }}
    >
      {children}
    </ThemeContext.Provider>
  );
};

export function useTheme(): ThemeContextType {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("useTheme must be used within a ThemeProvider");
  }
  return ctx;
}
