# @flint/react — Complete API Reference

React 19 headless component library for Flint A2UI. No CopilotKit or Vercel dependencies.
Bundle target: < 80 KB gzipped.

---

## Installation

```bash
npm install @flint/react
# or
yarn add @flint/react
```

## Provider Setup

```tsx
import { FlintProvider } from '@flint/react';

function App() {
  return (
    <FlintProvider
      catalogUrl="/a2ui/v1/catalog/flint-base/1.0"
      gatewayUrl="https://api.example.com"
      bearerToken={userToken}
    >
      {children}
    </FlintProvider>
  );
}
```

### FlintProvider Props

| Prop | Type | Required | Description |
|---|---|---|---|
| `catalogUrl` | `string` | ✅ | Full URL to the A2UI catalog JSON |
| `gatewayUrl` | `string` | ✅ | Base URL for fdb-gateway REST API |
| `bearerToken` | `string \| (() => string)` | ✅ | JWT for authenticated requests |
| `onError` | `(err: Error) => void` | — | Global error handler |

---

## FlintSurface

Subscribes to an AG-UI SSE run and renders the assembled A2UI surface.

```tsx
import { FlintSurface } from '@flint/react';

<FlintSurface
  surfaceId="orders-view"
  runId="run-abc123"
  fallback={<LoadingSpinner />}
  onSurfaceReady={(surface) => console.log('ready', surface)}
/>
```

### FlintSurface Props

| Prop | Type | Required | Description |
|---|---|---|---|
| `surfaceId` | `string` | ✅ | Surface identifier matching the A2UI `createSurface` message |
| `runId` | `string` | ✅ | AG-UI run ID; subscribes to `/agents/v1/{runId}/events` |
| `fallback` | `ReactNode` | — | Shown while loading |
| `onSurfaceReady` | `(surface: A2uiSurface) => void` | — | Called when first `updateComponents` arrives |

---

## Individual Components

All components are available as named exports:

```tsx
import {
  DataGrid, DataTable, Form, Button, Modal,
  Tabs, Sidebar, Alert, EmptyState,
  // ...all 55 slugs as PascalCase exports
} from '@flint/react';
```

### DataGrid

```tsx
<DataGrid
  columns={[
    { name: 'id',     type: 'uuid',   sortable: true, hidden: true },
    { name: 'status', type: 'text',   sortable: true },
    { name: 'total',  type: 'number', sortable: true, format: 'currency' },
  ]}
  data={rows}
  pagination={{ pageSize: 25, totalRows: 412 }}
  onRowClick={(row) => navigate(`/orders/${row.id}`)}
  onSort={(col, dir) => setSortState({ col, dir })}
  loading={isLoading}
  emptyState={<EmptyState title="No orders" />}
/>
```

### Form

```tsx
<Form
  fields={[
    { name: 'email',    type: 'email',  label: 'Email',    required: true },
    { name: 'role',     type: 'select', label: 'Role',     options: roleOptions },
    { name: 'notes',    type: 'textarea', label: 'Notes' },
  ]}
  onSubmit={async (data) => { await createUser(data); }}
  submitLabel="Create User"
  loading={isSubmitting}
/>
```

### Button

```tsx
<Button
  variant="primary"     // primary | secondary | outline | ghost | destructive
  size="md"             // sm | md | lg
  loading={isLoading}
  disabled={!isValid}
  onClick={handleSubmit}
>
  Submit Order
</Button>
```

---

## useFlint Hook

Access catalog, registry, and gateway client from any component:

```tsx
import { useFlint } from '@flint/react';

function MyComponent() {
  const { catalog, gateway, assembleSurface } = useFlint();

  // List components
  const components = catalog.listComponents({ category: 'input' });

  // Assemble a surface
  const surface = await assembleSurface({
    eventType: 'record.select',
    eventContext: { table: 'orders', record_id: orderId },
  });

  // Direct REST call
  const bindings = await gateway.getBindings('public', 'orders');
}
```

### useFlint Return Shape

```ts
interface FlintContext {
  catalog: {
    listComponents(opts?: { category?: string; appId?: string }): FlintComponent[];
    getComponent(slug: string): FlintComponent | undefined;
    search(query: string, limit?: number): Promise<FlintComponent[]>;
  };
  gateway: {
    getBindings(schema: string, table: string): Promise<Binding[]>;
    getComponent(slug: string): Promise<FlintComponent>;
    assembleSurface(ctx: AssemblyContext): Promise<A2uiSurface>;
  };
  assembleSurface(ctx: AssemblyContext): Promise<A2uiSurface>;
}
```

---

## registerFlintComponent

Register a custom renderer for a component slug:

```tsx
import { registerFlintComponent } from '@flint/react';

registerFlintComponent('my-custom-widget', {
  render: (props, children) => (
    <div className="my-widget" data-flint-component="my-custom-widget">
      {children}
    </div>
  ),
});
```

---

## exportDesignSyncTokens

Export the active catalog's design tokens in W3C 2024 format for Claude Design `/design-sync`:

```tsx
import { exportDesignSyncTokens } from '@flint/react';

const tokens = await exportDesignSyncTokens({ catalogUrl });
// Returns W3C Design Token JSON, suitable for /design-sync
```

---

## TypeScript Types

```ts
import type {
  FlintComponent,
  A2uiSurface,
  A2uiMessage,
  AssemblyContext,
  Binding,
  FlintColumn,
  FlintField,
} from '@flint/react';

// FlintComponent
interface FlintComponent {
  id: string;
  slug: string;
  category: string;
  primitiveType: string;    // PascalCase, e.g. "DataGrid"
  schema: JSONSchema;
  description?: string;
  renderers: { react?: string; flutter?: string; htmx?: string };
}

// AssemblyContext
interface AssemblyContext {
  eventType: string;
  eventContext?: Record<string, unknown>;
  applicationId?: string;
}
```
