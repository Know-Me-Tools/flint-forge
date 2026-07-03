# p3-c019 — PostgREST-parity query engine (`fdb-query`)

## Change ID
`p3-c019-postgrest-query-engine`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
**G3** — Full RLS CRUD in the REST surface. This change replaces the ad-hoc
`filters::build_where` with a single, authoritative, PostgREST-compatible
request→SQL translator, and completes `PgRest::execute` (currently `todo!()`)
so the RLS re-query path and the standalone REST adapter are both real.

## Problem
Two REST query-building code paths exist or are stubbed:
1. `fdb-reflection`'s `compilers/filters.rs::build_where` — partial operator set,
   coupled to the reflection router, operates on `sqlx::PgPool`.
2. `fdb-postgres`'s `PgRest::execute` — `todo!("PostgREST-compatible query
   builder + pgvector /rpc")`, the deadpool-based adapter used by the GraphQL
   subscription RLS re-query (p3-g4) and any direct REST-executor consumer.

Maintaining two translators for a security-critical surface guarantees drift: an
operator hardened in one path but not the other becomes an injection or a
filter-bypass. The requirement is **full PostgREST parity, no shortcuts**, on a
single authoritative translator.

## Decision
Extract the translator into a new **pure, I/O-free crate `fdb-query`** (Layer 0/1:
`serde` + string output, zero DB driver, zero async). It parses PostgREST request
grammar into a typed query plan and renders `(sql, params)` with every identifier
validated and every value bound as `$n`. Both `fdb-reflection` (REST router) and
`fdb-postgres` (`PgRest`) consume it. RLS is enforced by the *executor* (the
6-GUC `backend.acquire(rls)`), never by the translator — the translator is
oblivious to identity, which keeps it pure and testable.

## Scope — full PostgREST parity (delivered core-complete first, then parity pass)

### Core pass (this change, phase 1)
- **Horizontal filtering — all operators:** `eq, neq, gt, gte, lt, lte, like,
  ilike, match, imatch, in, is, isdistinct, cs, cd, ov, sl, sr, nxr, nxl, adj`.
- **Negation & modifiers:** `not.` prefix; `any()`/`all()` modifiers on the
  scalar comparison operators.
- **Logical trees:** `and`, `or`, arbitrarily nested (`and=(a.gt.1,or=(...))`),
  incl. top-level `not.and` / `not.or`.
- **Vertical filtering (`select`):** column lists, renaming (`alias:col`), casts
  (`col::type`), JSON paths (`col->key`, `col->>key`, `field->>0`).
- **Ordering:** `order` multi-column, `.asc`/`.desc`, `.nullsfirst`/`.nullslast`.
- **Pagination & count:** `limit`/`offset`, `Range`/`Range-Unit` header,
  `Content-Range` response, `Prefer: count=exact|planned|estimated`.
- **Writes:** bulk INSERT, UPSERT (`Prefer: resolution=merge-duplicates`,
  `on_conflict`), PATCH/DELETE with filters, `Prefer: return=representation|minimal`,
  `Prefer: missing=default`.
- **RPC:** `/rpc/<fn>` args (GET query params + POST body), scalar/set/table return.
- **Safety (non-negotiable):** hardened identifier validator (schema/table/column/
  relation/alias/cast/json-path); FTS query-string escaping; `in`-list and `like`
  pattern quoting; every value a bound parameter. Output is `(String, Vec<QueryParam>)`.

### Parity pass (this change, phase 2)
- **Resource embedding** (the defining PostgREST feature): `select=*,other(*)` via
  FK joins, FK disambiguation (`!fk`), inner joins (`!inner`), embedded filtering,
  ordering on embedded, top-level filtering by embedded (`?other.col=...`), spread
  (`...other(col)`), nested embedding to depth.
- **Full-text search:** `fts`, `plfts`, `phfts`, `wfts` with language/config option
  (`fts(english)`), correct `to_tsquery`/`plainto_tsquery`/`phraseto_tsquery`/
  `websearch_to_tsquery` mapping.
- **Edge cases:** empty `in` list, `null` handling in `is`/`in`, composite PKs,
  quoted values with reserved chars, unicode identifiers where Postgres allows,
  `limit=0`, large offset, ordering by embedded aggregate.

## Out of Scope
- The p3-g4 GraphQL subscription seam (separate branch/PR; consumes `PgRest`).
- The in-process `LISTEN` `ChangeStreamSource` (separate change).
- Non-Postgres backends (the translator targets Postgres SQL dialect only).

## Acceptance Criteria
### Core pass
- [ ] `fdb-query` crate exists (pure, no DB driver, `#![forbid(unsafe_code)]`).
- [ ] All listed horizontal operators + negation + any/all implemented, each with a
      unit test asserting exact `(sql, params)` including parameter binding.
- [ ] Nested `and`/`or` trees parse and render correctly (property/nested tests).
- [ ] `select` (rename, cast, json path), `order`, `limit`/`offset`, count modes.
- [ ] Writes: bulk insert, upsert, patch/delete-with-filter, `Prefer` handling.
- [ ] Identifier validator rejects every injection vector in a dedicated test suite.
- [ ] `fdb-reflection` REST handlers use `fdb-query` (old `build_where` removed).
- [ ] `PgRest::execute` implemented over `fdb-query` + `backend.acquire(rls)`; the
      GraphQL subscription re-query path is live (no `todo!()`).
- [ ] `cargo check --workspace`, `cargo clippy --workspace -- -D warnings`,
      `cargo test -p fdb-query -p fdb-postgres -p fdb-reflection` all green.

### Parity pass
- [ ] Resource embedding (FK join, `!fk`, `!inner`, embedded filter/order,
      top-level-by-embedded, spread, nested) with tests.
- [ ] FTS variants with language option, correct tsquery mapping, escaping tests.
- [ ] Edge-case suite green.

## Security Notes
- The translator never sees `RlsContext`; RLS is applied by the executor's 6 GUCs.
- Identifiers validated against schema metadata where available, and always against
  the hardened character/shape validator. Values are ALWAYS bound, never interpolated.
- FTS and pattern inputs are escaped per Postgres rules; a fuzz/property test guards this.
