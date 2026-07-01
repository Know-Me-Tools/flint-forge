# @flint/react — Claude Code Skill

## Package Overview

`@flint/react` is the Flint A2UI React 19 SDK. It provides headless primitive components that render Flint A2UI surfaces received from `fdb-gateway` via AG-UI SSE events.

## Quick Start

```tsx
import { FlintProvider, FlintSurface, registerBaseComponents } from '@flint/react';

// Register all 40 base Flint components at app init
registerBaseComponents();

function App() {
  return (
    <FlintProvider
      endpoint="https://api.myapp.com"
      applicationId="your-app-uuid"
      jwt={userJwt}
    >
      <FlintSurface surfaceId="main" />
    </FlintProvider>
  );
}
```

## All Component Slugs

### Layout
| Slug | Props |
|------|-------|
| `stack` | `direction`, `gap`, `justify`, `align`, `wrap` |
| `card` | `title`, `elevated` |
| `grid` | `columns`, `gap` |
| `split` | `ratio` |
| `tabs` | `items[]` (label, value, content), `defaultValue` |
| `accordion` | `items[]` (label, value, content), `allowMultiple` |
| `scroll` | `maxHeight` |
| `modal` | `open`, `onClose`, `title` |
| `drawer` | `open`, `onClose`, `side` |

### Data Display
| Slug | Props |
|------|-------|
| `data-grid` | `columns[]`, `data[]`, `loading`, `onRowClick`, `rowKey` |
| `table` | `headers[]`, `rows[][]` |
| `chart` | `type` (bar/line), `data[]` (label, value), `title` |
| `timeline` | `events[]` (id, label, timestamp, description) |
| `kanban` | `columns[]` (id, title, cards[]), `renderCard` |
| `calendar` | `year`, `month`, `events[]`, `onDateSelect` |
| `metric` | `label`, `value`, `unit`, `trend` |
| `badge` | `label`, `variant` (default/success/warning/error/info) |

### Input
| Slug | Props |
|------|-------|
| `form` | `fields[]`, `onSubmit`, `submitLabel` |
| `text-field` | `label`, `name`, `value`, `placeholder`, `type`, `required` |
| `select` | `label`, `name`, `options[]`, `value`, `required` |
| `date-picker` | `label`, `name`, `value`, `required` |
| `search` | `value`, `placeholder`, `onSearch` |
| `file-upload` | `label`, `name`, `accept`, `multiple` |
| `json-editor` | `label`, `name`, `value` |
| `rich-editor` | `label`, `name`, `value` |

### Action
| Slug | Props |
|------|-------|
| `button` | `variant`, `size`, `loading`, `onClick` |
| `confirm` | `message`, `onConfirm`, `onCancel`, `confirmLabel`, `cancelLabel` |
| `wizard` | `steps[]` (title, content), `onComplete` |
| `bulk-action` | `selectedCount`, `actions[]` (label, onClick, destructive) |
| `action-bar` | `actions[]` (label, icon, onClick, disabled) |

### Agent
| Slug | Props |
|------|-------|
| `agent-chat` | `messages[]` (id, role, content, timestamp), `onSend`, `loading` |
| `tool-call` | `name`, `status` (pending/running/complete/error), `args`, `result` |
| `streaming-text` | `text`, `streaming` |
| `decision` | `question`, `options[]` (id, label, description), `onSelect` |
| `progress-log` | `entries[]` (id, message, level, timestamp), `title` |
| `artifact` | `type` (code/text/image/file), `content`, `language`, `filename` |

### Navigation
| Slug | Props |
|------|-------|
| `nav-menu` | `items[]` (label, href, onClick, icon, active, children), `orientation` |
| `command-palette` | `open`, `onClose`, `commands[]`, `placeholder` |
| `filter-bar` | `filters[]`, `values`, `onChange`, `onReset` |
| `breadcrumb` | `items[]` (label, href) |

## Registering Custom Components

```tsx
import { registerFlintComponent } from '@flint/react';
import { z } from 'zod';

registerFlintComponent({
  slug: 'my-chart',               // must match flint_a2ui.components.slug
  category: 'data-display',
  primitiveType: 'Chart',
  propsSchema: z.object({
    data_source: z.string(),
    chart_type: z.enum(['bar', 'line', 'pie']),
  }),
  component: ({ data_source, chart_type }) => (
    <MyCompanyChart source={data_source} type={chart_type} />
  ),
});
```

## Design Tokens

All Flint components use `--flint-*` CSS custom properties. Override at provider level:

```tsx
<FlintProvider
  tokens={{
    '--flint-color-primary': '#6366f1',
    '--flint-font-sans': 'Inter, sans-serif',
  }}
>
```

## AG-UI Integration

`FlintProvider` auto-connects to `GET {endpoint}/realtime/v1/sse`. The stream delivers:
- `Custom { name: "a2ui:surface", value: { action, surfaceId, components } }` → updates `<FlintSurface>`
- `TextMessageContent` → updates `<StreamingText>` components
- `ToolCallStart/End` → updates `<ToolCall>` components

## Styling

All components expose `data-flint-component` and `data-flint-part` attributes for CSS targeting:

```css
[data-flint-component="data-grid"] [data-flint-part="row"]:hover {
  background: color-mix(in oklch, var(--flint-color-primary) 5%, transparent);
}
```

No Tailwind, no CSS-in-JS: pure CSS custom properties.
