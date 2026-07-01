# Execution — p5-a2ui-registry

**Date:** 2026-06-30  
**Backend:** `openspec`  
**Driver:** `/kbd-apply` (per change)  
**Phase:** p5-a2ui-registry  
**Changes:** 15 total (0 complete)

---

## Backend Selection

OpenSpec is the authoritative backend. The `openspec/changes/` directory contains all
15 p5-cXXX proposal directories — all proposals and task files already exist from the
planning phase. Execution routes through `/kbd-apply <change-id>` which fires
`task:before`/`task:after` hooks and updates `progress.json` after each change.

Do **not** drive changes via bare `/opsx:apply` — that path bypasses KBD progress
tracking and hooks.

---

## Dispatch Contract

```
For each wave (in order):
  /kbd-apply p5-c001-flint-a2ui-schema

  /kbd-apply p5-c002-base-components-seed
  /kbd-apply p5-c003-auto-binding-trigger

  /kbd-apply p5-c009-compiled-state-upgrade

  /kbd-apply p5-c014-sdk-schema-extensions

  /kbd-apply p5-c010-react-sdk        ← parallel with c011
  /kbd-apply p5-c011-flutter-sdk      ← parallel with c010

  --- MVP COMPLETE ---

  /kbd-apply p5-c005-application-model
  /kbd-apply p5-c004-embeddings-pipeline   ← stub only
  /kbd-apply p5-c012-htmx-renderer

  /kbd-apply p5-c006-rest-api

  /kbd-apply p5-c007-event-driven-assembly
  /kbd-apply p5-c008-protocol-surfaces
  /kbd-apply p5-c013-opendesign-integration

  /kbd-apply p5-c015-claude-design-skill
```

After each change completes:
1. Run artifact-refiner QA gate (skip if < 3 files modified or docs-only)
2. `/opsx:verify <change-id>`
3. `/opsx:archive <change-id>`
4. Update `progress.json` `changes_completed` + change `status: "done"`

---

## QA Gate Policy

| Change | QA Required | Reason |
|--------|-------------|--------|
| p5-c001 | YES | SQL migration — critical correctness |
| p5-c002 | NO | SQL seed only, < 3 Rust files |
| p5-c003 | YES | Rust source change (agui.rs) |
| p5-c009 | YES | Hot-path Rust struct change |
| p5-c014 | YES | SQL + Rust types |
| p5-c010 | YES | Full TypeScript package |
| p5-c011 | YES | Full Dart package |
| p5-c004 | NO | Stub body only |
| p5-c005 | YES | Rust use-case layer |
| p5-c006 | YES | Axum route handlers |
| p5-c007 | YES | Rust assembler + SSE |
| p5-c008 | NO | Stub bodies with todo!() |
| p5-c012 | YES | Axum + Askama renderer |
| p5-c013 | YES | Rust parser |
| p5-c015 | NO | Documentation-only |

---

## Pre-Flight Gates (must pass before Wave 1)

- [ ] `cargo check --workspace` passes on baseline codebase
- [ ] Two spec defects confirmed understood by executor:
  - R1: `is_system boolean NOT NULL DEFAULT false` must be added to `flint_a2ui.applications` DDL in c001
  - R2: `resolve_components_with_overrides()` must use `ra.application_id` (not `ra.app_id`) in c014
- [ ] Consumer audit grep documented before c009 starts:
  `grep -rn "CompiledState\|compiled\." crates/fdb-gateway/src/ crates/fdb-reflection/src/`

---

## MVP Success Criteria

Phase 5 MVP is complete when ALL of the following are true:

1. `migrations/0002_flint_a2ui.sql` applies cleanly against Postgres 18 with pgvector
2. 55 base components seeded in `flint_a2ui.components`
3. Auto-binding trigger fires on `flint_meta.cache_tables` INSERT
4. `ext-flint-meta` returns `'flint-forge/schema-descriptor/1.0'` for protocol
5. `cargo check --workspace` and `cargo clippy --workspace -- -D warnings` both pass after c009
6. `CompiledState` contains `a2ui_catalog: Arc<A2uiCatalog>` and loads from DB at startup
7. `migrations/0003_flint_a2ui_sdk_extensions.sql` applies cleanly
8. `@flint/react` package builds under 80kb gzipped
9. `flint_genui` Dart package passes `dart analyze` with zero warnings

---

## Carried Debt Tracking

| Item | Resolves In | OQ |
|------|-------------|-----|
| PgVectorRpc text serializer | Before SDK ships to prod | None |
| VectorRpcRequest.filter ignored | p5-c006 (attach WHERE) | None |
| semantic_search() stub | p5-c004 (needs OQ-10) | OQ-10 |
| FabricChangeSource empty stream | Phase 7 | OQ-FRF-1 |
| KetoSyncTask not wired | Phase 5 c007 (OQ-Iggy stub) | OQ-Iggy |
