import React, { createContext, useContext } from 'react';
import type { A2uiComponentSpec } from '../ag-ui/AgUiEventHandlers';

export interface SurfaceState {
  surfaceId: string;
  components: A2uiComponentSpec[];
  loading: boolean;
  error: string | null;
}

export const SurfaceContext = createContext<SurfaceState | null>(null);

export function useSurfaceContext(): SurfaceState {
  const ctx = useContext(SurfaceContext);
  if (!ctx) {
    throw new Error('useSurfaceContext must be called inside <FlintSurface>');
  }
  return ctx;
}
