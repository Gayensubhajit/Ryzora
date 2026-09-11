export const isTauri =
  typeof window !== "undefined" && Boolean((window as any).__TAURI_INTERNALS__);

export async function invokeTauri<T>(
  cmd: string,
  args: Record<string, unknown> = {}
): Promise<T> {
  if (!isTauri) {
    throw new Error(`Tauri IPC is not available in non-Tauri environments (calling ${cmd})`);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}
