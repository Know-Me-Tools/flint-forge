import React from 'react';
import type { ZodTypeAny } from 'zod';

export interface FlintCatalogEntry {
  slug: string;
  category: string;
  primitiveType: string;
  propsSchema: ZodTypeAny;
  component: React.ComponentType<Record<string, unknown>>;
  description?: string;
}

export type FlintComponentOverrides = Record<string, React.ComponentType<Record<string, unknown>>>;

const registry = new Map<string, FlintCatalogEntry>();

export function registerFlintComponent(entry: FlintCatalogEntry): void {
  registry.set(entry.slug, entry);
}

export function resolveFlintComponent(
  slug: string,
  overrides: FlintComponentOverrides,
): React.ComponentType<Record<string, unknown>> | undefined {
  const Override = overrides[slug];
  if (Override) return Override;
  return registry.get(slug)?.component;
}

export function getAllRegistered(): FlintCatalogEntry[] {
  return Array.from(registry.values());
}

export function getRegistrySize(): number {
  return registry.size;
}
