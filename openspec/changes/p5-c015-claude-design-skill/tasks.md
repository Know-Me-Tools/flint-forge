# p5-c015 Tasks — Claude Code Skill Package for Flint Components

## Tasks

- [x] Create `skills/flint-ui/` directory structure (SKILL.md, catalogs/, examples/, schemas/, plugins/)
- [x] Write `SKILL.md` with complete YAML frontmatter and skill body (component catalog summary, API usage examples for all 3 renderers)
- [x] Write `catalogs/components.md` — all 55 Flint component slugs with descriptions, prop signatures, and examples
- [x] Write `catalogs/react-api.md` — complete `@flint/react` API: FlintProvider, FlintSurface, registerFlintComponent, useFlint, hooks
- [x] Write `catalogs/flutter-api.md` — complete `flint_genui` API: FlintSurface, FlintCatalog, FlintA2uiTransport, FlintThemeData
- [x] Write `catalogs/htmx-api.md` — fdb-gateway HTMX routes, component renderers, SSE stream format
- [x] Write `examples/react-data-grid.tsx` — working DataGrid example with real Flint props
- [x] Write `examples/react-agent-chat.tsx` — AgentChat surface with streaming
- [x] Write `examples/flutter-surface.dart` — FlintSurface in a complete Flutter app
- [x] Write `examples/htmx-form.html` — Form fragment with HTMX attributes + SSE
- [x] Write `schemas/a2ui-message.json` — JSON Schema for A2UI v0.9 protocol messages
- [x] Write `schemas/design-token.json` — W3C 2024 design token JSON schema
- [x] Create `plugins/flint-components/open-design.json` for OpenDesign plugin distribution
- [ ] Test skill installation: `claude plugin install flint-ui@prometheus-ags/flint-forge`
- [ ] Gate test: Claude Code with skill installed generates correct `@flint/react` code without hallucinated props
- [ ] Gate test: `catalogs/components.md` slugs match `SELECT slug FROM flint_a2ui.components WHERE is_base = true`
