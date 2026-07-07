/**
 * slugMap.ts
 *
 * Maps all 55 A2UI catalog slugs to their React component implementations.
 * Slugs without a dedicated renderer use a `div` placeholder that carries
 * `data-flint-component` and `data-flint-placeholder` attributes so runtime
 * tooling can still identify and hydrate them later.
 *
 * Real component mappings:  27
 * Placeholder mappings:     28
 * Total:                    55
 */
import React from 'react';

// ── Layout ────────────────────────────────────────────────────────────────
import { Grid, Scroll, Modal, Stack, Tabs } from '../components/layout';

// ── Data display ──────────────────────────────────────────────────────────
import { Badge, DataGrid, Metric, Table, Timeline } from '../components/data-display';

// ── Artifact (used for code-block slug) ───────────────────────────────────
import { Artifact } from '../components/agent';

// ── Input ─────────────────────────────────────────────────────────────────
import {
  DatePicker,
  FileUpload,
  Form,
  JsonEditor,
  RichEditor,
  Search,
  Select,
  TextField,
} from '../components/input';

// ── Action ────────────────────────────────────────────────────────────────
import { ActionBar, Button, Confirm, Wizard } from '../components/action';

// ── Navigation ────────────────────────────────────────────────────────────
import { Breadcrumb, NavMenu } from '../components/navigation';

// ── Types ─────────────────────────────────────────────────────────────────

/** Canonical component type used by the slug map. */
export type SlugComponent = React.ComponentType<Record<string, unknown>>;

// ── Placeholder factory ───────────────────────────────────────────────────

/**
 * Creates a lightweight placeholder component for catalog slugs that don't
 * have a dedicated renderer yet.  The element carries discoverable data
 * attributes so agents, storybook, and dev tools can identify it.
 */
function makePlaceholder(slug: string): SlugComponent {
  function FlintPlaceholder(props: Record<string, unknown>): React.ReactElement {
    return React.createElement('div', {
      'data-flint-component': slug,
      'data-flint-placeholder': 'true',
      ...props,
    });
  }
  FlintPlaceholder.displayName = `FlintPlaceholder(${slug})`;
  return FlintPlaceholder;
}

// ── Slug → Component map ──────────────────────────────────────────────────

/**
 * Maps every A2UI catalog slug to its React component implementation.
 * Real components are imported above; all remaining slots use
 * `makePlaceholder(slug)` so the map is always complete and typesafe.
 *
 * Catalog sections (55 total):
 *   LAYOUT (8) · DATA DISPLAY (12) · INPUT (14) · ACTION (6) ·
 *   NAVIGATION (6) · FEEDBACK (8) · SYSTEM (1)
 */
export const SLUG_MAP: Record<string, SlugComponent> = {
  // ── LAYOUT (8) ───────────────────────────────────────────────────────────
  'container':   makePlaceholder('container'),   // no Container component yet
  'row':         makePlaceholder('row'),          // Stack covers horizontal, but different API
  'column':      makePlaceholder('column'),       // Stack covers vertical, but different API
  'grid':        Grid as SlugComponent,           // ✓ direct match
  'stack':       Stack as SlugComponent,          // ✓ direct match
  'divider':     makePlaceholder('divider'),      // no Divider component yet
  'spacer':      makePlaceholder('spacer'),       // no Spacer component yet
  'scroll-area': Scroll as SlugComponent,         // ✓ Scroll wraps scrollable content

  // ── DATA DISPLAY (12) ────────────────────────────────────────────────────
  'data-grid':   DataGrid as SlugComponent,       // ✓ direct match (generic DataGrid<T>)
  'data-table':  Table as SlugComponent,          // ✓ Table (simple header/row table)
  'text':        makePlaceholder('text'),          // no standalone Text component yet
  'badge':       Badge as SlugComponent,          // ✓ direct match
  'tag':         makePlaceholder('tag'),           // no Tag (removable chip) yet
  'avatar':      makePlaceholder('avatar'),        // no Avatar component yet
  'stat-card':   Metric as SlugComponent,         // ✓ Metric = label + value + trend ≈ StatCard
  'timeline':    Timeline as SlugComponent,       // ✓ direct match
  'code-block':  Artifact as SlugComponent,       // ✓ Artifact(type="code") renders a code block
  'json-viewer': JsonEditor as SlugComponent,     // ✓ nearest JSON handler (viewer/editor)
  'list':        makePlaceholder('list'),          // no List component yet
  'detail-view': makePlaceholder('detail-view'),  // no DetailView component yet

  // ── INPUT (14) ───────────────────────────────────────────────────────────
  'form':          Form as SlugComponent,         // ✓ direct match
  'text-input':    TextField as SlugComponent,    // ✓ TextField renders a text input
  'number-input':  TextField as SlugComponent,    // ✓ TextField with type="number"
  'select':        Select as SlugComponent,       // ✓ direct match
  'multi-select':  makePlaceholder('multi-select'), // no MultiSelect yet
  'date-picker':   DatePicker as SlugComponent,  // ✓ direct match
  'checkbox':      makePlaceholder('checkbox'),   // no Checkbox component yet
  'radio':         makePlaceholder('radio'),      // no Radio component yet
  'toggle':        makePlaceholder('toggle'),     // no Toggle component yet
  'textarea':      RichEditor as SlugComponent,  // ✓ RichEditor wraps a <textarea>
  'file-upload':   FileUpload as SlugComponent,  // ✓ direct match
  'search-input':  Search as SlugComponent,      // ✓ Search = debounced search input
  'color-picker':  makePlaceholder('color-picker'), // no ColorPicker yet
  'slider':        makePlaceholder('slider'),     // no Slider component yet

  // ── ACTION (6) ───────────────────────────────────────────────────────────
  'button':        Button as SlugComponent,       // ✓ direct match
  'action-bar':    ActionBar as SlugComponent,    // ✓ direct match
  'dropdown-menu': makePlaceholder('dropdown-menu'), // no DropdownMenu yet
  'context-menu':  makePlaceholder('context-menu'),  // no ContextMenu yet
  'fab':           makePlaceholder('fab'),         // no FAB component yet
  'link':          makePlaceholder('link'),        // no Link component yet

  // ── NAVIGATION (6) ───────────────────────────────────────────────────────
  'nav-bar':    NavMenu as SlugComponent,         // ✓ NavMenu (horizontal orientation)
  'sidebar':    NavMenu as SlugComponent,         // ✓ NavMenu (vertical orientation)
  'tabs':       Tabs as SlugComponent,            // ✓ direct match
  'breadcrumb': Breadcrumb as SlugComponent,      // ✓ direct match
  'pagination': makePlaceholder('pagination'),    // no Pagination component yet
  'stepper':    Wizard as SlugComponent,          // ✓ Wizard = multi-step process

  // ── FEEDBACK (8) ─────────────────────────────────────────────────────────
  'alert':           makePlaceholder('alert'),          // no Alert component yet
  'toast':           makePlaceholder('toast'),          // no Toast component yet
  'modal':           Modal as SlugComponent,            // ✓ direct match
  'dialog':          Confirm as SlugComponent,          // ✓ Confirm = ok/cancel dialog
  'loading-spinner': makePlaceholder('loading-spinner'), // no LoadingSpinner yet
  'progress-bar':    makePlaceholder('progress-bar'),   // no ProgressBar yet
  'empty-state':     makePlaceholder('empty-state'),    // no EmptyState yet
  'error-boundary':  makePlaceholder('error-boundary'), // no ErrorBoundary yet

  // ── SYSTEM (1) ───────────────────────────────────────────────────────────
  'flint-meta-schema': makePlaceholder('flint-meta-schema'), // internal introspection only
} as const;

// ── Public API ────────────────────────────────────────────────────────────

/**
 * Returns the React component registered for the given A2UI catalog slug,
 * or `undefined` if the slug is unknown.
 *
 * @example
 * const Component = fromSlug('data-grid');
 * if (Component) return <Component {...props} />;
 */
export function fromSlug(slug: string): SlugComponent | undefined {
  return (SLUG_MAP as Record<string, SlugComponent | undefined>)[slug];
}
