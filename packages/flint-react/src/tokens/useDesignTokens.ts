import { useEffect, useRef } from 'react';
import useSWR from 'swr';
import { injectTokens, removeTokens } from './FlintTokens';

interface CatalogTokenBundle {
  tokens: Record<string, string>;
}

async function fetchCatalogTokens(url: string, jwt: string): Promise<Record<string, string>> {
  const res = await fetch(url, {
    headers: { Authorization: `Bearer ${jwt}` },
  });
  if (!res.ok) return {};
  const data = (await res.json()) as CatalogTokenBundle;
  return data.tokens ?? {};
}

export function useDesignTokens(
  endpoint: string,
  applicationId: string,
  jwt: string,
  overrideTokens: Record<string, string>,
  rootRef: React.RefObject<HTMLElement | null>,
): void {
  const catalogUrl = `${endpoint}/a2ui/v1/catalog/flint-base/1.0`;

  const { data: remoteTokens } = useSWR(
    jwt ? [catalogUrl, jwt] : null,
    ([url, token]) => fetchCatalogTokens(url, token),
    { revalidateOnFocus: false },
  );

  const mergedRef = useRef<Record<string, string>>({});

  useEffect(() => {
    mergedRef.current = { ...(remoteTokens ?? {}), ...overrideTokens };
    const el = rootRef.current;
    if (!el) return;
    injectTokens(el, mergedRef.current);
    return () => {
      removeTokens(el);
    };
  }, [remoteTokens, overrideTokens, rootRef]);
}
