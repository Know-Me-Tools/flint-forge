import type { AgUiEvent, A2uiSurfaceEvent } from './AgUiEventHandlers';
import { isA2uiSurfaceEvent } from './AgUiEventHandlers';

export type SurfaceEventHandler = (event: A2uiSurfaceEvent['value']) => void;

export class FlintAgUiAdapter {
  private surfaceHandlers = new Map<string, Set<SurfaceEventHandler>>();

  handleEvent(event: AgUiEvent): void {
    if (isA2uiSurfaceEvent(event)) {
      const { surfaceId } = event.value;
      const handlers = this.surfaceHandlers.get(surfaceId);
      if (handlers) {
        for (const handler of handlers) {
          handler(event.value);
        }
      }
    }
  }

  subscribeSurface(surfaceId: string, handler: SurfaceEventHandler): () => void {
    if (!this.surfaceHandlers.has(surfaceId)) {
      this.surfaceHandlers.set(surfaceId, new Set());
    }
    this.surfaceHandlers.get(surfaceId)!.add(handler);
    return () => {
      this.surfaceHandlers.get(surfaceId)?.delete(handler);
    };
  }
}
