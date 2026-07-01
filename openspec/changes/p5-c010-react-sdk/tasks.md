# p5-c010 Tasks — Flint React SDK

## Tasks

- [x] Initialize `packages/flint-react/` as TypeScript npm package (`@flint/react`)
- [x] Implement `FlintProvider` with AG-UI SSE connection, JWT propagation, component override map, token injection
- [x] Implement `FlintSurface` widget: receives A2UI `createSurface`/`updateComponents`/`deleteSurface` messages and renders component tree
- [x] Implement `FlintRegistry`: slug → (ZodSchema, React component) mapping, seeded from base 40 Flint components
- [x] Implement `registerFlintComponent(opts)` — user-space extension API
- [x] Implement `FlintAgUiAdapter`: SSE client subscribing to `fdb-gateway` AG-UI stream, dispatching to surface state
- [x] Implement design token system: `GET /a2ui/v1/catalog/:id` → CSS custom property injection via SWR
- [x] Implement ALL layout components (Stack, Card, Grid, Split, Tabs, Accordion, Scroll, Modal, Drawer) — headless, `data-flint-component` attrs
- [x] Implement ALL data-display components (DataGrid, Table, Chart, Timeline, Kanban, Calendar, Metric, Badge) — headless
- [x] Implement ALL input components (Form, TextField, Select, DatePicker, JsonEditor, RichEditor, Search, FileUpload) — headless
- [x] Implement ALL action components (Button, Confirm, Wizard, BulkAction, ActionBar) — headless
- [x] Implement ALL agent-specific components (AgentChat, ToolCall, StreamingText, Decision, ProgressLog, Artifact) — headless with streaming state
- [x] Implement ALL navigation components (NavMenu, CommandPalette, FilterBar, Breadcrumb) — headless
- [x] Write `SKILL.md` for Claude Code skill distribution
- [x] Gate tests: FlintProvider renders, data-flint-app attr present, registry size ≥ 40
- [x] Gate tests: component smoke tests (Button, DataGrid, TextField, StreamingText, Breadcrumb)
- [ ] Gate test: bundle size < 80kb gzipped (requires build toolchain; deferred to CI)
- [ ] Gate test: axe-core passes on all components (deferred to CI)
- [ ] Publish to npm as `@flint/react` (internal registry initially)
