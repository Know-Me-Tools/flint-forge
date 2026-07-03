# Tasks — p3-c019-postgrest-query-engine

Execution model: **core-complete first, then parity.** Integration-First — build the
whole translator + wire both consumers, then verify (test-waits reserved for the two
pass boundaries). `fdb-query` is pure, so it is unit-testable without a DB — those
tests are the authoritative correctness gate and run cheaply on `cargo test -p fdb-query`.

## Phase 1 — Core-complete

### T1 — Scaffold `fdb-query` crate
- [x] New crate `crates/fdb-query`: `#![forbid(unsafe_code)]`, deps `serde`, `serde_json`,
      `thiserror` only. Add to workspace members. No DB driver, no async.
- [x] Public API sketch: `QueryPlan`, `parse_select_request(params, headers) -> Plan`,
      `parse_mutation_request(...)`, `Plan::render() -> (String, Vec<QueryParam>)`.
- [x] `QueryParam` enum (Text/Int/Bool/Json/Vector-agnostic → adapter binds).

### T2 — Identifier + value safety layer (do FIRST; everything depends on it)
- [x] Hardened identifier validator (schema/table/column/alias/relation/cast/json-key).
- [x] Parameter model: values ALWAYS bound as `$n`; renderer tracks the bind list.
- [x] Injection test suite (adapt/extend `is_safe_identifier` tests) — must be green
      before any operator lands.

### T3 — Horizontal filtering operator set
- [x] Operator enum + parser: eq neq gt gte lt lte like ilike match imatch in is
      isdistinct cs cd ov sl sr nxr nxl adj.
- [x] `not.` negation; `any()`/`all()` modifiers.
- [x] Per-operator render + unit test asserting exact `(sql, params)`.
- [x] `like`/`ilike` pattern handling; `in`-list parsing (incl. empty, quoted, null).

### T4 — Logical trees
- [x] `and`/`or` recursive parser; nested groups; top-level `not.and`/`not.or`.
- [x] Nested-tree render tests.

### T5 — select / order / pagination / count
- [x] `select`: column list, `alias:col`, `col::type`, JSON paths `->`/`->>`.
- [x] `order`: multi-column, asc/desc, nullsfirst/nullslast.
- [x] `limit`/`offset`; `Range` header parse; `Content-Range` compute.
- [x] `Prefer: count=exact|planned|estimated` → count strategy in plan.

### T6 — Writes
- [x] Bulk INSERT; UPSERT (`resolution=merge-duplicates`, `on_conflict`).
- [x] PATCH/DELETE with filter reuse (T3/T4).
- [x] `Prefer: return=representation|minimal`, `missing=default`.

### T7 — Wire `fdb-reflection` REST router onto `fdb-query` ✅
- [x] Replace `compilers/filters.rs::build_where` usage with `fdb-query`.
- [x] Keep handler behavior identical; existing REST tests stay green.
- [x] Remove/retire the superseded `build_where` (filters.rs is now a thin bridge).
- [x] Port the RFC-FORGE §3.3/G6 security gate to the bridge API (all guarantees intact).

### T8 — Implement `PgRest::execute` over `fdb-query`
- [x] `PgRest::execute`: parse `RestQuery` → `fdb-query` plan → render → run under
      `backend.acquire(rls)` (6-GUC RLS) → project rows to `RestResult`.
- [x] Removes the `todo!()`; the p3-g4 subscription RLS re-query is now live.
- [x] Row→JSON projection reuses the `PgVectorRpc` pattern (typed → JSON).

### T9 — Phase 1 verification (integration checkpoint)
- [x] `cargo check --workspace` clean.
- [x] `cargo clippy -p fdb-query -p fdb-postgres -p fdb-reflection -p fdb-gateway -- -D warnings`
      clean. (Full-workspace clippy trips a PRE-EXISTING, unrelated lint in the
      `hello-component` example crate — macro-generated WASI bindings, `used_underscore_items`
      — not introduced by this change.)
- [x] `cargo test -p fdb-query -p fdb-postgres -p fdb-reflection` (69 + 4 + 46 unit + gates).
- [ ] (DB-backed integration tests where a test PG is available.)

## Phase 2 — Parity

### T10 — Resource embedding
- [ ] FK-join planner from `DatabaseModel` FK metadata; `select=*,rel(*)`.
- [ ] `!fk` disambiguation, `!inner`, embedded filter/order, top-level-by-embedded,
      `...spread`, nested embedding.

### T11 — Full-text search
- [ ] `fts`/`plfts`/`phfts`/`wfts` → to_tsquery/plainto/phraseto/websearch mapping,
      language option `fts(english)`, escaping tests.

### T12 — Edge-case hardening
- [ ] Empty `in`, null in `is`/`in`, composite PK, reserved-char values, `limit=0`,
      large offset, order-by-embedded. Property tests where valuable.

### T13 — Parity verification (integration checkpoint)
- [ ] Full suite green; parity checklist in proposal.md satisfied.
