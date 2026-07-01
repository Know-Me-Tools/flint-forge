# p5-c015 Tasks — Claude Code Skill Package for Flint Components

## Tasks

- [ ] Create `skills/flint-ui/` directory structure (SKILL.md, catalogs/, examples/, schemas/)
- [ ] Write `SKILL.md` with complete YAML frontmatter and skill body (component catalog summary, API usage examples for all 3 renderers)
- [ ] Write `catalogs/components.md` — all 63 Flint component slugs with descriptions, prop signatures, and examples
- [ ] Write `catalogs/react-api.md` — complete `@flint/react` API: FlintProvider, FlintSurface, registerFlintComponent, useFlint, hooks
- [ ] Write `catalogs/flutter-api.md` — complete `flint_genui` API: FlintSurface, FlintCatalog, FlintA2uiTransport, FlintThemeData
- [ ] Write `catalogs/htmx-api.md` — fdb-gateway HTMX routes, Askama template conventions, SSE stream format
- [ ] Write `examples/react-data-grid.tsx` — working DataGrid example with real Flint props
- [ ] Write `examples/react-agent-chat.tsx` — AgentChat surface with streaming
- [ ] Write `examples/flutter-surface.dart` — FlintSurface in a complete Flutter app
- [ ] Write `examples/htmx-form.html` — Form fragment with HTMX attributes
- [ ] Write `schemas/a2ui-message.json` — JSON Schema for A2UI protocol messages
- [ ] Write `schemas/design-token.json` — W3C 2024 design token JSON schema
- [ ] Create `plugins/flint-components/open-design.json` for OpenDesign plugin distribution
- [ ] Test skill installation: `claude plugin install flint-ui@prometheus-ags/flint-forge`
- [ ] Gate test: Claude Code with skill installed generates correct `@flint/react` code without hallucinated props
- [ ] Gate test: `catalogs/components.md` slugs match `SELECT slug FROM flint_a2ui.components WHERE is_base = true`
