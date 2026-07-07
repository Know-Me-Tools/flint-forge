/**
 * Fetch design tokens from the Flint gateway in W3C Design Token format.
 * Used by Claude Design `/design-sync` and OpenDesign integrations.
 */
export async function exportDesignSyncTokens(opts: {
  gatewayUrl: string;
  systemId: string;
  bearerToken: string;
}): Promise<Record<string, unknown>> {
  const url = `${opts.gatewayUrl}/a2ui/v1/design-systems/${opts.systemId}/tokens`;
  const resp = await fetch(url, {
    headers: { Authorization: `Bearer ${opts.bearerToken}` },
  });
  if (!resp.ok) {
    throw new Error(
      `exportDesignSyncTokens: failed with HTTP ${resp.status} from ${url}`
    );
  }
  return resp.json() as Promise<Record<string, unknown>>;
}
