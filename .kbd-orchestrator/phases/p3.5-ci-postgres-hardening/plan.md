# Plan — p3.5-ci-postgres-hardening

_Generated: 2026-07-03. Backend: OpenSpec. From `assessment.md` (gaps G1–G5)._

Ordered change list. Ordering follows the cloud-native "make the pipeline
trustworthy before adding stages to it" principle: unblock the red gate (G4),
green the existing tests (G3), then add the DB service (G1), then make the
DB-integration tests gating (G2); bookkeeping (G5) last (no code dependency).

| # | Change ID | Goal | Depends on | Agent |
| --- | --- | --- | --- | --- |
| 1 | `p35-c001-clippy-unblock-hello-component` | G4 | — | rust-build-resolver |
| 2 | `p35-c002-gateway-test-debt` | G3 | c001 | rust-reviewer |
| 3 | `p35-c003-ci-postgres-service` | G1 | c001 | devops-engineer |
| 4 | `p35-c004-db-integration-tests` | G2 | c003 | tdd-guide |
| 5 | `p35-c005-p3-bookkeeping-reconcile` | G5 | — | (orchestrator; docs-only) |

## Changes

### 1. `p35-c001-clippy-unblock-hello-component` — G4 (unblock CI)
The current CI gate (`cargo clippy --workspace --all-targets -- -D warnings` in
`scripts/ci-check.sh`) is **already red** on `examples/hello-component`
(`used_underscore_items` from generated WASI bindings). Narrowly
`#[allow(clippy::used_underscore_items)]` on the generated-binding module (or exclude
the example from the workspace lint gate). First because nothing else can be
CI-validated while the gate fails. Small, mechanical.

### 2. `p35-c002-gateway-test-debt` — G3 (green existing tests)
- Isolate `keto_sync::tests::keto_sync_config_ignores_non_numeric_env` so it no longer
  flakes under parallel `set_var`/`remove_var` on `KETO_SYNC_INTERVAL_SECS` (serialize
  the env-mutating tests, or parse from an injected value rather than the process env).
- Clear `uninlined_format_args` in `tests/a2ui_seed_test.rs`.
- Gate: `cargo test -p fdb-gateway` green (non-DB tests) under parallel execution.

### 3. `p35-c003-ci-postgres-service` — G1 (DB service) — **largest**
- Extend the Dagger pipeline (`.dagger/main.go`) with a **Postgres service binding**
  (recommended over GitHub Actions `services:`, to keep local == CI) exposing a PG18
  instance with `pgvector` (≥ 0.7) and `pg_graphql`; export `DATABASE_URL`.
- Add a `cargo test` invocation to the CI flow, split into: always-on unit tests, and a
  DB-integration stage that runs when `DATABASE_URL` is set.
- **OQ-9 open**: does a prebuilt PG18 image with both extensions exist, or must we build
  a Dockerfile? (Resolve in c003's design/proposal — may pull in a small image build.)
- Migrations (incl. `0006_change_notify.sql`) applied to the CI DB before tests.

### 4. `p35-c004-db-integration-tests` — G2 (make tests gating)
- Convert the DB-gated tests to run when `DATABASE_URL` is present (rather than blanket
  `#[ignore]`): `fdb-realtime/tests/listen_live_pg.rs`, `fdb-reflection/tests/pgvector_rpc.rs`,
  `fdb-app/tests/meta_listener.rs`, `fdb-gateway/tests/a2ui_{schema,trigger}_test.rs`.
- **Add missing DB-backed coverage**: the embedding REST path (`GET …?select=*,child(*)`
  → correct nested JSON) and `PgRest::execute` end-to-end (both currently SQL-shape
  unit tests only).
- Gate: the DB stage from c003 runs all of these green.

### 5. `p35-c005-p3-bookkeeping-reconcile` — G5 (bookkeeping; no product code)
- In p3-auth-rls-keto `progress.json`: record c019 (PostgREST engine) + c020 (LISTEN
  source) as delivered; mark **c017 superseded-by-c020**; resolve/close **c018** against
  the merged introspection work; archive the `openspec/changes/p3-c019-*` dir.
- Purely reconciliation of tracked state to match `main`.

## First change to apply

`p35-c001-clippy-unblock-hello-component` — unblocks the CI gate so every subsequent
change can be CI-validated.
