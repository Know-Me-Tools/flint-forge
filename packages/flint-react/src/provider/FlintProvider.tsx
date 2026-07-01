import React, { useRef, useMemo, useCallback } from 'react';
import { FlintContext } from './FlintContext';
import { FlintAgUiAdapter } from '../ag-ui/FlintAgUiAdapter';
import { useAgUiStream } from '../ag-ui/useAgUiStream';
import { useDesignTokens } from '../tokens/useDesignTokens';
import { getAllRegistered, resolveFlintComponent } from '../registry/FlintRegistry';
import type { FlintComponentOverrides } from '../registry/FlintRegistry';

interface FlintProviderProps {
  endpoint: string;
  applicationId: string;
  jwt: string;
  components?: FlintComponentOverrides;
  tokens?: Record<string, string>;
  children: React.ReactNode;
}

export function FlintProvider({
  endpoint,
  applicationId,
  jwt,
  components: overrides = {},
  tokens = {},
  children,
}: FlintProviderProps): React.ReactElement {
  const rootRef = useRef<HTMLDivElement>(null);
  const adapterRef = useRef<FlintAgUiAdapter>(new FlintAgUiAdapter());

  useDesignTokens(endpoint, applicationId, jwt, tokens, rootRef);

  useAgUiStream({
    endpoint,
    applicationId,
    jwt,
    onEvent: (event) => adapterRef.current.handleEvent(event),
  });

  const catalog = useMemo(() => getAllRegistered(), []);

  const resolveComponent = useCallback(
    (slug: string) => resolveFlintComponent(slug, overrides),
    [overrides],
  );

  const ctx = useMemo(
    () => ({ endpoint, applicationId, jwt, catalog, overrides, tokens, resolveComponent }),
    [endpoint, applicationId, jwt, catalog, overrides, tokens, resolveComponent],
  );

  return (
    <FlintContext.Provider value={ctx}>
      <div
        ref={rootRef}
        data-flint-app={applicationId}
        data-flint-provider
      >
        {children}
      </div>
    </FlintContext.Provider>
  );
}
