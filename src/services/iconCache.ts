/**
 * iconCache.ts — Phase 24.2
 *
 * High-performance, centralized icon resolution cache.
 *
 * Architecture:
 *   1. Pure in-memory caching (Map<string, ResolvedIcon | null>)
 *   2. Batch IPC: aggregates requested app IDs and hints into single IPC calls
 *   3. Data URIs (Base64 PNG/XPM) & inline SVGs — zero reliance on asset protocol or convertFileSrc
 *   4. Instant reactive re-rendering via subscription
 */

import { useState, useEffect } from "react";
import { isTauri, invokeTauri } from "./tauri.ts";

export interface ResolvedIcon {
  dataUri?: string;
  svgContent?: string;
  fileUrl?: string;
}

interface BackendIconInfo {
  icon_svg_content?: string | null;
  icon_data_uri?: string | null;
  icon_path?: string | null;
  icon_name?: string | null;
  desktop_file?: string;
  name?: string;
}

export interface IconRequestItem {
  id: string;
  icon_name?: string | null;
  icon_path?: string | null;
}

// ── Global cache ─────────────────────────────────────────────────────────────
// undefined = pending/unknown; null = resolved, no icon; ResolvedIcon = found

const iconCache = new Map<string, ResolvedIcon | null>();
const pendingIds = new Set<string>();
const subscribers = new Map<string, Set<() => void>>();

function notifySubscribers(id: string): void {
  subscribers.get(id)?.forEach((cb) => cb());
}

// ── Batch queue ───────────────────────────────────────────────────────────────

let batchTimer: ReturnType<typeof setTimeout> | null = null;
const batchQueue = new Map<string, IconRequestItem>();

async function flushBatch(): Promise<void> {
  batchTimer = null;
  const itemsToFetch: IconRequestItem[] = [];

  for (const [id, req] of batchQueue.entries()) {
    if (!iconCache.has(id) && !pendingIds.has(id)) {
      pendingIds.add(id);
      itemsToFetch.push(req);
    }
  }
  batchQueue.clear();

  if (itemsToFetch.length === 0) return;

  try {
    const results = await invokeTauri<Record<string, BackendIconInfo>>(
      "resolve_desktop_icon_batch",
      {
        items: itemsToFetch.map((it) => ({
          id: it.id,
          icon_name: it.icon_name ?? null,
          icon_path: it.icon_path ?? null,
        })),
      }
    );

    for (const req of itemsToFetch) {
      pendingIds.delete(req.id);
      const info = results?.[req.id];
      const resolved = info ? backendToResolved(info) : null;
      iconCache.set(req.id, resolved);
      notifySubscribers(req.id);
    }
  } catch (err) {
    console.warn("[iconCache] batch resolve failed:", err);
    for (const req of itemsToFetch) {
      pendingIds.delete(req.id);
      if (!iconCache.has(req.id)) {
        iconCache.set(req.id, null);
      }
      notifySubscribers(req.id);
    }
  }
}

function enqueueBatch(items: IconRequestItem[]): void {
  let hasNew = false;
  for (const item of items) {
    if (!iconCache.has(item.id) && !pendingIds.has(item.id)) {
      if (!batchQueue.has(item.id)) {
        batchQueue.set(item.id, item);
        hasNew = true;
      }
    }
  }
  if (hasNew && batchTimer === null) {
    batchTimer = setTimeout(flushBatch, 30);
  }
}

function backendToResolved(info: BackendIconInfo): ResolvedIcon | null {
  if (info.icon_svg_content) {
    return { svgContent: info.icon_svg_content };
  }
  if (info.icon_data_uri) {
    return { dataUri: info.icon_data_uri };
  }
  return null;
}

// ── Public API ────────────────────────────────────────────────────────────────

export function prefetchIconsForPage(
  items: Array<{ id: string; icon_name?: string | null; icon_path?: string | null }>
): void {
  if (!isTauri) return;
  const needed: IconRequestItem[] = [];
  for (const item of items) {
    const targetId = item.id.toLowerCase().trim();
    if (!iconCache.has(targetId)) {
      needed.push({
        id: targetId,
        icon_name: item.icon_name ?? null,
        icon_path: item.icon_path ?? null,
      });
    }
  }
  if (needed.length > 0) {
    enqueueBatch(needed);
  }
}

export function useIconCached(
  packageId: string,
  iconName?: string | null,
  iconPath?: string | null
): ResolvedIcon | null | undefined {
  const [, forceUpdate] = useState(0);
  const targetId = packageId.toLowerCase().trim();

  const cached = iconCache.get(targetId);

  useEffect(() => {
    if (!targetId || iconCache.has(targetId)) return;

    const cb = () => forceUpdate((n) => n + 1);
    if (!subscribers.has(targetId)) subscribers.set(targetId, new Set());
    subscribers.get(targetId)!.add(cb);

    if (isTauri && !pendingIds.has(targetId)) {
      enqueueBatch([{ id: targetId, icon_name: iconName ?? null, icon_path: iconPath ?? null }]);
    } else if (!isTauri) {
      iconCache.set(targetId, null);
      notifySubscribers(targetId);
    }

    return () => {
      subscribers.get(targetId)?.delete(cb);
    };
  }, [targetId, iconName, iconPath]);

  return cached;
}

export function clearIconCache(): void {
  iconCache.clear();
  pendingIds.clear();
  batchQueue.clear();
  if (batchTimer !== null) {
    clearTimeout(batchTimer);
    batchTimer = null;
  }
}
