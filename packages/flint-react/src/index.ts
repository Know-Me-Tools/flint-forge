// Provider
export { FlintProvider } from './provider/FlintProvider';
export { FlintContext, useFlintContext } from './provider/FlintContext';
export { useFlint } from './provider/useFlint';

// Surface
export { FlintSurface } from './surface/FlintSurface';
export { SurfaceContext, useSurfaceContext } from './surface/SurfaceContext';
export { useSurface } from './surface/useSurface';

// Registry
export {
  registerFlintComponent,
  resolveFlintComponent,
  getAllRegistered,
  getRegistrySize,
} from './registry/FlintRegistry';
export type { FlintCatalogEntry, FlintComponentOverrides } from './registry/FlintRegistry';
export { zodToA2uiJsonSchema } from './registry/ComponentSchema';

// AG-UI
export { FlintAgUiAdapter } from './ag-ui/FlintAgUiAdapter';
export { useAgUiStream } from './ag-ui/useAgUiStream';
export type { AgUiEvent, A2uiSurfaceEvent, A2uiComponentSpec } from './ag-ui/AgUiEventHandlers';
export { isA2uiSurfaceEvent } from './ag-ui/AgUiEventHandlers';

// Tokens
export { DEFAULT_TOKENS, injectTokens, removeTokens } from './tokens/FlintTokens';
export { useDesignTokens } from './tokens/useDesignTokens';

// Components — layout
export { Stack, Card, Grid, Split, Tabs, Accordion, Scroll, Modal, Drawer } from './components/layout';

// Components — data-display
export { DataGrid, Table, Chart, Timeline, Kanban, Calendar, Metric, Badge } from './components/data-display';

// Components — input
export { Form, TextField, Select, DatePicker, Search, FileUpload, JsonEditor, RichEditor } from './components/input';

// Components — action
export { Button, Confirm, Wizard, BulkAction, ActionBar } from './components/action';

// Components — agent
export { AgentChat, ToolCall, StreamingText, Decision, ProgressLog, Artifact } from './components/agent';

// Components — navigation
export { NavMenu, CommandPalette, FilterBar, Breadcrumb } from './components/navigation';

// Base component registration (call once at app init)
export { registerBaseComponents } from './registry/baseComponents';
