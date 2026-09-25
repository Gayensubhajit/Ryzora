/**
 * Yields browser rendering frames (double rAF) to ensure React state commits
 * and paints to the screen before potentially expensive native IPC commands begin.
 */
export const yieldFrame = (): Promise<void> =>
  new Promise((resolve) => {
    if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
      window.requestAnimationFrame(() => {
        window.requestAnimationFrame(() => resolve());
      });
    } else {
      setTimeout(resolve, 16);
    }
  });
