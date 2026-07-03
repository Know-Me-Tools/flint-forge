# Assessment — p3.5-ci-postgres-hardening

_Generated: 2026-07-03. Phase status: assessment_ready (progress.json 0/0)._

Gap report against the phase goals (G1–G5), from direct codebase inspection on
`main` @ `8366033`.

## Summary

The p3 real-time + REST paths are **implemented and merged**, but CI verifies almost
none of it: `scripts/ci-check.sh` runs **only** `cargo fmt --check`, `cargo clippy
--workspace --all-targets -- -D warnings`, and `cargo check --workspace` — **no
`cargo test` at all**, and **no Postgres / `DATABASE_URL`**. So every integration test
(≥5 DB-gated files) is dark in CI, and the one thing CI *does* run (workspace clippy)
is **already red** on the `hello-component` example. This phase makes the DB-backed
paths CI-gating and clears the accumulated test debt.

## Per-goal gaps

### G1 — CI Postgres service — **NOT MET (large gap)**
- `scripts/ci-check.sh`: fmt + clippy + `cargo check` only. No `cargo test`, no DB.
- `.dagger/main.go`: wraps `ci-check.sh` in `rust:1.90-bookworm` — no Postgres sidecar,
  no `DATABASE_URL`, no pgvector/pg_graphql.
- No `.github/workflows/` — CI is Dagger + script only.
- **Work:** add a PG18 service (with `pgvector` + `pg_graphql`) to the Dagger pipeline
  (service binding) or a compose/`docker`-run step; export `DATABASE_URL`; add a
  `cargo test` invocation (split: always-on unit tests + DB-gated integration tests).
  Resolves **OQ-9**.

### G2 — Un-ignore / run the live-PG tests — **NOT MET**
DB-gated test files currently dark in CI (they pass locally with a DB, skip/ignored otherwise):
- `crates/fdb-realtime/tests/listen_live_pg.rs` — `#[ignore]` (2 tests).
- `crates/fdb-reflection/tests/pgvector_rpc.rs` — `DATABASE_URL`-gated (3 ignored).
- `crates/fdb-app/tests/meta_listener.rs` — DB-gated.
- `crates/fdb-gateway/tests/{a2ui_schema_test,a2ui_trigger_test}.rs` — DB-gated.
- **Missing coverage:** no DB-backed test of the **embedding REST path**
  (`GET …?select=*,child(*)` → nested JSON) or of `PgRest::execute` end-to-end — both
  only have SQL-shape unit tests today.
- **Work:** once G1 lands, gate these on `DATABASE_URL` presence (run when set, skip when
  not) rather than a blanket `#[ignore]`, and add the two missing DB-backed tests.

### G3 — Fix pre-existing `fdb-gateway` test debt — **NOT MET**
- `keto_sync::tests::keto_sync_config_ignores_non_numeric_env` (`keto_sync.rs:306`) —
  flakes under parallel `set_var`/`remove_var` on the shared `KETO_SYNC_INTERVAL_SECS`
  env var (confirmed failing on clean `main`). Fix: isolate (serialize the env-mutating
  tests, or parse from an injected value instead of the process env).
- `tests/a2ui_seed_test.rs` — `uninlined_format_args` clippy lint (2+ sites).
- **Work:** deterministic env-test isolation + inline the format args.

### G4 — Workspace clippy clean end-to-end — **NOT MET (blocks CI today)**
- `cargo clippy --workspace --all-targets -- -D warnings` (what `ci-check.sh` runs)
  **fails** on `examples/hello-component`: `used_underscore_items` from the
  macro-generated WASI bindings (`bindings::export`). This means the *current* CI gate
  is red regardless of any DB work.
- **Work:** narrowly `#[allow(clippy::used_underscore_items)]` on the generated-binding
  module (or exclude the example from the lint gate). Small but unblocks CI.

### G5 — Reconcile p3 bookkeeping — **NOT MET (carried from p3 reflection)**
- `progress.json` (p3) tracks c010–c018; c017/c018 still `pending`. Delivered-and-merged
  c019 (PostgREST engine) + c020 (LISTEN source) + the G4 seam are **absent** from the
  change array. c019 has an unarchived `openspec/changes/` dir; c020 has none.
- **Work:** record c019/c020 as delivered; mark **c017 superseded-by-c020**; resolve
  **c018** against the merged introspection work (archive or close); archive the c019
  openspec change. (Bookkeeping only — no product code.)

## Open questions for Plan

- **OQ-9** (central): which DB provisioning mechanism — Dagger service binding vs. a
  `docker run postgres` step vs. a GitHub Actions `services:` block (would require
  introducing `.github/workflows/`)? Recommendation: extend the existing Dagger pipeline
  (keeps "runs the same locally and in CI").
- Does a prebuilt PG18 image with **both** `pgvector ≥ 0.7` and `pg_graphql` exist, or
  must we build one (Dockerfile) — affects G1 scope materially.
- Should DB-gated tests be a **separate CI stage** (so unit CI stays fast and the DB
  stage is clearly attributable), or folded into one `cargo test`?

## Handoff note

Key gaps: CI runs no tests and has no DB (G1), so all integration tests are dark (G2);
the workspace clippy gate is already red on `hello-component` (G4 — unblock first, it's
cheap); gateway has a known env-flake + lint (G3); and p3's c019/c020 bookkeeping is
unreconciled (G5, bookkeeping-only). Suggested plan order: G4 (unblock CI) → G3 (green
the existing tests) → G1 (DB service) → G2 (un-ignore + new DB tests) → G5 (bookkeeping).
