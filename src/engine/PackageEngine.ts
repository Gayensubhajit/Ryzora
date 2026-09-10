/**
 * Ryzora Universal Package Engine — Phase 22
 *
 * Central frontend registry that all PackageProviders register with.
 * The engine de-duplicates packages, aggregates discovery across all
 * registered providers, and routes install/uninstall calls through
 * the typed InstallPipeline wrapper.
 *
 * This is the single source of truth for which providers Ryzora knows about.
 * Individual category views (LockScreenDetailView, WallpaperView, etc.) must
 * never bypass this registry.
 */

import type {
  PackageProvider,
  PackageItem,
  FrontendProviderSummary,
} from "../providers/types.ts";

// ─────────────────────────────────────────────────────────────────────────────
// PackageEngine
// ─────────────────────────────────────────────────────────────────────────────

export class PackageEngine {
  private readonly registry = new Map<string, PackageProvider>();

  /**
   * Registers a provider. If a provider with the same id is already registered,
   * it is silently replaced (useful for hot-reloading in dev).
   */
  register(provider: PackageProvider): void {
    this.registry.set(provider.id, provider);
  }

  /** Removes a provider by id. No-op if not registered. */
  unregister(id: string): void {
    this.registry.delete(id);
  }

  /** Returns the provider with the given id, or undefined if not registered. */
  getProvider(id: string): PackageProvider | undefined {
    return this.registry.get(id);
  }

  /**
   * Returns a summary of all registered providers, useful for the Repository
   * and Provider management views.
   */
  listProviders(): FrontendProviderSummary[] {
    return Array.from(this.registry.values()).map((p) => ({
      id: p.id,
      name: p.name,
      category: p.category,
      enabled: true,
      package_count: 0, // populated lazily on discovery
    }));
  }

  /**
   * Calls `discover()` on every registered provider and returns the merged,
   * de-duplicated package list. Packages with the same `id` from a later
   * provider are silently dropped (first-registered wins).
   *
   * Failures in individual providers are caught and logged; they never
   * prevent other providers from returning their packages.
   */
  async discoverAll(): Promise<PackageItem[]> {
    const seen = new Set<string>();
    const results: PackageItem[] = [];

    for (const provider of this.registry.values()) {
      try {
        const items = await Promise.resolve(provider.discover());
        for (const item of items) {
          const normId = item.id.trim().toLowerCase();
          if (!seen.has(normId)) {
            seen.add(normId);
            results.push(item);
          }
        }
      } catch (err) {
        console.warn(
          `[PackageEngine] Provider '${provider.id}' discover() failed:`,
          err,
        );
      }
    }

    return results;
  }

  /**
   * Discovers packages from providers matching the given category.
   */
  async discoverByCategory(category: string): Promise<PackageItem[]> {
    const all = await this.discoverAll();
    return all.filter(
      (pkg) =>
        pkg.category?.toLowerCase() === category.toLowerCase() ||
        pkg.package_type?.toLowerCase() === category.toLowerCase(),
    );
  }

  /**
   * Finds a package by id across all registered providers.
   */
  async findPackage(packageId: string): Promise<PackageItem | undefined> {
    const normId = packageId.trim().toLowerCase();
    for (const provider of this.registry.values()) {
      try {
        const items = await Promise.resolve(provider.discover());
        const found = items.find((p) => p.id.trim().toLowerCase() === normId);
        if (found) return found;
      } catch {
        // ignore
      }
    }
    return undefined;
  }

  /**
   * Returns the number of registered providers.
   */
  /**
   * Searches packages across all providers implementing `search()`.
   * For providers without `search()`, falls back to filtering their `discover()` results.
   */
  async searchAll(query: string): Promise<PackageItem[]> {
    const q = query.trim().toLowerCase();
    const seen = new Set<string>();
    const results: PackageItem[] = [];

    for (const provider of this.registry.values()) {
      try {
        let items: PackageItem[] = [];
        if (typeof provider.search === "function") {
          items = await Promise.resolve(provider.search(query));
        } else {
          const all = await Promise.resolve(provider.discover());
          items = q
            ? all.filter(
                (p) =>
                  p.title.toLowerCase().includes(q) ||
                  p.id.toLowerCase().includes(q) ||
                  p.description.toLowerCase().includes(q)
              )
            : all;
        }

        for (const item of items) {
          const normId = item.id.trim().toLowerCase();
          if (!seen.has(normId)) {
            seen.add(normId);
            results.push(item);
          }
        }
      } catch (err) {
        console.warn(`[PackageEngine] Provider '${provider.id}' search() failed:`, err);
      }
    }

    return results;
  }

  get providerCount(): number {
    return this.registry.size;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Global singleton
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The global PackageEngine singleton.
 * Import this and call `packageEngine.register(myProvider)` during app startup.
 */
export const packageEngine = new PackageEngine();
