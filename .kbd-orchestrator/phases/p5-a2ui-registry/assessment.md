# Assessment — p5-a2ui-registry

**Date:** 2026-06-30  
**Assessed by:** kbd-assess  
**Spec refs:** `docs/FLINT-A2UI-REGISTRY-SPEC.md` (RFC-FORGE-A2UI-001), `openspec/changes/p5-c001` through `p5-c015`

---

## 1. Summary

Phase 5 builds the Flint A2UI component registry on top of the completed Phase 2 substrate
(`CompiledState`, `PgVectorRpc`, OpenAPI compiler). The spec is well-designed and internally
consistent. **Zero implementation exists** for any p5 deliverable — everything must be built.

MVP gate is 7 of 15 changes (c001, c002, c003, c009, c014, c010, c011). The dependency
ordering is strict: database schema first (c001), then seed (c002) and trigger (c003) can run
in parallel, then SDK schema extensions (c014), then React (c010) and Flutter (c011).

The most significant structural gap: **`CompiledState` (p5-c009)** requires surgery to both
`fdb-reflection/src/compiled.rs` and `state_manager.rs` — these are the two files at the hot
path of every request. The field rename (`openapi_doc: serde_json::Value` → `openapi_doc: Arc<...>`
and adding `a2ui_catalog: Arc<A2uiCatalog>`) is a compile-breaking change that will cascade to
all compiled-state consumers in `fdb-gateway`. A careful reader-impact list is needed before
executing c009.

Three open questions gate specific changes and must be recorded before plan.

---

## 2. Current State vs. Required State

### 2.1 Database Schema (p5-c001)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `flint_a2ui` schema | SQL migration `migrations/0002_flint_a2ui.sql` | **No migrations dir at all** | Create dir + migration |
| `flint_a2ui.components` table | pgvector-backed JSONB catalog table | None | Full build |
| `flint_a2ui.applications` table | Application metadata | None | Full build |
| `flint_a2ui.design_systems` table | Token-aware design systems | None | Full build |
| `flint_a2ui.embeddings` table | vector(1536) with HNSW index | None | Full build |
| `flint_a2ui.schemas`, `bindings`, `events` | Supporting tables | None | Full build |
| `flint_a2ui.assembly_rules`, `roles`, `role_assignments` | Permission/assembly tables | None | Full build |
| `flint_a2ui.semantic_search()` | Stub SQL function (real impl in c004) | None | Stub only in c001 |
| RLS policies on `components`, `events` | GUC-backed row security | None | Full build |
| `CREATE EXTENSION IF NOT EXISTS vector` | pgvector in migration | Dockerfile already installs `postgresql-18-pgvector` ✅ | Migration SQL only |
| `is_system` column on `applications` | Used in seed INSERT | Spec mentions but p5-c001 DDL omits it | **Spec gap — add to DDL** |

**Finding F1:** `migrations/` directory does not exist. Must be created.

**Finding F2:** p5-c001 proposal seeds base applications with an `is_system` column that the
DDL block does not define. Either add `is_system boolean NOT NULL DEFAULT false` to the table
DDL or fix the seed INSERT to not reference it.

### 2.2 Base Components Seed (p5-c002)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `scripts/seed_a2ui_components.sql` | Idempotent SQL seed (50+ rows) | **scripts/ dir exists but no seed SQL** | Write the full 55-component seed |
| All 7 categories present | layout, data-display, input, action, navigation, feedback, system | None | Full build |
| `flint-meta-schema` system component | Special non-renderable system entry | None | Include in seed |
| Seed applied on fdb-gateway startup | Migration or init script | Depends on migration strategy | Coordinate with c001 |

### 2.3 Auto-Binding Trigger (p5-c003)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `flint_a2ui.auto_generate_bindings()` PL/pgSQL function | Fires on `flint_meta.cache_tables` INSERT | None | Full build |
| `a2ui_auto_bind_tables` trigger | AFTER INSERT ON `flint_meta.cache_tables` | None | Full build |
| `flint_a2ui.column_type_to_component()` | Type-mapping helper | None | Full build |
| `ext-flint-meta/agui.rs` `protocol` label fix | `'ag-ui/1.0'` → `'flint-forge/schema-descriptor/1.0'` | `agui.rs:83` currently returns `'ag-ui/1.0'` | One-line fix in agui.rs + update test assertion |

**Finding F3:** `ext-flint-meta/src/agui.rs:83` has the protocol label `'ag-ui/1.0'` and
`agui.rs:253` has a test asserting this value. Both must be updated as part of p5-c003.

**Finding F4:** The trigger function references `flint_a2ui.components` and `flint_a2ui.events`
— both must exist (c001 + c002 applied) before the trigger can be installed. Execution order:
c001 → c002 → c003.

### 2.4 CompiledState Upgrade (p5-c009)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `CompiledState.a2ui_catalog: Arc<A2uiCatalog>` | New field in `fdb-reflection/src/compiled.rs` | `compiled.rs` has no such field | Add field + type definition |
| `A2uiCatalog` / `A2uiCatalogEntry` types | New structs in `compiled.rs` or `lib.rs` | None | Define in `fdb-reflection` |
| `ReflectionEngine::load_a2ui_catalog()` | New async method on engine | None | Add to `engine.rs` |
| `StateManager::do_compile()` populates catalog | `do_compile` in `state_manager.rs` | Calls `engine.reflect()` only; does not populate `a2ui_catalog` | Extend `do_compile` |
| `CompiledState` consumers updated | All uses of `openapi_doc` or old field names | `fdb-gateway` reads `openapi_doc` from state | Audit all consumers |
| Cedar `a2ui:emit` check in `fke-server` | New authorization guard | None | Stub or full impl |
| Graceful degradation when `flint_a2ui` absent | Falls back to empty catalog | None | Required per spec |

**Finding F5:** `CompiledState` has 5 fields currently. Adding `a2ui_catalog` requires updating
`StateManager::do_compile()` (line 105-123 of `state_manager.rs`) — a single call site. However,
any callers that pattern-match or destructure `CompiledState` will also need updating.

**Finding F6:** p5-c009's spec uses `sqlx::query_as!` macro against `flint_a2ui.components`.
The codebase currently uses a mix of `sqlx::query_as()` (dynamic) and `sqlx::query_as!()` (macro).
For Phase 5 offline compilation, `query_as!()` requires a live DB with `flint_a2ui` installed.
Safe to use `query_as()` (dynamic) for the catalog load to avoid build-time DB dependency.

### 2.5 SDK Schema Extensions (p5-c014)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `migrations/0003_flint_a2ui_sdk_extensions.sql` | ALTER TABLE + new table | None | Full build |
| `components.renderers` column | JSONB `{"react":true,"flutter":true,"htmx":true}` | None | ALTER in migration |
| `component_overrides` table | Per-app/per-design-system prop overrides | None | Full build |
| `design_systems` import metadata columns | `source_format`, `source_content`, etc. | None | ALTER in migration |
| `resolve_components_with_overrides()` function | SQL function with override merge | None | Full build |
| Rust types: `ResolvedComponent`, `Renderers`, `DesignToken` | In `fdb-app/src/a2ui/types.rs` | `fdb-app/src/` exists but no `a2ui/` module | Create `crates/fdb-app/src/a2ui/` module |

**Finding F7:** `fdb-app` has no `a2ui` submodule. Must create `crates/fdb-app/src/a2ui/mod.rs`
+ `types.rs`. Check that `fdb-app/src/lib.rs` is updated to declare the new module.

**Finding F8:** The `component_overrides` RLS policy uses `ra.app_id` and `ra.role_name` but
the `role_assignments` table DDL (from c001) uses `application_id` and references `roles.id` (not
`role_name`). Column name mismatch in c014 policy — must be reconciled with c001 DDL.

### 2.6 React SDK (p5-c010)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `packages/flint-react/` npm package | Full TypeScript library | **No `packages/` dir** | Create package + full implementation |
| `FlintProvider`, `FlintSurface`, `FlintRegistry` | Core components | None | Full build |
| 63 component implementations | All categories | None | Full build |
| AG-UI SSE adapter | `FlintAgUiAdapter.ts` | None | Full build |
| Design token injection | CSS custom properties | None | Full build |
| Zod schema registry | `propsSchema` per component | None | Full build |
| Bundle size < 80kb gzipped | Performance budget | Not applicable (not built) | Enforce at build |
| `SKILL.md` for Claude Code | Component slug reference | None | Write after components defined |

**Finding F9:** No `packages/` directory. This is a new deliverable outside the Rust workspace.
Will need a separate `tsconfig.json`, `package.json`, `tsup` build config. No dependency on any
Rust workspace crate for the SDK itself (it talks to REST/SSE endpoints).

### 2.7 Flutter SDK (p5-c011)

| Item | Required | Exists | Gap |
|------|----------|--------|-----|
| `packages/flint_genui/` Dart package | Full Flutter package | No `packages/` dir | Full build |
| `FlintCatalog.build()` | 63 CatalogItem registrations | None | Full build |
| `FlintA2uiTransport` | Pure SSE, no Gemini | None | Full build |
| `cue ^0.3.11` animations | Surface + component animations | None | Full build |
| `FlintThemeData` ThemeExtension | Token resolver | None | Full build |
| pubspec.yaml | `genui: ^0.9.2`, `cue: ^0.3.11` | None | Write |

**OQ-13 impact:** `genui ^0.9.2` is alpha — API may change before Phase 5 ships. Pin to exact
version `0.9.2` in pubspec.yaml lock.

---

## 3. Extended Change Analysis (P1 / P2)

### p5-c004 — Embeddings Pipeline

Blocked by OQ-10 (text-embedding-3-large via liter-llm not confirmed). Can stub the DB function
with a no-op that `RAISE NOTICE` logs until OQ-10 is resolved. The `flint_a2ui.embeddings` table
and HNSW index are already in c001 — only the embedding generation procedure is gated.

### p5-c005 — Application Model

The `applications` and `roles`/`role_assignments` tables land in c001. This change adds the Rust
use-case layer and the `resolve_components()` function. Can be combined with c014's
`resolve_components_with_overrides()` if c005 ships first.

### p5-c006 — REST API

Component CRUD via Quarry REST compiler: the existing REST compiler generates routes from
`flint_meta.cache_tables`. After c001 lands, `flint_a2ui.*` tables will be reflected and routes
will appear automatically — but they will use the raw RLS-gated table access, not the custom
`resolve_components_with_overrides()` function. A dedicated `/a2ui/v1/` route prefix needs to
be added in `fdb-gateway` to expose the registry API with proper permission filtering.

### p5-c007 — Event-Driven Assembly

The `assembly_rules` table lands in c001. The Rust assembler is new work in `fdb-gateway` or
`fdb-app`. AG-UI event dispatch via Iggy still gates on OQ-Iggy (not yet resolved).

### p5-c008 — Protocol Surfaces

A2A task definitions and MCP tool server. Requires `fdb-gateway` routes from c006 and the
catalog from c009. Can be stubbed in this phase with `todo!()` bodies.

### p5-c012 — HTMX Renderer

Axum+Askama renderer in `fdb-gateway`. Lowest risk — no new crates. Admin/prototype surface only.

### p5-c013 — OpenDesign Integration

Gated by OQ-14 (GitHub-path install for now) and depends on c014 (SDK schema). `DESIGN.md`
parser in `fdb-app`. Confirmed not blocked if OpenDesign plugin marketplace is deferred.

### p5-c015 — Claude Design Skill

`skills/flint-ui/SKILL.md` + plugin manifest. Pure documentation/skill authoring — no Rust code.
Lowest risk in entire phase.

---

## 4. Dependency Map

```
p5-c001-flint-a2ui-schema
    ↓
  ┌─────────────────────────────────────────┐
  │                                         │
  p5-c002-base-components-seed       p5-c003-auto-binding-trigger
  │                                         │
  └──────────────────┬──────────────────────┘
                     │
              p5-c009-compiled-state-upgrade  ← also needs p5-c001
                     │
              p5-c014-sdk-schema-extensions   ← also needs p5-c001
                     │
       ┌─────────────┴──────────────────┐
       │                                │
p5-c010-react-sdk               p5-c011-flutter-sdk

p5-c004-embeddings-pipeline ← p5-c001 + OQ-10 resolution
p5-c005-application-model   ← p5-c001
p5-c006-rest-api             ← p5-c001, p5-c005
p5-c007-event-driven-assembly ← p5-c001, p5-c002, p5-c006
p5-c008-protocol-surfaces    ← p5-c006, p5-c009
p5-c012-htmx-renderer        ← p5-c001, p5-c002
p5-c013-opendesign-integration ← p5-c014
p5-c015-claude-design-skill   ← p5-c010 (references SDK slugs)
```

---

## 5. Risk Register

| ID | Risk | Severity | Likelihood | Mitigation |
|----|------|----------|------------|------------|
| R1 | `is_system` column missing from c001 DDL | HIGH | Confirmed gap | Add in c001 migration before any other change |
| R2 | RLS column mismatch (`app_id` vs `application_id`) in c014 | HIGH | Confirmed gap | Fix c014 policy to match c001 schema |
| R3 | `sqlx::query_as!()` macro in c009 requires live DB at compile time | MEDIUM | High | Use dynamic `query_as()` for catalog load |
| R4 | `genui ^0.9.2` alpha API instability | MEDIUM | Medium | Pin exact version `0.9.2` in pubspec.lock; isolate adapter layer |
| R5 | `CompiledState` consumer audit incomplete | MEDIUM | Medium | Search all `compiled.` references in `fdb-gateway/src/` before c009 |
| R6 | OQ-10 (liter-llm embeddings) not resolved | LOW | High | Stub c004 with RAISE NOTICE; unblock rest of phase |
| R7 | OQ-Iggy not resolved | LOW | Medium | Stub c007 event dispatch; implement rules table only |
| R8 | `fdb-app/src/a2ui/` module not declared in lib.rs | LOW | Certain | Add module declaration as first step of c014 |

---

## 6. Pre-Kickoff Gate Status

### OQ-10: text-embedding-3-large via liter-llm

**Status: UNRESOLVED — DEFER p5-c004**  
Gate advice: Stub `flint_a2ui.embed_components()` as a no-op that logs a notice. The
`flint_a2ui.embeddings` table and HNSW index in c001 are sufficient to unblock all other
changes. Embeddings pipeline activates when OQ-10 is resolved.

### OQ-13: `genui ^0.9.2` stability

**Status: ADVISORY — PROCEED with pin**  
Lock `genui` to `0.9.2` exactly in pubspec.lock. The package is alpha but this exact version
is confirmed functional for A2UI SSE integration.

### OQ-14: OpenDesign plugin marketplace

**Status: ADVISORY — PROCEED with GitHub-path install**  
p5-c013 does not require marketplace availability. GitHub-path install is the correct approach
for Phase 5. Document in c013 proposal.

---

## 7. Schema Discrepancy Inventory (Spec vs. OpenSpec)

The RFC (docs/FLINT-A2UI-REGISTRY-SPEC.md) has a richer schema than the OpenSpec proposals:

| Spec table | OpenSpec (c001) | Delta |
|------------|-----------------|-------|
| `components` | Simplified (12 columns) | RFC has 20+ columns; OpenSpec is the authoritative scope |
| `applications` | Missing `is_system` column | **Gap** — add to c001 |
| `design_systems` | In c001 | RFC `source_url`, `source_type`, `design_md` land in c014; c001 keeps minimal |
| `roles` / `role_assignments` | In c001 | Aligned |
| `schemas` table | RFC §5.1.5 | **Not in c001 scope** — omitted intentionally |
| `type_component_map` | RFC §7.2 | **Not in any c00X scope** — deferred or delivered by c003 |
| `function_component_map` | RFC §7.3 | Not in any c00X scope — deferred |
| `audit_log` | RFC §13.3 | Not in Phase 5 scope — Phase 6/7 |

OpenSpec changes take precedence over the RFC for scope. RFC tables not in any p5-cNNN are
deferred to later phases.

---

## 8. Files to Create or Modify (Phase 5 Impact)

### New directories / files
```
migrations/                               ← p5-c001 (create directory)
migrations/0002_flint_a2ui.sql            ← p5-c001
migrations/0003_flint_a2ui_sdk_extensions.sql ← p5-c014
scripts/seed_a2ui_components.sql          ← p5-c002
packages/                                 ← p5-c010, p5-c011 (new directory)
packages/flint-react/                     ← p5-c010 full package
packages/flint_genui/                     ← p5-c011 full package
crates/fdb-app/src/a2ui/mod.rs            ← p5-c014
crates/fdb-app/src/a2ui/types.rs          ← p5-c014
```

### Modified files
```
crates/fdb-reflection/src/compiled.rs     ← p5-c009 (add a2ui_catalog field + types)
crates/fdb-reflection/src/engine.rs       ← p5-c009 (add load_a2ui_catalog() method)
crates/fdb-reflection/src/state_manager.rs ← p5-c009 (update do_compile)
crates/fdb-app/src/lib.rs                ← p5-c014 (declare a2ui module)
crates/ext-flint-meta/src/agui.rs         ← p5-c003 (protocol label: line 83 + test line 253)
images/postgres18/Dockerfile              ← p5-c001 (add vector extension CREATE — already has apt pkg ✅)
```

---

## 9. Conclusions

1. **Phase 5 is buildable** — no blocking architectural gaps. All technical prerequisites
   from prior phases are confirmed delivered.

2. **Two spec defects** must be fixed before plan: the missing `is_system` column (R1) and
   the RLS column-name mismatch in c014 (R2).

3. **MVP is 7 changes** — c001 → (c002 + c003 in parallel) → c009 → c014 → (c010 + c011
   in parallel). All can proceed without OQ-10 or OQ-13 resolution.

4. **p5-c009 is the highest-risk MVP change** — it touches the hot-path compiled state and
   breaks all `CompiledState` consumers. Require full consumer audit before executing.

5. **SDK changes (c010, c011) are net-new outside the Rust workspace** — no Cargo.toml
   impact; they live in `packages/` as independent TypeScript and Dart projects.

6. **Extended changes (c004–c008, c012–c015)** are all parallel after MVP lands. OQ-10
   is the only unresolved blocker; all others can proceed.
