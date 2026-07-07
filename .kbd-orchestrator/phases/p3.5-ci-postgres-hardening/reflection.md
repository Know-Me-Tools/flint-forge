# Reflection — p3.5-ci-postgres-hardening

Generated: 2026-07-03

## Delta: what diverged from the plan

- **G1 (CI Postgres service) is implemented but not fully verified.** Change `p35-c003-ci-postgres-service` was applied, but the Dagger/Docker pipeline could not be executed in this environment because no Docker CLI/daemon is available. The code changes are in place; verification is blocked on environment capability, not correctness.
- **G2, G3, G4, G5 completed as planned.** `p35-c004-db-integration-tests` passed local live-Postgres validation and uncovered a critical `acquire` RLS bug that was fixed in the same change.
- **c019/c020 bookkeeping was missing from p3 tracking.** The reflection identified that p3's progress.json had delivered work (PostgREST engine + LISTEN source) recorded as untracked changes. This was reconciled in G5.
- **p3-c017 and p3-c018 were left pending.** c017 (FRF reconnect stub) is superseded by c020's LISTEN backend; c018 (introspection merge verify) is resolved by the merged introspection work and c016 gate.

## Root causes

- **CI verification gap:** The environment running this phase does not include the container runtime needed to validate Dagger pipelines. The phase made the implementation changes but could not run the integration proving step.
- **Tracking drift from p3:** p3 delivered c019 and c020 as branch/commit work without updating `progress.json` or creating/archiving OpenSpec artifacts. This is a process hygiene issue, not a technical failure.
- **Manual-only DB tests before this phase:** p3 left DB-backed tests behind `#[ignore]` or `DATABASE_URL` gating. Without CI DB provisioning, those tests had no automated path to run, which this phase addressed structurally.

## Corrective actions / carry-forward

- **Verify c003 in an environment with Docker/Dagger** before considering p3.5 fully closed in CI. The next agent or CI run should execute `scripts/ci-check.sh` and confirm the DB-backed stage runs green.
- **Bootstrap `flint_meta` in the CI DB** as a follow-up to c003; the implementation note flags this as still needed.
- **Keep OpenSpec / progress.json state in sync during execution**, not after the fact. Future phases should mark changes complete and archive OpenSpec directories as each change lands.
- **Resolve OQ-9** definitively: confirm the PG18 image contains `pgvector >= 0.7.0` and `pg_graphql`, and that `DATABASE_URL` reaches the test runner in CI.

## Risks and open questions carried forward

- OQ-9: PG18 image extension versions and CI `DATABASE_URL` wiring.
- OQ-3: `pg_graphql` PG18 tagged release status (gates any GraphQL hybrid work).
- OQ-FRF-1: FRF `WatchEntityType` delivery timeline remains deferred; `ListenChangeSource` is the working backend.
- OQ-12: `flint_meta.agui_descriptor()` GRANT scope (gates p5-c009).

## Recommended next phase

**Phase 4 — Flint Ember (`p4-*`)**

Build the in-database LLM / embeddings layer (`ext-flint-llm` / `flint_llm`) bound to Postgres through liter-llm, routed via flint-gate/UAR with no provider keys stored in the database. Per `docs/FLINT-FORGE-SPEC.md` §4.3 and the phase plan, the four changes are:

1. `p4-c001-liter-llm-binding` — pgrx wrap; route through flint-gate/UAR.
2. `p4-c002-async-embeddings` — background worker + `llm.jobs` queue + `llm.enable_embedding` declarative surface; rate-limit governor.
3. `p4-c003-sync-surface` — `llm.embed` / `llm.complete` with interrupt/timeout safety.
4. `p4-c004-summaries` — `llm.enable_summary` async surface.

Exit gate: a signed component handles an HTTP request and calls back into Quarry/Ember under origin identity, RLS-enforced.
