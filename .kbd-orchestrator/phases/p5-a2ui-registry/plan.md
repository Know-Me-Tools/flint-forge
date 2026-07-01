# Plan — p5-a2ui-registry

**Date:** 2026-06-30  
**Backend:** OpenSpec (openspec/ directory detected)  
**Phase:** p5-a2ui-registry  
**Changes:** 15 total — 7 MVP (P0), 5 extended (P1), 3 extended (P2)

---

## Ordering Rationale

The dependency graph from assessment drives the execution sequence:

1. Database schema must land first — every other change depends on `flint_a2ui` existing in Postgres.
2. Seed (c002) and trigger (c003) are data-only changes that can run in any order after c001, but c003 references the `components` table so c002 must precede it in a single transaction.
3. CompiledState upgrade (c009) requires the DB schema to exist at runtime but not at compile time — it can be authored in parallel with c002/c003 and must land before the SDK layer.
4. SDK schema extensions (c014) alter the schema c001 created and provide the Rust types SDKs need — must precede React (c010) and Flutter (c011).
5. React and Flutter SDKs are fully independent of each other once c014 lands; they can be executed in parallel.
6. Extended P1 changes (c005, c006, c007, c008) depend on the MVP foundation and are ordered by dependency within themselves.
7. P2 changes (c012, c013, c015) depend only on the MVP and each other's immediate dep; they are independent enough to parallel.
8. c004 (embeddings) is deferred pending OQ-10 resolution but the stub migration goes in c001 so the table exists.

**Pre-flight corrections** (from assessment findings R1 and R2) are handled as explicit task steps inside c001 and c014 — not as separate changes. The proposals must be amended before code is written.

---

## MVP Execution Wave (P0 — must ship first)

### Wave 1 — Database Foundation

**Change 1: `p5-c001-flint-a2ui-schema`**  
*Priority: P0 | Agent: rust-auditor + database-reviewer | Estimated scope: Large*

**Pre-execution correction (R1):** Before writing migration SQL, amend the DDL to add `is_system boolean NOT NULL DEFAULT false` to `CREATE TABLE flint_a2ui.applications`. The proposal's seed INSERT references this column but the table definition omits it.

Tasks:
1. Create `migrations/` directory at workspace root
2. Write `migrations/0002_flint_a2ui.sql`:
   - `CREATE EXTENSION IF NOT EXISTS vector;`
   - `CREATE SCHEMA IF NOT EXISTS flint_a2ui;`
   - `CREATE TABLE flint_a2ui.components` (12 columns per spec)
   - `CREATE TABLE flint_a2ui.applications` ← **add `is_system boolean NOT NULL DEFAULT false`**
   - `CREATE TABLE flint_a2ui.design_systems`
   - `CREATE TABLE flint_a2ui.embeddings` (vector(1536) column)
   - `CREATE TABLE flint_a2ui.schemas`
   - `CREATE TABLE flint_a2ui.bindings`
   - `CREATE TABLE flint_a2ui.events` (append-only enforced via RLS)
   - `CREATE TABLE flint_a2ui.assembly_rules`
   - `CREATE TABLE flint_a2ui.roles`
   - `CREATE TABLE flint_a2ui.role_assignments`
   - HNSW index on `embeddings.embedding` (`m=16, ef_construction=64`)
   - Stub `flint_a2ui.semantic_search()` function (`RAISE NOTICE` body)
   - RLS: enable on `components`, `events`, `component_overrides` (events append-only)
   - `GRANT USAGE ON SCHEMA flint_a2ui TO authenticated, service_role`
3. Verify migration parses with `psql --dry-run` or equivalent
4. `cargo check --workspace` passes (migration is SQL-only, no Rust change)

---

### Wave 2 — Seed + Trigger (parallel after Wave 1)

**Change 2: `p5-c002-base-components-seed`**  
*Priority: P0 | Agent: database-reviewer | Estimated scope: Medium*

Tasks:
1. Write `scripts/seed_a2ui_components.sql` with `INSERT ... ON CONFLICT (slug) DO NOTHING`:
   - 8 layout components: `flint-stack`, `flint-grid`, `flint-container`, `flint-divider`, `flint-spacer`, `flint-scroll`, `flint-panel`, `flint-tabs`
   - 12 data-display components: `flint-text`, `flint-badge`, `flint-avatar`, `flint-chip`, `flint-tag`, `flint-table`, `flint-list`, `flint-card`, `flint-stat`, `flint-timeline`, `flint-tree`, `flint-calendar`
   - 14 input components: `flint-input`, `flint-textarea`, `flint-select`, `flint-combobox`, `flint-checkbox`, `flint-radio`, `flint-switch`, `flint-slider`, `flint-date-picker`, `flint-time-picker`, `flint-file-upload`, `flint-color-picker`, `flint-rating`, `flint-otp`
   - 6 action components: `flint-button`, `flint-icon-button`, `flint-fab`, `flint-link`, `flint-menu`, `flint-dropdown`
   - 6 navigation components: `flint-nav`, `flint-breadcrumb`, `flint-sidebar`, `flint-app-bar`, `flint-tab-bar`, `flint-stepper`
   - 8 feedback components: `flint-toast`, `flint-alert`, `flint-dialog`, `flint-drawer`, `flint-popover`, `flint-tooltip`, `flint-progress`, `flint-skeleton`
   - 1 system: `flint-meta-schema` (non-renderable, `category = 'system'`)
   - Total: 55 components
2. Ensure `primitive_type` column maps to A2UI Basic Catalog types: `Text`, `Button`, `Row`, or `Container`
3. All `schema` column values are valid JSON matching A2UI component schema format
4. Run seed against local Postgres 18 instance after c001 migration applied

**Change 3: `p5-c003-auto-binding-trigger`**  
*Priority: P0 | Agent: database-reviewer + rust-reviewer | Estimated scope: Medium*

**Note:** c002 must be applied before this change is tested (trigger body references `flint_a2ui.components`).

Tasks:
1. Write `flint_a2ui.column_type_to_component(pg_type text) RETURNS text` SQL function (IMMUTABLE):
   - Maps: `text/varchar/char` → `'flint-input'`
   - Maps: `int/bigint/numeric/float/double` → `'flint-input'` (type=number)
   - Maps: `bool/boolean` → `'flint-switch'`
   - Maps: `date/timestamp/timestamptz` → `'flint-date-picker'`
   - Maps: `jsonb/json` → `'flint-textarea'`
   - Default: `'flint-input'`
2. Write `flint_a2ui.auto_generate_bindings()` PL/pgSQL trigger function:
   - Queries `flint_meta.columns` for NEW.table_name
   - Generates grid/form/detail binding entries in `flint_a2ui.bindings`
   - Inserts binding event to `flint_a2ui.events`
3. Create trigger: `CREATE TRIGGER a2ui_auto_bind_tables AFTER INSERT ON flint_meta.cache_tables FOR EACH ROW EXECUTE FUNCTION flint_a2ui.auto_generate_bindings();`
4. Update `crates/ext-flint-meta/src/agui.rs`:
   - Line 83: change `"ag-ui/1.0"` → `"flint-forge/schema-descriptor/1.0"`
   - Line 253: update test assertion to match new value
5. Run `cargo test -p ext-flint-meta` — test must pass with updated value

---

### Wave 3 — CompiledState Upgrade

**Change 4: `p5-c009-compiled-state-upgrade`**  
*Priority: P0 | Agent: rust-reviewer + rust-auditor | Estimated scope: Large — HIGH RISK*

**Consumer audit MANDATORY before coding.** Run this grep before touching any file:

```bash
grep -rn "CompiledState\|compiled\." crates/fdb-gateway/src/ crates/fdb-reflection/src/
```

Tasks:
1. Consumer audit: list all files that read fields from `CompiledState`; record in change notes
2. Add to `crates/fdb-reflection/src/compiled.rs`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct A2uiCatalogEntry {
       pub slug: String,
       pub primitive_type: String,
       pub category: String,
       pub schema: serde_json::Value,
       pub description: Option<String>,
   }
   
   #[derive(Debug, Clone)]
   pub struct A2uiCatalog {
       pub catalog_id: String,
       pub version: String,
       pub components: Vec<A2uiCatalogEntry>,
   }
   ```
3. Add `pub a2ui_catalog: Arc<A2uiCatalog>` field to `CompiledState`
4. Add `ReflectionEngine::load_a2ui_catalog()` to `crates/fdb-reflection/src/engine.rs`:
   - Use `sqlx::query_as()` (dynamic, NOT `query_as!()` macro — avoids build-time DB dep)
   - Graceful degradation: if `flint_a2ui` schema absent → return `A2uiCatalog { components: vec![], ... }`
   - `#[instrument(skip(pool), err)]` span
5. Update `StateManager::do_compile()` in `state_manager.rs`:
   - Call `engine.load_a2ui_catalog(&pool).await?` after `engine.reflect()`
   - Pass `Arc::new(catalog)` into `CompiledState` constructor
6. Update all `CompiledState` consumers identified in step 1 (struct update only)
7. Add Cedar `a2ui:emit` stub check in `fke-server` (can be `todo!()` for now)
8. `cargo check --workspace` must pass
9. `cargo clippy --workspace -- -D warnings` must pass

---

### Wave 4 — SDK Schema Extensions

**Change 5: `p5-c014-sdk-schema-extensions`**  
*Priority: P0 | Agent: database-reviewer + rust-reviewer | Estimated scope: Medium*

**Pre-execution correction (R2):** Fix `resolve_components_with_overrides()` RLS policy to use `application_id` (not `app_id`) and remove reference to `role_name` (which doesn't exist on `role_assignments`). Use `roles.id` join instead.

Tasks:
1. Write `migrations/0003_flint_a2ui_sdk_extensions.sql`:
   - `ALTER TABLE flint_a2ui.components ADD COLUMN renderers jsonb DEFAULT '{"react":true,"flutter":true,"htmx":false}'::jsonb`
   - `ALTER TABLE flint_a2ui.components ADD COLUMN react_pkg text`
   - `ALTER TABLE flint_a2ui.components ADD COLUMN flutter_pkg text`
   - `ALTER TABLE flint_a2ui.components ADD COLUMN htmx_template text`
   - `ALTER TABLE flint_a2ui.design_systems ADD COLUMN source_format text`
   - `ALTER TABLE flint_a2ui.design_systems ADD COLUMN source_content text`
   - `CREATE TABLE flint_a2ui.component_overrides` with RLS
   - `CREATE FUNCTION flint_a2ui.resolve_components_with_overrides(p_app_id uuid)` — use `ra.application_id` (not `ra.app_id`), join via `roles.id` (not `role_name`)
2. Create `crates/fdb-app/src/a2ui/` module:
   - `crates/fdb-app/src/a2ui/mod.rs` (pub mod types)
   - `crates/fdb-app/src/a2ui/types.rs` with:
     - `ResolvedComponent { slug, renderers, schema, overrides }`
     - `Renderers { react: bool, flutter: bool, htmx: bool }`
     - `DesignToken { value: serde_json::Value, token_type: String }` (W3C $value/$type)
3. Add `pub mod a2ui;` to `crates/fdb-app/src/lib.rs`
4. `cargo check -p fdb-app` passes

---

### Wave 5 — SDK Packages (parallel)

**Change 6: `p5-c010-react-sdk`**  
*Priority: P0 | Agent: typescript-reviewer | Estimated scope: XLarge*

**Bundle constraint:** < 80kb gzipped (microsite budget from performance.md).

Tasks:
1. Create `packages/flint-react/` directory with npm package scaffold:
   - `package.json` (`@flint/react`, `peerDependencies: react@^19`)
   - `tsconfig.json` (strict, composite)
   - `tsup.config.ts` (esm + cjs, dts, tree-shakeable)
2. Core provider layer (`src/core/`):
   - `FlintProvider.tsx` — context + AG-UI SSE connection
   - `FlintRegistry.ts` — Zod schema registry; `register(slug, propsSchema, renderer)`
   - `FlintSurface.tsx` — renders A2UI surface descriptor from AG-UI SSE `a2ui:surface` events
   - `FlintAgUiAdapter.ts` — parses SSE `data:` lines, type-narrows A2UI nodes
3. Design token injection (`src/tokens/`):
   - `useDesignTokens.ts` — fetches tokens from `/a2ui/v1/design-systems/:id`, injects as CSS custom properties on `data-flint-surface`
4. Component implementations (`src/components/`):
   - One file per category subdirectory
   - All 55 components with Zod prop schemas
   - Radix-style headless: behavior layer (`useFlintButton`, etc.) + default render
   - CSS custom properties via `data-flint-*` attributes only
5. Exports: `index.ts` — named exports only, no default barrel that blocks tree-shaking
6. `SKILL.md` — component slug reference table for Claude Code usage
7. Bundle size check: `pnpm build && gzip -c dist/index.js | wc -c` must be < 80000

**Change 7: `p5-c011-flutter-sdk`**  
*Priority: P0 | Agent: flutter-reviewer | Estimated scope: XLarge*

**genui pin:** `genui: 0.9.2` exactly (not range) in pubspec.yaml.

Tasks:
1. Create `packages/flint_genui/` Dart package:
   - `pubspec.yaml`: `genui: 0.9.2`, `cue: ^0.3.11`, `http: ^1.2`, `web: ^1.0` (no Firebase/Gemini)
2. Transport layer (`lib/src/transport/`):
   - `FlintA2uiTransport` — Dart `http` SSE client to fdb-gateway; emits `A2uiSurface` stream; no Gemini
3. Catalog registration (`lib/src/catalog/`):
   - `FlintCatalog.build()` — registers all 55 CatalogItem entries
   - Each entry: `CatalogItem(slug, builder: (ctx, props) => FlintXxx(...))`
4. Component widgets (`lib/src/widgets/`):
   - One file per category
   - All 55 widgets extending `genui` base types where applicable
   - `cue` animations on `FlintSurface` transitions and feedback components
5. Theme extension (`lib/src/theme/`):
   - `FlintThemeData` extending `ThemeExtension<FlintThemeData>`
   - `resolveToken(String tokenKey)` method using W3C `$value`/`$type` lookup
6. `FlintSurface` widget — receives `A2uiSurface` from transport and dispatches to catalog
7. `example/` app with `FlintProvider` usage demonstration

---

## Extended Execution Wave (P1 — after MVP)

### Change 8: `p5-c005-application-model`
*Priority: P1 | Agent: rust-reviewer | Estimated scope: Medium*

Rust use-case layer over `flint_a2ui.applications`:
- `ApplicationUseCase` in `crates/fdb-app/src/a2ui/`
- `resolve_components()` calling `flint_a2ui.resolve_components_with_overrides()`
- Depends on: c001 (schema), c009 (CompiledState), c014 (types)

### Change 9: `p5-c006-rest-api`
*Priority: P1 | Agent: rust-reviewer | Estimated scope: Medium*

`/a2ui/v1/` route group in `fdb-gateway`:
- `GET /a2ui/v1/components` — paginated catalog (uses `CompiledState.a2ui_catalog`)
- `GET /a2ui/v1/components/:slug` — single component with overrides
- `GET /a2ui/v1/applications` — application list
- `GET /a2ui/v1/design-systems` — design system list
- `POST /a2ui/v1/design-systems` — import design tokens
- All routes protected by `fdb-auth` JWT middleware + Cedar `a2ui:read` check
- Depends on: c001, c005, c009

### Change 10: `p5-c004-embeddings-pipeline`
*Priority: P1 (deferred pending OQ-10) | Agent: database-reviewer | Estimated scope: Small (stub)*

OQ-10 is unresolved. Ship a functional stub:
- Implement real `flint_a2ui.semantic_search()` body once OQ-10 resolves
- Until then: `RAISE NOTICE 'semantic_search: embeddings pipeline not yet active'`
- Stub records a `flint_a2ui.events` entry with `event_type = 'search_attempted'`
- Depends on: c001

### Change 11: `p5-c007-event-driven-assembly`
*Priority: P1 | Agent: rust-reviewer | Estimated scope: Medium*

Assembly rules engine + AG-UI event dispatch:
- `AssemblyEngine` in `fdb-app/src/a2ui/assembly.rs`
- Reads `flint_a2ui.assembly_rules` for a given surface + context
- Emits SSE `a2ui:surface` events per AG-UI transport spec
- **OQ-Iggy:** Iggy event bus dispatch stubbed with `todo!()` until resolved
- Depends on: c001, c002, c006

### Change 12: `p5-c008-protocol-surfaces`
*Priority: P1 | Agent: rust-reviewer | Estimated scope: Medium*

A2A task catalog and MCP tool server stubs:
- Define 16 A2A tasks in `fdb-app/src/a2ui/tasks.rs` as enum + descriptor
- MCP tool manifest at `openspec/mcp/a2ui-tools.json` (7 tools per spec §8)
- Wire into `fdb-gateway` as `/a2ui/v1/mcp` endpoint stub
- Depends on: c006, c009

---

## Extended Execution Wave (P2 — after P1 or in parallel where deps allow)

### Change 13: `p5-c012-htmx-renderer`
*Priority: P2 | Agent: rust-reviewer | Estimated scope: Small*

Axum + Askama renderer in `fdb-gateway`:
- Add `askama` crate to `fdb-gateway/Cargo.toml`
- `GET /a2ui/htmx/:slug` renders component preview as HTML fragment
- Template: `templates/a2ui/component.html.jinja`
- Admin/prototype surface only — no RLS requirement (service_role auth)
- Depends on: c001, c002

### Change 14: `p5-c013-opendesign-integration`
*Priority: P2 | Agent: rust-reviewer | Estimated scope: Medium*

`DESIGN.md` parser + design token importer:
- `OpenDesignParser` in `fdb-app/src/a2ui/design.rs`
- Parses W3C Design Token format from `DESIGN.md` frontmatter block
- Inserts into `flint_a2ui.design_systems.source_content` + `tokens` JSONB
- OpenDesign plugin installed via GitHub-path (OQ-14 resolution)
- Depends on: c014

### Change 15: `p5-c015-claude-design-skill`
*Priority: P2 | Agent: doc-updater | Estimated scope: Small*

Claude Code skill for UI generation:
- `skills/flint-ui/SKILL.md` — component slug reference, usage patterns, prop schema examples
- `skills/flint-ui/plugin.json` — skill marketplace manifest
- References all 55 component slugs from c002 + renderers from c014
- Depends on: c010 (React slug confirmations), c002 (all slugs)

---

## Execution Order Summary

```
Wave 1 (serial):    c001   ← spec defect R1 fixed inline
                      │
Wave 2 (parallel):  c002, c003   ← c003 requires c002 in same DB session
                      │
Wave 3 (serial):    c009   ← consumer audit mandatory first
                      │
Wave 4 (serial):    c014   ← spec defect R2 fixed inline
                      │
Wave 5 (parallel):  c010, c011

--- MVP COMPLETE ---

Wave 6 (parallel):  c005, c004(stub), c012
                      │
Wave 7 (serial):    c006   ← needs c005
                      │
Wave 8 (parallel):  c007, c008, c013
                      │
Wave 9:             c015   ← needs c010 slug confirmations
```

---

## Pre-Flight Checklist

Before executing any change:

- [ ] `cargo check --workspace` passes on current codebase (baseline)
- [ ] Postgres 18 instance available with `postgresql-18-pgvector` installed (confirm Dockerfile used)
- [ ] Confirm `sqlx` version in use: dynamic `query_as()` form available (no `DATABASE_URL` at build)
- [ ] Confirm `fdb-gateway/Cargo.toml` does not already pull `askama` (avoid dup for c012)
- [ ] OQ-12 (agui_descriptor GRANT scope) confirmed: stay `service_role` only — do NOT add `authenticated`

---

## Risk Mitigations in Execution

| Risk | Applied Where | Mitigation |
|------|--------------|------------|
| R1: `is_system` missing | c001 | Fixed in migration DDL before writing SQL |
| R2: column mismatch in c014 RLS | c014 | Fixed in migration SQL before writing |
| R3: `query_as!` macro requires live DB | c009 | Use `query_as()` dynamic form exclusively |
| R4: `genui ^0.9.2` alpha | c011 | Exact version pin `0.9.2` in pubspec.yaml |
| R5: CompiledState consumer audit | c009 | Step 1 in c009 task list (mandatory) |
| R6: OQ-10 unresolved | c004 | Stub body only; table exists from c001 |
| R7: OQ-Iggy unresolved | c007 | Event dispatch is `todo!()` stub |
| R8: a2ui module missing from lib.rs | c014 | Explicit task step in c014 |

---

## Carried Technical Debt (from prior phases)

These are non-blocking for Phase 5 but must be resolved before SDKs ship to production:

1. **PgVectorRpc serializer** — response rows use text format; needs typed JSON deserialization before pgvector surface is exposed via React/Flutter SDK
2. **VectorRpcRequest.filter** — silently ignored; must attach as `WHERE` clause before c006 ships
3. **FabricChangeSource** — empty stream stub until FRF RPC lands (OQ-FRF-1)
4. **KetoSyncTask** — `_keto_cache` not wired to subscription handler (OQ-Iggy)
