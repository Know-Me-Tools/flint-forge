import React, { createContext, useContext } from 'react';
import type { FlintCatalogEntry, FlintComponentOverrides } from '../registry/FlintRegistry';

export interface FlintContextValue {
  endpoint: string;
  applicationId: string;
  jwt: string;
  catalog: FlintCatalogEntry[];
  overrides: FlintComponentOverrides;
  tokens: Record<string, string>;
  resolveComponent: (slug: string) => React.ComponentType<Record<string, unknown>> | undefined;
}

export const FlintContext = createContext<FlintContextValue | null>(null);

export function useFlintContext(): FlintContextValue {
  const ctx = useContext(FlintContext);
  if (!ctx) {
    throw new Error('useFlintContext must be called inside <FlintProvider>');
  }
  return ctx;
}
