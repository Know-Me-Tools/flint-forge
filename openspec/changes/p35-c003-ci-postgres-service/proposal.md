# p35-c003 — CI Postgres service + cargo test in the pipeline

## Change ID
`p35-c003-ci-postgres-service`

## Phase
`p3.5-ci-postgres-hardening`

## Goal Mapping
**G1** — provision a PG18 + pgvector + pg_graphql database in CI, export
`DATABASE_URL`, and run `cargo test`. Resolves **OQ-9**.

## Depends on
`p35-c001` (gate green).

## Problem
`scripts/ci-check.sh` runs only `cargo fmt --check`, `cargo clippy`, and
`cargo check` — **no `cargo test`**. The Dagger pipeline (`.dagger/main.go`) wraps that
script in `rust:1.90-bookworm` with **no Postgres**. So no test runs in CI, and no
DB-integration test can run at all.

## Scope
- Extend the Dagger pipeline with a **Postgres service binding** (chosen over GitHub
  Actions `services:` to keep "runs the same locally and inside the Dagger container").
  The PG service must have `pgvector` (≥ 0.7) and `pg_graphql` installed.
  - **OQ-9 sub-question (resolve in this change's design):** use a prebuilt image that
    bundles both extensions, or build a small Dockerfile on top of `postgres:18`? If no
    single prebuilt image has both, build one and pin it.
- Apply workspace migrations (`migrations/`, incl. `0006_change_notify.sql`) to the CI
  DB before tests.
- Add `cargo test` to the CI flow, split into two stages:
  - **unit** — always runs (`cargo test --workspace --lib --bins`), no DB.
  - **db-integration** — runs with `DATABASE_URL` exported (the `--include-ignored` /
    DATABASE_URL-gated tests). Kept a distinct step so failures are attributable.
- `scripts/ci-check.sh` updated (or a sibling `ci-test.sh`) so local == CI.

## Out of Scope
- Converting the individual tests to DB-gated form (that's c004) — this change provides
  the *service + runner*; c004 makes the tests consume it.

## Acceptance Criteria
- [ ] Dagger pipeline starts a PG18 instance with pgvector + pg_graphql and exposes `DATABASE_URL`.
- [ ] Migrations apply cleanly to the CI DB.
- [ ] `cargo test --workspace` (unit) runs in CI and is green.
- [ ] The DB-integration stage runs the DATABASE_URL-gated tests and is green.
- [ ] Local invocation reproduces the CI DB stage (documented in README / script).

## Open Questions
- **OQ-9**: prebuilt PG18+pgvector+pg_graphql image vs. build-our-own — decided within
  this change; if built, the Dockerfile is pinned and lives under `.dagger/` or `docker/`.
