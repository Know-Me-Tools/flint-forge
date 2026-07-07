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

### T10 — Resource embedding ✅
- [x] FK-join planner from a caller-supplied `EmbedSchema` (fdb-query stays pure;
      fdb-reflection will map `DatabaseModel` -> `EmbedSchema`). `select=*,rel(*)`.
- [x] `!fk` disambiguation, `!inner`, embedded filter/order, top-level-by-embedded,
      `...spread`, nested embedding. Correlated json_agg / json_build_object subselects;
      `EXISTS` guards for `!inner` and top-level-by-embedded.
- [x] SECURITY: adversarial review found + fixed an unvalidated `parent_alias`/
      `parent_table` path; `resolve_embeds` now validates them (regression test added).

### T11 — Full-text search ✅
- [x] `fts`/`plfts`/`phfts`/`wfts` → to_tsquery/plainto/phraseto/websearch mapping via
      the `@@` operator; language option `fts(english)`; tsquery text bound as a param,
      regconfig validated; escaping tests.

### T12 — Edge-case hardening ✅
- [x] Empty `in`, null in `is`/`in`, composite PK, reserved-char values, `limit=0`,
      large offset, quoted commas, order-by-embedded — covered in the edge test suite.
- [x] SECURITY: adversarial review found + fixed unvalidated relation/SET-column in
      `UpdatePlan`/`DeletePlan::render`; both now validate (regression tests added).

### T13 — Parity verification (integration checkpoint) ✅
- [x] Full suite green: `cargo test -p fdb-query` (128 lib + 29 integration),
      `-p fdb-postgres` (4), `-p fdb-reflection` (46 + gates); `cargo check --workspace`
      clean; `cargo clippy -p fdb-query -p fdb-postgres -p fdb-reflection --all-targets
      -- -D warnings` clean. (Built via multi-agent workflow; every claim re-verified
      against the compiler by the orchestrator, incl. 2 adversarial-review security fixes.)
