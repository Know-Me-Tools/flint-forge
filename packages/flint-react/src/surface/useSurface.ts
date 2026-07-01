import { useReducer, useEffect, useRef } from 'react';
import type { A2uiComponentSpec } from '../ag-ui/AgUiEventHandlers';
import type { FlintAgUiAdapter } from '../ag-ui/FlintAgUiAdapter';

interface SurfaceReducerState {
  components: A2uiComponentSpec[];
  loading: boolean;
  error: string | null;
}

type SurfaceAction =
  | { type: 'create'; components: A2uiComponentSpec[] }
  | { type: 'update'; components: A2uiComponentSpec[] }
  | { type: 'delete' }
  | { type: 'loading' }
  | { type: 'error'; message: string };

function reducer(state: SurfaceReducerState, action: SurfaceAction): SurfaceReducerState {
  switch (action.type) {
    case 'create':
      return { components: action.components, loading: false, error: null };
    case 'update':
      return { ...state, components: action.components, loading: false };
    case 'delete':
      return { components: [], loading: false, error: null };
    case 'loading':
      return { ...state, loading: true, error: null };
    case 'error':
      return { ...state, loading: false, error: action.message };
  }
}

export function useSurface(surfaceId: string, adapter: FlintAgUiAdapter): SurfaceReducerState {
  const [state, dispatch] = useReducer(reducer, {
    components: [],
    loading: false,
    error: null,
  });

  const adapterRef = useRef(adapter);
  adapterRef.current = adapter;

  useEffect(() => {
    return adapterRef.current.subscribeSurface(surfaceId, (event) => {
      switch (event.action) {
        case 'createSurface':
          dispatch({ type: 'create', components: event.components ?? [] });
          break;
        case 'updateComponents':
          dispatch({ type: 'update', components: event.components ?? [] });
          break;
        case 'deleteSurface':
          dispatch({ type: 'delete' });
          break;
      }
    });
  }, [surfaceId]);

  return state;
}
