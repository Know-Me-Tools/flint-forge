# Phase 5 Assessment — Flint A2UI Component Registry

## Goal
Build the Flint-invented A2UI Component Registry layer on top of the official A2UI protocol, including base component seeding, auto-binding from DB metadata, embeddings/semantic search, REST API, event-driven assembly, protocol surfaces (A2A/MCP), SDKs (React, Flutter, HTMX), and design-tool integrations.

## Authority
- `docs/FLINT-PHASE-PLAN-REVISED.md` (RFC-FORGE-PHASES-002, 2026-06-30, status: Validated)
- `docs/FLINT-A2UI-REGISTRY-SPEC.md` (registry-specific spec)
- Existing OpenSpec changes `p5-c001` through `p5-c015`

## State
### Completed changes
- `p5-c001-flint-a2ui-schema` — `flint_a2ui` schema, pgvector/HNSW, RLS, migration wiring, gate tests
- `p5-c002-base-components-seed` — 55+ base components seeded
- `p5-c003-auto-binding-trigger` — auto-binding trigger on `flint_meta.cache_tables`
- `p5-c009-compiled-state-upgrade` — `A2uiCatalog` in `CompiledState` (Cedar items deferred to Phase 7)
- `p5-c010-react-sdk` — `@flint/react` package implemented (bundle/axe tests deferred to CI)
- `p5-c011-flutter-sdk` — `flint_genui` package implemented (publish deferred)
- `p5-c014-sdk-schema-extensions` — `0004_flint_a2ui_sdk_extensions.sql`, override resolution SQL function, Rust types (catalog endpoint deferred to p5-c006)

### Incomplete changes (in dependency order)
1. `p5-c004-embeddings-pipeline` — background embedder, hybrid search
2. `p5-c005-application-model` — `resolve_components()`, role hierarchy, Cedar capabilities
3. `p5-c006-rest-api` — 8 A2UI REST endpoints
4. `p5-c007-event-driven-assembly` — `A2uiAssembler`, surface assembly
5. `p5-c008-protocol-surfaces` — A2A/MCP tool surfaces
6. `p5-c012-htmx-renderer` — HTMX/Askama server renderer
7. `p5-c013-opendesign-integration` — OpenDesign/Claude Design import
8. `p5-c015-claude-design-skill` — Claude Code skill package

## Phase 5 MVP sign-off criteria (per revised plan)
Minimum viable: `p5-c014 + p5-c010 + p5-c011` complete. These are done (with deferred CI/publish tasks).

## Phase 7 blockers
`p7-c005-a2ui-surface-emitter` requires `p5-c001, p5-c003, p5-c007, p5-c009`. Therefore `p5-c007` is on the critical path for Phase 7.

## Risks
- OQ-10: liter-llm gateway availability for embedding generation
- OQ-11: A2UI catalog JSON Schema format
- Phase 7 A2UI emission depends on p5-c007 being complete
