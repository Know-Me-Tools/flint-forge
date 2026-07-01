# p5-c013 Tasks — OpenDesign + Claude Design Integration

## Tasks

- [ ] Implement `POST /a2ui/v1/design-systems/import` endpoint in `fdb-gateway`
- [ ] Implement `fdb-app/src/a2ui/design_md_parser.rs` — DESIGN.md 9-section parser (color, typography, spacing, layout, components, motion, voice, brand, anti-patterns)
- [ ] Implement W3C Design Token JSON → `design_systems.tokens jsonb` mapper
- [ ] Implement Claude Design ZIP import: extract + parse DESIGN.md from ZIP
- [ ] Add `source_format`, `source_content`, `imported_at` columns to `flint_a2ui.design_systems` (migration)
- [ ] Create `flint_a2ui.component_overrides` table with RLS (migration: `migrations/0003_flint_a2ui_overrides.sql`)
- [ ] Build OpenDesign plugin package structure: `open-design.json` + 2 SKILL.md skills
- [ ] Implement `flint-component-browser` OpenDesign skill: queries `GET /a2ui/v1/components` 
- [ ] Implement `flint-surface-preview` OpenDesign skill: calls `POST /htmx/components/:slug` → HTML preview
- [ ] Add `exportDesignSyncTokens()` export in `@flint/react` for Claude Design `/design-sync` compatibility
- [ ] Gate test: round-trip import of a DESIGN.md file → check tokens stored → check CSS vars on HTMX fragment
- [ ] Gate test: `component_overrides` row created when DESIGN.md includes component override section
- [ ] Document: `docs/integrations/opendesign.md` — how to install Flint plugin in OpenDesign
- [ ] Document: `docs/integrations/claude-design.md` — how to use `/design-sync` with Flint
