export type AgUiEventType =
  | 'RunStarted'
  | 'RunFinished'
  | 'RunError'
  | 'TextMessageStart'
  | 'TextMessageContent'
  | 'TextMessageEnd'
  | 'ToolCallStart'
  | 'ToolCallEnd'
  | 'Custom';

export interface AgUiEvent {
  type: AgUiEventType;
  [key: string]: unknown;
}

export interface A2uiSurfaceEvent {
  type: 'Custom';
  name: 'a2ui:surface';
  value: {
    action: 'createSurface' | 'updateComponents' | 'deleteSurface';
    surfaceId: string;
    components?: A2uiComponentSpec[];
  };
}

export interface A2uiComponentSpec {
  id: string;
  slug: string;
  props: Record<string, unknown>;
  children?: A2uiComponentSpec[];
}

export function isA2uiSurfaceEvent(event: AgUiEvent): event is A2uiSurfaceEvent {
  return (
    event.type === 'Custom' &&
    (event as Record<string, unknown>)['name'] === 'a2ui:surface'
  );
}
