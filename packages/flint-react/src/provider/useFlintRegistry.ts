/**
 * useFlintRegistry.ts
 *
 * Unified hook that combines:
 *  1. The runtime FlintCatalogEntry registry (entries registered via
 *     `registerFlintComponent` / `registerBaseComponents`).
 *  2. The static A2UI slug → React component map from `slugMap.ts`.
 *
 * Usage:
 *   const { listComponents, getComponent, search, fromSlug, slugMap } = useFlintRegistry();
 */
import { useFlint } from './useFlint';
import { fromSlug, SLUG_MAP } from '../registry/slugMap';
import type { SlugComponent } from '../registry/slugMap';
import type { FlintCatalogEntry } from '../registry/FlintRegistry';

// ── Public interface ──────────────────────────────────────────────────────

export interface FlintRegistryHook {
  /**
   * Returns all `FlintCatalogEntry` objects that have been registered with
   * `registerFlintComponent` at the time the `FlintProvider` rendered.
   */
  listComponents: () => FlintCatalogEntry[];

  /**
   * Looks up a single `FlintCatalogEntry` by its slug.
   * Returns `undefined` when the slug has not been registered at runtime
   * (use `fromSlug` to fall back to the static slug map instead).
   */
  getComponent: (slug: string) => FlintCatalogEntry | undefined;

  /**
   * Filters registered catalog entries whose `slug` or `description`
   * contains the search query (case-insensitive).
   */
  search: (query: string) => FlintCatalogEntry[];

  /**
   * Maps an A2UI catalog slug directly to a React component from the
   * static slug map.  Returns `undefined` for unknown slugs.
   *
   * @example
   * const Grid = fromSlug('grid');
   */
  fromSlug: (slug: string) => SlugComponent | undefined;

  /**
   * The full static slug map record.  Useful for iterating all 55 catalog
   * slots or performing batch lookups.
   */
  slugMap: typeof SLUG_MAP;
}

// ── Hook implementation ───────────────────────────────────────────────────

export function useFlintRegistry(): FlintRegistryHook {
  const { catalog } = useFlint();

  const listComponents = (): FlintCatalogEntry[] => catalog;

  const getComponent = (slug: string): FlintCatalogEntry | undefined =>
    catalog.find((entry) => entry.slug === slug);

  const search = (query: string): FlintCatalogEntry[] => {
    const q = query.toLowerCase();
    return catalog.filter(
      (entry) =>
        entry.slug.includes(q) ||
        (entry.description ?? '').toLowerCase().includes(q),
    );
  };

  return {
    listComponents,
    getComponent,
    search,
    fromSlug,
    slugMap: SLUG_MAP,
  };
}
