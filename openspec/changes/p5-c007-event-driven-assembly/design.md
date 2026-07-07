# p5-c007 Design — Event-Driven Component Assembly

## Context

`p5-c006` added the `/a2ui/v1/*` REST API surface, including `POST /a2ui/v1/surfaces/assemble`, which currently returns HTTP 501. This change implements the Rust assembler that the endpoint delegates to.

The assembler lives in `fdb-reflection`, the same crate that already compiles REST/GraphQL/OpenAPI artifacts and hosts the hot-swappable `A2uiCatalog` loaded from `flint_a2ui.components`. The DB already contains:

- `flint_a2ui.components` — base + app-specific component definitions with JSON Schema props.
- `flint_a2ui.bindings` — auto-generated table→component bindings (grid/form/detail/card) from `p5-c003`.
- `flint_a2ui.assembly_rules` — user-defined rules mapping event types + filters to assembly config.
- `flint_a2ui.resolve_components(app_id, jwt_claims)` — permission-filtered component resolution from `p5-c005`.

The assembler consumes these tables and emits a valid A2UI v0.9.1 message sequence (`createSurface` → `updateComponents` → `updateDataModel`).

## Goals / Non-Goals

**Goals:**
- Implement `A2uiAssembler` in `fdb-reflection/src/compilers/a2ui.rs`.
- Produce A2UI v0.9.1 messages from an `AssemblyContext` (event type, payload, app, JWT claims).
- Support explicit `assembly_rules` matches with priority ordering.
- Provide a safe default path: when no rule matches, generate a grid surface from `flint_a2ui.bindings` for the event's source table.
- Wire the assembler into `POST /a2ui/v1/surfaces/assemble` in `fdb-gateway`.
- Keep single-surface assembly under 500ms (indexed DB lookups only).

**Non-Goals:**
- Full Phase 7 AG-UI streaming integration is out of scope; the assembler returns/serializes the surface, and Phase 7 will emit it.
- AI/LLM-driven component selection is out of scope; selection is rule-driven or binding-driven.
- Nested multi-surface orchestration is out of scope; one event yields one surface.

## Decisions

### 1. Place the assembler in `fdb-reflection/src/compilers/a2ui.rs`

**Rationale:** `fdb-reflection` already owns DB-derived artifact generation (REST, GraphQL, OpenAPI, A2UI catalog) and has access to `A2uiCatalog`. Adding the assembler here keeps the hexagonal layering intact and lets it reuse the in-memory catalog snapshot when available.

**Alternative considered:** A new `fdb-a2ui` crate. Rejected because the assembler is tightly coupled to `flint_a2ui` schema reflection and would duplicate pool/catalog wiring.

### 2. `A2uiAssembler` holds only a `PgPool`, not the full `StateManager`

**Rationale:** Keeps the assembler usable from tests and from the gateway without dragging in the hot-swap compilation graph. The gateway can construct it with `A2uiAssembler::new(pool)`.

**Trade-off:** We lose the in-memory `A2uiCatalog` cache on every call. Mitigation: all lookups hit indexed tables (`assembly_rules`, `bindings`, `resolve_components()`), so the extra round trips remain well under the 500ms SLA.

### 3. Query rules with a single parameterized SQL query ordered by priority

```sql
SELECT assembly_config, event_filter
FROM flint_a2ui.assembly_rules
WHERE application_id = $1
  AND event_type = $2
  AND is_active = true
ORDER BY priority ASC, created_at ASC;
```

The Rust code evaluates `event_filter` JSONB predicates against the event payload in-process. The first matching rule wins.

**Rationale:** Keeps the hot path to a single indexed query; complex JSONB predicate evaluation is easier to unit-test in Rust than in SQL.

### 4. Default binding path maps `public.orders` → `DataGrid`

When no rule matches and the event payload carries `data_source = {"schema":"public","table":"orders"}`, the assembler:
1. Looks up `flint_a2ui.bindings` for `(public, orders, grid)`.
2. Falls back to `(public, orders, form)` if no grid binding exists.
3. Loads the bound component via `resolve_components(app_id, jwt_claims)`.
4. Emits an `updateComponents` message with a single root component of type `DataGrid` (or `Form`) bound to the table metadata.

**Rationale:** Mirrors the auto-generated bindings from `p5-c003` and gives every table a sensible default UI without custom rules.

### 5. A2UI messages are typed as an enum, serialized to JSON at the edge

```rust
pub enum A2uiMessage {
    CreateSurface(CreateSurface),
    UpdateComponents(UpdateComponents),
    UpdateDataModel(UpdateDataModel),
}
```

**Rationale:** Strong typing prevents malformed messages; serialization is deferred to the HTTP/AG-UI boundary.

### 6. Iggy integration is optional and feature-gated behind runtime discovery

The assembler accepts an optional `IggyPublisher` trait object. If it is `None`, the surface is returned synchronously. If it is `Some`, the surface is also published to the `a2ui.surfaces` topic.

**Rationale:** Phase 3 Iggy integration may not be available in all deployments. This avoids a hard dependency and lets the gateway inject the publisher when ready.

## Risks / Trade-offs

- **[Risk] Rule JSONB predicates evaluated in Rust may diverge from Postgres semantics.** → Mitigation: keep predicates simple (`{"data_source.table": "orders"}`) and add unit tests covering equality, presence, and nested paths.
- **[Risk] Default grid binding may expose columns the user cannot see.** → Mitigation: `resolve_components()` already filters by role assignments; data binding uses the same RLS context as the REST layer.
- **[Trade-off] In-process catalog cache is not used.** → Acceptable because assembly is rule-driven and uses indexed lookups; we can add catalog caching later without changing the public API.
- **[Risk] Event payload shape is not yet standardized across tools.** → Mitigation: define a minimal `AssemblyEvent` envelope with `event_type`, `data_source`, and `payload`; unknown shapes fall back to the default grid binding.

## Migration Plan

1. Add migration is not required — schema already exists.
2. Deploy `fdb-reflection` + `fdb-gateway` changes.
3. `POST /a2ui/v1/surfaces/assemble` changes from 501 to returning an A2UI surface; existing clients that handled 501 will start receiving payloads.
4. Rollback: revert the two crates; the endpoint returns 501 again.

## Open Questions

1. Should rule event filters support full JSONPath, or only dotted key equality?
2. Do we need a `deleteSurface` message path for teardown events, or is that Phase 7's responsibility?
3. Should the default binding prefer `card` over `grid` for small tables (<5 columns)?
