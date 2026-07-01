import React, { useMemo } from 'react';
import { SurfaceContext } from './SurfaceContext';
import { useSurface } from './useSurface';
import { useFlintContext } from '../provider/FlintContext';
import { FlintAgUiAdapter } from '../ag-ui/FlintAgUiAdapter';
import type { A2uiComponentSpec } from '../ag-ui/AgUiEventHandlers';

interface FlintSurfaceProps {
  surfaceId: string;
  className?: string;
}

function RenderComponent({ spec }: { spec: A2uiComponentSpec }): React.ReactElement | null {
  const { resolveComponent } = useFlintContext();
  const Component = resolveComponent(spec.slug);

  if (!Component) {
    return (
      <div
        data-flint-unknown-component={spec.slug}
        role="alert"
        aria-label={`Unknown component: ${spec.slug}`}
      />
    );
  }

  const children =
    spec.children && spec.children.length > 0
      ? spec.children.map((child) => <RenderComponent key={child.id} spec={child} />)
      : undefined;

  return <Component {...spec.props}>{children}</Component>;
}

export function FlintSurface({ surfaceId, className }: FlintSurfaceProps): React.ReactElement {
  const adapter = useMemo(() => new FlintAgUiAdapter(), []);
  const state = useSurface(surfaceId, adapter);

  return (
    <SurfaceContext.Provider value={{ surfaceId, ...state }}>
      <section
        data-flint-surface={surfaceId}
        className={className}
        aria-busy={state.loading}
        aria-label={`Flint surface: ${surfaceId}`}
      >
        {state.error && (
          <div role="alert" data-flint-error>
            {state.error}
          </div>
        )}
        {state.components.map((spec) => (
          <RenderComponent key={spec.id} spec={spec} />
        ))}
      </section>
    </SurfaceContext.Provider>
  );
}
