# Goals — p5-a2ui-registry

## Phase Summary

Deliver the Flint A2UI component registry and SDK platform: a PostgreSQL-native, pgvector-backed component registry with auto-binding from `flint_meta` cache tables, REST + event-driven API surfaces, and headless SDK renderers for React 19 and Flutter. Integrates OpenDesign and Claude Design tooling.

This is Phase 5 of the Flint Forge roadmap (RFC-FORGE-PHASES-002). 15 OpenSpec changes total; 7 are MVP.

## Goals

### Registry Core (MVP — must ship)

- **G1 — p5-c001-flint-a2ui-schema:** Pure SQL migration creating `flint_a2ui` schema: `component_definitions`, `component_embeddings` (pgvector HNSW index), `component_instances`, `component_overrides` tables. Unblocks all downstream changes.

- **G2 — p5-c002-base-components-seed:** Seed the 50+ Flint base component catalog (Text, Button, Row, Column, Card, Input, etc.) with JSON schema, visual metadata, and placeholder embeddings.

- **G3 — p5-c003-auto-binding-trigger:** `flint_a2ui.auto_bind_components()` trigger fires on `flint_meta.cache_tables` INSERT — auto-generates component instances bound to new cache table columns. Wire into `ext-flint-hooks`.

- **G4 — p5-c009-compiled-state-upgrade:** Extend `CompiledState` in `fdb-reflection` with `a2ui_registry: Option<A2uiRegistrySnapshot>` — pre-compiled registry state hot-swapped on DDL changes. Requires `p2-c003` (CompiledState struct) — already delivered.

### SDK Platform (MVP — must ship in order)

- **G5 — p5-c014-sdk-schema-extensions:** DB schema additions for SDK support: `component_overrides` table, `renderers` column on `component_definitions`, `design_systems` import metadata, `resolve_components_with_overrides()` Postgres function. MUST ship before G6 and G7.

- **G6 — p5-c010-react-sdk:** `@flint/react` — React 19 headless library. `FlintProvider`, `FlintSurface`, `FlintRegistry` (Zod schema), `FlintAgUiAdapter`. No CopilotKit/Vercel deps. < 80kb gzipped target.

- **G7 — p5-c011-flutter-sdk:** `flint_genui` Dart package extending `genui ^0.9.2` (flutter/genui, official Flutter org). `FlintA2uiTransport` pure SSE. `cue ^0.3.11` for animations. No Gemini/Firebase deps.

### Extended (P2 — ship if capacity allows)

- **G8 — p5-c004-embeddings-pipeline:** `flint_a2ui.embed_components()` procedure — calls liter-llm embedding endpoint, stores in `component_embeddings`. Gates semantic similarity search. Requires OQ-10 (text-embedding-3-large via liter-llm).

- **G9 — p5-c005-application-model:** `component_applications` table tracking which components are applied to which surfaces. Agent-readable application model.

- **G10 — p5-c006-rest-api:** REST endpoints for component registry CRUD via Quarry reflection compiler (`GET /flint_a2ui/component_definitions`, etc.). Should come for free from p2-c004 REST compiler once it lands.

- **G11 — p5-c007-event-driven-assembly:** `flint_a2ui.assemble_surface(surface_id)` — event-sourced surface assembly from component instances + overrides.

- **G12 — p5-c008-protocol-surfaces:** `flint_a2ui.surface_descriptor()` — returns an A2UI content object (JSON) with resolved component tree for AG-UI delivery.

- **G13 — p5-c012-htmx-renderer:** Axum+Askama HTMX renderer in `fdb-gateway`. Admin/prototyping surface ONLY — not for production agent-generated UI.

- **G14 — p5-c013-opendesign-integration:** OpenDesign (`nexu-io/open-design`, Apache-2.0, 73k stars) plugin + Claude Design ZIP import. `DESIGN.md` parser in `fdb-app`.

- **G15 — p5-c015-claude-design-skill:** Claude Code skill package (`skills/flint-ui/SKILL.md`) + OpenDesign plugin manifest. Installable via `claude plugin marketplace add`.

## Dependencies

### Resolved (all gates cleared)
- `CompiledState` struct in `fdb-reflection` — DELIVERED (p2-c003) ✅
- pgvector ≥ 0.7.0 in PG18 image — CONFIRMED (OQ-9 cleared in p2-quarry-backfill) ✅
- `PgVectorRpc` adapter in `fdb-postgres` — DELIVERED (p2-c006) ✅

### Open Questions Gating Specific Changes
- OQ-10: `text-embedding-3-large` availability via liter-llm → gates `p5-c004` (embeddings pipeline)
- OQ-13: `genui ^1.0.0` stable release → pin `^0.9.2` for now; `flint_genui` can ship with alpha
- OQ-14: OpenDesign plugin marketplace → GitHub-path install for now

### SDK Dependency Order (strict)
```
p5-c014 (SDK schema) → p5-c010 (React SDK)
                      → p5-c011 (Flutter SDK)
                      → p5-c013 (OpenDesign)
```

## Phase Complete When (MVP gate)
- [ ] `p5-c001` migration applied; `flint_a2ui` schema created with pgvector HNSW index
- [ ] `p5-c002` base catalog seeded (50+ components with JSON schema)
- [ ] `p5-c003` auto-binding trigger wired and tested
- [ ] `p5-c009` CompiledState extended with `a2ui_registry` field
- [ ] `p5-c014` SDK schema additions in DB
- [ ] `p5-c010` `@flint/react` package buildable (< 80kb target)
- [ ] `p5-c011` `flint_genui` Dart package publishable
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` clean
