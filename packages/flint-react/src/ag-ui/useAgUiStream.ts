import { useEffect, useRef, useCallback } from 'react';
import type { AgUiEvent } from './AgUiEventHandlers';

interface UseAgUiStreamOptions {
  endpoint: string;
  applicationId: string;
  jwt: string;
  onEvent: (event: AgUiEvent) => void;
  enabled?: boolean;
}

export function useAgUiStream({
  endpoint,
  applicationId,
  jwt,
  onEvent,
  enabled = true,
}: UseAgUiStreamOptions): void {
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  const abortRef = useRef<AbortController | null>(null);

  const connect = useCallback(() => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    const url = `${endpoint}/realtime/v1/sse?application_id=${encodeURIComponent(applicationId)}`;

    fetch(url, {
      headers: {
        Accept: 'text/event-stream',
        Authorization: `Bearer ${jwt}`,
      },
      signal: controller.signal,
    })
      .then(async (res) => {
        if (!res.ok || !res.body) return;
        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = '';

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split('\n');
          buffer = lines.pop() ?? '';

          let dataLine = '';
          for (const line of lines) {
            if (line.startsWith('data: ')) {
              dataLine = line.slice(6);
            } else if (line === '') {
              if (dataLine) {
                try {
                  const event = JSON.parse(dataLine) as AgUiEvent;
                  onEventRef.current(event);
                } catch {
                  // skip malformed SSE data
                }
                dataLine = '';
              }
            }
          }
        }
      })
      .catch(() => {
        // connection closed or aborted — normal teardown
      });
  }, [endpoint, applicationId, jwt]);

  useEffect(() => {
    if (!enabled) return;
    connect();
    return () => {
      abortRef.current?.abort();
    };
  }, [enabled, connect]);
}
