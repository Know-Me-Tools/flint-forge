# Implementation Plan — p4-flint-ember (Flint Ember)

Planned: 2026-07-03
Status: plan_complete
Assessed by: previous session

## Overview

Build the in-database LLM/embeddings layer (`ext-flint-llm` / `flint_llm`) for Flint Forge. The extension exposes two surfaces: a **sync** surface (`llm.embed`, `llm.complete`) for read/explicit paths, and an **async** surface (`llm.enable_embedding`, `llm.enable_summary`) backed by a pgrx background worker and the `llm.jobs` queue. All model calls route inward through flint-gate/UAR; no provider keys are stored in the database plaintext (resolved via `flint_vault`).

## Architecture decisions

1. **No external `liter-llm` crate.** Implement the flint-gate/UAR HTTP client directly inside `ext-flint-llm` using `reqwest` + `tokio`. The spec refers to "liter-llm" as the routing concept; the actual transport is an authenticated HTTPS call to flint-gate.
2. **Embedding default: `text-embedding-3-small` (1536-d).** The existing `flint_a2ui.embeddings` table already hard-codes `vector(1536)`. `text-embedding-3-large` (3072-d) remains available via explicit `model` parameter once flint-gate supports it (OQ-10).
3. **Background worker writeback via SPI.** The BGW runs as a Postgres background process and uses pgrx SPI helpers to update target rows, avoiding a second libpq connection and its credential management.
4. **Rate-limit governor is in-process token bucket.** Per-worker (single BGW process) token bucket for RPM and TPM; v1 does not enforce a global cross-process budget.
5. **Sync surface runs on a dedicated tokio runtime thread** spawned per call, with the backend thread blocked on a channel. `CHECK_FOR_INTERRUPTS` is not directly callable from a detached Rust thread, so the call uses a hard wall-clock timeout and returns an error; `pg_cancel_backend` kills the backend process and aborts the HTTP request via timeout.
6. **No Cedar enforcement inside the extension.** Cedar policy is applied at flint-gate/UAR; the extension forwards the origin JWT and relies on the gateway for authorization.

## Phase exit criterion (corrected)

> **Embeddings stay synced via the background worker without blocking inserts; sync calls honor `statement_timeout`/cancel.**
>
> *(The original goals.md copied the Phase 5 Kiln gate; this is the proper Phase 4 gate per `docs/FLINT-FORGE-SPEC.md` §7.)*

## Change 1: p4-c001-liter-llm-binding (G1)

Foundation: dependencies, HTTP client, vault integration, and image preload fix.

### Task 1.1: Add dependencies to `crates/ext-flint-llm/Cargo.toml`
**Scope:** S (1 file)
**Description:** Add `reqwest`, `serde_json`, `tokio` (`rt-multi-thread` only, no full), `secrecy`, `thiserror`, and `tracing` to the pgrx extension crate. No workspace inheritance because pgrx crates build standalone in the Docker stage.
**Acceptance criteria:**
- [ ] `cargo check` passes for `ext-flint-llm` (manual run in a pgrx-ready environment; note CI cannot validate).
- [ ] Crate still packages with `cargo pgrx package`.
**Files touched:** `crates/ext-flint-llm/Cargo.toml`

### Task 1.2: Define the flint-gate/UAR client module
**Scope:** M (2-3 files)
**Description:** Create `src/gate_client.rs` with:
- `GateClient` struct holding base URL, service-identity JWT, and `reqwest::Client`.
- `embed(input: &str, model: Option<&str>) -> Result<Vec<f32>, LlmError>`.
- `complete(prompt: &str, opts: &serde_json::Value) -> Result<String, LlmError>`.
- Request/response types: `EmbedRequest`, `EmbedResponse`, `CompleteRequest`, `CompleteResponse`.
- Headers: `Authorization: Bearer <service-token>`, `X-Forge-Origin-JWT: <origin-jwt>` (when provided), `Content-Type: application/json`.
- Default endpoints: `POST /v1/llm/embed` and `POST /v1/llm/complete`. Response shapes:
  - embed: `{"embedding": [f32], "model": "...", "usage": {...}}`
  - complete: `{"content": "...", "model": "...", "usage": {...}}`
- `LlmError` enum with `thiserror` variants: `Http`, `BadResponse`, `Gateway { code, message }`, `Timeout`, `Interrupted`.
**Acceptance criteria:**
- [ ] Module compiles under `cargo check -p flint_llm` (or equivalent pgrx check).
- [ ] No `unwrap`/`expect` in library paths; all errors map to `LlmError`.
- [ ] Unit tests for request serialization and response deserialization pass.
**Files touched:** `crates/ext-flint-llm/src/gate_client.rs`, `crates/ext-flint-llm/src/error.rs` (new), `crates/ext-flint-llm/src/lib.rs`

### Task 1.3: Vault-backed credential resolution
**Scope:** M (1-2 files)
**Description:** Create `src/credentials.rs` with a `resolve_service_token()` helper that calls `vault.get_secret('flint-gate-service-token')` via SPI. For dev/test, allow fallback to env var `FLINT_LLM_SERVICE_TOKEN` (documented as dev-only). The helper is `SECURITY DEFINER` via SQL wrapper and executed as `flint_llm_worker`.
**Acceptance criteria:**
- [ ] Function returns the service token without exposing it in logs or SQL result to unprivileged roles.
- [ ] Dev fallback works when `vault` secret is absent and env var is set.
- [ ] Missing token produces a clear `LlmError::Credential` message.
**Files touched:** `crates/ext-flint-llm/src/credentials.rs`, `crates/ext-flint-llm/sql/flint_llm.sql`

### Task 1.4: Fix Postgres image preload configuration
**Scope:** S (2 files)
**Description:** Reconcile `images/postgres18/Dockerfile` `CMD` and `images/postgres18/postgresql.flint.conf` so both preload `pg_net,pg_cron,ext_flint_llm` (order: `ext_flint_llm` last so its `_PG_init` runs after pg_net/pg_cron). Remove the stale `postgresql.flint.conf` value `pg_net,ext_flint_llm` that omits `pg_cron`.
**Acceptance criteria:**
- [ ] Dockerfile `CMD` and config file agree on `shared_preload_libraries`.
- [ ] Docker build still succeeds (verification requires Docker runtime).
**Files touched:** `images/postgres18/Dockerfile`, `images/postgres18/postgresql.flint.conf`

### Task 1.5: Expose `llm_version()` and lock down grants
**Scope:** S (2 files)
**Description:** Keep `llm_version()`; add SQL to create the `llm` schema, the `llm.jobs` table, and revoke all `llm.*` functions/tables from `PUBLIC`. Grant `USAGE` on schema `llm` to `authenticated`; grant `EXECUTE` on `llm_version()` to `authenticated`.
**Acceptance criteria:**
- [ ] `CREATE EXTENSION flint_llm` installs schema, table, and version function.
- [ ] `anon` cannot execute `llm_version()`; `authenticated` can.
**Files touched:** `crates/ext-flint-llm/src/lib.rs`, `crates/ext-flint-llm/sql/flint_llm.sql`

### Checkpoint 1
- [ ] `cargo fmt --all --check` green.
- [ ] p4-c001 builds standalone (`cargo pgrx package` in a pgrx-ready environment).
- [ ] No secrets or tokens logged anywhere.

---

## Change 2: p4-c002-async-embeddings (G2)

Build the background worker, queue processing, and `llm.enable_embedding()` declarative surface.

### Task 2.1: Background worker registration
**Scope:** M (2 files)
**Description:** Add `_PG_init` that registers a pgrx `BackgroundWorker` named `flint_llm_worker`. Configure `bgw_flags` to allow reconnect and not wait for crash recovery. The worker entry point initializes a dedicated tokio runtime and a `GateClient`, then enters the dequeue loop.
**Acceptance criteria:**
- [ ] Worker appears in `pg_stat_activity`/`pg_stat_bgworker` after loading the extension.
- [ ] Worker logs a clear startup message (no secrets).
- [ ] Worker exits cleanly on postmaster shutdown signal.
**Files touched:** `crates/ext-flint-llm/src/worker.rs` (new), `crates/ext-flint-llm/src/lib.rs`

### Task 2.2: Dequeue and batching loop
**Scope:** M (2 files)
**Description:** Implement `dequeue_jobs(batch_size: i64) -> Vec<JobRow>` using SPI with:
```sql
SELECT ... FROM llm.jobs
WHERE status = 'pending' AND visible_at <= now()
ORDER BY id FOR UPDATE SKIP LOCKED LIMIT $1;
```
Group pending jobs by `(kind, model)`, call `GateClient::embed` in batches (collect `input`s, send one request per model if gateway supports batching; otherwise one request per input), and mark jobs `completed` or `failed` with `retry_count`/`visible_at` backoff.
**Acceptance criteria:**
- [ ] Worker picks up only pending-visible jobs.
- [ ] Failed jobs retry with exponential backoff up to 3 times, then `failed`.
- [ ] `FOR UPDATE SKIP LOCKED` prevents multiple workers from grabbing the same job.
**Files touched:** `crates/ext-flint-llm/src/worker.rs`, `crates/ext-flint-llm/src/jobs.rs` (new)

### Task 2.3: SPI writeback for embedding results
**Scope:** M (2 files)
**Description:** After receiving an embedding vector, the worker executes a dynamic SQL `UPDATE` against `(schema_name, table_name)` using the JSON `pk` to locate the row and writes the vector to `target_column`. Use `spi::Spi::execute` under the `flint_llm_worker` role. Validate `target_column` is an existing `vector(dim)` column before write.
**Acceptance criteria:**
- [ ] A processed embedding job updates the target row's vector column.
- [ ] Invalid `pk` or missing table logs an error and marks job failed.
- [ ] No SQL injection: column/table names are identifier-quoted via pgrx helpers or manual `""` escaping.
**Files touched:** `crates/ext-flint-llm/src/worker.rs`, `crates/ext-flint-llm/src/writeback.rs` (new)

### Task 2.4: Rate-limit governor
**Scope:** M (1-2 files)
**Description:** Add a token-bucket governor in the worker process:
- RPM bucket: refill 60 tokens/minute, burst 10.
- TPM bucket: refill 1,000,000 tokens/minute, burst 100,000.
- Cost bucket: daily $10 cap (tracked in-memory; reset on worker restart).
Before each request, acquire tokens; if unavailable, sleep or defer job with `visible_at` set to refill time.
**Acceptance criteria:**
- [ ] Worker never exceeds 60 RPM in steady state.
- [ ] Worker pauses when TPM cap is reached.
- [ ] Worker stops processing when daily cost cap is hit and logs a warning.
**Files touched:** `crates/ext-flint-llm/src/governor.rs` (new), `crates/ext-flint-llm/src/worker.rs`

### Task 2.5: `llm.enable_embedding()` declarative surface
**Scope:** M (2-3 files)
**Description:** Implement `#[pg_extern] fn enable_embedding(table: PgRelation, column: &str, model: &str, dim: i32)` that:
1. Adds `column vector(dim)` if absent.
2. Adds an HNSW index on the column if absent.
3. Creates/updates a registry row in `llm.embedding_configs` (`schema_name`, `table_name`, `source_column`, `target_column`, `model`, `dim`).
4. Installs a generic `SECURITY DEFINER` trigger `llm_trigger_embed_<table>` that enqueues a job on INSERT/UPDATE of `source_column`.
The trigger captures `current_setting('request.headers', true)::json->>'authorization'` into `origin_jwt`.
**Acceptance criteria:**
- [ ] Calling `llm.enable_embedding('public.articles', 'body', 'text-embedding-3-small', 1536)` provisions the column, index, and trigger.
- [ ] Inserting a row enqueues a pending job with correct `schema_name`, `table_name`, `pk`, `source`, `target_column`, `model`.
- [ ] The worker eventually updates the target vector column.
**Files touched:** `crates/ext-flint-llm/src/provisioners.rs` (new), `crates/ext-flint-llm/src/triggers.rs` (new), `crates/ext-flint-llm/sql/flint_llm.sql`

### Task 2.6: Initial backfill on enable
**Scope:** S (1 file)
**Description:** When `enable_embedding` is called, enqueue an embedding job for every existing row where `source_column IS NOT NULL` and `target_column IS NULL`, using `visible_at = now() + id * interval '100 ms'` to spread load.
**Acceptance criteria:**
- [ ] Existing rows get enqueued with visible_at spread over time.
- [ ] No duplicate jobs are created if `enable_embedding` is called twice.
**Files touched:** `crates/ext-flint-llm/src/provisioners.rs`

### Checkpoint 2
- [ ] Docker image builds and boots with the BGW registered.
- [ ] Manual end-to-end: INSERT a row → job appears → worker processes → vector column populated.
- [ ] Governor prevents runaway calls when inserting 100 rows rapidly.

---

## Change 3: p4-c003-sync-surface (G3)

Implement the synchronous, interrupt/timeout-safe `llm.embed` and `llm.complete` functions.

### Task 3.1: `llm.embed()` sync function
**Scope:** M (2 files)
**Description:** Implement `#[pg_extern] fn embed(input: &str, model: default!(&str, "default") ) -> Option<pgvector::Vector>` that:
1. Resolves the model name (`default` → `text-embedding-3-small`).
2. Resolves service token via `credentials::resolve_service_token`.
3. Spawns the `GateClient::embed` call on a dedicated tokio runtime thread.
4. Blocks the backend thread on a channel with a hard timeout (default 30s, configurable via GUC `llm.sync_timeout_ms`).
5. Returns the embedding as `pgvector::Vector`.
**Acceptance criteria:**
- [ ] `SELECT llm.embed('hello world')` returns a 1536-dimensional vector.
- [ ] Timeout produces a clear SQL error.
- [ ] Function is not granted to `anon` or `PUBLIC`.
**Files touched:** `crates/ext-flint-llm/src/sync.rs` (new), `crates/ext-flint-llm/sql/flint_llm.sql`

### Task 3.2: `llm.complete()` sync function
**Scope:** M (1-2 files)
**Description:** Implement `#[pg_extern] fn complete(prompt: &str, opts: default!(Json, "{}") ) -> Option<String>` with the same timeout/interrupt pattern. `opts` is forwarded to flint-gate as-is. Returns the generated text.
**Acceptance criteria:**
- [ ] `SELECT llm.complete('Summarize this')` returns text.
- [ ] Malformed `opts` raises a SQL error before calling the gateway.
**Files touched:** `crates/ext-flint-llm/src/sync.rs`

### Task 3.3: Timeout and cancellation safety
**Scope:** M (1-2 files)
**Description:** Ensure the sync surface:
- Uses a bounded channel with `recv_timeout` on the backend thread.
- Aborts the tokio task on timeout.
- Does not call SPI or touch Postgres internals from the tokio thread.
- Respects GUC `llm.sync_timeout_ms` (loaded at call start).
**Acceptance criteria:**
- [ ] A slow gateway call times out and returns an error within `llm.sync_timeout_ms + 1s`.
- [ ] Repeated timeouts do not leak threads or file descriptors.
**Files touched:** `crates/ext-flint-llm/src/sync.rs`, `crates/ext-flint-llm/src/gate_client.rs`

### Task 3.4: SQL grants and trigger gating
**Scope:** S (1 file)
**Description:** Grant `EXECUTE` on `llm.embed` and `llm.complete` only to `authenticated`. Document that these functions must never be invoked from a row-level trigger as the default behavior.
**Acceptance criteria:**
- [ ] `authenticated` can call both functions; `anon` cannot.
- [ ] Extension README / doc comments warn against trigger usage.
**Files touched:** `crates/ext-flint-llm/sql/flint_llm.sql`

### Checkpoint 3
- [ ] Sync calls work end-to-end against a local mock or dev gateway.
- [ ] Timeout path tested with a deliberately slow mock.
- [ ] `pg_cancel_backend` on a running sync call terminates it (best-effort via process kill).

---

## Change 4: p4-c004-summaries (G4)

Build `llm.enable_summary()` on top of the async worker.

### Task 4.1: Prompt template engine
**Scope:** S (1 file)
**Description:** Implement a minimal template renderer in `src/templates.rs` supporting `{source_column}` substitution and `{column_name}` placeholders from the source row JSON. No arbitrary expression evaluation (security). Example: `"Summarize: {body}"`.
**Acceptance criteria:**
- [ ] Template renders with source row values.
- [ ] Missing placeholder leaves `{name}` unchanged and logs a warning.
- [ ] No code execution or nested template injection.
**Files touched:** `crates/ext-flint-llm/src/templates.rs` (new)

### Task 4.2: `llm.enable_summary()` declarative surface
**Scope:** M (2 files)
**Description:** Implement `#[pg_extern] fn enable_summary(table: PgRelation, source_col: &str, target_col: &str, prompt_template: &str)` that:
1. Adds `target_col text` if absent.
2. Records config in `llm.summary_configs`.
3. Installs a generic trigger that enqueues `kind = 'summarize'` jobs on INSERT/UPDATE of `source_col`, capturing the rendered prompt in `source->>'prompt'`.
**Acceptance criteria:**
- [ ] Calling `llm.enable_summary('public.articles', 'body', 'summary', 'Summarize: {body}')` provisions column and trigger.
- [ ] Inserting a row enqueues a summarize job with the rendered prompt.
**Files touched:** `crates/ext-flint-llm/src/provisioners.rs`, `crates/ext-flint-llm/src/triggers.rs`

### Task 4.3: Worker handling for summarize jobs
**Scope:** M (1-2 files)
**Description:** Extend the worker to process `kind = 'summarize'` jobs by calling `GateClient::complete(prompt, opts)` and writing the returned text to `target_column` via SPI. Reuse the same rate-limit governor (summaries consume RPM/TPM/cost budget).
**Acceptance criteria:**
- [ ] Summarize jobs are processed and target column updated.
- [ ] Failed summary jobs retry and eventually fail with backoff.
**Files touched:** `crates/ext-flint-llm/src/worker.rs`

### Task 4.4: Initial backfill on enable_summary
**Scope:** S (1 file)
**Description:** Same spread-load backfill pattern as embeddings: enqueue a summary job for existing rows where `target_col IS NULL`.
**Acceptance criteria:**
- [ ] Existing rows get enqueued without duplicates.
**Files touched:** `crates/ext-flint-llm/src/provisioners.rs`

### Checkpoint 4 (Phase exit)
- [ ] `llm.enable_embedding()` and `llm.enable_summary()` both work end-to-end.
- [ ] Sync functions time out safely.
- [ ] Docker image builds with all extensions preloaded.
- [ ] No provider keys stored in DB plaintext; credentials resolved from `flint_vault` or dev env.
- [ ] `cargo fmt` and workspace clippy green (pgrx crate excluded from workspace clippy; run `cargo pgrx clippy` if available).

---

## Verification commands

```bash
# Workspace checks (always available)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace

# pgrx-specific checks (requires pgrx toolchain + Postgres 18)
cd crates/ext-flint-llm
cargo pgrx package --pg-config "$(which pg_config)"
cargo pgrx test --pg-config "$(which pg_config)"

# Docker image build (requires Docker runtime)
docker build -f images/postgres18/Dockerfile -t flint-forge-pg:18 .
```

## Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| pgrx BackgroundWorker + tokio runtime is unstable/explored territory | High | Keep worker single-process, no async SPI, bounded channels, signal handling, extensive manual testing in Docker |
| flint-gate/UAR endpoint contract unknown | High | Define plausible contract in code, document assumptions, gate behind feature flag/env var; update when contract confirmed |
| CI cannot build pgrx extensions | Medium | Use Docker build as canonical validation; track CI gap as follow-up |
| Embedding dimension mismatch with A2UI (1536 vs 3072) | Medium | Default to 1536; make dimension explicit in `enable_embedding`; validate against gateway response |
| Credential leakage in logs/SPI | High | Use `secrecy::Secret`, never log tokens, revoke PUBLIC grants, audit via `vault.access_log` |
| Runaway LLM spend | High | Token-bucket governor, daily cost cap, async default, no anon access |

## Open questions to resolve during implementation

1. **flint-gate endpoint contract:** Confirm URL path, auth header format, and JSON response shape for `/v1/llm/embed` and `/v1/llm/complete`.
2. **OQ-10 embedding dimension/model:** Confirm whether `text-embedding-3-large` is available; if not, keep 1536 default.
3. **Service token storage:** Decide whether dev env uses `FLINT_LLM_SERVICE_TOKEN` or a Vault dev secret; production must use Vault.
4. **BGW SPI role:** Confirm `flint_llm_worker` has sufficient privileges for SPI writeback across schemas (it inherits `flint_secret_reader` already).
5. **Rate-limit defaults:** Confirm RPM/TPM/cost caps with ops; make them configurable via GUCs (`llm.rpm_limit`, `llm.tpm_limit`, `llm.daily_cost_usd_limit`).

## Parallelization

- **Sequential:** c001 → c002 → c003, c004 (c003 and c004 both depend on c001; c004 depends on c002).
- **Safe to parallelize after c002:** Documentation updates and additional integration tests for the worker.
