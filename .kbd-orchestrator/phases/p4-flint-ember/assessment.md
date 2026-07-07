# Assessment — p4-flint-ember

Generated: 2026-07-03
Status: assessment_complete

## Phase context

Flint Ember (`ext-flint-llm` / `flint_llm`) is the in-database LLM/embeddings layer. Per `docs/FLINT-FORGE-SPEC.md` §4.3 it exposes:

- **Surface 1 (sync):** `llm.embed(text, model)` and `llm.complete(prompt, opts)` — interrupt/timeout-safe, read/explicit path only.
- **Surface 2 (async):** a pgrx background worker dequeuing `llm.jobs` (`FOR UPDATE SKIP LOCKED`), plus declarative provisioners `llm.enable_embedding(...)` and `llm.enable_summary(...)`.
- **Sovereign routing:** all model calls route inward through flint-gate / UAR; the DB holds no provider keys in plaintext (resolved via `flint_vault`).

## What already exists

| Area | State | Evidence |
|---|---|---|
| `ext-flint-llm` crate skeleton | scaffolded | `crates/ext-flint-llm/src/lib.rs`, `Cargo.toml`, `flint_llm.control`, `sql/flint_llm.sql` |
| pgrx 0.18.1 / PG18 toolchain | configured | `crates/ext-flint-llm/Cargo.toml`; matches `ext-flint-meta`/`ext-flint-vault` |
| `llm.jobs` queue table | schema only | `crates/ext-flint-llm/sql/flint_llm.sql` |
| `flint_vault` secret resolver | implemented | `crates/ext-flint-vault/src/lib.rs` exposes `vault.resolve_api_key(provider, scope)` and `vault.get_secret(name, scope)`; `flint_llm_worker` role is pre-created |
| pgvector / `flint_a2ui.embeddings` | exists | `migrations/0002_flint_a2ui.sql` defines `vector(1536)` HNSW index; semantic search stub present |
| Postgres image build | partial | `images/postgres18/Dockerfile` builds `flint_llm` pgrx package and installs it; first-boot SQL creates extension |
| `shared_preload_libraries` config | inconsistent | `postgresql.flint.conf` lists `pg_net,ext_flint_llm`; `Dockerfile` `CMD` only preloads `pg_net,pg_cron` |
| `CompiledState` / `StateManager` hot-swap | delivered | `crates/fdb-reflection/src/state_manager.rs` |
| `flint_meta` schema cache | delivered | `crates/ext-flint-meta` |

## Gaps by goal

### G1 — `ext-flint-llm` pgrx extension with liter-llm binding

- **No liter-llm dependency or bridge.** `crates/ext-flint-llm/Cargo.toml` has a TODO comment and no actual liter-llm / reqwest / serde_json / tokio deps. Need to decide whether `liter-llm` is an external crate we import, a workspace crate we create, or an HTTP client speaking the flint-gate/UAR protocol directly.
- **No flint-gate/UAR client code.** No reqwest-based call path, no service-identity JWT handling, no `X-Forge-Origin-JWT` / `X-Forge-Signature` construction.
- **No vault integration.** Surface code does not yet call `vault.resolve_api_key` to obtain credentials without storing them in the DB.
- **Extension entry point incomplete.** `src/lib.rs` only exports `llm_version()`; the SQL functions in `flint_llm.sql` are comments, not implemented as `#[pg_extern]`.
- **Image preload mismatch.** The runtime `CMD` must include `ext_flint_llm` for the background worker to register; `postgresql.flint.conf` already expects it but the Dockerfile overrides it.

### G2 — Async embeddings background worker + `llm.jobs` + declarative surface

- **No BackgroundWorker implementation.** No `pgrx::bgworkers::BackgroundWorker` registration, no `_PG_init` worker registration, no dedicated tokio runtime or HTTP client inside the worker.
- **`llm.enable_embedding()` not implemented.** Need trigger that enqueues on INSERT/UPDATE, creates target `vector(dim)` column + HNSW index, and writes back embedding results.
- **Rate-limit governor missing.** No token-bucket / RPM / TPM / cost budget control in the worker.
- **SPI writeback path missing.** Worker must update target rows via SPI using `flint_llm_worker` role with RLS-bypass capability (or service_role).
- **Origin JWT capture pattern unproven.** `llm.jobs.origin_jwt` column exists, but no enqueue trigger captures `current_setting('request.headers', true)::json->>'authorization'` yet.

### G3 — Sync surface

- **`llm.embed()` / `llm.complete()` not implemented.** Need `#[pg_extern]` returning `vector` and `text` respectively.
- **Interrupt/timeout safety not implemented.** pgrx sync call must run liter-llm on a dedicated runtime thread with periodic `CHECK_FOR_INTERRUPTS` / `WaitLatch` and a hard timeout.
- **Error propagation not designed.** Need a stable SQL error class and mapping from HTTP/gateway failures.
- **No gating mechanism.** Need to ensure these are not callable from `anon` and cannot be used as default write triggers.

### G4 — Async summaries

- **`llm.enable_summary()` not implemented.** Same provisioner + trigger + worker pattern as embeddings, but target is a `text` column and prompt template is configurable.
- **Prompt templating missing.** No `prompt_template` substitution engine for row values.
- **Depends on G2 worker infrastructure.** Cannot be built before the BGW + queue pattern is proven.

## Cross-cutting risks

1. **pgrx background workers are officially "unexplored" territory for async runtimes.** The worker will host tokio + reqwest inside a Postgres background process; need careful signal/latch handling and no `std::process::exit`.
2. **CI cannot validate pgrx extensions today.** `scripts/ci-check.sh` only runs `cargo fmt`, `cargo clippy`, and `cargo check --workspace`. The pgrx crates are excluded from the workspace, so the only validation path is the Docker build (requires Docker runtime, unavailable here).
3. **Provider/model availability unresolved (OQ-10).** Need to confirm whether flint-gate/UAR exposes `text-embedding-3-large` (3072-d) or if we fall back to `text-embedding-3-small` (1536-d). The A2UI embeddings table currently hard-codes `vector(1536)` and `text-embedding-3-large` as a comment — a real mismatch risk.
4. **Vault role wiring unverified.** `flint_llm_worker` role is created by `flint_vault` SQL and granted `flint_secret_reader`, but the BGW must connect with a role that can both read `llm.jobs` and execute `vault.resolve_api_key`.
5. **shared_preload_libraries conflict.** Dockerfile `CMD` currently preloads `pg_net,pg_cron`; must be reconciled with `postgresql.flint.conf` value `pg_net,ext_flint_llm`. Both `pg_cron` and `ext_flint_llm` likely need to be preloaded.

## Recommended change shaping

Keep the four seeded changes but expand p4-c001 to resolve the liter-llm bridge decision:

- **p4-c001-liter-llm-binding:** add deps (`reqwest`, `serde_json`, `tokio` with `rt-multi-thread` only, `secrecy`), implement the flint-gate/UAR HTTP client, add `vault.resolve_api_key` integration, fix Dockerfile preload, expose `llm_version()` and basic `#[pg_extern]` stubs.
- **p4-c002-async-embeddings:** implement BGW registration (`_PG_init` + `BackgroundWorker`), worker dequeue/processing loop, SPI writeback, `llm.enable_embedding()` provisioner + trigger, rate-limit governor.
- **p4-c003-sync-surface:** implement `llm.embed()`/`llm.complete()` with dedicated runtime thread, interrupt checks, timeout, error propagation, and EXECUTE grants.
- **p4-c004-summaries:** implement `llm.enable_summary()` and prompt-template rendering on top of the G2 worker.

## Exit-criterion readiness

The phase gate is: *a signed component handles an HTTP request and calls back into Quarry/Ember under origin identity, RLS-enforced.* That is actually the **Phase 5** Kiln exit criterion per the spec; for Phase 4 the correct gate is: *embeddings stay synced via the worker without blocking inserts; sync calls honor `statement_timeout`/cancel.* This should be clarified before planning.

## Open questions to resolve before planning

1. Is `liter-llm` an external crate we add to `Cargo.toml`, or do we implement the flint-gate/UAR HTTP client directly inside `ext-flint-llm`?
2. What is the exact flint-gate endpoint + request/response shape for embeddings and completions?
3. Which embedding model dimension is actually available via flint-gate/UAR (OQ-10)?
4. Should the BGW use SPI directly or connect back via libpq/tokio-postgres with `service_role`?
5. What are the rate-limit/cost budgets for v1 (per-minute RPM, daily spend cap)?
