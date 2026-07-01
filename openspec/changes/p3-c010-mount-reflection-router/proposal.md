# p3-c010 — Mount Reflection Router in fdb-gateway

## Change ID
`p3-c010-mount-reflection-router`

## Phase
`p3-auth-rls-keto`

## Goal Mapping
Unblocks G3 verification surface. Currently CRUD requests 404 because the
`fdb-reflection` REST router is initialized but not nested under the Axum
router (`TODO(p2-c005)` comment in `fdb-gateway/src/main.rs`).

## Problem
`StateManager` + `ArcSwap` hot-reload was delivered in p2-c005, but the
reflection router that serves `/rest/<table>`, `/rpc/<name>` was never
`.nest()`-ed. This blocks all REST integration tests in this phase.

## Scope
- In `fdb-gateway/src/main.rs`: replace the `TODO(p2-c005)` comment with
  `.nest("/rest", reflection_router.clone())` (or the established mount path
  in `fdb-reflection::router()`).
- Confirm the `/rpc` and `/healthz` routes remain mounted.
- Add an integration test that GETs `/rest/<known_table>` and asserts a
  200/4xx response (not 404 from missing route).
- Verify `flint_meta.cedar_policies` table existence (pre-flight for c012).
  If absent, note as a finding — do NOT add the table here (that is c012).

## Out of Scope
- CRUD handler bodies (c013, c014).
- Cedar policy table creation (c012).
- New dependencies.

## Acceptance Criteria
- [ ] `cargo check --workspace` green
- [ ] `cargo clippy --workspace -- -D warnings` green
- [ ] `cargo test -p fdb-gateway` green; new integration test asserts reflection router is mounted
- [ ] GET `/rest/<table>` returns a non-404 response in the integration test
- [ ] Pre-flight note recorded for `flint_meta.cedar_policies` (present or absent)
