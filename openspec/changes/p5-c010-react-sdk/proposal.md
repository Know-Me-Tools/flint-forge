# p5-c010 — Flint React SDK (`@flint/react`)

**Phase:** 5 — Flint A2UI Component Registry  
**Priority:** P1 (MVP SDK surface — enables web clients without CopilotKit dependency)  
**Depends on:** p5-c002 (base components seed — defines the 63 component slugs), p5-c006 (REST API)  
**Blocks:** p5-c013 (OpenDesign integration), p5-c015 (Claude Design skill)

---

## What this change delivers

A React 19 component library (`@flint/react`) that implements all 63 Flint A2UI component definitions as headless primitives, with a Zod-schema-backed registry, component override system, and design token injection. This is the web-client equivalent of CopilotKit, without the vendor lock-in.

---

## Architecture

### Design Philosophy (Research-Validated)

Three research patterns combined:
1. **Radix-style headless primitives** (from assistant-ui architecture) — behavior and a11y logic separated from markup; styling via CSS custom properties only
2. **Zod schema registry** (from Tambo pattern) — each component declares a typed `propsSchema`; types map directly to `flint_a2ui.components.schema` JSONB
3. **Props map override** (from Thesys/Crayon pattern) — `<FlintProvider components={{ DataGrid: MyGrid }} />` for application-level overrides

### Package Structure

```
packages/flint-react/                   # npm: @flint/react
├── src/
│   ├── provider/
│   │   ├── FlintProvider.tsx           # Root provider: AG-UI connection + component overrides + design tokens
│   │   ├── FlintContext.tsx            # React context: catalog, overrides, token resolver
│   │   └── useFlint.ts                 # Hook: access catalog, dispatch, surface state
│   │
│   ├── surface/
│   │   ├── FlintSurface.tsx            # Renders an A2UI surface (createSurface → component tree)
│   │   ├── useSurface.ts               # Manages surface state, AG-UI event subscription
│   │   └── SurfaceContext.tsx
│   │
│   ├── registry/
│   │   ├── FlintRegistry.ts            # Component registry: slug → (schema, render)
│   │   ├── registerComponent.ts        # registerFlintComponent() — user-space custom registration
│   │   └── ComponentSchema.ts          # Zod schema → A2UI JSON Schema bridge
│   │
│   ├── components/
│   │   ├── layout/                     # Stack, Card, Grid, Split, Tabs, Accordion, ...
│   │   ├── data-display/               # DataGrid, Table, Chart, Timeline, Kanban, ...
│   │   ├── input/                      # Form, TextField, Select, DatePicker, JsonEditor, ...
│   │   ├── action/                     # Button, Confirm, Wizard, BulkAction, ...
│   │   ├── agent/                      # AgentChat, ToolCall, StreamingText, Decision, ...
│   │   └── navigation/                 # NavMenu, CommandPalette, FilterBar, ...
│   │
│   ├── tokens/
│   │   ├── FlintTokens.ts              # CSS custom property injection from design_systems JSONB
│   │   └── useDesignTokens.ts          # Hook: resolve tokens for current application_id
│   │
│   ├── ag-ui/
│   │   ├── FlintAgUiAdapter.ts         # Connects to fdb-gateway AG-UI SSE endpoint
│   │   ├── useAgUiStream.ts            # Hook: subscribe to AG-UI event stream
│   │   └── AgUiEventHandlers.ts        # Dispatch A2UI surface updates from AG-UI Custom events
│   │
│   └── index.ts
│
├── package.json                         # name: "@flint/react", peerDeps: react@^19, react-dom@^19
└── SKILL.md                             # Claude Code skill: knows all Flint component slugs and APIs
```

### Core Provider API

```tsx
import { FlintProvider, FlintSurface } from '@flint/react';

function App() {
  return (
    <FlintProvider
      endpoint="https://api.myapp.com"   // fdb-gateway base URL
      applicationId="app-uuid"
      jwt={userJwt}
      components={{                       // Optional: override specific components
        DataGrid: MyCustomDataGrid,
        Chart: MyChartLibrary,
      }}
      tokens={{                           // Optional: override design tokens
        '--flint-color-primary': '#6366f1',
        '--flint-font-sans': 'Inter, sans-serif',
      }}
    >
      <FlintSurface surfaceId="main" />
    </FlintProvider>
  );
}
```

### Component Registration (User-Space Extension)

```tsx
import { registerFlintComponent } from '@flint/react';
import { z } from 'zod';

registerFlintComponent({
  slug: 'my-custom-chart',                // Must match flint_a2ui.components.slug
  propsSchema: z.object({
    data_source: z.string(),
    chart_type: z.enum(['bar', 'line', 'pie']),
    metric: z.string(),
  }),
  component: ({ data_source, chart_type, metric }) => (
    <MyCompanyChart source={data_source} type={chart_type} field={metric} />
  ),
});
```

### AG-UI SSE Integration

```tsx
// FlintAgUiAdapter connects to:
// GET /graphql (subscriptions) OR
// GET /realtime/v1/sse (AG-UI event stream via fdb-gateway)
//
// Handles AG-UI event types:
// - RunStarted → surface loading state
// - Custom { type: "a2ui:surface" } → parse A2UI message, update surface
// - TextMessageContent → StreamingText component update
// - ToolCallStart/End → ToolCall component lifecycle
// - RunFinished → surface ready state
// - RunError → error boundary
```

### Design Token System

Design tokens are resolved from `GET /a2ui/v1/catalog/:catalog_id` which includes the design_systems token bundle. They are injected as CSS custom properties on the `FlintProvider` root element:

```css
/* Injected by FlintProvider */
:root[data-flint-app="app-uuid"] {
  --flint-color-primary: oklch(68% 0.21 250);
  --flint-color-surface: oklch(98% 0 0);
  --flint-text-base: clamp(1rem, 0.92rem + 0.4vw, 1.125rem);
  --flint-duration-normal: 300ms;
  --flint-ease-out-expo: cubic-bezier(0.16, 1, 0.3, 1);
}
```

All Flint components use ONLY these custom properties for styling (no hardcoded colors or sizes).

### Override Schema in Database

Each application can store component overrides in `flint_a2ui.components` with `application_id` set (non-null). The `resolve_components()` function returns base + app-specific components. The React SDK fetches this at mount and merges overrides into the registry.

---

## Component Implementation Pattern

Each component is headless: it implements behavior and renders semantic HTML. Users provide styling via CSS or Tailwind targeting the `data-flint-*` attributes:

```tsx
// packages/flint-react/src/components/data-display/DataGrid.tsx
export function DataGrid({
  columns,
  data_source,
  pagination = { page_size: 25 },
  row_actions = [],
}: DataGridProps) {
  const { resolveDataSource } = useFlint();
  const [data, loading] = useDataSource(data_source, pagination);

  return (
    <div data-flint-component="data-grid" role="region" aria-busy={loading}>
      <table data-flint-part="table">
        <thead>
          <tr>
            {columns.map(col => (
              <th key={col.field} data-flint-sortable={col.sortable}>
                {col.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {data.rows.map((row, i) => (
            <tr key={i} data-flint-part="row">
              {columns.map(col => (
                <td key={col.field}>{renderCell(row[col.field], col)}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <FlintPagination {...pagination} total={data.total} />
    </div>
  );
}
```

---

## Dependencies

```json
{
  "peerDependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0"
  },
  "dependencies": {
    "zod": "^3.22.0",
    "@radix-ui/react-slot": "^1.1.0",
    "swr": "^2.3.0"
  },
  "devDependencies": {
    "typescript": "^5.5.0",
    "tsup": "^8.0.0"
  }
}
```

No dependency on `@copilotkit/*`, `@ag-ui/*`, `ai` (Vercel), or any vendor AI SDK.

---

## Gate Tests

- [ ] `<FlintProvider>` renders without error given valid endpoint + applicationId
- [ ] AG-UI `Custom { type: "a2ui:surface" }` event renders `DataGrid` component
- [ ] Component override via `components={{ DataGrid: MyGrid }}` renders MyGrid instead
- [ ] Design tokens appear as CSS custom properties on root element
- [ ] `registerFlintComponent` adds custom slug to registry; AI can invoke it
- [ ] A11y: all components pass axe-core checks
- [ ] Bundle size: `@flint/react` gzipped JS < 80kb (microsite budget from performance.md)
