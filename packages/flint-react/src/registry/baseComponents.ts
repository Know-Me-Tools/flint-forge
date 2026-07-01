import { z } from 'zod';
import { registerFlintComponent } from './FlintRegistry';

// Layout
import { Stack, Card, Grid, Split, Tabs, Accordion, Scroll, Modal, Drawer } from '../components/layout';
// Data Display
import { DataGrid, Table, Chart, Timeline, Kanban, Calendar, Metric, Badge } from '../components/data-display';
// Input
import { Form, TextField, Select, DatePicker, Search, FileUpload, JsonEditor, RichEditor } from '../components/input';
// Action
import { Button, Confirm, Wizard, BulkAction, ActionBar } from '../components/action';
// Agent
import { AgentChat, ToolCall, StreamingText, Decision, ProgressLog, Artifact } from '../components/agent';
// Navigation
import { NavMenu, CommandPalette, FilterBar, Breadcrumb } from '../components/navigation';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyComponent = React.ComponentType<any>;

function reg(slug: string, category: string, primitiveType: string, schema: z.ZodTypeAny, component: AnyComponent): void {
  registerFlintComponent({ slug, category, primitiveType, propsSchema: schema, component });
}

// Import React for type compatibility
import React from 'react';

export function registerBaseComponents(): void {
  // Layout
  reg('stack', 'layout', 'Container', z.object({ direction: z.enum(['horizontal', 'vertical']).optional(), gap: z.number().optional(), children: z.any().optional() }), Stack as AnyComponent);
  reg('card', 'layout', 'Container', z.object({ title: z.string().optional(), elevated: z.boolean().optional(), children: z.any().optional() }), Card as AnyComponent);
  reg('grid', 'layout', 'Container', z.object({ columns: z.union([z.number(), z.string()]).optional(), gap: z.number().optional(), children: z.any().optional() }), Grid as AnyComponent);
  reg('split', 'layout', 'Container', z.object({ ratio: z.string().optional(), children: z.any().optional() }), Split as AnyComponent);
  reg('tabs', 'layout', 'Navigation', z.object({ items: z.array(z.object({ label: z.string(), value: z.string(), content: z.any() })), defaultValue: z.string().optional() }), Tabs as AnyComponent);
  reg('accordion', 'layout', 'Container', z.object({ items: z.array(z.object({ label: z.string(), value: z.string(), content: z.any() })), allowMultiple: z.boolean().optional() }), Accordion as AnyComponent);
  reg('scroll', 'layout', 'Container', z.object({ maxHeight: z.string().optional(), children: z.any().optional() }), Scroll as AnyComponent);
  reg('modal', 'layout', 'Overlay', z.object({ open: z.boolean(), onClose: z.function(), title: z.string().optional(), children: z.any().optional() }), Modal as AnyComponent);
  reg('drawer', 'layout', 'Overlay', z.object({ open: z.boolean(), onClose: z.function(), side: z.enum(['left', 'right']).optional(), children: z.any().optional() }), Drawer as AnyComponent);

  // Data Display
  reg('data-grid', 'data-display', 'Table', z.object({ columns: z.array(z.any()), data: z.array(z.record(z.unknown())), loading: z.boolean().optional() }), DataGrid as AnyComponent);
  reg('table', 'data-display', 'Table', z.object({ headers: z.array(z.string()), rows: z.array(z.array(z.any())) }), Table as AnyComponent);
  reg('chart', 'data-display', 'Chart', z.object({ type: z.enum(['bar', 'line']).optional(), data: z.array(z.object({ label: z.string(), value: z.number() })), title: z.string().optional() }), Chart as AnyComponent);
  reg('timeline', 'data-display', 'List', z.object({ events: z.array(z.object({ id: z.string(), label: z.string(), timestamp: z.string(), description: z.string().optional() })) }), Timeline as AnyComponent);
  reg('kanban', 'data-display', 'Board', z.object({ columns: z.array(z.object({ id: z.string(), title: z.string(), cards: z.array(z.any()) })) }), Kanban as AnyComponent);
  reg('calendar', 'data-display', 'Calendar', z.object({ year: z.number(), month: z.number(), events: z.array(z.object({ date: z.string(), label: z.string() })).optional() }), Calendar as AnyComponent);
  reg('metric', 'data-display', 'Metric', z.object({ label: z.string(), value: z.union([z.string(), z.number()]), unit: z.string().optional(), trend: z.enum(['up', 'down', 'flat']).optional() }), Metric as AnyComponent);
  reg('badge', 'data-display', 'Badge', z.object({ label: z.string(), variant: z.enum(['default', 'success', 'warning', 'error', 'info']).optional() }), Badge as AnyComponent);

  // Input
  reg('form', 'input', 'Form', z.object({ fields: z.array(z.any()), onSubmit: z.function(), submitLabel: z.string().optional() }), Form as AnyComponent);
  reg('text-field', 'input', 'Input', z.object({ label: z.string(), name: z.string(), value: z.string().optional(), placeholder: z.string().optional(), required: z.boolean().optional() }), TextField as AnyComponent);
  reg('select', 'input', 'Select', z.object({ label: z.string(), name: z.string(), options: z.array(z.object({ label: z.string(), value: z.string() })), value: z.string().optional() }), Select as AnyComponent);
  reg('date-picker', 'input', 'Input', z.object({ label: z.string(), name: z.string(), value: z.string().optional(), required: z.boolean().optional() }), DatePicker as AnyComponent);
  reg('search', 'input', 'Input', z.object({ value: z.string().optional(), placeholder: z.string().optional() }), Search as AnyComponent);
  reg('file-upload', 'input', 'Input', z.object({ label: z.string(), name: z.string(), accept: z.string().optional(), multiple: z.boolean().optional() }), FileUpload as AnyComponent);
  reg('json-editor', 'input', 'Editor', z.object({ label: z.string(), name: z.string(), value: z.string().optional() }), JsonEditor as AnyComponent);
  reg('rich-editor', 'input', 'Editor', z.object({ label: z.string(), name: z.string(), value: z.string().optional() }), RichEditor as AnyComponent);

  // Action
  reg('button', 'action', 'Button', z.object({ variant: z.enum(['primary', 'secondary', 'ghost', 'destructive']).optional(), size: z.enum(['sm', 'md', 'lg']).optional(), loading: z.boolean().optional(), children: z.any().optional() }), Button as AnyComponent);
  reg('confirm', 'action', 'Dialog', z.object({ message: z.string(), onConfirm: z.function(), onCancel: z.function(), confirmLabel: z.string().optional(), cancelLabel: z.string().optional() }), Confirm as AnyComponent);
  reg('wizard', 'action', 'MultiStep', z.object({ steps: z.array(z.object({ title: z.string(), content: z.any() })), onComplete: z.function().optional() }), Wizard as AnyComponent);
  reg('bulk-action', 'action', 'Toolbar', z.object({ selectedCount: z.number(), actions: z.array(z.object({ label: z.string(), onClick: z.function(), destructive: z.boolean().optional() })) }), BulkAction as AnyComponent);
  reg('action-bar', 'action', 'Toolbar', z.object({ actions: z.array(z.object({ label: z.string(), onClick: z.function(), disabled: z.boolean().optional() })) }), ActionBar as AnyComponent);

  // Agent
  reg('agent-chat', 'agent', 'Chat', z.object({ messages: z.array(z.object({ id: z.string(), role: z.enum(['user', 'assistant', 'tool']), content: z.string(), timestamp: z.string().optional() })), loading: z.boolean().optional() }), AgentChat as AnyComponent);
  reg('tool-call', 'agent', 'ToolCall', z.object({ name: z.string(), status: z.enum(['pending', 'running', 'complete', 'error']), args: z.string().optional(), result: z.string().optional() }), ToolCall as AnyComponent);
  reg('streaming-text', 'agent', 'Text', z.object({ text: z.string(), streaming: z.boolean().optional() }), StreamingText as AnyComponent);
  reg('decision', 'agent', 'Decision', z.object({ question: z.string(), options: z.array(z.object({ id: z.string(), label: z.string(), description: z.string().optional() })) }), Decision as AnyComponent);
  reg('progress-log', 'agent', 'Log', z.object({ entries: z.array(z.object({ id: z.string(), message: z.string(), level: z.enum(['info', 'warn', 'error']).optional(), timestamp: z.string().optional() })), title: z.string().optional() }), ProgressLog as AnyComponent);
  reg('artifact', 'agent', 'Artifact', z.object({ type: z.enum(['code', 'text', 'image', 'file']), content: z.string(), language: z.string().optional(), filename: z.string().optional() }), Artifact as AnyComponent);

  // Navigation
  reg('nav-menu', 'navigation', 'Navigation', z.object({ items: z.array(z.any()), orientation: z.enum(['horizontal', 'vertical']).optional(), ariaLabel: z.string().optional() }), NavMenu as AnyComponent);
  reg('command-palette', 'navigation', 'Search', z.object({ open: z.boolean(), onClose: z.function(), commands: z.array(z.any()), placeholder: z.string().optional() }), CommandPalette as AnyComponent);
  reg('filter-bar', 'navigation', 'Filter', z.object({ filters: z.array(z.any()), values: z.record(z.string()), onChange: z.function() }), FilterBar as AnyComponent);
  reg('breadcrumb', 'navigation', 'Navigation', z.object({ items: z.array(z.object({ label: z.string(), href: z.string().optional() })) }), Breadcrumb as AnyComponent);
}
