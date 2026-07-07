import React, { useEffect, useRef } from 'react';
import { FlintProvider, FlintSurface, Alert } from '@flint/react';

interface AgentChatProps {
  runId: string;
  gatewayUrl: string;
  bearerToken: string;
  catalogUrl?: string;
}

/**
 * AgentChat — subscribes to an AG-UI SSE run and renders the A2UI surface
 * assembled by the agent in real-time.
 *
 * The FlintSurface component handles:
 * - Connecting to GET /agents/v1/{runId}/events
 * - Processing RunStarted, TextMessage*, ToolCall*, Custom(a2ui:surface) events
 * - Rendering the assembled surface using the Flint catalog
 */
export function AgentChat({
  runId,
  gatewayUrl,
  bearerToken,
  catalogUrl = `${gatewayUrl}/a2ui/v1/catalog/flint-base/1.0`,
}: AgentChatProps) {
  return (
    <FlintProvider
      catalogUrl={catalogUrl}
      gatewayUrl={gatewayUrl}
      bearerToken={bearerToken}
    >
      <div className="agent-chat" data-run-id={runId}>
        <FlintSurface
          runId={runId}
          surfaceId={`run-${runId}`}
          fallback={
            <div className="agent-chat__loading">
              <span className="loading-spinner" />
              <span>Connecting to agent…</span>
            </div>
          }
        />
      </div>
    </FlintProvider>
  );
}

/**
 * Usage:
 *
 * // 1. Start a run
 * const { run_id } = await fetch('/agents/v1/runs', {
 *   method: 'POST',
 *   headers: { Authorization: `Bearer ${token}` },
 * }).then(r => r.json());
 *
 * // 2. Render the chat surface (subscribes to SSE automatically)
 * <AgentChat
 *   runId={run_id}
 *   gatewayUrl="https://api.example.com"
 *   bearerToken={userToken}
 * />
 *
 * // 3. Publish events to the run from your agent code
 * await fetch(`/agents/v1/${run_id}/events`, {
 *   method: 'POST',
 *   headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
 *   body: JSON.stringify({
 *     event: {
 *       type: 'TextMessageStart',
 *       message_id: crypto.randomUUID(),
 *       role: 'assistant',
 *     }
 *   }),
 * });
 */
