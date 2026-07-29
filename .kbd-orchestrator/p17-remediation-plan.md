# Fix the San Saba defect report — make flint-forge usable as an agentic Supabase replacement

## Context

San Saba Workspace filed a defect report (2026-07-28) after a single `PATCH` to
one row — the core interaction of their POC — was blocked by **four independent
defects stacked on top of each other**. The signature symptom: **every write
returned 403 or 502 while every read returned 200.**

Fixes exist on `fix/authz-optional-keto-and-param-types` (4 commits, pushed, not
merged, clippy/fmt clean, 112 tests passing). This plan reviews that branch,
closes the gaps it left, and lands it.

Review turned up findings the report did not have, and one reframes Defect 3.

### The central finding: the fix already exists and is already proven

`crates/fdb-reflection/src/compilers/filters.rs` contains a complete casting
toolkit — `CastHints`, `cast_hints_for`, `render_where_with_hints`,
`MutationBind`, `mutation_value_to_bind`, `mutation_placeholder`,
`bind_mutation_value` — **already verified against a live Postgres** by
`crates/fdb-reflection/tests/rest_typed_columns_live_pg.rs` for `int4`, `int8`,
`bool`, `uuid`, and embed-scoped child filters. That test asserts
`handle_update`'s exact shape at lines 238 and 242:

```
active = $1::boolean          // mutation_placeholder
WHERE id = $2::integer        // render_where_with_hints
```

**No production handler calls any of it.** A usage census found
`render_where_with_hints` has **zero** production call sites;
`cast_hints_for`, `mutation_value_to_bind`, `mutation_placeholder`,
`bind_mutation_value`, and `MutationBind` are dead outside tests. Only
`cast_hints_for_table` is wired, in `embed_schema.rs:33`.

Defect 3 is therefore not "add casting" — it is **"connect the handlers to the
casting layer that already exists and already passes."**

### Casts are inconsistent within a single request

Embedded child filters cast (`fdb-query/src/embed/render.rs:273` applies hints);
top-level filters do not. On one request, `?child.id=eq.5` works and `?id=eq.5`
fails.

### `json_bind` + `placeholder_for` produce a hard Postgres error

`json_bind` (`rest/responses.rs:45`) has two arms: `String` → `Text`, everything
else → `Json`. So `{"qty": 5}` binds as `QueryParam::Json("5")`, while the
branch's new `placeholder_for` emits `$1::int4` from the column type alone with
no knowledge of the bind channel. Result: `$1::int4` over a `jsonb` bind →
**`cannot cast type jsonb to integer`**.

`mutation_value_to_bind` + `mutation_placeholder` cannot produce this, because
the cast decision keys off the *bind variant*: a JSON `5` becomes `Text("5")`
and correctly receives `$1::int4`. They must be adopted **together** —
`CastHints` retains `json`/`jsonb` hints that `needs_no_cast` excludes, so
pairing `CastHints` with `json_bind` alone would reintroduce the `::jsonb`
string-quoting corruption already tried and reverted.

### `needs_no_cast` is measurably weaker than the shared implementation

`mutations.rs::needs_no_cast` vs `fdb-query/src/cast.rs::is_text_compatible`:

| | `needs_no_cast` | `is_text_compatible` |
|---|---|---|
| Case sensitivity | raw match | lowercases first |
| Type modifiers | none — `varchar(255)` → bogus `$1::varchar(255)` | strips via `base_type_name` |
| Multi-word types | `character varying` only | also `double precision`→`float8`, `timestamp with time zone`→`timestamptz`, `bit varying`→`varbit` |
| Identifier validation | **none** — splices `pg_type` unvalidated | `validate_identifier` before splicing |
| Array/quantifier | none — only `$n` / `$n::type` | `array_placeholder` → `$n::int4[]` for `in`/`any`/`all` |

The multi-word gap hits `timestamptz` directly — one of the three types the
report flags as unverified.

### Why it all stayed hidden

`rest_typed_columns_live_pg.rs`'s header (lines 19–24) says it deliberately
tests the query-builder layer instead of the HTTP router, because a
route-registration bug made every HTTP request 500. **That bug was fixed by
p16-c001** — routes now bind schema/table by closure capture — but nobody
revisited the test. `rest_rls_isolation.rs` documents both bugs in comments
(lines 131–139, 160–165) and uses `text` for every column to route around them.
Two tests, each avoiding the untested seam, and CI green.

**Research grounding.** Supabase's [2025 Security
Retro](https://supabase.com/blog/supabase-security-2025-retro) states their 2026
plan treats Zanzibar/OpenFGA as an *optional* add-on, not a request-path
dependency — supporting default-OFF Keto. Their answer to RLS-less exposed
tables is layered warnings (labels, alerts, the Splinter lint, RLS-on-by-default)
and **never refusing to serve** — supporting warn-by-default with opt-in strict.

### Decisions taken (confirmed with user)

| Question | Decision |
|---|---|
| Defect 3 scope | Unify on `filters.rs` helpers across all four verbs |
| Defect 4 default | Accept default-OFF (`FLINT_AUTHZ_MODE=rls`) as proposed |
| RLS exposure | Warn loudly + opt-in strict mode |
| Delivery | New KBD phase `p17` |

---

## Pre-work

**Tooling blockage: RESOLVED (2026-07-29).** The `pre_mutation` gate in
`prometheus-skill-pack/hooks/hooks.json` has been removed by the operator.
Shell access is confirmed working; the source tree is clean and cargo 1.96.1 is
available. Nothing here blocks the phase.

*Correction to an earlier diagnosis in this plan's own history:* the blockage
was **not** caused by the stale `active_phase` field. The guard
(`shared/scripts/kbd-harness-adapter.sh:32-36`) walks up from `$PWD` looking for
`.prometheus/project.json` and never reads `.kbd-orchestrator/project.json`,
`active_phase`, or the phases directory at all. That misattribution is recorded
here so it is not repeated.

**Ordinary bookkeeping cleanup** (not blockers, do whenever convenient):

- `.kbd-orchestrator/project.json` has `active_phase:
  "p16-v1.0-release-closure"`, but the only phase on disk is
  `p16-production-remediation`.
- The `p16-c008` entry in `deliverables` still reads *"still open, blocked on
  operator action"* while its own status is `completed` with
  `verified_live_deploy: true`.

**Still open, deferred:** the KBD control plane returns 401 for this project's
token (written 19:08 by `migrate --apply`; daemon restarted 20:27 into
`--mode server`). Rejection arrives in 0.6ms — an auth failure, not the 180ms
timeout. Irrelevant while the gate is removed, but it must be reissued before
the gate is ever re-enabled.

Then open phase `p17`.

---

## Change 1 — accept `4a23b29` + `738f9df`, add the missing schema test

**`4a23b29` (error unwrapping)** — `crates/fdb-postgres/src/error.rs`,
`backend.rs`. Independent, strictly an improvement. Accept unchanged.

**`738f9df` (keto-sync column)** — the `object_id AS object` alias is correct
against `crates/ext-flint-meta/sql/flint_meta.sql`, where the column is
`object_id`, part of the composite PK and of `keto_tuples_object_idx`.
`prime()` is implemented and wired. Accept unchanged.

**Add the test the report suggested but did not write.** `keto_sync.rs` has
*zero* coverage of the SQL it fixed — its 6 unit tests cover only the in-memory
cache and the interval parser, so the column mapping can only fail against a
live database, on a write. Add a `DATABASE_URL`-gated test in
`crates/fdb-gateway/tests/` asserting:

- `prime()` returns the correct tuple count (proves the column mapping), and
- `prime()` returns `Err` when the table is missing — the panic path
  `bootstrap.rs` depends on, currently unverified.

---

## Change 2 — connect the handlers to the existing casting layer

> **STATUS (2026-07-29): DONE, but the scope was wrong.** This was planned as a
> wiring change — "the fix already exists, just call it." Executing it against a
> live Postgres 18 found **four** distinct root causes, three of them
> pre-existing and one of them introduced by the first fix. The plan's premise
> (that `filters.rs` was already correct and merely unreferenced) was itself
> mistaken: `rest_typed_columns_live_pg.rs` "proved" that layer through **sqlx**,
> which declares bind types explicitly, while production uses **tokio-postgres**,
> which *infers* them. The proof did not transfer.
>
> | # | Root cause | Origin |
> |---|---|---|
> | 1 | Handlers never called the casting layer | the branch's gap (as planned) |
> | 2 | `$n::type` fails client-side — the cast sets the *parameter's* inferred type, so the driver rejects the bound `String` before sending | shipped in `render_where_with_hints`, never exercised |
> | 3 | `$n::text::T[]` infers scalar `text`, mismatching the `Vec<String>` an `in.(…)` filter binds | introduced by the fix for #2 |
> | 4 | **No `COMMIT` anywhere — every write silently rolled back** | pre-existing, workspace-wide |
>
> **#4 is the significant one and it was not in this plan at all.**
> `PgBackend::acquire` issues `BEGIN` (required for `SET LOCAL`), but nothing
> committed and no `Drop` guard existed. Writes *looked* successful because
> `RETURNING` reads inside the doomed transaction; deadpool then rolled it back
> on recycle. Verified against a live database. It affected **REST CRUD, `/rpc`,
> GraphQL mutations, and Kiln's `flint:host/db`** — broader than the San Saba
> report, which only covered REST.
>
> Had Change 2 shipped as planned, San Saba's `PATCH` would have returned `200`
> and still lost the write. The report would have read as resolved while data
> quietly vanished.
>
> **Fixes applied beyond the plan:**
> - `mutation_placeholder` → `$n::text::{t}`; `scalar_placeholder` → same;
>   `array_placeholder` → `$n::text[]::{t}[]` (`fdb-query/src/operator.rs`).
>   Two more in `fdb-query/src/mutation.rs`, which is dead code today (zero call
>   sites) — fixed for consistency, not a live fix.
> - `PgConn::commit()` (`fdb-postgres/src/conn.rs`), called from
>   `rest.rs::run_bound`, `backend.rs::execute_sql`, and `graphql.rs::execute`.
>   Chosen over a `Drop` guard because `Drop` cannot be async: a drop-time commit
>   would have to block the runtime or spawn a detached task whose failure
>   disappears — unacceptable on the path that enforces RLS.
> - Transaction contract documented on `DatabaseBackend::acquire` in `fdb-ports`.
> - `MutationBind::to_query_param` — the missing conversion that explains why the
>   toolkit was written but never wired: it had only a *sqlx* binder, while the
>   handlers hand `Vec<QueryParam>` to `execute_raw`.
>
> **`json_bind` was NOT deleted**, contrary to the instruction below. `rpc.rs:85`
> uses it correctly: `/rpc` binds *function arguments*, which have no reflected
> column to cast to. Its doc was rescoped to `/rpc`-only.

Supersedes commit `d837ac7` rather than extending it. Target: one casting
implementation, not two.

**Delete** `placeholder_for` and `needs_no_cast` from
`crates/fdb-reflection/src/compilers/rest/mutations.rs`. **Delete** `json_bind`
from `rest/responses.rs` and its re-export in `rest/mod.rs:28`.

**Adopt the `filters.rs` pairing** — `mutation_value_to_bind` +
`mutation_placeholder` + `bind_mutation_value` together, per the `json`/`jsonb`
caveat above. Build hints once per request with
`cast_hints_for(&state.model, &schema, &table)`.

**Convert all four production `render_where(` call sites** to
`render_where_with_hints`:

| File:line | Handler | Model access |
|---|---|---|
| `rest/mutations.rs:235` | `handle_update` WHERE | direct — already reads `state.model` at `:223` |
| `rest/mutations.rs:280` | `handle_delete` WHERE | direct |
| `rest/list.rs:146` | `build_inner_query`, non-embed | thread `state.model` in from `handle_list` |
| `rest/list.rs:167` | `build_inner_query`, embed top-level WHERE | same |

`build_inner_query(schema, table, params, embed_schema)` takes no model today;
its caller `handle_list` has `state: RestState`. Add the parameter.

**`PgRest` is out of scope.** `crates/fdb-postgres/src/rest.rs` owns no mutation
SQL — it implements only `RestExecutor::execute` (read-only) and `execute_raw`
(already-rendered pairs from `fdb-reflection`), and holds a `Pool` with no
`DatabaseModel`, so it cannot build hints without a signature change.
`mutations.rs` is the sole owner of mutation SQL text, so there is no counterpart
to keep in sync. Bringing `PgRest`'s read path to parity is a separate change —
note it, don't do it here.

**Preserve the hard-won invariant:** never blanket-`::jsonb`. The `jsonb → text`
assignment cast preserves JSON quoting, turning `tenant-a` into `"tenant-a"` and
breaking any RLS `WITH CHECK` comparison. `mutation_placeholder` already refuses
to cast a `MutationBind::Json`, which is what makes the pairing safe.

**Fix `bind_param`'s `BigInt` trap** (`filters.rs:145-161`): it has no `BigInt`
arm, so a `QueryParam::BigInt` falls into the catch-all and binds **NULL**.
Harmless today only because `BigInt` params flow through tokio-postgres'
`RestBind` path — a live trap the moment `bind_param` gains a caller on a
LIMIT-bearing query.

**Document the canonicalization dependency:**
`passes::normalization::canonicalize_pg_type` rewrites `int4`→`integer` before
the cast is built, so the cast target is the canonical name. Add a test
asserting every canonical form it can emit is a valid Postgres cast target.

Delete the stale `json_bind` doc claim that typed-column binding is "a known,
separately-tracked gap".

---

## Change 3 — accept `de0f103` (`FLINT_AUTHZ_MODE`), close its gaps

`crates/fdb-gateway/src/authz_mode.rs` is sound: pure resolver, 7 unit tests,
case-insensitive, unrecognized value is a hard error rather than a silent
default, legacy `FLINT_KETO_MUTATION_GATE` honoured only when the new var is
unset. `bootstrap.rs:162-197` opens no extra pool and spawns no poll task in
`rls` mode, and `prime()` fails fast on both an unreadable **and an empty** tuple
cache. Accept the design and the default.

1. **Write the CHANGELOG + MIGRATION entry** the report flags as missing.
   Breaking change: existing Keto deployments must set
   `FLINT_AUTHZ_MODE=rls+keto`. Document the `ListenChangeSource::new` signature
   change (`KetoConfig` → `Option<KetoConfig>`) in the same entry — a public API
   break in a shared crate, and the only way to make the subscription path
   optional.
2. **Replace `panic!` with `anyhow` at the binary edge.** `bootstrap.rs:136`,
   `175`, `181` panic on config errors; CLAUDE.md requires `anyhow` only at
   binary entry points. Have `run()` return `anyhow::Result<()>`.
3. **Delete the dead `Quarry` cluster** — larger than the report's one item.
   `execute_rest_mutation`, `with_keto`, `check_keto`, and the `keto` field are
   unreachable from any `src/`, kept alive only by `fdb-app/tests/gate_tests.rs`
   and `keto_check_test.rs`. Those tests are worse than dead: they are the *only*
   tests exercising a mutation Keto gate, giving false confidence about a path
   production never takes. The live gate `mutation_guard` has its own four tests
   at `mutations.rs:384`.
4. **Document the one true tuple shape** — `entities` / `insert|update|delete` /
   `schema.table`. Tuples seeded against the dead path's `"mutate"` / bare-table
   shape can never match.
5. **Make `ListenChangeSource`'s Keto skip match `FabricChangeSource`'s.** Fabric
   wraps the skip in a guarded private method and its doc explains why ("a future
   caller cannot forget it"); `ListenChangeSource::watch` uses an inline
   call-site `if let` — the pattern that doc argues against — and has no
   equivalent "skipped when not configured" test, despite `listen` now being the
   default source. Add the guard and the test.

---

## Change 4 — RLS exposure: warn loudly, opt-in strict mode

Not in the report's proposed scope, but it is the actual Broken Access Control
exposure and it is orthogonal to Keto — Keto's object is the *table*, not the
row, so Keto never protected these tables either.

**Precedent already exists and is inconsistent:** the GraphQL compiler already
skips non-RLS tables (`compilers/graphql.rs:95-98`, `if !table.rls_enabled {
continue; }`) while REST mounts them unconditionally. Same model, opposite
posture — the strongest argument for the change.

`passes/permission_analysis.rs` already warns per table and re-runs on every
hot-reload (via `engine.rs:58` ← `StateManager::do_compile`); its own comment
says *"Phase 2: warn only. Phase 4 Cedar policy check will block."* — never
implemented. Build on it:

- **Aggregate the warning** into one startup line naming every exposed table
  rather than one line per table.
- **Add a gauge** for the count of exposed policy-less tables, following the
  existing `telemetry` module pattern, so it is alertable rather than greppable.
- **`FLINT_REQUIRE_RLS=strict`** (default permissive) — refuses to mount routes
  for tables without RLS. The filter point is a single `continue` at the top of
  `generate()`'s table loop in `passes/endpoint_generation.rs`; both
  `EndpointKind` variants already carry the full `Table`. `generate(model)` takes
  no config, so add `generate_with_policy(model, strict)` and keep `generate` as
  a delegating wrapper — mirroring the existing `compile` / `compile_with_gates`
  pair at `compilers/rest/mod.rs:69,76`. One production call site.
  Mirror `authz_mode.rs` for the env parsing: pure resolver, unit-tested,
  unrecognized value a hard error.

**Strengthen the signal before gating on it.** `rls_enabled` alone is weak, for
two confirmed reasons:

- **`relforcerowsecurity` is never reflected.** `ext-flint-meta/src/triggers.rs`
  captures only `c.relrowsecurity`, so a table owned by the connecting role with
  RLS enabled but not FORCEd records `rls_enabled: true` while actually bypassing
  every policy. (Migration `0013_force_rls.sql` from p16-c001 applies FORCE to
  known tables, but nothing reflects or verifies it.)
- **Policies are collected but unreachable.** `flint_meta.cache_policies` is
  populated by `full_refresh()` (`triggers.rs:335-352`) but has **no accessor
  SRF**, nothing in Rust reads it, and `refresh_cache()` never updates it —
  `CREATE POLICY` isn't even in the event trigger's tag list
  (`triggers.rs:398-405`), so it goes stale immediately after any policy change.

So "RLS enabled, zero policies" — which denies everything for non-owners and
presents as the same "writes 403" symptom San Saba hit — is invisible today.
Closing this means a `flint_meta.policies()` SRF, a `CREATE POLICY`/`DROP POLICY`
branch in `refresh_cache()` plus the trigger tag list, `relforcerowsecurity` added
to `cache_tables`, and `policy_count` + `rls_forced` on `Table`.

**Scope call:** ship the warning, the gauge, and the reflection improvements
first; gate `FLINT_REQUIRE_RLS=strict` on the strengthened signal. Shipping
strict mode against `rls_enabled` as it stands today would under-block
(non-FORCEd owner tables, zero-policy tables) — the exact false assurance this
change is meant to remove.

---

## Change 5 — prove it through the HTTP layer

`rest_typed_columns_live_pg.rs` already proves the *query-builder* layer. What no
test covers is the **HTTP handler path** — precisely the seam Change 2 touches,
and precisely what San Saba exercised.

**Extend `rest_typed_columns_live_pg.rs`** to drive the compiled `RestCompiler`
router end-to-end, now that p16-c001 fixed the route-registration bug its header
cites as the reason for avoiding HTTP (confirmed stale:
`rest_rls_isolation.rs` drives real HTTP successfully, and
`compilers/rest/mod.rs:93-101` documents the fix). Harness to copy from
`rest_rls_isolation.rs`:

- `DATABASE_URL`-gated via `fn database_url() -> Option<String>` with an early
  `return`, **not** `#[ignore]` — this is what lets CI pick it up automatically;
- a second `authenticated`-role probe so it skips cleanly without
  `ext-flint-auth`;
- two pools — a raw setup pool for DDL/verification, a `PgRest`-wrapped pool as
  the executor under test, so verification reads bypass RLS while the exercised
  path does not;
- a **unique ephemeral schema per test fn**, since `#[tokio::test]` fns in one
  file run concurrently against the same database;
- `tower::ServiceExt::oneshot` with `Extension<RlsContext>` inserted directly;
- `tracing_subscriber::fmt().with_test_writer()` so a handler's real Postgres
  error surfaces instead of a bare 500.

Cover `date`, `numeric`, `uuid`, `bool`, `timestamptz`, `int4`, `int8`, plus
`text` and `jsonb` as must-not-regress controls — the report flags `numeric`,
`uuid`, and `timestamptz` as unverified. Add `varchar(255)` and `double
precision` to catch the two `needs_no_cast` gaps. For each:

- `POST` inserts it;
- `GET ?col=eq.<value>` filters on it (**the WHERE path the branch never fixed**);
- `PATCH ?id=eq.<uuid>` updates one typed column while filtering on another;
- `DELETE ?id=eq.<uuid>` removes it;
- a JSON **number** and **bool** body value land as real numeric/boolean values —
  the case that currently errors `cannot cast type jsonb to integer`;
- a `text` value round-trips as `tenant-a`, never `"tenant-a"`;
- `?id=in.(1,2)` renders `$n::int4[]`, covering the array path
  `placeholder_for` never handled;
- a top-level filter **and** an embed child filter in the same request both cast.

**Update `rest_rls_isolation.rs`** to use a `uuid` or `int4` primary key and
delete the workaround comments at lines 131–139 and 160–165 — converting it from
documenting the bug to proving it fixed.

**CI needs no changes.** `.github/workflows/ci.yml`'s `integration` job builds
`flint-forge-pg:18` from `images/postgres18/Dockerfile` (bundling `ext-flint-auth`,
pgvector, pg_graphql), sets `DATABASE_URL`, and runs `./scripts/ci-test.sh`,
whose db stage applies migrations, seeds A2UI, and runs `cargo test --workspace`
— which picks up any env-skip-gated test automatically.

---

## Verification

Cheapest gate first:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo fmt --all --check
```

```bash
cargo test --workspace
```

Then the live-database gate — the only thing that can prove Changes 2 and 5:

```bash
./scripts/ci-test.sh
```

Manual reproduction of San Saba's original failure, against a running gateway
with `FLINT_AUTHZ_MODE` unset (the new default):

```bash
curl -sS -X PATCH "$FORGE/sansaba/letter_agreement?id=eq.$ROW_ID" -H "Authorization: Bearer $JWT" -H 'content-type: application/json' -d '{"receipt_date":"2026-07-01"}' -i
```

Success criteria:

1. That `PATCH` returns `200`, not `403` or `502` — the report's exact scenario,
   on a fresh deployment with no Keto and no seeded tuples.
2. `receipt_date` reads back as `2026-07-01`; a `text` column written in the same
   request reads back unquoted; a numeric column written from a JSON number reads
   back as a number.
3. With `FLINT_AUTHZ_MODE=rls+keto` and an empty `flint_meta.keto_tuples`, the
   gateway **refuses to start** with an actionable message instead of booting and
   403ing every write.
4. `FLINT_AUTHZ_MODE=rls+ketto` (typo) fails startup hard.
5. A table with `rls_enabled: false` produces a startup WARN naming it; with
   `FLINT_REQUIRE_RLS=strict` its routes are not mounted.
6. A deliberate column rename in `flint_meta.keto_tuples` fails the new keto-sync
   test rather than silently emptying the cache.
7. `grep -rn "render_where(" crates/*/src/` returns no production call sites —
   one casting path, not two.

---

## Sequencing

| Order | Change | Rationale |
|---|---|---|
| 0 | Pre-work: open p17 (tooling already unblocked) | Bookkeeping only — no longer a blocker |
| 1 | Change 1 (accept 2 commits + keto schema test) | Independent; error unwrapping makes everything else diagnosable |
| 2 | Change 5 harness — HTTP-level typed test, failing | Write it first so Change 2 is verified, not assumed |
| 3 | Change 2 (connect handlers to `filters.rs`) | The core fix; now provable |
| 4 | Change 3 (accept `de0f103` + gaps) | Design already settled |
| 5 | Change 4 (RLS exposure) | Independent; largest unknown (policy reflection); safe to land last |

Changes 1–3 unblock San Saba. If Change 4 slips, it does not hold the rest.

## Defects found but deliberately NOT fixed here (2026-07-29)

Both are real, both are pre-existing, and both are outside p17's scope. Recorded
rather than silently folded in.

1. **`rest_router_extraction.rs` fails against a live database.**
   `assert_list_ok` builds its request with no `.extension(rls)`, while the
   sibling POST/PATCH helpers include one. `handle_list` requires
   `Extension<RlsContext>`, so axum rejects with `500` before the handler body —
   which is why the diagnostic run emitted no `tracing::error!` at all.
   **Proven pre-existing**: stashing every p17 source change and re-running
   reproduces it identically at `HEAD`. It surfaced only because this phase
   started a Postgres container; the test is `DATABASE_URL`-gated and had been
   silently skipping. One-line fix (add the extension), but it belongs to
   whoever owns that test.

2. **`render_inner_guards` applies no cast hints** (`fdb-query`).
   `render_projection` casts embed-scoped child filters via
   `re.cast_hints.qualified(&re.child_alias)` (`embed/render.rs:272`), but
   `render_inner_exists` does not. So `?select=*,orders!inner(*)&orders.total=gt.100`
   still binds uncast against a typed child column. Fixing it means threading
   child hints into `render_inner_exists` — an `fdb-query` API change.

### The pattern worth naming

Three separate tests in this codebase passed by **never running the code they
claimed to cover**: `rest_rls_isolation.rs` (every column `text`, with a comment
saying why), `rest_typed_columns_live_pg.rs` (drops below HTTP, citing a bug
p16-c001 had already fixed), and `rest_router_extraction.rs` (skipped without a
database). The uncommitted-transaction bug survived because *every* existing
test asserts on handler responses, which read inside the transaction being
discarded. Only a verification query on a **second connection** could see it.

Recommended follow-up: audit `DATABASE_URL`-gated tests for whether CI actually
runs them, and prefer independent-connection verification for any assertion
about persistence.

## Risk note

Change 2 deletes a shipped, reviewed commit's implementation in favor of an
existing one. The justification is concrete, not stylistic: `filters.rs`'s
version is already proven against a live Postgres for more types, is the only
one that avoids `cannot cast type jsonb to integer` on JSON numbers/bools,
validates the type identifier before splicing it into SQL, strips type
modifiers, handles array/quantifier placeholders, and reaches the WHERE clause
the branch's version cannot. Landing Change 5's failing HTTP test *before*
Change 2 is what makes the substitution safe rather than a leap of faith.

One investigated-and-dismissed concern, recorded so it is not re-raised:
`engine.rs:71-72` destructures 3 columns from the 5-column `flint_meta.tables()`
SRF, which looks like a positional mismatch that would silently swap
`rls_enabled` for `is_view`. It is not — the query names its columns explicitly
(`SELECT schema_name, table_name, rls_enabled FROM ...`), so Postgres resolves
them by name. The risk would be real only for `SELECT *`.
